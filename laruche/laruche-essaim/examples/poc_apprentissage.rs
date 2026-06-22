//! POC de la BOUCLE D'APPRENTISSAGE (skills auto-créés puis ré-appliqués).
//!
//! Prouve le bout-en-bout :
//! 1) un skill OKF présent en mémoire est RAPPELÉ automatiquement et injecté dans un tour
//!    lié (event `SkillApplied`) — fonctionne même sans LLM (le rappel précède l'appel modèle) ;
//! 2) avec un LLM réel (llama.cpp:8001), une trajectoire substantielle déclenche la
//!    PROPOSITION d'un nouveau skill (event `SkillProposed`).
//!
//! Lancer :  cargo run -p laruche-essaim --example poc_apprentissage
//! (les tours agent nécessitent un endpoint OpenAI-compatible sur http://localhost:8001)

use laruche_essaim::abeilles::{enregistrer_abeilles_builtin, enregistrer_memoire};
use laruche_essaim::{boucle_react_memoire, AbeilleRegistry, ChatEvent, EssaimConfig, Session};
use laruche_memoire::{MemoireCognitive, MemoryItem, NativeBackend};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mem: Arc<dyn MemoireCognitive> = Arc::new(NativeBackend::new());

    // ─── Seed : un skill appris « existe déjà » en mémoire ─────────────────────
    println!("════════ 0. Seed d'un skill OKF en mémoire ════════");
    let skill_okf = "---\ntype: skill\nname: veille-techno\ndescription: faire une veille \
        et la résumer\nallowed-tools: [web_deep_search]\nwhen_to_use: demande de veille\n---\n\
        ## Paradigm: veille rigoureuse\n## Step: web_deep_search puis résumé en 5 points sourcés.";
    mem.write(
        MemoryItem::new("tools.skills.veille_techno", skill_okf)
            .with_source("seed")
            .with_tags(vec!["skill".into(), "okf".into()]),
    )
    .await?;
    println!("skill 'veille-techno' écrit sous tools.skills.veille_techno\n");

    // ─── Compteurs d'events de la boucle d'apprentissage ───────────────────────
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
                        eprintln!("\n  🧠 SKILL APPLIQUÉ → {name}");
                    }
                    ChatEvent::SkillProposed { name } => {
                        proposed.fetch_add(1, Ordering::Relaxed);
                        eprintln!("\n  ✨ SKILL NÉ → {name}");
                    }
                    ChatEvent::ToolCall { name, .. } => eprintln!("\n  ┌─ outil → {name}"),
                    ChatEvent::Error { message } => eprintln!("\n  [erreur] {message}"),
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

    // ─── Conv : une demande LIÉE au skill → rappel automatique ─────────────────
    println!("════════ 1. Tâche liée → le skill doit être RAPPELÉ (SkillApplied) ════════");
    let mut conv = Session::new(&config.model);
    if let Err(e) = boucle_react_memoire(
        "Fais-moi une veille techno sur les bases de données vectorielles et résume.",
        &mut conv,
        &registry,
        &config,
        &tx,
        mem.clone(),
    )
    .await
    {
        eprintln!("[tour interrompu — LLM absent ? le rappel a tout de même été émis] {e}");
    }

    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
    println!("\n════════ Bilan ════════");
    println!(
        "SkillApplied (rappels)  : {}",
        applied.load(Ordering::Relaxed)
    );
    println!(
        "SkillProposed (naissances) : {}",
        proposed.load(Ordering::Relaxed)
    );
    assert!(
        applied.load(Ordering::Relaxed) >= 1,
        "le skill seedé aurait dû être rappelé (SkillApplied) au moins une fois"
    );
    println!("✅ Rappel automatique prouvé.");
    Ok(())
}
