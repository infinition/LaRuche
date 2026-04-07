//! LaRuche CLI — Interactive agent REPL with rich terminal UI
//!
//! Usage:
//!   laruche                     - Start TUI interface
//!   laruche --classic           - Start classic REPL (no TUI)
//!   laruche --cwd /path         - Start in specific directory
//!   laruche ask "question"      - One-shot question
//!   laruche discover            - Find nodes on network
//!   laruche doctor              - System health check

mod tui;

use anyhow::Result;
use crossterm::style::{Color, Stylize};
use laruche_essaim::{
    AbeilleRegistry, ChatEvent, EssaimConfig, Session,
    abeilles::enregistrer_abeilles_builtin,
    brain::boucle_react,
};
use std::io::{self, Write};
use std::path::PathBuf;
use tokio::sync::broadcast;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const AMBER: Color = Color::Rgb { r: 245, g: 158, b: 11 };

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let mut cwd: Option<PathBuf> = None;
    let mut command = "tui"; // Default: TUI mode
    let mut prompt_args: Vec<String> = Vec::new();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--cwd" | "-d" => {
                if i + 1 < args.len() { cwd = Some(PathBuf::from(&args[i + 1])); i += 2; }
                else { i += 1; }
            }
            "--classic" | "--repl" => { command = "chat"; i += 1; }
            "discover" | "scan" => { command = "discover"; i += 1; }
            "status" | "doctor" => { command = "doctor"; i += 1; }
            "server" => {
                command = "server";
                prompt_args = args[i + 1..].to_vec();
                break;
            }
            "ask" | "query" => { command = "ask"; prompt_args = args[i + 1..].to_vec(); break; }
            "mcp" => { command = "mcp"; i += 1; }
            "chat" => { command = "tui"; i += 1; }
            "help" | "--help" | "-h" => { command = "help"; i += 1; }
            _ => { command = "ask"; prompt_args = args[i..].to_vec(); break; }
        }
    }
    if let Some(ref dir) = cwd {
        if dir.exists() { std::env::set_current_dir(dir)?; }
        else { eprintln!("{} Not found: {}", "ERROR".red().bold(), dir.display()); std::process::exit(1); }
    }
    match command {
        "tui" => tui::run_tui().await?,
        "chat" => cmd_chat().await?,
        "ask" => cmd_ask(&prompt_args.join(" ")).await?,
        "server" => cmd_server(&prompt_args).await?,
        "mcp" => cmd_mcp().await?,
        "discover" => cmd_discover().await?,
        "doctor" => cmd_doctor().await?,
        "help" => print_help(),
        _ => print_help(),
    }
    Ok(())
}

fn print_banner() {
    let cwd = std::env::current_dir().unwrap_or_default();
    let model = get_model();
    eprintln!();
    eprintln!("  {}", r"  _            ____           _          ".with(AMBER));
    eprintln!("  {}", r" | |    __ _  |  _ \ _   _ __| |__   ___ ".with(AMBER));
    eprintln!("  {}", r" | |   / _` | | |_) | | | / _| '_ \ / _ \".with(AMBER));
    eprintln!("  {}", r" | |__| (_| | |  _ <| |_| \__|  | ||  __/".with(AMBER));
    eprintln!("  {}", r" |_____\__,_| |_| \_\\__,_|___|_| |_|\___|".with(AMBER));
    eprintln!();
    eprintln!("  {} {} {}",
        "L'Essaim Agent".bold(), format!("v{}", VERSION).dark_grey(),
        format!("Miel v{}", miel_protocol::PROTOCOL_VERSION).dark_grey());
    eprintln!("  {}", "─".repeat(55).dark_grey());
    eprintln!("  {}  {}   {}  {}   {}  {}",
        "cwd".dark_grey(), cwd.display().to_string().with(AMBER),
        "model".dark_grey(), model.with(Color::Cyan),
        "tools".dark_grey(), "21+".green());
    eprintln!("  {} /help {} /model {} /tools {} /cwd",
        "aide".dark_grey(), "modele".dark_grey(), "outils".dark_grey(), "dossier".dark_grey());
    eprintln!("  {}", "─".repeat(55).dark_grey());
    eprintln!();
}

fn print_help() {
    eprintln!("\n  {} {}\n", "LaRuche CLI".with(AMBER).bold(), "Agent IA local".dark_grey());
    eprintln!("  {}", "Commandes:".bold());
    eprintln!("    {}                       {}", "laruche".with(Color::Cyan), "Chat interactif");
    eprintln!("    {} {} {}", "laruche".with(Color::Cyan), "--cwd /chemin".with(AMBER), "Chat dans un dossier");
    eprintln!("    {} {} {}",  "laruche".with(Color::Cyan), "ask".with(AMBER), "\"question\"    One-shot");
    eprintln!("    {} {}            {}", "laruche".with(Color::Cyan), "discover".with(AMBER), "Scanner Miel");
    eprintln!("    {} {}              {}", "laruche".with(Color::Cyan), "doctor".with(AMBER), "Diagnostic");
    eprintln!("    {} {} {}  {}", "laruche".with(Color::Cyan), "server".with(AMBER), "[cmd]".dark_grey(), "Gerer le serveur (start/stop/install/update/uninstall)");
    eprintln!("    {} {}                 {}", "laruche".with(Color::Cyan), "mcp".with(AMBER), "MCP server (stdio, for Claude Desktop)");
    eprintln!("\n  {}", "Dans le chat:".bold());
    eprintln!("    {}      Aide              {}     Lister outils", "/help".with(AMBER), "/tools".with(AMBER));
    eprintln!("    {}     Nouvelle conv.     {}    Exporter .md", "/clear".with(AMBER), "/export".with(AMBER));
    eprintln!("    {} {}  Changer dossier   {} {} Changer modele", "/cwd".with(AMBER), "[path]".dark_grey(), "/model".with(AMBER), "[name]".dark_grey());
    eprintln!("    {}  Scanner reseau    {}    Diagnostic", "/discover".with(AMBER), "/doctor".with(AMBER));
    eprintln!("    {} {} Gerer serveur   {}      Quitter", "/server".with(AMBER), "[cmd]".dark_grey(), "/quit".with(AMBER));
    eprintln!();
    eprintln!("  {} /server start | stop | restart | status | install | uninstall | update", "Serveur:".bold());
}

fn get_ollama_url() -> String {
    std::env::var("OLLAMA_URL").or_else(|_| std::env::var("LARUCHE_OLLAMA_URL"))
        .unwrap_or_else(|_| "http://127.0.0.1:11434".to_string())
}
fn get_model() -> String {
    std::env::var("LARUCHE_MODEL").unwrap_or_else(|_| "gemma4:e4b".to_string())
}

#[allow(dead_code)]
async fn fetch_models() -> Vec<String> {
    let url = format!("{}/api/tags", get_ollama_url());
    reqwest::Client::builder().timeout(std::time::Duration::from_secs(5)).build().ok()
        .and_then(|c| {
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    c.get(&url).send().await.ok()
                        .and_then(|r| tokio::runtime::Handle::current().block_on(r.json::<serde_json::Value>()).ok())
                        .and_then(|d| d["models"].as_array().map(|a| a.iter().filter_map(|m| m["name"].as_str().map(String::from)).collect()))
                })
            })
        })
        .unwrap_or_default()
}

async fn pick_model(current: &str) -> Option<String> {
    let url = format!("{}/api/tags", get_ollama_url());
    let models: Vec<String> = match reqwest::Client::new().get(&url).send().await {
        Ok(r) => match r.json::<serde_json::Value>().await {
            Ok(d) => d["models"].as_array().unwrap_or(&vec![]).iter()
                .filter_map(|m| m["name"].as_str().map(String::from)).collect(),
            Err(_) => vec![],
        },
        Err(_) => vec![],
    };
    if models.is_empty() { eprintln!("  {} Aucun modele (Ollama?)", "!".red()); return None; }

    eprintln!("\n  {} {}", "Modeles Ollama".with(AMBER).bold(), format!("({})", models.len()).dark_grey());
    eprintln!("  {}", "─".repeat(40).dark_grey());
    for (i, name) in models.iter().enumerate() {
        let cur = if name == current { "●".green() } else { "·".dark_grey() };
        eprintln!("  {} {} {}", format!("{:>2}", i + 1).with(AMBER), cur, name.as_str().with(Color::Cyan));
    }
    eprintln!("  {}", "─".repeat(40).dark_grey());
    eprint!("  {} ", "Choix:".dark_grey());
    io::stderr().flush().ok();

    let mut line = String::new();
    io::stdin().read_line(&mut line).ok();
    let c = line.trim();
    if c.is_empty() { return None; }
    if let Ok(n) = c.parse::<usize>() {
        if n >= 1 && n <= models.len() { return Some(models[n - 1].clone()); }
    }
    models.iter().find(|m| m.contains(c)).cloned().or_else(|| Some(c.to_string()))
}

async fn cmd_chat() -> Result<()> {
    print_banner();
    let mut registry = AbeilleRegistry::new();
    enregistrer_abeilles_builtin(&mut registry);
    let config = EssaimConfig { ollama_url: get_ollama_url(), model: get_model(), ..EssaimConfig::default() };
    let sessions_dir = PathBuf::from("sessions");
    let mut session = Session::new_with_path(&config.model, &sessions_dir);
    let current_model = std::rc::Rc::new(std::cell::RefCell::new(config.model.clone()));
    let stdin = io::stdin();

    loop {
        eprint!("{} ", "❯".with(AMBER).bold());
        io::stderr().flush()?;
        let mut line = String::new();
        if stdin.read_line(&mut line).is_err() || line.is_empty() { break; }
        let input = line.trim().to_string();
        if input.is_empty() { continue; }

        if input.starts_with('/') {
            let parts: Vec<&str> = input.splitn(2, ' ').collect();
            let cmd = parts[0];
            let arg = parts.get(1).map(|s| s.trim()).unwrap_or("");
            match cmd {
                "/quit" | "/exit" | "/q" => break,
                "/help" | "/h" => { print_help(); continue; }
                "/clear" | "/new" => {
                    session = Session::new_with_path(&config.model, &sessions_dir);
                    eprintln!("  {} Nouvelle conversation\n", "✓".green()); continue;
                }
                "/cwd" => {
                    if arg.is_empty() {
                        eprintln!("  {} {}", "cwd".with(AMBER), std::env::current_dir().unwrap_or_default().display());
                    } else {
                        let p = PathBuf::from(arg);
                        if p.exists() && p.is_dir() { std::env::set_current_dir(&p).ok(); eprintln!("  {} {}", "✓".green(), p.display().to_string().with(AMBER)); }
                        else { eprintln!("  {} Introuvable: {}", "✗".red(), arg); }
                    }
                    continue;
                }
                "/model" | "/m" => {
                    let cur = current_model.borrow().clone();
                    if let Some(m) = pick_model(&cur).await {
                        *current_model.borrow_mut() = m.clone();
                        eprintln!("  {} Modele: {}\n", "✓".green(), m.with(Color::Cyan));
                    }
                    continue;
                }
                "/tools" | "/t" => {
                    eprintln!("\n  {} {}", "Abeilles".with(AMBER).bold(), format!("({} outils)", registry.noms().len()).dark_grey());
                    eprintln!("  {}", "─".repeat(50).dark_grey());
                    for name in registry.noms() {
                        if let Some(tool) = registry.get(name) {
                            let d = match tool.niveau_danger() {
                                laruche_essaim::NiveauDanger::Safe => "safe".green(),
                                laruche_essaim::NiveauDanger::NeedsApproval => "ask".yellow(),
                                laruche_essaim::NiveauDanger::Dangerous => "no".red(),
                            };
                            let desc: String = tool.description().chars().take(45).collect();
                            eprintln!("  {} {:<18} {} {}", "·".with(AMBER), name.with(Color::Cyan), format!("[{}]", d).dark_grey(), desc.dark_grey());
                        }
                    }
                    eprintln!(); continue;
                }
                "/export" | "/e" => {
                    session.auto_title();
                    let t = session.title.as_deref().unwrap_or("conv");
                    let f = format!("{}.md", t.chars().take(30).collect::<String>().replace(|c: char| !c.is_alphanumeric() && c != '-', "_"));
                    let mut md = format!("# {}\n\n", t);
                    for msg in &session.messages {
                        match msg {
                            laruche_essaim::Message::User(t) => md.push_str(&format!("**User:** {}\n\n", t)),
                            laruche_essaim::Message::Assistant(t) => md.push_str(&format!("{}\n\n---\n\n", t)),
                            _ => {}
                        }
                    }
                    match std::fs::write(&f, &md) {
                        Ok(_) => eprintln!("  {} {}", "✓".green(), f.with(AMBER)),
                        Err(e) => eprintln!("  {} {}", "✗".red(), e),
                    }
                    continue;
                }
                "/server" => {
                    let sub_args: Vec<String> = arg.split_whitespace().map(|s| s.to_string()).collect();
                    cmd_server(&sub_args).await.ok();
                    continue;
                }
                "/discover" | "/scan" => { cmd_discover().await.ok(); continue; }
                "/doctor" | "/status" => { cmd_doctor().await.ok(); continue; }
                _ => { eprintln!("  {} {} — /help", "?".yellow(), cmd); continue; }
            }
        }

        // Agent
        let (tx, mut rx) = broadcast::channel::<ChatEvent>(256);
        let model_for_run = current_model.borrow().clone();
        let mut agent_config = config.clone();
        agent_config.model = model_for_run;
        eprintln!();
        let result = boucle_react(&input, &mut session, &registry, &agent_config, &tx).await;

        drop(tx);
        while let Ok(event) = rx.try_recv() {
            match event {
                ChatEvent::Token { text } => { print!("{}", text); io::stdout().flush()?; }
                ChatEvent::ToolCall { name, .. } => {
                    eprintln!("\n  {} {}", "⚙ Tool".with(Color::Blue).bold(), name.with(Color::Cyan));
                }
                ChatEvent::ToolResult { name, success, elapsed_ms, .. } => {
                    let ms = elapsed_ms.unwrap_or(0);
                    eprintln!("  {} {} {}", if success {"✓".green()} else {"✗".red()}, name.with(Color::Cyan), format!("({}ms)",ms).dark_grey());
                }
                ChatEvent::Thinking { text } => { eprintln!("  {} {}", "💭".with(Color::Magenta), text.dark_grey()); }
                ChatEvent::Plan { items } => {
                    eprintln!("\n  {}", "Plan".with(AMBER).bold());
                    for item in &items {
                        let icon = match item.status.as_str() { "done"=>"✓".green(), "in_progress"=>"●".with(AMBER), _=>"○".dark_grey() };
                        eprintln!("    {} {}", icon, item.task);
                    }
                }
                ChatEvent::Error { message } => { eprintln!("\n  {} {}", "ERROR".red().bold(), message); }
                _ => {}
            }
        }

        match result {
            Ok(_) => {
                println!();
                eprintln!("  {} ~{} tokens {} {}",
                    "─".repeat(35).dark_grey(), session.estimated_tokens().to_string().dark_grey(),
                    "model:".dark_grey(), current_model.borrow().as_str().dark_grey());
                eprintln!();
            }
            Err(e) => { eprintln!("\n  {} {}\n", "ERROR".red().bold(), e); }
        }
    }

    session.auto_title();
    let _ = session.sauvegarder();
    eprintln!("\n  {} Au revoir !\n", "🐝".with(AMBER));
    Ok(())
}

async fn cmd_ask(prompt: &str) -> Result<()> {
    if prompt.is_empty() { eprintln!("{} laruche ask \"question\"", "Usage:".bold()); std::process::exit(1); }
    let mut reg = AbeilleRegistry::new();
    enregistrer_abeilles_builtin(&mut reg);
    let cfg = EssaimConfig { ollama_url: get_ollama_url(), model: get_model(), ..EssaimConfig::default() };
    let mut ses = Session::new(&cfg.model);
    let (tx, _) = broadcast::channel::<ChatEvent>(64);
    match boucle_react(prompt, &mut ses, &reg, &cfg, &tx).await {
        Ok(r) => println!("{}", r),
        Err(e) => { eprintln!("{} {}", "ERROR".red().bold(), e); std::process::exit(1); }
    }
    Ok(())
}

async fn cmd_discover() -> Result<()> {
    eprintln!("\n  {} Scanning Miel...\n", "🔍".with(AMBER));
    match laruche_client::LaRuche::discover().await {
        Ok(lr) => {
            for n in lr.nodes() {
                let name = n.manifest.node_name.as_deref().unwrap_or("?");
                let caps: Vec<String> = n.manifest.capabilities.iter().map(|c|c.to_string()).collect();
                eprintln!("  {} {} {}", "●".green(), name.with(AMBER).bold(),
                    format!("@ {}:{}", n.manifest.host, n.manifest.port.unwrap_or(0)).dark_grey());
                eprintln!("    [{}]", caps.join(", ").with(Color::Cyan));
            }
            eprintln!("\n  {} {} noeud(s)\n", "✓".green(), lr.node_count());
        }
        Err(e) => eprintln!("  {} {}", "✗".red(), e),
    }
    Ok(())
}

async fn cmd_doctor() -> Result<()> {
    let url = std::env::var("LARUCHE_URL").unwrap_or_else(|_| "http://127.0.0.1:8419".to_string());
    match reqwest::Client::builder().timeout(std::time::Duration::from_secs(5)).build()?
        .get(format!("{}/api/doctor", url)).send().await {
        Ok(resp) => {
            if let Ok(d) = resp.json::<serde_json::Value>().await {
                let st = d["status"].as_str().unwrap_or("?");
                eprintln!("\n  {} LaRuche {}", if st=="healthy"{"✓".green()}else{"✗".red()}, st.bold());
                eprintln!("  {}", "─".repeat(45).dark_grey());
                if let Some(checks) = d["checks"].as_array() {
                    for c in checks {
                        let n = c["name"].as_str().unwrap_or("?");
                        let s = c["status"].as_str().unwrap_or("?");
                        let dt = c["detail"].as_str().unwrap_or("");
                        let i = match s {"ok"=>"✓".green(),"warning"=>"⚠".yellow(),_=>"✗".red()};
                        eprintln!("  {} {:<22} {}", i, n.bold(), dt.dark_grey());
                    }
                }
                eprintln!();
            }
        }
        Err(e) => { eprintln!("\n  {} {}\n  {} laruche-node en marche?\n", "✗".red(), e, "→".dark_grey()); }
    }
    Ok(())
}

// ======================== Server management ========================

async fn cmd_server(args: &[String]) -> Result<()> {
    let sub = args.first().map(|s| s.as_str()).unwrap_or("help");

    match sub {
        "start" => server_start().await,
        "stop" => server_stop().await,
        "restart" => { server_stop().await?; tokio::time::sleep(std::time::Duration::from_secs(2)).await; server_start().await }
        "status" => server_status().await,
        "install" => server_install().await,
        "uninstall" => server_uninstall().await,
        "update" => server_update().await,
        "logs" => server_logs().await,
        _ => {
            eprintln!("\n  {} {}\n", "LaRuche Server".with(AMBER).bold(), "Management".dark_grey());
            eprintln!("  {}:", "Commands".bold());
            eprintln!("    {} {}    {}", "laruche server".with(Color::Cyan), "start".with(AMBER), "Start the LaRuche server");
            eprintln!("    {} {}     {}", "laruche server".with(Color::Cyan), "stop".with(AMBER), "Stop the server");
            eprintln!("    {} {}  {}", "laruche server".with(Color::Cyan), "restart".with(AMBER), "Restart the server");
            eprintln!("    {} {}   {}", "laruche server".with(Color::Cyan), "status".with(AMBER), "Check server status");
            eprintln!("    {} {}  {}", "laruche server".with(Color::Cyan), "install".with(AMBER), "Build & install as system service");
            eprintln!("    {} {}  {}", "laruche server".with(Color::Cyan), "uninstall".with(AMBER), "Remove system service");
            eprintln!("    {} {}   {}", "laruche server".with(Color::Cyan), "update".with(AMBER), "Rebuild from source (git pull + cargo build)");
            eprintln!("    {} {}     {}", "laruche server".with(Color::Cyan), "logs".with(AMBER), "Show recent server logs");
            eprintln!();
            Ok(())
        }
    }
}

/// Run MCP server over stdio (for Claude Desktop integration).
/// Reads JSON-RPC from stdin, writes responses to stdout.
async fn cmd_mcp() -> Result<()> {
    use std::io::{BufRead, BufReader, Write as IoWrite};

    eprintln!("LaRuche MCP server starting (stdio mode)...");

    // Build tool registry
    let mut registry = AbeilleRegistry::new();
    enregistrer_abeilles_builtin(&mut registry);
    let registry = std::sync::Arc::new(registry);

    eprintln!("LaRuche MCP server ready ({} tools)", registry.noms().len());

    let stdin = BufReader::new(std::io::stdin());
    let stdout = std::io::stdout();

    for line in stdin.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        let trimmed = line.trim();
        if trimmed.is_empty() { continue; }

        // Parse JSON-RPC request
        let parsed: serde_json::Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(e) => {
                let err = serde_json::json!({
                    "jsonrpc": "2.0", "id": null,
                    "error": {"code": -32700, "message": format!("Parse error: {}", e)}
                });
                let mut out = stdout.lock();
                let _ = out.write_all(serde_json::to_string(&err).unwrap_or_default().as_bytes());
                let _ = out.write_all(b"\n");
                let _ = out.flush();
                continue;
            }
        };

        let id = parsed.get("id").cloned();
        let method = parsed["method"].as_str().unwrap_or("");
        let params = parsed.get("params").cloned().unwrap_or(serde_json::json!({}));

        let response = match method {
            "initialize" => serde_json::json!({
                "jsonrpc": "2.0", "id": id,
                "result": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": {"tools": {}},
                    "serverInfo": {"name": "laruche-mcp", "version": VERSION}
                }
            }),
            "notifications/initialized" => {
                // Notification — no response needed but send ack
                serde_json::json!({"jsonrpc": "2.0", "id": id, "result": {}})
            }
            "tools/list" => {
                let tools: Vec<serde_json::Value> = registry.noms().iter().filter_map(|name| {
                    let a = registry.get(name)?;
                    Some(serde_json::json!({
                        "name": a.nom(),
                        "description": a.description(),
                        "inputSchema": a.schema()
                    }))
                }).collect();
                serde_json::json!({"jsonrpc": "2.0", "id": id, "result": {"tools": tools}})
            }
            "tools/call" => {
                let name = params["name"].as_str().unwrap_or("");
                let arguments = params.get("arguments").cloned().unwrap_or(serde_json::json!({}));
                let ctx = laruche_essaim::ContextExecution::default();

                match registry.executer(name, arguments, &ctx).await {
                    Ok(result) => {
                        let content = if result.success {
                            serde_json::json!([{"type": "text", "text": result.output}])
                        } else {
                            serde_json::json!([{"type": "text", "text": result.error.unwrap_or_else(|| "Unknown error".into())}])
                        };
                        serde_json::json!({
                            "jsonrpc": "2.0", "id": id,
                            "result": {"content": content, "isError": !result.success}
                        })
                    }
                    Err(e) => serde_json::json!({
                        "jsonrpc": "2.0", "id": id,
                        "error": {"code": -32000, "message": format!("Tool error: {}", e)}
                    }),
                }
            }
            _ => serde_json::json!({
                "jsonrpc": "2.0", "id": id,
                "error": {"code": -32601, "message": format!("Method not found: {}", method)}
            }),
        };

        let mut out = stdout.lock();
        let _ = out.write_all(serde_json::to_string(&response).unwrap_or_default().as_bytes());
        let _ = out.write_all(b"\n");
        let _ = out.flush();
    }

    Ok(())
}

async fn server_start() -> Result<()> {
    eprintln!("  {} Starting LaRuche server...", "⏳".with(AMBER));

    // Try to find the binary
    let exe = find_server_exe();
    if exe.is_none() {
        eprintln!("  {} Server binary not found. Run: laruche server install", "✗".red());
        return Ok(());
    }
    let exe = exe.unwrap();

    // Check if already running
    if probe_running().await {
        eprintln!("  {} Server already running!", "✓".green());
        return Ok(());
    }

    // Start as detached process (--no-tui because no terminal available)
    let child = {
        let mut cmd = std::process::Command::new(&exe);
        cmd.arg("--no-tui")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x00000008); // DETACHED_PROCESS
        }
        cmd.spawn()
    };

    match child {
        Ok(c) => {
            eprintln!("  {} Server started (PID: {})", "✓".green(), c.id());
            eprintln!("  {} http://localhost:8419", "→".dark_grey());
        }
        Err(e) => eprintln!("  {} Failed to start: {}", "✗".red(), e),
    }
    Ok(())
}

async fn server_stop() -> Result<()> {
    eprintln!("  {} Stopping LaRuche server...", "⏳".with(AMBER));

    if cfg!(windows) {
        let _ = std::process::Command::new("taskkill")
            .args(["/F", "/IM", "laruche-node.exe"])
            .output();
    } else {
        let _ = std::process::Command::new("pkill")
            .args(["-f", "laruche-node"])
            .output();
    }

    eprintln!("  {} Server stopped", "✓".green());
    Ok(())
}

async fn server_status() -> Result<()> {
    if probe_running().await {
        eprintln!("  {} LaRuche server is {}", "●".green(), "running".green().bold());
        cmd_doctor().await?;
    } else {
        eprintln!("  {} LaRuche server is {}", "●".red(), "stopped".red().bold());
        eprintln!("  {} laruche server start", "→".dark_grey());
    }
    Ok(())
}

async fn server_install() -> Result<()> {
    eprintln!("\n  {} Installing LaRuche server...\n", "📦".with(AMBER));

    // Find source directory
    let source_dir = find_source_dir();
    if source_dir.is_none() {
        eprintln!("  {} Source directory not found.", "✗".red());
        eprintln!("  {} Clone the repo: git clone https://github.com/infinition/LaRuche", "→".dark_grey());
        return Ok(());
    }
    let source_dir = source_dir.unwrap();

    // Build release
    eprintln!("  {} Building release binary...", "⚙".with(Color::Blue));
    let build = std::process::Command::new("cargo")
        .args(["build", "--release", "-p", "laruche-node"])
        .current_dir(&source_dir)
        .status();

    match build {
        Ok(s) if s.success() => {
            eprintln!("  {} Build successful", "✓".green());

            // Install to cargo bin (--force to overwrite existing)
            eprintln!("  {} Installing to cargo bin...", "⚙".with(Color::Blue));
            let install = std::process::Command::new("cargo")
                .args(["install", "--path", "laruche-node", "--force"])
                .current_dir(&source_dir)
                .status();
            match install {
                Ok(s) if s.success() => eprintln!("  {} Installed laruche-node", "✓".green()),
                _ => eprintln!("  {} cargo install failed", "⚠".yellow()),
            }

            if cfg!(windows) {
                eprintln!("\n  {} To start at boot (run as Admin):", "Info".with(AMBER));
                eprintln!("    sc.exe create LaRuche binPath= \"{}\" start= auto",
                    source_dir.join("target/release/laruche-node.exe").display());
                eprintln!("    sc.exe start LaRuche");
            } else {
                eprintln!("\n  {} To start at boot:", "Info".with(AMBER));
                eprintln!("    sudo cp {} /usr/local/bin/",
                    source_dir.join("target/release/laruche-node").display());
                eprintln!("    sudo systemctl enable laruche && sudo systemctl start laruche");
            }
        }
        _ => eprintln!("  {} Build failed. Check Rust toolchain.", "✗".red()),
    }
    eprintln!();
    Ok(())
}

async fn server_uninstall() -> Result<()> {
    eprintln!("  {} Uninstalling LaRuche server...", "🗑".with(AMBER));

    // Stop first
    server_stop().await?;

    if cfg!(windows) {
        let _ = std::process::Command::new("sc.exe").args(["delete", "LaRuche"]).output();
        eprintln!("  {} Windows service removed (if existed)", "✓".green());
    } else {
        let _ = std::process::Command::new("systemctl").args(["disable", "laruche"]).output();
        let _ = std::process::Command::new("rm").args(["-f", "/etc/systemd/system/laruche.service"]).output();
        let _ = std::process::Command::new("systemctl").args(["daemon-reload"]).output();
        eprintln!("  {} Systemd service removed", "✓".green());
    }

    // Remove binary
    let _ = std::process::Command::new("cargo").args(["uninstall", "laruche-node"]).output();
    eprintln!("  {} Binary removed", "✓".green());
    eprintln!();
    Ok(())
}

async fn server_update() -> Result<()> {
    eprintln!("\n  {} Updating LaRuche...\n", "🔄".with(AMBER));

    let source_dir = find_source_dir();
    if source_dir.is_none() {
        eprintln!("  {} Source directory not found.", "✗".red());
        return Ok(());
    }
    let source_dir = source_dir.unwrap();

    // Git pull
    eprintln!("  {} git pull...", "⚙".with(Color::Blue));
    let pull = std::process::Command::new("git").args(["pull"]).current_dir(&source_dir).status();
    match pull {
        Ok(s) if s.success() => eprintln!("  {} Source updated", "✓".green()),
        _ => eprintln!("  {} git pull failed (local changes?)", "⚠".yellow()),
    }

    // Rebuild
    eprintln!("  {} cargo build --release...", "⚙".with(Color::Blue));
    let was_running = probe_running().await;
    if was_running { server_stop().await?; }

    let build = std::process::Command::new("cargo")
        .args(["build", "--release", "-p", "laruche-node", "-p", "laruche-cli"])
        .current_dir(&source_dir)
        .status();

    match build {
        Ok(s) if s.success() => {
            eprintln!("  {} Build successful", "✓".green());
            // Reinstall CLI
            let _ = std::process::Command::new("cargo")
                .args(["install", "--path", "laruche-cli", "--force"])
                .current_dir(&source_dir)
                .status();
            eprintln!("  {} CLI updated", "✓".green());
        }
        _ => eprintln!("  {} Build failed", "✗".red()),
    }

    if was_running {
        eprintln!("  {} Restarting server...", "⏳".with(AMBER));
        server_start().await?;
    }

    eprintln!();
    Ok(())
}

async fn server_logs() -> Result<()> {
    if cfg!(windows) {
        eprintln!("  {} Windows: check the terminal where laruche-node was started", "Info".with(AMBER));
        eprintln!("  {} Or run: Get-EventLog -LogName Application -Source LaRuche", "→".dark_grey());
    } else {
        let _ = std::process::Command::new("journalctl")
            .args(["-u", "laruche", "-n", "50", "--no-pager"])
            .status();
    }
    Ok(())
}

pub async fn probe_running() -> bool {
    let port = std::env::var("LARUCHE_PORT").ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(miel_protocol::DEFAULT_API_PORT);
    let url = format!("http://127.0.0.1:{}/health", port);
    reqwest::Client::builder().timeout(std::time::Duration::from_secs(2)).build().ok()
        .and_then(|c| {
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    c.get(&url).send().await.ok().map(|r| r.status().is_success())
                })
            })
        })
        .unwrap_or(false)
}

pub fn find_server_exe() -> Option<PathBuf> {
    let exe_name = if cfg!(windows) { "laruche-node.exe" } else { "laruche-node" };

    // Check local build first (freshest)
    let local = PathBuf::from("target/release").join(exe_name);
    if local.exists() { return Some(local); }

    // Check cargo bin
    let cargo_dirs: Vec<PathBuf> = [
        std::env::var("CARGO_HOME").ok().map(PathBuf::from),
        std::env::var("HOME").ok().map(|h| PathBuf::from(h).join(".cargo")),
        std::env::var("USERPROFILE").ok().map(|h| PathBuf::from(h).join(".cargo")),
    ]
    .into_iter()
    .flatten()
    .collect();

    for dir in cargo_dirs {
        let exe = dir.join("bin").join(exe_name);
        if exe.exists() { return Some(exe); }
    }

    None
}

pub fn find_source_dir() -> Option<PathBuf> {
    // Check current directory
    if PathBuf::from("Cargo.toml").exists() && PathBuf::from("laruche-node").exists() {
        return Some(PathBuf::from("."));
    }
    // Check parent
    if PathBuf::from("../Cargo.toml").exists() && PathBuf::from("../laruche-node").exists() {
        return Some(PathBuf::from(".."));
    }
    None
}
