//! The **escale**: the halt where the tool "makes honey": context compaction
//! between passes to last over time.
//!
//! Driven by the [`Jauge`]. POC version: **extractive** compaction (no LLM call,
//! deterministic): keep the anchor (the mission), a summary of intermediate turns
//! (tools used + latest observations), and the N recent turns intact. LLM
//! consolidation into cognitive memory will come via the bridge (`Source`).

use crate::cap::jauge::{Besoin, Jauge};
use crate::carnet::Carnet;
use crate::evenement::{Emetteur, Evenement};
use crate::fournisseur::Fournisseur;
use crate::messagerie::{Message, Role};
use crate::nectar::Source;

/// Examines the gauge and compacts if needed. Returns an escale event if a
/// compaction occurred (for the UI), otherwise `None`.
pub fn peut_etre(carnet: &mut Carnet, jauge: &Jauge, garder_recents: usize) -> Option<Evenement> {
    let cible = match jauge.besoin() {
        Besoin::Rien => return None,
        Besoin::Compacter => garder_recents,
        // Consolidation: more aggressive compaction (keep fewer turns).
        Besoin::Consolider => (garder_recents / 2).max(4),
    };
    compacter(&mut carnet.historique, cible).map(|(avant, apres)| Evenement::Escale { avant, apres })
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

/// **Cognitive consolidation** (heavy escale): extracts durable facts from the history
/// via an auxiliary LLM call, writes them to memory (`source`), then restarts on a fresh
/// context (anchor + resume). Makes the mission *cumulative* without saturating the context.
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
    // Fresh context: anchor (mission) + resume instruction via memory.
    carnet.historique = vec![
        Message::systeme(format!(
            "=== Resumed after cognitive consolidation: {} fact(s) stored to long-term memory. \
             Use memory_search to recall what you already found, then continue the mission from where it stood. ===",
            faits.len()
        )),
        Message::utilisateur(carnet.mission.clone()),
    ];
    Some(Evenement::Escale { avant, apres: carnet.historique.len() })
}

/// Renders the history as flat text for extraction.
fn rendu_historique(h: &[Message]) -> String {
    h.iter()
        .filter(|m| !m.contenu.trim().is_empty())
        .map(|m| {
            let role = match m.role {
                Role::Systeme => "system",
                Role::Utilisateur => "user",
                Role::Assistant => "assistant",
                Role::Observation => "tool",
            };
            format!("[{role}] {}", m.contenu)
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
