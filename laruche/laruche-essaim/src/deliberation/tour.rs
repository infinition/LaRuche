//! Ce qu'un specialiste rend a chaque tour, et comment on le lit.
//!
//! Les indicateurs que l'interface montre a cote des avatars viennent d'ICI: ils sont
//! **declares par le specialiste**, jamais deduits par nous. La distinction compte. Un
//! accord que nous inferions de son texte serait notre interpretation presentee comme
//! son avis; un accord qu'il ecrit est verifiable, et il peut avoir tort de façon
//! visible.

use serde::{Deserialize, Serialize};

/// Position d'un specialiste vis-a-vis de la direction qui se dessine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Accord {
    /// Se rallie.
    Approuve,
    /// Suit, avec des conditions.
    #[default]
    Reserve,
    /// Juge la direction fausse.
    Oppose,
}

impl Accord {
    /// Le symbole affiche a cote de l'avatar.
    pub fn symbole(&self) -> &'static str {
        match self {
            Accord::Approuve => "✔",
            Accord::Reserve => "⚠",
            Accord::Oppose => "✖",
        }
    }
}

/// Une intervention, telle qu'elle sera stockee et affichee.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Intervention {
    pub specialiste: String,
    pub tour: u8,
    pub accord: Accord,
    /// 0 a 100, tel que declare.
    pub confiance: u8,
    /// Ce qui l'a fait changer d'avis. Vide = position inchangee.
    #[serde(default)]
    pub changement: String,
    /// Ce qui le ferait changer d'avis. Exige d'un contradicteur.
    #[serde(default)]
    pub refutable: String,
    #[serde(default)]
    pub hypotheses: Vec<String>,
    #[serde(default)]
    pub inconnues: Vec<String>,
    pub position: String,
    /// Jetons consommes, pour que le plafond de l'orchestrateur soit tenable.
    #[serde(default)]
    pub jetons: u32,
}

impl Intervention {
    /// A-t-il bouge a ce tour ?
    ///
    /// C'est le signal le plus utile du dispositif: un debat ou plus personne ne bouge
    /// est un debat termine, et un specialiste qui bouge a chaque tour sans jamais dire
    /// pourquoi est un specialiste qui capitule.
    pub fn a_change(&self) -> bool {
        let c = self.changement.trim().to_lowercase();
        !c.is_empty() && c != "aucun" && c != "aucune" && c != "-" && c != "non"
    }
}

/// Lit la reponse d'un specialiste.
///
/// Tolerant par construction: un champ absent prend une valeur par defaut plutot que de
/// faire echouer le tour. Un modele qui oublie `CONFIANCE` a quand meme dit quelque
/// chose d'utile, et perdre son intervention entiere pour un champ manquant serait le
/// pire des arbitrages.
///
/// En revanche `POSITION` est obligatoire: sans texte, il n'y a pas d'intervention. A
/// defaut de marqueur, on prend tout ce qui ne ressemble pas a un champ - un modele qui
/// ignore le format a souvent quand meme raisonne.
pub fn lire(specialiste: &str, tour: u8, brut: &str) -> Option<Intervention> {
    let mut accord = Accord::default();
    let mut confiance: u8 = 50;
    let mut changement = String::new();
    let mut refutable = String::new();
    let mut hypotheses = Vec::new();
    let mut inconnues = Vec::new();
    let mut position = String::new();
    let mut dans_position = false;
    let mut hors_champs: Vec<&str> = Vec::new();

    for ligne in brut.lines() {
        let l = ligne.trim();
        // Une fois dans POSITION, tout le reste est du texte: un modele peut y ecrire
        // « CONFIANCE » dans une phrase sans qu'on doive le relire comme un champ.
        if dans_position {
            position.push_str(ligne);
            position.push('\n');
            continue;
        }
        let sans_gras = l.trim_start_matches(['#', '*', '-', ' ']);
        let (cle, valeur) = match sans_gras.split_once(':') {
            Some((c, v)) => (
                c.trim().trim_matches('*').to_uppercase(),
                // Le gras Markdown se referme APRES le deux-points (`**ACCORD:** oppose`),
                // donc la valeur commence souvent par les asterisques de fermeture.
                v.trim().trim_start_matches(['*', ' ']).trim().to_string(),
            ),
            None => {
                if !l.is_empty() {
                    hors_champs.push(ligne);
                }
                continue;
            }
        };
        match cle.as_str() {
            "ACCORD" => {
                let v = valeur.to_lowercase();
                accord = if v.starts_with("approuve") {
                    Accord::Approuve
                } else if v.starts_with("oppose") {
                    Accord::Oppose
                } else {
                    Accord::Reserve
                };
            }
            "CONFIANCE" => {
                // On prend le premier nombre trouve: « 80 % », « 80/100 » et « 80 »
                // veulent tous dire la meme chose.
                let chiffres: String = valeur.chars().take_while(|c| c.is_ascii_digit()).collect();
                if let Ok(n) = chiffres.parse::<u32>() {
                    confiance = n.min(100) as u8;
                }
            }
            "CHANGEMENT" => changement = valeur,
            "REFUTABLE" => refutable = valeur,
            "HYPOTHESES" | "HYPOTHÈSES" => hypotheses = decouper(&valeur),
            "INCONNUES" => inconnues = decouper(&valeur),
            "POSITION" => {
                dans_position = true;
                if !valeur.is_empty() {
                    position.push_str(&valeur);
                    position.push('\n');
                }
            }
            _ => {
                if !l.is_empty() {
                    hors_champs.push(ligne);
                }
            }
        }
    }

    if position.trim().is_empty() {
        // Pas de marqueur POSITION: on recupere ce qui n'etait pas un champ. Un modele
        // qui ignore le format a souvent quand meme raisonne, et jeter sa reponse pour
        // un marqueur manquant serait absurde.
        //
        // Mais on exige que ca ressemble a une phrase. Une reponse coupee en plein nom
        // de champ laisse un fragment comme `CHANGEM`, qui n'est pas une position: le
        // prendre pour telle transformerait une troncature en intervention vide.
        let recolte = hors_champs.join("\n");
        let ressemble_a_une_phrase =
            recolte.contains(' ') && recolte.trim().chars().count() >= 12;
        if ressemble_a_une_phrase {
            position = recolte;
        }
    }
    if position.trim().is_empty() {
        return None;
    }

    Some(Intervention {
        specialiste: specialiste.to_string(),
        tour,
        accord,
        confiance,
        changement,
        refutable,
        hypotheses,
        inconnues,
        position: position.trim().to_string(),
        jetons: 0,
    })
}

fn decouper(v: &str) -> Vec<String> {
    let bas = v.trim().to_lowercase();
    if bas.is_empty() || bas == "aucune" || bas == "aucun" || bas == "-" {
        return Vec::new();
    }
    v.split(';')
        .map(|s| s.trim().trim_start_matches(['-', '*', ' ']).to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const COMPLET: &str = "\
ACCORD: reserve
CONFIANCE: 72
CHANGEMENT: l'argument de l'ingenieur sur la latence a 10k connexions
REFUTABLE: un benchmark montrant moins de 50 ms au p99
HYPOTHESES: le reseau est fiable ; la charge est uniforme
INCONNUES: le cout reel du chiffrement
POSITION:
Il faut mesurer avant de choisir.
Deuxieme ligne.";

    #[test]
    fn lit_tous_les_champs() {
        let i = lire("scientifique", 2, COMPLET).unwrap();
        assert_eq!(i.accord, Accord::Reserve);
        assert_eq!(i.confiance, 72);
        assert!(i.a_change());
        assert!(i.refutable.contains("benchmark"));
        assert_eq!(i.hypotheses.len(), 2);
        assert_eq!(i.inconnues.len(), 1);
        assert!(i.position.starts_with("Il faut mesurer"));
        assert!(i.position.contains("Deuxieme ligne"));
        assert_eq!(i.tour, 2);
    }

    #[test]
    fn aucun_changement_veut_dire_position_stable() {
        let brut = "ACCORD: approuve\nCONFIANCE: 90\nCHANGEMENT: aucun\nPOSITION:\nRien a ajouter.";
        let i = lire("x", 1, brut).unwrap();
        assert!(!i.a_change());
        assert_eq!(i.accord, Accord::Approuve);
    }

    #[test]
    fn tolere_le_gras_et_les_puces() {
        let brut = "**ACCORD:** oppose\n- **CONFIANCE:** 30/100\nPOSITION:\nNon.";
        let i = lire("x", 1, brut).unwrap();
        assert_eq!(i.accord, Accord::Oppose);
        assert_eq!(i.confiance, 30);
    }

    #[test]
    fn une_reponse_tronquee_garde_ses_signaux() {
        // Coupee juste apres CONFIANCE: c'est tout l'interet des champs courts en tete.
        let brut = "ACCORD: oppose\nCONFIANCE: 20\nCHANGEM";
        // Sans POSITION ni texte libre exploitable, il n'y a pas d'intervention...
        assert!(lire("x", 1, brut).is_none());
        // ...mais des qu'une bribe de texte existe, l'accord et la confiance sont sauves.
        let brut2 = "ACCORD: oppose\nCONFIANCE: 20\nCe systeme ne tiendra pas la charge.";
        let i = lire("x", 1, brut2).unwrap();
        assert_eq!(i.accord, Accord::Oppose);
        assert_eq!(i.confiance, 20);
        assert!(i.position.contains("tiendra pas"));
    }

    #[test]
    fn sans_format_le_texte_est_quand_meme_recupere() {
        let brut = "Je pense que cette approche est correcte, pour trois raisons.";
        let i = lire("x", 1, brut).unwrap();
        assert!(i.position.contains("trois raisons"));
        // Valeurs par defaut prudentes: ni accord, ni confiance elevee.
        assert_eq!(i.accord, Accord::Reserve);
        assert_eq!(i.confiance, 50);
    }

    #[test]
    fn le_mot_confiance_dans_la_prose_n_est_pas_relu_comme_un_champ() {
        let brut = "ACCORD: approuve\nCONFIANCE: 80\nPOSITION:\nMa CONFIANCE: elle est haute.";
        let i = lire("x", 1, brut).unwrap();
        assert_eq!(i.confiance, 80);
        assert!(i.position.contains("elle est haute"));
    }

    #[test]
    fn une_reponse_vide_ne_produit_rien() {
        assert!(lire("x", 1, "").is_none());
        assert!(lire("x", 1, "   \n  \n").is_none());
    }

    #[test]
    fn les_symboles_correspondent_aux_accords() {
        assert_eq!(Accord::Approuve.symbole(), "✔");
        assert_eq!(Accord::Reserve.symbole(), "⚠");
        assert_eq!(Accord::Oppose.symbole(), "✖");
    }
}
