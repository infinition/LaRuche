use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use laruche_essaim::abeille::{Abeille, NiveauDanger, ResultatAbeille};
use laruche_essaim::cron::ScheduledTask;
use laruche_essaim::ContextExecution;

pub struct AbeilleCronCreate {
    pub cron_store: Arc<RwLock<laruche_essaim::cron::CronScheduler>>,
}

#[async_trait]
impl Abeille for AbeilleCronCreate {
    fn nom(&self) -> &str {
        "cron_create"
    }

    fn niveau_danger(&self) -> NiveauDanger {
        NiveauDanger::Safe
    }

    fn description(&self) -> &str {
        "Crée une tâche planifiée (cron ou fire_at) qui exécutera un prompt spécifié."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "name": { "type": "string", "description": "Nom de la tâche" },
                "prompt": { "type": "string", "description": "Le prompt à exécuter" },
                "cron_expr": { "type": "string", "description": "Expression cron (ex: '*/5 * * * *')" },
                "fire_at": { "type": "string", "description": "Date ISO8601 (ex: '2026-12-31T23:59:00Z')" }
            },
            "required": ["name", "prompt"]
        })
    }

    async fn executer(
        &self,
        args: Value,
        _ctx: &ContextExecution,
    ) -> anyhow::Result<ResultatAbeille> {
        let name = args["name"].as_str().unwrap_or("Unnamed task").to_string();
        let prompt = match args["prompt"].as_str() {
            Some(p) => p.to_string(),
            None => {
                return Ok(ResultatAbeille::err(
                    "Le paramètre 'prompt' est requis.".to_string(),
                ))
            }
        };
        let cron_expr = args["cron_expr"].as_str().map(|s| s.to_string());
        let fire_at = args["fire_at"].as_str().and_then(|s| {
            chrono::DateTime::parse_from_rfc3339(s)
                .ok()
                .map(|dt| dt.with_timezone(&chrono::Utc))
        });

        if cron_expr.is_none() && fire_at.is_none() {
            return Ok(ResultatAbeille::err(
                "Vous devez spécifier soit 'cron_expr' soit 'fire_at'.".to_string(),
            ));
        }

        let skills: Vec<String> = args["skills"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        let task = ScheduledTask {
            id: Uuid::new_v4(),
            name,
            prompt,
            cron_expr,
            fire_at,
            channel: None,
            provider: None,
            model: None,
            profile_id: None,
            skills,
            enabled: true,
            created_at: chrono::Utc::now(),
            last_run: None,
            run_count: 0,
        };

        let id = {
            let mut cron = self.cron_store.write().await;
            cron.add(task)
        };

        Ok(ResultatAbeille::ok(format!(
            "Tâche cron créée avec l'ID {}",
            id
        )))
    }
}

pub struct AbeilleCronList {
    pub cron_store: Arc<RwLock<laruche_essaim::cron::CronScheduler>>,
}

#[async_trait]
impl Abeille for AbeilleCronList {
    fn nom(&self) -> &str {
        "cron_list"
    }

    fn niveau_danger(&self) -> NiveauDanger {
        NiveauDanger::Safe
    }

    fn description(&self) -> &str {
        "Liste les tâches planifiées."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {}
        })
    }

    async fn executer(
        &self,
        _args: Value,
        _ctx: &ContextExecution,
    ) -> anyhow::Result<ResultatAbeille> {
        let cron = self.cron_store.read().await;
        let tasks: Vec<Value> = cron
            .list()
            .iter()
            .map(|t| {
                json!({
                    "id": t.id.to_string(),
                    "name": t.name,
                    "cron_expr": t.cron_expr,
                    "fire_at": t.fire_at,
                    "enabled": t.enabled,
                })
            })
            .collect();

        Ok(ResultatAbeille::ok(
            serde_json::to_string(&tasks).unwrap_or_default(),
        ))
    }
}

pub struct AbeilleCronDelete {
    pub cron_store: Arc<RwLock<laruche_essaim::cron::CronScheduler>>,
}

#[async_trait]
impl Abeille for AbeilleCronDelete {
    fn nom(&self) -> &str {
        "cron_delete"
    }

    fn niveau_danger(&self) -> NiveauDanger {
        NiveauDanger::Safe
    }

    fn description(&self) -> &str {
        "Supprime une tâche planifiée par son ID."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "id": { "type": "string" }
            },
            "required": ["id"]
        })
    }

    async fn executer(
        &self,
        args: Value,
        _ctx: &ContextExecution,
    ) -> anyhow::Result<ResultatAbeille> {
        let id_str = match args["id"].as_str() {
            Some(i) => i,
            None => {
                return Ok(ResultatAbeille::err(
                    "Le paramètre 'id' est requis.".to_string(),
                ))
            }
        };

        if let Ok(uuid) = Uuid::parse_str(id_str) {
            let mut cron = self.cron_store.write().await;
            if cron.remove(&uuid) {
                return Ok(ResultatAbeille::ok(format!("Tâche {} supprimée", uuid)));
            }
        }
        Ok(ResultatAbeille::err(
            "Tâche non trouvée ou ID invalide".to_string(),
        ))
    }
}

pub struct AbeilleWatcherCreate {
    pub watcher_store: Arc<RwLock<laruche_watchers::WatchersRegistry>>,
}

#[async_trait]
impl Abeille for AbeilleWatcherCreate {
    fn nom(&self) -> &str {
        "watcher_create"
    }

    fn niveau_danger(&self) -> NiveauDanger {
        NiveauDanger::Safe
    }

    fn description(&self) -> &str {
        "Crée un watcher (surveille un fichier, une URL ou des logs) qui déclenchera un prompt si une condition est remplie."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "watcher_type": { "type": "string", "description": "'file', 'url' ou 'log'" },
                "target": { "type": "string", "description": "Le fichier ou URL à surveiller" },
                "condition": { "type": "string", "description": "Condition pour déclencher" },
                "prompt": { "type": "string", "description": "Le prompt à exécuter" }
            },
            "required": ["name", "watcher_type", "target", "prompt"]
        })
    }

    async fn executer(
        &self,
        args: Value,
        _ctx: &ContextExecution,
    ) -> anyhow::Result<ResultatAbeille> {
        let name = args["name"]
            .as_str()
            .unwrap_or("Unnamed Watcher")
            .to_string();
        let prompt = match args["prompt"].as_str() {
            Some(p) => p.to_string(),
            None => {
                return Ok(ResultatAbeille::err(
                    "Le paramètre 'prompt' est requis.".to_string(),
                ))
            }
        };
        let target = args["target"].as_str().unwrap_or("").to_string();
        let condition = args["condition"].as_str().unwrap_or("").to_string();
        let w_type_str = args["watcher_type"].as_str().unwrap_or("file");

        let watcher_type = match w_type_str {
            "url" => laruche_watchers::WatcherType::Url,
            "log" => laruche_watchers::WatcherType::Log,
            _ => laruche_watchers::WatcherType::File,
        };

        let profile_id = args.get("profile_id").and_then(|v| v.as_str()).map(|s| s.to_string());
        let model = args.get("model").and_then(|v| v.as_str()).map(|s| s.to_string());
        let watcher = laruche_watchers::Watcher {
            id: Uuid::new_v4(),
            name,
            watcher_type,
            target,
            condition,
            prompt,
            active: true,
            created_at: chrono::Utc::now(),
            last_run: None,
            run_count: 0,
            last_state: None,
            profile_id,
            model,
        };

        let id = watcher.id.clone();
        let mut registry = self.watcher_store.write().await;
        registry.add(watcher);

        Ok(ResultatAbeille::ok(format!(
            "Watcher créé avec l'ID {}",
            id
        )))
    }
}

pub struct AbeilleWatcherList {
    pub watcher_store: Arc<RwLock<laruche_watchers::WatchersRegistry>>,
}

#[async_trait]
impl Abeille for AbeilleWatcherList {
    fn nom(&self) -> &str {
        "watcher_list"
    }

    fn niveau_danger(&self) -> NiveauDanger {
        NiveauDanger::Safe
    }

    fn description(&self) -> &str {
        "Liste les watchers."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {}
        })
    }

    async fn executer(
        &self,
        _args: Value,
        _ctx: &ContextExecution,
    ) -> anyhow::Result<ResultatAbeille> {
        let registry = self.watcher_store.read().await;
        let watchers: Vec<Value> = registry
            .list()
            .iter()
            .map(|w| {
                json!({
                    "id": w.id,
                    "name": w.name,
                    "watcher_type": w.watcher_type,
                    "target": w.target,
                    "active": w.active,
                })
            })
            .collect();

        Ok(ResultatAbeille::ok(
            serde_json::to_string(&watchers).unwrap_or_default(),
        ))
    }
}

pub struct AbeilleWatcherDelete {
    pub watcher_store: Arc<RwLock<laruche_watchers::WatchersRegistry>>,
}

#[async_trait]
impl Abeille for AbeilleWatcherDelete {
    fn nom(&self) -> &str {
        "watcher_delete"
    }

    fn niveau_danger(&self) -> NiveauDanger {
        NiveauDanger::Safe
    }

    fn description(&self) -> &str {
        "Supprime un watcher par son ID."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "id": { "type": "string" }
            },
            "required": ["id"]
        })
    }

    async fn executer(
        &self,
        args: Value,
        _ctx: &ContextExecution,
    ) -> anyhow::Result<ResultatAbeille> {
        let id_str = match args["id"].as_str() {
            Some(i) => i,
            None => {
                return Ok(ResultatAbeille::err(
                    "Le paramètre 'id' est requis.".to_string(),
                ))
            }
        };

        if let Ok(uuid) = Uuid::parse_str(id_str) {
            let mut registry = self.watcher_store.write().await;
            if registry.remove(&uuid) {
                return Ok(ResultatAbeille::ok(format!("Watcher {} supprimé", uuid)));
            }
        }
        Ok(ResultatAbeille::err(
            "Watcher non trouvé ou ID invalide".to_string(),
        ))
    }
}

pub struct AbeilleSessionSearch {
    pub sessions_store:
        Arc<tokio::sync::RwLock<std::collections::HashMap<Uuid, laruche_essaim::Session>>>,
}

#[async_trait]
impl Abeille for AbeilleSessionSearch {
    fn nom(&self) -> &str {
        "session_search"
    }

    fn niveau_danger(&self) -> NiveauDanger {
        NiveauDanger::Safe
    }

    fn description(&self) -> &str {
        "Recherche textuelle dans les conversations/sessions passees. Retourne les extraits pertinents."
    }

    fn schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": { "type": "string", "description": "Le texte a chercher" },
                "limit": { "type": "integer", "description": "Nombre maximum de résultats (défaut 20)" }
            },
            "required": ["query"]
        })
    }

    async fn executer(
        &self,
        args: Value,
        _ctx: &ContextExecution,
    ) -> anyhow::Result<ResultatAbeille> {
        let query = match args["query"].as_str() {
            Some(q) => q.to_lowercase(),
            None => {
                return Ok(ResultatAbeille::err(
                    "Le paramètre 'query' est requis.".to_string(),
                ))
            }
        };
        let limit = args["limit"].as_u64().unwrap_or(20) as usize;

        if query.is_empty() {
            return Ok(ResultatAbeille::ok(
                serde_json::to_string(&Vec::<Value>::new()).unwrap_or_default(),
            ));
        }

        let sessions = self.sessions_store.read().await;
        let mut results = Vec::new();

        for session in sessions.values() {
            for msg in &session.messages {
                let text = match msg {
                    laruche_essaim::Message::User(t) | laruche_essaim::Message::Assistant(t) => {
                        t.clone()
                    }
                    laruche_essaim::Message::UserMultimodal { text, .. } => text.clone(),
                    _ => continue,
                };
                if text.to_lowercase().contains(&query) {
                    let preview: String = text.chars().take(200).collect();
                    results.push(serde_json::json!({
                        "session_id": session.id.to_string(),
                        "session_title": session.title,
                        "role": match msg {
                            laruche_essaim::Message::User(_) | laruche_essaim::Message::UserMultimodal { .. } => "user",
                            _ => "assistant",
                        },
                        "preview": preview,
                    }));
                    if results.len() >= limit {
                        break;
                    }
                }
            }
            if results.len() >= limit {
                break;
            }
        }

        Ok(ResultatAbeille::ok(
            serde_json::to_string(&results).unwrap_or_default(),
        ))
    }
}
pub struct AbeilleKanbanCreate {
    pub kanban_board: Arc<RwLock<laruche_kanban::KanbanBoard>>,
}

#[async_trait]
impl Abeille for AbeilleKanbanCreate {
    fn nom(&self) -> &str {
        "kanban_create"
    }

    fn niveau_danger(&self) -> NiveauDanger {
        NiveauDanger::Safe
    }

    fn description(&self) -> &str {
        "Ajoute une nouvelle tâche au Kanban global de LaRuche."
    }

    fn schema(&self) -> Value {
        json!({
            "title": { "type": "string", "description": "Titre court de la tâche" },
            "description": { "type": "string", "description": "Description détaillée de la tâche à accomplir" }
        })
    }

    async fn executer(
        &self,
        arguments: Value,
        _ctx: &ContextExecution,
    ) -> anyhow::Result<ResultatAbeille> {
        let title = arguments
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Sans titre");
        let desc = arguments
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let profile_id = arguments.get("profile_id").and_then(|v| v.as_str()).map(|s| s.to_string());
        let model = arguments.get("model").and_then(|v| v.as_str()).map(|s| s.to_string());
        let task = {
            let mut board = self.kanban_board.write().await;
            board.create(title.to_string(), desc.to_string(), None, profile_id, model)
        };
        Ok(ResultatAbeille::ok(format!(
            "Tâche Kanban créée avec succès. ID: {}",
            task.id
        )))
    }
}

pub struct AbeilleKanbanList {
    pub kanban_board: Arc<RwLock<laruche_kanban::KanbanBoard>>,
}

#[async_trait]
impl Abeille for AbeilleKanbanList {
    fn nom(&self) -> &str {
        "kanban_list"
    }

    fn niveau_danger(&self) -> NiveauDanger {
        NiveauDanger::Safe
    }

    fn description(&self) -> &str {
        "Liste toutes les tâches du Kanban de LaRuche avec leur statut actuel."
    }

    fn schema(&self) -> Value {
        json!({})
    }

    async fn executer(
        &self,
        _arguments: Value,
        _ctx: &ContextExecution,
    ) -> anyhow::Result<ResultatAbeille> {
        let board = self.kanban_board.read().await;
        let tasks = board.list();

        if tasks.is_empty() {
            return Ok(ResultatAbeille::ok("Le Kanban est actuellement vide."));
        }

        let mut output = String::new();
        for task in tasks {
            output.push_str(&format!(
                "- [{:?}] {} (ID: {})\n  {}\n",
                task.status, task.title, task.id, task.description
            ));
        }

        Ok(ResultatAbeille::ok(output))
    }
}
