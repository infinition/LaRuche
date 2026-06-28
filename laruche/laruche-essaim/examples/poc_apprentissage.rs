//! POC of the LEARNING LOOP (auto-created skills then re-applied).
//!
//! Proves end-to-end:
//! 1) an OKF skill present in memory is RECALLED automatically and injected into a related
//!    turn (event `SkillApplied`): works even without an LLM (recall precedes the model call);
//! 2) with a real LLM (llama.cpp:8001), a substantial trajectory triggers the
//!    PROPOSAL of a new skill (event `SkillProposed`).
//!
//! Run:  cargo run -p laruche-essaim --example poc_apprentissage
//! (agent turns require an OpenAI-compatible endpoint at http://localhost:8001)

use laruche_essaim::abeilles::{enregistrer_abeilles_builtin, enregistrer_memoire};
use laruche_essaim::{boucle_react_memoire, AbeilleRegistry, ChatEvent, EssaimConfig, Session};
use laruche_memoire::{MemoireCognitive, MemoryItem, NativeBackend};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mem: Arc<dyn MemoireCognitive> = Arc::new(NativeBackend::new());

    // ─── Seed: a learned skill "already exists" in memory ──────────────────────
    println!("════════ 0. Seed an OKF skill into memory ════════");
    let skill_okf = "---\ntype: skill\nname: veille-techno\ndescription: faire une veille \
        et la résumer\nallowed-tools: [web_deep_search]\nwhen_to_use: demande de veille\n---\n\
        ## Paradigm: veille rigoureuse\n## Step: web_deep_search puis résumé en 5 points sourcés.";
    mem.write(
        MemoryItem::new("tools.skills.veille_techno", skill_okf)
            .with_source("seed")
            .with_tags(vec!["skill".into(), "okf".into()]),
    )
    .await?;
    println!("skill 'veille-techno' written under tools.skills.veille_techno\n");

    // ─── Learning-loop event counters ──────────────────────────────────────────
    let applied = Arc::new(AtomicUsize::new(0));
    let proposed = Arc::new(AtomicUsize::new(0));
    let (tx, _) = tokio::sync::broadcast::channel::<ChatEvent>(512);
    {
        let (applied, proposed) = (applied.clone(), proposed.clone());
        let mut rx = tx.subscribe();
        tokio::spawn(async move {
            while let Ok(ev) = rx.recv().await {
                match ev {
                    ChatEvent::Token { text } => print!("{text}"),
                    ChatEvent::SkillApplied { name } => {
                        applied.fetch_add(1, Ordering::Relaxed);
                        eprintln!("\n  🧠 SKILL APPLIED -> {name}");
                    }
                    ChatEvent::SkillProposed { name } => {
                        proposed.fetch_add(1, Ordering::Relaxed);
                        eprintln!("\n  ✨ SKILL BORN -> {name}");
                    }
                    ChatEvent::ToolCall { name, .. } => eprintln!("\n  ┌─ tool -> {name}"),
                    ChatEvent::Error { message } => eprintln!("\n  [error] {message}"),
                    _ => {}
                }
            }
        });
    }

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

    // ─── Conv: a request RELATED to the skill -> automatic recall ──────────────
    println!("════════ 1. Related task -> the skill must be RECALLED (SkillApplied) ════════");
    let mut conv = Session::new(&config.model);
    if let Err(e) = boucle_react_memoire(
        "Do a tech watch on vector databases and summarize it.",
        &mut conv,
        &registry,
        &config,
        &tx,
        mem.clone(),
    )
    .await
    {
        eprintln!("[turn interrupted - LLM down? the recall was emitted anyway] {e}");
    }

    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
    println!("\n════════ Summary ════════");
    println!(
        "SkillApplied (recalls)  : {}",
        applied.load(Ordering::Relaxed)
    );
    println!(
        "SkillProposed (births) : {}",
        proposed.load(Ordering::Relaxed)
    );
    assert!(
        applied.load(Ordering::Relaxed) >= 1,
        "the seeded skill should have been recalled (SkillApplied) at least once"
    );
    println!("✅ Automatic recall proven.");
    Ok(())
}
