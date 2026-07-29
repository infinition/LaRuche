//! LaRuche TUI: Rich terminal interface connected to a LaRuche server.
//!
//! Connects to a LaRuche server via WebSocket (/ws/chat) for agent capabilities.
//! Falls back to direct Ollama if no server found.

pub mod markdown;

use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
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
pub enum TuiEvent {
    /// A single token to append to the current streaming response.
    Token(String),
    /// The agent is calling a tool.
    ToolCall { name: String, args: String },
    /// A tool has returned a result.
    ToolResult {
        name: String,
        success: bool,
        ms: u64,
    },
    /// Plan update from the agent.
    Plan(Vec<(String, String)>),
    /// Agent thinking / reasoning trace.
    Thinking(String),
    /// Stream finished: the full response is included.
    Done(String),
    /// An error occurred.
    Error(String),

    // Background UI loading
    MemoryTreeLoaded(Vec<serde_json::Value>),
    NodeDetailsLoaded(serde_json::Value),
    MissionsLoaded(Vec<serde_json::Value>),
    MissionDossierLoaded { slug: String, markdown: String },
    ActionFinished(String),
}

const AMBER: Color = Color::Rgb(245, 158, 11);
const BG: Color = Color::Rgb(9, 9, 11);
const BG_PANEL: Color = Color::Rgb(17, 17, 19);
const BORDER: Color = Color::Rgb(42, 42, 46);
const TEXT_DIM: Color = Color::Rgb(113, 113, 122);

#[derive(Clone)]
pub struct ChatMessage {
    pub role: String,
    pub text: String,
}

pub struct App {
    input: String,
    cursor_pos: usize,
    messages: Vec<ChatMessage>,
    chat_scroll: u16,
    tools: Vec<String>,
    plan: Vec<(String, String)>,
    pub current_screen: Screen,
    pub active_panel: Panel,
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
    // WebSocket streaming channel
    pub event_rx: Option<tokio::sync::mpsc::Receiver<TuiEvent>>,
    pub stream_task: Option<tokio::task::JoinHandle<()>>,
    // Buffer for tokens as they stream in
    streaming_response: String,
    // Auth
    auth_token: Option<String>,
    user_name: Option<String>,
    user_role: Option<String>,

    // Memory view state
    memory_nodes: Vec<serde_json::Value>,
    selected_node_idx: usize,
    selected_node_details: Option<serde_json::Value>,
    memory_active_pane: MemoryPane,
    memory_input_mode: MemoryInputMode,
    memory_tree_scroll: usize,
    memory_details_scroll: usize,
    selected_item_idx: usize,

    // Missions view state
    missions: Vec<serde_json::Value>,
    selected_mission_idx: usize,
    selected_mission_dossier: Option<String>,
    missions_active_pane: MissionsPane,
    missions_input_mode: MissionsInputMode,
    missions_dossier_scroll: usize,

    // Shared background event sender
    pub ui_tx: Option<tokio::sync::mpsc::Sender<TuiEvent>>,
    // Agent sidebar scroll
    sidebar_scroll: u16,
}

#[derive(PartialEq, Clone, Debug)]
pub enum MemoryPane {
    Tree,
    Details,
}

#[derive(PartialEq, Clone, Debug)]
pub enum MemoryInputMode {
    Normal,
    CreateNode,
    AddItem,
    EditNode,
    EditItem,
}

#[derive(PartialEq, Clone, Debug)]
pub enum MissionsPane {
    List,
    Dossier,
}

#[derive(PartialEq, Clone, Debug)]
pub enum MissionsInputMode {
    Normal,
    CreateMission,
}


#[derive(PartialEq, Clone)]
pub enum ChatView {
    Messages,
    Activity,
    Status,
}

#[derive(PartialEq, Clone)]
pub enum Screen {
    Chat,
    Memory,
    Missions,
}

#[derive(PartialEq, Clone)]
pub enum Panel {
    Input,
    Chat,
    MemoryTree,
    MissionsList,
}

impl App {
    async fn new() -> Self {
        let server_url = discover_server().await;
        let connected = !server_url.is_empty();
        let mut model = fetch_model(&server_url).await;
        let tools = fetch_tools(&server_url).await;
        let cwd = std::env::current_dir()
            .unwrap_or_default()
            .display()
            .to_string();

        let welcome = if connected {
            format!("Connected to {} - ready!", server_url)
        } else {
            "No LaRuche server found. Run: cargo run -p laruche-node".to_string()
        };

        let mut auth_token = None;
        let mut user_name = None;
        let mut user_role = None;

        // Load persisted config (including auth token)
        let cfg_path = dirs_config_path();
        let mut saved_model = String::new();
        if let Ok(content) = std::fs::read_to_string(&cfg_path) {
            if let Ok(cfg) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(m) = cfg["model"].as_str() {
                    saved_model = m.to_string();
                }
                if let Some(t) = cfg["auth_token"].as_str() {
                    auth_token = Some(t.to_string());
                }
                if let Some(n) = cfg["user_name"].as_str() {
                    user_name = Some(n.to_string());
                }
                if let Some(r) = cfg["user_role"].as_str() {
                    user_role = Some(r.to_string());
                }
            }
        }

        // Verify auth token with server if connected
        if connected && auth_token.is_some() {
            let check = reqwest::Client::new()
                .get(format!("{}/api/auth/me", server_url))
                .header(
                    "Cookie",
                    format!("laruche_auth={}", auth_token.as_deref().unwrap_or("")),
                )
                .send()
                .await;
            match check {
                Ok(r) if r.status().is_success() => {
                    if let Ok(data) = r.json::<serde_json::Value>().await {
                        user_name = data["display_name"].as_str().map(|s| s.to_string());
                        user_role = data["role"].as_str().map(|s| s.to_string());
                    }
                }
                _ => {
                    auth_token = None;
                    user_name = None;
                    user_role = None;
                }
            }
        }

        if !saved_model.is_empty() {
            model = saved_model;
        }

        let auth_info = match (&user_name, &user_role) {
            (Some(n), Some(r)) => format!(" | {} ({})", n, r),
            _ => String::new(),
        };

        let welcome_msg = if connected {
            format!("Connected to {}{}", server_url, auth_info)
        } else {
            welcome
        };

        
        App {
            input: String::new(),
            cursor_pos: 0,
            messages: vec![ChatMessage {
                role: "system".into(),
                text: welcome_msg,
            }],
            chat_scroll: 0,
            tools,
            plan: vec![],
            current_screen: Screen::Chat,
            active_panel: Panel::Input,
            model,
            server_url,
            cwd,
            tokens: 0,
            status_msg: if connected {
                "Connected".into()
            } else {
                "Offline".into()
            },
            is_streaming: false,
            should_quit: false,
            session_id: None,
            connected,
            history: Vec::new(),
            history_idx: None,
            history_draft: String::new(),
            autocomplete_suggestion: String::new(),
            chat_view: ChatView::Messages,
            activity_log: Vec::new(),
            event_rx: None,
            stream_task: None,
            streaming_response: String::new(),
            auth_token,
            user_name,
            user_role,

            memory_nodes: Vec::new(),
            selected_node_idx: 0,
            selected_node_details: None,
            memory_active_pane: MemoryPane::Tree,
            memory_input_mode: MemoryInputMode::Normal,
            memory_tree_scroll: 0,
            memory_details_scroll: 0,
            selected_item_idx: 0,

            missions: Vec::new(),
            selected_mission_idx: 0,
            selected_mission_dossier: None,
            missions_active_pane: MissionsPane::List,
            missions_input_mode: MissionsInputMode::Normal,
            missions_dossier_scroll: 0,

            ui_tx: None,
            sidebar_scroll: 0,
        }
    }

    fn save_config(&self) {
        let path = dirs_config_path();
        if let Some(parent) = std::path::Path::new(&path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let cfg = serde_json::json!({
            "model": self.model,
            "server_url": self.server_url,
            "auth_token": self.auth_token,
            "user_name": self.user_name,
            "user_role": self.user_role,
        });
        let _ = std::fs::write(
            &path,
            serde_json::to_string_pretty(&cfg).unwrap_or_default(),
        );
    }

    fn trigger_load_memory(&self) {
        if let Some(ref tx) = self.ui_tx {
            let url = self.server_url.clone();
            let token = self.auth_token.clone();
            let tx_cloned = tx.clone();
            tokio::spawn(async move {
                fetch_memory_tree_bg(url, token, tx_cloned).await;
            });
        }
    }

    fn trigger_load_node_details(&self, node_id: String) {
        if let Some(ref tx) = self.ui_tx {
            let url = self.server_url.clone();
            let token = self.auth_token.clone();
            let tx_cloned = tx.clone();
            tokio::spawn(async move {
                fetch_node_details_bg(url, node_id, token, tx_cloned).await;
            });
        }
    }

    fn trigger_load_missions(&self) {
        if let Some(ref tx) = self.ui_tx {
            let url = self.server_url.clone();
            let token = self.auth_token.clone();
            let tx_cloned = tx.clone();
            tokio::spawn(async move {
                fetch_missions_bg(url, token, tx_cloned).await;
            });
        }
    }

    fn trigger_load_mission_dossier(&self, slug: String) {
        if let Some(ref tx) = self.ui_tx {
            let url = self.server_url.clone();
            let token = self.auth_token.clone();
            let tx_cloned = tx.clone();
            tokio::spawn(async move {
                fetch_mission_dossier_bg(url, slug, token, tx_cloned).await;
            });
        }
    }
}

// ======================== Background tasks for Memory and Missions ========================

struct TreeDrawItem {
    id: String,
    label: String,
    depth: usize,
    is_protected: bool,
}

fn build_draw_tree(
    nodes: &[serde_json::Value],
    parent: Option<&str>,
    depth: usize,
    visited: &mut std::collections::HashSet<String>,
    out: &mut Vec<TreeDrawItem>,
) {
    for n in nodes {
        let id = match n.get("id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => continue,
        };
        if visited.contains(id) {
            continue;
        }
        let n_parent = n.get("parent_id").and_then(|v| v.as_str()).unwrap_or("");
        let matches = match parent {
            Some(p) => n_parent == p,
            None => n_parent.is_empty() || !nodes.iter().any(|other| other.get("id").and_then(|v| v.as_str()) == Some(n_parent)),
        };
        if matches {
            visited.insert(id.to_string());
            let label = n.get("label").and_then(|v| v.as_str()).unwrap_or(id).to_string();
            let is_protected = n.get("protected").and_then(|v| v.as_bool()).unwrap_or(false);

            out.push(TreeDrawItem {
                id: id.to_string(),
                label,
                depth,
                is_protected,
            });
            build_draw_tree(nodes, Some(id), depth + 1, visited, out);
        }
    }
}

async fn fetch_memory_tree_bg(url: String, token: Option<String>, tx: tokio::sync::mpsc::Sender<TuiEvent>) {
    let client = reqwest::Client::new();
    let mut req = client.get(format!("{}/api/memory/tree", url));
    if let Some(t) = token {
        req = req.header("Cookie", format!("laruche_auth={}", t));
    }
    match req.send().await {
        Ok(resp) => {
            if let Ok(data) = resp.json::<serde_json::Value>().await {
                if let Some(nodes) = data["nodes"].as_array() {
                    let _ = tx.send(TuiEvent::MemoryTreeLoaded(nodes.clone())).await;
                    return;
                }
            }
            let _ = tx.send(TuiEvent::Error("Failed to parse memory tree".into())).await;
        }
        Err(e) => {
            let _ = tx.send(TuiEvent::Error(format!("Network error: {}", e))).await;
        }
    }
}

async fn fetch_node_details_bg(url: String, node_id: String, token: Option<String>, tx: tokio::sync::mpsc::Sender<TuiEvent>) {
    let client = reqwest::Client::new();
    let mut req = client.get(format!("{}/api/memory/node/{}", url, node_id));
    if let Some(t) = token {
        req = req.header("Cookie", format!("laruche_auth={}", t));
    }
    match req.send().await {
        Ok(resp) => {
            if let Ok(data) = resp.json::<serde_json::Value>().await {
                if data["status"] == "ok" {
                    let _ = tx.send(TuiEvent::NodeDetailsLoaded(data["node"].clone())).await;
                    return;
                }
            }
            let _ = tx.send(TuiEvent::Error(format!("Failed to load node details for {}", node_id))).await;
        }
        Err(e) => {
            let _ = tx.send(TuiEvent::Error(format!("Network error: {}", e))).await;
        }
    }
}

async fn fetch_missions_bg(url: String, token: Option<String>, tx: tokio::sync::mpsc::Sender<TuiEvent>) {
    let client = reqwest::Client::new();
    let mut req = client.get(format!("{}/api/missions", url));
    if let Some(t) = token {
        req = req.header("Cookie", format!("laruche_auth={}", t));
    }
    match req.send().await {
        Ok(resp) => {
            if let Ok(data) = resp.json::<serde_json::Value>().await {
                if let Some(list) = data.as_array() {
                    let _ = tx.send(TuiEvent::MissionsLoaded(list.clone())).await;
                    return;
                }
            }
            let _ = tx.send(TuiEvent::Error("Failed to parse missions list".into())).await;
        }
        Err(e) => {
            let _ = tx.send(TuiEvent::Error(format!("Network error: {}", e))).await;
        }
    }
}

async fn fetch_mission_dossier_bg(url: String, slug: String, token: Option<String>, tx: tokio::sync::mpsc::Sender<TuiEvent>) {
    let client = reqwest::Client::new();
    let mut req = client.get(format!("{}/api/missions/{}/dossier", url, slug));
    if let Some(t) = token {
        req = req.header("Cookie", format!("laruche_auth={}", t));
    }
    match req.send().await {
        Ok(resp) => {
            if let Ok(data) = resp.json::<serde_json::Value>().await {
                if let Some(md) = data["markdown"].as_str() {
                    let _ = tx.send(TuiEvent::MissionDossierLoaded { slug, markdown: md.to_string() }).await;
                    return;
                }
            }
            let _ = tx.send(TuiEvent::Error("Failed to load mission dossier".into())).await;
        }
        Err(e) => {
            let _ = tx.send(TuiEvent::Error(format!("Network error: {}", e))).await;
        }
    }
}

async fn run_mission_bg(url: String, slug: String, token: Option<String>, tx: tokio::sync::mpsc::Sender<TuiEvent>) {
    let client = reqwest::Client::new();
    let mut req = client.post(format!("{}/api/missions/{}/run", url, slug));
    if let Some(t) = token {
        req = req.header("Cookie", format!("laruche_auth={}", t));
    }
    match req.send().await {
        Ok(resp) => {
            if resp.status().is_success() {
                let _ = tx.send(TuiEvent::ActionFinished("Mission started successfully.".into())).await;
            } else {
                let _ = tx.send(TuiEvent::Error("Failed to start the mission".into())).await;
            }
        }
        Err(e) => {
            let _ = tx.send(TuiEvent::Error(format!("Network error: {}", e))).await;
        }
    }
}

async fn create_mission_bg(url: String, objective: String, token: Option<String>, tx: tokio::sync::mpsc::Sender<TuiEvent>) {
    let client = reqwest::Client::new();
    let mut req = client.post(format!("{}/api/missions", url))
        .json(&serde_json::json!({ "objective": objective }));
    if let Some(t) = token {
        req = req.header("Cookie", format!("laruche_auth={}", t));
    }
    match req.send().await {
        Ok(resp) => {
            if resp.status().is_success() {
                let _ = tx.send(TuiEvent::ActionFinished("Mission created successfully.".into())).await;
            } else {
                let _ = tx.send(TuiEvent::Error("Failed to create the mission".into())).await;
            }
        }
        Err(e) => {
            let _ = tx.send(TuiEvent::Error(format!("Network error: {}", e))).await;
        }
    }
}

async fn update_mission_status_bg(url: String, slug: String, status: String, token: Option<String>, tx: tokio::sync::mpsc::Sender<TuiEvent>) {
    let client = reqwest::Client::new();
    let mut req = client.post(format!("{}/api/missions/{}", url, slug))
        .json(&serde_json::json!({ "status": status }));
    if let Some(t) = token {
        req = req.header("Cookie", format!("laruche_auth={}", t));
    }
    match req.send().await {
        Ok(resp) => {
            if resp.status().is_success() {
                let _ = tx.send(TuiEvent::ActionFinished(format!("Mission status updated to {}", status))).await;
            } else {
                let _ = tx.send(TuiEvent::Error("Update failed".into())).await;
            }
        }
        Err(e) => {
            let _ = tx.send(TuiEvent::Error(format!("Network error: {}", e))).await;
        }
    }
}

async fn delete_mission_bg(url: String, slug: String, token: Option<String>, tx: tokio::sync::mpsc::Sender<TuiEvent>) {
    let client = reqwest::Client::new();
    let mut req = client.delete(format!("{}/api/missions/{}", url, slug));
    if let Some(t) = token {
        req = req.header("Cookie", format!("laruche_auth={}", t));
    }
    match req.send().await {
        Ok(resp) => {
            if resp.status().is_success() {
                let _ = tx.send(TuiEvent::ActionFinished("Mission deleted.".into())).await;
            } else {
                let _ = tx.send(TuiEvent::Error("Deletion failed".into())).await;
            }
        }
        Err(e) => {
            let _ = tx.send(TuiEvent::Error(format!("Network error: {}", e))).await;
        }
    }
}

async fn decompose_mission_bg(url: String, slug: String, token: Option<String>, tx: tokio::sync::mpsc::Sender<TuiEvent>) {
    let client = reqwest::Client::new();
    let mut req = client.post(format!("{}/api/missions/{}/decompose", url, slug));
    if let Some(t) = token {
        req = req.header("Cookie", format!("laruche_auth={}", t));
    }
    match req.send().await {
        Ok(resp) => {
            if resp.status().is_success() {
                let _ = tx.send(TuiEvent::ActionFinished("Mission decomposed into kanban tasks.".into())).await;
            } else {
                let _ = tx.send(TuiEvent::Error("Decomposition failed".into())).await;
            }
        }
        Err(e) => {
            let _ = tx.send(TuiEvent::Error(format!("Network error: {}", e))).await;
        }
    }
}

async fn create_memory_node_bg(url: String, parent_id: String, label: String, one_liner: String, token: Option<String>, tx: tokio::sync::mpsc::Sender<TuiEvent>) {
    let client = reqwest::Client::new();
    let slug = slugify(&label);
    let node_id = if parent_id.is_empty() {
        slug
    } else {
        format!("{}.{}", parent_id, slug)
    };
    let mut req = client.post(format!("{}/api/memory/node/create", url))
        .json(&serde_json::json!({
            "node_id": node_id,
            "label": label,
            "one_liner": one_liner,
        }));
    if let Some(t) = token {
        req = req.header("Cookie", format!("laruche_auth={}", t));
    }
    match req.send().await {
        Ok(resp) => {
            if resp.status().is_success() {
                let _ = tx.send(TuiEvent::ActionFinished(format!("Node created: {}", node_id))).await;
            } else {
                let _ = tx.send(TuiEvent::Error("Failed to create the node".into())).await;
            }
        }
        Err(e) => {
            let _ = tx.send(TuiEvent::Error(format!("Network error: {}", e))).await;
        }
    }
}

async fn add_memory_item_bg(url: String, node_id: String, content: String, token: Option<String>, tx: tokio::sync::mpsc::Sender<TuiEvent>) {
    let client = reqwest::Client::new();
    let mut req = client.post(format!("{}/api/memory/write", url))
        .json(&serde_json::json!({
            "node_id": node_id,
            "content": content,
        }));
    if let Some(t) = token {
        req = req.header("Cookie", format!("laruche_auth={}", t));
    }
    match req.send().await {
        Ok(resp) => {
            if resp.status().is_success() {
                let _ = tx.send(TuiEvent::ActionFinished("Item added successfully.".into())).await;
            } else {
                let _ = tx.send(TuiEvent::Error("Failed to add the item".into())).await;
            }
        }
        Err(e) => {
            let _ = tx.send(TuiEvent::Error(format!("Network error: {}", e))).await;
        }
    }
}

async fn delete_memory_item_bg(url: String, item_id: String, token: Option<String>, tx: tokio::sync::mpsc::Sender<TuiEvent>) {
    let client = reqwest::Client::new();
    let mut req = client.post(format!("{}/api/memory/delete", url))
        .json(&serde_json::json!({
            "item_id": item_id,
        }));
    if let Some(t) = token {
        req = req.header("Cookie", format!("laruche_auth={}", t));
    }
    match req.send().await {
        Ok(resp) => {
            if resp.status().is_success() {
                let _ = tx.send(TuiEvent::ActionFinished("Item deleted.".into())).await;
            } else {
                let _ = tx.send(TuiEvent::Error("Deletion failed".into())).await;
            }
        }
        Err(e) => {
            let _ = tx.send(TuiEvent::Error(format!("Network error: {}", e))).await;
        }
    }
}

async fn delete_memory_node_bg(url: String, node_id: String, token: Option<String>, tx: tokio::sync::mpsc::Sender<TuiEvent>) {
    let client = reqwest::Client::new();
    let mut req = client.post(format!("{}/api/memory/node/delete", url))
        .json(&serde_json::json!({
            "node_id": node_id,
        }));
    if let Some(t) = token {
        req = req.header("Cookie", format!("laruche_auth={}", t));
    }
    match req.send().await {
        Ok(resp) => {
            if resp.status().is_success() {
                let _ = tx.send(TuiEvent::ActionFinished(format!("Node deleted: {}", node_id))).await;
            } else {
                let _ = tx.send(TuiEvent::Error("Failed to delete the node".into())).await;
            }
        }
        Err(e) => {
            let _ = tx.send(TuiEvent::Error(format!("Network error: {}", e))).await;
        }
    }
}

fn slugify(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect::<String>()
        .split('_')
        .filter(|seg| !seg.is_empty())
        .collect::<Vec<_>>()
        .join("_")
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
        if probe_server(&url).await {
            return url;
        }
    }

    // 2. mDNS discovery via laruche-client
    if let Ok(lr) = laruche_client::LaRuche::discover().await {
        for node in lr.nodes() {
            if let Some(url) = node.manifest.api_url() {
                if probe_server(&url).await {
                    return url;
                }
            }
        }
    }

    // 3. Localhost probe
    let local = "http://127.0.0.1:8419".to_string();
    if probe_server(&local).await {
        return local;
    }

    String::new()
}

async fn probe_server(url: &str) -> bool {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
        .ok()
        .and_then(|c| {
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    c.get(format!("{}/health", url))
                        .send()
                        .await
                        .ok()
                        .map(|r| r.status().is_success())
                })
            })
        })
        .unwrap_or(false)
}

async fn fetch_model(url: &str) -> String {
    if url.is_empty() {
        return std::env::var("LARUCHE_MODEL").unwrap_or_else(|_| "?".into());
    }
    reqwest::Client::new()
        .get(format!("{}/models", url))
        .send()
        .await
        .ok()
        .and_then(|r| {
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(r.json::<serde_json::Value>())
            })
            .ok()
        })
        .and_then(|d| {
            // Prefer the default_model field from ModelsResponse
            d["default_model"]
                .as_str()
                .filter(|s| !s.is_empty())
                .map(String::from)
                .or_else(|| {
                    d["models"]
                        .as_array()?
                        .first()?
                        .get("name")?
                        .as_str()
                        .map(String::from)
                })
        })
        .unwrap_or_else(|| "?".into())
}

async fn fetch_tools(url: &str) -> Vec<String> {
    if url.is_empty() {
        return vec![];
    }
    reqwest::Client::new()
        .get(format!("{}/api/tools", url))
        .send()
        .await
        .ok()
        .and_then(|r| {
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(r.json::<serde_json::Value>())
            })
            .ok()
        })
        .and_then(|d| {
            d.as_array().map(|a| {
                a.iter()
                    .filter_map(|t| t["name"].as_str().map(String::from))
                    .collect()
            })
        })
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
        url.trim_start_matches("https://")
            .trim_start_matches("http://")
    );

    // Build WS request with auth cookie if available
    let ws_request = if let Some(ref token) = auth_token {
        tokio_tungstenite::tungstenite::http::Request::builder()
            .uri(&ws_url)
            .header("Cookie", format!("laruche_auth={}", token))
            .header(
                "Host",
                url.trim_start_matches("https://")
                    .trim_start_matches("http://"),
            )
            .header("Connection", "Upgrade")
            .header("Upgrade", "websocket")
            .header("Sec-WebSocket-Version", "13")
            .header(
                "Sec-WebSocket-Key",
                tokio_tungstenite::tungstenite::handshake::client::generate_key(),
            )
            .body(())
            .unwrap()
    } else {
        tokio_tungstenite::tungstenite::http::Request::builder()
            .uri(&ws_url)
            .header(
                "Host",
                url.trim_start_matches("https://")
                    .trim_start_matches("http://"),
            )
            .header("Connection", "Upgrade")
            .header("Upgrade", "websocket")
            .header("Sec-WebSocket-Version", "13")
            .header(
                "Sec-WebSocket-Key",
                tokio_tungstenite::tungstenite::handshake::client::generate_key(),
            )
            .body(())
            .unwrap()
    };

    // Try WebSocket first
    match tokio_tungstenite::connect_async(ws_request).await {
        Ok((ws_stream, _)) => {
            let (mut write, mut read) = ws_stream.split();

            // Send the message
            let payload = serde_json::json!({ "type": "message", "text": text, "model": model });
            if let Err(e) = write
                .send(WsMessage::Text(payload.to_string().into()))
                .await
            {
                let _ = tx
                    .send(TuiEvent::Error(format!("WS send error: {}", e)))
                    .await;
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
                                    let args = data["args"]
                                        .as_str()
                                        .or_else(|| data["arguments"].as_str())
                                        .unwrap_or("")
                                        .to_string();
                                    let _ = tx.send(TuiEvent::ToolCall { name, args }).await;
                                }
                                "tool_result" => {
                                    let name = data["name"].as_str().unwrap_or("?").to_string();
                                    let success = data["success"].as_bool().unwrap_or(true);
                                    let ms = data["elapsed_ms"]
                                        .as_u64()
                                        .or_else(|| data["ms"].as_u64())
                                        .unwrap_or(0);
                                    let _ =
                                        tx.send(TuiEvent::ToolResult { name, success, ms }).await;
                                }
                                "plan" => {
                                    let steps: Vec<(String, String)> = data["steps"]
                                        .as_array()
                                        .map(|arr| {
                                            arr.iter()
                                                .map(|s| {
                                                    let task = s["task"]
                                                        .as_str()
                                                        .unwrap_or("?")
                                                        .to_string();
                                                    let status = s["status"]
                                                        .as_str()
                                                        .unwrap_or("pending")
                                                        .to_string();
                                                    (task, status)
                                                })
                                                .collect()
                                        })
                                        .unwrap_or_default();
                                    let _ = tx.send(TuiEvent::Plan(steps)).await;
                                }
                                "thinking" => {
                                    let thought = data["text"].as_str().unwrap_or("").to_string();
                                    let _ = tx.send(TuiEvent::Thinking(thought)).await;
                                }
                                "done" => {
                                    // The server may send the full response in "text"
                                    let final_text = data["text"]
                                        .as_str()
                                        .map(|s| s.to_string())
                                        .unwrap_or(full_response.clone());
                                    let _ = tx.send(TuiEvent::Done(final_text)).await;
                                    return;
                                }
                                "error" => {
                                    let err = data["text"]
                                        .as_str()
                                        .or_else(|| data["message"].as_str())
                                        .unwrap_or("Unknown error")
                                        .to_string();
                                    let _ = tx.send(TuiEvent::Error(err)).await;
                                    let _ = tx.send(TuiEvent::Done(full_response.clone())).await;
                                    return;
                                }
                                _ => {
                                    // Handle thought/status/compaction/failover as agent activity
                                    // (not chat tokens)
                                    match event_type {
                                        "thought" | "status" | "compaction" | "failover" => {
                                            let thought_text = data["text"]
                                                .as_str()
                                                .or_else(|| data["message"].as_str())
                                                .unwrap_or("")
                                                .to_string();
                                            if !thought_text.is_empty() {
                                                let _ = tx.send(TuiEvent::Thinking(thought_text)).await;
                                            }
                                        }
                                        _ => {
                                            // Truly unknown event: if it has text, treat as token
                                            if let Some(t) = data["text"].as_str() {
                                                full_response.push_str(t);
                                                let _ = tx.send(TuiEvent::Token(t.to_string())).await;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Ok(WsMessage::Close(_)) => {
                        // Server closed the connection: finalize
                        let _ = tx.send(TuiEvent::Done(full_response)).await;
                        return;
                    }
                    Err(e) => {
                        let _ = tx
                            .send(TuiEvent::Error(format!("WS read error: {}", e)))
                            .await;
                        let _ = tx.send(TuiEvent::Done(full_response)).await;
                        return;
                    }
                    _ => {} // Ping/Pong/Binary: ignore
                }
            }

            // Stream ended without explicit done
            let _ = tx.send(TuiEvent::Done(full_response)).await;
        }
        Err(ws_err) => {
            // WebSocket connection failed: fall back to HTTP POST /api/webhook
            let _ = tx
                .send(TuiEvent::Thinking(format!(
                    "WS failed ({}), falling back to HTTP...",
                    ws_err
                )))
                .await;
            fallback_http_send(&url, &text, &tx).await;
        }
    }
}

/// Fallback: POST /api/webhook and send the result as a single Done event.
async fn fallback_http_send(url: &str, text: &str, tx: &tokio::sync::mpsc::Sender<TuiEvent>) {
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
                        let _ = tx
                            .send(TuiEvent::Error(format!("Server error: {}", err)))
                            .await;
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
                        let _ = tx
                            .send(TuiEvent::ToolResult {
                                name,
                                success: ok,
                                ms,
                            })
                            .await;
                    }
                }
                let _ = tx.send(TuiEvent::Done(clean.trim().to_string())).await;
            } else {
                let _ = tx
                    .send(TuiEvent::Error("Error parsing response".into()))
                    .await;
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
    execute!(stdout, EnterAlternateScreen, crossterm::event::EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new().await;
    let (ui_tx, mut ui_rx) = tokio::sync::mpsc::channel::<TuiEvent>(100);
    app.ui_tx = Some(ui_tx);

    loop {
        // Drain general UI background events
        while let Ok(evt) = ui_rx.try_recv() {
            match evt {
                TuiEvent::MemoryTreeLoaded(nodes) => {
                    app.memory_nodes = nodes;
                    if !app.memory_nodes.is_empty() {
                        let mut draw_items = Vec::new();
                        let mut visited = std::collections::HashSet::new();
                        build_draw_tree(&app.memory_nodes, None, 0, &mut visited, &mut draw_items);
                        if app.selected_node_idx >= draw_items.len() {
                            app.selected_node_idx = 0;
                        }
                        if let Some(item) = draw_items.get(app.selected_node_idx) {
                            app.trigger_load_node_details(item.id.clone());
                        }
                    }
                }
                TuiEvent::NodeDetailsLoaded(node) => {
                    app.selected_node_details = Some(node);
                    if let Some(items) = app.selected_node_details.as_ref().and_then(|n| n["items"].as_array()) {
                        if app.selected_item_idx >= items.len() {
                            app.selected_item_idx = 0;
                        }
                    }
                }
                TuiEvent::MissionsLoaded(list) => {
                    app.missions = list;
                    if !app.missions.is_empty() {
                        if app.selected_mission_idx >= app.missions.len() {
                            app.selected_mission_idx = 0;
                        }
                        if let Some(m) = app.missions.get(app.selected_mission_idx) {
                            if let Some(slug) = m["slug"].as_str() {
                                app.trigger_load_mission_dossier(slug.to_string());
                            }
                        }
                    }
                }
                TuiEvent::MissionDossierLoaded { slug, markdown } => {
                    if let Some(m) = app.missions.get(app.selected_mission_idx) {
                        if m["slug"].as_str() == Some(&slug) {
                            app.selected_mission_dossier = Some(markdown);
                        }
                    }
                }
                TuiEvent::ActionFinished(msg) => {
                    app.status_msg = msg.clone();
                    app.activity_log.push(format!("[OK] [{}] {}", chrono::Local::now().format("%H:%M:%S"), msg));
                    match app.current_screen {
                        Screen::Memory => {
                            app.trigger_load_memory();
                            let mut draw_items = Vec::new();
                            let mut visited = std::collections::HashSet::new();
                            build_draw_tree(&app.memory_nodes, None, 0, &mut visited, &mut draw_items);
                            if let Some(item) = draw_items.get(app.selected_node_idx) {
                                app.trigger_load_node_details(item.id.clone());
                            }
                        }
                        Screen::Missions => {
                            app.trigger_load_missions();
                        }
                        _ => {}
                    }
                }
                TuiEvent::Error(err) => {
                    app.status_msg = format!("Error: {}", err);
                    app.activity_log.push(format!("[ERR] [{}] {}", chrono::Local::now().format("%H:%M:%S"), err));
                }
                _ => {}
            }
        }
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
                                    "[TOOL] [{}] Call: {} ({})",
                                    ts, name, short_args
                                ));
                                app.messages.push(ChatMessage {
                                    role: "tool".into(),
                                    text: format!("{} {}", name, short_args),
                                });
                                app.status_msg = format!("Tool: {}...", name);
                            }
                            TuiEvent::ToolResult { name, success, ms } => {
                                let icon = if success { "✓" } else { "✗" };
                                app.activity_log
                                    .push(format!("[TOOL] [{}] {} {} ({}ms)", ts, icon, name, ms));
                                app.status_msg = format!("{} {} ({}ms)", icon, name, ms);
                            }
                            TuiEvent::Plan(steps) => {
                                app.plan = steps;
                                app.activity_log.push(format!(
                                    "[{}] Plan updated ({} steps)",
                                    ts,
                                    app.plan.len()
                                ));
                            }
                            TuiEvent::Thinking(thought) => {
                                let short = if thought.chars().count() > 60 {
                                    format!("{}...", thought.chars().take(60).collect::<String>())
                                } else {
                                    thought
                                };
                                app.activity_log.push(format!("[THINK] [{}] {}", ts, short));
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
                                    let cleaned = markdown::clean_agent_text(&text);
                                    if !cleaned.is_empty() {
                                        app.messages.push(ChatMessage {
                                            role: "assistant".into(),
                                            text: cleaned,
                                        });
                                    }
                                    app.activity_log.push(format!(
                                        "[OK] [{}] Response: {} chars",
                                        ts,
                                        text.chars().count()
                                    ));
                                } else {
                                    app.activity_log
                                        .push(format!("[ERR] [{}] Empty response", ts));
                                }
                                app.is_streaming = false;
                                app.status_msg = "Ready".into();
                                app.streaming_response.clear();
                                // Auto scroll
                                let total = app
                                    .messages
                                    .iter()
                                    .map(|m| m.text.lines().count() + 1)
                                    .sum::<usize>();
                                app.chat_scroll = total.saturating_sub(12) as u16;
                                done_received = true;
                            }
                            TuiEvent::Error(err) => {
                                app.activity_log.push(format!("[ERR] [{}] {}", ts, err));
                                app.status_msg = format!("Error: {}", err);
                            }
                            _ => {}
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
                    app.status_msg = "Connection error".into();
                }
            }
        }

        terminal.draw(|f| ui(f, &mut app))?;
        if app.should_quit {
            break;
        }

        if event::poll(std::time::Duration::from_millis(50))? {
            let ev = event::read()?;
            match ev {
                Event::Key(key) => {
                    if key.kind != crossterm::event::KeyEventKind::Press {
                        continue;
                    }

                // Global
                match (key.modifiers, key.code) {
                    (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
                        if app.is_streaming {
                            if let Some(task) = app.stream_task.take() {
                                task.abort();
                            }
                            app.is_streaming = false;
                            app.status_msg = "Generation interrupted.".into();
                            app.activity_log.push(format!("[INFO] [{}] Generation interrupted by the user", chrono::Local::now().format("%H:%M:%S")));
                        } else {
                            app.should_quit = true;
                        }
                        continue;
                    }
                    (KeyModifiers::CONTROL, KeyCode::Char('q')) => {
                        app.should_quit = true;
                        continue;
                    }
                    (KeyModifiers::NONE, KeyCode::F(1)) | (KeyModifiers::CONTROL, KeyCode::Char('1')) => {
                        app.current_screen = Screen::Chat;
                        app.active_panel = Panel::Input;
                        continue;
                    }
                    (KeyModifiers::NONE, KeyCode::F(2)) | (KeyModifiers::CONTROL, KeyCode::Char('2')) => {
                        app.current_screen = Screen::Memory;
                        app.active_panel = Panel::MemoryTree;
                        app.trigger_load_memory();
                        continue;
                    }
                    (KeyModifiers::NONE, KeyCode::F(3)) | (KeyModifiers::CONTROL, KeyCode::Char('3')) => {
                        app.current_screen = Screen::Missions;
                        app.active_panel = Panel::MissionsList;
                        app.trigger_load_missions();
                        continue;
                    }
                    (KeyModifiers::NONE, KeyCode::Tab) => {
                        match app.current_screen {
                            Screen::Chat => {
                                app.current_screen = Screen::Memory;
                                app.active_panel = Panel::MemoryTree;
                                app.memory_active_pane = MemoryPane::Tree;
                                app.trigger_load_memory();
                            }
                            Screen::Memory => {
                                if app.memory_active_pane == MemoryPane::Tree {
                                    app.memory_active_pane = MemoryPane::Details;
                                } else {
                                    app.current_screen = Screen::Missions;
                                    app.active_panel = Panel::MissionsList;
                                    app.missions_active_pane = MissionsPane::List;
                                    app.trigger_load_missions();
                                }
                            }
                            Screen::Missions => {
                                if app.missions_active_pane == MissionsPane::List {
                                    app.missions_active_pane = MissionsPane::Dossier;
                                } else {
                                    app.current_screen = Screen::Chat;
                                    app.active_panel = Panel::Input;
                                }
                            }
                        }
                        continue;
                    }
                    (KeyModifiers::SHIFT, KeyCode::BackTab) => {
                        match app.current_screen {
                            Screen::Chat => {
                                app.current_screen = Screen::Missions;
                                app.active_panel = Panel::MissionsList;
                                app.missions_active_pane = MissionsPane::Dossier;
                                app.trigger_load_missions();
                            }
                            Screen::Memory => {
                                if app.memory_active_pane == MemoryPane::Details {
                                    app.memory_active_pane = MemoryPane::Tree;
                                } else {
                                    app.current_screen = Screen::Chat;
                                    app.active_panel = Panel::Input;
                                }
                            }
                            Screen::Missions => {
                                if app.missions_active_pane == MissionsPane::Dossier {
                                    app.missions_active_pane = MissionsPane::List;
                                } else {
                                    app.current_screen = Screen::Memory;
                                    app.active_panel = Panel::MemoryTree;
                                    app.memory_active_pane = MemoryPane::Details;
                                    app.trigger_load_memory();
                                }
                            }
                        }
                        continue;
                    }
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
                        KeyCode::Esc | KeyCode::Enter => app.active_panel = Panel::Input,
                        _ => {}
                    },
                    Panel::MemoryTree => {
                        match app.memory_active_pane {
                            MemoryPane::Tree => {
                                let mut draw_items = Vec::new();
                                let mut visited = std::collections::HashSet::new();
                                build_draw_tree(&app.memory_nodes, None, 0, &mut visited, &mut draw_items);
                                match key.code {
                                    KeyCode::Up => {
                                        if !draw_items.is_empty()
                                            && app.selected_node_idx > 0 {
                                                app.selected_node_idx -= 1;
                                                if let Some(item) = draw_items.get(app.selected_node_idx) {
                                                    app.trigger_load_node_details(item.id.clone());
                                                }
                                            }
                                    }
                                    KeyCode::Down => {
                                        if !draw_items.is_empty()
                                            && app.selected_node_idx + 1 < draw_items.len() {
                                                app.selected_node_idx += 1;
                                                if let Some(item) = draw_items.get(app.selected_node_idx) {
                                                    app.trigger_load_node_details(item.id.clone());
                                                }
                                            }
                                    }
                                    KeyCode::Char('n') | KeyCode::Char('N') => {
                                        app.memory_input_mode = MemoryInputMode::CreateNode;
                                        app.active_panel = Panel::Input;
                                        app.input.clear();
                                        app.cursor_pos = 0;
                                    }
                                    KeyCode::Char('a') | KeyCode::Char('A') => {
                                        app.memory_input_mode = MemoryInputMode::AddItem;
                                        app.active_panel = Panel::Input;
                                        app.input.clear();
                                        app.cursor_pos = 0;
                                    }
                                    KeyCode::Char('e') | KeyCode::Char('E') => {
                                        if let Some(item) = draw_items.get(app.selected_node_idx) {
                                            if !item.is_protected {
                                                app.memory_input_mode = MemoryInputMode::EditNode;
                                                app.active_panel = Panel::Input;
                                                app.input = item.label.clone();
                                                app.cursor_pos = app.input.chars().count();
                                            }
                                        }
                                    }
                                    KeyCode::Char('d') | KeyCode::Char('D') => {
                                        if let Some(item) = draw_items.get(app.selected_node_idx) {
                                            if !item.is_protected {
                                                let url = app.server_url.clone();
                                                let token = app.auth_token.clone();
                                                let node_id = item.id.clone();
                                                if let Some(ref tx) = app.ui_tx {
                                                    let tx = tx.clone();
                                                    tokio::spawn(async move {
                                                        delete_memory_node_bg(url, node_id, token, tx).await;
                                                    });
                                                }
                                            }
                                        }
                                    }
                                    KeyCode::Esc => {
                                        app.active_panel = Panel::Input;
                                    }
                                    _ => {}
                                }
                            }
                            MemoryPane::Details => {
                                let mut item_count = 0;
                                if let Some(ref details) = app.selected_node_details {
                                    if let Some(items) = details["items"].as_array() {
                                        item_count = items.len();
                                    }
                                }
                                match key.code {
                                    KeyCode::Up => {
                                        if item_count > 0 && app.selected_item_idx > 0 {
                                            app.selected_item_idx -= 1;
                                        }
                                    }
                                    KeyCode::Down => {
                                        if item_count > 0 && app.selected_item_idx + 1 < item_count {
                                            app.selected_item_idx += 1;
                                        }
                                    }
                                    KeyCode::Char('a') | KeyCode::Char('A') => {
                                        app.memory_input_mode = MemoryInputMode::AddItem;
                                        app.active_panel = Panel::Input;
                                        app.input.clear();
                                        app.cursor_pos = 0;
                                    }
                                    KeyCode::Char('e') | KeyCode::Char('E') => {
                                        if let Some(ref details) = app.selected_node_details {
                                            if let Some(items) = details["items"].as_array() {
                                                if let Some(item) = items.get(app.selected_item_idx) {
                                                    if let Some(content) = item["content"].as_str() {
                                                        app.memory_input_mode = MemoryInputMode::EditItem;
                                                        app.active_panel = Panel::Input;
                                                        app.input = content.to_string();
                                                        app.cursor_pos = app.input.chars().count();
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    KeyCode::Char('d') | KeyCode::Char('D') => {
                                        if let Some(ref details) = app.selected_node_details {
                                            if let Some(items) = details["items"].as_array() {
                                                if let Some(item) = items.get(app.selected_item_idx) {
                                                    if let Some(item_id) = item["id"].as_str() {
                                                        let url = app.server_url.clone();
                                                        let token = app.auth_token.clone();
                                                        let itm_id = item_id.to_string();
                                                        if let Some(ref tx) = app.ui_tx {
                                                            let tx = tx.clone();
                                                            tokio::spawn(async move {
                                                                delete_memory_item_bg(url, itm_id, token, tx).await;
                                                            });
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    KeyCode::Esc => {
                                        app.active_panel = Panel::Input;
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                    Panel::MissionsList => {
                        match app.missions_active_pane {
                            MissionsPane::List => {
                                match key.code {
                                    KeyCode::Up => {
                                        if !app.missions.is_empty() && app.selected_mission_idx > 0 {
                                            app.selected_mission_idx -= 1;
                                            if let Some(m) = app.missions.get(app.selected_mission_idx) {
                                                if let Some(slug) = m["slug"].as_str() {
                                                    app.trigger_load_mission_dossier(slug.to_string());
                                                }
                                            }
                                        }
                                    }
                                    KeyCode::Down => {
                                        if !app.missions.is_empty() && app.selected_mission_idx + 1 < app.missions.len() {
                                            app.selected_mission_idx += 1;
                                            if let Some(m) = app.missions.get(app.selected_mission_idx) {
                                                if let Some(slug) = m["slug"].as_str() {
                                                    app.trigger_load_mission_dossier(slug.to_string());
                                                }
                                            }
                                        }
                                    }
                                    KeyCode::Char('c') | KeyCode::Char('C') => {
                                        app.missions_input_mode = MissionsInputMode::CreateMission;
                                        app.active_panel = Panel::Input;
                                        app.input.clear();
                                        app.cursor_pos = 0;
                                    }
                                    KeyCode::Char('r') | KeyCode::Char('R') => {
                                        if let Some(m) = app.missions.get(app.selected_mission_idx) {
                                            if let Some(slug) = m["slug"].as_str() {
                                                let url = app.server_url.clone();
                                                let token = app.auth_token.clone();
                                                let slg = slug.to_string();
                                                if let Some(ref tx) = app.ui_tx {
                                                    let tx = tx.clone();
                                                    tokio::spawn(async move {
                                                        run_mission_bg(url, slg, token, tx).await;
                                                    });
                                                }
                                            }
                                        }
                                    }
                                    KeyCode::Char('p') | KeyCode::Char('P') => {
                                        if let Some(m) = app.missions.get(app.selected_mission_idx) {
                                            if let Some(slug) = m["slug"].as_str() {
                                                let status = if m["status"] == "paused" { "active" } else { "paused" };
                                                let url = app.server_url.clone();
                                                let token = app.auth_token.clone();
                                                let slg = slug.to_string();
                                                let new_status = status.to_string();
                                                if let Some(ref tx) = app.ui_tx {
                                                    let tx = tx.clone();
                                                    tokio::spawn(async move {
                                                        update_mission_status_bg(url, slg, new_status, token, tx).await;
                                                    });
                                                }
                                            }
                                        }
                                    }
                                    KeyCode::Char('d') | KeyCode::Char('D') => {
                                        if let Some(m) = app.missions.get(app.selected_mission_idx) {
                                            if let Some(slug) = m["slug"].as_str() {
                                                let url = app.server_url.clone();
                                                let token = app.auth_token.clone();
                                                let slg = slug.to_string();
                                                if let Some(ref tx) = app.ui_tx {
                                                    let tx = tx.clone();
                                                    tokio::spawn(async move {
                                                        delete_mission_bg(url, slg, token, tx).await;
                                                    });
                                                }
                                            }
                                        }
                                    }
                                    KeyCode::Char('k') | KeyCode::Char('K') => {
                                        if let Some(m) = app.missions.get(app.selected_mission_idx) {
                                            if let Some(slug) = m["slug"].as_str() {
                                                let url = app.server_url.clone();
                                                let token = app.auth_token.clone();
                                                let slg = slug.to_string();
                                                if let Some(ref tx) = app.ui_tx {
                                                    let tx = tx.clone();
                                                    tokio::spawn(async move {
                                                        decompose_mission_bg(url, slg, token, tx).await;
                                                    });
                                                }
                                            }
                                        }
                                    }
                                    KeyCode::Esc => {
                                        app.active_panel = Panel::Input;
                                    }
                                    _ => {}
                                }
                            }
                            MissionsPane::Dossier => {
                                match key.code {
                                    KeyCode::Up => {
                                        app.missions_dossier_scroll = app.missions_dossier_scroll.saturating_sub(2);
                                    }
                                    KeyCode::Down => {
                                        app.missions_dossier_scroll = app.missions_dossier_scroll.saturating_add(2);
                                    }
                                    KeyCode::Esc => {
                                        app.active_panel = Panel::Input;
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
                }
                Event::Mouse(mouse) => {
                    let size = terminal.size().unwrap_or_default();
                    let area = Rect::new(0, 0, size.width, size.height);
                    let input_lines = app.input.lines().count().clamp(1, 10) as u16 + 2;
                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([
                            Constraint::Length(6), // Header
                            Constraint::Min(10),   // Content
                            Constraint::Length(input_lines), // Input
                            Constraint::Length(1), // Footer/Status
                        ])
                        .split(area);

                    let row = mouse.row;
                    let col = mouse.column;

                    match mouse.kind {
                        // ── Mouse scroll ──
                        crossterm::event::MouseEventKind::ScrollUp => {
                            if row >= chunks[1].y && row < chunks[1].y + chunks[1].height {
                                if app.current_screen == Screen::Chat {
                                    // Determine if scroll is in sidebar or chat
                                    if col < 36 {
                                        app.sidebar_scroll = app.sidebar_scroll.saturating_sub(3);
                                    } else {
                                        app.chat_scroll = app.chat_scroll.saturating_sub(3);
                                    }
                                } else if app.current_screen == Screen::Memory {
                                    if col < 38 {
                                        app.memory_tree_scroll = app.memory_tree_scroll.saturating_sub(3);
                                    } else {
                                        app.memory_details_scroll = app.memory_details_scroll.saturating_sub(3);
                                    }
                                } else if app.current_screen == Screen::Missions
                                    && col >= 38 {
                                        app.missions_dossier_scroll = app.missions_dossier_scroll.saturating_sub(3);
                                    }
                            }
                        }
                        crossterm::event::MouseEventKind::ScrollDown => {
                            if row >= chunks[1].y && row < chunks[1].y + chunks[1].height {
                                if app.current_screen == Screen::Chat {
                                    if col < 36 {
                                        app.sidebar_scroll = app.sidebar_scroll.saturating_add(3);
                                    } else {
                                        app.chat_scroll = app.chat_scroll.saturating_add(3);
                                    }
                                } else if app.current_screen == Screen::Memory {
                                    if col < 38 {
                                        app.memory_tree_scroll = app.memory_tree_scroll.saturating_add(3);
                                    } else {
                                        app.memory_details_scroll = app.memory_details_scroll.saturating_add(3);
                                    }
                                } else if app.current_screen == Screen::Missions
                                    && col >= 38 {
                                        app.missions_dossier_scroll = app.missions_dossier_scroll.saturating_add(3);
                                    }
                            }
                        }
                        // ── Mouse click ──
                        crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left) => {
                            // 1. Header click (tabs navigation) at row 4
                            if row == 4 {
                                if (45..=51).contains(&col) {
                                    app.current_screen = Screen::Chat;
                                    app.active_panel = Panel::Input;
                                } else if (52..=61).contains(&col) {
                                    app.current_screen = Screen::Memory;
                                    app.active_panel = Panel::MemoryTree;
                                    app.memory_active_pane = MemoryPane::Tree;
                                    app.trigger_load_memory();
                                } else if (62..=73).contains(&col) {
                                    app.current_screen = Screen::Missions;
                                    app.active_panel = Panel::MissionsList;
                                    app.missions_active_pane = MissionsPane::List;
                                    app.trigger_load_missions();
                                }
                            }
                            // 2. Content area click
                            else if row >= chunks[1].y && row < chunks[1].y + chunks[1].height {
                                if app.current_screen == Screen::Chat {
                                    // Agent sidebar (left 36 cols) or Chat (right)
                                    if col < 36 {
                                        // Clicked in agent sidebar: no panel change needed
                                    } else {
                                        app.active_panel = Panel::Chat;
                                    }
                                } else if app.current_screen == Screen::Memory {
                                    let mem_chunks = Layout::default()
                                        .direction(Direction::Horizontal)
                                        .constraints([Constraint::Length(38), Constraint::Min(0)])
                                        .split(chunks[1]);
                                    if col >= mem_chunks[0].x && col < mem_chunks[0].x + mem_chunks[0].width {
                                        app.active_panel = Panel::MemoryTree;
                                        app.memory_active_pane = MemoryPane::Tree;
                                    } else {
                                        app.active_panel = Panel::MemoryTree;
                                        app.memory_active_pane = MemoryPane::Details;
                                    }
                                } else if app.current_screen == Screen::Missions {
                                    let mis_chunks = Layout::default()
                                        .direction(Direction::Horizontal)
                                        .constraints([Constraint::Length(38), Constraint::Min(0)])
                                        .split(chunks[1]);
                                    if col >= mis_chunks[0].x && col < mis_chunks[0].x + mis_chunks[0].width {
                                        app.active_panel = Panel::MissionsList;
                                        app.missions_active_pane = MissionsPane::List;
                                    } else {
                                        app.active_panel = Panel::MissionsList;
                                        app.missions_active_pane = MissionsPane::Dossier;
                                    }
                                } else {
                                    app.active_panel = Panel::Chat;
                                }
                            }
                            // 3. Input area click
                            else if row >= chunks[2].y && row < chunks[2].y + chunks[2].height {
                                app.active_panel = Panel::Input;
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }

    app.save_config();
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), crossterm::event::DisableMouseCapture, LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

async fn handle_input(app: &mut App, key: KeyCode) {
    if app.current_screen == Screen::Memory && app.memory_input_mode != MemoryInputMode::Normal {
        match key {
            KeyCode::Enter => {
                let text = app.input.trim().to_string();
                app.input.clear();
                app.cursor_pos = 0;
                let mode = app.memory_input_mode.clone();
                app.memory_input_mode = MemoryInputMode::Normal;
                app.active_panel = Panel::MemoryTree;
                if !text.is_empty() {
                    let url = app.server_url.clone();
                    let token = app.auth_token.clone();
                    if let Some(ref tx) = app.ui_tx {
                        let tx = tx.clone();
                        match mode {
                            MemoryInputMode::CreateNode => {
                                let mut draw_items = Vec::new();
                                let mut visited = std::collections::HashSet::new();
                                build_draw_tree(&app.memory_nodes, None, 0, &mut visited, &mut draw_items);
                                let parent_id = if let Some(item) = draw_items.get(app.selected_node_idx) {
                                    item.id.clone()
                                } else {
                                    "".to_string()
                                };
                                tokio::spawn(async move {
                                    create_memory_node_bg(url, parent_id, text, "".into(), token, tx).await;
                                });
                            }
                            MemoryInputMode::AddItem => {
                                let mut draw_items = Vec::new();
                                let mut visited = std::collections::HashSet::new();
                                build_draw_tree(&app.memory_nodes, None, 0, &mut visited, &mut draw_items);
                                if let Some(item) = draw_items.get(app.selected_node_idx) {
                                    let node_id = item.id.clone();
                                    tokio::spawn(async move {
                                        add_memory_item_bg(url, node_id, text, token, tx).await;
                                    });
                                }
                            }
                            MemoryInputMode::EditNode => {
                                let mut draw_items = Vec::new();
                                let mut visited = std::collections::HashSet::new();
                                build_draw_tree(&app.memory_nodes, None, 0, &mut visited, &mut draw_items);
                                if let Some(item) = draw_items.get(app.selected_node_idx) {
                                    let node_id = item.id.clone();
                                    tokio::spawn(async move {
                                        let client = reqwest::Client::new();
                                        let mut req = client.post(format!("{}/api/memory/node/update", url))
                                            .json(&serde_json::json!({ "node_id": node_id, "label": text }));
                                        if let Some(t) = token {
                                            req = req.header("Cookie", format!("laruche_auth={}", t));
                                        }
                                        match req.send().await {
                                            Ok(resp) if resp.status().is_success() => {
                                                let _ = tx.send(TuiEvent::ActionFinished("Node updated.".into())).await;
                                            }
                                            _ => {
                                                let _ = tx.send(TuiEvent::Error("Update failed".into())).await;
                                            }
                                        }
                                    });
                                }
                            }
                            MemoryInputMode::EditItem => {
                                if let Some(details) = &app.selected_node_details {
                                    if let Some(items) = details["items"].as_array() {
                                        if let Some(item) = items.get(app.selected_item_idx) {
                                            if let Some(item_id) = item["id"].as_str() {
                                                let itm_id = item_id.to_string();
                                                tokio::spawn(async move {
                                                    let client = reqwest::Client::new();
                                                    let mut req = client.post(format!("{}/api/memory/update", url))
                                                        .json(&serde_json::json!({ "item_id": itm_id, "content": text }));
                                                    if let Some(t) = token {
                                                        req = req.header("Cookie", format!("laruche_auth={}", t));
                                                    }
                                                    match req.send().await {
                                                        Ok(resp) if resp.status().is_success() => {
                                                            let _ = tx.send(TuiEvent::ActionFinished("Fact updated.".into())).await;
                                                        }
                                                        _ => {
                                                            let _ = tx.send(TuiEvent::Error("Update failed".into())).await;
                                                        }
                                                    }
                                                });
                                            }
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
                return;
            }
            KeyCode::Esc => {
                app.input.clear();
                app.cursor_pos = 0;
                app.memory_input_mode = MemoryInputMode::Normal;
                app.active_panel = Panel::MemoryTree;
                return;
            }
            _ => {}
        }
    }

    if app.current_screen == Screen::Missions && app.missions_input_mode != MissionsInputMode::Normal {
        match key {
            KeyCode::Enter => {
                let text = app.input.trim().to_string();
                app.input.clear();
                app.cursor_pos = 0;
                let mode = app.missions_input_mode.clone();
                app.missions_input_mode = MissionsInputMode::Normal;
                app.active_panel = Panel::MissionsList;
                if !text.is_empty() {
                    let url = app.server_url.clone();
                    let token = app.auth_token.clone();
                    if let Some(ref tx) = app.ui_tx {
                        let tx = tx.clone();
                        if mode == MissionsInputMode::CreateMission {
                            tokio::spawn(async move {
                                create_mission_bg(url, text, token, tx).await;
                            });
                        }
                    }
                }
                return;
            }
            KeyCode::Esc => {
                app.input.clear();
                app.cursor_pos = 0;
                app.missions_input_mode = MissionsInputMode::Normal;
                app.active_panel = Panel::MissionsList;
                return;
            }
            _ => {}
        }
    }

    match key {
        KeyCode::Enter => {
            // Accept autocomplete if any
            if !app.autocomplete_suggestion.is_empty() && app.input.ends_with(' ') {
                app.input.push_str(&app.autocomplete_suggestion);
                app.autocomplete_suggestion.clear();
            }
            let text = app.input.trim().to_string();
            app.input.clear();
            app.cursor_pos = 0;
            app.autocomplete_suggestion.clear();
            app.history_idx = None;
            if text.is_empty() {
                return;
            }
            // Save to history
            if app.history.last().map(|h| h != &text).unwrap_or(true) {
                app.history.push(text.clone());
                if app.history.len() > 100 {
                    app.history.remove(0);
                }
            }

            // Slash commands
            if text.starts_with('/') {
                match text.split_whitespace().next().unwrap_or("") {
                    "/quit" | "/q" => {
                        app.should_quit = true;
                        return;
                    }
                    "/help" | "/h" => {
                        app.messages.push(ChatMessage { role:"system".into(), text:"/quit /help /clear /model /tools /cwd [path] /discover /doctor /server [cmd] /export".into() });
                        return;
                    }
                    "/clear" | "/new" => {
                        // Save current conversation title to activity
                        if !app.messages.is_empty() {
                            let ts = chrono::Local::now().format("%H:%M:%S").to_string();
                            app.activity_log.push(format!(
                                "[{}] Session closed ({} msgs)",
                                ts,
                                app.messages.len()
                            ));
                        }
                        app.messages.clear();
                        app.session_id = None;
                        app.plan.clear();
                        app.status_msg = "New conversation".into();
                        return;
                    }

                    "/cwd" => {
                        let arg = text.strip_prefix("/cwd").unwrap_or("").trim();
                        if arg.is_empty() {
                            app.messages.push(ChatMessage {
                                role: "system".into(),
                                text: format!("cwd: {}", app.cwd),
                            });
                        } else if std::path::Path::new(arg).is_dir() {
                            std::env::set_current_dir(arg).ok();
                            app.cwd = arg.into();
                            app.status_msg = format!("cwd: {}", arg);
                        } else {
                            app.status_msg = format!("Not found: {}", arg);
                        }
                        return;
                    }
                    "/model" => {
                        let arg = text.strip_prefix("/model").unwrap_or("").trim();
                        if !arg.is_empty() {
                            app.model = arg.to_string();
                            app.status_msg = format!("Model: {}", arg);
                        } else {
                            app.messages.push(ChatMessage {
                                role: "system".into(),
                                text: format!("model: {}", app.model),
                            });
                        }
                        return;
                    }
                    "/discover" | "/scan" => {
                        app.messages.push(ChatMessage {
                            role: "system".into(),
                            text: "Scanning Miel network...".into(),
                        });
                        app.status_msg = "Scanning Miel...".into();
                        // Re-discover server
                        let url = discover_server().await;
                        if url.is_empty() {
                            app.messages.push(ChatMessage {
                                role: "error".into(),
                                text: "No LaRuche server found".into(),
                            });
                        } else {
                            app.server_url = url.clone();
                            app.connected = true;
                            app.tools = fetch_tools(&url).await;
                            app.model = fetch_model(&url).await;
                            app.messages.push(ChatMessage {
                                role: "system".into(),
                                text: format!("Connected: {}", url),
                            });
                        }
                        app.status_msg = if app.connected {
                            "Connected"
                        } else {
                            "Offline"
                        }
                        .into();
                        return;
                    }
                    "/doctor" | "/status" => {
                        if app.connected {
                            match reqwest::Client::new()
                                .get(format!("{}/api/doctor", app.server_url))
                                .send()
                                .await
                            {
                                Ok(r) => {
                                    if let Ok(d) = r.json::<serde_json::Value>().await {
                                        let mut info = format!(
                                            "LaRuche: {}\n",
                                            d["status"].as_str().unwrap_or("?")
                                        );
                                        if let Some(checks) = d["checks"].as_array() {
                                            for c in checks {
                                                info.push_str(&format!(
                                                    "  {} {}: {}\n",
                                                    if c["status"].as_str() == Some("ok") {
                                                        "✓"
                                                    } else {
                                                        "✗"
                                                    },
                                                    c["name"].as_str().unwrap_or("?"),
                                                    c["detail"].as_str().unwrap_or("")
                                                ));
                                            }
                                        }
                                        app.messages.push(ChatMessage {
                                            role: "system".into(),
                                            text: info,
                                        });
                                    }
                                }
                                Err(e) => app.messages.push(ChatMessage {
                                    role: "error".into(),
                                    text: format!("Doctor: {}", e),
                                }),
                            }
                        } else {
                            app.messages.push(ChatMessage {
                                role: "error".into(),
                                text: "No server connected".into(),
                            });
                        }
                        return;
                    }
                    "/server" => {
                        let arg = text.strip_prefix("/server").unwrap_or("").trim();
                        let sub_args: Vec<String> =
                            arg.split_whitespace().map(|s| s.to_string()).collect();
                        match sub_args.first().map(|s| s.as_str()).unwrap_or("help") {
                            "start" => {
                                app.messages.push(ChatMessage {
                                    role: "system".into(),
                                    text: "Starting the server...".into(),
                                });
                                app.status_msg = "Starting...".into();
                                // Try to start
                                if let Some(exe) = super::find_server_exe() {
                                    let mut cmd = std::process::Command::new(&exe);
                                    cmd.arg("--no-tui")
                                        .stdout(std::process::Stdio::null())
                                        .stderr(std::process::Stdio::null());
                                    #[cfg(windows)]
                                    {
                                        use std::os::windows::process::CommandExt;
                                        cmd.creation_flags(0x00000008);
                                    }
                                    match cmd.spawn() {
                                        Ok(c) => {
                                            app.messages.push(ChatMessage {
                                                role: "system".into(),
                                                text: format!("Server started (PID: {})", c.id()),
                                            });
                                            tokio::time::sleep(std::time::Duration::from_secs(2))
                                                .await;
                                            app.server_url = "http://127.0.0.1:8419".into();
                                            app.connected = true;
                                            app.tools = fetch_tools(&app.server_url).await;
                                            app.model = fetch_model(&app.server_url).await;
                                        }
                                        Err(e) => app.messages.push(ChatMessage {
                                            role: "error".into(),
                                            text: format!("Failed: {}", e),
                                        }),
                                    }
                                } else {
                                    app.messages.push(ChatMessage{role:"error".into(), text:"laruche-node binary not found. Run: /server install".into()});
                                }
                                app.status_msg = if app.connected {
                                    "Connected"
                                } else {
                                    "Offline"
                                }
                                .into();
                            }
                            "stop" => {
                                if cfg!(windows) {
                                    let _ = std::process::Command::new("taskkill")
                                        .args(["/F", "/IM", "laruche-node.exe"])
                                        .output();
                                } else {
                                    let _ = std::process::Command::new("pkill")
                                        .args(["-f", "laruche-node"])
                                        .output();
                                }
                                app.connected = false;
                                app.messages.push(ChatMessage {
                                    role: "system".into(),
                                    text: "Server stopped".into(),
                                });
                                app.status_msg = "Offline".into();
                            }
                            "restart" => {
                                app.messages.push(ChatMessage {
                                    role: "system".into(),
                                    text: "Restarting...".into(),
                                });
                                if cfg!(windows) {
                                    let _ = std::process::Command::new("taskkill")
                                        .args(["/F", "/IM", "laruche-node.exe"])
                                        .output();
                                } else {
                                    let _ = std::process::Command::new("pkill")
                                        .args(["-f", "laruche-node"])
                                        .output();
                                }
                                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                                if let Some(exe) = super::find_server_exe() {
                                    let mut cmd = std::process::Command::new(&exe);
                                    cmd.arg("--no-tui")
                                        .stdout(std::process::Stdio::null())
                                        .stderr(std::process::Stdio::null());
                                    #[cfg(windows)]
                                    {
                                        use std::os::windows::process::CommandExt;
                                        cmd.creation_flags(0x00000008);
                                    }
                                    let _ = cmd.spawn();
                                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                                    app.connected = super::probe_running().await;
                                }
                                app.messages.push(ChatMessage {
                                    role: "system".into(),
                                    text: if app.connected {
                                        "Server restarted"
                                    } else {
                                        "Restart failed"
                                    }
                                    .into(),
                                });
                                app.status_msg = if app.connected {
                                    "Connected"
                                } else {
                                    "Offline"
                                }
                                .into();
                            }
                            "status" => {
                                let running = super::probe_running().await;
                                app.messages.push(ChatMessage {
                                    role: "system".into(),
                                    text: if running {
                                        "Server: running"
                                    } else {
                                        "Server: stopped"
                                    }
                                    .into(),
                                });
                            }
                            "install" => {
                                app.messages.push(ChatMessage{role:"system".into(), text:"Installing the server (cargo build --release + install)...".into()});
                                if let Some(src) = super::find_source_dir() {
                                    let build = std::process::Command::new("cargo")
                                        .args(["build", "--release", "-p", "laruche-node"])
                                        .current_dir(&src)
                                        .status();
                                    match build {
                                        Ok(s) if s.success() => {
                                            app.messages.push(ChatMessage {
                                                role: "system".into(),
                                                text: "Release build OK. Installing...".into(),
                                            });
                                            let inst = std::process::Command::new("cargo")
                                                .args([
                                                    "install",
                                                    "--path",
                                                    "laruche-node",
                                                    "--force",
                                                ])
                                                .current_dir(&src)
                                                .status();
                                            match inst {
                                                Ok(s) if s.success() => {
                                                    app.messages.push(ChatMessage {
                                                        role: "system".into(),
                                                        text: "laruche-node installed successfully"
                                                            .into(),
                                                    })
                                                }
                                                _ => app.messages.push(ChatMessage {
                                                    role: "error".into(),
                                                    text: "cargo install failed".into(),
                                                }),
                                            }
                                        }
                                        _ => app.messages.push(ChatMessage {
                                            role: "error".into(),
                                            text: "Build failed. Check the Rust toolchain."
                                                .into(),
                                        }),
                                    }
                                } else {
                                    app.messages.push(ChatMessage{role:"error".into(), text:"Source directory not found. Run from the LaRuche folder.".into()});
                                }
                            }
                            "update" => {
                                app.messages.push(ChatMessage {
                                    role: "system".into(),
                                    text: "Updating (git pull + rebuild)...".into(),
                                });
                                if let Some(src) = super::find_source_dir() {
                                    let _ = std::process::Command::new("git")
                                        .args(["pull"])
                                        .current_dir(&src)
                                        .status();
                                    let build = std::process::Command::new("cargo")
                                        .args(["build", "--release", "-p", "laruche-node"])
                                        .current_dir(&src)
                                        .status();
                                    match build {
                                        Ok(s) if s.success() => {
                                            let _ = std::process::Command::new("cargo")
                                                .args([
                                                    "install",
                                                    "--path",
                                                    "laruche-node",
                                                    "--force",
                                                ])
                                                .current_dir(&src)
                                                .status();
                                            app.messages.push(ChatMessage{role:"system".into(), text:"Update complete. Run /server restart to apply.".into()});
                                        }
                                        _ => app.messages.push(ChatMessage {
                                            role: "error".into(),
                                            text: "Build failed.".into(),
                                        }),
                                    }
                                } else {
                                    app.messages.push(ChatMessage {
                                        role: "error".into(),
                                        text: "Source directory not found.".into(),
                                    });
                                }
                            }
                            "uninstall" => {
                                if cfg!(windows) {
                                    let _ = std::process::Command::new("taskkill")
                                        .args(["/F", "/IM", "laruche-node.exe"])
                                        .output();
                                } else {
                                    let _ = std::process::Command::new("pkill")
                                        .args(["-f", "laruche-node"])
                                        .output();
                                }
                                let _ = std::process::Command::new("cargo")
                                    .args(["uninstall", "laruche-node"])
                                    .status();
                                app.connected = false;
                                app.messages.push(ChatMessage {
                                    role: "system".into(),
                                    text: "laruche-node uninstalled".into(),
                                });
                            }
                            _ => {
                                app.messages.push(ChatMessage {
                                    role: "system".into(),
                                    text:
                                        "/server start|stop|restart|status|install|update|uninstall"
                                            .into(),
                                });
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
                            Ok(_) => app.messages.push(ChatMessage {
                                role: "system".into(),
                                text: format!("Exported: {}", f),
                            }),
                            Err(e) => app.messages.push(ChatMessage {
                                role: "error".into(),
                                text: format!("Error: {}", e),
                            }),
                        }
                        return;
                    }
                    "/login" => {
                        let arg = text.strip_prefix("/login").unwrap_or("").trim();
                        let parts: Vec<&str> = arg.splitn(2, ' ').collect();
                        if parts.len() < 2 || parts[0].is_empty() {
                            app.messages.push(ChatMessage {
                                role: "system".into(),
                                text: "/login <name> <password>".into(),
                            });
                            return;
                        }
                        let name = parts[0];
                        let pw = parts[1];
                        let resp = reqwest::Client::new()
                            .post(format!("{}/api/auth/login", app.server_url))
                            .json(&serde_json::json!({"display_name": name, "password": pw}))
                            .send()
                            .await;
                        match resp {
                            Ok(r) if r.status().is_success() => {
                                // Extract cookie from Set-Cookie header
                                if let Some(cookie) =
                                    r.headers().get("set-cookie").and_then(|v| v.to_str().ok())
                                {
                                    if let Some(token) = cookie
                                        .split(';')
                                        .next()
                                        .and_then(|s| s.strip_prefix("laruche_auth="))
                                    {
                                        app.auth_token = Some(token.to_string());
                                    }
                                }
                                if let Ok(data) = r.json::<serde_json::Value>().await {
                                    app.user_name =
                                        data["display_name"].as_str().map(|s| s.to_string());
                                    app.user_role = data["role"].as_str().map(|s| s.to_string());
                                }
                                app.save_config();
                                app.messages.push(ChatMessage {
                                    role: "system".into(),
                                    text: format!(
                                        "Logged in as {}",
                                        app.user_name.as_deref().unwrap_or("?")
                                    ),
                                });
                            }
                            _ => app.messages.push(ChatMessage {
                                role: "error".into(),
                                text: "Invalid credentials".into(),
                            }),
                        }
                        return;
                    }
                    "/enroll" => {
                        let arg = text.strip_prefix("/enroll").unwrap_or("").trim();
                        let parts: Vec<&str> = arg.splitn(2, ' ').collect();
                        let name = parts
                            .first()
                            .filter(|s| !s.is_empty())
                            .unwrap_or(&"CLIUser");
                        let pw = parts.get(1).unwrap_or(&"");
                        let mut body = serde_json::json!({"display_name": name});
                        if !pw.is_empty() {
                            body["password"] = serde_json::json!(pw);
                        }
                        let resp = reqwest::Client::new()
                            .post(format!("{}/api/auth/enroll", app.server_url))
                            .json(&body)
                            .send()
                            .await;
                        match resp {
                            Ok(r) if r.status().is_success() => {
                                if let Some(cookie) =
                                    r.headers().get("set-cookie").and_then(|v| v.to_str().ok())
                                {
                                    if let Some(token) = cookie
                                        .split(';')
                                        .next()
                                        .and_then(|s| s.strip_prefix("laruche_auth="))
                                    {
                                        app.auth_token = Some(token.to_string());
                                    }
                                }
                                if let Ok(data) = r.json::<serde_json::Value>().await {
                                    app.user_name =
                                        data["display_name"].as_str().map(|s| s.to_string());
                                    app.user_role = data["role"].as_str().map(|s| s.to_string());
                                }
                                app.save_config();
                                app.messages.push(ChatMessage {
                                    role: "system".into(),
                                    text: format!(
                                        "Account created: {} ({})",
                                        app.user_name.as_deref().unwrap_or("?"),
                                        app.user_role.as_deref().unwrap_or("user")
                                    ),
                                });
                            }
                            _ => app.messages.push(ChatMessage {
                                role: "error".into(),
                                text: "Enrollment error".into(),
                            }),
                        }
                        return;
                    }
                    "/logout" => {
                        app.auth_token = None;
                        app.user_name = None;
                        app.user_role = None;
                        app.save_config();
                        app.messages.push(ChatMessage {
                            role: "system".into(),
                            text: "Logged out".into(),
                        });
                        return;
                    }
                    "/whoami" => {
                        let info = match (&app.user_name, &app.user_role) {
                            (Some(n), Some(r)) => format!("{} ({})", n, r),
                            _ => {
                                "Not authenticated. /login <name> <pwd> or /enroll <name> [pwd]".into()
                            }
                        };
                        app.messages.push(ChatMessage {
                            role: "system".into(),
                            text: info,
                        });
                        return;
                    }
                    _ => {
                        app.status_msg = format!("? {} - /help", text);
                        return;
                    }
                }
            }

            if !app.connected {
                app.messages.push(ChatMessage {
                    role: "error".into(),
                    text: "No LaRuche server connected!".into(),
                });
                return;
            }

            // Show user message immediately + scroll
            app.messages.push(ChatMessage {
                role: "user".into(),
                text: text.clone(),
            });
            app.is_streaming = true;
            app.status_msg = "Thinking...".into();
            let timestamp = chrono::Local::now().format("%H:%M:%S").to_string();
            app.activity_log.push(format!(
                "[{}] Prompt: {}",
                timestamp,
                text.chars().take(50).collect::<String>()
            ));
            // Auto-scroll to show the new message
            let total = app
                .messages
                .iter()
                .map(|m| m.text.lines().count() + 1)
                .sum::<usize>();
            app.chat_scroll = total.saturating_sub(12) as u16;

            // Spawn the WebSocket streaming task (falls back to HTTP internally)
            let (tx_evt, rx_evt) = tokio::sync::mpsc::channel::<TuiEvent>(64);
            app.event_rx = Some(rx_evt);
            app.streaming_response.clear();
            let url = app.server_url.clone();
            let model = app.model.clone();
            let token = app.auth_token.clone();
            let session = app.session_id.clone();
            app.stream_task = Some(tokio::spawn(async move {
                stream_via_websocket(url, text, model, token, session, tx_evt).await;
            }));
        }
        KeyCode::Char(c) => {
            // Insert at char position (not byte position)
            let byte_pos = app
                .input
                .char_indices()
                .nth(app.cursor_pos)
                .map(|(i, _)| i)
                .unwrap_or(app.input.len());
            app.input.insert(byte_pos, c);
            app.cursor_pos += 1;
            app.history_idx = None;
            update_autocomplete(app);
        }
        KeyCode::Backspace => {
            if app.cursor_pos > 0 {
                app.cursor_pos -= 1;
                let byte_pos = app
                    .input
                    .char_indices()
                    .nth(app.cursor_pos)
                    .map(|(i, _)| i)
                    .unwrap_or(0);
                let next_byte = app
                    .input
                    .char_indices()
                    .nth(app.cursor_pos + 1)
                    .map(|(i, _)| i)
                    .unwrap_or(app.input.len());
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
            if app.history.is_empty() {
                return;
            }
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
            if let Some(idx) = app.history_idx {
                if idx + 1 < app.history.len() {
                    app.history_idx = Some(idx + 1);
                    app.input = app.history[idx + 1].clone();
                } else {
                    app.history_idx = None;
                    app.input = app.history_draft.clone();
                }
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


/// Autocomplete suggestions for slash commands.
fn update_autocomplete(app: &mut App) {
    app.autocomplete_suggestion.clear();
    let input = &app.input;
    if input.is_empty() {
        return;
    }

    // Slash command completions
    let commands = [
        "/help",
        "/clear",
        "/quit",
        "/model",
        "/tools",
        "/cwd",
        "/discover",
        "/doctor",
        "/server start",
        "/server stop",
        "/server restart",
        "/server status",
        "/server install",
        "/server uninstall",
        "/server update",
        "/export",
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
    let input_lines = app.input.lines().count().clamp(1, 10) as u16 + 2;
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6), // Header
            Constraint::Min(10),   // Chat
            Constraint::Length(input_lines), // Input
            Constraint::Length(1), // Footer/Status
        ])
        .split(f.area());

    draw_header(f, chunks[0], app);
    match app.current_screen {
        Screen::Chat => {
            // Split horizontally: left sidebar for agent activity, right for chat
            let h_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Length(36), // Agent sidebar
                    Constraint::Min(20),   // Chat messages
                ])
                .split(chunks[1]);

            draw_agent_sidebar(f, h_chunks[0], app);
            draw_chat(f, h_chunks[1], app);
        }
        Screen::Memory => {
            draw_memory(f, chunks[1], app);
        }
        Screen::Missions => {
            draw_missions(f, chunks[1], app);
        }
    }
    draw_input(f, chunks[2], app);
    draw_status(f, chunks[3], app);
}

fn draw_memory(f: &mut Frame, area: Rect, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(38), // Tree
            Constraint::Min(0),      // Details
        ])
        .split(area);

    let is_tree_focused = app.active_panel == Panel::MemoryTree && app.memory_active_pane == MemoryPane::Tree;
    let is_details_focused = app.active_panel == Panel::MemoryTree && app.memory_active_pane == MemoryPane::Details;

    // 1. Build & Draw Tree
    let mut draw_items = Vec::new();
    let mut visited = std::collections::HashSet::new();
    build_draw_tree(&app.memory_nodes, None, 0, &mut visited, &mut draw_items);

    // Keep selected index in bounds
    if !draw_items.is_empty() && app.selected_node_idx >= draw_items.len() {
        app.selected_node_idx = draw_items.len() - 1;
    }

    // Scroll calculations
    let tree_visible_height = chunks[0].height.saturating_sub(2) as usize;
    if !draw_items.is_empty() {
        if app.selected_node_idx >= app.memory_tree_scroll + tree_visible_height {
            app.memory_tree_scroll = app.selected_node_idx - tree_visible_height + 1;
        } else if app.selected_node_idx < app.memory_tree_scroll {
            app.memory_tree_scroll = app.selected_node_idx;
        }
    }

    let mut list_items = Vec::new();
    for (idx, item) in draw_items.iter().enumerate().skip(app.memory_tree_scroll).take(tree_visible_height) {
        let style = if idx == app.selected_node_idx {
            Style::default().fg(Color::Black).bg(AMBER).add_modifier(Modifier::BOLD)
        } else if item.is_protected {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::White)
        };

        let indent = "  ".repeat(item.depth);
        let prefix = if item.depth > 0 { "└─ " } else { "● " };
        let lock = if item.is_protected { " 🔒" } else { "" };
        list_items.push(ListItem::new(Line::from(Span::styled(
            format!("{}{}{}{}", indent, prefix, item.label, lock),
            style,
        ))));
    }

    if list_items.is_empty() {
        list_items.push(ListItem::new(Line::from(Span::styled(
            " No node loaded",
            Style::default().fg(TEXT_DIM).add_modifier(Modifier::ITALIC),
        ))));
    }

    let b_tree = Block::default()
        .title(" Cognitive Map ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if is_tree_focused { AMBER } else { BORDER }))
        .style(Style::default().bg(BG));
    f.render_widget(List::new(list_items).block(b_tree), chunks[0]);

    // 2. Draw Details
    let b_details = Block::default()
        .title(" Node Details ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if is_details_focused { AMBER } else { BORDER }))
        .style(Style::default().bg(BG));

    if let Some(ref details) = app.selected_node_details {
        // Split details area vertically: Metadata (height 5) and Items list (Min(0))
        let details_area = b_details.inner(chunks[1]);
        let details_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(5), // Metadata
                Constraint::Min(0),    // Facts
            ])
            .split(details_area);

        // Metadata
        let label = details["label"].as_str().unwrap_or("?");
        let id = details["id"].as_str().unwrap_or("?");
        let desc = details["one_liner"].as_str().unwrap_or("");
        let is_prot = details["protected"].as_bool().unwrap_or(false);

        let prot_status = if is_prot {
            Span::styled(" [PROTECTED SYSTEM 🔒]", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        } else {
            Span::styled(" [EDITABLE ✏️]", Style::default().fg(Color::Green))
        };

        let meta_lines = vec![
            Line::from(vec![
                Span::styled("Label : ", Style::default().fg(TEXT_DIM)),
                Span::styled(label, Style::default().fg(AMBER).add_modifier(Modifier::BOLD)),
                prot_status,
            ]),
            Line::from(vec![
                Span::styled("ID    : ", Style::default().fg(TEXT_DIM)),
                Span::styled(id, Style::default().fg(Color::Cyan)),
            ]),
            Line::from(vec![
                Span::styled("Desc  : ", Style::default().fg(TEXT_DIM)),
                Span::styled(desc, Style::default().fg(Color::White)),
            ]),
            Line::from(Span::styled("─".repeat(details_chunks[0].width as usize), Style::default().fg(BORDER))),
        ];
        f.render_widget(Paragraph::new(meta_lines), details_chunks[0]);

        // Facts List
        let mut fact_items = Vec::new();
        if let Some(items) = details["items"].as_array() {
            // Keep selected item index in bounds
            if !items.is_empty() && app.selected_item_idx >= items.len() {
                app.selected_item_idx = items.len() - 1;
            }

            let facts_visible_height = details_chunks[1].height.saturating_sub(1) as usize; // reserve last line for help
            if !items.is_empty() {
                if app.selected_item_idx >= app.memory_details_scroll + facts_visible_height {
                    app.memory_details_scroll = app.selected_item_idx - facts_visible_height + 1;
                } else if app.selected_item_idx < app.memory_details_scroll {
                    app.memory_details_scroll = app.selected_item_idx;
                }
            }

            for (idx, item) in items.iter().enumerate().skip(app.memory_details_scroll).take(facts_visible_height) {
                let content = item["content"].as_str().unwrap_or("");
                let source = item["source"].as_str().unwrap_or("system");
                let is_sel = idx == app.selected_item_idx && is_details_focused;

                let style = if is_sel {
                    Style::default().fg(Color::Black).bg(AMBER).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };

                let src_span = if is_sel {
                    Span::styled(format!(" (from {})", source), Style::default().fg(Color::Black).add_modifier(Modifier::ITALIC))
                } else {
                    Span::styled(format!(" (from {})", source), Style::default().fg(TEXT_DIM).add_modifier(Modifier::ITALIC))
                };

                fact_items.push(ListItem::new(Line::from(vec![
                    Span::styled(format!("  {} · ", idx + 1), if is_sel { Style::default().fg(Color::Black).add_modifier(Modifier::BOLD) } else { Style::default().fg(AMBER) }),
                    Span::styled(content, style),
                    src_span,
                ])));
            }
        }

        if fact_items.is_empty() {
            fact_items.push(ListItem::new(Line::from(Span::styled(
                " No fact stored in this node. Press 'a' to add one.",
                Style::default().fg(TEXT_DIM).add_modifier(Modifier::ITALIC),
            ))));
        }

        // Help bar at the bottom
        let help_text = if is_tree_focused {
            " [Tab] Details  [n] New sub-node  [e] Rename  [d] Delete  [a] Add Fact "
        } else if is_details_focused {
            " [Tab] Tree  [a] Add Fact  [e] Edit Fact  [d] Delete Fact "
        } else {
            " Press Tab to activate this panel "
        };
        let help_line = Line::from(Span::styled(help_text, Style::default().fg(TEXT_DIM).add_modifier(Modifier::ITALIC)));

        let details_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),
                Constraint::Length(1), // Help text
            ])
            .split(details_chunks[1]);

        f.render_widget(List::new(fact_items), details_layout[0]);
        f.render_widget(Paragraph::new(help_line), details_layout[1]);
        f.render_widget(Block::default().borders(Borders::NONE).style(Style::default().bg(BG)), chunks[1]);
        // Render surrounding border block
        f.render_widget(b_details, chunks[1]);
    } else {
        let empty_text = vec![
            Line::from(""),
            Line::from(Span::styled("  Please select a node in the cognitive map on the left.", Style::default().fg(TEXT_DIM))),
        ];
        f.render_widget(Paragraph::new(empty_text).block(b_details), chunks[1]);
    }
}

fn draw_missions(f: &mut Frame, area: Rect, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(38), // List
            Constraint::Min(0),      // Dossier
        ])
        .split(area);

    let is_list_focused = app.active_panel == Panel::MissionsList && app.missions_active_pane == MissionsPane::List;
    let is_dossier_focused = app.active_panel == Panel::MissionsList && app.missions_active_pane == MissionsPane::Dossier;

    // 1. Draw List
    let b_list = Block::default()
        .title(" Active Missions ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if is_list_focused { AMBER } else { BORDER }))
        .style(Style::default().bg(BG));

    let mut list_items = Vec::new();
    for (idx, m) in app.missions.iter().enumerate() {
        let objective = m["objective"].as_str().unwrap_or("?");
        let status = m["status"].as_str().unwrap_or("active");
        let runs = m["iterations"].as_u64().unwrap_or(0);
        let cadence = m["cadence"].as_str().unwrap_or("manual");

        let is_sel = idx == app.selected_mission_idx;
        let style = if is_sel {
            Style::default().fg(Color::Black).bg(AMBER).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let status_badge = match status {
            "active" => Span::styled(" [ACTIVE]", Style::default().fg(Color::Green)),
            "paused" => Span::styled(" [PAUSED]", Style::default().fg(Color::Yellow)),
            _ => Span::styled(" [DONE]  ", Style::default().fg(TEXT_DIM)),
        };

        let runs_text = format!(" ({} iterations, {})", runs, cadence);
        let runs_span = if is_sel {
            Span::styled(runs_text, Style::default().fg(Color::Black).add_modifier(Modifier::ITALIC))
        } else {
            Span::styled(runs_text, Style::default().fg(TEXT_DIM).add_modifier(Modifier::ITALIC))
        };

        list_items.push(ListItem::new(Line::from(vec![
            status_badge,
            Span::styled(objective, style),
            runs_span,
        ])));
    }

    if list_items.is_empty() {
        list_items.push(ListItem::new(Line::from(Span::styled(
            " No mission created. Press 'c' to start one.",
            Style::default().fg(TEXT_DIM).add_modifier(Modifier::ITALIC),
        ))));
    }

    f.render_widget(List::new(list_items).block(b_list), chunks[0]);

    // 2. Draw Dossier
    let b_dossier = Block::default()
        .title(" Mission Dossier ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if is_dossier_focused { AMBER } else { BORDER }))
        .style(Style::default().bg(BG));

    if let Some(ref dossier) = app.selected_mission_dossier {
        let md_lines = markdown::parse_markdown(dossier);

        let dossier_area = b_dossier.inner(chunks[1]);
        let dossier_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),
                Constraint::Length(1), // help text
            ])
            .split(dossier_area);

        // Help bar
        let help_text = if is_list_focused {
            " [Tab] Dossier  [c] Create Mission  [r] Run Turn  [p] Pause/Resume  [d] Delete  [k] Decompose "
        } else {
            " [Tab] List  [Up/Down arrows] Scroll the dossier "
        };
        let help_line = Line::from(Span::styled(help_text, Style::default().fg(TEXT_DIM).add_modifier(Modifier::ITALIC)));

        f.render_widget(
            Paragraph::new(Text::from(md_lines))
                .wrap(Wrap { trim: false })
                .scroll((app.missions_dossier_scroll as u16, 0)),
            dossier_layout[0],
        );
        f.render_widget(Paragraph::new(help_line), dossier_layout[1]);
        f.render_widget(Block::default().borders(Borders::NONE).style(Style::default().bg(BG)), chunks[1]);
        f.render_widget(b_dossier, chunks[1]);
    } else {
        let empty_text = vec![
            Line::from(""),
            Line::from(Span::styled("  No mission dossier loaded, or select a mission.", Style::default().fg(TEXT_DIM))),
        ];
        f.render_widget(Paragraph::new(empty_text).block(b_dossier), chunks[1]);
    }
}

fn draw_header(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(45), // ASCII Logo + client version
            Constraint::Length(36), // Tabs
            Constraint::Min(0),      // Connection and Model
        ])
        .split(area);

    // Left chunk (ASCII Art + Subtitle)
    let logo_lines = vec![
        Line::from(Span::styled(r#"    __       ___            __        "#, Style::default().fg(AMBER).add_modifier(Modifier::BOLD))),
        Line::from(Span::styled(r#"   / /  ___ / _ \__ __  ___ / /  ___  "#, Style::default().fg(AMBER).add_modifier(Modifier::BOLD))),
        Line::from(Span::styled(r#"  / /__/ _ `/ , _/ // // __|/ _ \/ -_) "#, Style::default().fg(AMBER).add_modifier(Modifier::BOLD))),
        Line::from(Span::styled(r#" /____/\_,_/_/|_|\_,_/ \___/_//_/\__/  "#, Style::default().fg(AMBER).add_modifier(Modifier::BOLD))),
        Line::from(Span::styled(format!("   🐝 CLI Client v{}", env!("CARGO_PKG_VERSION")), Style::default().fg(TEXT_DIM))),
    ];

    let b_art = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(BORDER))
        .style(Style::default().bg(BG_PANEL));
    f.render_widget(Paragraph::new(logo_lines).block(b_art), chunks[0]);

    // Middle chunk (Tabs)
    let tab_chat = if app.current_screen == Screen::Chat {
        Span::styled(" Chat ", Style::default().fg(Color::Black).bg(AMBER).add_modifier(Modifier::BOLD))
    } else {
        Span::styled(" Chat ", Style::default().fg(TEXT_DIM))
    };
    let tab_memory = if app.current_screen == Screen::Memory {
        Span::styled(" Memory ", Style::default().fg(Color::Black).bg(AMBER).add_modifier(Modifier::BOLD))
    } else {
        Span::styled(" Memory ", Style::default().fg(TEXT_DIM))
    };
    let tab_missions = if app.current_screen == Screen::Missions {
        Span::styled(" Missions ", Style::default().fg(Color::Black).bg(AMBER).add_modifier(Modifier::BOLD))
    } else {
        Span::styled(" Missions ", Style::default().fg(TEXT_DIM))
    };

    let tab_line = vec![
        Line::from(""), // spacer
        Line::from(""), // spacer
        Line::from(""), // spacer
        Line::from(""), // spacer
        Line::from(vec![
            tab_chat,
            Span::raw("  "),
            tab_memory,
            Span::raw("  "),
            tab_missions,
        ]),
    ];

    let b_tabs = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(BORDER))
        .style(Style::default().bg(BG_PANEL));
    f.render_widget(Paragraph::new(tab_line).block(b_tabs), chunks[1]);

    // Right chunk (Connection & Model)
    let conn = if app.connected {
        Span::styled(" Connected ", Style::default().fg(Color::Green))
    } else {
        Span::styled(" Offline ", Style::default().fg(Color::Red))
    };

    let right_line = vec![
        Line::from(""), // spacer
        Line::from(""), // spacer
        Line::from(""), // spacer
        Line::from(""), // spacer
        Line::from(vec![
            conn,
            Span::styled("  │  ", Style::default().fg(BORDER)),
            Span::styled(format!("Model: {}  ", app.model), Style::default().fg(Color::Cyan)),
        ]),
    ];

    let b_right = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(BORDER))
        .style(Style::default().bg(BG_PANEL));
    f.render_widget(
        Paragraph::new(right_line).block(b_right).alignment(ratatui::layout::Alignment::Right),
        chunks[2],
    );
}

/// Basic markdown rendering for terminal: **bold**, *italic*, `code`, ### headers, - lists
fn render_md_line(line: &str) -> Line<'static> {
    let trimmed = line.trim();

    // Headers
    if trimmed.starts_with("### ") {
        return Line::from(Span::styled(
            format!("  {}", trimmed.strip_prefix("### ").unwrap_or(trimmed)),
            Style::default().fg(AMBER).add_modifier(Modifier::BOLD),
        ));
    }
    if trimmed.starts_with("## ") {
        return Line::from(Span::styled(
            format!("  {}", trimmed.strip_prefix("## ").unwrap_or(trimmed)),
            Style::default().fg(AMBER).add_modifier(Modifier::BOLD),
        ));
    }
    if trimmed.starts_with("# ") {
        return Line::from(Span::styled(
            format!("  {}", trimmed.strip_prefix("# ").unwrap_or(trimmed)),
            Style::default().fg(AMBER).add_modifier(Modifier::BOLD),
        ));
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
                    Span::styled(
                        format!("  {}. ", &trimmed[..dot_pos]),
                        Style::default().fg(AMBER),
                    ),
                    Span::styled(
                        trimmed[dot_pos + 2..].to_string(),
                        Style::default().fg(Color::White),
                    ),
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
            if !current.is_empty() {
                spans.push(Span::styled(
                    current.clone(),
                    Style::default().fg(Color::White),
                ));
                current.clear();
            }
            let mut code = String::new();
            while let Some(&nc) = chars.peek() {
                if nc == '`' {
                    chars.next();
                    break;
                }
                code.push(chars.next().unwrap());
            }
            spans.push(Span::styled(code, Style::default().fg(Color::Cyan)));
        } else if c == '*' && chars.peek() == Some(&'*') {
            chars.next(); // consume second *
            if !current.is_empty() {
                spans.push(Span::styled(
                    current.clone(),
                    Style::default().fg(Color::White),
                ));
                current.clear();
            }
            let mut bold = String::new();
            while let Some(nc) = chars.next() {
                if nc == '*' && chars.peek() == Some(&'*') {
                    chars.next();
                    break;
                }
                bold.push(nc);
            }
            spans.push(Span::styled(
                bold,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ));
        } else if c == '*' {
            if !current.is_empty() {
                spans.push(Span::styled(
                    current.clone(),
                    Style::default().fg(Color::White),
                ));
                current.clear();
            }
            let mut italic = String::new();
            for nc in chars.by_ref() {
                if nc == '*' {
                    break;
                }
                italic.push(nc);
            }
            spans.push(Span::styled(
                italic,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::ITALIC),
            ));
        } else {
            current.push(c);
        }
    }
    if !current.is_empty() {
        spans.push(Span::styled(current, Style::default().fg(Color::White)));
    }

    Line::from(spans)
}

fn draw_agent_sidebar(f: &mut Frame, area: Rect, app: &App) {
    let mut lines: Vec<Line> = Vec::new();

    // ── Plan section ──
    if !app.plan.is_empty() {
        lines.push(Line::from(Span::styled(
            " ─── Plan ───",
            Style::default().fg(AMBER).add_modifier(Modifier::BOLD),
        )));
        for (task, status) in &app.plan {
            let (icon, color) = match status.as_str() {
                "done" => ("✓", Color::Green),
                "in_progress" | "running" => ("⟳", AMBER),
                _ => ("○", TEXT_DIM),
            };
            let short_task: String = task.chars().take(28).collect();
            lines.push(Line::from(vec![
                Span::styled(format!(" {} ", icon), Style::default().fg(color)),
                Span::styled(short_task, Style::default().fg(Color::White)),
            ]));
        }
        lines.push(Line::from(""));
    }

    // ── Activity log section ──
    lines.push(Line::from(Span::styled(
        " ─── Activity ───",
        Style::default().fg(AMBER).add_modifier(Modifier::BOLD),
    )));

    if app.activity_log.is_empty() && !app.is_streaming {
        lines.push(Line::from(Span::styled(
            " Waiting...",
            Style::default().fg(TEXT_DIM).add_modifier(Modifier::ITALIC),
        )));
    } else {
        // Show the last N entries that fit
        let max_entries = (area.height as usize).saturating_sub(6);
        let start = app.activity_log.len().saturating_sub(max_entries);
        for entry in &app.activity_log[start..] {
            let (icon, color) = if entry.starts_with("[OK]") {
                ("✓", Color::Green)
            } else if entry.starts_with("[ERR]") {
                ("✗", Color::Red)
            } else if entry.starts_with("[TOOL]") {
                ("⚙", Color::Blue)
            } else if entry.starts_with("[THINK]") {
                ("💭", Color::Magenta)
            } else {
                ("·", TEXT_DIM)
            };

            // Extract the text after the tag and timestamp
            let display_text = entry
                .find("] ")
                .and_then(|i| {
                    let rest = &entry[i + 2..];
                    // Skip the second bracket (timestamp)
                    rest.find("] ").map(|j| &rest[j + 2..]).or(Some(rest))
                })
                .unwrap_or(entry);

            // Truncate for sidebar width
            let short: String = display_text.chars().take(30).collect();
            lines.push(Line::from(vec![
                Span::styled(format!(" {} ", icon), Style::default().fg(color)),
                Span::styled(short, Style::default().fg(TEXT_DIM)),
            ]));
        }
    }

    // ── Streaming indicator ──
    if app.is_streaming {
        lines.push(Line::from(""));
        let spinner_frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        let idx = ((chrono::Utc::now().timestamp_millis() / 80) % spinner_frames.len() as i64) as usize;
        let stream_chars = app.streaming_response.chars().count();
        if stream_chars > 0 {
            lines.push(Line::from(vec![
                Span::styled(
                    format!(" {} Writing... ", spinner_frames[idx]),
                    Style::default().fg(AMBER).add_modifier(Modifier::BOLD),
                ),
            ]));
            lines.push(Line::from(Span::styled(
                format!(" {} chars", stream_chars),
                Style::default().fg(TEXT_DIM),
            )));
        } else {
            lines.push(Line::from(vec![
                Span::styled(
                    format!(" {} Thinking...", spinner_frames[idx]),
                    Style::default().fg(AMBER).add_modifier(Modifier::BOLD),
                ),
            ]));
        }
    }

    let title = Line::from(vec![
        Span::styled(" 🐝 Agent ", Style::default().fg(AMBER).add_modifier(Modifier::BOLD)),
    ]);

    f.render_widget(
        Paragraph::new(Text::from(lines))
            .wrap(Wrap { trim: false })
            .scroll((app.sidebar_scroll, 0))
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(BORDER))
                    .style(Style::default().bg(BG_PANEL)),
            ),
        area,
    );
}

fn draw_chat(f: &mut Frame, area: Rect, app: &App) {
    let is_active = app.active_panel == Panel::Chat;

    let title = Line::from(vec![
        Span::styled(
            " Chat ",
            Style::default().fg(AMBER).add_modifier(Modifier::BOLD),
        ),
    ]);

    let mut lines: Vec<Line> = Vec::new();

    // Only show user, assistant, system, error messages: tools go to sidebar
    for msg in &app.messages {
        match msg.role.as_str() {
            "user" => {
                lines.push(Line::from(vec![
                    Span::styled(
                        "  ❯ ",
                        Style::default().fg(AMBER).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(&msg.text, Style::default().fg(AMBER)),
                ]));
                lines.push(Line::from(""));
            }
            "assistant" => {
                let mut md_lines = markdown::parse_markdown(&msg.text);
                lines.append(&mut md_lines);
                lines.push(Line::from(""));
            }
            "error" => {
                lines.push(Line::from(Span::styled(
                    format!("  ✗ {}", msg.text),
                    Style::default().fg(Color::Red),
                )));
                lines.push(Line::from(""));
            }
            "system" => {
                lines.push(Line::from(Span::styled(
                    format!("  {}", msg.text),
                    Style::default().fg(TEXT_DIM).add_modifier(Modifier::ITALIC),
                )));
                lines.push(Line::from(""));
            }
            // "tool" messages are now shown in the agent sidebar
            _ => {}
        }
    }

    // Streaming response with blinking cursor
    if app.is_streaming {
        let cleaned = markdown::clean_agent_text(&app.streaming_response);
        if !cleaned.is_empty() {
            let total_lines = cleaned.lines().count();
            for (i, l) in cleaned.lines().enumerate() {
                if i == total_lines - 1 {
                    let cursor_char = if (chrono::Utc::now().timestamp_millis() / 250) % 2 == 0 {
                        "▍"
                    } else {
                        " "
                    };
                    let mut line_spans = render_md_line(l).spans;
                    line_spans.push(Span::styled(cursor_char, Style::default().fg(AMBER)));
                    lines.push(Line::from(line_spans));
                } else {
                    lines.push(render_md_line(l));
                }
            }
            lines.push(Line::from(""));
        } else {
            // Show reflection animation (spinner)
            let spinner_frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
            let idx = ((chrono::Utc::now().timestamp_millis() / 80) % spinner_frames.len() as i64) as usize;
            lines.push(Line::from(vec![
                Span::styled(format!("  {} Thinking...", spinner_frames[idx]), Style::default().fg(AMBER).add_modifier(Modifier::BOLD)),
            ]));
            lines.push(Line::from(""));
        }
    }

    f.render_widget(
        Paragraph::new(Text::from(lines))
            .wrap(Wrap { trim: false })
            .scroll((app.chat_scroll, 0))
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(if is_active { AMBER } else { BORDER }))
                    .style(Style::default().bg(BG)),
            ),
        area,
    );
}

fn draw_input(f: &mut Frame, area: Rect, app: &App) {
    let is_active = app.active_panel == Panel::Input;

    let content = if app.input.is_empty() && !is_active {
        Line::from(Span::styled(
            "Tab to type...",
            Style::default().fg(TEXT_DIM),
        ))
    } else {
        // Show input + autocomplete suggestion in dim
        let mut spans = vec![Span::styled(&app.input, Style::default().fg(Color::White))];
        if !app.autocomplete_suggestion.is_empty() && is_active {
            spans.push(Span::styled(
                &app.autocomplete_suggestion,
                Style::default().fg(Color::Rgb(60, 60, 65)),
            ));
            spans.push(Span::styled(
                " →",
                Style::default().fg(Color::Rgb(60, 60, 65)),
            ));
        }
        Line::from(spans)
    };

    let title_text = if app.current_screen == Screen::Memory && app.memory_input_mode != MemoryInputMode::Normal {
        match app.memory_input_mode {
            MemoryInputMode::CreateNode => " New node (Enter to confirm, Esc to cancel) ",
            MemoryInputMode::AddItem => " New fact (Enter to confirm, Esc to cancel) ",
            MemoryInputMode::EditNode => " Rename node (Enter to confirm, Esc to cancel) ",
            MemoryInputMode::EditItem => " Edit fact (Enter to confirm, Esc to cancel) ",
            _ => " Prompt ",
        }
    } else if app.current_screen == Screen::Missions && app.missions_input_mode != MissionsInputMode::Normal {
        match app.missions_input_mode {
            MissionsInputMode::CreateMission => " New mission (Enter to confirm, Esc to cancel) ",
            _ => " Prompt ",
        }
    } else {
        " Prompt "
    };

    let block_title = if is_active {
        Span::styled(format!(" {} > ", title_text), Style::default().fg(AMBER).add_modifier(Modifier::BOLD))
    } else {
        Span::styled(format!(" {} ", title_text), Style::default().fg(TEXT_DIM))
    };

    f.render_widget(
        Paragraph::new(content).block(
            Block::default()
                .title(block_title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(if is_active { AMBER } else { BORDER }))
                .style(Style::default().bg(BG_PANEL)),
        ),
        area,
    );

    // Cursor: x + 1 (left border) + cursor_pos
    if is_active {
        f.set_cursor_position((area.x + 1 + app.cursor_pos as u16, area.y + 1));
    }
}

fn draw_status(f: &mut Frame, area: Rect, app: &App) {
    let s = Line::from(vec![
        Span::styled(" cwd: ", Style::default().fg(TEXT_DIM)),
        Span::styled(
            app.cwd
                .chars()
                .rev()
                .take(25)
                .collect::<String>()
                .chars()
                .rev()
                .collect::<String>(),
            Style::default().fg(AMBER),
        ),
        Span::styled("  │  ", Style::default().fg(BORDER)),
        Span::styled(&app.status_msg, Style::default().fg(TEXT_DIM)),
        Span::styled("  │  ", Style::default().fg(BORDER)),
        Span::styled(
            "Tab:panel  ⇅:scroll  Ctrl+C:quit  Ctrl+=/−:zoom",
            Style::default().fg(TEXT_DIM),
        ),
    ]);
    f.render_widget(Paragraph::new(s).style(Style::default().bg(BG_PANEL)), area);
}
