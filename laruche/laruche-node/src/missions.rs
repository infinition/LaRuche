//! Missions — travail/recherche AU LONG COURS piloté par objectif (« La Reine »).
//!
//! La métadonnée opérationnelle vit ici (`missions.json`) ; le SAVOIR capitalisé vit dans la
//! carte cognitive sous `missions.<slug>` (`.findings`, `.questions`…). Chaque itération avance
//! la mission : l'agent lit l'état en mémoire, fait l'étape suivante, écrit ses trouvailles.
//! (MVP : run manuel ou cron ; dream/synthèse/skills = itérations suivantes.)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mission {
    pub slug: String,
    pub objective: String,
    /// Cadence cron (ex. "0 9 * * 1" = lundi 9h) ; None = manuel uniquement.
    #[serde(default)]
    pub cadence: Option<String>,
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

    pub fn mark_run(&mut self, slug: &str, when: String) {
        if let Some(m) = self.missions.get_mut(slug) {
            m.iterations += 1;
            m.last_run = Some(when);
        }
        self.save();
    }
}

/// Slugifie un texte en identifiant de nœud (`missions.<slug>`).
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

/// Construit le prompt d'une itération de mission : l'agent lit l'état déjà capitalisé puis
/// avance d'une étape et écrit ses trouvailles sous `missions.<slug>`.
pub fn prompt_iteration(mission: &Mission, etat_actuel: &str) -> String {
    let node_id = format!("missions.{}", mission.slug);
    let etat = if etat_actuel.trim().is_empty() {
        "(rien encore — c'est la première itération)".to_string()
    } else {
        etat_actuel.to_string()
    };
    format!(
        "Tu avances une MISSION de recherche AU LONG COURS (tu la reprendras à chaque itération).\n\
         OBJECTIF : {objective}\n\
         Itération n°{iter}.\n\n\
         Déjà capitalisé en mémoire sous `{node}` :\n{etat}\n\n\
         Fais LA prochaine étape la plus utile pour faire AVANCER le dossier (recherche web approfondie, \
         analyse, recoupement de sources). Puis OBLIGATOIREMENT :\n\
         1) écris les NOUVEAUX faits/sources durables via memory_write sous le node_id `{node}.findings` \
         (un fait = un item clair, sourcé) ;\n\
         2) note les questions encore ouvertes via memory_write sous `{node}.questions`.\n\
         Ne répète PAS ce qui est déjà connu ci-dessus. Sois rigoureux, sourcé, et conclus par un court \
         bilan de ce que cette itération a ajouté.",
        objective = mission.objective,
        iter = mission.iterations + 1,
        node = node_id,
        etat = etat,
    )
}
