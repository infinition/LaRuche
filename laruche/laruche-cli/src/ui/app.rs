pub use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame, Terminal,
};
use tokio::sync::mpsc;

/// Events sent from the WebSocket background task to the TUI main loop.
#[derive(Debug, Clone)]
enum TuiEvent {
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
enum SidebarPanel {
    Tools,
    Models,
    Sessions,
    Plan,
}

#[derive(PartialEq, Clone)]
enum ChatView {
    Messages,
    Activity,
    Status,
}

#[derive(PartialEq, Clone)]
enum Panel {
    Input,
    Chat,
    Sidebar,
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
                .get(format!("{}/api/auth/me", &server_url))
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

        let app = App {
            input: String::new(),
            cursor_pos: 0,
            messages: vec![ChatMessage {
                role: "system".into(),
                text: welcome_msg,
            }],
            chat_scroll: 0,
            tools,
            plan: vec![],
            active_panel: Panel::Input,
            sidebar_panel: SidebarPanel::Tools,
            model,
            server_url,
            cwd,
            tokens: 0,
            status_msg: if connected {
                "Connected".into()
            } else {
                "Disconnected".into()
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
            available_models: Vec::new(),
            model_cursor: 0,
            event_rx: None,
            streaming_response: String::new(),
            auth_token,
            user_name,
            user_role,
        };
        app
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
}

fn dirs_config_path() -> String {
    if let Ok(home) = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")) {
        format!("{}/.laruche/cli-config.json", home)
    } else {
        "cli-config.json".to_string()
    }
}
