//! POC fonctionnel de la fusion LaRuche × paradigm.
//!
//! 1) Round-trip mémoire DÉTERMINISTE via le trait `MemoireCognitive` (prouve la couche
//!    mémoire, indépendamment du LLM).
//! 2) L'agent essaim RÉEL tournant sur llama.cpp (:8001) : il mémorise un fait dans une
//!    conversation, puis le rappelle dans une conversation NEUVE — donc via la mémoire,
//!    pas via l'historique.
//!
//! Lancer :  cargo run -p laruche-essaim --example poc_memoire

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
                    eprintln!("\n  ┌─ outil → {name} {args}");
                }
                ChatEvent::ToolResult { name, result, .. } => {
                    eprintln!(
                        "  └─ outil ← {name}: {}",
                        result.lines().next().unwrap_or("")
                    );
                }
                ChatEvent::Done { .. } => eprintln!("\n  [fin du tour]"),
                ChatEvent::Error { message } => eprintln!("\n  [erreur] {message}"),
                _ => {}
            }
        }
    });
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mem: Arc<dyn MemoireCognitive> = Arc::new(NativeBackend::new());

    // ─── 1) Preuve déterministe de la couche mémoire ───────────────────────────
    println!("════════ 1. Round-trip mémoire (déterministe, sans LLM) ════════");
    mem.write(
        MemoryItem::new("people.fabien", "Préfère le langage Rust pour ses projets.")
            .with_source("test direct"),
    )
    .await?;
    let pack = mem
        .search("quel langage préfère fabien", SearchOpts::default())
        .await?;
    println!("recherche → {}", pack.to_prompt_text());
    println!("santé backend → {}\n", mem.health().await?);

    // ─── 2) L'agent réel sur llama.cpp:8001 ────────────────────────────────────
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

    // NB : aucun prompt ne mentionne d'outil mémoire — l'auto-récup + l'auto-curation
    // sont gérées par `boucle_react_memoire` (P2). C'est ça la « mémoire dans la boucle ».
    println!("════════ 2a. Conversation A — l'agent MÉMORISE (auto-curation) ════════");
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
        eprintln!("[tour A interrompu] {e}");
    }

    println!("\n════════ 2b. Conversation B (NEUVE) — l'agent RAPPELLE (auto-récup) ════════");
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
        eprintln!("[tour B interrompu] {e}");
    }

    // Laisse le printer vider le flux.
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    println!("\n════════ POC terminé ════════");
    Ok(())
}
