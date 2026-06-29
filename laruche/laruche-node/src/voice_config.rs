//! Persisted voice options: external-STT preference + per-chat Telegram voice replies.

use crate::*;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::Json;
use std::sync::Arc;

const VOICE_CONFIG_FILE: &str = "laruche-voice.json";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct VoiceConfig {
    /// false (default) = send the audio to the model (native STT). true = transcribe via
    /// the external STT service first.
    #[serde(default)]
    pub stt_external: bool,
    /// Telegram chat_ids that opted into voice-note replies (/voice). Persisted so the
    /// toggle survives a restart.
    #[serde(default)]
    pub telegram_voice_chats: Vec<i64>,
}

pub(crate) fn charger() -> VoiceConfig {
    std::fs::read_to_string(VOICE_CONFIG_FILE)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn sauver(c: &VoiceConfig) {
    if let Ok(j) = serde_json::to_string_pretty(c) {
        let _ = std::fs::write(VOICE_CONFIG_FILE, j);
    }
}

/// Persist a chat's voice-reply preference (idempotent).
pub(crate) fn set_telegram_voice(chat_id: i64, on: bool) {
    let mut c = charger();
    let has = c.telegram_voice_chats.contains(&chat_id);
    if on && !has {
        c.telegram_voice_chats.push(chat_id);
    } else if !on && has {
        c.telegram_voice_chats.retain(|&x| x != chat_id);
    }
    sauver(&c);
}

/// GET /api/config/voice - current voice options.
pub(crate) async fn api_get_voice() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "stt_external": charger().stt_external }))
}

/// POST /api/config/voice - update voice options (auth required).
pub(crate) async fn api_set_voice(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    auth_user::extract_user_from_headers(&headers, &state.cookie_secret)
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let mut c = charger();
    if let Some(v) = body.get("stt_external").and_then(|x| x.as_bool()) {
        c.stt_external = v;
    }
    sauver(&c);
    Ok(Json(serde_json::json!({ "status": "ok" })))
}
