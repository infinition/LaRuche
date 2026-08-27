//! Built-in Abeilles (tools) for the Essaim agent.

pub mod browser;
pub mod navigateur;
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
pub mod finding;
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
pub mod research_mode;
pub mod run_script;
pub mod shell;
pub mod spawn_specialist;
pub mod task_complete;
pub mod todo;
pub mod web_deep;
pub mod web_discover;
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
    registry.enregistrer(Box::new(web_discover::WebDiscover));
    registry.enregistrer(Box::new(image_search::ImageSearch));
    registry.enregistrer(Box::new(media::MediaPresent));
    // Math
    registry.enregistrer(Box::new(math::MathEval));
    // Calendar
    registry.enregistrer(Box::new(calendrier::CalendarAdd));
    registry.enregistrer(Box::new(calendrier::CalendarList));
    // Browser
    // Superseded by `browser`, which keeps one CDP session open instead of
    // spawning a throwaway Chrome per call. The old module is kept compiled so
    // re-registering it stays a one-line change.
    registry.enregistrer(Box::new(navigateur::Browser));
    // Git
    registry.enregistrer(Box::new(git::GitStatus));
    registry.enregistrer(Box::new(git::GitDiff));
    registry.enregistrer(Box::new(git::GitLog));
    registry.enregistrer(Box::new(git::GitCommit));
    // System
    registry.enregistrer(Box::new(essaim_status::SystemInfo));
    // Clarification (ask the user a question)
    registry.enregistrer(Box::new(clarify::Clarify));
    // Deep-research self-declaration (intercepted by the butinage engine)
    registry.enregistrer(Box::new(research_mode::ResearchMode));
    // Findings ledger (intercepted by the butinage engine)
    registry.enregistrer(Box::new(finding::Finding));
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

/// Register the delegate abeille (requires registry references + config).
/// Call this AFTER enregistrer_abeilles_builtin.
///
/// Two registries on purpose:
/// - `full_registry`: the LIVE main registry. tool_call / tool_search / run_script
///   must see every tool that will ever register on it (node-local crons/watchers,
///   memory, plugins, MCP loaded in the background...). They carry their own
///   by-name recursion guards. Wiring them on a snapshot registry made
///   `tool_call(tool="cron_list")` fail with "Unknown tool" while cron_list
///   existed on the main registry.
/// - `sub_registry`: the reduced toolset handed to spawned scouts (delegate /
///   spawn_specialist), which must NOT be able to delegate recursively.
pub fn enregistrer_delegation(
    registry: &AbeilleRegistry,
    full_registry: std::sync::Arc<AbeilleRegistry>,
    sub_registry: std::sync::Arc<AbeilleRegistry>,
    config: crate::brain::EssaimConfig,
) {
    registry.enregistrer(Box::new(run_script::RunScript {
        registry: full_registry.clone(),
    }));
    registry.enregistrer(Box::new(run_script::ToolSearch {
        registry: full_registry.clone(),
    }));
    registry.enregistrer(Box::new(run_script::ToolCall {
        registry: full_registry,
    }));
    registry.enregistrer(Box::new(delegation::Delegate {
        registry: sub_registry.clone(),
        config: config.clone(),
    }));
    registry.enregistrer(Box::new(mixture::MixtureOfAgents { config: config.clone() }));
    registry.enregistrer(Box::new(spawn_specialist::SpawnSpecialist {
        registry: sub_registry,
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

/// Canonical node id for a skill name.
///
/// HYPHENS ARE PRESERVED. The disk sync indexes a skill under its folder name
/// verbatim (`skills/watcher-architecte` -> `capacities.skills.watcher-architecte`),
/// so mangling `-` into `_` here made `skill_view("watcher-architecte")` look up a
/// node that does not exist, for 40 of the 73 skills shipped. The symptom was
/// silent: the tool answered "No active OKF document found", the agent assumed the
/// skill was missing, and improvised. Whatever the writer stores is the truth.
pub fn skill_node_id(name: &str) -> String {
    let trimmed = name.trim();
    if trimmed.starts_with("capacities.skills.") {
        return trimmed.to_string();
    }
    // Tolerate a legacy node_id (tools.skills.*) -> remap it to capacities.skills.*.
    if let Some(rest) = trimmed.strip_prefix("tools.skills.") {
        return format!("capacities.skills.{rest}");
    }
    // Slug computed by laruche-skills, the crate that also WRITES skills, so the
    // reader and the writer can no longer disagree.
    laruche_skills::skill_node_id(trimmed)
}

/// Node ids to try, in order, when READING a skill by name.
///
/// Belt and braces on top of `skill_node_id`: rows written before the hyphen fix
/// (and any skill a human names with the other separator) still resolve, without
/// migrating a single row. `-` and `_` are interchangeable at lookup time.
pub fn skill_node_id_candidates(name: &str) -> Vec<String> {
    let base = skill_node_id(name);
    // The "capacities.skills." prefix holds only letters and dots, so swapping
    // separators can never damage it: only the slug changes.
    let mut out = vec![base.clone()];
    for variante in [base.replace('-', "_"), base.replace('_', "-")] {
        if !out.contains(&variante) {
            out.push(variante);
        }
    }
    out
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
        "List all OKF skills stored in cognitive memory under capacities.skills.*."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "limit": { "type": "integer", "description": "Max number of skills to return" }
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
                    "Failed to read skills: {e}"
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
                sans_trace: false,
            };
            match self.mem.search("type: skill", opts).await {
                Ok(pack) => {
                    let text = pack.to_prompt_text();
                    if text.trim().is_empty() {
                        Ok(ResultatAbeille::ok(
                            "No OKF skill found under capacities.skills.",
                        ))
                    } else {
                        Ok(ResultatAbeille::ok(format!("Skills found:\n{text}")))
                    }
                }
                Err(e) => Ok(ResultatAbeille::err(format!(
                    "Skill search failed: {e}"
                ))),
            }
        } else {
            Ok(ResultatAbeille::ok(format!(
                "OKF skills under capacities.skills:\n{}",
                lines.join("\n")
            )))
        }
    }
}

/// Is `nom` reachable as an executable on PATH? Pure filesystem, no process spawn.
fn sur_le_path(nom: &str) -> bool {
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };
    // Windows resolves a bare name through PATHEXT; the usual suspects are enough.
    let suffixes: &[&str] = if cfg!(windows) {
        &["", ".exe", ".cmd", ".bat", ".ps1"]
    } else {
        &[""]
    };
    std::env::split_paths(&path).any(|dir| {
        suffixes
            .iter()
            .any(|s| dir.join(format!("{nom}{s}")).is_file())
    })
}

/// Report on the `prerequisites.commands` a skill declares, appended to its body.
///
/// Eleven of the shipped skills declare their prerequisites and NOTHING read them:
/// pure decoration. The cost of that gap, observed: asked to turn on a light, the
/// agent ran `openhue get room`, got exit 1, and spent five commands hunting the
/// binary across the disk, never once considering that a tool absent from PATH is
/// an installation to perform rather than a mystery to solve. The skill knew all
/// along, in its own frontmatter.
fn etat_prerequis(contenu: &str) -> String {
    let Some(bloc) = contenu.split("prerequisites:").nth(1) else {
        return String::new();
    };
    let Some(ligne) = bloc.lines().find(|l| l.trim_start().starts_with("commands:")) else {
        return String::new();
    };
    let noms: Vec<String> = ligne
        .split_once('[')
        .and_then(|(_, r)| r.split_once(']'))
        .map(|(inner, _)| {
            inner
                .split(',')
                .map(|s| s.trim().trim_matches(['"', '\'']).to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default();
    if noms.is_empty() {
        return String::new();
    }

    let manquants: Vec<&String> = noms.iter().filter(|n| !sur_le_path(n)).collect();
    if manquants.is_empty() {
        return format!(
            "\n\n---\n## Prerequisites check\nAll declared commands are on PATH: {}.\n",
            noms.join(", ")
        );
    }
    format!(
        "\n\n---\n## Prerequisites check\nNOT ON PATH: {}.\n\
         This skill cannot work until they are installed. Do it NOW with the Install \
         section above, then verify, then carry on with the task. Do not go looking for \
         the binary elsewhere: absent from PATH means absent. A leftover config file \
         proves nothing, it outlives the program it configured.\n",
        manquants
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    )
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
        "Read a named OKF skill from capacities.skills.<name> and return its full Markdown content."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "name": { "type": "string", "description": "Skill name or full node_id (capacities.skills.<name>)" }
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
            .ok_or_else(|| anyhow::anyhow!("'name' is required"))?;

        // Try every spelling of the slug before giving up: the writer and the
        // reader disagreed on `-` vs `_` for a long time, and rows from both eras
        // coexist in the same database.
        let candidats = skill_node_id_candidates(name);
        let mut derniere_erreur: Option<String> = None;
        for node_id in &candidats {
            let node = match self.mem.read_node(node_id).await {
                Ok(node) => node,
                Err(e) => {
                    derniere_erreur = Some(e.to_string());
                    continue;
                }
            };
            let Some(items) = node["items"].as_array() else {
                continue;
            };
            for item in items.iter().rev() {
                if let Some(content) = item["content"].as_str() {
                    if content.contains("type: skill") {
                        return Ok(ResultatAbeille::ok(format!(
                            "{content}{}",
                            etat_prerequis(content)
                        )));
                    }
                }
            }
        }
        if let Some(e) = derniere_erreur {
            return Ok(ResultatAbeille::err(format!("Failed to read skill: {e}")));
        }
        Ok(ResultatAbeille::err(format!(
            "No skill named '{name}'. Tried {}. Use skill_list to see what exists.",
            candidats.join(", ")
        )))
    }
}

#[cfg(test)]
mod slug_tests {
    use super::*;

    /// A skill that declares its prerequisites must say when they are missing.
    ///
    /// Asked to turn on a light, the agent ran the CLI, got exit 1, then spent five
    /// commands hunting the binary across the disk. The skill declared
    /// `prerequisites: commands: [openhue]` in its own frontmatter and nothing read
    /// it: eleven of the shipped skills carry that field, purely decorative.
    #[test]
    fn les_prerequis_declares_sont_verifies() {
        let skill = "---
type: skill
name: openhue
prerequisites:
  commands: [openhue_absent_xyz]
---
# body";
        let rapport = etat_prerequis(skill);
        assert!(rapport.contains("NOT ON PATH"), "a missing command must be flagged: {rapport}");
        assert!(rapport.contains("openhue_absent_xyz"));
        assert!(rapport.contains("Install"), "and point at the fix");

        // A command that certainly exists is reported as satisfied.
        let present = if cfg!(windows) { "cmd" } else { "sh" };
        let ok = format!("---
type: skill
prerequisites:
  commands: [{present}]
---
# body");
        assert!(etat_prerequis(&ok).contains("All declared commands are on PATH"));

        // A skill without the field stays untouched: no noise added.
        assert_eq!(etat_prerequis("---
type: skill
name: x
---
# body"), "");
    }

    /// A hyphen in a skill folder must survive the round trip. Mangling it into
    /// `_` made `skill_view("watcher-architecte")` read a node that nothing ever
    /// wrote, for 40 of the 73 shipped skills. The tool then reported the skill
    /// missing and the agent improvised a watcher that could never fire.
    #[test]
    fn le_tiret_survit_a_la_normalisation() {
        assert_eq!(
            skill_node_id("watcher-architecte"),
            "capacities.skills.watcher-architecte"
        );
        // The writer of record agrees, which is the whole point.
        assert_eq!(
            skill_node_id("watcher-architecte"),
            laruche_skills::skill_node_id("watcher-architecte")
        );
    }

    #[test]
    fn un_node_id_complet_passe_tel_quel() {
        assert_eq!(
            skill_node_id("capacities.skills.watcher-architecte"),
            "capacities.skills.watcher-architecte"
        );
        assert_eq!(
            skill_node_id("tools.skills.legacy-name"),
            "capacities.skills.legacy-name"
        );
    }

    /// Rows written before the fix used `_`. Both spellings must resolve, so no
    /// database migration is needed.
    #[test]
    fn les_deux_orthographes_sont_essayees() {
        let c = skill_node_id_candidates("watcher-architecte");
        assert!(c.contains(&"capacities.skills.watcher-architecte".to_string()));
        assert!(c.contains(&"capacities.skills.watcher_architecte".to_string()));

        let c2 = skill_node_id_candidates("watcher_architecte");
        assert!(c2.contains(&"capacities.skills.watcher_architecte".to_string()));
        assert!(c2.contains(&"capacities.skills.watcher-architecte".to_string()));

        // The prefix itself is never touched by the separator swap.
        for id in skill_node_id_candidates("a_b-c") {
            assert!(id.starts_with("capacities.skills."), "prefix damaged: {id}");
        }
    }

    /// INTEGRATION GUARD. Walks the real `skills/` directory and checks that every
    /// folder the disk sync will write is a node id the reader will actually look
    /// for. This is the test that would have caught the whole cascade: 40 skills
    /// written under `watcher-architecte` and read under `watcher_architecte`, with
    /// `skill_view` answering "not found" and the agent improvising.
    ///
    /// Never compress something whose decompression path is not tested.
    #[test]
    fn chaque_dossier_de_skill_est_atteignable_par_le_lecteur() {
        let racine = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("skills");
        let Ok(entrees) = std::fs::read_dir(&racine) else {
            return; // no skills/ in this checkout: nothing to guard
        };
        let mut verifies = 0usize;
        for e in entrees.flatten() {
            if !e.path().is_dir() {
                continue;
            }
            if !e.path().join("SKILL.md").exists() {
                continue;
            }
            let dossier = e.file_name().to_string_lossy().to_string();
            // What sync_skills_disk_to_sql writes.
            let ecrit = laruche_skills::skill_node_id(&dossier);
            // What skill_view will try.
            let cherches = skill_node_id_candidates(&dossier);
            assert!(
                cherches.contains(&ecrit),
                "skill `{dossier}` is written as `{ecrit}` but the reader only tries {cherches:?}"
            );
            verifies += 1;
        }
        assert!(verifies > 0, "no skill checked: the guard would be vacuous");
    }
}
