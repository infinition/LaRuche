//! System status endpoints (voice STT/TTS availability, metrics history) - split out of main.rs.

use crate::*;
use axum::extract::State;
use axum::response::Json;
use std::sync::Arc;

pub(crate) async fn api_voice_status(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let listener = state.listener.read().await;
    let nodes = listener.get_nodes().await;

    // Service explicitly chosen per capability (otherwise: first found, previous behavior).
    let (want_stt, want_tts, stt_model, tts_model) = {
        let sel = state.capability_selection.read().await;
        (
            sel.get("stt").and_then(|s| s.node_id.clone()),
            sel.get("tts").and_then(|s| s.node_id.clone()),
            sel.get("stt").map(|s| s.model.clone()).unwrap_or_default(),
            sel.get("tts").map(|s| s.model.clone()).unwrap_or_default(),
        )
    };

    let mut stt_available = false;
    let mut tts_available = false;
    let mut stt_url = String::new();
    let mut tts_url = String::new();
    let mut stt_locked = false; // url locked by user selection
    let mut tts_locked = false;

    for node in nodes.values() {
        let nid = node.manifest.node_id.map(|x| x.to_string());
        let caps: Vec<String> = node
            .manifest
            .capabilities
            .iter()
            .map(|c| c.to_string())
            .collect();
        let url = node
            .manifest
            .port
            .map(|p| format!("http://{}:{}", node.manifest.host, p));

        if caps.iter().any(|c| c == "stt") {
            stt_available = true;
            if want_stt.is_some() && nid == want_stt {
                if let Some(u) = &url {
                    stt_url = u.clone();
                }
                stt_locked = true;
            } else if !stt_locked && stt_url.is_empty() {
                if let Some(u) = &url {
                    stt_url = u.clone();
                }
            }
        }
        if caps.iter().any(|c| c == "tts") {
            tts_available = true;
            if want_tts.is_some() && nid == want_tts {
                if let Some(u) = &url {
                    tts_url = u.clone();
                }
                tts_locked = true;
            } else if !tts_locked && tts_url.is_empty() {
                if let Some(u) = &url {
                    tts_url = u.clone();
                }
            }
        }
    }

    // Fallback: mDNS discovery can fail (Windows firewall, interface binding) even when
    // a voice service runs locally. Probe the default localhost ports directly so a TTS
    // (8422) or STT (8421) service started on this machine is still detected.
    if !tts_available {
        if let Some(u) = probe_local_voice(8422).await {
            tts_available = true;
            if tts_url.is_empty() { tts_url = u; }
        }
    }
    if !stt_available {
        if let Some(u) = probe_local_voice(8421).await {
            stt_available = true;
            if stt_url.is_empty() { stt_url = u; }
        }
    }

    Json(serde_json::json!({
        "stt": { "available": stt_available, "url": stt_url, "selected_model": stt_model, "is_selected": stt_locked },
        "tts": { "available": tts_available, "url": tts_url, "selected_model": tts_model, "is_selected": tts_locked },
    }))
}

/// POST /api/voice/tts - same-origin proxy to the TTS service. Lets the browser reach
/// it without CORS / mixed-content (the page may be HTTPS while the TTS is HTTP), and
/// works from a remote device (which cannot reach the server's 127.0.0.1). Body is the
/// synthesize payload ({"text": "..."}); the audio is streamed straight back.
pub(crate) async fn api_tts_proxy(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> axum::response::Response {
    use axum::response::IntoResponse;
    // Prefer a discovered TTS node, otherwise the local default port.
    let tts_base = {
        let listener = state.listener.read().await;
        let nodes = listener.get_nodes().await;
        nodes
            .values()
            .find(|n| {
                n.manifest
                    .capabilities
                    .iter()
                    .any(|c| matches!(c, miel_protocol::capabilities::Capability::Tts))
            })
            .and_then(|n| n.manifest.port.map(|p| format!("http://{}:{}", n.manifest.host, p)))
            .unwrap_or_else(|| "http://127.0.0.1:8422".to_string())
    };
    let client = reqwest::Client::new();
    match client.post(format!("{tts_base}/synthesize")).json(&body).send().await {
        Ok(resp) => {
            let ct = resp
                .headers()
                .get(axum::http::header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("audio/wav")
                .to_string();
            match resp.bytes().await {
                Ok(b) => ([(axum::http::header::CONTENT_TYPE, ct)], b).into_response(),
                Err(_) => (axum::http::StatusCode::BAD_GATEWAY, "tts read failed").into_response(),
            }
        }
        Err(_) => (axum::http::StatusCode::BAD_GATEWAY, "tts unreachable").into_response(),
    }
}

/// Quick `/health` probe of a local voice service. Returns its base URL when it
/// responds, otherwise None. Short timeout so the status endpoint stays snappy.
async fn probe_local_voice(port: u16) -> Option<String> {
    let url = format!("http://127.0.0.1:{port}");
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(600))
        .build()
        .ok()?;
    match client.get(format!("{url}/health")).send().await {
        Ok(r) if r.status().is_success() => Some(url),
        _ => None,
    }
}

/// GET /metrics/history - recent metrics snapshots and node events for the dashboard.
pub(crate) async fn get_metrics_history(
    State(state): State<Arc<AppState>>,
) -> Json<MetricsHistoryResponse> {
    let snapshots = state.metrics_history.read().await;
    let events = state.node_events.read().await;
    Json(MetricsHistoryResponse {
        snapshots: snapshots.iter().cloned().collect(),
        events: events.iter().cloned().collect(),
    })
}
