//! Event log endpoints (list recent events, export as NDJSON) - split out of main.rs.

use crate::*;
use axum::extract::State;
use axum::response::Json;
use std::sync::Arc;

// ======================== Events Endpoints ========================

pub(crate) async fn api_get_events(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> Json<Vec<laruche_events::Event>> {
    let since_id = params
        .get("since")
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);
    let events = state.events.read().await.since(since_id);
    Json(events)
}

pub(crate) async fn api_export_events(
    State(state): State<Arc<AppState>>,
) -> Result<String, axum::http::StatusCode> {
    let ndjson = state
        .events
        .read()
        .await
        .to_ndjson()
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(ndjson)
}
