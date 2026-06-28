//! Discord interaction webhook (slash command and interaction callbacks) - split out of main.rs.

use crate::*;
use axum::extract::State;
use axum::response::Json;
use std::sync::Arc;

// ======================== Discord Webhook ========================

/// POST /api/channels/discord/webhook: receive Discord Interactions (slash commands).
/// Discord sends interactions as POST requests to the configured endpoint URL.
/// Interaction types:
///   1 = PING (verification), 2 = APPLICATION_COMMAND (slash command),
///   3 = MESSAGE_COMPONENT, 4 = APPLICATION_COMMAND_AUTOCOMPLETE
pub(crate) async fn api_discord_webhook(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let interaction_type = body["type"].as_u64().unwrap_or(0);

    match interaction_type {
        // Type 1: PING: Discord verification handshake
        1 => {
            info!("Discord: PING received (verification)");
            Json(serde_json::json!({"type": 1}))
        }
        // Type 2: APPLICATION_COMMAND: slash command
        2 => {
            let command_name = body["data"]["name"].as_str().unwrap_or("");
            let user = body["member"]["user"]["username"]
                .as_str()
                .or_else(|| body["user"]["username"].as_str())
                .unwrap_or("unknown");

            // Extract the user's input from the command options
            let input = body["data"]["options"]
                .as_array()
                .and_then(|opts| {
                    opts.iter()
                        .find(|o| {
                            o["name"].as_str() == Some("prompt")
                                || o["name"].as_str() == Some("message")
                        })
                        .and_then(|o| o["value"].as_str())
                })
                .unwrap_or("");

            if input.is_empty() {
                return Json(serde_json::json!({
                    "type": 4,
                    "data": {
                        "content": "Please provide a prompt. Usage: `/ask <your question>`"
                    }
                }));
            }

            info!(
                user = user,
                command = command_name,
                input = &input[..input.len().min(50)],
                "Discord slash command"
            );

            // Run agent query: persistent session per Discord user (conversational memory).
            let response = run_agent_query(&state, "discord", user, input).await;

            // Truncate if needed (Discord max: 2000 chars)
            let truncated = if response.len() > 1990 {
                format!("{}...", &response[..1990])
            } else {
                response
            };

            // Type 4 = CHANNEL_MESSAGE_WITH_SOURCE
            Json(serde_json::json!({
                "type": 4,
                "data": {
                    "content": truncated
                }
            }))
        }
        // Unknown interaction type
        _ => {
            warn!(
                interaction_type = interaction_type,
                "Discord: unknown interaction type"
            );
            Json(serde_json::json!({"type": 1}))
        }
    }
}
