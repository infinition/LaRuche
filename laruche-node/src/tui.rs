//! Server TUI — fixed-layout terminal UI with scrolling logs.
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
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph, Wrap},
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

struct TuiState {
    logs: Vec<LogEntry>,
    scroll_offset: usize,
    auto_scroll: bool,
}

impl TuiState {
    fn new() -> Self {
        Self {
            logs: Vec::with_capacity(MAX_LOG_LINES),
            scroll_offset: 0,
            auto_scroll: true,
        }
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

        // Handle input (poll with timeout for ~60fps)
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Char('Q') => break,
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => break,
                    KeyCode::Up => {
                        tui.auto_scroll = false;
                        tui.scroll_offset = tui.scroll_offset.saturating_sub(1);
                    }
                    KeyCode::Down => {
                        tui.scroll_offset = (tui.scroll_offset + 1).min(tui.logs.len().saturating_sub(1));
                        // Re-enable auto-scroll if at bottom
                        if tui.scroll_offset >= tui.logs.len().saturating_sub(1) {
                            tui.auto_scroll = true;
                        }
                    }
                    KeyCode::PageUp => {
                        tui.auto_scroll = false;
                        tui.scroll_offset = tui.scroll_offset.saturating_sub(20);
                    }
                    KeyCode::PageDown => {
                        tui.scroll_offset = (tui.scroll_offset + 20).min(tui.logs.len().saturating_sub(1));
                        if tui.scroll_offset >= tui.logs.len().saturating_sub(1) {
                            tui.auto_scroll = true;
                        }
                    }
                    KeyCode::Home => {
                        tui.auto_scroll = false;
                        tui.scroll_offset = 0;
                    }
                    KeyCode::End => {
                        tui.auto_scroll = true;
                        tui.scroll_offset = tui.logs.len().saturating_sub(1);
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

    // Main layout: header (3) | body | footer (1)
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // header
            Constraint::Min(10),   // body
            Constraint::Length(1), // footer
        ])
        .split(size);

    draw_header(f, main_chunks[0], stats);

    // Body: logs (left, flexible) | stats sidebar (right, 30 cols)
    let sidebar_width = 32;
    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(40),
            Constraint::Length(sidebar_width),
        ])
        .split(main_chunks[1]);

    draw_logs(f, body_chunks[0], tui);
    draw_sidebar(f, body_chunks[1], stats);
    draw_footer(f, main_chunks[2], tui);
}

fn draw_header(f: &mut Frame, area: Rect, stats: &LiveStats) {
    let amber = Color::Rgb(255, 191, 0);
    let dim = Color::Rgb(120, 120, 120);

    let provider_label = if stats.provider == "ollama" || stats.provider.is_empty() {
        stats.model.clone()
    } else {
        format!("{}/{}", stats.provider, stats.model)
    };

    let caps = stats.capabilities.join(", ");

    let line = Line::from(vec![
        Span::styled("  LARUCHE ", Style::default().fg(Color::Black).bg(amber).add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        Span::styled(&stats.node_name, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        Span::styled(format!("  {}:{}", stats.host, stats.port), Style::default().fg(dim)),
        Span::raw("  "),
        Span::styled(format!("up {}", stats.uptime), Style::default().fg(Color::Green)),
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
                    Style::default().fg(level_color).add_modifier(Modifier::BOLD),
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
        .title_style(Style::default().fg(Color::Rgb(255, 191, 0)).add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(60, 60, 60)));

    let paragraph = Paragraph::new(lines).block(block).wrap(Wrap { trim: false });
    f.render_widget(paragraph, area);
}

fn draw_sidebar(f: &mut Frame, area: Rect, stats: &LiveStats) {
    let amber = Color::Rgb(255, 191, 0);
    let dim = Color::Rgb(100, 100, 100);

    // Split sidebar into sections
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8),  // system gauges
            Constraint::Length(7),  // GPU (if available) or network
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
                Style::default().fg(if stats.cpu_pct > 80.0 { Color::Red } else { Color::Green }),
            ),
        ]);
        f.render_widget(Paragraph::new(cpu_label), cpu_chunks[0]);

        let cpu_gauge = Gauge::default()
            .gauge_style(Style::default().fg(if stats.cpu_pct > 80.0 { Color::Red } else { Color::Cyan }))
            .ratio((stats.cpu_pct as f64 / 100.0).clamp(0.0, 1.0));
        f.render_widget(cpu_gauge, cpu_chunks[1]);

        let ram_label = Line::from(vec![
            Span::styled(" RAM ", Style::default().fg(Color::White)),
            Span::styled(
                format!("{:.0}% ({}/{}G)", stats.ram_pct, stats.ram_used_mb / 1024, stats.ram_total_mb / 1024),
                Style::default().fg(if stats.ram_pct > 85.0 { Color::Red } else { Color::Green }),
            ),
        ]);
        f.render_widget(Paragraph::new(ram_label), cpu_chunks[2]);

        let ram_gauge = Gauge::default()
            .gauge_style(Style::default().fg(if stats.ram_pct > 85.0 { Color::Red } else { Color::Magenta }))
            .ratio((stats.ram_pct as f64 / 100.0).clamp(0.0, 1.0));
        f.render_widget(ram_gauge, cpu_chunks[3]);
    }

    // ─── GPU / Network ───
    {
        let block = Block::default()
            .title(if stats.gpu_pct.is_some() { " GPU " } else { " Network " })
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
                let vram_pct = if total > 0 { used as f32 / total as f32 * 100.0 } else { 0.0 };
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
                        Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(vec![
                    Span::styled(" Tokens: ", Style::default().fg(dim)),
                    Span::styled(format!("{}", stats.active_tokens), Style::default().fg(Color::White)),
                ]),
                Line::from(vec![
                    Span::styled(" Requests: ", Style::default().fg(dim)),
                    Span::styled(format!("{}", stats.total_requests), Style::default().fg(Color::White)),
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
                    Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled(" Sessions ", Style::default().fg(dim)),
                Span::styled(format!("{}", stats.session_count), Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled(" Queue    ", Style::default().fg(dim)),
                Span::styled(
                    format!("{}", stats.queue_depth),
                    Style::default().fg(if stats.queue_depth > 0 { Color::Yellow } else { Color::White }),
                ),
            ]),
            Line::from(vec![
                Span::styled(" Auth     ", Style::default().fg(dim)),
                Span::styled(format!("{}", stats.active_tokens), Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled(" Requests ", Style::default().fg(dim)),
                Span::styled(format!("{}", stats.total_requests), Style::default().fg(Color::White)),
            ]),
        ];
        f.render_widget(Paragraph::new(lines), inner);
    }
}

fn draw_footer(f: &mut Frame, area: Rect, tui: &TuiState) {
    let amber = Color::Rgb(255, 191, 0);
    let dim = Color::Rgb(80, 80, 80);

    let scroll_status = if tui.auto_scroll {
        Span::styled(" AUTO-SCROLL ", Style::default().fg(Color::Black).bg(Color::Green))
    } else {
        Span::styled(" SCROLLED ", Style::default().fg(Color::Black).bg(Color::Yellow))
    };

    let line = Line::from(vec![
        Span::styled(" q", Style::default().fg(amber).add_modifier(Modifier::BOLD)),
        Span::styled(" quit  ", Style::default().fg(dim)),
        Span::styled("Up/Down", Style::default().fg(amber).add_modifier(Modifier::BOLD)),
        Span::styled(" scroll  ", Style::default().fg(dim)),
        Span::styled("PgUp/PgDn", Style::default().fg(amber).add_modifier(Modifier::BOLD)),
        Span::styled(" page  ", Style::default().fg(dim)),
        Span::styled("Home/End", Style::default().fg(amber).add_modifier(Modifier::BOLD)),
        Span::styled(" jump  ", Style::default().fg(dim)),
        Span::raw("  "),
        scroll_status,
    ]);

    f.render_widget(Paragraph::new(line), area);
}
