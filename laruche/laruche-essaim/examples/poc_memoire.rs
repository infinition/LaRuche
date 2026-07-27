//! Functional POC of the LaRuche x paradigm merge.
//!
//! 1) DETERMINISTIC memory round-trip via the `MemoireCognitive` trait (proves the memory
//!    layer, independently of the LLM).
//! 2) The REAL essaim agent running on llama.cpp (:8001): it stores a fact in one
//!    conversation, then recalls it in a FRESH conversation, so via memory,
//!    not via history.
//!
//! Run:  cargo run -p laruche-essaim --example poc_memoire

use laruche_essaim::abeilles::{enregistrer_abeilles_builtin, enregistrer_memoire};
use laruche_essaim::{boucle_react_memoire, AbeilleRegistry, ChatEvent, EssaimConfig, Session};
use laruche_memoire::{MemoireCognitive, MemoryItem, NativeBackend, SearchOpts};
use std::sync::Arc;

fn spawn_printer(tx: &tokio::sync::broadcast::Sender<ChatEvent>) {
    let mut rx = tx.subscribe();
    tokio::spawn(async move {
        while let Ok(ev) = rx.recv().await {
            match ev {
                ChatEvent::Token { text } => print!("{text}"),
                ChatEvent::ToolCall { name, args, .. } => {
                    eprintln!("\n  ┌─ tool → {name} {args}");
                }
                ChatEvent::ToolResult { name, result, .. } => {
                    eprintln!(
                        "  └─ tool ← {name}: {}",
                        result.lines().next().unwrap_or("")
                    );
                }
                ChatEvent::Done { .. } => eprintln!("\n  [end of turn]"),
                ChatEvent::Error { message } => eprintln!("\n  [error] {message}"),
                _ => {}
            }
        }
    });
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mem: Arc<dyn MemoireCognitive> = Arc::new(NativeBackend::new());

    // ─── 1) Deterministic proof of the memory layer ───────────────────────────
    println!("════════ 1. Memory round-trip (deterministic, no LLM) ════════");
    mem.write(
        MemoryItem::new("people.alex", "Préfère le langage Rust pour ses projets.")
            .with_source("test direct"),
    )
    .await?;
    let pack = mem
        .search("quel langage préfère alex", SearchOpts::default())
        .await?;
    println!("search → {}", pack.to_prompt_text());
    println!("backend health → {}\n", mem.health().await?);

    // ─── 2) The real agent on llama.cpp:8001 ────────────────────────────────────
    let config = EssaimConfig {
        provider: "openai".into(),
        model: "qwen3.6-35b-a3b".into(),
        api_base: Some("http://localhost:8001".into()),
        api_key: "sk-local".into(),
        max_iterations: 5,
        temperature: 0.2,
        max_tokens: 1024,
        ..Default::default()
    };

    let mut registry = AbeilleRegistry::new();
    enregistrer_abeilles_builtin(&mut registry);
    enregistrer_memoire(&mut registry, mem.clone());

    let (tx, _) = tokio::sync::broadcast::channel::<ChatEvent>(512);
    spawn_printer(&tx);

    // NB: no prompt mentions a memory tool; auto-retrieval + auto-curation
    // are handled by `boucle_react_memoire` (P2). This is "memory in the loop".
    println!("════════ 2a. Conversation A - the agent STORES (auto-curation) ════════");
    let mut conv_a = Session::new(&config.model);
    let r = boucle_react_memoire(
        "Petite info sur moi : je code toujours en tongs en écoutant du jazz.",
        &mut conv_a,
        &registry,
        &config,
        &tx,
        mem.clone(),
    )
    .await;
    if let Err(e) = r {
        eprintln!("[turn A interrupted] {e}");
    }

    println!("\n════════ 2b. Conversation B (FRESH) - the agent RECALLS (auto-retrieval) ════════");
    let mut conv_b = Session::new(&config.model);
    let r = boucle_react_memoire(
        "Dans quelles conditions est-ce que je code, d'habitude ? Réponds directement.",
        &mut conv_b,
        &registry,
        &config,
        &tx,
        mem.clone(),
    )
    .await;
    if let Err(e) = r {
        eprintln!("[turn B interrupted] {e}");
    }

    // Let the printer drain the stream.
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    println!("\n════════ POC finished ════════");
    Ok(())
}
