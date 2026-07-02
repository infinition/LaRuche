use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use laruche_essaim::abeille::{Abeille, NiveauDanger, ResultatAbeille};
use laruche_essaim::cron::ScheduledTask;
use laruche_essaim::ContextExecution;

/// Agent tool: send a direct message to ANOTHER LaRuche instance (or its user) over
/// the mesh, by its `laruche` ID. Reuses the local /api/mesh/send endpoint (peer resolution +
/// inter-instance POST). Outbound: approval required.
pub struct AbeilleMeshSend;

#[async_trait]
impl Abeille for AbeilleMeshSend {
    fn nom(&self) -> &str {
        "mesh_send"
    }
    fn niveau_danger(&self) -> NiveauDanger {
        NiveauDanger::NeedsApproval
    }
    fn description(&self) -> &str {
        "Send a direct message to another LaRuche instance (or its user) over the mesh, \
         identified by its laruche ID. List available recipients via the Messages / mesh peers button. \
         OUTBOUND: requires approval."
    }
    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "to_id": { "type": "string", "description": "Laruche ID of the recipient" },
                "text": { "type": "string", "description": "Message to send" }
            },
            "required": ["to_id", "text"]
        })
    }
    async fn executer(&self, args: Value, _ctx: &ContextExecution) -> anyhow::Result<ResultatAbeille> {
        let to_id = args["to_id"].as_str().unwrap_or("").to_string();
        let text = args["text"].as_str().unwrap_or("").to_string();
        if to_id.is_empty() || text.trim().is_empty() {
            return Ok(ResultatAbeille::err(
                "'to_id' and 'text' are required.".to_string(),
            ));
        }
        let client = reqwest::Client::new();
        let res = client
            .post("http://127.0.0.1:8419/api/mesh/send")
            .json(&json!({ "to_id": to_id, "text": text }))
            .send()
            .await;
        match res {
            Ok(r) if r.status().is_success() => {
                Ok(ResultatAbeille::ok(format!("Message sent to `{to_id}`.")))
            }
            Ok(r) => Ok(ResultatAbeille::err(format!("Send failed (HTTP {}).", r.status()))),
            Err(e) => Ok(ResultatAbeille::err(format!("Send failed: {e}"))),
        }
    }
}

pub struct AbeilleCronCreate {
    pub cron_store: Arc<RwLock<laruche_essaim::cron::CronScheduler>>,
}

#[async_trait]
impl Abeille for AbeilleCronCreate {
    fn nom(&self) -> &str {
        "cron_create"
    }

    fn niveau_danger(&self) -> NiveauDanger {
        NiveauDanger::NeedsApproval
    }

    fn description(&self) -> &str {
        "Create a scheduled task (cron or one-shot fire_at) that runs a given prompt."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "name": { "type": "string", "description": "Task name" },
                "prompt": { "type": "string", "description": "Prompt to execute" },
                "cron_expr": { "type": "string", "description": "Cron expression (e.g. '*/5 * * * *')" },
                "fire_at": { "type": "string", "description": "ISO8601 datetime (e.g. '2026-12-31T23:59:00Z')" }
            },
            "required": ["name", "prompt"]
        })
    }

    async fn executer(
        &self,
        args: Value,
        ctx: &ContextExecution,
    ) -> anyhow::Result<ResultatAbeille> {
        let name = args["name"].as_str().unwrap_or("Unnamed task").to_string();
        let prompt = match args["prompt"].as_str() {
            Some(p) => p.to_string(),
            None => {
                return Ok(ResultatAbeille::err(
                    "Parameter 'prompt' is required.".to_string(),
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
                "Specify either 'cron_expr' or 'fire_at'.".to_string(),
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

        // Origin channel (e.g. `telegram:12345`): the recurring task replies where it was requested.
        // The agent can force another channel via the `channel` argument.
        let channel = args["channel"]
            .as_str()
            .map(|s| s.to_string())
            .or_else(|| ctx.channel.clone());

        let log_name = name.clone();
        let task = ScheduledTask {
            id: Uuid::new_v4(),
            name,
            prompt,
            cron_expr,
            fire_at,
            channel,
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
        laruche_essaim::feed_journal::record(
            "LaRuche",
            "cron",
            "created the scheduled task",
            log_name,
            chrono::Utc::now(),
        );

        Ok(ResultatAbeille::ok(format!(
            "Cron task created with ID {}",
            id
        )))
    }
}

pub struct AbeilleCronList {
    pub cron_store: Arc<RwLock<laruche_essaim::cron::CronScheduler>>,
    pub missions: Arc<RwLock<crate::missions::MissionStore>>,
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
        "List everything scheduled: cron tasks AND mission cadences (kind='mission', managed via mission_* tools)."
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
        let mut tasks: Vec<Value> = cron
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
        drop(cron);
        // Mission cadences are schedules too: list them here so "what is
        // scheduled?" gets ONE truthful answer. Managed via mission_* tools.
        // Finished missions have no schedule anymore.
        let store = self.missions.read().await;
        for m in store
            .list()
            .iter()
            .filter(|m| m.cadence.is_some() && m.status != "done")
        {
            tasks.push(json!({
                "id": format!("mission:{}", m.slug),
                "name": format!("Mission: {}", m.objective),
                "cron_expr": m.cadence,
                "enabled": m.status == "active",
                "kind": "mission",
            }));
        }

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
        "Delete a scheduled task by ID."
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
                    "Parameter 'id' is required.".to_string(),
                ))
            }
        };

        if let Ok(uuid) = Uuid::parse_str(id_str) {
            let mut cron = self.cron_store.write().await;
            if cron.remove(&uuid) {
                return Ok(ResultatAbeille::ok(format!("Task {} deleted", uuid)));
            }
        }
        Ok(ResultatAbeille::err(
            "Task not found or invalid ID".to_string(),
        ))
    }
}

pub struct AbeilleMissionList {
    pub missions: Arc<RwLock<crate::missions::MissionStore>>,
}

#[async_trait]
impl Abeille for AbeilleMissionList {
    fn nom(&self) -> &str {
        "mission_list"
    }

    fn niveau_danger(&self) -> NiveauDanger {
        NiveauDanger::Safe
    }

    fn description(&self) -> &str {
        "List the long-running missions (objective, cadence, status, iterations, last run)."
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
        let store = self.missions.read().await;
        let items: Vec<Value> = store
            .list()
            .iter()
            .map(|m| {
                json!({
                    "slug": m.slug,
                    "objective": m.objective,
                    "cadence": m.cadence,
                    "status": m.status,
                    "iterations": m.iterations,
                    "last_run": m.last_run,
                    "channel": m.channel,
                })
            })
            .collect();
        Ok(ResultatAbeille::ok(
            serde_json::to_string(&items).unwrap_or_default(),
        ))
    }
}

pub struct AbeilleMissionCreate {
    pub missions: Arc<RwLock<crate::missions::MissionStore>>,
}

#[async_trait]
impl Abeille for AbeilleMissionCreate {
    fn nom(&self) -> &str {
        "mission_create"
    }

    fn niveau_danger(&self) -> NiveauDanger {
        NiveauDanger::NeedsApproval
    }

    fn description(&self) -> &str {
        "Create a long-running mission. `objective` = what to accomplish across iterations; \
         `cadence` = optional cron expression (e.g. '0 9 * * *') for automatic iterations, \
         omit it for a manual mission; `channel` = optional delivery channel for reports."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "objective": { "type": "string", "description": "Mission objective" },
                "cadence": { "type": "string", "description": "Cron expression for automatic iterations (optional)" },
                "channel": { "type": "string", "description": "Delivery channel, e.g. telegram:123 (optional)" }
            },
            "required": ["objective"]
        })
    }

    async fn executer(
        &self,
        args: Value,
        _ctx: &ContextExecution,
    ) -> anyhow::Result<ResultatAbeille> {
        let objective = match args["objective"].as_str().map(str::trim) {
            Some(o) if !o.is_empty() => o.to_string(),
            _ => {
                return Ok(ResultatAbeille::err(
                    "Parameter 'objective' is required.".to_string(),
                ))
            }
        };
        let cadence = args["cadence"]
            .as_str()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from);
        let channel = args["channel"]
            .as_str()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from);
        let slug = crate::missions::slugify(&objective);
        {
            let mut store = self.missions.write().await;
            if store.get(&slug).is_some() {
                return Ok(ResultatAbeille::err(format!(
                    "A mission with slug '{slug}' already exists (mission_list to inspect it)."
                )));
            }
            store.upsert(crate::missions::Mission {
                slug: slug.clone(),
                objective: objective.clone(),
                cadence: cadence.clone(),
                profile_id: None,
                model: None,
                channel,
                status: "active".to_string(),
                iterations: 0,
                last_run: None,
                created_at: chrono::Utc::now().to_rfc3339(),
            });
        }
        laruche_essaim::feed_journal::record(
            "LaRuche",
            "mission",
            "created the mission",
            &objective,
            chrono::Utc::now(),
        );
        Ok(ResultatAbeille::ok(format!(
            "Mission '{slug}' created{}",
            match cadence {
                Some(c) => format!(" (cadence: {c})"),
                None => " (manual, no cadence)".to_string(),
            }
        )))
    }
}

pub struct AbeilleMissionDelete {
    pub missions: Arc<RwLock<crate::missions::MissionStore>>,
}

#[async_trait]
impl Abeille for AbeilleMissionDelete {
    fn nom(&self) -> &str {
        "mission_delete"
    }

    fn niveau_danger(&self) -> NiveauDanger {
        NiveauDanger::Safe
    }

    fn description(&self) -> &str {
        "Delete a mission by slug (see mission_list). Its memory dossier (missions.<slug>) is kept."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "slug": { "type": "string" }
            },
            "required": ["slug"]
        })
    }

    async fn executer(
        &self,
        args: Value,
        _ctx: &ContextExecution,
    ) -> anyhow::Result<ResultatAbeille> {
        let slug = match args["slug"].as_str().map(str::trim) {
            Some(s) if !s.is_empty() => s,
            _ => {
                return Ok(ResultatAbeille::err(
                    "Parameter 'slug' is required.".to_string(),
                ))
            }
        };
        let removed = {
            let mut store = self.missions.write().await;
            store.remove(slug)
        };
        if removed {
            laruche_essaim::feed_journal::record(
                "LaRuche",
                "mission",
                "deleted the mission",
                slug,
                chrono::Utc::now(),
            );
            Ok(ResultatAbeille::ok(format!("Mission '{slug}' deleted")))
        } else {
            Ok(ResultatAbeille::err(format!(
                "Unknown mission: '{slug}' (mission_list to see the slugs)."
            )))
        }
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
        NiveauDanger::NeedsApproval
    }

    fn description(&self) -> &str {
        "Create a watcher that monitors a file, URL, or log stream and fires a prompt when a condition is met."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "watcher_type": { "type": "string", "description": "'file', 'url', or 'log'" },
                "target": { "type": "string", "description": "File path or URL to watch" },
                "condition": { "type": "string", "description": "LEGACY natural-language condition (LLM gate at every event). PREFER 'regles' below: deterministic, free at runtime. For 'log': plain substring the new lines must contain." },
                "regles": { "type": "object", "description": "COMPILED condition tree (preferred): deterministic predicates evaluated at every poll for free. Read the 'watcher-architecte' skill (skill_view) for the full op list and examples. Ops: et/ou/non, jour_semaine{jours:[mar,jeu]}, heure_entre{de,a}, plage_date{du,au}, apparu, supprime, modifie, contenu_change, est_down, down_depuis_min{minutes}, retour_en_ligne, contient{motif}, taille_depasse_mo{mo}, status_http{codes}, llm_check{question} (the ONLY op that costs an LLM call, after the deterministic prefix passed). A state rule (down_depuis_min) re-fires every cooldown while true." },
                "prompt": { "type": "string", "description": "Prompt to run when triggered" },
                "interval_secs": { "type": "integer", "description": "Poll interval in seconds (default: 10 for file/log, 60 for url; floor 5)" },
                "cooldown_secs": { "type": "integer", "description": "Minimum seconds between two fires (default: 900 for url, 0 otherwise)" },
                "sustained": { "type": "boolean", "description": "LEGACY mode only (ignored with 'regles'): keep re-firing every cooldown while the situation lasts. Requires a condition." }
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
                    "Parameter 'prompt' is required.".to_string(),
                ))
            }
        };
        let target = args["target"].as_str().unwrap_or("").to_string();
        let condition = args["condition"].as_str().unwrap_or("").to_string();
        let regles = match args.get("regles") {
            None | Some(Value::Null) => None,
            Some(v) => match serde_json::from_value::<laruche_watchers::Regle>(v.clone()) {
                Ok(r) => Some(r),
                Err(e) => {
                    return Ok(ResultatAbeille::err(format!(
                        "Invalid 'regles' tree: {e}. Read the watcher-architecte skill \
                         (skill_view) for the op list and examples."
                    )))
                }
            },
        };
        let w_type_str = args["watcher_type"].as_str().unwrap_or("file");

        let watcher_type = match w_type_str {
            "url" => laruche_watchers::WatcherType::Url,
            "log" => laruche_watchers::WatcherType::Log,
            _ => laruche_watchers::WatcherType::File,
        };

        let profile_id = args
            .get("profile_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let model = args
            .get("model")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let log_name = name.clone();
        let watcher = laruche_watchers::Watcher {
            id: Uuid::new_v4(),
            name,
            watcher_type,
            target,
            condition,
            prompt,
            channel: _ctx.channel.clone(), // inherits the agent's origin channel
            active: true,
            created_at: chrono::Utc::now(),
            last_run: None,
            run_count: 0,
            last_state: None,
            profile_id,
            model,
            interval_secs: args.get("interval_secs").and_then(|v| v.as_u64()),
            cooldown_secs: args.get("cooldown_secs").and_then(|v| v.as_u64()),
            sustained: args
                .get("sustained")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            regles,
        };

        let id = watcher.id.clone();
        let mut registry = self.watcher_store.write().await;
        registry.add(watcher);
        laruche_essaim::feed_journal::record(
            "LaRuche",
            "watcher",
            "created the watcher",
            log_name,
            chrono::Utc::now(),
        );

        Ok(ResultatAbeille::ok(format!(
            "Watcher created with ID {}",
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
        "List all active watchers."
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
        "Delete a watcher by ID."
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
                    "Parameter 'id' is required.".to_string(),
                ))
            }
        };

        if let Ok(uuid) = Uuid::parse_str(id_str) {
            let mut registry = self.watcher_store.write().await;
            if registry.remove(&uuid) {
                return Ok(ResultatAbeille::ok(format!("Watcher {} deleted", uuid)));
            }
        }
        Ok(ResultatAbeille::err(
            "Watcher not found or invalid ID".to_string(),
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
        "Full-text search across past sessions. Returns matching excerpts."
    }

    fn schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": { "type": "string", "description": "Text to search for" },
                "limit": { "type": "integer", "description": "Max results to return (default 20)" }
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
                    "Parameter 'query' is required.".to_string(),
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
        "Add a new task to the LaRuche global Kanban board."
    }

    fn schema(&self) -> Value {
        json!({
            "title": { "type": "string", "description": "Short task title" },
            "description": { "type": "string", "description": "Detailed description of the task to accomplish" }
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
            .unwrap_or("Untitled");
        let desc = arguments
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let profile_id = arguments
            .get("profile_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let model = arguments
            .get("model")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let task = {
            let mut board = self.kanban_board.write().await;
            board.create(
                title.to_string(),
                desc.to_string(),
                None,
                profile_id,
                model,
                _ctx.channel.clone(), // inherits the agent's origin channel
            )
        };
        laruche_essaim::feed_journal::record(
            "LaRuche",
            "kanban",
            "created the kanban task",
            title.to_string(),
            chrono::Utc::now(),
        );
        Ok(ResultatAbeille::ok(format!(
            "Kanban task created. ID: {}",
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
        "List all LaRuche Kanban tasks with their current status."
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
            return Ok(ResultatAbeille::ok("The Kanban board is currently empty."));
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
