//! LaRuche CLI
//!
//! Command-line tool for discovering and interacting with LaRuche nodes.
//!
//! Usage:
//!   laruche discover           - Find LaRuche nodes on the network
//!   laruche ask "question"     - Ask a question to the best available node
//!   laruche status             - Show detailed status of connected nodes
//!   laruche chat               - Interactive chat session

use anyhow::Result;
use laruche_client::LaRuche;
use std::io::{self, BufRead, Write};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("laruche_client=info,miel_protocol=info")
        .init();

    let args: Vec<String> = std::env::args().collect();
    let command = args.get(1).map(|s| s.as_str()).unwrap_or("help");

    print_banner();

    match command {
        "discover" | "scan" => cmd_discover().await?,
        "ask" | "query" => {
            let prompt = args.get(2..).map(|a| a.join(" ")).unwrap_or_default();
            if prompt.is_empty() {
                eprintln!("Usage: laruche ask \"your question here\"");
                std::process::exit(1);
            }
            cmd_ask(&prompt).await?;
        }
        "chat" => cmd_chat().await?,
        "status" => cmd_status().await?,
        "help" | "--help" | "-h" => print_help(),
        _ => {
            // If no subcommand, treat the entire arg as a prompt
            let prompt = args[1..].join(" ");
            cmd_ask(&prompt).await?;
        }
    }

    Ok(())
}

fn print_banner() {
    eprintln!(
        r#"
   LaRuche CLI v{}
  LAND Protocol v{}
"#,
        env!("CARGO_PKG_VERSION"),
        miel_protocol::PROTOCOL_VERSION,
    );
}

fn print_help() {
    println!(
        r#"Usage: laruche <command> [args]

Commands:
  discover        Scan the network for LaRuche nodes
  ask "prompt"    Send a prompt to the best available node
  chat            Start an interactive chat session
  status          Show detailed status of all nodes
  help            Show this help message

Examples:
  laruche discover
  laruche ask "Explain quantum computing in simple terms"
  laruche chat
  laruche status

Environment:
  LARUCHE_URL     Direct connection URL (skip discovery)
                  Example: LARUCHE_URL=http://192.168.1.42:8419
"#
    );
}

async fn get_laruche() -> Result<LaRuche> {
    if let Ok(url) = std::env::var("LARUCHE_URL") {
        eprintln!("   Connecting directly to {url}");
        Ok(LaRuche::connect(&url))
    } else {
        eprintln!("   Discovering LaRuche nodes on the network...");
        match LaRuche::discover().await {
            Ok(laruche) => {
                eprintln!("   Found {} node(s)\n", laruche.node_count());
                Ok(laruche)
            }
            Err(e) => {
                eprintln!("   No LaRuche nodes found: {e}");
                eprintln!("   Make sure a LaRuche node is running: cargo run -p laruche-node");
                eprintln!("   Or set LARUCHE_URL to connect directly\n");
                Err(e.into())
            }
        }
    }
}

async fn cmd_discover() -> Result<()> {
    eprintln!("   Scanning network for LaRuche nodes...\n");

    let laruche = get_laruche().await?;

    println!("┌─────────────────────────────────────────────────────┐");
    println!("│            Discovered LaRuche Nodes                 │");
    println!("├─────────────────────────────────────────────────────┤");

    for node in laruche.nodes() {
        let name = node.manifest.node_name.as_deref().unwrap_or("unknown");
        let host = &node.manifest.host;
        let port = node.manifest.port.unwrap_or(0);
        let caps: Vec<String> = node
            .manifest
            .capabilities
            .iter()
            .map(|c| c.to_string())
            .collect();
        let tps = node
            .manifest
            .tokens_per_sec
            .map(|t| format!("{t:.1}"))
            .unwrap_or("-".into());

        println!("│   {name}");
        println!("│     Host: {host}:{port}");
        println!("│     Capabilities: [{}]", caps.join(", "));
        println!("│     Speed: {tps} tokens/sec");
        println!(
            "│     Tier: {}",
            node.manifest.tier.as_deref().unwrap_or("unknown")
        );
        println!("├─────────────────────────────────────────────────────┤");
    }

    println!(
        "│  Total: {} node(s)                                  │",
        laruche.node_count()
    );
    println!("└─────────────────────────────────────────────────────┘");

    Ok(())
}

async fn cmd_ask(prompt: &str) -> Result<()> {
    let laruche = get_laruche().await?;

    eprintln!("   Thinking...\n");

    match laruche.ask(prompt).await {
        Ok(resp) => {
            println!("{}", resp.text);
            eprintln!(
                "\n  ---\n   {} tokens | {}ms | model: {} | node: {}",
                resp.tokens, resp.latency_ms, resp.model, resp.node_name
            );
        }
        Err(e) => {
            eprintln!("   Error: {e}");
            eprintln!("   Is Ollama running? Try: ollama serve");
        }
    }

    Ok(())
}

async fn cmd_chat() -> Result<()> {
    let laruche = get_laruche().await?;

    println!("   Interactive chat (type 'quit' to exit)\n");

    let stdin = io::stdin();
    loop {
        print!("  You > ");
        io::stdout().flush()?;

        let mut line = String::new();
        stdin.lock().read_line(&mut line)?;
        let prompt = line.trim();

        if prompt.is_empty() {
            continue;
        }
        if prompt == "quit" || prompt == "exit" {
            println!("   Au revoir !");
            break;
        }

        match laruche.ask(prompt).await {
            Ok(resp) => {
                println!("\n   > {}\n", resp.text.trim());
            }
            Err(e) => {
                eprintln!("   Error: {e}\n");
            }
        }
    }

    Ok(())
}

async fn cmd_status() -> Result<()> {
    let laruche = get_laruche().await?;

    for node in laruche.nodes() {
        let url = node.manifest.api_url().unwrap_or_default();
        println!("Fetching status from {url}...");

        let client = reqwest::Client::new();
        match client.get(&url).send().await {
            Ok(resp) => {
                if let Ok(status) = resp.json::<serde_json::Value>().await {
                    println!("{}", serde_json::to_string_pretty(&status)?);
                }
            }
            Err(e) => eprintln!("   Could not reach node: {e}"),
        }
    }

    Ok(())
}
