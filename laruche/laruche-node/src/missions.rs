//! Missions: long-running, objective-driven work/research ("La Reine").
//!
//! Operational metadata lives here (`missions.json`); capitalized KNOWLEDGE lives in the
//! cognitive map under `missions.<slug>` (`.findings`, `.questions`...). Each iteration advances
//! the mission: the agent reads the state from memory, performs the next step, writes its findings.
//! (MVP: manual run or cron; dream/synthesis/skills = later iterations.)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mission {
    pub slug: String,
    pub objective: String,
    /// Cron cadence (e.g. "0 9 * * 1" = Monday 9am); None = manual only.
    #[serde(default)]
    pub cadence: Option<String>,
    /// Provider profile to use for iterations (resolves provider/model/key). None = default.
    #[serde(default)]
    pub profile_id: Option<String>,
    /// Explicit model (overrides the profile). None = profile/default model.
    #[serde(default)]
    pub model: Option<String>,
    /// Delivery channel for the iteration report (e.g. `telegram:123`). None = background work
    /// (result written to memory `missions.<slug>` only, no notification).
    #[serde(default)]
    pub channel: Option<String>,
    /// "active" | "paused" | "done".
    #[serde(default = "default_status")]
    pub status: String,
    #[serde(default)]
    pub iterations: u32,
    #[serde(default)]
    pub last_run: Option<String>,
    #[serde(default)]
    pub created_at: String,
}

fn default_status() -> String {
    "active".to_string()
}

#[derive(Debug, Default)]
pub struct MissionStore {
    missions: HashMap<String, Mission>,
    path: PathBuf,
}

impl MissionStore {
    pub fn new(path: &Path) -> Self {
        let mut store = Self {
            missions: HashMap::new(),
            path: path.to_path_buf(),
        };
        if let Ok(raw) = std::fs::read_to_string(path) {
            if let Ok(list) = serde_json::from_str::<Vec<Mission>>(&raw) {
                for m in list {
                    store.missions.insert(m.slug.clone(), m);
                }
            }
        }
        store
    }

    pub fn save(&self) {
        let list: Vec<&Mission> = self.missions.values().collect();
        if let Ok(json) = serde_json::to_string_pretty(&list) {
            let _ = std::fs::write(&self.path, json);
        }
    }

    pub fn list(&self) -> Vec<Mission> {
        let mut v: Vec<Mission> = self.missions.values().cloned().collect();
        v.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        v
    }

    pub fn get(&self, slug: &str) -> Option<Mission> {
        self.missions.get(slug).cloned()
    }

    pub fn upsert(&mut self, m: Mission) {
        self.missions.insert(m.slug.clone(), m);
        self.save();
    }

    pub fn remove(&mut self, slug: &str) -> bool {
        let existed = self.missions.remove(slug).is_some();
        if existed {
            self.save();
        }
        existed
    }

    pub fn mark_run(&mut self, slug: &str, when: String) {
        if let Some(m) = self.missions.get_mut(slug) {
            m.iterations += 1;
            m.last_run = Some(when);
        }
        self.save();
    }
}

/// Slugifies a text into a node identifier (`missions.<slug>`).
pub fn slugify(s: &str) -> String {
    let mut slug: String = s
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    while slug.contains("__") {
        slug = slug.replace("__", "_");
    }
    slug.trim_matches('_').chars().take(40).collect()
}

/// Builds the prompt for a mission iteration: the agent reads the already-capitalized state then
/// advances one step and writes its findings under `missions.<slug>`.
pub fn prompt_iteration(mission: &Mission, etat_actuel: &str) -> String {
    let node_id = format!("missions.{}", mission.slug);
    let etat = if etat_actuel.trim().is_empty() {
        "(nothing yet - this is the first iteration)".to_string()
    } else {
        etat_actuel.to_string()
    };
    format!(
        "You are advancing a LONG-RUNNING research MISSION (you will resume it at each iteration).\n\
         OBJECTIVE: {objective}\n\
         Iteration #{iter}.\n\n\
         Already capitalized in memory under `{node}`:\n{etat}\n\n\
         If the state above contains unresolved \"open questions\", handle them AS A PRIORITY. \
         Otherwise, identify the most important angle not yet covered. \
         Do THE next most useful step to ADVANCE the case (deep web research, \
         analysis, cross-checking sources). Then YOU MUST:\n\
         1) write the NEW lasting facts/sources via memory_write under the node_id `{node}.findings` \
         (one fact = one clear, sourced item);\n\
         2) note the still-open questions via memory_write under `{node}.questions`;\n\
         3) update the case SYNTHESIS via memory_write under `{node}.synthese` (a global, \
         readable overview integrating this iteration).\n\
         Do NOT repeat what is already known above. Be rigorous, sourced, and conclude with a short \
         summary of what this iteration added.",
        objective = mission.objective,
        iter = mission.iterations + 1,
        node = node_id,
        etat = etat,
    )
}
