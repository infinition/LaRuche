//! Le **journal du Feed** — un log d'événements système **persistant** (append-only ndjson).
//!
//! Problème résolu : avant, le Feed ne montrait que les *exécutions* (via `last_run`) et les
//! mutations mémoire ; les *créations* (cron, watcher, mission, kanban) et les runs du curateur
//! n'étaient jamais journalisés, et tout disparaissait au redémarrage. Ce journal enregistre
//! **toute action système** de façon durable.
//!
//! Accès **global** (comme `MESH_SIGNER`) pour éviter de threader un `Arc` dans chaque outil :
//! le node l'initialise au démarrage ([`init`]), tout le monde appelle [`record`], et `api_feed`
//! lit [`recent`]. Sans init, [`record`] est un no-op (jamais de panique).

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

/// Un événement de feed durable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedEvent {
    /// Timestamp epoch en millisecondes.
    pub ts: i64,
    /// Auteur affiché (« LaRuche », « User », « Curateur »…).
    pub actor: String,
    /// Catégorie (« cron », « watcher », « mission », « kanban », « curator »…).
    pub kind: String,
    /// Verbe d'action lisible (« a créé le cron », « a lancé le curateur »…).
    pub action: String,
    /// Objet concerné (nom de la tâche, du watcher…).
    pub object: String,
}

struct Journal {
    events: VecDeque<FeedEvent>,
    path: PathBuf,
    cap: usize,
}

static JOURNAL: OnceLock<Mutex<Journal>> = OnceLock::new();

/// Initialise le journal : charge l'historique existant depuis `path` (ndjson) et borne la
/// taille à `cap`. À appeler une fois au démarrage du node. Idempotent (ignoré si déjà init).
pub fn init(path: PathBuf, cap: usize) {
    let mut events = VecDeque::new();
    if let Ok(contenu) = std::fs::read_to_string(&path) {
        for ligne in contenu.lines() {
            if ligne.trim().is_empty() {
                continue;
            }
            if let Ok(ev) = serde_json::from_str::<FeedEvent>(ligne) {
                events.push_back(ev);
            }
        }
        while events.len() > cap {
            events.pop_front();
        }
    }
    let _ = JOURNAL.set(Mutex::new(Journal { events, path, cap }));
}

/// Enregistre un événement (mémoire + append disque). No-op si non initialisé.
pub fn record(
    actor: impl Into<String>,
    kind: impl Into<String>,
    action: impl Into<String>,
    object: impl Into<String>,
    now: chrono::DateTime<chrono::Utc>,
) {
    let Some(lock) = JOURNAL.get() else { return };
    let ev = FeedEvent {
        ts: now.timestamp_millis(),
        actor: actor.into(),
        kind: kind.into(),
        action: action.into(),
        object: object.into(),
    };
    let Ok(mut j) = lock.lock() else { return };
    // Append disque (best-effort : on ne bloque jamais sur une erreur d'écriture).
    if let Ok(ligne) = serde_json::to_string(&ev) {
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&j.path) {
            let _ = writeln!(f, "{ligne}");
        }
    }
    j.events.push_back(ev);
    while j.events.len() > j.cap {
        j.events.pop_front();
    }
}

/// Renvoie les `limit` événements les plus récents (du plus ancien au plus récent).
pub fn recent(limit: usize) -> Vec<FeedEvent> {
    let Some(lock) = JOURNAL.get() else { return Vec::new() };
    let Ok(j) = lock.lock() else { return Vec::new() };
    let n = j.events.len();
    let start = n.saturating_sub(limit);
    j.events.iter().skip(start).cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_sans_init_est_un_noop() {
        // Ne doit pas paniquer même si JOURNAL n'est pas initialisé dans ce test.
        record("LaRuche", "cron", "a créé le cron", "x", chrono::Utc::now());
        // recent() renvoie vide tant que non init (ou les events d'un autre test — on teste juste l'absence de panique).
        let _ = recent(10);
    }
}
