//! The **Feed journal**: a **persistent** system event log (append-only ndjson).
//!
//! Problem solved: previously, the Feed only showed *executions* (via `last_run`) and
//! memory mutations; *creations* (cron, watcher, mission, kanban) and curator runs
//! were never journaled, and everything was lost on restart. This journal records
//! **every system action** durably.
//!
//! **Global** access (like `MESH_SIGNER`) to avoid threading an `Arc` through each tool:
//! the node initializes it at startup ([`init`]), everyone calls [`record`], and `api_feed`
//! reads [`recent`]. Without init, [`record`] is a no-op (never panics).

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

/// A durable feed event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedEvent {
    /// Epoch timestamp in milliseconds.
    pub ts: i64,
    /// Displayed actor ("LaRuche", "User", "Curateur", etc.).
    pub actor: String,
    /// Category ("cron", "watcher", "mission", "kanban", "curator", etc.).
    pub kind: String,
    /// Readable action verb ("created the cron", "started the curator", etc.).
    pub action: String,
    /// Affected object (task name, watcher name, etc.).
    pub object: String,
}

struct Journal {
    events: VecDeque<FeedEvent>,
    path: PathBuf,
    cap: usize,
}

static JOURNAL: OnceLock<Mutex<Journal>> = OnceLock::new();

/// Initializes the journal: loads existing history from `path` (ndjson) and bounds the
/// size to `cap`. Call once at node startup. Idempotent (ignored if already initialized).
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

/// Records an event (memory + disk append). No-op if not initialized.
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
    // Disk append (best-effort: never blocks on a write error).
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

/// Returns the `limit` most recent events (from oldest to newest).
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
        // Must not panic even if JOURNAL is not initialized in this test.
        record("LaRuche", "cron", "created the cron", "x", chrono::Utc::now());
        // recent() returns empty while not initialized (or events from another test: we only test the absence of panic).
        let _ = recent(10);
    }
}
