//! # laruche-evals: the proof harness.
//!
//! Replays a FIXED mission set against the REAL butinage engine (real provider, real
//! tools) and judges each run with hard checks: terminal reason, mission mode, web
//! effort, scout fan-out, pass count, forbidden "resignation" phrasing, produced
//! files. Prints a markdown table, writes JSONL results, and diffs against a saved
//! baseline - so every engine/prompt change is MEASURED, not vibed.
//!
//! Usage (from the workspace root, with your provider reachable):
//! ```text
//! cargo run -p laruche-evals -- [--missions evals/missions.json] [--only <id-substr>]
//!     [--repeat N] [--judge] [--save-baseline]
//! ```
//! Provider via env: RUCHE_PROVIDER, RUCHE_MODEL, RUCHE_API_KEY, RUCHE_API_BASE,
//! OLLAMA_URL, RUCHE_CONTEXT_MAX, RUCHE_AUX_MODEL.

use anyhow::{Context, Result};
use laruche_essaim::abeille::AbeilleRegistry;
use laruche_essaim::abeilles::{enregistrer_abeilles_builtin, enregistrer_delegation};
use laruche_essaim::brain::{ChatEvent, EssaimConfig};
use laruche_essaim::butinage_pont::{executer_avec_bilan, RapportMission};
use laruche_essaim::session::Session;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Delegation tool aliases (mirror of the bridge's OUTILS_DELEGATION).
const OUTILS_DELEGATION: &[&str] = &["delegate", "delegate_task", "deleguer", "spawn_specialist"];

/// "Resignation" phrasings: the agent handing the work back to the user or asking
/// permission to continue. An autonomous mission must never end on these.
const DEMISSIONS: &[&str] = &[
    "je te conseille de chercher",
    "je te conseille de cibler ta recherche",
    "tu peux chercher",
    "je t'invite à chercher",
    "cherche toi-même",
    "à toi de chercher",
    "si tu veux que j'essaie",
    "si tu veux que je continue",
    "n'hésite pas à me demander",
    "veux-tu que je continue",
    "veux-tu que j'essaie",
    "dis-moi si tu veux",
    "i suggest you search",
    "you can search for",
    "let me know if you want me to",
    "if you want me to continue",
    "would you like me to",
];

// ───────────────────────── Mission set ─────────────────────────

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct Verifs {
    /// Expected terminal reason (accomplie | plafond | erreur | ...).
    fin_attendue: Option<String>,
    /// Expected final mode (standard | exploration) - tests the mode-decision channels.
    mode_attendu: Option<String>,
    /// Web-effort floor (searches + dispatched scouts).
    min_web: Option<usize>,
    /// Scout fan-out floor (delegate calls actually attempted).
    min_delegations: Option<usize>,
    /// Pass ceiling (efficiency check: a trivial question must not take 10 passes).
    max_passes: Option<usize>,
    /// The final text must contain ALL of these (lowercase match).
    texte_doit_contenir: Vec<String>,
    /// The final text must contain NONE of these (lowercase match).
    texte_interdit: Vec<String>,
    /// Fail if the agent resigned (built-in DEMISSIONS patterns).
    interdire_demission: bool,
    /// A file that must exist after the run (relative to the cwd).
    fichier_existe: Option<String>,
}

fn timeout_defaut() -> u64 {
    600
}

#[derive(Debug, Clone, Deserialize)]
struct Mission {
    id: String,
    prompt: String,
    #[serde(default = "timeout_defaut")]
    timeout_secs: u64,
    #[serde(default)]
    verifs: Verifs,
}

// ───────────────────────── Measures & results ─────────────────────────

/// Counters harvested from the live ChatEvent stream during one run.
#[derive(Debug, Default, Clone, Serialize)]
struct Mesures {
    appels_outils: usize,
    delegations: usize,
    echecs_outils: usize,
    compactions: usize,
}

#[derive(Debug, Serialize)]
struct Resultat {
    id: String,
    ok: bool,
    fin: String,
    mode: String,
    passes: usize,
    web: usize,
    delegations: usize,
    tokens: u64,
    duree_s: f64,
    /// Names of the checks that failed (empty when ok).
    echecs: Vec<String>,
    /// The agent resigned (handed work back / asked permission) - reported always.
    demission: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    juge: Option<serde_json::Value>,
    texte_extrait: String,
}

fn contient_demission(texte: &str) -> bool {
    let t = texte.to_lowercase();
    DEMISSIONS.iter().any(|d| t.contains(d))
}

/// Hard checks: `(rapport, mesures) -> failed check names`.
fn verifier(v: &Verifs, r: &RapportMission, m: &Mesures) -> Vec<String> {
    let mut echecs = Vec::new();
    let texte = r.texte.to_lowercase();
    if let Some(fin) = &v.fin_attendue {
        if &r.fin != fin {
            echecs.push(format!("fin={} (attendu {fin})", r.fin));
        }
    }
    if let Some(mode) = &v.mode_attendu {
        if &r.mode_final != mode {
            echecs.push(format!("mode={} (attendu {mode})", r.mode_final));
        }
    }
    if let Some(min) = v.min_web {
        if r.recolte_web < min {
            echecs.push(format!("web={} (min {min})", r.recolte_web));
        }
    }
    if let Some(min) = v.min_delegations {
        if m.delegations < min {
            echecs.push(format!("delegations={} (min {min})", m.delegations));
        }
    }
    if let Some(max) = v.max_passes {
        if r.passes > max {
            echecs.push(format!("passes={} (max {max})", r.passes));
        }
    }
    for attendu in &v.texte_doit_contenir {
        if !texte.contains(&attendu.to_lowercase()) {
            echecs.push(format!("texte sans « {attendu} »"));
        }
    }
    for interdit in &v.texte_interdit {
        if texte.contains(&interdit.to_lowercase()) {
            echecs.push(format!("texte contient « {interdit} »"));
        }
    }
    if v.interdire_demission && contient_demission(&r.texte) {
        echecs.push("démission (renvoie l'utilisateur chercher / demande la permission)".into());
    }
    if let Some(chemin) = &v.fichier_existe {
        if !std::path::Path::new(chemin).exists() {
            echecs.push(format!("fichier absent: {chemin}"));
        }
    }
    echecs
}

// ───────────────────────── Optional LLM judge ─────────────────────────

const PROMPT_JUGE: &str = "You are an evaluation judge for an autonomous research agent. \
Given a MISSION and the agent's FINAL ANSWER, output STRICT JSON only: \
{\"complet\": <bool: does the answer actually accomplish the mission>, \
\"resigne\": <bool: does the agent hand the work back to the user or ask permission>, \
\"sources\": <int: number of concrete source URLs cited>, \
\"score\": <0-100 overall quality>, \"raison\": \"<one line>\"}. No prose outside the JSON.";

async fn juger(config: &EssaimConfig, mission: &str, reponse: &str) -> Option<serde_json::Value> {
    use futures_util::StreamExt;
    let messages = vec![
        serde_json::json!({"role": "system", "content": PROMPT_JUGE}),
        serde_json::json!({"role": "user", "content": format!("MISSION:\n{mission}\n\nFINAL ANSWER:\n{reponse}")}),
    ];
    let modele = config.aux_model.clone().unwrap_or_else(|| config.model.clone());
    let mut stream = laruche_essaim::providers::provider_chat_stream(
        &config.provider,
        &modele,
        &messages,
        0.0,
        600,
        &config.api_key,
        config.api_base.as_deref(),
        &config.ollama_url,
        None,
    )
    .await
    .ok()?;
    let mut texte = String::new();
    while let Some(chunk) = stream.next().await {
        texte.push_str(&chunk.text);
    }
    let (deb, fin) = (texte.find('{')?, texte.rfind('}')?);
    serde_json::from_str(&texte[deb..=fin]).ok()
}

// ───────────────────────── Runner ─────────────────────────

fn env_ou(cle: &str, defaut: &str) -> String {
    std::env::var(cle).unwrap_or_else(|_| defaut.to_string())
}

fn config_depuis_env() -> EssaimConfig {
    let defauts = EssaimConfig::default();
    EssaimConfig {
        provider: env_ou("RUCHE_PROVIDER", &defauts.provider),
        model: env_ou("RUCHE_MODEL", &defauts.model),
        api_key: env_ou("RUCHE_API_KEY", ""),
        api_base: std::env::var("RUCHE_API_BASE").ok().filter(|s| !s.is_empty()),
        ollama_url: env_ou("OLLAMA_URL", &defauts.ollama_url),
        context_max_tokens: std::env::var("RUCHE_CONTEXT_MAX")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(defauts.context_max_tokens),
        aux_model: std::env::var("RUCHE_AUX_MODEL").ok().filter(|s| !s.is_empty()),
        curateur_actif: false, // no capability review during evals
        ..defauts
    }
}

/// Runs one mission against the real engine; returns (rapport, mesures, durée).
async fn lancer_mission(
    mission: &Mission,
    registry: &AbeilleRegistry,
    config: &EssaimConfig,
) -> (Result<RapportMission>, Mesures, f64) {
    let (tx, mut rx) = tokio::sync::broadcast::channel::<ChatEvent>(4096);
    // Live collector: counts tool calls / delegations / failures from the event stream.
    let collecteur = tokio::spawn(async move {
        let mut m = Mesures::default();
        loop {
            match rx.recv().await {
                Ok(ChatEvent::ToolCall { name, .. }) => {
                    m.appels_outils += 1;
                    if OUTILS_DELEGATION.contains(&name.as_str()) {
                        m.delegations += 1;
                    }
                }
                Ok(ChatEvent::ToolResult { success, .. }) => {
                    if !success {
                        m.echecs_outils += 1;
                    }
                }
                Ok(ChatEvent::Compaction { .. }) => m.compactions += 1,
                Ok(_) => {}
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
        m
    });

    let mut session = Session::new(&config.model);
    let t0 = std::time::Instant::now();
    let resultat = tokio::time::timeout(
        std::time::Duration::from_secs(mission.timeout_secs),
        executer_avec_bilan(
            &mission.prompt,
            &mut session,
            registry,
            config,
            &tx,
            &None,
            &None, // no cognitive memory during evals: runs stay independent
            None,
            &[],
            None, // no approval channel: fully autonomous
        ),
    )
    .await;
    let duree = t0.elapsed().as_secs_f64();
    drop(tx); // closes the event stream -> collector finishes
    let mesures = collecteur.await.unwrap_or_default();

    let rapport = match resultat {
        Ok(r) => r.context("engine error"),
        Err(_) => Err(anyhow::anyhow!("timeout after {}s", mission.timeout_secs)),
    };
    (rapport, mesures, duree)
}

fn extrait(texte: &str, n: usize) -> String {
    let one: String = texte.split_whitespace().collect::<Vec<_>>().join(" ");
    one.chars().take(n).collect()
}

#[tokio::main]
async fn main() -> Result<()> {
    // ── Args ──
    let args: Vec<String> = std::env::args().skip(1).collect();
    let valeur = |flag: &str| -> Option<String> {
        args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1).cloned())
    };
    let chemin_missions = valeur("--missions").unwrap_or_else(|| "evals/missions.json".into());
    let only = valeur("--only");
    let repeat: usize = valeur("--repeat").and_then(|v| v.parse().ok()).unwrap_or(1);
    let avec_juge = args.iter().any(|a| a == "--judge");
    let save_baseline = args.iter().any(|a| a == "--save-baseline");

    // ── Mission set ──
    let brut = std::fs::read_to_string(&chemin_missions)
        .with_context(|| format!("reading {chemin_missions} (run from the workspace root?)"))?;
    let mut missions: Vec<Mission> = serde_json::from_str(&brut).context("parsing missions")?;
    if let Some(f) = &only {
        // Comma-separated substrings: `--only controle,deep_english` keeps any mission
        // whose id contains ANY of the terms.
        let termes: Vec<&str> = f.split(',').map(str::trim).filter(|s| !s.is_empty()).collect();
        missions.retain(|m| termes.iter().any(|t| m.id.contains(t)));
    }
    anyhow::ensure!(!missions.is_empty(), "no mission matches");

    // ── Engine (same wiring as the node, minus UI/memory) ──
    let config = config_depuis_env();
    let registry = Arc::new(AbeilleRegistry::new());
    enregistrer_abeilles_builtin(&registry);
    let sub_registry = Arc::new({
        let r = AbeilleRegistry::new();
        enregistrer_abeilles_builtin(&r);
        r
    });
    enregistrer_delegation(&registry, registry.clone(), sub_registry, config.clone());

    println!(
        "## Evals - provider={} model={} · {} mission(s) × {repeat}\n",
        config.provider,
        config.model,
        missions.len()
    );

    // ── Run ──
    let mut resultats: Vec<Resultat> = Vec::new();
    for mission in &missions {
        // Clean the artifacts a mission asserts on (fresh run).
        if let Some(f) = &mission.verifs.fichier_existe {
            let _ = std::fs::remove_file(f);
        }
        for essai in 0..repeat {
            let tag = if repeat > 1 { format!("{}#{}", mission.id, essai + 1) } else { mission.id.clone() };
            eprintln!("▶ {tag} …");
            let (rapport, mesures, duree) = lancer_mission(mission, &registry, &config).await;
            let res = match rapport {
                Ok(r) => {
                    let echecs = verifier(&mission.verifs, &r, &mesures);
                    let juge = if avec_juge { juger(&config, &mission.prompt, &r.texte).await } else { None };
                    Resultat {
                        id: tag,
                        ok: echecs.is_empty(),
                        fin: r.fin.clone(),
                        mode: r.mode_final.clone(),
                        passes: r.passes,
                        web: r.recolte_web,
                        delegations: mesures.delegations,
                        tokens: r.tokens_entree + r.tokens_sortie,
                        duree_s: duree,
                        demission: contient_demission(&r.texte),
                        echecs,
                        juge,
                        texte_extrait: extrait(&r.texte, 200),
                    }
                }
                Err(e) => Resultat {
                    id: tag,
                    ok: false,
                    fin: "timeout_ou_erreur".into(),
                    mode: "?".into(),
                    passes: 0,
                    web: 0,
                    delegations: mesures.delegations,
                    tokens: 0,
                    duree_s: duree,
                    demission: false,
                    echecs: vec![format!("{e}")],
                    juge: None,
                    texte_extrait: String::new(),
                },
            };
            eprintln!(
                "  {} fin={} mode={} passes={} web={} deleg={} {:.0}s{}",
                if res.ok { "✅" } else { "❌" },
                res.fin, res.mode, res.passes, res.web, res.delegations, res.duree_s,
                if res.echecs.is_empty() { String::new() } else { format!(" - {}", res.echecs.join(" · ")) }
            );
            resultats.push(res);
        }
    }

    // ── Report ──
    println!("\n| mission | ok | fin | mode | passes | web | deleg | tokens | durée | échecs |");
    println!("|---|---|---|---|---|---|---|---|---|---|");
    for r in &resultats {
        println!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {:.0}s | {} |",
            r.id,
            if r.ok { "✅" } else { "❌" },
            r.fin, r.mode, r.passes, r.web, r.delegations, r.tokens, r.duree_s,
            r.echecs.join(" · ")
        );
    }
    let ok = resultats.iter().filter(|r| r.ok).count();
    println!("\n**{ok}/{} pass** - démissions: {}", resultats.len(),
        resultats.iter().filter(|r| r.demission).count());

    // JSONL results (one line per run, machine-diffable).
    std::fs::create_dir_all("evals/results")?;
    let horodatage = chrono::Utc::now().format("%Y%m%d-%H%M%S");
    let chemin_out = format!("evals/results/run-{horodatage}.jsonl");
    let lignes: Vec<String> = resultats.iter().filter_map(|r| serde_json::to_string(r).ok()).collect();
    std::fs::write(&chemin_out, lignes.join("\n"))?;
    println!("→ {chemin_out}");

    // Baseline diff: regressions are what evals exist to catch.
    let chemin_baseline = "evals/baseline.json";
    let actuel: std::collections::BTreeMap<String, bool> =
        resultats.iter().map(|r| (r.id.clone(), r.ok)).collect();
    if let Ok(brut) = std::fs::read_to_string(chemin_baseline) {
        if let Ok(base) = serde_json::from_str::<std::collections::BTreeMap<String, bool>>(&brut) {
            for (id, ok) in &actuel {
                match base.get(id) {
                    Some(true) if !ok => println!("⚠ RÉGRESSION: {id} (passait dans la baseline)"),
                    Some(false) if *ok => println!("✔ amélioration: {id}"),
                    _ => {}
                }
            }
        }
    }
    if save_baseline {
        std::fs::write(chemin_baseline, serde_json::to_string_pretty(&actuel)?)?;
        println!("→ baseline sauvegardée ({chemin_baseline})");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rapport(fin: &str, mode: &str, web: usize, passes: usize, texte: &str) -> RapportMission {
        RapportMission {
            texte: texte.into(),
            fin: fin.into(),
            succes: fin == "accomplie",
            passes,
            recolte_web: web,
            tokens_entree: 0,
            tokens_sortie: 0,
            mode_final: mode.into(),
            etapes_plan: 0,
            etapes_faites: 0,
        }
    }

    #[test]
    fn verifie_les_seuils_et_le_mode() {
        let v = Verifs {
            fin_attendue: Some("accomplie".into()),
            mode_attendu: Some("exploration".into()),
            min_web: Some(5),
            max_passes: Some(30),
            ..Default::default()
        };
        let ok = rapport("accomplie", "exploration", 8, 12, "réponse avec sources");
        assert!(verifier(&v, &ok, &Mesures::default()).is_empty());
        let ko = rapport("plafond", "standard", 1, 40, "…");
        let echecs = verifier(&v, &ko, &Mesures::default());
        assert_eq!(echecs.len(), 4, "fin+mode+web+passes: {echecs:?}");
    }

    #[test]
    fn detecte_la_demission() {
        assert!(contient_demission("Je te conseille de chercher sur des forums."));
        assert!(contient_demission("If you want me to continue, just ask!"));
        assert!(!contient_demission("Voici les 12 sources trouvées."));
        let v = Verifs { interdire_demission: true, ..Default::default() };
        let r = rapport("accomplie", "standard", 3, 4, "Si tu veux que j'essaie encore, dis-le !");
        assert_eq!(verifier(&v, &r, &Mesures::default()).len(), 1);
    }

    #[test]
    fn verifie_delegations_et_contenus() {
        let v = Verifs {
            min_delegations: Some(2),
            texte_doit_contenir: vec!["broken sword".into()],
            texte_interdit: vec!["lorem ipsum".into()],
            ..Default::default()
        };
        let r = rapport("accomplie", "exploration", 6, 10, "Rapport final sur Broken Sword.");
        let m = Mesures { delegations: 3, ..Default::default() };
        assert!(verifier(&v, &r, &m).is_empty());
        let m0 = Mesures::default();
        assert_eq!(verifier(&v, &r, &m0).len(), 1, "fan-out manquant");
    }

    #[test]
    fn parse_le_jeu_de_missions_embarque() {
        // The shipped mission set must always parse (CI guard against typos).
        let brut = include_str!("../../evals/missions.json");
        let missions: Vec<Mission> = serde_json::from_str(brut).unwrap();
        assert!(missions.len() >= 6);
        assert!(missions.iter().any(|m| m.verifs.interdire_demission));
    }
}
