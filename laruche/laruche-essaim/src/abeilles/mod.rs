//! Built-in Abeilles (tools) for the Essaim agent.

pub mod browser;
pub mod calendrier;
pub mod clarify;
pub mod git;
pub mod image_search;
pub mod job;
pub mod kanban_next;
pub mod knowledge;
// Re-export plugin loader
pub mod plugins;
pub use plugins::charger_plugins;
pub mod delegation;
pub mod essaim_status;
pub mod forge;
pub use forge::enregistrer_forge;
pub mod execute_code;
pub mod fichiers;
pub mod file_watch;
pub mod lsp;
pub mod math;
pub mod mcp_resources;
pub mod mcp_tool;
pub mod media;
pub mod memoire;
pub mod mixture;
pub mod plan_mode;
pub mod read_extract;
pub mod recherche_fichiers;
pub mod reload_plugins;
pub mod run_script;
pub mod shell;
pub mod spawn_specialist;
pub mod task_complete;
pub mod todo;
pub mod web_deep;
pub mod web_fetch;
pub mod web_recherche;
pub mod worktree;

use crate::abeille::{Abeille, AbeilleRegistry, ContextExecution, NiveauDanger, ResultatAbeille};
use crate::job_queue::JobQueue;
use anyhow::Result;
use async_trait::async_trait;
use laruche_memoire::{MemoireCognitive, SearchOpts};
use std::sync::Arc;

/// Register Kanban tools against the shared board owned by the node runtime.
pub fn enregistrer_kanban(
    registry: &AbeilleRegistry,
    kanban_board: Arc<tokio::sync::RwLock<laruche_kanban::KanbanBoard>>,
) {
    registry.enregistrer(Box::new(kanban_next::KanbanNext {
        kanban_board: kanban_board.clone(),
    }));
    registry.enregistrer(Box::new(kanban_next::KanbanComplete { kanban_board }));
}

/// Register all built-in Abeilles into the registry.
pub fn enregistrer_abeilles_builtin(registry: &AbeilleRegistry) {
    // File operations
    registry.enregistrer(Box::new(fichiers::FileRead));
    registry.enregistrer(Box::new(read_extract::ReadExtract));
    registry.enregistrer(Box::new(fichiers::FileList));
    registry.enregistrer(Box::new(fichiers::FileWrite));
    registry.enregistrer(Box::new(fichiers::FileEdit));
    registry.enregistrer(Box::new(recherche_fichiers::FileSearch));
    // Shell
    registry.enregistrer(Box::new(shell::ShellExec));
    registry.enregistrer(Box::new(execute_code::ExecuteCode));
    registry.enregistrer(Box::new(todo::Todo));
    // Web
    registry.enregistrer(Box::new(web_recherche::WebSearch));
    registry.enregistrer(Box::new(web_fetch::WebFetch));
    registry.enregistrer(Box::new(web_deep::WebDeepSearch));
    registry.enregistrer(Box::new(image_search::ImageSearch));
    registry.enregistrer(Box::new(media::MediaPresent));
    // Math
    registry.enregistrer(Box::new(math::MathEval));
    // Calendar
    registry.enregistrer(Box::new(calendrier::CalendarAdd));
    registry.enregistrer(Box::new(calendrier::CalendarList));
    // Browser
    registry.enregistrer(Box::new(browser::BrowserNavigate));
    registry.enregistrer(Box::new(browser::BrowserScreenshot));
    // Git
    registry.enregistrer(Box::new(git::GitStatus));
    registry.enregistrer(Box::new(git::GitDiff));
    registry.enregistrer(Box::new(git::GitLog));
    registry.enregistrer(Box::new(git::GitCommit));
    // System
    registry.enregistrer(Box::new(essaim_status::SystemInfo));
    // Clarification (pose une question à l'utilisateur)
    registry.enregistrer(Box::new(clarify::Clarify));
    // File watch
    registry.enregistrer(Box::new(file_watch::FileWatch));
    // Worktree
    registry.enregistrer(Box::new(worktree::AbeilleGitWorktreeEnter));
    registry.enregistrer(Box::new(worktree::AbeilleGitWorktreeExit));
    // LSP
    registry.enregistrer(Box::new(lsp::AbeilleLsp));
    // Task completion signal
    registry.enregistrer(Box::new(task_complete::TaskComplete));

    tracing::info!(
        count = registry.noms().len(),
        "Built-in Abeilles registered"
    );
}

/// Register the cognitive-memory abeilles (search + write), backed by any
/// `MemoireCognitive` implementation (sidecar paradigm today, native Rust later).
///
/// Call this from `laruche-node` once a backend is built, e.g.:
/// ```ignore
/// let mem = Arc::new(SidecarBackend::loopback());
/// enregistrer_memoire(&mut registry, mem);
/// ```
pub fn enregistrer_memoire(
    registry: &AbeilleRegistry,
    mem: std::sync::Arc<dyn laruche_memoire::MemoireCognitive>,
) {
    registry.enregistrer(Box::new(memoire::MemoireSearch { mem: mem.clone() }));
    registry.enregistrer(Box::new(memoire::MemoireWrite {
        mem: mem.clone(),
        propose: false,
    }));
    registry.enregistrer(Box::new(memoire::MemoireUpdateItem { mem: mem.clone() }));
    registry.enregistrer(Box::new(memoire::MemoireDelete { mem: mem.clone() }));
    registry.enregistrer(Box::new(memoire::MemoireMoveItem { mem: mem.clone() }));
    registry.enregistrer(Box::new(memoire::MemoireReview { mem: mem.clone() }));
    registry.enregistrer(Box::new(memoire::MemoireListProposed { mem: mem.clone() }));
    registry.enregistrer(Box::new(memoire::MemoireStats { mem: mem.clone() }));
    registry.enregistrer(Box::new(memoire::MemoireMutations { mem: mem.clone() }));
    registry.enregistrer(Box::new(memoire::MemoireTree { mem: mem.clone() }));
    registry.enregistrer(Box::new(memoire::MemoireReadNode { mem: mem.clone() }));
    registry.enregistrer(Box::new(memoire::MemoireGrep { mem: mem.clone() }));
    registry.enregistrer(Box::new(memoire::MemoireDoctor { mem: mem.clone() }));
    registry.enregistrer(Box::new(memoire::MemoireSkillCreate { mem: mem.clone() }));
    registry.enregistrer(Box::new(memoire::MemoireSkillPatch { mem: mem.clone() }));
    registry.enregistrer(Box::new(memoire::MemoireSkillDelete { mem: mem.clone() }));
    registry.enregistrer(Box::new(memoire::MemoireDeleteNode { mem: mem.clone() }));
    registry.enregistrer(Box::new(memoire::MemoireCreateNode { mem: mem.clone() }));
    registry.enregistrer(Box::new(memoire::MemoireUpdateNode { mem: mem.clone() }));
    registry.enregistrer(Box::new(SkillList { mem: mem.clone() }));
    registry.enregistrer(Box::new(SkillView { mem: mem.clone() }));
    registry.enregistrer(Box::new(memoire::MemoireSuggestNodes { mem }));
    tracing::info!("Memoire abeilles registered (cognitive memory wired)");
}

/// Register the delegate abeille (requires registry reference + config).
/// Call this AFTER enregistrer_abeilles_builtin, passing an Arc<AbeilleRegistry>.
pub fn enregistrer_delegation(
    registry: &AbeilleRegistry,
    sub_registry: std::sync::Arc<AbeilleRegistry>,
    config: crate::brain::EssaimConfig,
) {
    registry.enregistrer(Box::new(run_script::RunScript {
        registry: sub_registry.clone(),
    }));
    registry.enregistrer(Box::new(run_script::ToolSearch {
        registry: sub_registry.clone(),
    }));
    registry.enregistrer(Box::new(run_script::ToolCall {
        registry: sub_registry.clone(),
    }));
    registry.enregistrer(Box::new(delegation::Delegate {
        registry: sub_registry.clone(),
        config: config.clone(),
    }));
    registry.enregistrer(Box::new(mixture::MixtureOfAgents { config: config.clone() }));
    registry.enregistrer(Box::new(spawn_specialist::SpawnSpecialist {
        registry: sub_registry.clone(),
        config,
    }));
    tracing::info!("Delegate + run_script + mixture + spawn_specialist abeilles registered");
}

/// Register the JobQueue tools (submit_job, check_job_status).
/// Call this with an Arc<JobQueue> shared across the application.
pub fn enregistrer_jobs(
    registry: &AbeilleRegistry,
    queue: Arc<JobQueue>,
) {
    registry.enregistrer(Box::new(job::SubmitJob {
        queue: queue.clone(),
    }));
    registry.enregistrer(Box::new(job::CheckJobStatus { queue }));
    tracing::info!("JobQueue abeilles registered (submit_job, check_job_status)");
}

pub(crate) fn skill_node_id(name: &str) -> String {
    let trimmed = name.trim();
    if trimmed.starts_with("capacities.skills.") {
        return trimmed.to_string();
    }
    // Tolère un node_id legacy (tools.skills.*) -> le remappe vers capacities.skills.*.
    if let Some(rest) = trimmed.strip_prefix("tools.skills.") {
        return format!("capacities.skills.{rest}");
    }
    let mut slug = trimmed
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect::<String>();
    while slug.contains("__") {
        slug = slug.replace("__", "_");
    }
    let slug = slug.trim_matches('_');
    if slug.is_empty() {
        "capacities.skills".to_string()
    } else {
        format!("capacities.skills.{slug}")
    }
}

pub struct SkillList {
    pub mem: Arc<dyn MemoireCognitive>,
}

#[async_trait]
impl Abeille for SkillList {
    fn nom(&self) -> &str {
        "skill_list"
    }

    fn description(&self) -> &str {
        "Liste les skills OKF stockes dans la memoire cognitive sous capacities.skills.*."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "limit": { "type": "integer", "description": "Nombre max de skills a lister" }
            }
        })
    }

    fn niveau_danger(&self) -> NiveauDanger {
        NiveauDanger::Safe
    }

    async fn executer(
        &self,
        args: serde_json::Value,
        _ctx: &ContextExecution,
    ) -> Result<ResultatAbeille> {
        let limit = args["limit"].as_u64().unwrap_or(50) as usize;
        let root = match self.mem.read_node("capacities.skills").await {
            Ok(root) => root,
            Err(e) => {
                return Ok(ResultatAbeille::err(format!(
                    "Lecture skills impossible: {e}"
                )))
            }
        };
        let mut lines = Vec::new();
        if let Some(children) = root["children"].as_array() {
            for child in children.iter().take(limit) {
                if let Some(id) = child["id"].as_str().or_else(|| child["node_id"].as_str()) {
                    let label = child["label"].as_str().unwrap_or(id);
                    lines.push(format!("- {id} ({label})"));
                }
            }
        }
        if lines.is_empty() {
            let opts = SearchOpts {
                depth: None,
                limit: Some(limit.min(u8::MAX as usize) as u8),
            };
            match self.mem.search("type: skill", opts).await {
                Ok(pack) => {
                    let text = pack.to_prompt_text();
                    if text.trim().is_empty() {
                        Ok(ResultatAbeille::ok(
                            "Aucun skill OKF trouve sous capacities.skills.",
                        ))
                    } else {
                        Ok(ResultatAbeille::ok(format!("Skills trouves:\n{text}")))
                    }
                }
                Err(e) => Ok(ResultatAbeille::err(format!(
                    "Recherche skills impossible: {e}"
                ))),
            }
        } else {
            Ok(ResultatAbeille::ok(format!(
                "Skills OKF sous capacities.skills:\n{}",
                lines.join("\n")
            )))
        }
    }
}

pub struct SkillView {
    pub mem: Arc<dyn MemoireCognitive>,
}

#[async_trait]
impl Abeille for SkillView {
    fn nom(&self) -> &str {
        "skill_view"
    }

    fn description(&self) -> &str {
        "Lit un skill OKF par nom depuis capacities.skills.<nom> et renvoie son Markdown complet."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "name": { "type": "string", "description": "Nom du skill ou node_id capacities.skills.<nom>" }
            },
            "required": ["name"]
        })
    }

    fn niveau_danger(&self) -> NiveauDanger {
        NiveauDanger::Safe
    }

    async fn executer(
        &self,
        args: serde_json::Value,
        _ctx: &ContextExecution,
    ) -> Result<ResultatAbeille> {
        let name = args["name"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("'name' manquant"))?;
        let node_id = skill_node_id(name);
        let node = match self.mem.read_node(&node_id).await {
            Ok(node) => node,
            Err(e) => {
                return Ok(ResultatAbeille::err(format!(
                    "Lecture skill impossible: {e}"
                )))
            }
        };
        let Some(items) = node["items"].as_array() else {
            return Ok(ResultatAbeille::err(format!(
                "Skill introuvable: {node_id}"
            )));
        };
        for item in items.iter().rev() {
            if let Some(content) = item["content"].as_str() {
                if content.contains("type: skill") {
                    return Ok(ResultatAbeille::ok(content.to_string()));
                }
            }
        }
        Ok(ResultatAbeille::err(format!(
            "Aucun document OKF actif dans {node_id}"
        )))
    }
}
