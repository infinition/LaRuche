//! Server TUI: fixed-layout terminal UI with scrolling logs.
//!
//! Provides a Ratatui-based interface for laruche-node with:
//! - Header bar: node name, IP, port, uptime
//! - Scrolling log panel (main area)
//! - Right sidebar: live stats (CPU, RAM, GPU, peers, sessions, queue)
//! - Footer: key hints

use crate::AppState;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Gauge, Paragraph, Wrap},
    Frame, Terminal,
};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing_subscriber::Layer;

// ─── Log buffer shared between tracing layer and TUI ────────────────────────

const MAX_LOG_LINES: usize = 2000;

#[derive(Clone)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: tracing::Level,
    pub message: String,
}

/// Shared log buffer for the TUI.
pub struct TuiLogBuffer {
    tx: mpsc::UnboundedSender<LogEntry>,
}

impl TuiLogBuffer {
    pub fn new() -> (Self, mpsc::UnboundedReceiver<LogEntry>) {
        let (tx, rx) = mpsc::unbounded_channel();
        (Self { tx }, rx)
    }

    pub fn sender(&self) -> mpsc::UnboundedSender<LogEntry> {
        self.tx.clone()
    }
}

// ─── Custom tracing Layer ───────────────────────────────────────────────────

pub struct TuiTracingLayer {
    tx: mpsc::UnboundedSender<LogEntry>,
}

impl TuiTracingLayer {
    pub fn new(tx: mpsc::UnboundedSender<LogEntry>) -> Self {
        Self { tx }
    }
}

impl<S> Layer<S> for TuiTracingLayer
where
    S: tracing::Subscriber,
{
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let mut visitor = MessageVisitor(String::new());
        event.record(&mut visitor);
        let now = chrono::Local::now().format("%H:%M:%S").to_string();
        let _ = self.tx.send(LogEntry {
            timestamp: now,
            level: *event.metadata().level(),
            message: visitor.0,
        });
    }
}

struct MessageVisitor(String);

impl tracing::field::Visit for MessageVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.0 = format!("{:?}", value);
        } else if !self.0.is_empty() {
            self.0.push_str(&format!(" {}={:?}", field.name(), value));
        } else {
            self.0 = format!("{}={:?}", field.name(), value);
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.0 = value.to_string();
        } else if !self.0.is_empty() {
            self.0.push_str(&format!(" {}={}", field.name(), value));
        } else {
            self.0 = format!("{}={}", field.name(), value);
        }
    }
}

// ─── TUI application state ─────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
enum View {
    Overview,
    Logs,
    Activity,
    Sessions,
    Swarm,
}

impl View {
    const ALL: [View; 5] = [
        View::Overview,
        View::Logs,
        View::Activity,
        View::Sessions,
        View::Swarm,
    ];
    fn title(self) -> &'static str {
        match self {
            View::Overview => "Overview",
            View::Logs => "Logs",
            View::Activity => "Activity",
            View::Sessions => "Sessions",
            View::Swarm => "Swarm",
        }
    }
    fn index(self) -> usize {
        View::ALL.iter().position(|&v| v == self).unwrap_or(0)
    }
}

/// One slash command, Claude-Code style: name, one-line help, and the action it runs.
struct Command {
    name: &'static str,
    help: &'static str,
}

const COMMANDS: &[Command] = &[
    Command { name: "/overview", help: "System overview + live logs" },
    Command { name: "/logs", help: "Full-width log stream" },
    Command { name: "/activity", help: "Activity feed (audit trail)" },
    Command { name: "/sessions", help: "Active chat sessions" },
    Command { name: "/swarm", help: "Mesh peers" },
    Command { name: "/clear", help: "Clear the log buffer" },
    Command { name: "/help", help: "Toggle the help overlay" },
    Command { name: "/quit", help: "Stop the node and exit" },
];

struct TuiState {
    logs: Vec<LogEntry>,
    scroll_offset: usize,
    auto_scroll: bool,
    view: View,
    /// Scroll position for the list views (activity / sessions / swarm).
    vscroll: usize,
    /// Slash-command palette: when active, keystrokes edit `cmd_input`.
    cmd_mode: bool,
    cmd_input: String,
    show_help: bool,
    /// Set to true by the `/quit` command so the main loop exits cleanly.
    quit: bool,
}

impl TuiState {
    fn new() -> Self {
        Self {
            logs: Vec::with_capacity(MAX_LOG_LINES),
            scroll_offset: 0,
            auto_scroll: true,
            view: View::Overview,
            vscroll: 0,
            cmd_mode: false,
            cmd_input: String::new(),
            show_help: false,
            quit: false,
        }
    }

    /// Commands whose name starts with the current input (without the matched prefix hidden).
    fn suggestions(&self) -> Vec<&'static Command> {
        let q = self.cmd_input.trim();
        COMMANDS
            .iter()
            .filter(|c| q.len() <= 1 || c.name.starts_with(q))
            .collect()
    }

    /// Run the typed slash command. Unknown commands are ignored (palette just closes).
    fn run_command(&mut self) {
        let cmd = self.cmd_input.trim().to_string();
        self.cmd_mode = false;
        self.cmd_input.clear();
        match cmd.as_str() {
            "/overview" => self.set_view(View::Overview),
            "/logs" => self.set_view(View::Logs),
            "/activity" => self.set_view(View::Activity),
            "/sessions" => self.set_view(View::Sessions),
            "/swarm" => self.set_view(View::Swarm),
            "/clear" => {
                self.logs.clear();
                self.scroll_offset = 0;
                self.auto_scroll = true;
            }
            "/help" => self.show_help = !self.show_help,
            "/quit" | "/q" => self.quit = true,
            _ => {}
        }
    }

    fn set_view(&mut self, v: View) {
        self.view = v;
        self.vscroll = 0;
    }

    fn cycle_view(&mut self, forward: bool) {
        let i = self.view.index();
        let n = View::ALL.len();
        let next = if forward { (i + 1) % n } else { (i + n - 1) % n };
        self.set_view(View::ALL[next]);
    }

    fn push_log(&mut self, entry: LogEntry) {
        self.logs.push(entry);
        if self.logs.len() > MAX_LOG_LINES {
            self.logs.drain(0..500); // trim oldest 500
            self.scroll_offset = self.scroll_offset.saturating_sub(500);
        }
        if self.auto_scroll {
            self.scroll_offset = self.logs.len().saturating_sub(1);
        }
    }
}

// ─── Main TUI loop ─────────────────────────────────────────────────────────

pub async fn run_tui(
    app_state: Arc<AppState>,
    mut log_rx: mpsc::UnboundedReceiver<LogEntry>,
) -> anyhow::Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    std::io::stdout().execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(std::io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let mut tui = TuiState::new();
    let start_time = std::time::Instant::now();

    loop {
        // Drain log messages (non-blocking)
        while let Ok(entry) = log_rx.try_recv() {
            tui.push_log(entry);
        }

        // Read live stats from AppState
        let stats = read_stats(&app_state, start_time).await;

        // Draw
        terminal.draw(|f| draw_ui(f, &tui, &stats))?;
        if tui.quit {
            break;
        }

        // Length of the list rendered by the active view (for scroll clamping).
        let list_len = match tui.view {
            View::Activity => stats.activity.len(),
            View::Sessions => stats.sessions.len(),
            View::Swarm => stats.peers.len(),
            _ => 0,
        };
        let logs_view = matches!(tui.view, View::Logs | View::Overview);

        // Handle input (poll with timeout for ~60fps)
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                // Command palette captures all keystrokes while open.
                if tui.cmd_mode {
                    match key.code {
                        KeyCode::Esc => {
                            tui.cmd_mode = false;
                            tui.cmd_input.clear();
                        }
                        KeyCode::Enter => tui.run_command(),
                        KeyCode::Backspace => {
                            tui.cmd_input.pop();
                            if tui.cmd_input.is_empty() {
                                tui.cmd_mode = false;
                            }
                        }
                        KeyCode::Tab => {
                            if let Some(c) = tui.suggestions().first() {
                                tui.cmd_input = c.name.to_string();
                            }
                        }
                        KeyCode::Char(c) => tui.cmd_input.push(c),
                        _ => {}
                    }
                    continue;
                }
                match key.code {
                    KeyCode::Char('q') | KeyCode::Char('Q') => break,
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => break,
                    KeyCode::Char('/') | KeyCode::Char(':') => {
                        tui.cmd_mode = true;
                        tui.cmd_input = "/".to_string();
                    }
                    KeyCode::Char('?') => tui.show_help = !tui.show_help,
                    KeyCode::Esc => tui.show_help = false,
                    KeyCode::Tab => tui.cycle_view(true),
                    KeyCode::BackTab => tui.cycle_view(false),
                    KeyCode::Char(d @ '1'..='5') => {
                        let idx = (d as u8 - b'1') as usize;
                        if idx < View::ALL.len() {
                            tui.set_view(View::ALL[idx]);
                        }
                    }
                    KeyCode::Up => {
                        if logs_view {
                            tui.auto_scroll = false;
                            tui.scroll_offset = tui.scroll_offset.saturating_sub(1);
                        } else {
                            tui.vscroll = tui.vscroll.saturating_sub(1);
                        }
                    }
                    KeyCode::Down => {
                        if logs_view {
                            tui.scroll_offset =
                                (tui.scroll_offset + 1).min(tui.logs.len().saturating_sub(1));
                            if tui.scroll_offset >= tui.logs.len().saturating_sub(1) {
                                tui.auto_scroll = true;
                            }
                        } else {
                            tui.vscroll = (tui.vscroll + 1).min(list_len.saturating_sub(1));
                        }
                    }
                    KeyCode::PageUp => {
                        if logs_view {
                            tui.auto_scroll = false;
                            tui.scroll_offset = tui.scroll_offset.saturating_sub(20);
                        } else {
                            tui.vscroll = tui.vscroll.saturating_sub(10);
                        }
                    }
                    KeyCode::PageDown => {
                        if logs_view {
                            tui.scroll_offset =
                                (tui.scroll_offset + 20).min(tui.logs.len().saturating_sub(1));
                            if tui.scroll_offset >= tui.logs.len().saturating_sub(1) {
                                tui.auto_scroll = true;
                            }
                        } else {
                            tui.vscroll = (tui.vscroll + 10).min(list_len.saturating_sub(1));
                        }
                    }
                    KeyCode::Home => {
                        if logs_view {
                            tui.auto_scroll = false;
                            tui.scroll_offset = 0;
                        } else {
                            tui.vscroll = 0;
                        }
                    }
                    KeyCode::End => {
                        if logs_view {
                            tui.auto_scroll = true;
                            tui.scroll_offset = tui.logs.len().saturating_sub(1);
                        } else {
                            tui.vscroll = list_len.saturating_sub(1);
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    std::io::stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}

// ─── Stats snapshot ─────────────────────────────────────────────────────────

struct LiveStats {
    node_name: String,
    host: String,
    port: u16,
    uptime: String,
    model: String,
    provider: String,
    cpu_pct: f32,
    ram_pct: f32,
    ram_used_mb: u64,
    ram_total_mb: u64,
    gpu_pct: Option<f32>,
    vram_used_mb: Option<u64>,
    vram_total_mb: Option<u64>,
    peer_count: usize,
    session_count: usize,
    queue_depth: usize,
    active_tokens: usize,
    total_requests: usize,
    capabilities: Vec<String>,
    // Per-view data (built once per frame).
    activity: Vec<(String, String, String, String)>, // ts, level, tag, message
    sessions: Vec<String>,                            // session keys (channel:user)
    peers: Vec<(String, String)>,                     // name, endpoint
}

async fn read_stats(state: &Arc<AppState>, start: std::time::Instant) -> LiveStats {
    let manifest = state.manifest.read().await;
    let sys = state.sys.read().await;
    let listener = state.listener.read().await;
    let nodes = listener.get_nodes().await;
    let queue = state.queue.read().await;
    let auth = state.auth.read().await;
    let sessions = state.essaim_sessions.read().await;
    let essaim_cfg = state.essaim_config.read().await;
    let activity = state.activity_log.read().await;

    let cpu_pct = sys.global_cpu_usage();
    let used_mem = sys.used_memory();
    let total_mem = sys.total_memory();
    let ram_pct = if total_mem > 0 {
        (used_mem as f32 / total_mem as f32) * 100.0
    } else {
        0.0
    };

    let elapsed = start.elapsed().as_secs();
    let uptime = format_duration(elapsed);

    let act_start = activity.len().saturating_sub(400);
    let activity_rows: Vec<(String, String, String, String)> = activity
        .iter()
        .skip(act_start)
        .map(|e| {
            (
                e.timestamp.clone(),
                e.level.clone(),
                e.tag.clone(),
                e.message.clone(),
            )
        })
        .collect();
    let mut session_keys: Vec<String> = sessions.keys().map(|k| k.to_string()).collect();
    session_keys.sort();
    let self_name = manifest.node_name.clone();
    let peers: Vec<(String, String)> = nodes
        .values()
        .map(|n| {
            let name = n
                .manifest
                .node_name
                .clone()
                .unwrap_or_else(|| n.service_fullname.clone());
            let endpoint = format!("{}:{}", n.manifest.host, n.manifest.port.unwrap_or(0));
            (name, endpoint)
        })
        .filter(|(name, _)| name != &self_name)
        .collect();

    LiveStats {
        node_name: manifest.node_name.clone(),
        host: manifest.api_endpoint.host.clone(),
        port: manifest.api_endpoint.port,
        uptime,
        model: essaim_cfg.model.clone(),
        provider: essaim_cfg.provider.clone(),
        cpu_pct,
        ram_pct,
        ram_used_mb: used_mem / 1024,
        ram_total_mb: total_mem / 1024,
        gpu_pct: manifest.resources.accelerator_usage_pct,
        vram_used_mb: manifest.resources.vram_used_mb,
        vram_total_mb: manifest.resources.vram_total_mb,
        peer_count: nodes.len().saturating_sub(1), // exclude self
        session_count: sessions.len(),
        queue_depth: queue.depth(),
        active_tokens: auth.list_tokens().len(),
        total_requests: activity.len(),
        capabilities: manifest.capabilities.to_flags(),
        activity: activity_rows,
        sessions: session_keys,
        peers,
    }
}

fn format_duration(secs: u64) -> String {
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    if h > 0 {
        format!("{}h{:02}m{:02}s", h, m, s)
    } else if m > 0 {
        format!("{}m{:02}s", m, s)
    } else {
        format!("{}s", s)
    }
}

// ─── Drawing ────────────────────────────────────────────────────────────────

fn draw_ui(f: &mut Frame, tui: &TuiState, stats: &LiveStats) {
    let size = f.area();

    // Main layout: header | tab bar | body | footer/command line
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(11), // header (logo + info)
            Constraint::Length(1),  // tab bar
            Constraint::Min(8),     // body (active view)
            Constraint::Length(1),  // footer / command line
        ])
        .split(size);

    draw_header(f, main_chunks[0], stats);
    draw_tabs(f, main_chunks[1], tui);

    match tui.view {
        View::Overview => {
            // Logs (left) + live stats sidebar (right), like the web dashboard.
            let body = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Min(40), Constraint::Length(32)])
                .split(main_chunks[2]);
            draw_logs(f, body[0], tui);
            draw_sidebar(f, body[1], stats);
        }
        View::Logs => draw_logs(f, main_chunks[2], tui),
        View::Activity => draw_activity(f, main_chunks[2], tui, stats),
        View::Sessions => draw_sessions(f, main_chunks[2], tui, stats),
        View::Swarm => draw_peers(f, main_chunks[2], tui, stats),
    }

    if tui.cmd_mode {
        draw_command_line(f, main_chunks[3], tui);
        draw_palette(f, size, tui);
    } else {
        draw_footer(f, main_chunks[3], tui);
    }

    if tui.show_help {
        draw_help(f, size);
    }
}

/// Top tab bar: the active view is highlighted, Claude-Code style.
fn draw_tabs(f: &mut Frame, area: Rect, tui: &TuiState) {
    let amber = Color::Rgb(255, 191, 0);
    let dim = Color::Rgb(110, 110, 110);
    let mut spans: Vec<Span> = vec![Span::raw(" ")];
    for (i, v) in View::ALL.iter().enumerate() {
        let label = format!(" {} {} ", i + 1, v.title());
        if *v == tui.view {
            spans.push(Span::styled(
                label,
                Style::default()
                    .fg(Color::Black)
                    .bg(amber)
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            spans.push(Span::styled(label, Style::default().fg(dim)));
        }
        spans.push(Span::raw(" "));
    }
    spans.push(Span::styled(
        "  /  command palette   ? help",
        Style::default().fg(Color::Rgb(70, 70, 70)),
    ));
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn draw_header(f: &mut Frame, area: Rect, stats: &LiveStats) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(8), Constraint::Length(3)])
        .split(area);

    let amber = Color::Rgb(255, 191, 0);
    let dim = Color::Rgb(120, 120, 120);

    let logo = format!(
        r#"
  ██╗      █████╗ ██████╗ ██╗   ██╗ ██████╗██╗  ██╗███████╗
  ██║     ██╔══██╗██╔══██╗██║   ██║██╔════╝██║  ██║██╔════╝
  ██║     ███████║██████╔╝██║   ██║██║     ███████║█████╗
  ██║     ██╔══██║██╔══██╗██║   ██║██║     ██╔══██║██╔══╝
  ███████╗██║  ██║██║  ██║╚██████╔╝╚██████╗██║  ██║███████╗
  ╚══════╝╚═╝  ╚═╝╚═╝  ╚═╝ ╚═════╝  ╚═════╝╚═╝  ╚═╝╚══════╝
  Plug in AI. That's it. • Miel Protocol v{}"#,
        env!("CARGO_PKG_VERSION")
    );

    let logo_para = Paragraph::new(logo).style(Style::default().fg(Color::Rgb(200, 200, 200)));
    f.render_widget(logo_para, chunks[0]);

    let provider_label = if stats.provider == "ollama" || stats.provider.is_empty() {
        stats.model.clone()
    } else {
        format!("{}/{}", stats.provider, stats.model)
    };

    let caps = stats.capabilities.join(", ");

    let line = Line::from(vec![
        Span::styled(
            "  LARUCHE ",
            Style::default()
                .fg(Color::Black)
                .bg(amber)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            &stats.node_name,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("  {}:{}  ", stats.host, stats.port),
            Style::default().fg(dim),
        ),
        Span::styled(
            format!("up {}", stats.uptime),
            Style::default().fg(Color::Green),
        ),
        Span::raw("  "),
        Span::styled(&provider_label, Style::default().fg(Color::Cyan)),
        Span::raw("  "),
        Span::styled(format!("[{}]", caps), Style::default().fg(dim)),
    ]);

    let block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(Color::Rgb(60, 60, 60)));

    let paragraph = Paragraph::new(line).block(block);
    f.render_widget(paragraph, area);
}

fn draw_logs(f: &mut Frame, area: Rect, tui: &TuiState) {
    let inner_height = area.height.saturating_sub(2) as usize; // borders
    let total = tui.logs.len();

    // Calculate visible window
    let end = if tui.auto_scroll {
        total
    } else {
        (tui.scroll_offset + inner_height).min(total)
    };
    let start = end.saturating_sub(inner_height);

    let lines: Vec<Line> = tui.logs[start..end]
        .iter()
        .map(|entry| {
            let level_color = match entry.level {
                tracing::Level::ERROR => Color::Red,
                tracing::Level::WARN => Color::Yellow,
                tracing::Level::INFO => Color::Green,
                tracing::Level::DEBUG => Color::Rgb(100, 100, 200),
                tracing::Level::TRACE => Color::Rgb(80, 80, 80),
            };
            let level_str = match entry.level {
                tracing::Level::ERROR => "ERR",
                tracing::Level::WARN => "WRN",
                tracing::Level::INFO => "INF",
                tracing::Level::DEBUG => "DBG",
                tracing::Level::TRACE => "TRC",
            };

            Line::from(vec![
                Span::styled(
                    format!(" {} ", entry.timestamp),
                    Style::default().fg(Color::Rgb(100, 100, 100)),
                ),
                Span::styled(
                    format!("{} ", level_str),
                    Style::default()
                        .fg(level_color)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    entry.message.clone(),
                    Style::default().fg(Color::Rgb(200, 200, 200)),
                ),
            ])
        })
        .collect();

    let scroll_indicator = if total > inner_height && !tui.auto_scroll {
        format!(" [{}/{}] ", end, total)
    } else {
        String::new()
    };

    let title = format!(" Logs ({}) {}", total, scroll_indicator);
    let block = Block::default()
        .title(title)
        .title_style(
            Style::default()
                .fg(Color::Rgb(255, 191, 0))
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(60, 60, 60)));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false });
    f.render_widget(paragraph, area);
}

fn draw_sidebar(f: &mut Frame, area: Rect, stats: &LiveStats) {
    let amber = Color::Rgb(255, 191, 0);
    let dim = Color::Rgb(100, 100, 100);

    // Split sidebar into sections
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8), // system gauges
            Constraint::Length(7), // GPU (if available) or network
            Constraint::Min(6),    // essaim status
        ])
        .split(area);

    // ─── System ───
    {
        let block = Block::default()
            .title(" System ")
            .title_style(Style::default().fg(amber).add_modifier(Modifier::BOLD))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(60, 60, 60)));
        let inner = block.inner(chunks[0]);
        f.render_widget(block, chunks[0]);

        // CPU gauge
        let cpu_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // label
                Constraint::Length(1), // gauge
                Constraint::Length(1), // label
                Constraint::Length(1), // gauge
                Constraint::Min(0),
            ])
            .split(inner);

        let cpu_label = Line::from(vec![
            Span::styled(" CPU ", Style::default().fg(Color::White)),
            Span::styled(
                format!("{:.0}%", stats.cpu_pct),
                Style::default().fg(if stats.cpu_pct > 80.0 {
                    Color::Red
                } else {
                    Color::Green
                }),
            ),
        ]);
        f.render_widget(Paragraph::new(cpu_label), cpu_chunks[0]);

        let cpu_gauge = Gauge::default()
            .gauge_style(Style::default().fg(if stats.cpu_pct > 80.0 {
                Color::Red
            } else {
                Color::Cyan
            }))
            .ratio((stats.cpu_pct as f64 / 100.0).clamp(0.0, 1.0));
        f.render_widget(cpu_gauge, cpu_chunks[1]);

        let ram_label = Line::from(vec![
            Span::styled(" RAM ", Style::default().fg(Color::White)),
            Span::styled(
                format!(
                    "{:.0}% ({}/{}G)",
                    stats.ram_pct,
                    stats.ram_used_mb / 1024,
                    stats.ram_total_mb / 1024
                ),
                Style::default().fg(if stats.ram_pct > 85.0 {
                    Color::Red
                } else {
                    Color::Green
                }),
            ),
        ]);
        f.render_widget(Paragraph::new(ram_label), cpu_chunks[2]);

        let ram_gauge = Gauge::default()
            .gauge_style(Style::default().fg(if stats.ram_pct > 85.0 {
                Color::Red
            } else {
                Color::Magenta
            }))
            .ratio((stats.ram_pct as f64 / 100.0).clamp(0.0, 1.0));
        f.render_widget(ram_gauge, cpu_chunks[3]);
    }

    // ─── GPU / Network ───
    {
        let block = Block::default()
            .title(if stats.gpu_pct.is_some() {
                " GPU "
            } else {
                " Network "
            })
            .title_style(Style::default().fg(amber).add_modifier(Modifier::BOLD))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(60, 60, 60)));
        let inner = block.inner(chunks[1]);
        f.render_widget(block, chunks[1]);

        if let Some(gpu) = stats.gpu_pct {
            let gpu_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Min(0),
                ])
                .split(inner);

            let gpu_label = Line::from(vec![
                Span::styled(" GPU ", Style::default().fg(Color::White)),
                Span::styled(format!("{:.0}%", gpu), Style::default().fg(Color::Green)),
            ]);
            f.render_widget(Paragraph::new(gpu_label), gpu_chunks[0]);

            let gpu_gauge = Gauge::default()
                .gauge_style(Style::default().fg(Color::Rgb(118, 185, 0)))
                .ratio((gpu as f64 / 100.0).clamp(0.0, 1.0));
            f.render_widget(gpu_gauge, gpu_chunks[1]);

            if let (Some(used), Some(total)) = (stats.vram_used_mb, stats.vram_total_mb) {
                let vram_pct = if total > 0 {
                    used as f32 / total as f32 * 100.0
                } else {
                    0.0
                };
                let vram_label = Line::from(vec![
                    Span::styled(" VRAM ", Style::default().fg(Color::White)),
                    Span::styled(
                        format!("{}/{}G", used / 1024, total / 1024),
                        Style::default().fg(Color::Green),
                    ),
                ]);
                f.render_widget(Paragraph::new(vram_label), gpu_chunks[2]);

                let vram_gauge = Gauge::default()
                    .gauge_style(Style::default().fg(Color::Rgb(255, 140, 0)))
                    .ratio((vram_pct as f64 / 100.0).clamp(0.0, 1.0));
                f.render_widget(vram_gauge, gpu_chunks[3]);
            }
        } else {
            // Show network info instead
            let lines = vec![
                Line::from(vec![
                    Span::styled(" Peers: ", Style::default().fg(dim)),
                    Span::styled(
                        format!("{}", stats.peer_count),
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(vec![
                    Span::styled(" Tokens: ", Style::default().fg(dim)),
                    Span::styled(
                        format!("{}", stats.active_tokens),
                        Style::default().fg(Color::White),
                    ),
                ]),
                Line::from(vec![
                    Span::styled(" Requests: ", Style::default().fg(dim)),
                    Span::styled(
                        format!("{}", stats.total_requests),
                        Style::default().fg(Color::White),
                    ),
                ]),
            ];
            f.render_widget(Paragraph::new(lines), inner);
        }
    }

    // ─── Essaim ───
    {
        let block = Block::default()
            .title(" Essaim ")
            .title_style(Style::default().fg(amber).add_modifier(Modifier::BOLD))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(60, 60, 60)));
        let inner = block.inner(chunks[2]);
        f.render_widget(block, chunks[2]);

        let lines = vec![
            Line::from(vec![
                Span::styled(" Peers    ", Style::default().fg(dim)),
                Span::styled(
                    format!("{}", stats.peer_count),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled(" Sessions ", Style::default().fg(dim)),
                Span::styled(
                    format!("{}", stats.session_count),
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled(" Queue    ", Style::default().fg(dim)),
                Span::styled(
                    format!("{}", stats.queue_depth),
                    Style::default().fg(if stats.queue_depth > 0 {
                        Color::Yellow
                    } else {
                        Color::White
                    }),
                ),
            ]),
            Line::from(vec![
                Span::styled(" Auth     ", Style::default().fg(dim)),
                Span::styled(
                    format!("{}", stats.active_tokens),
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled(" Requests ", Style::default().fg(dim)),
                Span::styled(
                    format!("{}", stats.total_requests),
                    Style::default().fg(Color::White),
                ),
            ]),
        ];
        f.render_widget(Paragraph::new(lines), inner);
    }
}

fn draw_footer(f: &mut Frame, area: Rect, tui: &TuiState) {
    let amber = Color::Rgb(255, 191, 0);
    let dim = Color::Rgb(80, 80, 80);
    let k = |s: &'static str| Span::styled(s, Style::default().fg(amber).add_modifier(Modifier::BOLD));
    let t = |s: &'static str| Span::styled(s, Style::default().fg(dim));
    let mut spans = vec![
        k(" /"),
        t(" cmd  "),
        k("Tab"),
        t(" view  "),
        k("1-5"),
        t(" jump  "),
        k("Up/Dn"),
        t(" scroll  "),
        k("?"),
        t(" help  "),
        k("q"),
        t(" quit  "),
    ];
    if matches!(tui.view, View::Logs | View::Overview) {
        spans.push(if tui.auto_scroll {
            Span::styled(" AUTO ", Style::default().fg(Color::Black).bg(Color::Green))
        } else {
            Span::styled(" SCROLLED ", Style::default().fg(Color::Black).bg(Color::Yellow))
        });
    }
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

/// The slash-command input line (replaces the footer while the palette is open).
fn draw_command_line(f: &mut Frame, area: Rect, tui: &TuiState) {
    let amber = Color::Rgb(255, 191, 0);
    let line = Line::from(vec![
        Span::styled(" > ", Style::default().fg(amber).add_modifier(Modifier::BOLD)),
        Span::styled(tui.cmd_input.clone(), Style::default().fg(Color::White)),
        Span::styled("\u{2588}", Style::default().fg(amber)),
        Span::styled(
            "   Enter run  Tab complete  Esc cancel",
            Style::default().fg(Color::Rgb(80, 80, 80)),
        ),
    ]);
    f.render_widget(
        Paragraph::new(line).style(Style::default().bg(Color::Rgb(28, 28, 28))),
        area,
    );
}

/// Floating completion popup for the command palette, anchored above the command line.
fn draw_palette(f: &mut Frame, size: Rect, tui: &TuiState) {
    let suggestions = tui.suggestions();
    if suggestions.is_empty() {
        return;
    }
    let amber = Color::Rgb(255, 191, 0);
    let n = suggestions.len().min(8) as u16;
    let h = n + 2;
    let w = 50u16.min(size.width.saturating_sub(2));
    let y = size.height.saturating_sub(h + 1);
    let area = Rect { x: 1, y, width: w, height: h };
    let lines: Vec<Line> = suggestions
        .iter()
        .take(8)
        .map(|c| {
            Line::from(vec![
                Span::styled(
                    format!(" {:<11}", c.name),
                    Style::default().fg(amber).add_modifier(Modifier::BOLD),
                ),
                Span::styled(c.help, Style::default().fg(Color::Rgb(170, 170, 170))),
            ])
        })
        .collect();
    let block = Block::default()
        .title(" commands ")
        .title_style(Style::default().fg(amber).add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(80, 80, 80)));
    f.render_widget(Clear, area);
    f.render_widget(Paragraph::new(lines).block(block), area);
}

/// Centered help overlay (toggled with `?` or `/help`).
fn draw_help(f: &mut Frame, size: Rect) {
    let amber = Color::Rgb(255, 191, 0);
    let dim = Color::Rgb(170, 170, 170);
    let row = |key: &str, desc: &str| {
        Line::from(vec![
            Span::styled(
                format!("  {:<12}", key),
                Style::default().fg(amber).add_modifier(Modifier::BOLD),
            ),
            Span::styled(desc.to_string(), Style::default().fg(dim)),
        ])
    };
    let mut lines = vec![
        Line::from(""),
        row("/", "Open the command palette"),
        row("Tab / 1-5", "Switch view"),
        row("Up/Dn PgUp", "Scroll the active view"),
        row("Home/End", "Jump to top / bottom"),
        row("? ", "Toggle this help"),
        row("q / Ctrl-C", "Quit the node"),
        Line::from(""),
        Line::from(Span::styled(
            "  Commands",
            Style::default().fg(amber).add_modifier(Modifier::BOLD),
        )),
    ];
    for c in COMMANDS {
        lines.push(Line::from(vec![
            Span::styled(
                format!("  {:<11}", c.name),
                Style::default().fg(Color::Cyan),
            ),
            Span::styled(c.help.to_string(), Style::default().fg(dim)),
        ]));
    }
    let h = (lines.len() as u16 + 2).min(size.height.saturating_sub(2));
    let w = 56u16.min(size.width.saturating_sub(4));
    let x = size.width.saturating_sub(w) / 2;
    let y = size.height.saturating_sub(h) / 2;
    let area = Rect { x, y, width: w, height: h };
    let block = Block::default()
        .title(" Help ")
        .title_style(Style::default().fg(amber).add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(amber));
    f.render_widget(Clear, area);
    f.render_widget(Paragraph::new(lines).block(block).wrap(Wrap { trim: false }), area);
}

/// Activity feed (the dashboard Audit trail), newest first.
fn draw_activity(f: &mut Frame, area: Rect, tui: &TuiState, stats: &LiveStats) {
    let amber = Color::Rgb(255, 191, 0);
    let inner_h = area.height.saturating_sub(2) as usize;
    let total = stats.activity.len();
    let max_start = total.saturating_sub(inner_h.max(1));
    let start = tui.vscroll.min(max_start);
    let rows: Vec<Line> = stats
        .activity
        .iter()
        .rev()
        .skip(start)
        .take(inner_h.max(1))
        .map(|(ts, level, tag, msg)| {
            let lc = match level.as_str() {
                "error" => Color::Red,
                "warn" => Color::Yellow,
                _ => Color::Green,
            };
            let time = ts.get(11..19).unwrap_or("");
            Line::from(vec![
                Span::styled(format!(" {} ", time), Style::default().fg(Color::Rgb(100, 100, 100))),
                Span::styled(
                    format!("{:<8}", tag),
                    Style::default().fg(lc).add_modifier(Modifier::BOLD),
                ),
                Span::styled(format!(" {}", msg), Style::default().fg(Color::Rgb(200, 200, 200))),
            ])
        })
        .collect();
    let body = if total == 0 {
        vec![Line::from(Span::styled(
            "  No activity yet.",
            Style::default().fg(Color::Rgb(110, 110, 110)),
        ))]
    } else {
        rows
    };
    let block = Block::default()
        .title(format!(" Activity ({}) ", total))
        .title_style(Style::default().fg(amber).add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(60, 60, 60)));
    f.render_widget(Paragraph::new(body).block(block).wrap(Wrap { trim: false }), area);
}

/// Active chat sessions (channel:user keys).
fn draw_sessions(f: &mut Frame, area: Rect, tui: &TuiState, stats: &LiveStats) {
    let amber = Color::Rgb(255, 191, 0);
    let inner_h = area.height.saturating_sub(2) as usize;
    let total = stats.sessions.len();
    let rows: Vec<Line> = stats
        .sessions
        .iter()
        .skip(tui.vscroll.min(total.saturating_sub(1)))
        .take(inner_h.max(1))
        .map(|key| {
            Line::from(vec![
                Span::styled(" \u{2022} ", Style::default().fg(amber)),
                Span::styled(key.clone(), Style::default().fg(Color::White)),
            ])
        })
        .collect();
    let body = if total == 0 {
        vec![Line::from(Span::styled(
            "  No active sessions.",
            Style::default().fg(Color::Rgb(110, 110, 110)),
        ))]
    } else {
        rows
    };
    let block = Block::default()
        .title(format!(" Sessions ({}) ", total))
        .title_style(Style::default().fg(amber).add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(60, 60, 60)));
    f.render_widget(Paragraph::new(body).block(block), area);
}

/// Mesh peers (the swarm).
fn draw_peers(f: &mut Frame, area: Rect, tui: &TuiState, stats: &LiveStats) {
    let amber = Color::Rgb(255, 191, 0);
    let inner_h = area.height.saturating_sub(2) as usize;
    let total = stats.peers.len();
    let rows: Vec<Line> = stats
        .peers
        .iter()
        .skip(tui.vscroll.min(total.saturating_sub(1)))
        .take(inner_h.max(1))
        .map(|(name, endpoint)| {
            Line::from(vec![
                Span::styled(" \u{2022} ", Style::default().fg(Color::Green)),
                Span::styled(
                    format!("{:<24}", name),
                    Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
                ),
                Span::styled(endpoint.clone(), Style::default().fg(Color::Rgb(120, 120, 120))),
            ])
        })
        .collect();
    let body = if total == 0 {
        vec![Line::from(Span::styled(
            "  No peers connected.",
            Style::default().fg(Color::Rgb(110, 110, 110)),
        ))]
    } else {
        rows
    };
    let block = Block::default()
        .title(format!(" Swarm peers ({}) ", total))
        .title_style(Style::default().fg(amber).add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(60, 60, 60)));
    f.render_widget(Paragraph::new(body).block(block), area);
}
