//! La constitution: les regles communes a TOUS les specialistes.
//!
//! C'est la couche verrouillee de la deliberation. Les specialistes ne diffèrent que
//! par leur strategie de raisonnement, posee au-dessus de ce socle identique. Une
//! constitution commune rend les tours comparables entre eux et les sorties
//! exploitables par l'arbitre.
//!
//! Ce qu'elle ne fait PAS, et qu'il faut avoir en tete: elle reduit la variance de
//! FORME, pas les biais PARTAGES. Dix strategies sur un meme modele de base ont les
//! memes angles morts, et l'accord qui en sort mesure alors la conformite plutot que la
//! justesse. La diversite reelle vient des modeles differents - c'est le champ `modele`
//! du specialiste, et la raison pour laquelle l'essaim compte ici.
//!
//! Le motif est deja eprouve dans LaRuche: la charte de LaReine est compilee dans le
//! binaire et surchargeable par un noeud memoire (`system.prompt_reine`). On fait pareil,
//! avec `system.constitution`.

/// Le socle, compile dans le binaire.
///
/// L'ordre des regles n'est pas anodin: « la verite avant le consensus » vient en
/// dernier parce que c'est celle qui doit gagner quand deux regles se contredisent.
pub const CONSTITUTION: &str = "\
# Constitution

These rules bind you and every other participant. They are not negotiable, and your
strategy does not replace them.

1. **Never invent.** If you do not know, say so. A plausible wrong answer costs more
   than an admission of ignorance, because nobody is going to check it.

2. **Flag your assumptions.** Every line of reasoning rests on some; yours too. A silent
   assumption is an error nobody can correct.

3. **Keep the three apart.** A fact can be checked. An assumption is a working guess. An
   opinion is a judgement. Never present them in the same tone.

4. **State your confidence**, and lower it when you extrapolate. Confidence that is
   uniformly high tells the reader nothing.

5. **Name the unknowns.** What you do not know, and that would change your answer, is
   worth more than what you already know.

6. **If you change your mind, say what changed it.** Precisely: which argument, from
   whom. A position that moves without a stated reason is a capitulation, not a
   revision, and it is the most common failure of a debate between models.

7. **Attack arguments, never participants.** And when a position survives your scrutiny,
   say so: that is a result.

8. **Truth before consensus.** Agreement reached by smoothing over a real disagreement
   loses information. If you still disagree, hold your position and say why. This is the
   rule that wins when the rules conflict.
";

/// Le format de sortie exige a chaque tour.
///
/// Les champs STRUCTURES viennent en premier, le texte libre en dernier. Ce n'est pas
/// une question de gout: une reponse tronquee - budget de jetons atteint, modele
/// bavard - perd alors sa prose et garde ses signaux. Dans l'autre sens on perd
/// l'accord et la confiance, c'est-a-dire tout ce dont l'arbitre et l'interface ont
/// besoin. La meme lecon avait ete tiree du format de la scorecard de LaReine.
pub const FORMAT_TOUR: &str = "\
# Format de reponse

Reponds EXACTEMENT dans cet ordre, une entree par ligne, puis ta position en texte
libre a la fin. Les champs courts d'abord: si ta reponse est coupee, mieux vaut perdre
la prose que les signaux.

ACCORD: approuve | reserve | oppose
CONFIANCE: un entier de 0 a 100
CHANGEMENT: ce qui t'a fait changer d'avis depuis ton tour precedent, ou `aucun`
REFUTABLE: ce qui te ferait changer d'avis
HYPOTHESES: separees par des `;`, ou `aucune`
INCONNUES: separees par des `;`, ou `aucune`
POSITION:
<ton raisonnement et ta reponse, en texte libre>

`ACCORD` porte sur la direction generale qui se dessine dans le debat, pas sur ta
propre position: `approuve` si tu t'y rallies, `reserve` si tu la suis avec des
conditions, `oppose` si tu la juges fausse.
";

/// La constitution effective: le socle, ou la surcharge de l'utilisateur si elle
/// existe et n'est pas vide.
///
/// Meme mecanique que `prompt_reine_defaut()`: on peut la reecrire depuis la memoire,
/// mais on ne peut pas la supprimer par accident - une surcharge vide retombe sur le
/// socle plutot que de laisser les specialistes sans regles.
pub fn constitution_effective(surcharge: Option<&str>) -> &str {
    match surcharge {
        Some(s) if !s.trim().is_empty() => s,
        _ => CONSTITUTION,
    }
}

/// Assemble le prompt systeme d'un specialiste: constitution, puis strategie, puis
/// format. Toujours dans cet ordre.
///
/// La constitution en tete, et non en pied: c'est la partie stable, identique pour tous
/// les specialistes et tous les tours. Un prefixe stable est ce qu'un fournisseur peut
/// garder en cache, alors qu'un suffixe change a chaque appel.
pub fn prompt_specialiste(constitution: &str, strategie: &str) -> String {
    format!("{constitution}\n\n# Ta strategie\n\n{strategie}\n\n{FORMAT_TOUR}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_huit_regles_sont_la() {
        for n in 1..=8 {
            assert!(
                CONSTITUTION.contains(&format!("{n}. **")),
                "regle {n} absente"
            );
        }
    }

    #[test]
    fn la_verite_avant_le_consensus_est_la_derniere() {
        // Elle doit l'emporter en cas de conflit, donc etre enoncee en dernier.
        let pos_verite = CONSTITUTION.find("Truth before consensus").unwrap();
        let pos_inventer = CONSTITUTION.find("Never invent").unwrap();
        assert!(pos_verite > pos_inventer);
    }

    #[test]
    fn le_format_met_les_champs_courts_avant_la_prose() {
        let accord = FORMAT_TOUR.find("ACCORD:").unwrap();
        let confiance = FORMAT_TOUR.find("CONFIANCE:").unwrap();
        let position = FORMAT_TOUR.find("POSITION:").unwrap();
        assert!(accord < position, "ACCORD doit preceder POSITION");
        assert!(confiance < position, "CONFIANCE doit preceder POSITION");
    }

    #[test]
    fn une_surcharge_vide_retombe_sur_le_socle() {
        assert_eq!(constitution_effective(None), CONSTITUTION);
        assert_eq!(constitution_effective(Some("   \n  ")), CONSTITUTION);
        assert_eq!(constitution_effective(Some("mes regles")), "mes regles");
    }

    #[test]
    fn la_constitution_est_en_tete_du_prompt() {
        let p = prompt_specialiste("CONST", "STRAT");
        // Prefixe stable en tete: c'est ce qu'un fournisseur peut mettre en cache.
        assert!(p.starts_with("CONST"));
        assert!(p.find("CONST").unwrap() < p.find("STRAT").unwrap());
        assert!(p.find("STRAT").unwrap() < p.find("ACCORD:").unwrap());
    }
}
