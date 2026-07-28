//! Pure catalogue of parameterised automation blueprints.
//!
//! A blueprint is a template with holes: a title, a prompt, a schedule, and the slots the
//! user fills. It produced a cron task and nothing else, which left the two other things
//! LaRuche schedules, a watcher and a piece of research, without any starting point.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Slot {
    pub name: String,
    pub label: String,
    pub default: String,
}

/// What a blueprint instantiates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Cible {
    /// A scheduled task: a prompt on a cron expression.
    #[default]
    Cron,
    /// A watcher: a target, a condition, and a prompt fired when it holds.
    Watcher,
    /// A piece of research: an objective, optionally on a cadence.
    Recherche,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Blueprint {
    pub id: String,
    pub title: String,
    pub schedule_template: String,
    pub prompt_template: String,
    pub slots: Vec<Slot>,
    /// Defaults to `Cron`, so a blueprint written before this field, on disk or in a user
    /// file, keeps working untouched.
    #[serde(default)]
    pub cible: Cible,
    /// Fields specific to the target, templated like the rest: `target`, `condition` and
    /// `watcher_type` for a watcher. Kept as a map rather than adding three optional
    /// fields to the struct, so a fourth target costs nothing here.
    #[serde(default)]
    pub extras: HashMap<String, String>,
}

/// Returns the built-in, parameterised automation catalogue.
pub fn catalogue() -> Vec<Blueprint> {
    vec![
        Blueprint {
            id: "veille-quotidienne".into(),
            title: "Veille quotidienne : {sujet}".into(),
            schedule_template: "0 {heure} * * *".into(),
            prompt_template: "Realise une veille quotidienne sur {sujet}. Resume les informations importantes, cite les sources et signale ce qui demande une action.".into(),
            slots: vec![
                Slot { name: "sujet".into(), label: "Sujet a surveiller".into(), default: "IA et produits OpenAI".into() },
                Slot { name: "heure".into(), label: "Heure locale (0-23)".into(), default: "9".into() },
            ],
            cible: Cible::Cron,
            extras: HashMap::new(),
        },
        Blueprint {
            id: "consolidation-hebdomadaire".into(),
            title: "Consolidation hebdomadaire : {perimetre}".into(),
            schedule_template: "0 {heure} * * {jour}".into(),
            prompt_template: "Consolide les elements de {perimetre} de la semaine. Produis les decisions, progres, risques et priorites de la semaine suivante.".into(),
            slots: vec![
                Slot { name: "perimetre".into(), label: "Perimetre".into(), default: "mes projets actifs".into() },
                Slot { name: "jour".into(), label: "Jour (1=lundi, 7=dimanche)".into(), default: "5".into() },
                Slot { name: "heure".into(), label: "Heure locale (0-23)".into(), default: "17".into() },
            ],
            cible: Cible::Cron,
            extras: HashMap::new(),
        },
        Blueprint {
            id: "distillation-inbox".into(),
            title: "Distillation inbox : {source}".into(),
            schedule_template: "*/{minutes} * * * *".into(),
            prompt_template: "Examine {source}, distille les nouveaux elements en actions, informations a retenir et questions a clarifier. Ne traite pas deux fois le meme element.".into(),
            slots: vec![
                Slot { name: "source".into(), label: "Source inbox".into(), default: "mon inbox".into() },
                Slot { name: "minutes".into(), label: "Frequence en minutes".into(), default: "30".into() },
            ],
            cible: Cible::Cron,
            extras: HashMap::new(),
        },
        Blueprint {
            id: "revue-mensuelle".into(),
            title: "Revue mensuelle : {objectif}".into(),
            schedule_template: "0 {heure} {jour} * *".into(),
            prompt_template: "Fais une revue mensuelle liee a {objectif}. Compare le plan et le reel, identifie les apprentissages puis propose un plan concret pour le mois a venir.".into(),
            slots: vec![
                Slot { name: "objectif".into(), label: "Objectif principal".into(), default: "mes objectifs strategiques".into() },
                Slot { name: "jour".into(), label: "Jour du mois".into(), default: "1".into() },
                Slot { name: "heure".into(), label: "Heure locale (0-23)".into(), default: "10".into() },
            ],
            cible: Cible::Cron,
            extras: HashMap::new(),
        },
        // ── Watchers: a condition on the world, not a clock ──────────────────────────
        Blueprint {
            id: "veille-page-web".into(),
            title: "Changement sur {page}".into(),
            schedule_template: String::new(), // a watcher polls, it has no cron expression
            prompt_template: "La page a change. Resume ce qui est nouveau par rapport a l'etat precedent, et dis si cela demande une action.".into(),
            slots: vec![
                Slot { name: "page".into(), label: "URL a surveiller".into(), default: "https://example.com/pricing".into() },
            ],
            cible: Cible::Watcher,
            extras: HashMap::from([
                ("watcher_type".to_string(), "url".to_string()),
                ("target".to_string(), "{page}".to_string()),
                ("condition".to_string(), "le contenu de la page a change".to_string()),
            ]),
        },
        Blueprint {
            id: "surveillance-erreurs".into(),
            title: "Erreurs dans {fichier}".into(),
            schedule_template: String::new(),
            prompt_template: "De nouvelles lignes d'erreur sont apparues. Donne la cause probable et l'action a mener.".into(),
            slots: vec![
                Slot { name: "fichier".into(), label: "Fichier de log".into(), default: "logs/app.log".into() },
                Slot { name: "motif".into(), label: "Motif a repérer".into(), default: "ERROR".into() },
            ],
            cible: Cible::Watcher,
            extras: HashMap::from([
                ("watcher_type".to_string(), "log".to_string()),
                ("target".to_string(), "{fichier}".to_string()),
                ("condition".to_string(), "une nouvelle ligne contient {motif}".to_string()),
            ]),
        },
        // ── Research: an objective, run once or on a cadence ─────────────────────────
        Blueprint {
            id: "etat-de-l-art".into(),
            title: "Etat de l'art : {sujet}".into(),
            schedule_template: String::new(), // on demand unless the user sets a cadence
            prompt_template: "Etablis un etat de l'art sur {sujet}. Recense les approches, compare-les, cite tes sources et conclus sur ce qui est le plus solide aujourd'hui.".into(),
            slots: vec![
                Slot { name: "sujet".into(), label: "Sujet a etudier".into(), default: "les modeles de memoire pour agents".into() },
            ],
            cible: Cible::Recherche,
            extras: HashMap::new(),
        },
        Blueprint {
            id: "comparatif-solutions".into(),
            title: "Comparatif : {besoin}".into(),
            schedule_template: String::new(),
            prompt_template: "Compare les solutions repondant a {besoin}. Pour chacune: ce qu'elle fait, ce qu'elle coute, ses limites. Termine par une recommandation argumentee.".into(),
            slots: vec![
                Slot { name: "besoin".into(), label: "Besoin a couvrir".into(), default: "heberger un modele en local".into() },
            ],
            cible: Cible::Recherche,
            extras: HashMap::new(),
        },
    ]
}

/// Substitutes the slots inside the target-specific fields, exactly like the title and the
/// prompt. Empty for a cron blueprint, which needs none.
pub fn instancier_extras(
    bp: &Blueprint,
    valeurs: &HashMap<String, String>,
) -> HashMap<String, String> {
    bp.extras
        .iter()
        .map(|(cle, gabarit)| {
            let valeur = bp.slots.iter().fold(gabarit.clone(), |sortie, slot| {
                let v = valeurs.get(&slot.name).unwrap_or(&slot.default);
                sortie.replace(&format!("{{{}}}", slot.name), v)
            });
            (cle.clone(), valeur)
        })
        .collect()
}

/// Instantiates a blueprint using supplied values and slot defaults.
/// Unknown placeholders are left untouched, which keeps a malformed template visible.
pub fn instancier(bp: &Blueprint, valeurs: &HashMap<String, String>) -> (String, String, String) {
    let substitute = |template: &str| {
        bp.slots.iter().fold(template.to_string(), |output, slot| {
            let value = valeurs.get(&slot.name).unwrap_or(&slot.default);
            output.replace(&format!("{{{}}}", slot.name), value)
        })
    };

    (
        substitute(&bp.title),
        substitute(&bp.schedule_template),
        substitute(&bp.prompt_template),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalogue_contains_ready_to_use_blueprints() {
        let blueprints = catalogue();
        assert!(blueprints.len() >= 6, "cron, watcher and research families");
        assert!(blueprints
            .iter()
            .all(|blueprint| !blueprint.slots.is_empty()));
    }

    #[test]
    fn chaque_famille_a_son_point_de_depart() {
        let bps = catalogue();
        for cible in [Cible::Cron, Cible::Watcher, Cible::Recherche] {
            assert!(
                bps.iter().any(|b| b.cible == cible),
                "aucun blueprint pour {cible:?}"
            );
        }
        // A watcher needs its target and its type, otherwise nothing can be created.
        for b in bps.iter().filter(|b| b.cible == Cible::Watcher) {
            assert!(b.extras.contains_key("target"), "{} sans cible", b.id);
            assert!(b.extras.contains_key("watcher_type"), "{} sans type", b.id);
        }
    }

    #[test]
    fn les_extras_recoivent_les_memes_valeurs_que_le_titre() {
        let bp = catalogue()
            .into_iter()
            .find(|b| b.id == "veille-page-web")
            .unwrap();
        let mut valeurs = HashMap::new();
        valeurs.insert("page".to_string(), "https://laruche.dev".to_string());
        let extras = instancier_extras(&bp, &valeurs);
        assert_eq!(extras.get("target").map(String::as_str), Some("https://laruche.dev"));
        assert_eq!(extras.get("watcher_type").map(String::as_str), Some("url"));
    }

    #[test]
    fn instancier_substitutes_values_and_uses_defaults() {
        let blueprint = catalogue()
            .into_iter()
            .find(|blueprint| blueprint.id == "veille-quotidienne")
            .unwrap();
        let mut valeurs = HashMap::new();
        valeurs.insert("sujet".into(), "securite Rust".into());

        let (name, cron_expr, prompt) = instancier(&blueprint, &valeurs);
        assert_eq!(name, "Veille quotidienne : securite Rust");
        assert_eq!(cron_expr, "0 9 * * *");
        assert!(prompt.contains("securite Rust"));
    }
}
