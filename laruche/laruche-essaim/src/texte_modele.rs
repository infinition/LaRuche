//! Nettoyage du texte tel que le modele l'ecrit, avant stockage et affichage.
//!
//! Certains modeles - deepseek-v4-flash le fait systematiquement - ecrivent leurs
//! emoji sous forme de sequences d'echappement UTF-16 au lieu du caractere:
//!
//! ```text
//! Bien le bonjour depuis la ruche 🐝✨
//! ```
//!
//! Ce n'est pas un defaut d'encodage de notre cote: dans la meme phrase, un tiret
//! cadratin arrive en vrai caractere. Le modele raisonne comme s'il ecrivait a
//! l'interieur d'une chaine JSON, probablement parce que tout son dialogue d'outils
//! en est fait. On decode donc a la reception, une fois, pour que le chat, le fil,
//! la memoire et les canaux voient tous la meme chose.

/// Decode les sequences `\uXXXX` litterales, paires de substitution comprises.
///
/// Le contenu des blocs de code est laisse intact: c'est le seul endroit ou un
/// `\uXXXX` ecrit tel quel est probablement voulu - un exemple de code, une
/// explication sur l'echappement. Remplacer la dedans transformerait une reponse
/// juste en reponse fausse.
///
/// Toute sequence qui ne forme pas un caractere valide est laissee telle quelle:
/// mieux vaut un echappement visible qu'un remplacement silencieux par `?`.
pub fn decoder_echappements(texte: &str) -> String {
    if !texte.contains("\\u") {
        return texte.to_string();
    }

    let octets: Vec<char> = texte.chars().collect();
    let mut sortie = String::with_capacity(texte.len());
    let mut i = 0usize;
    let mut dans_bloc = false;
    let mut dans_inline = false;

    while i < octets.len() {
        // Cloture de code: ``` en debut de ligne ouvre ou ferme un bloc.
        if octets[i] == '`' {
            let triple = octets.get(i + 1) == Some(&'`') && octets.get(i + 2) == Some(&'`');
            if triple {
                dans_bloc = !dans_bloc;
                sortie.push_str("```");
                i += 3;
                continue;
            }
            // Un backtick simple bascule le code en ligne, hors bloc.
            if !dans_bloc {
                dans_inline = !dans_inline;
            }
            sortie.push('`');
            i += 1;
            continue;
        }

        if !dans_bloc && !dans_inline && octets[i] == '\\' && octets.get(i + 1) == Some(&'u') {
            if let Some((c, consommes)) = lire_echappement(&octets[i..]) {
                sortie.push(c);
                i += consommes;
                continue;
            }
        }

        sortie.push(octets[i]);
        i += 1;
    }
    sortie
}

/// Lit `\uXXXX` (6 caracteres) ou une paire haute+basse (12), et rend le caractere
/// obtenu avec le nombre de caracteres consommes.
fn lire_echappement(reste: &[char]) -> Option<(char, usize)> {
    let point = |depart: usize| -> Option<u32> {
        if reste.len() < depart + 6 {
            return None;
        }
        if reste[depart] != '\\' || reste[depart + 1] != 'u' {
            return None;
        }
        let hex: String = reste[depart + 2..depart + 6].iter().collect();
        if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        u32::from_str_radix(&hex, 16).ok()
    };

    let premier = point(0)?;

    // Substitution haute: seule elle ne vaut rien, il lui faut sa basse.
    if (0xD800..=0xDBFF).contains(&premier) {
        let second = point(6)?;
        if !(0xDC00..=0xDFFF).contains(&second) {
            return None;
        }
        let combine = 0x10000 + ((premier - 0xD800) << 10) + (second - 0xDC00);
        return char::from_u32(combine).map(|c| (c, 12));
    }

    // Une substitution basse orpheline est invalide: on n'y touche pas.
    if (0xDC00..=0xDFFF).contains(&premier) {
        return None;
    }
    char::from_u32(premier).map(|c| (c, 6))
}

#[cfg(test)]
mod tests {
    use super::decoder_echappements;

    #[test]
    fn decode_une_paire_de_substitution() {
        // Le cas reel: l'abeille, telle que deepseek l'ecrit.
        assert_eq!(
            decoder_echappements("depuis la ruche \\uD83D\\uDC1D\\u2728"),
            "depuis la ruche \u{1F41D}\u{2728}"
        );
    }

    #[test]
    fn decode_le_plan_de_base() {
        assert_eq!(decoder_echappements("caf\\u00e9"), "café");
    }

    #[test]
    fn laisse_le_texte_ordinaire_intact() {
        let t = "Une phrase — avec un vrai tiret et 🐝 un vrai emoji.";
        assert_eq!(decoder_echappements(t), t);
    }

    #[test]
    fn ne_touche_pas_aux_blocs_de_code() {
        // Un exemple de code qui PARLE d'echappement doit survivre tel quel.
        let t = "Voici:\n```python\nprint(\"\\uD83D\\uDC1D\")\n```\nfin";
        assert_eq!(decoder_echappements(t), t);
    }

    #[test]
    fn ne_touche_pas_au_code_en_ligne() {
        let t = "ecris `\\u2728` pour l'etoile";
        assert_eq!(decoder_echappements(t), t);
    }

    #[test]
    fn laisse_les_sequences_invalides() {
        // Substitution haute orpheline, basse orpheline, hex incomplet: rien ne bouge.
        for t in ["\\uD83D seul", "\\uDC1D seul", "\\u12 court", "\\uZZZZ"] {
            assert_eq!(decoder_echappements(t), t, "cas: {t}");
        }
    }

    #[test]
    fn decode_apres_un_bloc_ferme() {
        assert_eq!(
            decoder_echappements("```\n\\u2728\n```\npuis \\u2728"),
            "```\n\\u2728\n```\npuis \u{2728}"
        );
    }

    #[test]
    fn sans_echappement_le_texte_est_rendu_tel_quel() {
        let t = "rien a faire ici";
        assert_eq!(decoder_echappements(t), t);
    }
}
