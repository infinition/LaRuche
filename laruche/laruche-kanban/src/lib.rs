use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskStatus {
    Triage,
    Todo,
    Ready,
    Running,
    Blocked,
    Done,
    Archived,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KanbanTask {
    pub id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<String>,
    pub title: String,
    #[serde(default)]
    pub profile_id: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    pub description: String,
    /// Result delivery channel (e.g. `telegram:123`). `None` -> board default -> home channel.
    #[serde(default)]
    pub channel: Option<String>,
    pub status: TaskStatus,
    #[serde(default)]
    pub blocks: Vec<Uuid>,
    #[serde(default)]
    pub blocked_by: Vec<Uuid>,
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheduled_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
    /// When the task finished. `created_at` alone could not place a completed task on a
    /// timeline: a task created Monday and finished Friday landed on Monday. Defaulted so
    /// boards written before this field still load.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
}

/// Delai par defaut entre deux releves de la colonne Ready, en secondes.
pub const DELAI_DEFAUT: u64 = 5;

/// Ce qui est ECRIT sur le disque.
///
/// Le fichier ne contenait que la table des taches, alors que le tableau porte
/// aussi des reglages. `set_default_channel` appelait bien `save`, mais `save`
/// n'ecrivait pas ce champ: le canal par defaut etait donc perdu a chaque
/// redemarrage, silencieusement, et le reglage semblait ne servir a rien.
#[derive(Serialize, Deserialize)]
struct Disque {
    tasks: HashMap<Uuid, KanbanTask>,
    #[serde(default)]
    default_channel: Option<String>,
    #[serde(default = "delai_defaut")]
    delai_secs: u64,
    #[serde(default)]
    todo_actif: bool,
    #[serde(default = "todo_periode_defaut")]
    todo_periode_min: u64,
    #[serde(default)]
    todo_dernier: Option<DateTime<Utc>>,
}

fn delai_defaut() -> u64 {
    DELAI_DEFAUT
}

/// Une fois par jour. Une releve de la colonne A faire lance du travail sans
/// que personne n'ait rien demande ce jour-la: la cadence par defaut doit etre
/// celle qu'on remarque a peine, pas celle qui surprend.
pub const TODO_PERIODE_DEFAUT_MIN: u64 = 1440;

fn todo_periode_defaut() -> u64 {
    TODO_PERIODE_DEFAUT_MIN
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KanbanBoard {
    tasks: HashMap<Uuid, KanbanTask>,
    /// Board default channel: used to deliver the result of a task without its own channel.
    #[serde(default)]
    default_channel: Option<String>,
    /// Secondes entre deux releves de la colonne Ready par le repartiteur.
    #[serde(default = "delai_defaut")]
    delai_secs: u64,
    /// La releve de la colonne A faire. Eteinte par defaut: une tache posee la
    /// est une intention, pas un ordre, et elle ne doit se mettre a tourner que
    /// si on l'a demande.
    #[serde(default)]
    todo_actif: bool,
    #[serde(default = "todo_periode_defaut")]
    todo_periode_min: u64,
    #[serde(default)]
    todo_dernier: Option<DateTime<Utc>>,
    #[serde(skip)]
    storage_path: PathBuf,
}

impl KanbanBoard {
    pub fn new(path: &Path) -> Self {
        let mut board = Self {
            tasks: HashMap::new(),
            default_channel: None,
            delai_secs: DELAI_DEFAUT,
            todo_actif: false,
            todo_periode_min: TODO_PERIODE_DEFAUT_MIN,
            todo_dernier: None,
            storage_path: path.to_path_buf(),
        };
        board.load();
        board
    }

    fn load(&mut self) {
        let Ok(content) = fs::read_to_string(&self.storage_path) else {
            return;
        };
        // Nouveau format d'abord, ancien ensuite. Un tableau ecrit par une
        // version precedente est une simple table de taches, et il doit
        // continuer a s'ouvrir: personne ne devrait perdre son tableau en
        // mettant a jour.
        if let Ok(d) = serde_json::from_str::<Disque>(&content) {
            self.tasks = d.tasks;
            self.default_channel = d.default_channel;
            self.delai_secs = d.delai_secs;
            self.todo_actif = d.todo_actif;
            self.todo_periode_min = d.todo_periode_min;
            self.todo_dernier = d.todo_dernier;
        } else if let Ok(tasks) = serde_json::from_str::<HashMap<Uuid, KanbanTask>>(&content) {
            self.tasks = tasks;
        }
    }

    pub fn save(&self) {
        if let Some(parent) = self.storage_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let d = Disque {
            tasks: self.tasks.clone(),
            default_channel: self.default_channel.clone(),
            delai_secs: self.delai_secs,
            todo_actif: self.todo_actif,
            todo_periode_min: self.todo_periode_min,
            todo_dernier: self.todo_dernier,
        };
        if let Ok(content) = serde_json::to_string_pretty(&d) {
            let _ = fs::write(&self.storage_path, content);
        }
    }

    /// Secondes entre deux releves de la colonne Ready.
    pub fn delai_secs(&self) -> u64 {
        self.delai_secs.clamp(1, 3600)
    }

    /// Regle ce delai. Borne a une seconde au minimum: en dessous, le
    /// repartiteur passerait son temps a prendre un verrou d'ecriture sur le
    /// tableau pour ne rien y trouver.
    /* ── La releve de la colonne A faire ──────────────────────────────────
     *
     * Elle ne LANCE rien elle-meme: elle fait passer les taches de A faire a
     * Pret, et le repartiteur les prend une par une comme il le fait deja.
     *
     * C'est ce qui rend la chose simple. Un second executeur aurait sa propre
     * boucle, sa propre question de concurrence, et sa propre facon de resoudre
     * le fournisseur de chaque tache. En promouvant, tout cela reste ou c'est
     * deja ecrit: chaque tache garde son profil, elles s'executent une a la
     * fois, et on les voit s'aligner dans Pret au lieu de partir en silence.
     */
    pub fn todo_actif(&self) -> bool {
        self.todo_actif
    }

    pub fn todo_periode_min(&self) -> u64 {
        self.todo_periode_min.clamp(5, 60 * 24 * 30)
    }

    pub fn todo_dernier(&self) -> Option<DateTime<Utc>> {
        self.todo_dernier
    }

    pub fn set_todo_releve(&mut self, actif: bool, periode_min: u64) {
        self.todo_actif = actif;
        self.todo_periode_min = periode_min.clamp(5, 60 * 24 * 30);
        self.save();
    }

    /// L'echeance est-elle passee ? Une releve jamais faite est due tout de
    /// suite: sinon activer le reglage ne se voit qu'au bout d'une periode, et
    /// on croit que ca ne marche pas.
    pub fn todo_est_du(&self, maintenant: DateTime<Utc>) -> bool {
        if !self.todo_actif {
            return false;
        }
        match self.todo_dernier {
            None => true,
            Some(d) => {
                (maintenant - d).num_minutes() >= self.todo_periode_min() as i64
            }
        }
    }

    /// Fait passer les taches A faire dans Pret, les plus anciennes d'abord.
    /// Rend le nombre promu.
    pub fn promouvoir_todo(&mut self, maintenant: DateTime<Utc>) -> usize {
        let mut ids: Vec<(DateTime<Utc>, Uuid)> = self
            .tasks
            .values()
            .filter(|t| t.status == TaskStatus::Todo)
            .map(|t| (t.created_at, t.id))
            .collect();
        ids.sort_by_key(|(d, _)| *d);
        for (_, id) in &ids {
            if let Some(t) = self.tasks.get_mut(id) {
                t.status = TaskStatus::Ready;
            }
        }
        // L'horodatage bouge meme quand la colonne est vide: sans cela, une
        // colonne vide rend la releve due en permanence, et le journal se
        // remplit d'une ligne par minute pour ne rien faire.
        self.todo_dernier = Some(maintenant);
        self.save();
        ids.len()
    }

    pub fn set_delai_secs(&mut self, secs: u64) {
        self.delai_secs = secs.clamp(1, 3600);
        self.save();
    }

    pub fn list(&self) -> Vec<KanbanTask> {
        let mut list: Vec<_> = self.tasks.values().cloned().collect();
        list.sort_by_key(|t| t.created_at);
        list
    }

    pub fn get(&self, id: &Uuid) -> Option<KanbanTask> {
        self.tasks.get(id).cloned()
    }

    /// Returns the next task that can be processed by the orchestrator.
    ///
    /// An explicitly `Ready` task always has priority. Otherwise, returns the
    /// first non-terminal task for which every dependency is `Done` or
    /// `Archived`. The ordering is the same stable creation order as `list`.
    pub fn next_ready(&self) -> Option<KanbanTask> {
        let tasks = self.list();

        if let Some(task) = tasks.iter().find(|task| task.status == TaskStatus::Ready) {
            return Some(task.clone());
        }

        tasks.into_iter().find(|task| {
            !matches!(task.status, TaskStatus::Done | TaskStatus::Archived)
                && task.blocked_by.iter().all(|dependency_id| {
                    self.tasks.get(dependency_id).is_some_and(|dependency| {
                        matches!(dependency.status, TaskStatus::Done | TaskStatus::Archived)
                    })
                })
        })
    }

    pub fn create(
        &mut self,
        title: String,
        description: String,
        idempotency_key: Option<String>,
        profile_id: Option<String>,
        model: Option<String>,
        channel: Option<String>,
    ) -> KanbanTask {
        // Idempotency check
        if let Some(ref key) = idempotency_key {
            if let Some(existing) = self
                .tasks
                .values()
                .find(|t| t.idempotency_key.as_ref() == Some(key))
            {
                return existing.clone();
            }
        }

        let task = KanbanTask {
            id: Uuid::new_v4(),
            idempotency_key,
            title,
            description,
            channel,
            profile_id,
            model,
            status: TaskStatus::Todo,
            blocks: Vec::new(),
            blocked_by: Vec::new(),
            created_at: Utc::now(),
            scheduled_at: None,
            result: None,
            completed_at: None,
        };

        self.tasks.insert(task.id, task.clone());
        self.save();
        task
    }

    /// Board default channel (delivery for tasks without their own channel).
    pub fn default_channel(&self) -> Option<String> {
        self.default_channel.clone()
    }

    /// Sets the board default channel (`None`/empty = none).
    pub fn set_default_channel(&mut self, channel: Option<String>) {
        self.default_channel = channel.filter(|c| !c.trim().is_empty());
        self.save();
    }

    /// Updates a task's channel (`None`/empty = inherit the default).
    pub fn set_channel(&mut self, id: Uuid, channel: Option<String>) -> bool {
        if let Some(t) = self.tasks.get_mut(&id) {
            t.channel = channel.filter(|c| !c.trim().is_empty());
            self.save();
            true
        } else {
            false
        }
    }

    /// Effective channel of a task: its own channel, otherwise the board default.
    pub fn effective_channel(&self, id: Uuid) -> Option<String> {
        self.tasks
            .get(&id)
            .and_then(|t| t.channel.clone())
            .or_else(|| self.default_channel.clone())
    }

    pub fn update(
        &mut self,
        id: Uuid,
        title: Option<String>,
        description: Option<String>,
    ) -> Option<KanbanTask> {
        if let Some(task) = self.tasks.get_mut(&id) {
            if let Some(t) = title {
                task.title = t;
            }
            if let Some(d) = description {
                task.description = d;
            }
            let updated = task.clone();
            self.save();
            Some(updated)
        } else {
            None
        }
    }

    pub fn change_status(&mut self, id: Uuid, status: TaskStatus) -> bool {
        if let Some(task) = self.tasks.get_mut(&id) {
            task.status = status.clone();

            // If the task is completed (Done/Archived), we need to update the children it blocks
            if status == TaskStatus::Done || status == TaskStatus::Archived {
                let blocks = task.blocks.clone();
                for child_id in blocks {
                    if let Some(child) = self.tasks.get_mut(&child_id) {
                        child.blocked_by.retain(|parent_id| parent_id != &id);
                        if child.blocked_by.is_empty() && child.status == TaskStatus::Blocked {
                            child.status = TaskStatus::Ready;
                        }
                    }
                }
            }

            self.save();
            true
        } else {
            false
        }
    }

    pub fn complete(&mut self, id: Uuid, result: String) -> bool {
        if let Some(task) = self.tasks.get_mut(&id) {
            task.result = Some(result);
            task.completed_at = Some(Utc::now());
            self.save(); // save the result first so change_status picks it up or we can just call change_status
        }
        self.change_status(id, TaskStatus::Done)
    }

    pub fn add_dependency(&mut self, child_id: Uuid, parent_id: Uuid) -> bool {
        if child_id == parent_id {
            return false;
        }

        let mut parent_exists = false;
        let mut parent_is_done = false;

        if let Some(parent) = self.tasks.get_mut(&parent_id) {
            parent_exists = true;
            if !parent.blocks.contains(&child_id) {
                parent.blocks.push(child_id);
            }
            parent_is_done =
                parent.status == TaskStatus::Done || parent.status == TaskStatus::Archived;
        }

        if !parent_exists {
            return false;
        }

        if let Some(child) = self.tasks.get_mut(&child_id) {
            if !child.blocked_by.contains(&parent_id) {
                child.blocked_by.push(parent_id);
            }
            if !parent_is_done && child.status != TaskStatus::Blocked {
                child.status = TaskStatus::Blocked;
            }
            self.save();
            true
        } else {
            false
        }
    }

    pub fn remove(&mut self, id: &Uuid) -> bool {
        if let Some(task) = self.tasks.remove(id) {
            // Unblock children
            for child_id in task.blocks {
                if let Some(child) = self.tasks.get_mut(&child_id) {
                    child.blocked_by.retain(|pid| pid != id);
                    if child.blocked_by.is_empty() && child.status == TaskStatus::Blocked {
                        child.status = TaskStatus::Ready;
                    }
                }
            }
            // Remove from parents
            for parent_id in task.blocked_by {
                if let Some(parent) = self.tasks.get_mut(&parent_id) {
                    parent.blocks.retain(|cid| cid != id);
                }
            }
            self.save();
            true
        } else {
            false
        }
    }

    pub fn claim_ready_task(&mut self) -> Option<KanbanTask> {
        // Find a task in Ready (or Todo) state that has no dependencies
        let ready_id = self.tasks.iter().find_map(|(id, task)| {
            if (task.status == TaskStatus::Ready || task.status == TaskStatus::Todo)
                && task.blocked_by.is_empty() {
                    return Some(*id);
                }
            None
        });

        if let Some(id) = ready_id {
            if let Some(task) = self.tasks.get_mut(&id) {
                task.status = TaskStatus::Running;
                let claimed = task.clone();
                self.save();
                return Some(claimed);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_board_dependencies() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("board.json");
        let mut board = KanbanBoard::new(&path);

        let t1 = board.create("Task 1".into(), "Desc 1".into(), None, None, None, None);
        let t2 = board.create("Task 2".into(), "Desc 2".into(), None, None, None, None);

        // t2 depends on t1
        board.add_dependency(t2.id, t1.id);

        let t2_updated = board.get(&t2.id).unwrap();
        assert_eq!(t2_updated.status, TaskStatus::Blocked);

        // complete t1
        board.complete(t1.id, "Done".into());

        let t2_unblocked = board.get(&t2.id).unwrap();
        assert_eq!(t2_unblocked.status, TaskStatus::Ready);
    }

    #[test]
    fn test_idempotency() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("board.json");
        let mut board = KanbanBoard::new(&path);

        let t1 = board.create("A".into(), "B".into(), Some("my_key".into()), None, None, None);
        let t2 = board.create("C".into(), "D".into(), Some("my_key".into()), None, None, None);

        assert_eq!(t1.id, t2.id);
        assert_eq!(board.list().len(), 1);
    }

    #[test]
    fn next_ready_prioritizes_ready_then_unblocked_tasks() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("board.json");
        let mut board = KanbanBoard::new(&path);

        let first = board.create("First".into(), "".into(), None, None, None, None);
        let ready = board.create("Ready".into(), "".into(), None, None, None, None);
        board.change_status(ready.id, TaskStatus::Ready);

        assert_eq!(board.next_ready().unwrap().id, ready.id);

        board.change_status(ready.id, TaskStatus::Done);
        assert_eq!(board.next_ready().unwrap().id, first.id);
    }

    #[test]
    fn next_ready_accepts_archived_dependencies() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("board.json");
        let mut board = KanbanBoard::new(&path);

        let parent = board.create("Parent".into(), "".into(), None, None, None, None);
        let child = board.create("Child".into(), "".into(), None, None, None, None);
        assert!(board.add_dependency(child.id, parent.id));
        board.change_status(parent.id, TaskStatus::Archived);

        assert_eq!(board.next_ready().unwrap().id, child.id);
    }
}
