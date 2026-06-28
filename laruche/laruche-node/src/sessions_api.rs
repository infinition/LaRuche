//! Session endpoints (list, delete, messages, search, export, fork) and the client-facing message display helpers with their tests - split out of main.rs.

use crate::*;
use axum::extract::State;
use axum::response::Json;
use axum::http::StatusCode;
use std::sync::Arc;

/// List all sessions with metadata.
pub(crate) async fn api_list_sessions(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> Json<serde_json::Value> {
    let caller = auth_user::extract_user_from_headers(&headers, &state.cookie_secret);
    let sessions = state.essaim_sessions.read().await;
    let list: Vec<serde_json::Value> = sessions
        .values()
        .filter(|s| {
            // Show: user's own sessions + legacy sessions (no owner)
            s.user_id.is_none() || s.user_id == caller
        })
        .map(|s| {
            serde_json::json!({
                "id": s.id.to_string(),
                "title": s.title,
                "model": s.model,
                "messages": s.len(),
                "estimated_tokens": s.estimated_tokens(),
                "created_at": s.created_at.to_rfc3339(),
                "updated_at": s.updated_at.to_rfc3339(),
            })
        })
        .collect();
    Json(serde_json::json!(list))
}

/// Delete a session by ID (with ownership check).
pub(crate) async fn api_delete_session(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> StatusCode {
    let caller = auth_user::extract_user_from_headers(&headers, &state.cookie_secret);
    if let Ok(uuid) = Uuid::parse_str(&id) {
        let mut sessions = state.essaim_sessions.write().await;
        // Check ownership before deleting
        if let Some(session) = sessions.get(&uuid) {
            if session.user_id.is_some() && session.user_id != caller {
                warn!(session_id = %uuid, "Unauthorized session delete attempt");
                return StatusCode::FORBIDDEN;
            }
        }
        if sessions.remove(&uuid).is_some() {
            let path = std::path::PathBuf::from("sessions").join(format!("{}.json", uuid));
            let _ = std::fs::remove_file(path);
            info!(session_id = %uuid, "Session deleted");
            return StatusCode::OK;
        }
    }
    StatusCode::NOT_FOUND
}

fn strip_display_tag_blocks(text: &str, tag: &str) -> String {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let mut clean = text.to_string();
    while let Some(start) = clean.find(&open) {
        if let Some(end) = clean[start + open.len()..].find(&close) {
            let end = start + open.len() + end + close.len();
            clean.replace_range(start..end, "");
        } else {
            clean.truncate(start);
            break;
        }
    }
    clean
}

/// Removes instructions injected for the ReAct loop from the user-facing transcript.
fn display_user_text(text: &str) -> Option<String> {
    const CAPABILITY_HINT: &str = "\n\n[SYSTEM] You can schedule (cron_create), watch (watcher_create) and search your past conversations (session_search) yourself.";
    const AUTO_CONTINUE: &str = "Continue immediately with the next step of the plan";
    const OUTPUT_RECOVERY: &str = "Continue exactly from the interrupted response.";
    const FAILOVER_RECOVERY: &str = "The previous response was truncated twice.";

    if text.starts_with(AUTO_CONTINUE)
        || text.starts_with(OUTPUT_RECOVERY)
        || text.starts_with(FAILOVER_RECOVERY)
    {
        return None;
    }

    let text = text.strip_suffix(CAPABILITY_HINT).unwrap_or(text);
    let text = text.strip_prefix("/no_think\n").unwrap_or(text);
    if let Some((_, steering)) = text.split_once("\n") {
        if text.starts_with("[User steering injected during") {
            return Some(steering.to_string());
        }
    }
    Some(text.to_string())
}

/// Converts the durable ReAct transcript to the clean presentation transcript.
/// Internal tool and plan tags remain in storage for the agent, while the UI gets
/// plain assistant text plus the latest structured plan for the left-hand workflow.
fn session_message_for_client(message: &laruche_essaim::Message) -> Option<serde_json::Value> {
    match message {
        laruche_essaim::Message::User(text) => {
            display_user_text(text).map(|text| serde_json::json!({"role": "user", "text": text}))
        }
        laruche_essaim::Message::UserMultimodal { text, attachments } => {
            let text = display_user_text(text)?;
            let att_meta: Vec<serde_json::Value> = attachments
                .iter()
                .map(|a| {
                    serde_json::json!({
                        "kind": a.kind,
                        "mime_type": a.mime_type,
                        "filename": a.filename,
                        "data": if a.kind == "image" { a.data.clone() } else { String::new() }
                    })
                })
                .collect();
            Some(serde_json::json!({
                "role": "user",
                "text": text,
                "attachments": att_meta
            }))
        }
        laruche_essaim::Message::Assistant(text) => {
            let plan = laruche_essaim::brain::parse_plan(text)
                .and_then(|items| serde_json::to_value(items).ok());
            let clean = strip_display_tag_blocks(
                &strip_display_tag_blocks(&strip_display_tag_blocks(text, "tool_call"), "plan"),
                "think",
            );
            let mut value = serde_json::json!({"role": "assistant", "text": clean.trim()});
            if let Some(plan) = plan {
                value["plan"] = plan;
            }
            Some(value)
        }
        laruche_essaim::Message::Thought { phase, kind, text } => Some(serde_json::json!({
            "role": "thought",
            "phase": phase,
            "kind": kind,
            "text": text,
        })),
        laruche_essaim::Message::PromptDebug {
            payload,
            model,
            provider,
        } => Some(serde_json::json!({
            "role": "prompt_debug",
            "payload": payload,
            "model": model,
            "provider": provider,
        })),
        laruche_essaim::Message::Observation { tool, result, .. } => {
            Some(serde_json::json!({"role": "tool", "tool": tool, "text": result}))
        }
        laruche_essaim::Message::ToolCall { name, args } => {
            Some(serde_json::json!({"role": "tool_call", "tool": name, "args": args}))
        }
        // System/compaction notes are model context, never visible chat messages.
        laruche_essaim::Message::System(_) => None,
    }
}

#[cfg(test)]
mod session_display_tests {
    use super::*;

    #[test]
    fn user_display_hides_agent_only_instructions() {
        let raw = "Download this\n\n[SYSTEM] You can schedule (cron_create), watch (watcher_create) and search your past conversations (session_search) yourself.";
        assert_eq!(display_user_text(raw).as_deref(), Some("Download this"));
        assert!(display_user_text(
            "Continue immediately with the next step of the plan, without stopping."
        )
        .is_none());
    }

    #[test]
    fn assistant_display_keeps_plan_structured_and_hides_markup() {
        let message = laruche_essaim::Message::Assistant(
            "<plan>[{\"task\":\"Download\",\"status\":\"done\"}]</plan>\nFile ready.<tool_call>{}</tool_call>"
                .into(),
        );
        let display = session_message_for_client(&message).unwrap();
        assert_eq!(display["text"], "File ready.");
        assert_eq!(display["plan"][0]["task"], "Download");
    }

    #[test]
    fn active_context_stats_progressent_pendant_les_outils() {
        let mut stats = ActiveContextStats {
            messages: 1,
            base_tokens: 65,
            running: true,
            ..ActiveContextStats::default()
        };

        stats.apply_event(&ChatEvent::Token {
            text: "I will fetch the page then analyze the result.".into(),
        });
        stats.apply_event(&ChatEvent::ToolCall {
            name: "web_fetch".into(),
            args: serde_json::json!({"url":"https://example.test/long-page"}),
            iteration: Some(1),
        });
        stats.apply_event(&ChatEvent::ToolResult {
            name: "web_fetch".into(),
            result: "content ".repeat(200),
            success: true,
            elapsed_ms: Some(42),
        });

        assert!(stats.messages >= 4);
        assert!(stats.used_tokens() > 65);
        assert!(stats.running);
    }
}

/// GET /api/sessions/:id/messages - get session messages (with ownership check).
pub(crate) async fn api_get_session_messages(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let caller = auth_user::extract_user_from_headers(&headers, &state.cookie_secret);
    let uuid = Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let sessions = state.essaim_sessions.read().await;
    match sessions.get(&uuid) {
        Some(session) if session.user_id.is_some() && session.user_id != caller => {
            Err(StatusCode::FORBIDDEN)
        }
        Some(session) => {
            let messages: Vec<serde_json::Value> = session
                .messages
                .iter()
                .filter_map(session_message_for_client)
                .collect();
            Ok(Json(serde_json::json!({
                "session_id": id,
                "title": session.title,
                "messages": messages,
            })))
        }
        None => {
            // Fallback: try loading from disk
            drop(sessions);
            let path = std::path::Path::new("sessions").join(format!("{}.json", id));
            if let Ok(session) = Session::charger(&path) {
                let messages: Vec<serde_json::Value> = session
                    .messages
                    .iter()
                    .filter_map(session_message_for_client)
                    .collect();
                state.essaim_sessions.write().await.insert(uuid, session);
                Ok(Json(
                    serde_json::json!({"session_id":id,"messages":messages}),
                ))
            } else {
                Err(StatusCode::NOT_FOUND)
            }
        }
    }
}

/// GET /api/sessions/search?q=query - search across all sessions.
pub(crate) async fn api_search_sessions(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let caller = auth_user::extract_user_from_headers(&headers, &state.cookie_secret);
    let query = params
        .get("q")
        .map(|s| s.to_lowercase())
        .unwrap_or_default();
    if query.is_empty() {
        return Json(serde_json::json!([]));
    }

    let sessions = state.essaim_sessions.read().await;
    let mut results = Vec::new();

    for session in sessions.values() {
        // Only search user's own sessions + legacy
        if session.user_id.is_some() && session.user_id != caller {
            continue;
        }
        for msg in &session.messages {
            let text = match msg {
                laruche_essaim::Message::User(t) | laruche_essaim::Message::Assistant(t) => {
                    t.clone()
                }
                laruche_essaim::Message::UserMultimodal { text, .. } => text.clone(),
                _ => continue,
            };
            if text.to_lowercase().contains(&query) {
                let preview: String = text.chars().take(150).collect();
                results.push(serde_json::json!({
                    "session_id": session.id.to_string(),
                    "session_title": session.title,
                    "role": match msg {
                        laruche_essaim::Message::User(_) | laruche_essaim::Message::UserMultimodal { .. } => "user",
                        _ => "assistant",
                    },
                    "preview": preview,
                }));
                if results.len() >= 20 {
                    break;
                }
            }
        }
        if results.len() >= 20 {
            break;
        }
    }

    Json(serde_json::json!(results))
}

/// GET /api/sessions/:id/export - export a session as Markdown.
// TODO: Add PDF export support (e.g. via printpdf or headless Chrome).
//       For now, only Markdown export is implemented.
pub(crate) async fn api_export_session(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<String, StatusCode> {
    let caller = auth_user::extract_user_from_headers(&headers, &state.cookie_secret);
    let uuid = Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let sessions = state.essaim_sessions.read().await;
    let session = sessions.get(&uuid).ok_or(StatusCode::NOT_FOUND)?;
    if session.user_id.is_some() && session.user_id != caller {
        return Err(StatusCode::FORBIDDEN);
    }

    let mut md = format!(
        "# {}\n\n*Session: {} | Model: {} | Date: {}*\n\n---\n\n",
        session.title.as_deref().unwrap_or("Conversation"),
        session.id,
        session.model,
        session.created_at.format("%Y-%m-%d %H:%M"),
    );

    for msg in &session.messages {
        match msg {
            laruche_essaim::Message::User(text) => {
                md.push_str(&format!("## User\n\n{}\n\n", text));
            }
            laruche_essaim::Message::UserMultimodal { text, attachments } => {
                md.push_str(&format!(
                    "## User\n\n{}\n\n*({} attachment(s) attached)*\n\n",
                    text,
                    attachments.len()
                ));
            }
            laruche_essaim::Message::Assistant(text) => {
                // Strip tool_call tags
                let mut clean = text.clone();
                while let Some(s) = clean.find("<tool_call>") {
                    if let Some(e) = clean.find("</tool_call>") {
                        clean = format!("{}{}", &clean[..s], &clean[e + "</tool_call>".len()..]);
                    } else {
                        clean.truncate(s);
                        break;
                    }
                }
                // Strip plan tags
                while let Some(s) = clean.find("<plan>") {
                    if let Some(e) = clean.find("</plan>") {
                        clean = format!("{}{}", &clean[..s], &clean[e + "</plan>".len()..]);
                    } else {
                        clean.truncate(s);
                        break;
                    }
                }
                let clean = clean.trim();
                if !clean.is_empty() {
                    md.push_str(&format!("## Assistant\n\n{}\n\n", clean));
                }
            }
            laruche_essaim::Message::Observation { tool, result, .. } => {
                md.push_str(&format!(
                    "> **Tool: {}**\n> ```\n> {}\n> ```\n\n",
                    tool,
                    &result[..result.len().min(500)]
                ));
            }
            _ => {}
        }
    }

    Ok(md)
}

/// POST /api/sessions/:id/fork - fork (branch) a session (with ownership check).
pub(crate) async fn api_fork_session(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let caller = auth_user::extract_user_from_headers(&headers, &state.cookie_secret);
    let uuid = Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let sessions_dir = std::path::Path::new("sessions");
    let current_model = state.essaim_config.read().await.model.clone();

    let mut sessions = state.essaim_sessions.write().await;
    let original = sessions.get(&uuid).ok_or(StatusCode::NOT_FOUND)?;
    if original.user_id.is_some() && original.user_id != caller {
        return Err(StatusCode::FORBIDDEN);
    }
    let mut forked = original.fork(&current_model, sessions_dir);
    // Inherit user_id from parent
    forked.user_id = caller;
    let forked_id = forked.id;

    if let Err(e) = forked.sauvegarder() {
        tracing::warn!(error = %e, "Failed to save forked session");
    }

    sessions.insert(forked_id, forked);

    Ok(Json(serde_json::json!({
        "id": forked_id.to_string(),
        "message": "Session forked successfully",
    })))
}
