const TRUNCATION_PREFIX: &str = "\n… (tronqué, ";
const TRUNCATION_SUFFIX: &str = " chars omis)";

pub fn tronquer_resultat(output: &str, max: usize) -> String {
    let total = output.chars().count();
    if total <= max {
        return output.to_string();
    }

    let omitted = total - max;
    format!(
        "{}{}{}{}",
        prendre_chars(output, max),
        TRUNCATION_PREFIX,
        omitted,
        TRUNCATION_SUFFIX
    )
}

// `&mut [String]` et non `&mut Vec<String>`: la fonction tronque des chaines en place
// sans jamais ajouter ni retirer d'element, donc elle n'a pas besoin du vecteur.
pub fn appliquer_budget_agregat(outputs: &mut [String], budget_total: usize) {
    if taille_totale(outputs) <= budget_total {
        return;
    }
    if budget_total == 0 {
        outputs.iter_mut().for_each(String::clear);
        return;
    }

    loop {
        let total = taille_totale(outputs);
        if total <= budget_total {
            break;
        }

        let Some((index, taille)) = outputs
            .iter()
            .enumerate()
            .map(|(index, output)| (index, output.chars().count()))
            .max_by_key(|(_, taille)| *taille)
        else {
            break;
        };

        if taille == 0 {
            break;
        }

        let excedent = total - budget_total;
        let cible = taille
            .saturating_sub(excedent)
            .min(taille.saturating_sub(1));
        let reduit = tronquer_pour_taille_finale(&outputs[index], cible);

        if reduit.chars().count() >= taille {
            outputs[index] = prendre_chars(&outputs[index], cible);
        } else {
            outputs[index] = reduit;
        }
    }
}

fn tronquer_pour_taille_finale(output: &str, taille_finale_max: usize) -> String {
    let total = output.chars().count();
    if total <= taille_finale_max {
        return output.to_string();
    }

    for garder in (0..=taille_finale_max.min(total)).rev() {
        let omitted = total - garder;
        let marqueur = marqueur(omitted);
        if garder + marqueur.chars().count() <= taille_finale_max {
            return format!("{}{}", prendre_chars(output, garder), marqueur);
        }
    }

    prendre_chars(&marqueur(total), taille_finale_max)
}

fn marqueur(omitted: usize) -> String {
    format!("{TRUNCATION_PREFIX}{omitted}{TRUNCATION_SUFFIX}")
}

fn taille_totale(outputs: &[String]) -> usize {
    outputs.iter().map(|output| output.chars().count()).sum()
}

fn prendre_chars(output: &str, max: usize) -> String {
    output.chars().take(max).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gros_output_tronque_sur_frontiere_de_char() {
        let output = "é".repeat(20);
        let tronque = tronquer_resultat(&output, 5);

        assert!(tronque.starts_with(&"é".repeat(5)));
        assert!(tronque.contains("15 chars omis"));
        assert!(tronque.is_char_boundary(tronque.len()));
    }

    #[test]
    fn budget_agregat_respecte_le_total() {
        let mut outputs = vec!["a".repeat(80), "b".repeat(40), "petit".to_string()];

        appliquer_budget_agregat(&mut outputs, 70);

        let total: usize = outputs.iter().map(|output| output.chars().count()).sum();
        assert!(total <= 70);
        assert!(outputs.iter().any(|output| output.contains("tronqué")));
    }

    #[test]
    fn petits_outputs_restent_intacts_si_budget_suffisant() {
        let mut outputs = vec!["alpha".to_string(), "beta".to_string()];

        appliquer_budget_agregat(&mut outputs, 20);

        assert_eq!(outputs, vec!["alpha".to_string(), "beta".to_string()]);
    }
}
