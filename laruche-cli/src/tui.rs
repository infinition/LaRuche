//! LaRuche TUI — Rich terminal interface connected to a LaRuche server.
//!
//! Connects to a LaRuche server via WebSocket (/ws/chat) for agent capabilities.
//! Falls back to direct Ollama if no server found.

use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    execute,
};
use futures_util::{SinkExt, StreamExt};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame, Terminal,
};
use std::io;
use tokio_tungstenite::tungstenite::Message as WsMessage;

/// Events sent from the WebSocket background task to the TUI main loop.
#[derive(Debug, Clone)]
enum TuiEvent {
    /// A single token to append to the current streaming response.
    Token(String),
    /// The agent is calling a tool.
    ToolCall { name: String, args: String },
    /// A tool has returned a result.
    ToolResult { name: String, success: bool, ms: u64 },
    /// Plan update from the agent.
    Plan(Vec<(String, String)>),
    /// Agent thinking / reasoning trace.
    Thinking(String),
    /// Stream finished — the full response is included.
    Done(String),
    /// An error occurred.
    Error(String),
}

const AMBER: Color = Color::Rgb(245, 158, 11);
const BG: Color = Color::Rgb(9, 9, 11);
const BG_PANEL: Color = Color::Rgb(17, 17, 19);
const BORDER: Color = Color::Rgb(42, 42, 46);
const TEXT_DIM: Color = Color::Rgb(113, 113, 122);

#[derive(Clone)]
struct ChatMessage {
    role: String,
    text: String,
}

struct App {
    input: String,
    cursor_pos: usize,
    messages: Vec<ChatMessage>,
    chat_scroll: u16,
    tools: Vec<String>,
    plan: Vec<(String, String)>,
    active_panel: Panel,
    sidebar_panel: SidebarPanel,
    model: String,
    server_url: String,
    cwd: String,
    #[allow(dead_code)]
    tokens: usize,
    status_msg: String,
    is_streaming: bool,
    should_quit: bool,
    session_id: Option<String>,
    connected: bool,
    // History
    history: Vec<String>,
    history_idx: Option<usize>,
    history_draft: String,
    // Autocomplete
    autocomplete_suggestion: String,
    // Chat view toggle
    chat_view: ChatView,
    activity_log: Vec<String>,
    // Models list for sidebar picker
    available_models: Vec<String>,
    model_cursor: usize,
    // WebSocket streaming channel
    event_rx: Option<tokio::sync::mpsc::Receiver<TuiEvent>>,
    // Buffer for tokens as they stream in
    streaming_response: String,
    // Auth
    auth_token: Option<String>,
    user_name: Option<String>,
    user_role: Option<String>,
}

#[derive(PartialEq, Clone)]
enum SidebarPanel { Tools, Models, Sessions, Plan }

#[derive(PartialEq, Clone)]
enum ChatView { Messages, Activity, Status }

#[derive(PartialEq, Clone)]
enum Panel { Input, Chat, Sidebar }

impl App {
    async fn new() -> Self {
        let server_url = discover_server().await;
        let connected = !server_url.is_empty();
        let mut model = fetch_model(&server_url).await;
        let tools = fetch_tools(&server_url).await;
        let cwd = std::env::current_dir().unwrap_or_default().display().to_string();

        let welcome = if connected {
            format!("Connecte a {} — pret !", server_url)
        } else {
            "Aucun serveur LaRuche trouve. Lancez: cargo run -p laruche-node".to_string()
        };

        let mut auth_token = None;
        let mut user_name = None;
        let mut user_role = None;

        // Load persisted config (including auth token)
        let cfg_path = dirs_config_path();
        let mut saved_model = String::new();
        if let Ok(content) = std::fs::read_to_string(&cfg_path) {
            if let Ok(cfg) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(m) = cfg["model"].as_str() { saved_model = m.to_string(); }
                if let Some(t) = cfg["auth_token"].as_str() { auth_token = Some(t.to_string()); }
                if let Some(n) = cfg["user_name"].as_str() { user_name = Some(n.to_string()); }
                if let Some(r) = cfg["user_role"].as_str() { user_role = Some(r.to_string()); }
            }
        }

        // Verify auth token with server if connected
        if connected && auth_token.is_some() {
            let check = reqwest::Client::new()
                .get(format!("{}/api/auth/me", &server_url))
                .header("Cookie", format!("laruche_auth={}", auth_token.as_deref().unwrap_or("")))
                .send().await;
            match check {
                Ok(r) if r.status().is_success() => {
                    if let Ok(data) = r.json::<serde_json::Value>().await {
                        user_name = data["display_name"].as_str().map(|s| s.to_string());
                        user_role = data["role"].as_str().map(|s| s.to_string());
                    }
                }
                _ => { auth_token = None; user_name = None; user_role = None; }
            }
        }

        if !saved_model.is_empty() { model = saved_model; }

        let auth_info = match (&user_name, &user_role) {
            (Some(n), Some(r)) => format!(" | {} ({})", n, r),
            _ => String::new(),
        };

        let welcome_msg = if connected {
            format!("Connecte a {}{}", server_url, auth_info)
        } else {
            welcome
        };

        let app = App {
            input: String::new(), cursor_pos: 0,
            messages: vec![ChatMessage { role: "system".into(), text: welcome_msg }],
            chat_scroll: 0, tools, plan: vec![],
            active_panel: Panel::Input, sidebar_panel: SidebarPanel::Tools,
            model, server_url, cwd,
            tokens: 0, status_msg: if connected { "Connecte".into() } else { "Deconnecte".into() },
            is_streaming: false, should_quit: false,
            session_id: None, connected,
            history: Vec::new(), history_idx: None, history_draft: String::new(),
            autocomplete_suggestion: String::new(),
            chat_view: ChatView::Messages,
            activity_log: Vec::new(),
            available_models: Vec::new(),
            model_cursor: 0,
            event_rx: None,
            streaming_response: String::new(),
            auth_token, user_name, user_role,
        };
        app
    }

    fn save_config(&self) {
        let path = dirs_config_path();
        if let Some(parent) = std::path::Path::new(&path).parent() { let _ = std::fs::create_dir_all(parent); }
        let cfg = serde_json::json!({
            "model": self.model,
            "server_url": self.server_url,
            "auth_token": self.auth_token,
            "user_name": self.user_name,
            "user_role": self.user_role,
        });
        let _ = std::fs::write(&path, serde_json::to_string_pretty(&cfg).unwrap_or_default());
    }
}

fn dirs_config_path() -> String {
    if let Ok(home) = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")) {
        format!("{}/.laruche/cli-config.json", home)
    } else {
        "cli-config.json".to_string()
    }
}

/// Discover a LaRuche server: LARUCHE_URL env → mDNS → localhost probe.
async fn discover_server() -> String {
    // 1. Explicit URL
    if let Ok(url) = std::env::var("LARUCHE_URL") {
        if probe_server(&url).await { return url; }
    }

    // 2. mDNS discovery via laruche-client
    if let Ok(lr) = laruche_client::LaRuche::discover().await {
        for node in lr.nodes() {
            if let Some(url) = node.manifest.api_url() {
                if probe_server(&url).await { return url; }
            }
        }
    }

    // 3. Localhost probe
    let local = "http://127.0.0.1:8419".to_string();
    if probe_server(&local).await { return local; }

    String::new()
}

async fn probe_server(url: &str) -> bool {
    reqwest::Client::builder().timeout(std::time::Duration::from_secs(2)).build().ok()
        .and_then(|c| {
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    c.get(format!("{}/health", url)).send().await.ok().map(|r| r.status().is_success())
                })
            })
        })
        .unwrap_or(false)
}

async fn fetch_model(url: &str) -> String {
    if url.is_empty() { return std::env::var("LARUCHE_MODEL").unwrap_or_else(|_| "?".into()); }
    reqwest::Client::new().get(format!("{}/models", url)).send().await.ok()
        .and_then(|r| tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(r.json::<serde_json::Value>())).ok())
        .and_then(|d| d["models"].as_array()?.first()?.get("name")?.as_str().map(String::from))
        .unwrap_or_else(|| "?".into())
}

async fn fetch_tools(url: &str) -> Vec<String> {
    if url.is_empty() { return vec![]; }
    reqwest::Client::new().get(format!("{}/api/tools", url)).send().await.ok()
        .and_then(|r| tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(r.json::<serde_json::Value>())).ok())
        .and_then(|d| d.as_array().map(|a| a.iter().filter_map(|t| t["name"].as_str().map(String::from)).collect()))
        .unwrap_or_default()
}

/// Connect to ws://{server}/ws/chat, send the message, and stream TuiEvents
/// back through the channel. Falls back to POST /api/webhook on WS failure.
async fn stream_via_websocket(
    url: String,
    text: String,
    model: String,
    auth_token: Option<String>,
    _session_id: Option<String>,
    tx: tokio::sync::mpsc::Sender<TuiEvent>,
) {
    // Build the WebSocket URL: http://host:port -> ws://host:port/ws/chat
    let ws_url = format!(
        "ws://{}/ws/chat",
        url.trim_start_matches("https://").trim_start_matches("http://")
    );

    // Build WS request with auth cookie if available
    let ws_request = if let Some(ref token) = auth_token {
        tokio_tungstenite::tungstenite::http::Request::builder()
            .uri(&ws_url)
            .header("Cookie", format!("laruche_auth={}", token))
            .header("Host", url.trim_start_matches("https://").trim_start_matches("http://"))
            .header("Connection", "Upgrade")
            .header("Upgrade", "websocket")
            .header("Sec-WebSocket-Version", "13")
            .header("Sec-WebSocket-Key", tokio_tungstenite::tungstenite::handshake::client::generate_key())
            .body(())
            .unwrap()
    } else {
        tokio_tungstenite::tungstenite::http::Request::builder()
            .uri(&ws_url)
            .header("Host", url.trim_start_matches("https://").trim_start_matches("http://"))
            .header("Connection", "Upgrade")
            .header("Upgrade", "websocket")
            .header("Sec-WebSocket-Version", "13")
            .header("Sec-WebSocket-Key", tokio_tungstenite::tungstenite::handshake::client::generate_key())
            .body(())
            .unwrap()
    };

    // Try WebSocket first
    match tokio_tungstenite::connect_async(ws_request).await {
        Ok((ws_stream, _)) => {
            let (mut write, mut read) = ws_stream.split();

            // Send the message
            let payload = serde_json::json!({ "type": "message", "text": text, "model": model });
            if let Err(e) = write.send(WsMessage::Text(payload.to_string().into())).await {
                let _ = tx.send(TuiEvent::Error(format!("WS send error: {}", e))).await;
                let _ = tx.send(TuiEvent::Done(String::new())).await;
                return;
            }

            let mut full_response = String::new();

            // Read events from the stream
            while let Some(msg_result) = read.next().await {
                match msg_result {
                    Ok(WsMessage::Text(raw)) => {
                        let text_str: &str = raw.as_ref();
                        if let Ok(data) = serde_json::from_str::<serde_json::Value>(text_str) {
                            let event_type = data["type"].as_str().unwrap_or("");
                            match event_type {
                                "token" => {
                                    let tok = data["text"].as_str().unwrap_or("").to_string();
                                    full_response.push_str(&tok);
                                    let _ = tx.send(TuiEvent::Token(tok)).await;
                                }
                                "tool_call" => {
                                    let name = data["name"].as_str().unwrap_or("?").to_string();
                                    let args = data["args"].as_str()
                                        .or_else(|| data["arguments"].as_str())
                                        .unwrap_or("").to_string();
                                    let _ = tx.send(TuiEvent::ToolCall { name, args }).await;
                                }
                                "tool_result" => {
                                    let name = data["name"].as_str().unwrap_or("?").to_string();
                                    let success = data["success"].as_bool().unwrap_or(true);
                                    let ms = data["elapsed_ms"].as_u64()
                                        .or_else(|| data["ms"].as_u64())
                                        .unwrap_or(0);
                                    let _ = tx.send(TuiEvent::ToolResult { name, success, ms }).await;
                                }
                                "plan" => {
                                    let steps: Vec<(String, String)> = data["steps"]
                                        .as_array()
                                        .map(|arr| arr.iter().map(|s| {
                                            let task = s["task"].as_str().unwrap_or("?").to_string();
                                            let status = s["status"].as_str().unwrap_or("pending").to_string();
                                            (task, status)
                                        }).collect())
                                        .unwrap_or_default();
                                    let _ = tx.send(TuiEvent::Plan(steps)).await;
                                }
                                "thinking" => {
                                    let thought = data["text"].as_str().unwrap_or("").to_string();
                                    let _ = tx.send(TuiEvent::Thinking(thought)).await;
                                }
                                "done" => {
                                    // The server may send the full response in "text"
                                    let final_text = data["text"].as_str()
                                        .map(|s| s.to_string())
                                        .unwrap_or(full_response.clone());
                                    let _ = tx.send(TuiEvent::Done(final_text)).await;
                                    return;
                                }
                                "error" => {
                                    let err = data["text"].as_str()
                                        .or_else(|| data["message"].as_str())
                                        .unwrap_or("Unknown error").to_string();
                                    let _ = tx.send(TuiEvent::Error(err)).await;
                                    let _ = tx.send(TuiEvent::Done(full_response.clone())).await;
                                    return;
                                }
                                _ => {
                                    // Unknown event type — if it has text, treat as token
                                    if let Some(t) = data["text"].as_str() {
                                        full_response.push_str(t);
                                        let _ = tx.send(TuiEvent::Token(t.to_string())).await;
                                    }
                                }
                            }
                        }
                    }
                    Ok(WsMessage::Close(_)) => {
                        // Server closed the connection — finalize
                        let _ = tx.send(TuiEvent::Done(full_response)).await;
                        return;
                    }
                    Err(e) => {
                        let _ = tx.send(TuiEvent::Error(format!("WS read error: {}", e))).await;
                        let _ = tx.send(TuiEvent::Done(full_response)).await;
                        return;
                    }
                    _ => {} // Ping/Pong/Binary — ignore
                }
            }

            // Stream ended without explicit done
            let _ = tx.send(TuiEvent::Done(full_response)).await;
        }
        Err(ws_err) => {
            // WebSocket connection failed — fall back to HTTP POST /api/webhook
            let _ = tx.send(TuiEvent::Thinking(format!(
                "WS failed ({}), falling back to HTTP...", ws_err
            ))).await;
            fallback_http_send(&url, &text, &tx).await;
        }
    }
}

/// Fallback: POST /api/webhook and send the result as a single Done event.
async fn fallback_http_send(
    url: &str,
    text: &str,
    tx: &tokio::sync::mpsc::Sender<TuiEvent>,
) {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .unwrap();

    let body = serde_json::json!({ "prompt": text });
    let resp = client
        .post(format!("{}/api/webhook", url))
        .json(&body)
        .send()
        .await;

    match resp {
        Ok(r) => {
            if let Ok(data) = r.json::<serde_json::Value>().await {
                if let Some(err) = data["error"].as_str() {
                    if !err.is_empty() {
                        let _ = tx.send(TuiEvent::Error(format!("Erreur serveur: {}", err))).await;
                        let _ = tx.send(TuiEvent::Done(String::new())).await;
                        return;
                    }
                }
                let response = data["response"].as_str().unwrap_or("").to_string();
                // Clean tool_call/plan tags
                let mut clean = response.clone();
                while let Some(s) = clean.find("<tool_call>") {
                    if let Some(e) = clean.find("</tool_call>") {
                        clean = format!("{}{}", &clean[..s], &clean[e + "</tool_call>".len()..]);
                    } else {
                        clean.truncate(s);
                        break;
                    }
                }
                while let Some(s) = clean.find("<plan>") {
                    if let Some(e) = clean.find("</plan>") {
                        clean = format!("{}{}", &clean[..s], &clean[e + "</plan>".len()..]);
                    } else {
                        clean.truncate(s);
                        break;
                    }
                }
                // Extract tool info for activity log
                if let Some(tools) = data["tools_used"].as_array() {
                    for t in tools {
                        let name = t["name"].as_str().unwrap_or("?").to_string();
                        let ms = t["elapsed_ms"].as_u64().unwrap_or(0);
                        let ok = t["success"].as_bool().unwrap_or(true);
                        let _ = tx.send(TuiEvent::ToolResult { name, success: ok, ms }).await;
                    }
                }
                let _ = tx.send(TuiEvent::Done(clean.trim().to_string())).await;
            } else {
                let _ = tx.send(TuiEvent::Error("Error parsing response".into())).await;
                let _ = tx.send(TuiEvent::Done(String::new())).await;
            }
        }
        Err(e) => {
            let _ = tx.send(TuiEvent::Error(format!("HTTP error: {}", e))).await;
            let _ = tx.send(TuiEvent::Done(String::new())).await;
        }
    }
}

pub async fn run_tui() -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new().await;

    loop {
        // Drain all pending TuiEvents from the WebSocket background task
        if let Some(ref mut rx) = app.event_rx {
            let mut channel_closed = false;
            let mut done_received = false;
            loop {
                match rx.try_recv() {
                    Ok(evt) => {
                        let ts = chrono::Local::now().format("%H:%M:%S").to_string();
                        match evt {
                            TuiEvent::Token(tok) => {
                                app.streaming_response.push_str(&tok);
                                app.status_msg = format!(
                                    "Streaming... ({} chars)",
                                    app.streaming_response.chars().count()
                                );
                            }
                            TuiEvent::ToolCall { name, args } => {
                                let short_args = if args.chars().count() > 40 {
                                    format!("{}...", args.chars().take(40).collect::<String>())
                                } else {
                                    args.clone()
                                };
                                app.activity_log.push(format!(
                                    "[TOOL] [{}] Appel: {} ({})",
                                    ts, name, short_args
                                ));
                                app.messages.push(ChatMessage {
                                    role: "tool".into(),
                                    text: format!("{} {}", name, short_args),
                                });
                                app.status_msg = format!("Outil: {}...", name);
                            }
                            TuiEvent::ToolResult { name, success, ms } => {
                                let icon = if success { "✓" } else { "✗" };
                                app.activity_log.push(format!(
                                    "[TOOL] [{}] {} {} ({}ms)",
                                    ts, icon, name, ms
                                ));
                                app.status_msg = format!(
                                    "{} {} ({}ms)",
                                    icon, name, ms
                                );
                            }
                            TuiEvent::Plan(steps) => {
                                app.plan = steps;
                                app.activity_log.push(format!(
                                    "[{}] Plan mis a jour ({} etapes)",
                                    ts, app.plan.len()
                                ));
                            }
                            TuiEvent::Thinking(thought) => {
                                let short = if thought.chars().count() > 60 {
                                    format!("{}...", thought.chars().take(60).collect::<String>())
                                } else {
                                    thought
                                };
                                app.activity_log.push(format!(
                                    "[THINK] [{}] {}",
                                    ts, short
                                ));
                            }
                            TuiEvent::Done(final_text) => {
                                // Use the accumulated streaming_response if we
                                // received tokens, otherwise use final_text
                                let text = if !app.streaming_response.is_empty() {
                                    std::mem::take(&mut app.streaming_response)
                                } else {
                                    final_text
                                };
                                if !text.is_empty() {
                                    app.messages.push(ChatMessage {
                                        role: "assistant".into(),
                                        text: text.clone(),
                                    });
                                    app.activity_log.push(format!(
                                        "[OK] [{}] Response: {} chars",
                                        ts,
                                        text.chars().count()
                                    ));
                                } else {
                                    app.activity_log.push(format!(
                                        "[ERR] [{}] Empty response",
                                        ts
                                    ));
                                }
                                app.is_streaming = false;
                                app.status_msg = "Pret".into();
                                app.streaming_response.clear();
                                // Auto scroll
                                let total = app.messages.iter()
                                    .map(|m| m.text.lines().count() + 1)
                                    .sum::<usize>();
                                app.chat_scroll = total.saturating_sub(12) as u16;
                                done_received = true;
                            }
                            TuiEvent::Error(err) => {
                                app.activity_log.push(format!(
                                    "[ERR] [{}] {}",
                                    ts, err
                                ));
                                app.status_msg = format!("Erreur: {}", err);
                            }
                        }
                    }
                    Err(tokio::sync::mpsc::error::TryRecvError::Empty) => break,
                    Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                        channel_closed = true;
                        break;
                    }
                }
            }
            if done_received || channel_closed {
                app.event_rx = None;
                if channel_closed && app.is_streaming {
                    // Channel died without Done event
                    if !app.streaming_response.is_empty() {
                        let text = std::mem::take(&mut app.streaming_response);
                        app.messages.push(ChatMessage {
                            role: "assistant".into(),
                            text,
                        });
                    }
                    app.is_streaming = false;
                    app.status_msg = "Erreur connexion".into();
                }
            }
        }

        terminal.draw(|f| ui(f, &mut app))?;
        if app.should_quit { break; }

        if event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != crossterm::event::KeyEventKind::Press { continue; }

                // Global
                match (key.modifiers, key.code) {
                    (KeyModifiers::CONTROL, KeyCode::Char('c')) | (KeyModifiers::CONTROL, KeyCode::Char('q')) => { app.should_quit = true; continue; }
                    _ => {}
                }

                match app.active_panel {
                    Panel::Input => handle_input(&mut app, key.code).await,
                    Panel::Chat => match key.code {
                        KeyCode::Up => app.chat_scroll = app.chat_scroll.saturating_sub(2),
                        KeyCode::Down => app.chat_scroll = app.chat_scroll.saturating_add(2),
                        KeyCode::PageUp => app.chat_scroll = app.chat_scroll.saturating_sub(10),
                        KeyCode::PageDown => app.chat_scroll = app.chat_scroll.saturating_add(10),
                        KeyCode::Right => {
                            app.chat_view = match app.chat_view {
                                ChatView::Messages => ChatView::Activity,
                                ChatView::Activity => ChatView::Status,
                                ChatView::Status => ChatView::Messages,
                            };
                            app.chat_scroll = 0;
                        }
                        KeyCode::Left => {
                            app.chat_view = match app.chat_view {
                                ChatView::Messages => ChatView::Status,
                                ChatView::Activity => ChatView::Messages,
                                ChatView::Status => ChatView::Activity,
                            };
                            app.chat_scroll = 0;
                        }
                        KeyCode::Tab => app.active_panel = Panel::Sidebar,
                        KeyCode::Esc | KeyCode::Enter => app.active_panel = Panel::Input,
                        _ => {}
                    },
                    Panel::Sidebar => match key.code {
                        KeyCode::Tab | KeyCode::Esc => app.active_panel = Panel::Input,
                        KeyCode::Left => {
                            app.sidebar_panel = match app.sidebar_panel {
                                SidebarPanel::Models => SidebarPanel::Tools,
                                SidebarPanel::Sessions => SidebarPanel::Models,
                                SidebarPanel::Plan => SidebarPanel::Sessions,
                                SidebarPanel::Tools => SidebarPanel::Plan,
                            };
                            if app.sidebar_panel == SidebarPanel::Models && app.available_models.is_empty() {
                                app.available_models = fetch_tools_or_models(&app.server_url, "models").await;
                                // Set cursor to current model
                                app.model_cursor = app.available_models.iter().position(|m| *m == app.model).unwrap_or(0);
                            }
                        }
                        KeyCode::Right => {
                            app.sidebar_panel = match app.sidebar_panel {
                                SidebarPanel::Tools => SidebarPanel::Models,
                                SidebarPanel::Models => SidebarPanel::Sessions,
                                SidebarPanel::Sessions => SidebarPanel::Plan,
                                SidebarPanel::Plan => SidebarPanel::Tools,
                            };
                            if app.sidebar_panel == SidebarPanel::Models && app.available_models.is_empty() {
                                app.available_models = fetch_tools_or_models(&app.server_url, "models").await;
                                app.model_cursor = app.available_models.iter().position(|m| *m == app.model).unwrap_or(0);
                            }
                        }
                        KeyCode::Up => {
                            if app.sidebar_panel == SidebarPanel::Models && !app.available_models.is_empty() {
                                app.model_cursor = app.model_cursor.saturating_sub(1);
                            }
                        }
                        KeyCode::Down => {
                            if app.sidebar_panel == SidebarPanel::Models && !app.available_models.is_empty() {
                                if app.model_cursor + 1 < app.available_models.len() { app.model_cursor += 1; }
                            }
                        }
                        KeyCode::Enter => {
                            if app.sidebar_panel == SidebarPanel::Models && !app.available_models.is_empty() {
                                let selected = app.available_models[app.model_cursor].clone();
                                app.model = selected.clone();
                                app.status_msg = format!("Model: {}", selected);
                                app.activity_log.push(format!("[{}] Model changed: {}", chrono::Local::now().format("%H:%M:%S"), selected));
                                app.save_config();
                                // Notify server if connected
                                if app.connected {
                                    let _ = reqwest::Client::new()
                                        .post(format!("{}/config/default_model", app.server_url))
                                        .json(&serde_json::json!({"capability":"llm","model":&selected}))
                                        .send().await;
                                }
                                app.active_panel = Panel::Input;
                            } else {
                                app.active_panel = Panel::Input;
                            }
                        }
                        _ => {}
                    },
                }
            }
        }
    }

    app.save_config();
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

async fn handle_input(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Enter => {
            // Accept autocomplete if any
            if !app.autocomplete_suggestion.is_empty() && app.input.ends_with(' ') {
                app.input.push_str(&app.autocomplete_suggestion);
                app.autocomplete_suggestion.clear();
            }
            let text = app.input.trim().to_string();
            app.input.clear(); app.cursor_pos = 0;
            app.autocomplete_suggestion.clear();
            app.history_idx = None;
            if text.is_empty() { return; }
            // Save to history
            if app.history.last().map(|h| h != &text).unwrap_or(true) {
                app.history.push(text.clone());
                if app.history.len() > 100 { app.history.remove(0); }
            }

            // Slash commands
            if text.starts_with('/') {
                match text.split_whitespace().next().unwrap_or("") {
                    "/quit"|"/q" => { app.should_quit = true; return; }
                    "/help"|"/h" => {
                        app.messages.push(ChatMessage { role:"system".into(), text:"/quit /help /clear /model /tools /cwd [path] /discover /doctor /server [cmd] /export".into() });
                        return;
                    }
                    "/clear"|"/new" => {
                        // Save current conversation title to activity
                        if !app.messages.is_empty() {
                            let ts = chrono::Local::now().format("%H:%M:%S").to_string();
                            app.activity_log.push(format!("[{}] Session fermee ({} msgs)", ts, app.messages.len()));
                        }
                        app.messages.clear();
                        app.session_id = None;
                        app.plan.clear();
                        app.status_msg = "Nouvelle conversation".into();
                        return;
                    }
                    "/tools"|"/t" => { app.active_panel = Panel::Sidebar; return; }
                    "/cwd" => {
                        let arg = text.strip_prefix("/cwd").unwrap_or("").trim();
                        if arg.is_empty() { app.messages.push(ChatMessage{role:"system".into(),text:format!("cwd: {}",app.cwd)}); }
                        else if std::path::Path::new(arg).is_dir() { std::env::set_current_dir(arg).ok(); app.cwd=arg.into(); app.status_msg=format!("cwd: {}",arg); }
                        else { app.status_msg = format!("Introuvable: {}", arg); }
                        return;
                    }
                    "/model" => {
                        let arg = text.strip_prefix("/model").unwrap_or("").trim();
                        if !arg.is_empty() { app.model = arg.to_string(); app.status_msg = format!("Model: {}", arg); }
                        else { app.messages.push(ChatMessage{role:"system".into(),text:format!("model: {}",app.model)}); }
                        return;
                    }
                    "/discover"|"/scan" => {
                        app.messages.push(ChatMessage{role:"system".into(), text:"Scanning reseau Miel...".into()});
                        app.status_msg = "Scan Miel...".into();
                        // Re-discover server
                        let url = discover_server().await;
                        if url.is_empty() {
                            app.messages.push(ChatMessage{role:"error".into(), text:"Aucun serveur LaRuche trouve".into()});
                        } else {
                            app.server_url = url.clone();
                            app.connected = true;
                            app.tools = fetch_tools(&url).await;
                            app.model = fetch_model(&url).await;
                            app.messages.push(ChatMessage{role:"system".into(), text:format!("Connecte: {}", url)});
                        }
                        app.status_msg = if app.connected {"Connecte"} else {"Deconnecte"}.into();
                        return;
                    }
                    "/doctor"|"/status" => {
                        if app.connected {
                            match reqwest::Client::new().get(format!("{}/api/doctor", app.server_url)).send().await {
                                Ok(r) => {
                                    if let Ok(d) = r.json::<serde_json::Value>().await {
                                        let mut info = format!("LaRuche: {}\n", d["status"].as_str().unwrap_or("?"));
                                        if let Some(checks) = d["checks"].as_array() {
                                            for c in checks {
                                                info.push_str(&format!("  {} {}: {}\n",
                                                    if c["status"].as_str()==Some("ok") {"✓"} else {"✗"},
                                                    c["name"].as_str().unwrap_or("?"),
                                                    c["detail"].as_str().unwrap_or("")));
                                            }
                                        }
                                        app.messages.push(ChatMessage{role:"system".into(), text:info});
                                    }
                                }
                                Err(e) => app.messages.push(ChatMessage{role:"error".into(), text:format!("Doctor: {}", e)}),
                            }
                        } else {
                            app.messages.push(ChatMessage{role:"error".into(), text:"Pas de serveur connecte".into()});
                        }
                        return;
                    }
                    "/server" => {
                        let arg = text.strip_prefix("/server").unwrap_or("").trim();
                        let sub_args: Vec<String> = arg.split_whitespace().map(|s|s.to_string()).collect();
                        match sub_args.first().map(|s|s.as_str()).unwrap_or("help") {
                            "start" => {
                                app.messages.push(ChatMessage{role:"system".into(), text:"Demarrage du serveur...".into()});
                                app.status_msg = "Demarrage...".into();
                                // Try to start
                                if let Some(exe) = super::find_server_exe() {
                                    let mut cmd = std::process::Command::new(&exe);
                                    cmd.arg("--no-tui").stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
                                    #[cfg(windows)] { use std::os::windows::process::CommandExt; cmd.creation_flags(0x00000008); }
                                    match cmd.spawn() {
                                        Ok(c) => {
                                            app.messages.push(ChatMessage{role:"system".into(), text:format!("Serveur demarre (PID: {})", c.id())});
                                            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                                            app.server_url = "http://127.0.0.1:8419".into();
                                            app.connected = true;
                                            app.tools = fetch_tools(&app.server_url).await;
                                            app.model = fetch_model(&app.server_url).await;
                                        }
                                        Err(e) => app.messages.push(ChatMessage{role:"error".into(), text:format!("Echec: {}", e)}),
                                    }
                                } else {
                                    app.messages.push(ChatMessage{role:"error".into(), text:"Binaire laruche-node introuvable. Faites: /server install".into()});
                                }
                                app.status_msg = if app.connected {"Connecte"} else {"Deconnecte"}.into();
                            }
                            "stop" => {
                                if cfg!(windows) { let _ = std::process::Command::new("taskkill").args(["/F","/IM","laruche-node.exe"]).output(); }
                                else { let _ = std::process::Command::new("pkill").args(["-f","laruche-node"]).output(); }
                                app.connected = false;
                                app.messages.push(ChatMessage{role:"system".into(), text:"Serveur arrete".into()});
                                app.status_msg = "Deconnecte".into();
                            }
                            "restart" => {
                                app.messages.push(ChatMessage{role:"system".into(), text:"Redemarrage...".into()});
                                if cfg!(windows) { let _ = std::process::Command::new("taskkill").args(["/F","/IM","laruche-node.exe"]).output(); }
                                else { let _ = std::process::Command::new("pkill").args(["-f","laruche-node"]).output(); }
                                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                                if let Some(exe) = super::find_server_exe() {
                                    let mut cmd = std::process::Command::new(&exe);
                                    cmd.arg("--no-tui").stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
                                    #[cfg(windows)] { use std::os::windows::process::CommandExt; cmd.creation_flags(0x00000008); }
                                    let _ = cmd.spawn();
                                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                                    app.connected = super::probe_running().await;
                                }
                                app.messages.push(ChatMessage{role:"system".into(), text:if app.connected {"Serveur redemarre"} else {"Echec redemarrage"}.into()});
                                app.status_msg = if app.connected {"Connecte"} else {"Deconnecte"}.into();
                            }
                            "status" => {
                                let running = super::probe_running().await;
                                app.messages.push(ChatMessage{role:"system".into(), text:if running {"Serveur: en marche"} else {"Serveur: arrete"}.into()});
                            }
                            "install" => {
                                app.messages.push(ChatMessage{role:"system".into(), text:"Installation du serveur (cargo build --release + install)...".into()});
                                if let Some(src) = super::find_source_dir() {
                                    let build = std::process::Command::new("cargo")
                                        .args(["build","--release","-p","laruche-node"])
                                        .current_dir(&src)
                                        .status();
                                    match build {
                                        Ok(s) if s.success() => {
                                            app.messages.push(ChatMessage{role:"system".into(), text:"Build release OK. Installation...".into()});
                                            let inst = std::process::Command::new("cargo")
                                                .args(["install","--path","laruche-node","--force"])
                                                .current_dir(&src)
                                                .status();
                                            match inst {
                                                Ok(s) if s.success() => app.messages.push(ChatMessage{role:"system".into(), text:"laruche-node installe avec succes".into()}),
                                                _ => app.messages.push(ChatMessage{role:"error".into(), text:"cargo install a echoue".into()}),
                                            }
                                        }
                                        _ => app.messages.push(ChatMessage{role:"error".into(), text:"Build echoue. Verifiez le toolchain Rust.".into()}),
                                    }
                                } else {
                                    app.messages.push(ChatMessage{role:"error".into(), text:"Repertoire source introuvable. Lancez depuis le dossier LaRuche.".into()});
                                }
                            }
                            "update" => {
                                app.messages.push(ChatMessage{role:"system".into(), text:"Mise a jour (git pull + rebuild)...".into()});
                                if let Some(src) = super::find_source_dir() {
                                    let _ = std::process::Command::new("git").args(["pull"]).current_dir(&src).status();
                                    let build = std::process::Command::new("cargo")
                                        .args(["build","--release","-p","laruche-node"])
                                        .current_dir(&src)
                                        .status();
                                    match build {
                                        Ok(s) if s.success() => {
                                            let _ = std::process::Command::new("cargo")
                                                .args(["install","--path","laruche-node","--force"])
                                                .current_dir(&src).status();
                                            app.messages.push(ChatMessage{role:"system".into(), text:"Mise a jour terminee. /server restart pour appliquer.".into()});
                                        }
                                        _ => app.messages.push(ChatMessage{role:"error".into(), text:"Build echoue.".into()}),
                                    }
                                } else {
                                    app.messages.push(ChatMessage{role:"error".into(), text:"Repertoire source introuvable.".into()});
                                }
                            }
                            "uninstall" => {
                                if cfg!(windows) { let _ = std::process::Command::new("taskkill").args(["/F","/IM","laruche-node.exe"]).output(); }
                                else { let _ = std::process::Command::new("pkill").args(["-f","laruche-node"]).output(); }
                                let _ = std::process::Command::new("cargo").args(["uninstall","laruche-node"]).status();
                                app.connected = false;
                                app.messages.push(ChatMessage{role:"system".into(), text:"laruche-node desinstalle".into()});
                            }
                            _ => {
                                app.messages.push(ChatMessage{role:"system".into(), text:"/server start|stop|restart|status|install|update|uninstall".into()});
                            }
                        }
                        return;
                    }
                    "/export" => {
                        let mut md = String::from("# Conversation\n\n");
                        for msg in &app.messages {
                            match msg.role.as_str() {
                                "user" => md.push_str(&format!("**User:** {}\n\n", msg.text)),
                                "assistant" => md.push_str(&format!("{}\n\n---\n\n", msg.text)),
                                _ => {}
                            }
                        }
                        let f = "conversation.md";
                        match std::fs::write(f, &md) {
                            Ok(_) => app.messages.push(ChatMessage{role:"system".into(), text:format!("Exporte: {}", f)}),
                            Err(e) => app.messages.push(ChatMessage{role:"error".into(), text:format!("Erreur: {}", e)}),
                        }
                        return;
                    }
                    "/login" => {
                        let arg = text.strip_prefix("/login").unwrap_or("").trim();
                        let parts: Vec<&str> = arg.splitn(2, ' ').collect();
                        if parts.len() < 2 || parts[0].is_empty() {
                            app.messages.push(ChatMessage{role:"system".into(), text:"/login <nom> <mot_de_passe>".into()});
                            return;
                        }
                        let name = parts[0];
                        let pw = parts[1];
                        let resp = reqwest::Client::new()
                            .post(format!("{}/api/auth/login", app.server_url))
                            .json(&serde_json::json!({"display_name": name, "password": pw}))
                            .send().await;
                        match resp {
                            Ok(r) if r.status().is_success() => {
                                // Extract cookie from Set-Cookie header
                                if let Some(cookie) = r.headers().get("set-cookie").and_then(|v| v.to_str().ok()) {
                                    if let Some(token) = cookie.split(';').next().and_then(|s| s.strip_prefix("laruche_auth=")) {
                                        app.auth_token = Some(token.to_string());
                                    }
                                }
                                if let Ok(data) = r.json::<serde_json::Value>().await {
                                    app.user_name = data["display_name"].as_str().map(|s| s.to_string());
                                    app.user_role = data["role"].as_str().map(|s| s.to_string());
                                }
                                app.save_config();
                                app.messages.push(ChatMessage{role:"system".into(), text:format!("Connecte en tant que {}", app.user_name.as_deref().unwrap_or("?"))});
                            }
                            _ => app.messages.push(ChatMessage{role:"error".into(), text:"Identifiants incorrects".into()}),
                        }
                        return;
                    }
                    "/enroll" => {
                        let arg = text.strip_prefix("/enroll").unwrap_or("").trim();
                        let parts: Vec<&str> = arg.splitn(2, ' ').collect();
                        let name = parts.first().filter(|s| !s.is_empty()).unwrap_or(&"CLIUser");
                        let pw = parts.get(1).unwrap_or(&"");
                        let mut body = serde_json::json!({"display_name": name});
                        if !pw.is_empty() { body["password"] = serde_json::json!(pw); }
                        let resp = reqwest::Client::new()
                            .post(format!("{}/api/auth/enroll", app.server_url))
                            .json(&body).send().await;
                        match resp {
                            Ok(r) if r.status().is_success() => {
                                if let Some(cookie) = r.headers().get("set-cookie").and_then(|v| v.to_str().ok()) {
                                    if let Some(token) = cookie.split(';').next().and_then(|s| s.strip_prefix("laruche_auth=")) {
                                        app.auth_token = Some(token.to_string());
                                    }
                                }
                                if let Ok(data) = r.json::<serde_json::Value>().await {
                                    app.user_name = data["display_name"].as_str().map(|s| s.to_string());
                                    app.user_role = data["role"].as_str().map(|s| s.to_string());
                                }
                                app.save_config();
                                app.messages.push(ChatMessage{role:"system".into(), text:format!("Compte cree: {} ({})", app.user_name.as_deref().unwrap_or("?"), app.user_role.as_deref().unwrap_or("user"))});
                            }
                            _ => app.messages.push(ChatMessage{role:"error".into(), text:"Erreur enrollment".into()}),
                        }
                        return;
                    }
                    "/logout" => {
                        app.auth_token = None;
                        app.user_name = None;
                        app.user_role = None;
                        app.save_config();
                        app.messages.push(ChatMessage{role:"system".into(), text:"Deconnecte".into()});
                        return;
                    }
                    "/whoami" => {
                        let info = match (&app.user_name, &app.user_role) {
                            (Some(n), Some(r)) => format!("{} ({})", n, r),
                            _ => "Non authentifie. /login <nom> <mdp> ou /enroll <nom> [mdp]".into(),
                        };
                        app.messages.push(ChatMessage{role:"system".into(), text:info});
                        return;
                    }
                    _ => { app.status_msg = format!("? {} — /help", text); return; }
                }
            }

            if !app.connected {
                app.messages.push(ChatMessage{role:"error".into(),text:"Aucun serveur LaRuche connecte !".into()});
                return;
            }

            // Show user message immediately + scroll
            app.messages.push(ChatMessage { role:"user".into(), text: text.clone() });
            app.is_streaming = true;
            app.status_msg = "Reflexion...".into();
            let timestamp = chrono::Local::now().format("%H:%M:%S").to_string();
            app.activity_log.push(format!("[{}] Prompt: {}", timestamp, text.chars().take(50).collect::<String>()));
            // Auto-scroll to show the new message
            let total = app.messages.iter().map(|m| m.text.lines().count()+1).sum::<usize>();
            app.chat_scroll = total.saturating_sub(12) as u16;

            // Spawn the WebSocket streaming task (falls back to HTTP internally)
            let (tx_evt, rx_evt) = tokio::sync::mpsc::channel::<TuiEvent>(64);
            app.event_rx = Some(rx_evt);
            app.streaming_response.clear();
            let url = app.server_url.clone();
            let model = app.model.clone();
            let token = app.auth_token.clone();
            let session = app.session_id.clone();
            tokio::spawn(async move {
                stream_via_websocket(url, text, model, token, session, tx_evt).await;
            });
        }
        KeyCode::Char(c) => {
            // Insert at char position (not byte position)
            let byte_pos = app.input.char_indices().nth(app.cursor_pos).map(|(i,_)|i).unwrap_or(app.input.len());
            app.input.insert(byte_pos, c);
            app.cursor_pos += 1;
            app.history_idx = None;
            update_autocomplete(app);
        }
        KeyCode::Backspace => {
            if app.cursor_pos > 0 {
                app.cursor_pos -= 1;
                let byte_pos = app.input.char_indices().nth(app.cursor_pos).map(|(i,_)|i).unwrap_or(0);
                let next_byte = app.input.char_indices().nth(app.cursor_pos + 1).map(|(i,_)|i).unwrap_or(app.input.len());
                app.input.replace_range(byte_pos..next_byte, "");
            }
            app.autocomplete_suggestion.clear();
            update_autocomplete(app);
        }
        KeyCode::Left => app.cursor_pos = app.cursor_pos.saturating_sub(1),
        KeyCode::Right => {
            if app.cursor_pos < app.input.chars().count() {
                app.cursor_pos += 1;
            } else if !app.autocomplete_suggestion.is_empty() {
                // Accept autocomplete suggestion
                app.input.push_str(&app.autocomplete_suggestion);
                app.cursor_pos = app.input.chars().count();
                app.autocomplete_suggestion.clear();
                update_autocomplete(app);
            }
        }
        KeyCode::Up => {
            // History: navigate up
            if app.history.is_empty() { return; }
            match app.history_idx {
                None => {
                    app.history_draft = app.input.clone();
                    app.history_idx = Some(app.history.len() - 1);
                    app.input = app.history[app.history.len() - 1].clone();
                }
                Some(idx) if idx > 0 => {
                    app.history_idx = Some(idx - 1);
                    app.input = app.history[idx - 1].clone();
                }
                _ => {}
            }
            app.cursor_pos = app.input.chars().count();
            app.autocomplete_suggestion.clear();
        }
        KeyCode::Down => {
            // History: navigate down
            match app.history_idx {
                Some(idx) => {
                    if idx + 1 < app.history.len() {
                        app.history_idx = Some(idx + 1);
                        app.input = app.history[idx + 1].clone();
                    } else {
                        app.history_idx = None;
                        app.input = app.history_draft.clone();
                    }
                }
                None => {}
            }
            app.cursor_pos = app.input.chars().count();
            app.autocomplete_suggestion.clear();
        }
        KeyCode::Home => app.cursor_pos = 0,
        KeyCode::End => app.cursor_pos = app.input.chars().count(),
        KeyCode::Tab => app.active_panel = Panel::Chat,
        KeyCode::Esc => app.active_panel = Panel::Chat,
        _ => {}
    }
}

/// Fetch models list from server.
async fn fetch_tools_or_models(url: &str, what: &str) -> Vec<String> {
    if url.is_empty() { return vec![]; }
    let endpoint = if what == "models" { "/models" } else { "/api/tools" };
    reqwest::Client::new().get(format!("{}{}", url, endpoint)).send().await.ok()
        .and_then(|r| tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(r.json::<serde_json::Value>())).ok())
        .and_then(|d| {
            if what == "models" {
                d["models"].as_array().map(|a| a.iter().filter_map(|m| m["name"].as_str().map(String::from)).collect())
            } else {
                d.as_array().map(|a| a.iter().filter_map(|t| t["name"].as_str().map(String::from)).collect())
            }
        })
        .unwrap_or_default()
}

/// Autocomplete suggestions for slash commands.
fn update_autocomplete(app: &mut App) {
    app.autocomplete_suggestion.clear();
    let input = &app.input;
    if input.is_empty() { return; }

    // Slash command completions
    let commands = [
        "/help", "/clear", "/quit", "/model", "/tools", "/cwd",
        "/discover", "/doctor", "/server start", "/server stop",
        "/server restart", "/server status", "/server install",
        "/server uninstall", "/server update", "/export",
    ];

    if input.starts_with('/') {
        let input_chars = input.chars().count();
        for cmd in &commands {
            if cmd.starts_with(input) && cmd.chars().count() > input_chars {
                app.autocomplete_suggestion = cmd.chars().skip(input_chars).collect();
                return;
            }
        }
    }

    // Recent history completion (only for ASCII-safe prefix matching)
    for h in app.history.iter().rev() {
        if h.starts_with(input) && h.len() > input.len() {
            let input_chars = input.chars().count();
            app.autocomplete_suggestion = h.chars().skip(input_chars).collect();
            return;
        }
    }
}

fn ui(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default().direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(10), Constraint::Length(3), Constraint::Length(1)])
        .split(f.area());

    draw_header(f, chunks[0], app);
    let body = Layout::default().direction(Direction::Horizontal)
        .constraints([Constraint::Length(22), Constraint::Min(40)])
        .split(chunks[1]);
    draw_sidebar(f, body[0], app);
    draw_chat(f, body[1], app);
    draw_input(f, chunks[2], app);
    draw_status(f, chunks[3], app);
}

fn draw_header(f: &mut Frame, area: Rect, app: &App) {
    let conn = if app.connected {
        Span::styled(" Connected ", Style::default().fg(Color::Green))
    } else {
        Span::styled(" Offline ", Style::default().fg(Color::Red))
    };
    let h = Line::from(vec![
        Span::styled(" 🐝 ", Style::default()),
        Span::styled("LaRuche ", Style::default().fg(AMBER).add_modifier(Modifier::BOLD)),
        Span::styled("L'Essaim", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        conn,
        Span::styled("  │  ", Style::default().fg(BORDER)),
        Span::styled(if app.server_url.is_empty() { "-".to_string() } else { format!("{}/app#chat", app.server_url) }, Style::default().fg(TEXT_DIM)),
        Span::styled("  │  ", Style::default().fg(BORDER)),
        Span::styled(format!("model: {}", app.model), Style::default().fg(Color::Cyan)),
    ]);
    f.render_widget(Paragraph::new(h).block(Block::default().borders(Borders::BOTTOM).border_style(Style::default().fg(BORDER)).style(Style::default().bg(BG_PANEL))), area);
}

fn draw_sidebar(f: &mut Frame, area: Rect, app: &mut App) {
    let chunks = Layout::default().direction(Direction::Vertical)
        .constraints([
            Constraint::Length(if app.plan.is_empty() { 0 } else { (app.plan.len() as u16 + 2).min(8) }),
            Constraint::Min(5),
        ]).split(area);

    if !app.plan.is_empty() {
        let items: Vec<ListItem> = app.plan.iter().map(|(task,status)| {
            let icon = match status.as_str() { "done"=>Span::styled("✓ ",Style::default().fg(Color::Green)), "in_progress"=>Span::styled("● ",Style::default().fg(AMBER)), _=>Span::styled("○ ",Style::default().fg(TEXT_DIM)) };
            ListItem::new(Line::from(vec![icon, Span::raw(task.chars().take(18).collect::<String>())]))
        }).collect();
        f.render_widget(List::new(items).block(Block::default().title(Span::styled(" Plan ",Style::default().fg(AMBER).add_modifier(Modifier::BOLD))).borders(Borders::ALL).border_style(Style::default().fg(BORDER)).style(Style::default().bg(BG_PANEL))), chunks[0]);
    }

    let is_active = app.active_panel == Panel::Sidebar;

    // Tab indicators at top
    let tab_titles = vec![
        if app.sidebar_panel == SidebarPanel::Tools { Span::styled(" Abeilles ", Style::default().fg(AMBER).add_modifier(Modifier::BOLD)) }
        else { Span::styled(" Abeilles ", Style::default().fg(TEXT_DIM)) },
        Span::styled(" | ", Style::default().fg(BORDER)),
        if app.sidebar_panel == SidebarPanel::Models { Span::styled("Models", Style::default().fg(AMBER).add_modifier(Modifier::BOLD)) }
        else { Span::styled("Models", Style::default().fg(TEXT_DIM)) },
        Span::styled(" | ", Style::default().fg(BORDER)),
        if app.sidebar_panel == SidebarPanel::Sessions { Span::styled("Sess", Style::default().fg(AMBER).add_modifier(Modifier::BOLD)) }
        else { Span::styled("Sess", Style::default().fg(TEXT_DIM)) },
        Span::styled("|", Style::default().fg(BORDER)),
        if app.sidebar_panel == SidebarPanel::Plan { Span::styled("Plan", Style::default().fg(AMBER).add_modifier(Modifier::BOLD)) }
        else { Span::styled("Plan", Style::default().fg(TEXT_DIM)) },
    ];
    let title = Line::from(tab_titles);

    let items: Vec<ListItem> = match app.sidebar_panel {
        SidebarPanel::Tools => {
            let mut items: Vec<ListItem> = Vec::new();
            if app.is_streaming {
                items.push(ListItem::new(Span::styled(" ● Agent actif...", Style::default().fg(AMBER).add_modifier(Modifier::BOLD))));
                items.push(ListItem::new(Span::raw("")));
            }
            for n in &app.tools {
                items.push(ListItem::new(Span::styled(
                    format!(" · {}", n.chars().take(16).collect::<String>()),
                    Style::default().fg(Color::Cyan),
                )));
            }
            items
        }
        SidebarPanel::Models => {
            if app.available_models.is_empty() {
                vec![
                    ListItem::new(Span::styled(format!(" ● {}", app.model), Style::default().fg(Color::Green))),
                    ListItem::new(Span::styled(" ← → pour charger", Style::default().fg(TEXT_DIM))),
                ]
            } else {
                app.available_models.iter().enumerate().map(|(i, name)| {
                    let is_current = *name == app.model;
                    let is_cursor = i == app.model_cursor;
                    let prefix = if is_cursor && is_current { "▸●" }
                        else if is_cursor { "▸ " }
                        else if is_current { " ●" }
                        else { "  " };
                    let style = if is_cursor {
                        Style::default().fg(AMBER).add_modifier(Modifier::BOLD)
                    } else if is_current {
                        Style::default().fg(Color::Green)
                    } else {
                        Style::default().fg(Color::Cyan)
                    };
                    ListItem::new(Span::styled(format!("{} {}", prefix, name.chars().take(14).collect::<String>()), style))
                }).collect()
            }
        }
        SidebarPanel::Sessions => {
            app.messages.iter().rev()
                .filter(|m| m.role == "user")
                .take(8)
                .map(|m| ListItem::new(Span::styled(
                    format!(" · {}", m.text.chars().take(18).collect::<String>()),
                    Style::default().fg(TEXT_DIM),
                ))).collect()
        }
        SidebarPanel::Plan => {
            if app.plan.is_empty() {
                vec![ListItem::new(Span::styled(" Aucun plan", Style::default().fg(TEXT_DIM).add_modifier(Modifier::ITALIC)))]
            } else {
                app.plan.iter().map(|(task, status)| {
                    let (icon, color) = match status.as_str() {
                        "done" => ("✓", Color::Green),
                        "in_progress" => ("●", AMBER),
                        _ => ("○", Color::Rgb(113,113,122)),
                    };
                    ListItem::new(Line::from(vec![
                        Span::styled(format!(" {} ", icon), Style::default().fg(color)),
                        Span::styled(task.chars().take(16).collect::<String>(), Style::default().fg(if status == "done" { TEXT_DIM } else { Color::White })),
                    ]))
                }).collect()
            }
        }
    };

    f.render_widget(List::new(items)
        .block(Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(if is_active { AMBER } else { BORDER }))
            .style(Style::default().bg(BG_PANEL))),
        chunks[1]);
}

/// Basic markdown rendering for terminal: **bold**, *italic*, `code`, ### headers, - lists
fn render_md_line(line: &str) -> Line<'static> {
    let trimmed = line.trim();

    // Headers
    if trimmed.starts_with("### ") {
        return Line::from(Span::styled(format!("  {}", &trimmed[4..]), Style::default().fg(AMBER).add_modifier(Modifier::BOLD)));
    }
    if trimmed.starts_with("## ") {
        return Line::from(Span::styled(format!("  {}", &trimmed[3..]), Style::default().fg(AMBER).add_modifier(Modifier::BOLD)));
    }
    if trimmed.starts_with("# ") {
        return Line::from(Span::styled(format!("  {}", &trimmed[2..]), Style::default().fg(AMBER).add_modifier(Modifier::BOLD)));
    }
    // Bullet points
    if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
        let indent = line.len() - line.trim_start().len();
        let prefix = " ".repeat(indent + 2);
        return Line::from(vec![
            Span::styled(format!("{}· ", prefix), Style::default().fg(AMBER)),
            Span::styled(trimmed[2..].to_string(), Style::default().fg(Color::White)),
        ]);
    }
    // Numbered lists
    if trimmed.len() > 2 && trimmed.chars().next().unwrap_or(' ').is_ascii_digit() {
        if let Some(dot_pos) = trimmed.find(". ") {
            if dot_pos <= 3 {
                return Line::from(vec![
                    Span::styled(format!("  {}. ", &trimmed[..dot_pos]), Style::default().fg(AMBER)),
                    Span::styled(trimmed[dot_pos+2..].to_string(), Style::default().fg(Color::White)),
                ]);
            }
        }
    }

    // Inline formatting: **bold**, *italic*, `code`
    let mut spans: Vec<Span<'static>> = vec![Span::raw("  ".to_string())];
    let mut chars = trimmed.chars().peekable();
    let mut current = String::new();

    while let Some(c) = chars.next() {
        if c == '`' {
            if !current.is_empty() { spans.push(Span::styled(current.clone(), Style::default().fg(Color::White))); current.clear(); }
            let mut code = String::new();
            while let Some(&nc) = chars.peek() { if nc == '`' { chars.next(); break; } code.push(chars.next().unwrap()); }
            spans.push(Span::styled(code, Style::default().fg(Color::Cyan)));
        } else if c == '*' && chars.peek() == Some(&'*') {
            chars.next(); // consume second *
            if !current.is_empty() { spans.push(Span::styled(current.clone(), Style::default().fg(Color::White))); current.clear(); }
            let mut bold = String::new();
            while let Some(nc) = chars.next() {
                if nc == '*' && chars.peek() == Some(&'*') { chars.next(); break; }
                bold.push(nc);
            }
            spans.push(Span::styled(bold, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)));
        } else if c == '*' {
            if !current.is_empty() { spans.push(Span::styled(current.clone(), Style::default().fg(Color::White))); current.clear(); }
            let mut italic = String::new();
            while let Some(nc) = chars.next() { if nc == '*' { break; } italic.push(nc); }
            spans.push(Span::styled(italic, Style::default().fg(Color::White).add_modifier(Modifier::ITALIC)));
        } else {
            current.push(c);
        }
    }
    if !current.is_empty() { spans.push(Span::styled(current, Style::default().fg(Color::White))); }

    Line::from(spans)
}

fn draw_chat(f: &mut Frame, area: Rect, app: &App) {
    let is_active = app.active_panel == Panel::Chat;

    // Tab title
    let title = Line::from(vec![
        if app.chat_view == ChatView::Messages { Span::styled(" Chat ", Style::default().fg(AMBER).add_modifier(Modifier::BOLD)) }
        else { Span::styled(" Chat ", Style::default().fg(TEXT_DIM)) },
        Span::styled("│", Style::default().fg(BORDER)),
        if app.chat_view == ChatView::Activity { Span::styled(" Activity ", Style::default().fg(AMBER).add_modifier(Modifier::BOLD)) }
        else { Span::styled(" Activity ", Style::default().fg(TEXT_DIM)) },
        Span::styled("│", Style::default().fg(BORDER)),
        if app.chat_view == ChatView::Status { Span::styled(" Status ", Style::default().fg(AMBER).add_modifier(Modifier::BOLD)) }
        else { Span::styled(" Status ", Style::default().fg(TEXT_DIM)) },
        if is_active { Span::styled("  ←→", Style::default().fg(Color::Rgb(50,50,55))) } else { Span::raw("") },
    ]);

    let mut lines: Vec<Line> = Vec::new();

    match app.chat_view {
        ChatView::Messages => {
            for msg in &app.messages {
                match msg.role.as_str() {
                    "user" => {
                        lines.push(Line::from(vec![
                            Span::styled("  ❯ ", Style::default().fg(AMBER).add_modifier(Modifier::BOLD)),
                            Span::styled(&msg.text, Style::default().fg(AMBER)),
                        ]));
                        lines.push(Line::from(""));
                    }
                    "assistant" => {
                        for l in msg.text.lines() {
                            // Basic markdown rendering
                            let styled = render_md_line(l);
                            lines.push(styled);
                        }
                        lines.push(Line::from(""));
                    }
                    "tool" => {
                        lines.push(Line::from(vec![
                            Span::styled("  ⚙ ", Style::default().fg(Color::Blue)),
                            Span::styled(&msg.text, Style::default().fg(Color::Cyan)),
                        ]));
                    }
                    "error" => {
                        lines.push(Line::from(Span::styled(format!("  ✗ {}", msg.text), Style::default().fg(Color::Red))));
                        lines.push(Line::from(""));
                    }
                    "system" => {
                        lines.push(Line::from(Span::styled(format!("  {}", msg.text), Style::default().fg(TEXT_DIM).add_modifier(Modifier::ITALIC))));
                        lines.push(Line::from(""));
                    }
                    _ => {}
                }
            }
            if app.is_streaming {
                // Show the partial streaming response as it comes in
                if !app.streaming_response.is_empty() {
                    for l in app.streaming_response.lines() {
                        lines.push(render_md_line(l));
                    }
                }
                lines.push(Line::from(Span::styled("  ▍", Style::default().fg(AMBER))));
            }
        }
        ChatView::Activity => {
            if app.activity_log.is_empty() {
                lines.push(Line::from(Span::styled("  Aucune activite", Style::default().fg(TEXT_DIM).add_modifier(Modifier::ITALIC))));
            } else {
                for entry in &app.activity_log {
                    let (icon, color) = if entry.starts_with("[OK]") { ("✓", Color::Green) }
                        else if entry.starts_with("[ERR]") { ("✗", Color::Red) }
                        else if entry.starts_with("[TOOL]") { ("⚙", Color::Blue) }
                        else if entry.starts_with("[THINK]") { ("💭", Color::Magenta) }
                        else { ("·", Color::Rgb(113,113,122)) };
                    lines.push(Line::from(vec![
                        Span::styled(format!("  {} ", icon), Style::default().fg(color)),
                        Span::styled(entry.as_str(), Style::default().fg(TEXT_DIM)),
                    ]));
                }
            }
        }
        ChatView::Status => {
            // Connection
            lines.push(Line::from(vec![
                Span::styled("  Serveur  ", Style::default().fg(TEXT_DIM)),
                if app.connected { Span::styled("● Connecte", Style::default().fg(Color::Green)) }
                else { Span::styled("● Deconnecte", Style::default().fg(Color::Red)) },
            ]));
            lines.push(Line::from(vec![
                Span::styled("  URL      ", Style::default().fg(TEXT_DIM)),
                Span::styled(if app.server_url.is_empty() { "-" } else { &app.server_url }, Style::default().fg(Color::Cyan)),
            ]));
            lines.push(Line::from(""));

            // Model
            lines.push(Line::from(vec![
                Span::styled("  Modele   ", Style::default().fg(TEXT_DIM)),
                Span::styled(&app.model, Style::default().fg(AMBER)),
            ]));
            lines.push(Line::from(""));

            // Working directory
            lines.push(Line::from(vec![
                Span::styled("  CWD      ", Style::default().fg(TEXT_DIM)),
                Span::styled(&app.cwd, Style::default().fg(AMBER)),
            ]));
            lines.push(Line::from(""));

            // Tools
            lines.push(Line::from(vec![
                Span::styled("  Abeilles ", Style::default().fg(TEXT_DIM)),
                Span::styled(format!("{} outils", app.tools.len()), Style::default().fg(Color::Green)),
            ]));
            lines.push(Line::from(""));

            // Session
            lines.push(Line::from(vec![
                Span::styled("  Session  ", Style::default().fg(TEXT_DIM)),
                Span::styled(
                    app.session_id.as_deref().map(|s| &s[..8.min(s.len())]).unwrap_or("nouvelle"),
                    Style::default().fg(TEXT_DIM),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  Messages ", Style::default().fg(TEXT_DIM)),
                Span::styled(format!("{}", app.messages.len()), Style::default().fg(Color::White)),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  Historiq. ", Style::default().fg(TEXT_DIM)),
                Span::styled(format!("{} commandes", app.history.len()), Style::default().fg(Color::White)),
            ]));
            lines.push(Line::from(""));

            // Channels (if connected, fetch from server)
            lines.push(Line::from(Span::styled("  ─── Services ───", Style::default().fg(BORDER))));
            lines.push(Line::from(vec![
                Span::styled("  Telegram ", Style::default().fg(TEXT_DIM)),
                Span::styled("voir /server", Style::default().fg(TEXT_DIM)),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  STT      ", Style::default().fg(TEXT_DIM)),
                Span::styled("voir Settings web", Style::default().fg(TEXT_DIM)),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  TTS      ", Style::default().fg(TEXT_DIM)),
                Span::styled("voir Settings web", Style::default().fg(TEXT_DIM)),
            ]));
        }
    }

    f.render_widget(
        Paragraph::new(Text::from(lines))
            .wrap(Wrap { trim: false })
            .scroll((app.chat_scroll, 0))
            .block(Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(if is_active { AMBER } else { BORDER }))
                .style(Style::default().bg(BG))),
        area,
    );
}

fn draw_input(f: &mut Frame, area: Rect, app: &App) {
    let is_active = app.active_panel == Panel::Input;

    let content = if app.input.is_empty() && !is_active {
        Line::from(Span::styled("Tab pour taper...", Style::default().fg(TEXT_DIM)))
    } else {
        // Show input + autocomplete suggestion in dim
        let mut spans = vec![Span::styled(&app.input, Style::default().fg(Color::White))];
        if !app.autocomplete_suggestion.is_empty() && is_active {
            spans.push(Span::styled(&app.autocomplete_suggestion, Style::default().fg(Color::Rgb(60, 60, 65))));
            spans.push(Span::styled(" →", Style::default().fg(Color::Rgb(60, 60, 65))));
        }
        Line::from(spans)
    };

    let prompt = if is_active { "> " } else { "  " };
    f.render_widget(Paragraph::new(content)
        .block(Block::default()
            .title(Span::styled(prompt, Style::default().fg(AMBER).add_modifier(Modifier::BOLD)))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(if is_active { AMBER } else { BORDER }))
            .style(Style::default().bg(BG_PANEL))),
        area);

    // Cursor: x + 1 (left border) + 2 (prompt "> ") + cursor_pos
    if is_active {
        f.set_cursor_position((area.x + 1 + app.cursor_pos as u16, area.y + 1));
    }
}

fn draw_status(f: &mut Frame, area: Rect, app: &App) {
    let s = Line::from(vec![
        Span::styled(" cwd: ",Style::default().fg(TEXT_DIM)),
        Span::styled(app.cwd.chars().rev().take(25).collect::<String>().chars().rev().collect::<String>(),Style::default().fg(AMBER)),
        Span::styled("  │  ",Style::default().fg(BORDER)),
        Span::styled(&app.status_msg,Style::default().fg(TEXT_DIM)),
        Span::styled("  │  ",Style::default().fg(BORDER)),
        Span::styled("Tab:panel ↑↓:scroll Ctrl-C:quit",Style::default().fg(TEXT_DIM)),
    ]);
    f.render_widget(Paragraph::new(s).style(Style::default().bg(BG_PANEL)), area);
}
