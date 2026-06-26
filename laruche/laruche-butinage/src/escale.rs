//! L'**escale** — la halte où l'abeille « fait le miel » : compaction du contexte
//! entre deux passes pour tenir sur la durée.
//!
//! Pilotée par la [`Jauge`]. Version POC : compaction **extractive** (sans appel LLM,
//! déterministe) — on garde l'ancre (la mission), un résumé des tours intermédiaires
//! (outils utilisés + dernières observations), et les N tours récents intacts. La
//! consolidation LLM vers la mémoire cognitive viendra via le pont (`Source`).

use crate::cap::jauge::{Besoin, Jauge};
use crate::carnet::Carnet;
use crate::evenement::Evenement;
use crate::messagerie::{Message, Role};

/// Examine la jauge et compacte si nécessaire. Renvoie un événement d'escale si une
/// compaction a eu lieu (pour l'UI), sinon `None`.
pub fn peut_etre(carnet: &mut Carnet, jauge: &Jauge, garder_recents: usize) -> Option<Evenement> {
    let cible = match jauge.besoin() {
        Besoin::Rien => return None,
        Besoin::Compacter => garder_recents,
        // Consolidation : compaction plus agressive (on garde moins de tours).
        Besoin::Consolider => (garder_recents / 2).max(4),
    };
    compacter(&mut carnet.historique, cible).map(|(avant, apres)| Evenement::Escale { avant, apres })
}

/// Compaction extractive de l'historique. Renvoie `(avant, après)` si elle a eu lieu.
pub fn compacter(historique: &mut Vec<Message>, garder_recents: usize) -> Option<(usize, usize)> {
    let avant = historique.len();
    if avant <= garder_recents + 2 {
        return None; // trop court pour valoir le coup
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

/// Résumé extractif d'un bloc de messages : outils utilisés + dernières observations.
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
        "[Compacted context — {} earlier messages summarized]\nTools already used: {}\nMost recent observations:\n{}",
        msgs.len(),
        if outils.is_empty() { "(none)".into() } else { outils.join(", ") },
        if derniers.is_empty() { "(none)".into() } else { derniers.join("\n") }
    )
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
        // ancre + résumé + 6 récents = 8
        assert_eq!(apres, 8);
        assert_eq!(h.len(), 8);
        assert_eq!(h[0].contenu, "MISSION"); // ancre préservée
        assert_eq!(h[1].role, Role::Systeme); // résumé
        assert!(h[1].contenu.contains("web_search")); // outils mentionnés
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
        jauge.utilise = 750; // ratio 0.75 → Compacter
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
}
