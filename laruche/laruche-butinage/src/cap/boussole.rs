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
    /// L'itinéraire a-t-il encore des étapes ouvertes ?
    pub plan_inacheve: bool,
    /// Nombre d'auto-continuations déjà consommées.
    pub auto_continue: usize,
    /// Plafond d'auto-continuations (borne anti-runaway).
    pub auto_continue_max: usize,
    /// Mission de recherche longue : on refuse les conclusions prématurées.
    pub mode_exploration: bool,
    /// Nombre d'appels d'outils web/réseau réellement effectués.
    pub recolte_web: usize,
    /// En mode exploration, minimum d'appels web avant d'accepter une fin.
    pub min_web_exploration: usize,
}

impl ContexteCap {
    fn auto_dispo(&self) -> bool {
        self.auto_continue < self.auto_continue_max
    }
}

/// Conseils injectés (anglais — meilleur suivi d'instructions, préfixe cacheable).
mod nudge {
    pub const REPRISE_TRONQUEE: &str = "Your previous message was cut off mid-output. Continue exactly \
        where it stopped — do not repeat what you already wrote; finish the sentence or the tool-call block.";
    pub const REFORMER_OUTIL: &str = "You emitted something that looks like a tool call but is not valid \
        and executable. Re-emit ONLY one valid tool call now — no markdown, no prose.";
    pub const ETAPE_SUIVANTE: &str = "Continue with the next open step of your plan immediately, without \
        stopping and without asking me. Call the needed tool directly. Conclude ONLY when every plan step is done.";
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

        // Texte seul : on décide sur les faits, par priorité.
        Issue::TexteSeul(t) => {
            // 1) Sortie tronquée → reprise exacte (borné).
            if t.tronquee && ctx.auto_dispo() {
                return Decision::Relancer(nudge::REPRISE_TRONQUEE.to_string());
            }
            // 2) Tool call malformé → reformer (borné). Rail pour modèles faibles.
            if t.malforme && ctx.auto_dispo() {
                return Decision::Relancer(nudge::REFORMER_OUTIL.to_string());
            }
            // 3) Plan inachevé → enchaîner l'étape suivante (borné).
            if t.plan_inacheve && ctx.auto_dispo() {
                return Decision::Relancer(nudge::ETAPE_SUIVANTE.to_string());
            }
            // 4) Mode exploration : refuser une fin avant un minimum d'effort web (borné).
            //    Fait, pas heuristique de contenu : on ne lit pas « rien trouvé », on
            //    constate qu'on n'a pas assez cherché et que le plan n'est pas bouclé.
            if ctx.mode_exploration
                && ctx.recolte_web < ctx.min_web_exploration
                && ctx.auto_dispo()
            {
                return Decision::Relancer(nudge::EXPLORER_PLUS.to_string());
            }
            // 5) Sinon : fin naturelle.
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
            plan_inacheve: false,
            auto_continue: 0,
            auto_continue_max: 6,
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
    fn plan_inacheve_enchaine() {
        let mut t = base_texte();
        t.plan_inacheve = true;
        match cap(&ctx(), texte(t)) {
            Decision::Relancer(n) => assert!(n.contains("next open step")),
            autre => panic!("attendu Relancer, eu {autre:?}"),
        }
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
    fn budget_auto_continue_epuise_pose() {
        let mut c = ctx();
        c.auto_continue = 6; // == max → plus de relance
        let mut t = base_texte();
        t.plan_inacheve = true;
        assert_eq!(cap(&c, texte(t)), Decision::Poser(FinDeVol::Accomplie));
    }
}
