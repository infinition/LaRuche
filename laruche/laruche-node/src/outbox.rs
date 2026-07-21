//! **Delivery registry**: guarantees a produced answer eventually reaches its
//! channel, even across a crash or a restart.
//!
//! The window this closes is real: a mission can run for minutes (crons,
//! watchers, long research), and everything between "the answer exists" and
//! "the platform acknowledged it" was pure loss — the node restarts and the
//! user never learns the task completed.
//!
//! Protocol: [`enregistrer`] BEFORE sending, [`confirmer`] once the platform
//! accepted. Anything still pending at boot is replayed by [`rejouer`].
//! At-least-once delivery: a crash between the send and the confirm re-sends a
//! message once. That is the right trade-off here — a duplicate answer is a
//! minor annoyance, a lost one is invisible and unrecoverable.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

/// Beyond this many attempts an entry is dropped: a permanently failing
/// destination (revoked bot, deleted chat) must not be retried forever.
const MAX_TENTATIVES: u32 = 5;
/// Entries older than this are dropped at replay: delivering a two-day-old
/// answer out of the blue is worse than not delivering it.
const AGE_MAX_HEURES: i64 = 24;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvoiEnAttente {
    pub id: String,
    /// `telegram`, `discord`, ...
    pub canal: String,
    /// Channel-specific destination (Telegram chat id, Discord channel id...).
    pub destination: String,
    pub texte: String,
    pub cree_le: chrono::DateTime<chrono::Utc>,
    #[serde(default)]
    pub tentatives: u32,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Etat {
    #[serde(default)]
    en_attente: Vec<EnvoiEnAttente>,
}

static ETAT: std::sync::OnceLock<Mutex<Etat>> = std::sync::OnceLock::new();

fn chemin() -> PathBuf {
    std::env::var("LARUCHE_OUTBOX")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("outbox.json"))
}

fn etat() -> &'static Mutex<Etat> {
    ETAT.get_or_init(|| {
        let e = std::fs::read_to_string(chemin())
            .ok()
            .and_then(|j| serde_json::from_str(&j).ok())
            .unwrap_or_default();
        Mutex::new(e)
    })
}

fn persister() {
    let json = {
        let g = etat().lock().unwrap();
        serde_json::to_string_pretty(&*g).unwrap_or_default()
    };
    let c = chemin();
    let tmp = c.with_extension("json.tmp");
    if std::fs::write(&tmp, json).is_ok() {
        let _ = std::fs::rename(&tmp, &c);
    }
}

/// Registers an outbound message BEFORE attempting delivery. Returns the id to
/// pass to [`confirmer`].
pub fn enregistrer(canal: &str, destination: &str, texte: &str) -> String {
    let id = uuid::Uuid::new_v4().to_string();
    {
        let mut g = etat().lock().unwrap();
        g.en_attente.push(EnvoiEnAttente {
            id: id.clone(),
            canal: canal.to_string(),
            destination: destination.to_string(),
            texte: texte.to_string(),
            cree_le: chrono::Utc::now(),
            tentatives: 1,
        });
    }
    persister();
    id
}

/// Marks a message as delivered (the platform accepted it).
pub fn confirmer(id: &str) {
    {
        let mut g = etat().lock().unwrap();
        g.en_attente.retain(|e| e.id != id);
    }
    persister();
}

/// Everything still undelivered, minus entries that are too old or have burnt
/// their attempts (both are dropped here, so this is also the GC).
pub fn a_rejouer() -> Vec<EnvoiEnAttente> {
    let limite = chrono::Utc::now() - chrono::Duration::hours(AGE_MAX_HEURES);
    let (garde, rejouables) = {
        let mut g = etat().lock().unwrap();
        let mut rejouables = Vec::new();
        g.en_attente.retain(|e| {
            if e.cree_le < limite || e.tentatives >= MAX_TENTATIVES {
                tracing::warn!(
                    canal = %e.canal, id = %e.id, tentatives = e.tentatives,
                    "outbox: entry abandoned (too old or too many attempts)"
                );
                return false;
            }
            true
        });
        for e in g.en_attente.iter_mut() {
            e.tentatives += 1;
            rejouables.push(e.clone());
        }
        (g.en_attente.len(), rejouables)
    };
    persister();
    if !rejouables.is_empty() {
        tracing::info!(en_attente = garde, "outbox: replaying undelivered messages");
    }
    rejouables
}

/// Replays pending messages at boot. Only Telegram is wired today (the only
/// channel with a persistent bot token available at startup).
pub async fn rejouer(token_telegram: Option<&str>) {
    for envoi in a_rejouer() {
        let livre = match envoi.canal.as_str() {
            "telegram" => match token_telegram {
                Some(token) => {
                    let client = reqwest::Client::new();
                    client
                        .post(format!("https://api.telegram.org/bot{token}/sendMessage"))
                        .json(&serde_json::json!({
                            "chat_id": envoi.destination,
                            "text": format!("📬 (delayed delivery)\n{}", envoi.texte),
                        }))
                        .send()
                        .await
                        .map(|r| r.status().is_success())
                        .unwrap_or(false)
                }
                None => false,
            },
            _ => false,
        };
        if livre {
            confirmer(&envoi.id);
            tracing::info!(canal = %envoi.canal, "outbox: pending message delivered");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The store is process-global: this test drives the whole lifecycle in one
    /// go rather than fighting over shared state.
    #[test]
    fn cycle_enregistrer_confirmer_rejouer() {
        let dir = std::env::temp_dir().join(format!("outbox-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        std::env::set_var("LARUCHE_OUTBOX", dir.join("outbox.json"));

        let id = enregistrer("telegram", "123", "hello");
        assert_eq!(a_rejouer().len(), 1, "pending until confirmed");
        confirmer(&id);
        assert!(a_rejouer().is_empty(), "confirmed entries are gone");

        // Attempts are counted and eventually abandoned (no infinite retry).
        let _ = enregistrer("telegram", "123", "boom");
        for _ in 0..MAX_TENTATIVES {
            a_rejouer();
        }
        assert!(a_rejouer().is_empty(), "burnt entry dropped");

        std::env::remove_var("LARUCHE_OUTBOX");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
