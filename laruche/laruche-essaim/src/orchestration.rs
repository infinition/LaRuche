//! Orchestration: PURE helpers for skill injection (Lot 10.B) and the kanban
//! orchestrator loop (Lot 11.B). No dependency on laruche-memoire or
//! laruche-kanban, so it compiles right now; the integration (cron daemon / brain)
//! calls these functions once skills are loaded / tasks are read.

/// Assembles OKF skill content at the top of a prompt (10.B).
/// `skills`: `(name, markdown_body)` pairs already loaded from OKF memory
/// (`capacities.skills.<slug>`). The body ideally has the frontmatter stripped and
/// keeps only procedural knowledge. Order is preserved.
pub fn assembler_prompt_skills(base_prompt: &str, skills: &[(String, String)]) -> String {
    if skills.is_empty() {
        return base_prompt.to_string();
    }
    let mut out = String::new();
    out.push_str("# Skills activated for this task\n\n");
    for (name, body) in skills {
        // Hint: explicitly surface the tools/plugins declared useful
        // for this skill (frontmatter `tools:`/`allowed-tools:`), even when the frontmatter
        // is stripped from the body, so the model knows WHICH tools to prefer for this skill.
        let outils = extraire_outils_skill(body);
        let hint = if outils.is_empty() {
            String::new()
        } else {
            format!(
                "**Recommended tools/plugins for this skill: {}**\n\n",
                outils.join(", ")
            )
        };
        out.push_str(&format!(
            "## Skill: {}\n{}{}\n\n---\n\n",
            name.trim(),
            hint,
            body.trim()
        ));
    }
    out.push_str(base_prompt);
    out
}

/// Extracts the list of tools declared in an OKF skill's frontmatter
/// (`tools: [a, b]` or `allowed-tools: [a, b]`). Empty if absent.
pub fn extraire_outils_skill(body: &str) -> Vec<String> {
    for ligne in body.lines() {
        let l = ligne.trim();
        let reste = l
            .strip_prefix("tools:")
            .or_else(|| l.strip_prefix("allowed-tools:"));
        if let Some(reste) = reste {
            let reste = reste.trim().trim_start_matches('[').trim_end_matches(']');
            let outils: Vec<String> = reste
                .split(',')
                .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string())
                .filter(|s| !s.is_empty())
                .collect();
            return outils;
        }
    }
    Vec::new()
}

/// Minimal view of a kanban task for orchestrator selection (11.B).
/// Decoupled from `laruche-kanban` types: the daemon maps its tasks onto it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TacheLite {
    pub id: String,
    /// "todo" | "ready" | "blocked" | "done" | "archived" (case-insensitive).
    pub status: String,
    /// Ids of the tasks this one depends on.
    pub blocked_by: Vec<String>,
}

fn est_terminee(status: &str) -> bool {
    let s = status.to_lowercase();
    s == "done" || s == "archived"
}

/// Returns the id of the next task the orchestrator can run, or `None`
/// if the board is empty / everything is done or blocked.
///
/// Rule: take the first `ready` task, or a `todo`/`blocked` task whose ALL
/// dependencies are done (logical auto-unblocking, consistent with
/// `KanbanBoard::change_status`). Done/archived tasks are ignored.
pub fn prochaine_tache_ready(taches: &[TacheLite]) -> Option<String> {
    let terminees: std::collections::HashSet<&str> = taches
        .iter()
        .filter(|t| est_terminee(&t.status))
        .map(|t| t.id.as_str())
        .collect();

    // Priority to explicitly "ready" tasks.
    if let Some(t) = taches
        .iter()
        .find(|t| t.status.eq_ignore_ascii_case("ready"))
    {
        return Some(t.id.clone());
    }
    // Otherwise a non-done task whose dependencies are all satisfied.
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

/// `true` if at least one task remains to run (useful to bound the loop).
pub fn board_a_du_travail(taches: &[TacheLite]) -> bool {
    taches.iter().any(|t| !est_terminee(&t.status))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extrait_les_outils_du_frontmatter_skill() {
        let body = "---\ntype: skill\ntools: [web_search, web_fetch]\n---\n# Proc";
        assert_eq!(extraire_outils_skill(body), vec!["web_search", "web_fetch"]);
        assert!(extraire_outils_skill("# pas de frontmatter").is_empty());
        // the hint must appear in the assembly
        let out = assembler_prompt_skills("BASE", &[("meteo".into(), body.to_string())]);
        assert!(out.contains("Recommended tools/plugins for this skill: web_search, web_fetch"));
    }

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
        assert!(i1 < i2 && i2 < ib, "skills before the prompt, in order");
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
        // parent first (no dependency), child still blocked.
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
        // parent done, child becomes runnable.
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
