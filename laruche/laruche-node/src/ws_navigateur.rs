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

/// Only a browser extension may take this socket.
///
/// The CORS layer does not apply to websockets: any page the user happens to
/// visit could otherwise open `ws://127.0.0.1:<port>/ws/navigateur`, answer in
/// the extension's place and feed the agent whatever it likes. `Origin` cannot
/// be forged by page script, so requiring an extension origin closes that door.
/// A missing Origin is refused too: real browsers always send one.
fn origine_autorisee(headers: &axum::http::HeaderMap) -> bool {
    headers
        .get(axum::http::header::ORIGIN)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|o| o.starts_with("chrome-extension://") || o.starts_with("moz-extension://"))
}

pub(crate) async fn ws_navigateur_handler(
    ws: WebSocketUpgrade,
    State(_state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> axum::response::Response {
    if !origine_autorisee(&headers) {
        tracing::warn!(
            origin = ?headers.get(axum::http::header::ORIGIN),
            "browser bridge: rejected a connection that is not a browser extension"
        );
        return (
            axum::http::StatusCode::FORBIDDEN,
            axum::Json(serde_json::json!({ "error": "extension origin required" })),
        )
            .into_response();
    }
    let agent = params.get("agent").cloned();
    ws.on_upgrade(move |socket| ws_navigateur_connection(socket, agent))
}

pub(crate) async fn ws_navigateur_connection(socket: ws::WebSocket, agent: Option<String>) {
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

    // Inbound: replies from the extension.
    while let Some(Ok(msg)) = receiver.next().await {
        match msg {
            ws::Message::Text(txt) => pont.message_recu(&txt).await,
            ws::Message::Close(_) => break,
            _ => {}
        }
    }

    ecriture.abort();
    pont.debrancher().await;
    tracing::info!("LaRuche browser extension disconnected");
}
