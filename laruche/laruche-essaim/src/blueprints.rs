//! Pure catalogue of parameterised cron automation blueprints.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Slot {
    pub name: String,
    pub label: String,
    pub default: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Blueprint {
    pub id: String,
    pub title: String,
    pub schedule_template: String,
    pub prompt_template: String,
    pub slots: Vec<Slot>,
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
        },
    ]
}

/// Instantiates a blueprint using supplied values and slot defaults.
/// Unknown placeholders remain untouched, which keeps a malformed template visible.
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
        assert!((3..=5).contains(&blueprints.len()));
        assert!(blueprints
            .iter()
            .all(|blueprint| !blueprint.slots.is_empty()));
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
