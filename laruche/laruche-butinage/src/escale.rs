//! The **escale**: the halt where the tool "makes honey": context compaction
//! between passes to last over time.
//!
//! Driven by the [`Jauge`]. Two compaction modes:
//! - **LLM summary** (default, [`compacter_intelligent`]): dense summary preserving
//!   discoveries, decisions, plan state and dead-ends — what a long research needs
//!   to not re-explore. Falls back to extractive when the auxiliary call fails.
//! - **Extractive** ([`compacter`]): deterministic, no LLM call (tools used +
//!   latest observations). POC mode and fallback.
//!
//! When the gauge is critical and a memory is plugged in, [`consolider`] extracts
//! durable facts to long-term memory and restarts on a fresh — but self-sufficient —
//! context (mission + plan + facts + last turns).

use crate::cap::jauge::{Besoin, Jauge};
use crate::carnet::Carnet;
use crate::evenement::{Emetteur, Evenement};
use crate::fournisseur::Fournisseur;
use crate::messagerie::{Message, Role};
use crate::nectar::Source;

/// Number of recent turns to keep for a given gauge need. `None` = nothing to do.
fn cible_recents(jauge: &Jauge, garder_recents: usize) -> Option<usize> {
    match jauge.besoin() {
        Besoin::Rien => None,
        Besoin::Compacter => Some(garder_recents),
        // Consolidation: more aggressive compaction (keep fewer turns).
        Besoin::Consolider => Some((garder_recents / 2).max(4)),
    }
}

/// Examines the gauge and compacts (extractively) if needed. Returns an escale event
/// if a compaction occurred (for the UI), otherwise `None`.
pub fn peut_etre(carnet: &mut Carnet, jauge: &Jauge, garder_recents: usize) -> Option<Evenement> {
    let cible = cible_recents(jauge, garder_recents)?;
    compacter(&mut carnet.historique, cible).map(|(avant, apres)| Evenement::Escale { avant, apres })
}

const PROMPT_COMPACTION: &str = "You are compacting the working context of an autonomous agent \
mid-mission. Summarize the transcript segment below DENSELY so the agent can continue seamlessly. \
You MUST preserve: (1) concrete discoveries and facts WITH their sources/URLs; (2) decisions taken \
and why; (3) the current plan state and what remains to do; (4) dead ends and approaches already \
tried that FAILED (so they are not retried); (5) exact identifiers (paths, names, ids, numbers). \
Omit pleasantries and reasoning chatter. Output the summary only, no preamble.";

/// **LLM compaction** (default mode): summarizes the old turns via an auxiliary call,
/// keeping the anchor (mission) and the recent turns intact. Falls back to the
/// extractive [`compacter`] when the call fails or returns nothing.
pub async fn compacter_intelligent(
    carnet: &mut Carnet,
    fournisseur: &dyn Fournisseur,
    jauge: &Jauge,
    garder_recents: usize,
    emet: &dyn Emetteur,
) -> Option<Evenement> {
    let cible = cible_recents(jauge, garder_recents)?;
    let avant = carnet.historique.len();
    if avant <= cible + 2 {
        return None; // too short to be worthwhile
    }
    emet.emettre(Evenement::Statut("🍯 LLM context compaction…".into()));
    let split = avant - cible;
    let rendu = rendu_historique(&carnet.historique[..split]);
    let messages = vec![Message::systeme(PROMPT_COMPACTION), Message::utilisateur(rendu)];
    match fournisseur.repondre(&messages, &[]).await {
        Ok(r) if !r.texte.trim().is_empty() => {
            let ancre = carnet.historique[..split]
                .iter()
                .find(|m| m.role == Role::Utilisateur)
                .cloned();
            let resume = Message::systeme(format!(
                "[Compacted context: {} earlier messages summarized]\n{}",
                split,
                r.texte.trim()
            ));
            let queue: Vec<Message> = carnet.historique[split..].to_vec();
            let mut nouveau = Vec::with_capacity(cible + 2);
            if let Some(a) = ancre {
                nouveau.push(a);
            }
            nouveau.push(resume);
            nouveau.extend(queue);
            carnet.historique = nouveau;
            Some(Evenement::Escale { avant, apres: carnet.historique.len() })
        }
        // Auxiliary call failed or empty: deterministic fallback, never skip the pass.
        _ => compacter(&mut carnet.historique, cible)
            .map(|(avant, apres)| Evenement::Escale { avant, apres }),
    }
}

/// Extractive compaction of the history. Returns `(avant, apres)` if it occurred.
pub fn compacter(historique: &mut Vec<Message>, garder_recents: usize) -> Option<(usize, usize)> {
    let avant = historique.len();
    if avant <= garder_recents + 2 {
        return None; // too short to be worthwhile
    }
    let split = avant - garder_recents;
    let milieu = &historique[..split];
    let ancre = milieu.iter().find(|m| m.role == Role::Utilisateur).cloned();
    let resume = Message::systeme(resumer(milieu));
    let queue: Vec<Message> = historique[split..].to_vec();

    let mut nouveau = Vec::with_capacity(garder_recents + 2);
    if let Some(a) = ancre {
        nouveau.push(a);
    }
    nouveau.push(resume);
    nouveau.extend(queue);

    *historique = nouveau;
    Some((avant, historique.len()))
}

/// Extractive summary of a block of messages: tools used + latest observations.
fn resumer(msgs: &[Message]) -> String {
    let mut outils: Vec<&str> = Vec::new();
    for m in msgs {
        if let Some(o) = m.outil.as_deref() {
            if !outils.contains(&o) {
                outils.push(o);
            }
        }
    }
    let derniers: Vec<String> = msgs
        .iter()
        .rev()
        .filter(|m| m.role == Role::Observation)
        .take(2)
        .map(|m| {
            let apercu: String = m.contenu.chars().take(400).collect();
            format!("- {}: {}", m.outil.as_deref().unwrap_or("?"), apercu)
        })
        .collect();

    format!(
        "[Compacted context: {} earlier messages summarized]\nTools already used: {}\nMost recent observations:\n{}",
        msgs.len(),
        if outils.is_empty() { "(none)".into() } else { outils.join(", ") },
        if derniers.is_empty() { "(none)".into() } else { derniers.join("\n") }
    )
}

const PROMPT_EXTRACTION: &str = "You are a memory consolidator. From the agent conversation below, extract \
the DURABLE facts about the WORLD, the MISSION and the USER worth remembering across sessions: discoveries, \
decisions, stable user preferences, key results, useful URLs. Output STRICT JSON only, an array of objects: \
[{\"node_id\":\"<domain>.<subject>\",\"content\":\"<concise fact>\"}]. Use snake_case dotted node_ids \
(e.g. research.dungeon_siege, decisions.archi, people.fabien). \
DO NOT memorize the agent's OWN capabilities, tool names, or anything already in its system prompt - that is \
NOT a durable fact. NEVER use the reserved domains `system.*` or `capacities.*` (system-managed). \
If nothing durable, output []. No prose, JSON only.";

/// Default extraction prompt (to expose it in the UI "restore default").
pub fn prompt_extraction_defaut() -> &'static str {
    PROMPT_EXTRACTION
}

/// Renders the plan with statuses, so a fresh context still shows where the mission stands.
fn rendu_plan(it: &crate::itineraire::Itineraire) -> String {
    if it.est_vide() {
        return String::new();
    }
    let lignes: Vec<String> = it
        .etapes
        .iter()
        .map(|e| format!("- [{:?}] {}", e.statut, e.titre))
        .collect();
    format!("\nCurrent plan state:\n{}", lignes.join("\n"))
}

/// **Cognitive consolidation** (heavy escale): extracts durable facts from the history
/// via an auxiliary LLM call, writes them to memory (`source`), then restarts on a fresh
/// but SELF-SUFFICIENT context: mission (with its attachments), extracted facts inline,
/// plan state, and the last few turns. Makes the mission *cumulative* without
/// saturating the context — and without amnesia about where it stood.
pub async fn consolider(
    carnet: &mut Carnet,
    fournisseur: &dyn Fournisseur,
    source: &dyn Source,
    emet: &dyn Emetteur,
    prompt_extraction: Option<&str>,
) -> Option<Evenement> {
    emet.emettre(Evenement::Statut("🧠 Cognitive consolidation…".into()));
    let messages = vec![
        Message::systeme(prompt_extraction.unwrap_or(PROMPT_EXTRACTION)),
        Message::utilisateur(rendu_historique(&carnet.historique)),
    ];
    let reponse = fournisseur.repondre(&messages, &[]).await.ok()?;
    let faits = parse_faits(&reponse.texte);
    if faits.is_empty() {
        // Extraction produced nothing usable. Keep the full history rather than wiping it:
        // resetting now would drop all the mission context for zero saved facts.
        emet.emettre(Evenement::Statut(
            "🧠 Consolidation skipped (no facts extracted)".into(),
        ));
        return None;
    }
    for (node_id, content) in &faits {
        source.consigner(node_id, content).await;
    }

    let avant = carnet.historique.len();
    // The original anchor may carry multimodal pieces (images/audio): keep them.
    let pieces_ancre = carnet
        .historique
        .iter()
        .find(|m| m.role == Role::Utilisateur)
        .map(|m| m.pieces.clone())
        .unwrap_or_default();
    // Last few turns (non-internal): the immediate thread survives the reset.
    let mut queue: Vec<Message> = carnet
        .historique
        .iter()
        .rev()
        .filter(|m| !m.interne)
        .take(3)
        .cloned()
        .collect();
    queue.reverse();
    let faits_liste: Vec<String> = faits
        .iter()
        .take(20)
        .map(|(n, c)| format!("- {n}: {c}"))
        .collect();

    let mut nouveau = vec![
        Message::systeme(format!(
            "=== Resumed after cognitive consolidation ===\n\
             {} fact(s) stored to long-term memory, including:\n{}\n{}\n\
             Use memory_search to recall older details. Continue the mission from where it stood.",
            faits.len(),
            faits_liste.join("\n"),
            rendu_plan(&carnet.itineraire)
        )),
        Message::utilisateur_multimodal(carnet.mission.clone(), pieces_ancre),
    ];
    nouveau.extend(queue);
    carnet.historique = nouveau;
    Some(Evenement::Escale { avant, apres: carnet.historique.len() })
}

/// Renders the history as flat text for extraction/compaction.
fn rendu_historique(h: &[Message]) -> String {
    h.iter()
        .filter(|m| !m.contenu.trim().is_empty() || !m.appels.is_empty())
        .map(|m| {
            let role = match m.role {
                Role::Systeme => "system",
                Role::Utilisateur => "user",
                Role::Assistant => "assistant",
                Role::Observation => "tool",
            };
            let mut ligne = format!("[{role}] {}", m.contenu);
            for a in &m.appels {
                ligne.push_str(&format!("\n[call] {} {}", a.nom, a.args));
            }
            ligne
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// Extracts the `[{node_id, content}]` list from text (tolerant of chatter around the JSON).
fn parse_faits(texte: &str) -> Vec<(String, String)> {
    let (Some(deb), Some(fin)) = (texte.find('['), texte.rfind(']')) else {
        return Vec::new();
    };
    if fin <= deb {
        return Vec::new();
    }
    let json = &texte[deb..=fin];
    let Ok(v) = serde_json::from_str::<serde_json::Value>(json) else {
        return Vec::new();
    };
    v.as_array()
        .map(|a| {
            a.iter()
                .filter_map(|f| {
                    let node = f.get("node_id").and_then(|x| x.as_str())?;
                    let content = f.get("content").and_then(|x| x.as_str())?;
                    if node.trim().is_empty() || content.trim().is_empty() {
                        None
                    } else {
                        Some((node.to_string(), content.to_string()))
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cap::jauge::Jauge;
    use crate::carnet::ModeMission;

    fn t0() -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::from_timestamp(1_700_000_000, 0).unwrap()
    }

    fn hist(n: usize) -> Vec<Message> {
        let mut v = vec![Message::utilisateur("MISSION")];
        for i in 0..n {
            v.push(Message::assistant(format!("pensée {i}")));
            v.push(Message::observation("web_search", format!("résultat {i}")));
        }
        v
    }

    #[test]
    fn compacter_garde_ancre_resume_et_queue() {
        let mut h = hist(10); // 1 + 20 = 21 messages
        let (avant, apres) = compacter(&mut h, 6).unwrap();
        assert_eq!(avant, 21);
        // anchor + summary + 6 recent = 8
        assert_eq!(apres, 8);
        assert_eq!(h.len(), 8);
        assert_eq!(h[0].contenu, "MISSION"); // anchor preserved
        assert_eq!(h[1].role, Role::Systeme); // summary
        assert!(h[1].contenu.contains("web_search")); // tools mentioned
    }

    #[test]
    fn compacter_noop_si_court() {
        let mut h = hist(1); // 3 messages
        assert!(compacter(&mut h, 6).is_none());
        assert_eq!(h.len(), 3);
    }

    #[test]
    fn peut_etre_compacte_quand_jauge_haute() {
        let mut carnet = Carnet::ouvrir("m", ModeMission::Standard, t0());
        carnet.historique = hist(20);
        let mut jauge = Jauge::nouvelle(1000, 0.70, 0.85);
        jauge.utilise = 750; // ratio 0.75 -> Compacter
        let ev = peut_etre(&mut carnet, &jauge, 8);
        assert!(matches!(ev, Some(Evenement::Escale { .. })));
        assert!(carnet.historique.len() < 41);
    }

    #[test]
    fn peut_etre_rien_si_jauge_basse() {
        let mut carnet = Carnet::ouvrir("m", ModeMission::Standard, t0());
        carnet.historique = hist(20);
        let mut jauge = Jauge::nouvelle(1000, 0.70, 0.85);
        jauge.utilise = 300; // ratio 0.3
        assert!(peut_etre(&mut carnet, &jauge, 8).is_none());
        assert_eq!(carnet.historique.len(), 41);
    }

    #[tokio::test]
    async fn compacter_intelligent_utilise_le_resume_llm() {
        use crate::fournisseur::{ErreurFournisseur, ReponseModele};
        use crate::issue::StopReason;
        struct FournisseurResume;
        #[async_trait::async_trait]
        impl Fournisseur for FournisseurResume {
            async fn repondre(
                &self,
                _m: &[Message],
                _s: &[serde_json::Value],
            ) -> Result<ReponseModele, ErreurFournisseur> {
                Ok(ReponseModele {
                    texte: "RESUME_DENSE: la source X confirme Y.".into(),
                    stop: StopReason::FinTour,
                    appels: vec![],
                    usage: None,
                })
            }
        }
        let mut carnet = Carnet::ouvrir("m", ModeMission::Standard, t0());
        carnet.historique = hist(20);
        let mut jauge = Jauge::nouvelle(1000, 0.70, 0.85);
        jauge.utilise = 750;
        let ev = compacter_intelligent(
            &mut carnet,
            &FournisseurResume,
            &jauge,
            8,
            &crate::evenement::Silencieux,
        )
        .await;
        assert!(matches!(ev, Some(Evenement::Escale { .. })));
        assert_eq!(carnet.historique[0].contenu, "MISSION"); // anchor kept
        assert!(carnet.historique[1].contenu.contains("RESUME_DENSE")); // LLM summary
    }

    #[tokio::test]
    async fn compacter_intelligent_retombe_sur_l_extractif_en_cas_d_echec() {
        use crate::fournisseur::{ErreurFournisseur, ReponseModele};
        struct FournisseurMort;
        #[async_trait::async_trait]
        impl Fournisseur for FournisseurMort {
            async fn repondre(
                &self,
                _m: &[Message],
                _s: &[serde_json::Value],
            ) -> Result<ReponseModele, ErreurFournisseur> {
                Err(ErreurFournisseur { status: 500, retry_after: None, corps: "down".into() })
            }
        }
        let mut carnet = Carnet::ouvrir("m", ModeMission::Standard, t0());
        carnet.historique = hist(20);
        let mut jauge = Jauge::nouvelle(1000, 0.70, 0.85);
        jauge.utilise = 750;
        let ev = compacter_intelligent(
            &mut carnet,
            &FournisseurMort,
            &jauge,
            8,
            &crate::evenement::Silencieux,
        )
        .await;
        // fallback: the extractive compaction still happened
        assert!(matches!(ev, Some(Evenement::Escale { .. })));
        assert!(carnet.historique.len() < 41);
        assert!(carnet.historique[1].contenu.contains("Tools already used"));
    }

    #[test]
    fn parse_faits_extrait_le_json_entoure_de_bavardage() {
        let txt = "Voici les faits :\n[{\"node_id\":\"research.x\",\"content\":\"A\"}, \
                   {\"node_id\":\"\",\"content\":\"vide\"}, {\"node_id\":\"decisions.y\",\"content\":\"B\"}]\nVoilà.";
        let f = parse_faits(txt);
        assert_eq!(f, vec![
            ("research.x".to_string(), "A".to_string()),
            ("decisions.y".to_string(), "B".to_string()),
        ]);
    }

    #[test]
    fn parse_faits_vide_si_pas_de_json() {
        assert!(parse_faits("rien à consolider").is_empty());
        assert!(parse_faits("[]").is_empty());
    }
}
