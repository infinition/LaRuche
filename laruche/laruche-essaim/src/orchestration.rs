//! Orchestration : helpers PURS pour l'injection de skills (Lot 10.B) et la
//! boucle orchestrateur kanban (Lot 11.B). Sans dépendance à laruche-memoire ni
//! laruche-kanban → compile dès maintenant ; l'intégration (daemon cron / brain)
//! appelle ces fonctions une fois les skills chargés / les tâches lues.

/// Assemble le contenu de skills OKF en tête d'un prompt (10.B).
/// `skills` : paires `(nom, corps_markdown)` déjà chargées depuis la mémoire OKF
/// (`capacities.skills.<slug>`). Le corps inclut idéalement le frontmatter retiré et
/// seulement la connaissance procédurale. Ordre préservé.
pub fn assembler_prompt_skills(base_prompt: &str, skills: &[(String, String)]) -> String {
    if skills.is_empty() {
        return base_prompt.to_string();
    }
    let mut out = String::new();
    out.push_str("# Compétences activées pour cette tâche\n\n");
    for (name, body) in skills {
        out.push_str(&format!(
            "## Skill : {}\n{}\n\n---\n\n",
            name.trim(),
            body.trim()
        ));
    }
    out.push_str(base_prompt);
    out
}

/// Vue minimale d'une tâche kanban pour la sélection orchestrateur (11.B).
/// Découplée des types de `laruche-kanban` : le daemon mappe ses tâches dessus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TacheLite {
    pub id: String,
    /// "todo" | "ready" | "blocked" | "done" | "archived" (insensible à la casse).
    pub status: String,
    /// Ids des tâches dont celle-ci dépend.
    pub blocked_by: Vec<String>,
}

fn est_terminee(status: &str) -> bool {
    let s = status.to_lowercase();
    s == "done" || s == "archived"
}

/// Renvoie l'id de la prochaine tâche exécutable par l'orchestrateur, ou `None`
/// si le board est vide / tout est terminé ou bloqué.
///
/// Règle : on prend la 1re tâche `ready`, ou une `todo`/`blocked` dont TOUTES les
/// dépendances sont terminées (auto-déblocage logique, cohérent avec
/// `KanbanBoard::change_status`). On ignore les tâches terminées/archivées.
pub fn prochaine_tache_ready(taches: &[TacheLite]) -> Option<String> {
    let terminees: std::collections::HashSet<&str> = taches
        .iter()
        .filter(|t| est_terminee(&t.status))
        .map(|t| t.id.as_str())
        .collect();

    // Priorité aux tâches explicitement "ready".
    if let Some(t) = taches
        .iter()
        .find(|t| t.status.eq_ignore_ascii_case("ready"))
    {
        return Some(t.id.clone());
    }
    // Sinon une tâche non terminée dont toutes les dépendances sont satisfaites.
    taches
        .iter()
        .find(|t| {
            !est_terminee(&t.status)
                && !t.status.eq_ignore_ascii_case("archived")
                && t.blocked_by
                    .iter()
                    .all(|dep| terminees.contains(dep.as_str()))
        })
        .map(|t| t.id.clone())
}

/// `true` s'il reste au moins une tâche à exécuter (utile pour borner la boucle).
pub fn board_a_du_travail(taches: &[TacheLite]) -> bool {
    taches.iter().any(|t| !est_terminee(&t.status))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assemblage_prefixe_les_skills_dans_l_ordre() {
        let skills = vec![
            ("recherche-web".into(), "Utilise web_deep_search.".into()),
            ("synthese".into(), "Resume en 5 points.".into()),
        ];
        let p = assembler_prompt_skills("Fais la veille IA.", &skills);
        let i1 = p.find("recherche-web").unwrap();
        let i2 = p.find("synthese").unwrap();
        let ib = p.find("Fais la veille IA").unwrap();
        assert!(i1 < i2 && i2 < ib, "skills avant le prompt, dans l'ordre");
    }

    #[test]
    fn assemblage_sans_skill_renvoie_le_prompt() {
        assert_eq!(assembler_prompt_skills("X", &[]), "X");
    }

    #[test]
    fn selection_priorise_ready() {
        let t = vec![
            TacheLite {
                id: "a".into(),
                status: "todo".into(),
                blocked_by: vec![],
            },
            TacheLite {
                id: "b".into(),
                status: "ready".into(),
                blocked_by: vec![],
            },
        ];
        assert_eq!(prochaine_tache_ready(&t).as_deref(), Some("b"));
    }

    #[test]
    fn selection_respecte_les_dependances() {
        let t = vec![
            TacheLite {
                id: "parent".into(),
                status: "todo".into(),
                blocked_by: vec![],
            },
            TacheLite {
                id: "enfant".into(),
                status: "blocked".into(),
                blocked_by: vec!["parent".into()],
            },
        ];
        // parent d'abord (pas de dépendance), enfant encore bloqué.
        assert_eq!(prochaine_tache_ready(&t).as_deref(), Some("parent"));

        let t2 = vec![
            TacheLite {
                id: "parent".into(),
                status: "done".into(),
                blocked_by: vec![],
            },
            TacheLite {
                id: "enfant".into(),
                status: "blocked".into(),
                blocked_by: vec!["parent".into()],
            },
        ];
        // parent terminé → enfant devient exécutable.
        assert_eq!(prochaine_tache_ready(&t2).as_deref(), Some("enfant"));
    }

    #[test]
    fn board_vide_ou_termine() {
        assert!(!board_a_du_travail(&[]));
        let t = vec![TacheLite {
            id: "a".into(),
            status: "done".into(),
            blocked_by: vec![],
        }];
        assert!(!board_a_du_travail(&t));
        assert_eq!(prochaine_tache_ready(&t), None);
    }
}
