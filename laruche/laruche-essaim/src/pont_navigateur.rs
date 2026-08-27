//! Rendezvous between the `browser` tool and the LaRuche Chrome extension.
//!
//! The extension cannot be reached directly from a tool: it connects inward, to
//! the node's websocket route, and the tool runs somewhere else entirely. This
//! module is the meeting point. The node hands over the socket, the tool calls
//! [`PontNavigateur::appeler`], and the reply is matched back by request id.
//!
//! The extension deliberately stays dumb. It knows how to navigate a tab, run a
//! script and capture the screen, nothing more: every behaviour that matters
//! (mapping a page, clicking a ref, the on-page indicator) is built here as
//! JavaScript and sent over. That keeps one implementation for both transports,
//! rather than a Rust version and a JavaScript version drifting apart.

use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot, Mutex};

/// How long a single extension command may take before we give up on it.
/// Generous, because a navigation on a slow site legitimately takes a while.
pub const TIMEOUT_APPEL: Duration = Duration::from_secs(45);

#[derive(Default)]
struct Etat {
    /// Outbound channel to the connected extension, if any.
    sortie: Option<mpsc::UnboundedSender<String>>,
    /// Calls waiting for their reply, keyed by request id.
    attentes: HashMap<u64, oneshot::Sender<Value>>,
    prochain_id: u64,
    /// Free-text description of the connected browser, shown to the user.
    agent: Option<String>,
}

pub struct PontNavigateur {
    etat: Mutex<Etat>,
}

static PONT: OnceLock<PontNavigateur> = OnceLock::new();

impl PontNavigateur {
    pub fn global() -> &'static Self {
        PONT.get_or_init(|| Self {
            etat: Mutex::new(Etat::default()),
        })
    }

    /// Register a freshly connected extension. Any previous one is dropped:
    /// a second browser taking over is normal (the user reloaded the extension),
    /// and keeping both would make replies ambiguous.
    pub async fn brancher(&self, sortie: mpsc::UnboundedSender<String>, agent: Option<String>) {
        let mut etat = self.etat.lock().await;
        etat.sortie = Some(sortie);
        etat.agent = agent;
        // Callers waiting on the old socket will now time out rather than hang
        // forever, so release them at once with an explicit error.
        for (_, tx) in etat.attentes.drain() {
            let _ = tx.send(json!({ "ok": false, "error": "browser reconnected mid-call" }));
        }
    }

    pub async fn debrancher(&self) {
        let mut etat = self.etat.lock().await;
        etat.sortie = None;
        etat.agent = None;
        for (_, tx) in etat.attentes.drain() {
            let _ = tx.send(json!({ "ok": false, "error": "browser disconnected" }));
        }
    }

    pub async fn est_connecte(&self) -> bool {
        let etat = self.etat.lock().await;
        etat.sortie.as_ref().is_some_and(|s| !s.is_closed())
    }

    pub async fn agent(&self) -> Option<String> {
        self.etat.lock().await.agent.clone()
    }

    /// Feed one inbound frame from the extension. Unknown or malformed frames
    /// are ignored on purpose: a future extension version may send events this
    /// build knows nothing about, and that must not break the connection.
    pub async fn message_recu(&self, texte: &str) {
        let Ok(v) = serde_json::from_str::<Value>(texte) else {
            return;
        };
        let Some(id) = v.get("id").and_then(Value::as_u64) else {
            return;
        };
        let mut etat = self.etat.lock().await;
        if let Some(tx) = etat.attentes.remove(&id) {
            let _ = tx.send(v);
        }
    }

    /// Send one command to the extension and wait for its reply.
    pub async fn appeler(&self, action: &str, params: Value) -> Result<Value> {
        let (id, rx) = {
            let mut etat = self.etat.lock().await;
            let Some(sortie) = etat.sortie.clone() else {
                return Err(anyhow!(
                    "The LaRuche extension is not connected. Open Chrome with the extension \
                     installed and enabled, or use mode \"launch\" to drive a browser started \
                     by LaRuche."
                ));
            };
            if sortie.is_closed() {
                etat.sortie = None;
                return Err(anyhow!("The LaRuche extension just disconnected."));
            }

            etat.prochain_id += 1;
            let id = etat.prochain_id;
            let (tx, rx) = oneshot::channel();
            etat.attentes.insert(id, tx);

            let trame = json!({ "id": id, "action": action, "params": params }).to_string();
            if sortie.send(trame).is_err() {
                etat.attentes.remove(&id);
                etat.sortie = None;
                return Err(anyhow!("Could not reach the LaRuche extension."));
            }
            (id, rx)
        };

        let reponse = match tokio::time::timeout(TIMEOUT_APPEL, rx).await {
            Err(_) => {
                self.etat.lock().await.attentes.remove(&id);
                return Err(anyhow!(
                    "The extension did not answer '{action}' within {}s.",
                    TIMEOUT_APPEL.as_secs()
                ));
            }
            Ok(Err(_)) => return Err(anyhow!("The extension call was cancelled.")),
            Ok(Ok(v)) => v,
        };

        if reponse.get("ok").and_then(Value::as_bool) == Some(true) {
            return Ok(reponse.get("result").cloned().unwrap_or(Value::Null));
        }
        Err(anyhow!(
            "{}",
            reponse
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("the extension reported an unspecified failure")
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn refuses_calls_with_no_extension() {
        let pont = PontNavigateur {
            etat: Mutex::new(Etat::default()),
        };
        assert!(!pont.est_connecte().await);
        let err = pont.appeler("navigate", json!({})).await.unwrap_err();
        assert!(err.to_string().contains("not connected"));
    }

    #[tokio::test]
    async fn matches_reply_to_its_call() {
        let pont = PontNavigateur {
            etat: Mutex::new(Etat::default()),
        };
        let (tx, mut rx) = mpsc::unbounded_channel();
        pont.brancher(tx, Some("test".into())).await;
        assert!(pont.est_connecte().await);

        let appel = async {
            let trame = rx.recv().await.expect("command must be sent");
            let v: Value = serde_json::from_str(&trame).unwrap();
            assert_eq!(v["action"], "eval");
            let id = v["id"].as_u64().unwrap();
            // Answer an unrelated id first: it must not unblock the caller.
            pont.message_recu(&json!({ "id": 9999, "ok": true }).to_string())
                .await;
            pont.message_recu(
                &json!({ "id": id, "ok": true, "result": { "value": 42 } }).to_string(),
            )
            .await;
        };

        let (res, _) = tokio::join!(pont.appeler("eval", json!({ "script": "1" })), appel);
        assert_eq!(res.unwrap()["value"], 42);
    }

    #[tokio::test]
    async fn disconnect_releases_waiters() {
        let pont = PontNavigateur {
            etat: Mutex::new(Etat::default()),
        };
        let (tx, mut rx) = mpsc::unbounded_channel();
        pont.brancher(tx, None).await;

        let appel = async {
            let _ = rx.recv().await;
            pont.debrancher().await;
        };
        let (res, _) = tokio::join!(pont.appeler("navigate", json!({})), appel);
        assert!(res.unwrap_err().to_string().contains("disconnected"));
    }
}
