//! Slack Events API (url_verification challenge, message and app_mention event callbacks) - split out of main.rs.

use crate::*;
use axum::extract::State;
use axum::response::Json;
use std::sync::Arc;

// ======================== Slack Events ========================

/// POST /api/channels/slack/events: receive Slack Events API callbacks.
/// Handles:
///   - `url_verification` challenge (required by Slack during setup)
///   - `event_callback` with `message` and `app_mention` events
pub(crate) async fn api_slack_events(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let event_type = body["type"].as_str().unwrap_or("");

    match event_type {
        // Slack URL verification challenge
        "url_verification" => {
            let challenge = body["challenge"].as_str().unwrap_or("");
            info!("Slack: URL verification challenge");
            Json(serde_json::json!({"challenge": challenge}))
        }
        // Actual event callbacks
        "event_callback" => {
            let event = &body["event"];
            let event_subtype = event["type"].as_str().unwrap_or("");
            let subtype = event["subtype"].as_str();

            // Ignore bot messages to prevent loops
            if event.get("bot_id").is_some() || subtype == Some("bot_message") {
                return Json(serde_json::json!({"ok": true}));
            }

            let text = event["text"].as_str().unwrap_or("");
            let channel = event["channel"].as_str().unwrap_or("");
            let user = event["user"].as_str().unwrap_or("unknown");

            if text.is_empty() || channel.is_empty() {
                return Json(serde_json::json!({"ok": true}));
            }

            match event_subtype {
                "message" | "app_mention" => {
                    info!(
                        user = user,
                        channel = channel,
                        event_type = event_subtype,
                        text = &text[..text.len().min(50)],
                        "Slack event"
                    );

                    // Strip bot mention (e.g., "<@U123456> what is Rust?" -> "what is Rust?")
                    let clean_text = if text.starts_with('<') {
                        text.find('>').map(|i| text[i + 1..].trim()).unwrap_or(text)
                    } else {
                        text
                    };

                    if clean_text.is_empty() {
                        return Json(serde_json::json!({"ok": true}));
                    }

                    // Run agent query: persistent session per Slack channel (conversational memory).
                    let response = channels_api::run_agent_query(&state, "slack", channel, clean_text).await;

                    // Post reply via Slack API
                    let config_path = std::path::Path::new("channels-config.json");
                    if let Ok(content) = std::fs::read_to_string(config_path) {
                        if let Ok(config) = serde_json::from_str::<serde_json::Value>(&content) {
                            let bot_token = &laruche_essaim::secrets::substituer(
                                config["slack"]["bot_token"].as_str().unwrap_or(""),
                            );
                            if !bot_token.is_empty() {
                                let http = reqwest::Client::new();
                                let _ = http
                                    .post("https://slack.com/api/chat.postMessage")
                                    .header("Authorization", format!("Bearer {}", bot_token))
                                    .json(&serde_json::json!({
                                        "channel": channel,
                                        "text": response,
                                    }))
                                    .send()
                                    .await;
                                info!(
                                    channel = channel,
                                    response_len = response.len(),
                                    "Slack replied"
                                );
                            }
                        }
                    }
                }
                _ => {
                    // Ignore other event types
                }
            }

            Json(serde_json::json!({"ok": true}))
        }
        _ => {
            warn!(event_type = event_type, "Slack: unknown event type");
            Json(serde_json::json!({"ok": true}))
        }
    }
}

