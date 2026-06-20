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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KanbanBoard {
    tasks: HashMap<Uuid, KanbanTask>,
    #[serde(skip)]
    storage_path: PathBuf,
}

impl KanbanBoard {
    pub fn new(path: &Path) -> Self {
        let mut board = Self {
            tasks: HashMap::new(),
            storage_path: path.to_path_buf(),
        };
        board.load();
        board
    }

    fn load(&mut self) {
        if let Ok(content) = fs::read_to_string(&self.storage_path) {
            if let Ok(tasks) = serde_json::from_str::<HashMap<Uuid, KanbanTask>>(&content) {
                self.tasks = tasks;
            }
        }
    }

    pub fn save(&self) {
        if let Some(parent) = self.storage_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(content) = serde_json::to_string_pretty(&self.tasks) {
            let _ = fs::write(&self.storage_path, content);
        }
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
            profile_id,
            model,
            status: TaskStatus::Todo,
            blocks: Vec::new(),
            blocked_by: Vec::new(),
            created_at: Utc::now(),
            scheduled_at: None,
            result: None,
        };

        self.tasks.insert(task.id, task.clone());
        self.save();
        task
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
            if task.status == TaskStatus::Ready || task.status == TaskStatus::Todo {
                if task.blocked_by.is_empty() {
                    return Some(*id);
                }
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

        let t1 = board.create("Task 1".into(), "Desc 1".into(), None, None, None);
        let t2 = board.create("Task 2".into(), "Desc 2".into(), None, None, None);

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

        let t1 = board.create("A".into(), "B".into(), Some("my_key".into()), None, None);
        let t2 = board.create("C".into(), "D".into(), Some("my_key".into()), None, None);

        assert_eq!(t1.id, t2.id);
        assert_eq!(board.list().len(), 1);
    }

    #[test]
    fn next_ready_prioritizes_ready_then_unblocked_tasks() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("board.json");
        let mut board = KanbanBoard::new(&path);

        let first = board.create("First".into(), "".into(), None, None, None);
        let ready = board.create("Ready".into(), "".into(), None, None, None);
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

        let parent = board.create("Parent".into(), "".into(), None, None, None);
        let child = board.create("Child".into(), "".into(), None, None, None);
        assert!(board.add_dependency(child.id, parent.id));
        board.change_status(parent.id, TaskStatus::Archived);

        assert_eq!(board.next_ready().unwrap().id, child.id);
    }
}
