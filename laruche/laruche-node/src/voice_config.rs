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
    /// TTS playback speed (1.0 = normal). Applied to both web and Telegram voice.
    #[serde(default = "default_speed")]
    pub tts_speed: f32,
    /// TTS voice override (e.g. a Kokoro voice id like "ff_siwis"). Empty = service default.
    #[serde(default)]
    pub tts_voice: String,
    /// TTS backend override (kokoro/voicebox/edge-tts/...). Empty = the service default.
    #[serde(default)]
    pub tts_backend: String,
}

fn default_speed() -> f32 {
    1.0
}

pub(crate) fn charger() -> VoiceConfig {
    let mut c: VoiceConfig = std::fs::read_to_string(VOICE_CONFIG_FILE)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    // Default-derived f32 is 0.0; keep speed in a sane range.
    if !(0.5..=2.0).contains(&c.tts_speed) {
        c.tts_speed = 1.0;
    }
    c
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
    let c = charger();
    Json(serde_json::json!({
        "stt_external": c.stt_external,
        "tts_speed": c.tts_speed,
        "tts_voice": c.tts_voice,
        "tts_backend": c.tts_backend,
    }))
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
    if let Some(v) = body.get("tts_speed").and_then(|x| x.as_f64()) {
        c.tts_speed = (v as f32).clamp(0.5, 2.0);
    }
    if let Some(v) = body.get("tts_voice").and_then(|x| x.as_str()) {
        c.tts_voice = v.trim().to_string();
    }
    if let Some(v) = body.get("tts_backend").and_then(|x| x.as_str()) {
        c.tts_backend = v.trim().to_string();
    }
    sauver(&c);
    Ok(Json(serde_json::json!({ "status": "ok" })))
}
