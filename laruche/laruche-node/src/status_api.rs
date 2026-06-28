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

    for (_id, node) in &nodes {
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

    Json(serde_json::json!({
        "stt": { "available": stt_available, "url": stt_url, "selected_model": stt_model, "is_selected": stt_locked },
        "tts": { "available": tts_available, "url": tts_url, "selected_model": tts_model, "is_selected": tts_locked },
    }))
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
