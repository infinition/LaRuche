//! WebSocket endpoint for the LaRuche Chrome extension.
//!
//! The extension connects here and stays connected. Frames are relayed verbatim
//! between the socket and [`PontNavigateur`], which the `browser` tool talks to.
//! No protocol logic lives here on purpose: the command vocabulary belongs to
//! the tool, and this file should not need editing when it grows.
//!
//! Protocol:
//!   Server -> extension  {"id":1,"action":"eval","params":{...}}
//!   extension -> Server  {"id":1,"ok":true,"result":{...}}
//!                        {"id":1,"ok":false,"error":"..."}

use crate::*;
use axum::extract::State;
use axum::response::IntoResponse;
use laruche_essaim::pont_navigateur::PontNavigateur;
use std::sync::Arc;

/// The LaRuche extension, and only it, may take this socket.
///
/// Identity comes from the `key` field of `extension-chrome/manifest.json`: it
/// fixes the extension id to the hash of that public key, so the id no longer
/// depends on where the folder sits on disk and can be pinned here.
///
/// Set `LARUCHE_EXTENSION_ID` to a comma-separated list of ids to allow others,
/// which is what a fork or a second build needs. `*` allows any extension, the
/// behaviour this route had before, kept for anyone driving it from their own
/// unpacked build without wanting to touch the manifest.
const EXTENSION_ID: &str = "ahgfjacmpohglimmcfnlbeccdghpkboo";

/// Only the LaRuche extension may take this socket.
///
/// Two doors are closed here, and they are not the same door.
///
/// The CORS layer does not apply to websockets, so any page the user happens to
/// visit could otherwise open `ws://127.0.0.1:<port>/ws/navigateur`, answer in
/// the extension's place and feed the agent whatever it likes. `Origin` cannot
/// be forged by page script, so requiring an extension origin closes that one.
/// A missing Origin is refused too: real browsers always send one.
///
/// Accepting ANY extension origin left the second door open. The bridge hands
/// whoever holds it a raw DevTools channel over the user's own Chrome, sessions
/// included, and [`PontNavigateur::brancher`] silently replaces the extension
/// already connected. Any other extension the user installs, on any pretext,
/// could therefore take the channel from under LaRuche. Matching the id closes
/// it: the origin is the extension's identity and Chrome sets it, not the page.
fn origine_autorisee(headers: &axum::http::HeaderMap) -> bool {
    let Some(origine) = headers
        .get(axum::http::header::ORIGIN)
        .and_then(|v| v.to_str().ok())
    else {
        return false;
    };
    let Some(id) = origine
        .strip_prefix("chrome-extension://")
        .or_else(|| origine.strip_prefix("moz-extension://"))
    else {
        return false;
    };
    // Origin carries no path, but a stray trailing slash costs nothing to accept.
    let id = id.trim_end_matches('/');
    if id.is_empty() {
        return false;
    }
    match std::env::var("LARUCHE_EXTENSION_ID") {
        Ok(liste) if liste.trim() == "*" => true,
        Ok(liste) if !liste.trim().is_empty() => liste.split(',').any(|a| a.trim() == id),
        _ => id == EXTENSION_ID,
    }
}

pub(crate) async fn ws_navigateur_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> axum::response::Response {
    if !origine_autorisee(&headers) {
        // The id is named on both sides: a refusal here otherwise reads as "the
        // extension will not connect" with nothing to compare, and the honest
        // cause is usually a build whose id is not the pinned one.
        tracing::warn!(
            origin = ?headers.get(axum::http::header::ORIGIN),
            attendu = EXTENSION_ID,
            "browser bridge: rejected a connection that is not the LaRuche extension"
        );
        return (
            axum::http::StatusCode::FORBIDDEN,
            axum::Json(serde_json::json!({
                "error": format!(
                    "this socket only accepts the LaRuche extension (id {EXTENSION_ID}); \
                     set LARUCHE_EXTENSION_ID to allow another build"
                )
            })),
        )
            .into_response();
    }
    let agent = params.get("agent").cloned();
    ws.on_upgrade(move |socket| ws_navigateur_connection(socket, agent, state))
}

pub(crate) async fn ws_navigateur_connection(
    socket: ws::WebSocket,
    agent: Option<String>,
    state: Arc<AppState>,
) {
    use futures_util::{SinkExt, StreamExt};

    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();

    let pont = PontNavigateur::global();
    pont.brancher(tx, agent.clone()).await;
    tracing::info!(agent = ?agent, "LaRuche browser extension connected");

    // Outbound: commands from the tool to the extension.
    let ecriture = tokio::spawn(async move {
        while let Some(trame) = rx.recv().await {
            if sender.send(ws::Message::Text(trame)).await.is_err() {
                break;
            }
        }
    });

    // Inbound: replies from the extension, and now its own requests.
    while let Some(Ok(msg)) = receiver.next().await {
        match msg {
            ws::Message::Text(txt) => {
                if let Some(demande) = pont.message_recu(&txt).await {
                    traiter_demande(&state, pont, demande).await;
                }
            }
            ws::Message::Close(_) => break,
            _ => {}
        }
    }

    ecriture.abort();
    pont.debrancher().await;
    tracing::info!("LaRuche browser extension disconnected");
}

/// Ce que l'extension demande de sa propre initiative.
///
/// Un seul verbe aujourd'hui, `garder`: l'utilisateur met de cote un lien ou une
/// note depuis sa navigation, et ca atterrit dans la memoire. La socket est deja
/// restreinte a l'identifiant de l'extension et vit sur la boucle locale, donc
/// elle porte la meme confiance que le pilotage qu'elle sert deja.
///
/// La reponse repart par la meme socket, avec l'identifiant que l'extension a
/// choisi: sans accuse, elle ne peut pas savoir si elle doit garder l'entree en
/// file, et une note perdue en silence est pire que pas de fonction du tout.
async fn traiter_demande(
    state: &Arc<AppState>,
    pont: &laruche_essaim::pont_navigateur::PontNavigateur,
    demande: serde_json::Value,
) {
    let jeton = demande.get("req").cloned().unwrap_or(serde_json::Value::Null);
    let repondre = |ok: bool, erreur: Option<String>| {
        serde_json::json!({ "req": jeton, "ok": ok, "error": erreur })
    };

    if demande["type"].as_str() != Some("garder") {
        pont.pousser(repondre(false, Some("unknown request".into())))
            .await;
        return;
    }

    let noeud = demande["entree"]["noeud"].as_str().unwrap_or("").trim();
    let contenu = demande["entree"]["contenu"].as_str().unwrap_or("").trim();
    if noeud.is_empty() || contenu.is_empty() {
        pont.pousser(repondre(false, Some("noeud et contenu requis".into())))
            .await;
        return;
    }

    let item = laruche_memoire::MemoryItem::new(noeud, contenu).with_source("extension-chrome");
    match state.memoire.write(item).await {
        Ok(_) => {
            tracing::info!(noeud, "extension: entree gardee en memoire");
            pont.pousser(repondre(true, None)).await;
        }
        Err(e) => {
            tracing::warn!(error = %e, noeud, "extension: echec de la garde en memoire");
            pont.pousser(repondre(false, Some(e.to_string()))).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{header::ORIGIN, HeaderMap, HeaderValue};

    fn avec_origine(valeur: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(ORIGIN, HeaderValue::from_str(valeur).unwrap());
        h
    }

    #[test]
    fn accepte_l_extension_laruche() {
        let h = avec_origine(&format!("chrome-extension://{EXTENSION_ID}"));
        assert!(origine_autorisee(&h));
    }

    #[test]
    fn refuse_une_autre_extension() {
        // The whole point of pinning: an extension origin is no longer enough.
        let h = avec_origine("chrome-extension://abcdefghijklmnopabcdefghijklmnop");
        assert!(!origine_autorisee(&h));
    }

    #[test]
    fn refuse_une_page_web_et_l_origine_absente() {
        assert!(!origine_autorisee(&avec_origine("https://exemple.test")));
        assert!(!origine_autorisee(&HeaderMap::new()));
        assert!(!origine_autorisee(&avec_origine("chrome-extension://")));
    }
}
