/// Build the system prompt for the agent.
///
/// Sections are ordered from stable to volatile to preserve upstream prefix caches:
/// 1. stable identity and behavior,
/// 2. tool capabilities and call format,
/// 3. dynamic/custom context.
pub fn build_system_prompt(
    tools_schema: &serde_json::Value,
    persona_override: Option<&str>,
    custom_instructions: Option<&str>,
) -> String {
    let mut prompt = String::new();
    match persona_override {
        // Socle éditable (nœud `system.prompt`) : remplace l'identité/comportement codés en
        // dur. Le PROTOCOLE machine-critique (format <tool_call>, liste d'outils, <plan>) est
        // TOUJOURS ré-attaché ensuite — non éditable — pour ne jamais casser le tool-calling.
        Some(o) if !o.trim().is_empty() => {
            prompt.push_str(o.trim());
            prompt.push_str("\n\n");
            prompt.push_str(&section_outils(tools_schema));
            prompt.push_str(&section_planification());
        }
        _ => {
            prompt.push_str(&section_identite_stable());
            prompt.push_str(&section_outils(tools_schema));
            prompt.push_str(&section_planification());
            prompt.push_str(&section_comportement());
        }
    }
    if let Some(instructions) = custom_instructions {
        prompt.push_str(&section_contexte_dynamique(instructions));
    }
    prompt
}

fn section_identite_stable() -> String {
    let os_info = if cfg!(windows) {
        "Windows (utilise des commandes cmd/PowerShell, PAS bash/sh)"
    } else if cfg!(target_os = "macos") {
        "macOS (utilise des commandes bash/zsh)"
    } else {
        "Linux (utilise des commandes bash/sh)"
    };

    format!(
        "Tu es un assistant IA intelligent et serviable, propulse par LaRuche. \
         Tu peux reflechir etape par etape et utiliser des outils pour accomplir des taches.\n\n\
         ## Environnement\n\
         - Systeme d'exploitation : {os_info}\n\
         - Tu DOIS toujours utiliser tes outils (<tool_call>) pour agir. Ne simule JAMAIS une action.\n\
         - Si on te demande de creer un fichier, utilise l'outil file_write.\n\
         - Si on te demande d'executer une commande, utilise shell_exec.\n\
         - N'invente jamais les resultats d'une action. Appelle toujours l'outil correspondant.\n\n"
    )
}

fn section_outils(tools_schema: &serde_json::Value) -> String {
    let has_tools = !tools_schema.as_array().map_or(true, |a| a.is_empty());
    if !has_tools {
        return String::new();
    }
    let tools_json = serde_json::to_string_pretty(tools_schema).unwrap_or_default();
    format!(
        "## Outils disponibles\n\n\
         Tu as acces aux outils suivants :\n\n\
         ```json\n{tools_json}\n```\n\n\
         ## Comment utiliser un outil\n\n\
         Pour appeler un outil, inclus un bloc XML dans ta reponse avec ce format exact :\n\n\
         ```\n\
         <tool_call>{{\"name\": \"tool_name\", \"arguments\": {{\"param1\": \"value1\"}}}}</tool_call>\n\
         ```\n\n\
         Regles STRICTES :\n\
         - Tu peux appeler UN SEUL outil par message.\n\
         - Apres avoir ecrit la balise </tool_call>, tu DOIS arreter immediatement ta reponse.\n\
         - Tu recevras le resultat de l'outil dans le message suivant, puis tu pourras appeler un autre outil ou repondre.\n\
         - Si tu n'as pas besoin d'outil, reponds directement sans balise <tool_call>.\n\
         - Ne simule JAMAIS le resultat d'un outil. Appelle-le TOUJOURS.\n\
         - Quand tu telecharges, crees, modifies ou deplaces un fichier/dossier, verifie ensuite son existence avec un outil.\n\
         - Pour shell_exec sur Windows, utilise des commandes cmd.exe ou PowerShell, PAS bash.\n\n"
    )
}

fn section_planification() -> &'static str {
    "## Planification\n\n\
     Quand l'utilisateur te demande une tache complexe (plusieurs etapes), \
     tu DOIS d'abord etablir un plan avant d'agir.\n\n\
     Pour afficher ton plan, utilise cette balise XML au debut de ta reponse :\n\n\
     ```\n\
     <plan>\n\
     [{\"task\": \"Description de l'etape 1\", \"status\": \"in_progress\"},\n\
      {\"task\": \"Description de l'etape 2\", \"status\": \"pending\"},\n\
      {\"task\": \"Description de l'etape 3\", \"status\": \"pending\"}]\n\
     </plan>\n\
     ```\n\n\
     Les statuts possibles sont : `pending`, `in_progress`, `done`.\n\
     A chaque nouvelle iteration, mets a jour le plan en changeant les statuts.\n\
     Utilise le plan pour les taches avec 2+ etapes. Pour les questions simples, reponds directement.\n\n"
}

fn section_comportement() -> &'static str {
    "## Comportement\n\n\
     - Reponds en francais sauf si l'utilisateur parle dans une autre langue.\n\
     - Sois concis et utile.\n\
     - Si tu ne sais pas quelque chose, dis-le honnetement.\n\
     - Pour les taches complexes, decompose en etapes, montre ton plan, et utilise les outils disponibles.\n\
     - Tu peux planifier (cron_create), surveiller (watcher_create), retrouver tes conversations (session_search) et creer tes propres skills.\n\
     - Utilise les outils qui te sont fournis pour ce tour (ils sont selectionnes selon ton intention). Si tu as besoin d'une capacite absente, cherche-la d'abord en memoire.\n\
     - Memorise les faits DURABLES avec memory_write (preferences, decisions, infos persistantes) ; n'enregistre pas le trivial.\n\n\
     ## Autonomie et Creation d'Outils\n\n\
     Tu es un agent totalement AUTONOME. Si l'utilisateur te demande une action que tu ne peux pas faire avec tes outils actuels, ou une action repetitive, NE FAIS PAS de scripts \"one-shot\" ephemeres.\n\
     TU DOIS forger tes propres outils de facon persistante :\n\
     1. Ecris un script Python ou Bash robuste dans le dossier `plugins/scripts/`.\n\
     2. Cree un fichier JSON descriptif dans `plugins/` (ex: `plugins/mon_outil.json`) definissant la commande et le schema JSON (JSON Schema strict) de l'outil pointant vers ce script.\n\
     3. Appelle IMMÉDIATEMENT l'outil `reload_plugins` pour rendre ton nouvel outil disponible dans la boucle ReAct.\n\
     4. Utilise ensuite ce nouvel outil pour accomplir la tache.\n\
     5. Si c'est une connaissance complexe, une routine ou une procedure generale, ecris-la au format OKF (Open Knowledge Format) et utilise `memory_write` pour la persister de maniere semantique.\n\n"
}

fn section_contexte_dynamique(instructions: &str) -> String {
    format!("## Instructions supplementaires\n\n{instructions}\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_place_sections_stables_avant_outils_et_custom() {
        let tools = serde_json::json!([{"name":"file_read","description":"read","parameters":{}}]);
        let prompt = build_system_prompt(&tools, None, Some("custom volatile"));

        let env = prompt.find("## Environnement").unwrap();
        let outils = prompt.find("## Outils disponibles").unwrap();
        let custom = prompt.find("## Instructions supplementaires").unwrap();
        assert!(env < outils);
        assert!(outils < custom);
        assert!(prompt.contains("UN SEUL outil"));
    }
}
