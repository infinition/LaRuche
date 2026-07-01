//! Local/system HTTP endpoints (cwd, local media, onboarding, file suggest, RPC, model preload, webhook) - split out of main.rs.

use crate::*;
use axum::extract::{Query, State};
use axum::response::{IntoResponse, Json};
use axum::http::StatusCode;
use std::sync::Arc;

/// GET /api/cwd: get current working directory.
pub(crate) async fn api_get_cwd() -> Json<serde_json::Value> {
    let cwd = std::env::current_dir()
        .unwrap_or_default()
        .display()
        .to_string();
    Json(serde_json::json!({"cwd": cwd}))
}

/// POST /api/cwd: set current working directory.
pub(crate) async fn api_set_cwd(Json(body): Json<serde_json::Value>) -> Json<serde_json::Value> {
    let path = body["cwd"].as_str().unwrap_or("");
    if path.is_empty() {
        return Json(serde_json::json!({"error": "cwd is required"}));
    }
    let p = std::path::Path::new(path);
    if !p.exists() || !p.is_dir() {
        return Json(serde_json::json!({"error": format!("Directory not found: {}", path)}));
    }
    match std::env::set_current_dir(p) {
        Ok(()) => {
            info!(cwd = path, "Working directory changed");
            Json(serde_json::json!({"cwd": path, "status": "ok"}))
        }
        Err(e) => Json(serde_json::json!({"error": e.to_string()})),
    }
}

/// GET /api/media/local?path=...: serves an explicitly selected local media file.
///
/// The route is intentionally confined to the current workspace. `media_present`
/// applies the same restriction before it ever produces a local-media card.
#[derive(Deserialize)]
pub(crate) struct LocalMediaQuery {
    path: String,
}

pub(crate) async fn api_media_local(
    Query(query): Query<LocalMediaQuery>,
) -> Result<axum::response::Response, StatusCode> {
    const MAX_LOCAL_MEDIA_BYTES: u64 = 250 * 1024 * 1024;

    let root = std::env::current_dir()
        .ok()
        .and_then(|path| std::fs::canonicalize(path).ok())
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let path = std::fs::canonicalize(&query.path).map_err(|_| StatusCode::NOT_FOUND)?;
    if !path.starts_with(&root) {
        return Err(StatusCode::FORBIDDEN);
    }
    let metadata = std::fs::metadata(&path).map_err(|_| StatusCode::NOT_FOUND)?;
    if !metadata.is_file() {
        return Err(StatusCode::NOT_FOUND);
    }
    if metadata.len() > MAX_LOCAL_MEDIA_BYTES {
        return Err(StatusCode::PAYLOAD_TOO_LARGE);
    }
    let bytes = tokio::fs::read(&path)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mime = local_media_mime(&path);
    Ok((
        [
            (axum::http::header::CONTENT_TYPE, mime),
            (axum::http::header::CACHE_CONTROL, "no-store"),
        ],
        bytes,
    )
        .into_response())
}

fn local_media_mime(path: &std::path::Path) -> &'static str {
    match path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "avif" => "image/avif",
        "pdf" => "application/pdf",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "ogv" => "video/ogg",
        "mov" => "video/quicktime",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "ogg" => "audio/ogg",
        "m4a" => "audio/mp4",
        "flac" => "audio/flac",
        _ => "application/octet-stream",
    }
}

pub(crate) async fn api_onboarding(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let mut steps = Vec::new();

    // 1. Ollama installed?
    let ec = state.essaim_config.read().await;
    let ollama_ok = reqwest::Client::new()
        .get(format!("{}/api/tags", ec.ollama_url))
        .timeout(std::time::Duration::from_secs(3))
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false);
    steps.push(serde_json::json!({
        "step": 1, "title": "Ollama",
        "done": ollama_ok,
        "instruction": if ollama_ok { "Ollama is connected." }
            else { "Install Ollama: https://ollama.com/download" },
    }));

    // 2. LLM model configured?
    steps.push(serde_json::json!({
        "step": 2, "title": "LLM Model",
        "done": ollama_ok,
        "instruction": format!("Current model: {}. For Gemma 4: ollama pull gemma4:e4b", ec.model),
    }));
    let _ = ec;

    // 3. Embedding model (semantic memory)? REAL probe — this was a hardcoded
    // `done: false` stub. We ask for an actual vector through the same client the
    // memory uses (HttpEmbedder: Ollama `/api/embed` or llama.cpp `/v1/embeddings`,
    // format auto-detected), so the check reflects the truth whatever the backend.
    let embed_url = std::env::var("LARUCHE_EMBED_URL")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "http://127.0.0.1:11434".to_string());
    let embed_model = std::env::var("LARUCHE_EMBED_MODEL")
        .unwrap_or_else(|_| "nomic-embed-text".to_string());
    let embed_ok = {
        use laruche_memoire::Embedder;
        laruche_memoire::HttpEmbedder::new(&embed_url, &embed_model)
            .embed("ping")
            .await
            .map(|v| !v.is_empty())
            .unwrap_or(false)
    };
    steps.push(serde_json::json!({
        "step": 3, "title": "Embeddings Model (RAG)",
        "done": embed_ok,
        "instruction": if embed_ok {
            format!("Semantic memory active: {embed_model} @ {embed_url}")
        } else {
            format!("Run lancer_embeddings.bat (auto-download), or: ollama pull {embed_model}")
        },
    }));

    // 4. Voice services?
    let listener = state.listener.read().await;
    let nodes = listener.get_nodes().await;
    let has_stt = nodes.values().any(|n| {
        n.manifest
            .capabilities
            .iter()
            .any(|c| c.to_string() == "stt")
    });
    let has_tts = nodes.values().any(|n| {
        n.manifest
            .capabilities
            .iter()
            .any(|c| c.to_string() == "tts")
    });
    steps.push(serde_json::json!({
        "step": 4, "title": "Voice services (STT/TTS)",
        "done": has_stt && has_tts,
        "instruction": if has_stt && has_tts { "STT and TTS available." }
            else { "Run: cd laruche-voix && python -m src.stt_service && python -m src.tts_service" },
    }));

    // 5. Chrome for browser tools?
    let has_chrome = if cfg!(windows) {
        std::path::Path::new(r"C:\Program Files\Google\Chrome\Application\chrome.exe").exists()
    } else {
        which::which("google-chrome").is_ok()
    };
    steps.push(serde_json::json!({
        "step": 5, "title": "Chrome/Edge (browser tools)",
        "done": has_chrome,
        "instruction": if has_chrome { "Chrome detected." } else { "Install Chrome for browser_navigate/screenshot." },
    }));

    let done_count = steps
        .iter()
        .filter(|s| s["done"].as_bool().unwrap_or(false))
        .count();

    Json(serde_json::json!({
        "progress": format!("{}/{}", done_count, steps.len()),
        "complete": done_count == steps.len(),
        "steps": steps,
    }))
}

/// GET /api/files/suggest?q=partial_path: autocomplete file paths.
pub(crate) async fn api_files_suggest(
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let query = params.get("q").map(|s| s.as_str()).unwrap_or(".");
    let path = std::path::Path::new(query);

    // Determine the directory to list and the prefix to match
    let (dir, prefix) = if path.is_dir() {
        (path.to_path_buf(), String::new())
    } else {
        let parent = path.parent().unwrap_or(std::path::Path::new("."));
        let prefix = path
            .file_name()
            .map(|f| f.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        (parent.to_path_buf(), prefix)
    };

    let mut suggestions = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten().take(20) {
            let name = entry.file_name().to_string_lossy().to_string();
            if prefix.is_empty() || name.to_lowercase().starts_with(&prefix) {
                let full_path = entry.path().display().to_string();
                let is_dir = entry.path().is_dir();
                suggestions.push(serde_json::json!({
                    "name": name,
                    "path": full_path,
                    "is_dir": is_dir,
                }));
            }
        }
    }

    Json(serde_json::json!(suggestions))
}

/// POST /api/rpc: Remote Procedure Call between Miel nodes.
/// Body: {"method": "infer|status|tools|ping", "params": {...}}
/// Allows nodes to invoke capabilities on each other.
pub(crate) async fn api_rpc(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    *state.last_activity.write().await = std::time::Instant::now();
    let method = body["method"].as_str().unwrap_or("");
    let params = &body["params"];

    match method {
        "ping" => {
            let manifest = state.manifest.read().await;
            Json(serde_json::json!({
                "result": "pong",
                "node": state.config.node_name,
                "uptime_secs": manifest.uptime_secs,
            }))
        }
        "tools" => Json(serde_json::json!({
            "result": state.essaim_registry.noms(),
        })),
        "status" => {
            let manifest = state.manifest.read().await;
            Json(serde_json::json!({
                "result": {
                    "node_name": manifest.node_name,
                    "tier": format!("{:?}", manifest.hardware_tier),
                    "cpu_pct": manifest.resources.cpu_usage_pct,
                    "memory_used_mb": manifest.resources.memory_used_mb,
                    "tokens_per_sec": manifest.performance.tokens_per_sec,
                    "queue_depth": manifest.performance.queue_depth,
                }
            }))
        }
        "execute_tool" => {
            let tool_name = params["name"].as_str().unwrap_or("");
            let tool_args = params["arguments"].clone();
            let ctx = laruche_essaim::ContextExecution::default();
            match state
                .essaim_registry
                .executer(tool_name, tool_args, &ctx)
                .await
            {
                Ok(result) => Json(serde_json::json!({
                    "result": {
                        "success": result.success,
                        "output": result.output,
                        "error": result.error,
                    }
                })),
                Err(e) => Json(serde_json::json!({ "error": e.to_string() })),
            }
        }
        _ => Json(serde_json::json!({
            "error": format!("Unknown RPC method: '{}'. Available: ping, tools, status, execute_tool", method),
        })),
    }
}

/// POST /api/preload: preload a model into Ollama VRAM.
/// Sends a minimal generate request to warm up the model.
pub(crate) async fn api_preload(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let default_model = state.essaim_config.read().await.model.clone();
    let model = body["model"].as_str().unwrap_or(&default_model).to_string();

    info!(model = %model, "Preloading model into Ollama");
    let start = std::time::Instant::now();

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    // Ollama loads the model on first request. Send a minimal prompt.
    let result = client
        .post(format!("{}/api/generate", state.config.ollama_url))
        .json(&serde_json::json!({
            "model": model,
            "prompt": "",
            "stream": false,
            "options": { "num_predict": 1 },
            "keep_alive": "10m",
        }))
        .send()
        .await;

    let elapsed = start.elapsed().as_millis() as u64;

    match result {
        Ok(resp) if resp.status().is_success() => {
            info!(model = %model, elapsed_ms = elapsed, "Model preloaded");
            Json(serde_json::json!({
                "status": "loaded",
                "model": model,
                "elapsed_ms": elapsed,
            }))
        }
        Ok(resp) => {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            warn!(model = %model, status = %status, "Preload failed");
            Json(serde_json::json!({
                "status": "error",
                "error": format!("Ollama {}: {}", status, &body[..body.len().min(200)]),
            }))
        }
        Err(e) => {
            warn!(model = %model, error = %e, "Preload failed");
            Json(serde_json::json!({
                "status": "error",
                "error": e.to_string(),
            }))
        }
    }
}

/// POST /api/webhook: trigger the agent via HTTP (for external integrations).
/// Body: {"prompt": "...", "model": "optional-model-override"}
/// Returns: {"response": "...", "session_id": "..."}
pub(crate) async fn api_webhook(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let prompt = body["prompt"].as_str().ok_or(StatusCode::BAD_REQUEST)?;
    let prompt_for_agent = inject_no_think(prompt, body["no_think"].as_bool().unwrap_or(false));
    let model_override = body["model"].as_str().map(|s| s.to_string());

    // Use current dynamic default model, not initial config
    let current_model = get_llm_default(&state).await;
    let sessions_dir = std::path::Path::new("sessions");
    let session_id = uuid::Uuid::new_v4();
    let mut session = Session::new_with_id(session_id, &current_model, sessions_dir);

    let mut config = state.essaim_config.read().await.clone();
    // Explicit per-request model wins; otherwise the "web" channel override (Settings > Channels),
    // which falls back to the global active model when unset.
    match model_override {
        Some(m) => config.model = m,
        None => {
            config.model = current_model;
            apply_channel_model(&state, "web", &mut config).await;
        }
    }

    let (tx, mut rx) = broadcast::channel::<ChatEvent>(256);

    let result = boucle_react_memoire(
        &prompt_for_agent,
        &mut session,
        &state.essaim_registry,
        &config,
        &tx,
        state.memoire.clone(),
    )
    .await;

    // Collect events for the response
    drop(tx);
    let mut tools_used: Vec<serde_json::Value> = Vec::new();
    let mut plan_items: Vec<serde_json::Value> = Vec::new();
    while let Ok(event) = rx.try_recv() {
        match event {
            ChatEvent::ToolCall { name, args, .. } => {
                tools_used.push(serde_json::json!({"name": name, "args": args}));
            }
            ChatEvent::ToolResult {
                name,
                success,
                elapsed_ms,
                ..
            } => {
                if let Some(last) = tools_used.last_mut() {
                    if last["name"].as_str() == Some(&name) {
                        last["success"] = serde_json::json!(success);
                        last["elapsed_ms"] = serde_json::json!(elapsed_ms);
                    }
                }
            }
            ChatEvent::Plan { items } => {
                plan_items = items
                    .iter()
                    .map(|i| serde_json::json!({"task": i.task, "status": i.status}))
                    .collect();
            }
            _ => {}
        }
    }

    // Save session
    session.auto_title();
    let _ = session.sauvegarder();
    // Sync to peers
    let sync_state = state.clone();
    let sync_session = session.clone();
    tokio::spawn(async move {
        sync::push_session_to_peers(&sync_session, &sync_state).await;
    });
    state
        .essaim_sessions
        .write()
        .await
        .insert(session_id, session);

    match result {
        Ok(response) => Ok(Json(serde_json::json!({
            "response": response,
            "session_id": session_id.to_string(),
            "tools_used": tools_used,
            "plan": plan_items,
        }))),
        Err(e) => Ok(Json(serde_json::json!({
            "error": e.to_string(),
            "session_id": session_id.to_string(),
        }))),
    }
}

