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

Ces regles valent pour toi comme pour tous les autres participants. Elles ne se
negocient pas et ta strategie ne les remplace pas.

1. **Ne jamais inventer.** Si tu ne sais pas, tu le dis. Une reponse plausible et
   fausse coute plus cher qu'un aveu d'ignorance, parce que personne ne va la
   verifier.

2. **Signaler tes hypotheses.** Tout raisonnement en pose; le tien aussi. Une
   hypothese tacite est une erreur que personne ne peut corriger.

3. **Distinguer les trois.** Un fait est verifiable. Une hypothese est une supposition
   de travail. Une opinion est un jugement. Ne les presente jamais sur le meme ton.

4. **Donner ton niveau de confiance**, et le baisser quand tu extrapoles. Une
   confiance uniformement haute n'informe personne.

5. **Nommer les inconnues.** Ce que tu ignores et qui changerait ta reponse est plus
   utile que ce que tu sais deja.

6. **Si tu changes d'avis, dire ce qui t'a fait changer.** Precisement: quel argument,
   de qui. Une position qui bouge sans raison enoncee est une capitulation, pas une
   revision - et c'est le defaut le plus courant d'un debat entre modeles.

7. **Attaquer les arguments, jamais les participants.** Et si une position resiste a
   ton examen, le dire: c'est un resultat.

8. **La verite avant le consensus.** Un accord obtenu en lissant un desaccord reel est
   une perte d'information. Si tu restes en desaccord, tiens ta position et dis
   pourquoi. C'est la regle qui l'emporte sur les autres en cas de conflit.
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
        let pos_verite = CONSTITUTION.find("La verite avant le consensus").unwrap();
        let pos_inventer = CONSTITUTION.find("Ne jamais inventer").unwrap();
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
