//! Voice pipeline (STT/TTS websocket) - split out of main.rs.

use crate::*;
use axum::extract::State;

use std::sync::Arc;

// ======================== Voice Pipeline ========================

/// WebSocket handler for voice: receives audio, returns audio.
/// Protocol:
///   Client → binary (PCM 16kHz 16-bit mono) or JSON {"type":"config","stt_url":"...","tts_url":"..."}
///   Server → binary (WAV audio) or JSON {"type":"transcript","text":"..."} / {"type":"error",...}
pub(crate) async fn ws_audio_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> axum::response::Response {
    ws.on_upgrade(move |socket| ws_audio_connection(socket, state))
}

/// Resolve the STT/TTS service base URLs the way the runtime does: local
/// defaults (8421/8422), overridden by any stt/tts capability node discovered
/// on the Miel mesh. Shared by the voice websocket, the doctor and onboarding
/// so their probes reflect the URLs actually used at runtime.
pub(crate) async fn resolve_voice_urls(state: &Arc<AppState>) -> (String, String) {
    let mut stt_url = "http://127.0.0.1:8421".to_string();
    let mut tts_url = "http://127.0.0.1:8422".to_string();
    let listener = state.listener.read().await;
    let nodes = listener.get_nodes().await;
    for (_id, node) in &nodes {
        let caps: Vec<String> = node
            .manifest
            .capabilities
            .iter()
            .map(|c| c.to_string())
            .collect();
        let host = &node.manifest.host;
        if caps.iter().any(|c| c == "stt") {
            if let Some(port) = node.manifest.port {
                stt_url = format!("http://{}:{}", host, port);
                info!(stt_url = %stt_url, "Discovered STT node via Miel");
            }
        }
        if caps.iter().any(|c| c == "tts") {
            if let Some(port) = node.manifest.port {
                tts_url = format!("http://{}:{}", host, port);
                info!(tts_url = %tts_url, "Discovered TTS node via Miel");
            }
        }
    }
    (stt_url, tts_url)
}

/// True if the voice service answers 2xx on GET /health quickly.
///
/// 1s. A health check on a local service is either immediate or the service is not
/// there; three seconds only bought a longer wait, twice over (STT then TTS), on every
/// Settings tab.
pub(crate) async fn voice_service_up(base_url: &str) -> bool {
    reqwest::Client::new()
        .get(format!("{}/health", base_url))
        .timeout(std::time::Duration::from_millis(1000))
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

pub(crate) async fn ws_audio_connection(socket: ws::WebSocket, state: Arc<AppState>) {
    use futures_util::{SinkExt, StreamExt};

    let (mut sender, mut receiver) = socket.split();

    // Default STT/TTS endpoints (mesh-aware): can be overridden by client config message
    let (mut stt_url, mut tts_url) = resolve_voice_urls(&state).await;

    let _ = sender
        .send(ws::Message::Text(
            serde_json::json!({"type": "ready", "stt_url": &stt_url, "tts_url": &tts_url})
                .to_string()
                .into(),
        ))
        .await;

    let client = reqwest::Client::new();

    while let Some(Ok(msg)) = receiver.next().await {
        match msg {
            ws::Message::Binary(audio_data) => {
                // Step 1: Send audio to STT service
                let stt_result = client
                    .post(format!("{}/transcribe", stt_url))
                    .multipart(
                        reqwest::multipart::Form::new().part(
                            "file",
                            reqwest::multipart::Part::bytes(audio_data.to_vec())
                                .file_name("audio.wav")
                                .mime_str("audio/wav")
                                .unwrap(),
                        ),
                    )
                    .send()
                    .await;

                let transcript = match stt_result {
                    Ok(resp) => match resp.json::<serde_json::Value>().await {
                        Ok(json) => json["text"].as_str().unwrap_or("").to_string(),
                        Err(e) => {
                            let _ = sender.send(ws::Message::Text(
                                    serde_json::json!({"type":"error","message":format!("STT parse error: {}", e)}).to_string().into()
                                )).await;
                            continue;
                        }
                    },
                    Err(e) => {
                        let _ = sender.send(ws::Message::Text(
                            serde_json::json!({"type":"error","message":format!("STT unavailable: {}", e)}).to_string().into()
                        )).await;
                        continue;
                    }
                };

                if transcript.is_empty() {
                    continue;
                }

                // Send transcript to client
                let _ = sender
                    .send(ws::Message::Text(
                        serde_json::json!({"type":"transcript","text":&transcript})
                            .to_string()
                            .into(),
                    ))
                    .await;

                // Step 2: Run through ReAct agent
                let sessions_dir = std::path::Path::new("sessions");
                let audio_config = state.essaim_config.read().await.clone();
                let mut session = Session::new_with_path(&audio_config.model, sessions_dir);
                let (tx, _rx) = broadcast::channel::<ChatEvent>(64);

                let agent_result = boucle_react_memoire(
                    &transcript,
                    &mut session,
                    &state.essaim_registry,
                    &audio_config,
                    &tx,
                    state.memoire.clone(),
                )
                .await;

                let response_text = match agent_result {
                    Ok(text) => text,
                    Err(e) => {
                        let _ = sender.send(ws::Message::Text(
                            serde_json::json!({"type":"error","message":format!("Agent error: {}", e)}).to_string().into()
                        )).await;
                        continue;
                    }
                };

                // Send text response
                let _ = sender
                    .send(ws::Message::Text(
                        serde_json::json!({"type":"response","text":&response_text})
                            .to_string()
                            .into(),
                    ))
                    .await;

                // Step 3: Send response to TTS service
                let tts_result = client
                    .post(format!("{}/synthesize", tts_url))
                    .json(&serde_json::json!({"text": &response_text}))
                    .send()
                    .await;

                match tts_result {
                    Ok(resp) if resp.status().is_success() => {
                        if let Ok(audio_bytes) = resp.bytes().await {
                            let _ = sender
                                .send(ws::Message::Binary(audio_bytes.to_vec().into()))
                                .await;
                        }
                    }
                    Ok(resp) => {
                        let _ = sender.send(ws::Message::Text(
                            serde_json::json!({"type":"error","message":format!("TTS error: {}", resp.status())}).to_string().into()
                        )).await;
                    }
                    Err(e) => {
                        let _ = sender.send(ws::Message::Text(
                            serde_json::json!({"type":"error","message":format!("TTS unavailable: {}", e)}).to_string().into()
                        )).await;
                    }
                }
            }
            ws::Message::Text(text) => {
                // Config messages
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                    if json["type"].as_str() == Some("config") {
                        if let Some(url) = json["stt_url"].as_str() {
                            stt_url = url.to_string();
                        }
                        if let Some(url) = json["tts_url"].as_str() {
                            tts_url = url.to_string();
                        }
                    }
                }
            }
            ws::Message::Close(_) => break,
            _ => {}
        }
    }
}

