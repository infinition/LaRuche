//! La **boussole** — `cap()` : l'unique décision « continuer / poser / relancer ».
//!
//! Remplace l'enchevêtrement d'heuristiques string de l'ancien `brain.rs`. Toute
//! décision repose sur des **faits** ([`Issue`]) : stop_reason natif, état de
//! l'itinéraire, compteurs d'auto-continuation et d'appels web. Aucun matching de
//! contenu, aucun exemple métier en dur. Fonction **pure** → entièrement testable.

use crate::issue::{FinDeVol, Issue};

/// Ce que la boucle doit faire après une passe.
#[derive(Debug, Clone, PartialEq)]
pub enum Decision {
    /// Terminer le butinage avec cette raison.
    Poser(FinDeVol),
    /// Relancer une passe en injectant ce conseil (en anglais) ; ne pas rendre la main.
    Relancer(String),
    /// Exécuter ces outils puis observer.
    Recolter(Vec<crate::issue::Appel>),
    /// Rendre la main à l'utilisateur pour une clarification.
    Clarifier(String),
}

/// Contexte de décision : l'état pertinent du butinage au moment du choix.
#[derive(Debug, Clone)]
pub struct ContexteCap {
    /// Relances STÉRILES consécutives déjà consommées (remis à 0 dès qu'un outil s'exécute).
    pub auto_continue: usize,
    /// Borne DURE des relances stériles. Petit (~3). Au-delà, on rend la main au lieu de
    /// boucler. Couvre les seuls rails « modèle faible » (troncature, tool malformé, exploration).
    pub relance_max: usize,
    /// Mission de recherche longue : on pousse un peu plus avant d'accepter une fin.
    pub mode_exploration: bool,
    /// Nombre d'appels d'outils web/réseau réellement effectués.
    pub recolte_web: usize,
    /// En mode exploration, minimum d'appels web en-deçà duquel on relance (borné par `relance_max`).
    pub min_web_exploration: usize,
}

impl ContexteCap {
    fn relance_dispo(&self) -> bool {
        self.auto_continue < self.relance_max
    }
}

/// Conseils injectés (anglais — meilleur suivi d'instructions, préfixe cacheable).
mod nudge {
    pub const REPRISE_TRONQUEE: &str = "Your previous message was cut off mid-output. Continue exactly \
        where it stopped — do not repeat what you already wrote; finish the sentence or the tool-call block.";
    pub const REFORMER_OUTIL: &str = "You emitted something that looks like a tool call but is not valid \
        and executable. Re-emit ONLY one valid tool call now — no markdown, no prose.";
    pub const DEMARRER_PLAN: &str = "Plan recorded. Now EXECUTE the first step by calling the needed \
        tool — do not just restate the plan or ask for confirmation.";
    pub const EXPLORER_PLUS: &str = "This is a long-running research mission and you have not searched \
        enough yet. Do not conclude. Open a NEW angle and call a web tool now: vary queries (synonyms, EN/FR), \
        try archives/forums/source sites, and advanced operators. Record the queries and URLs you try.";
}

/// **La** décision de continuation. Pure : `(contexte, issue) -> Decision`.
pub fn cap(ctx: &ContexteCap, issue: Issue) -> Decision {
    match issue {
        // Terminaisons explicites : on fait confiance aux outils dédiés. Le résumé de
        // `mission_accomplie` est porté par l'appelant (qui a déjà l'Issue) → ici on se
        // contente de décider la fin.
        Issue::MissionAccomplie { .. } => Decision::Poser(FinDeVol::Accomplie),
        Issue::Clarification(q) => Decision::Clarifier(q),
        Issue::Outils(appels) => Decision::Recolter(appels),

        // Plan posé seul : on relance (borné) pour qu'il passe à l'action, comme un tool
        // call enchaîne. S'il radote au lieu d'agir, relance_max coupe.
        Issue::PlanEnregistre => {
            if ctx.relance_dispo() {
                Decision::Relancer(nudge::DEMARRER_PLAN.to_string())
            } else {
                Decision::Poser(FinDeVol::Accomplie)
            }
        }

        // Texte seul. ÉTAT DE L'ART (third-party/third-party/Claude Code) : une réponse sans
        // tool call = FIN DE TOUR, on rend la main. Une tâche multi-étapes continue
        // *parce qu'il y a des tool calls*, jamais à cause d'un « plan inachevé ».
        // Seuls quelques rails BORNÉS (modèles faibles) peuvent relancer.
        Issue::TexteSeul(t) => {
            // Rail 1 : sortie tronquée (finish_reason=length) → reprise exacte (standard).
            if t.tronquee && ctx.relance_dispo() {
                return Decision::Relancer(nudge::REPRISE_TRONQUEE.to_string());
            }
            // Rail 2 : tool call malformé → re-émettre. Modèle faible qui a raté la syntaxe.
            if t.malforme && ctx.relance_dispo() {
                return Decision::Relancer(nudge::REFORMER_OUTIL.to_string());
            }
            // Rail 3 : recherche longue pas assez fouillée → pousser un peu. BORNÉ par
            //   relance_max sur les tours STÉRILES (auto_continue se remet à 0 dès qu'un
            //   outil s'exécute) → tant que le modèle cherche vraiment ça continue, dès
            //   qu'il radote (texte seul répété) on coupe.
            if ctx.mode_exploration
                && ctx.recolte_web < ctx.min_web_exploration
                && ctx.relance_dispo()
            {
                return Decision::Relancer(nudge::EXPLORER_PLUS.to_string());
            }
            // Sinon : fin de tour (on rend la main à l'utilisateur).
            Decision::Poser(FinDeVol::Accomplie)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::issue::{Appel, StopReason, TexteSeul};
    use serde_json::json;

    fn ctx() -> ContexteCap {
        ContexteCap {
            auto_continue: 0,
            relance_max: 3,
            mode_exploration: false,
            recolte_web: 0,
            min_web_exploration: 12,
        }
    }

    fn texte(t: TexteSeul) -> Issue {
        Issue::TexteSeul(t)
    }

    fn base_texte() -> TexteSeul {
        TexteSeul {
            texte: "voici la réponse".into(),
            fin_native: Some(StopReason::FinTour),
            plan_inacheve: false,
            malforme: false,
            tronquee: false,
        }
    }

    #[test]
    fn mission_accomplie_pose() {
        let d = cap(&ctx(), Issue::MissionAccomplie { resume: "fini".into(), confiance: 0.9 });
        assert_eq!(d, Decision::Poser(FinDeVol::Accomplie));
    }

    #[test]
    fn clarify_rend_la_main() {
        let d = cap(&ctx(), Issue::Clarification("quelle ville ?".into()));
        assert_eq!(d, Decision::Clarifier("quelle ville ?".into()));
    }

    #[test]
    fn outils_declenchent_recolte() {
        let appels = vec![Appel::nouveau("web", json!({"q": "x"}))];
        let d = cap(&ctx(), Issue::Outils(appels.clone()));
        assert_eq!(d, Decision::Recolter(appels));
    }

    #[test]
    fn texte_simple_pose_la_fin() {
        let d = cap(&ctx(), texte(base_texte()));
        assert_eq!(d, Decision::Poser(FinDeVol::Accomplie));
    }

    #[test]
    fn troncature_relance() {
        let mut t = base_texte();
        t.tronquee = true;
        match cap(&ctx(), texte(t)) {
            Decision::Relancer(n) => assert!(n.contains("cut off")),
            autre => panic!("attendu Relancer, eu {autre:?}"),
        }
    }

    #[test]
    fn malforme_relance() {
        let mut t = base_texte();
        t.malforme = true;
        assert!(matches!(cap(&ctx(), texte(t)), Decision::Relancer(_)));
    }

    #[test]
    fn plan_inacheve_ne_force_plus_la_continuation() {
        // État de l'art : un plan inachevé NE relance PAS (anti-rambling). Texte seul = fin.
        let mut t = base_texte();
        t.plan_inacheve = true;
        assert_eq!(cap(&ctx(), texte(t)), Decision::Poser(FinDeVol::Accomplie));
    }

    #[test]
    fn exploration_refuse_fin_precoce() {
        let mut c = ctx();
        c.mode_exploration = true;
        c.recolte_web = 3; // < min_web_exploration (12)
        match cap(&c, texte(base_texte())) {
            Decision::Relancer(n) => assert!(n.contains("research mission")),
            autre => panic!("attendu Relancer, eu {autre:?}"),
        }
    }

    #[test]
    fn exploration_accepte_fin_apres_effort() {
        let mut c = ctx();
        c.mode_exploration = true;
        c.recolte_web = 15; // >= min
        assert_eq!(cap(&c, texte(base_texte())), Decision::Poser(FinDeVol::Accomplie));
    }

    #[test]
    fn relances_bornees_par_relance_max() {
        // Les rails (troncature/exploration) cessent après relance_max relances stériles.
        let mut c = ctx(); // relance_max = 3
        let mut t = base_texte();
        t.tronquee = true;
        c.auto_continue = 2; // < 3 → encore une reprise
        assert!(matches!(cap(&c, texte(t.clone())), Decision::Relancer(_)));
        c.auto_continue = 3; // == relance_max → on rend la main
        assert_eq!(cap(&c, texte(t)), Decision::Poser(FinDeVol::Accomplie));
    }

    #[test]
    fn exploration_sterile_finit_par_rendre_la_main() {
        // Modèle qui radote en texte seul sans chercher → coupe après relance_max.
        let mut c = ctx();
        c.mode_exploration = true;
        c.recolte_web = 0;
        c.auto_continue = 3; // relances stériles épuisées
        assert_eq!(cap(&c, texte(base_texte())), Decision::Poser(FinDeVol::Accomplie));
    }
}
