/// Build the system prompt for the agent.
///
/// Sections are ordered from stable to volatile to preserve upstream prefix caches:
/// 1. stable identity and behavior,
/// 2. tool capabilities and call format,
/// 3. dynamic/custom context.
/// Assemble le system prompt à partir de sections ÉDITABLES (chargées de la carte cognitive,
/// hot-reload par tour) et de sections VERROUILLÉES (protocole machine-critique, codé en dur).
///
/// - `identity_override` (nœud `system.prompt`) → remplace l'identité par défaut.
/// - `behavior_override` (nœud `system.behavior`) → remplace le comportement par défaut.
/// - `custom_instructions` (nœud `system.soul`) → couche d'instructions additionnelle.
/// - Verrouillé (jamais éditable) : liste d'outils + format `<tool_call>` + format `<plan>`.
///   Éditer ces formats casserait le tool-calling → ils restent dans le code.
pub fn build_system_prompt(
    tools_schema: &serde_json::Value,
    identity_override: Option<&str>,
    behavior_override: Option<&str>,
    capability_index: Option<&str>,
    custom_instructions: Option<&str>,
) -> String {
    let mut prompt = String::new();
    // 1) Identité (éditable) ou défaut codé.
    match identity_override {
        Some(o) if !o.trim().is_empty() => {
            prompt.push_str(o.trim());
            prompt.push_str("\n\n");
        }
        _ => prompt.push_str(&section_identite_stable()),
    }
    // 2) Protocole VERROUILLÉ + outils générés + index de capacités.
    prompt.push_str(&section_outils(tools_schema));
    push_capability_index(&mut prompt, capability_index);
    prompt.push_str(&section_planification());
    // 3) Comportement (éditable) ou défaut codé.
    match behavior_override {
        Some(o) if !o.trim().is_empty() => {
            prompt.push_str(o.trim());
            prompt.push_str("\n\n");
        }
        _ => prompt.push_str(&section_comportement()),
    }
    // 4) Instructions additionnelles (SOUL).
    if let Some(instructions) = custom_instructions {
        prompt.push_str(&section_contexte_dynamique(instructions));
    }
    prompt
}

/// Catalogue compact des capacités (noms par famille) : le LLM sait ce qui EXISTE au-delà des
/// schémas injectés ce tour, et peut tout atteindre via `tool_call`. Stable → cacheable.
fn push_capability_index(prompt: &mut String, index: Option<&str>) {
    if let Some(idx) = index {
        if !idx.trim().is_empty() {
            prompt.push_str(idx);
        }
    }
}

pub fn section_identite_stable() -> String {
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

/// Type court d'un paramètre : enum > tableau typé > primitif abrégé. Format familier au modèle.
fn type_court(spec: &serde_json::Value) -> String {
    if let Some(en) = spec.get("enum").and_then(|v| v.as_array()) {
        let vals: Vec<&str> = en.iter().filter_map(|x| x.as_str()).collect();
        if !vals.is_empty() {
            return vals.join("|");
        }
    }
    fn prim(t: &str) -> &str {
        match t {
            "integer" => "int",
            "boolean" => "bool",
            other => other,
        }
    }
    match spec.get("type").and_then(|v| v.as_str()).unwrap_or("string") {
        "array" => match spec
            .get("items")
            .and_then(|i| i.get("type"))
            .and_then(|v| v.as_str())
        {
            Some(it) => format!("{}[]", prim(it)),
            None => "array".to_string(),
        },
        other => prim(other).to_string(),
    }
}

/// Hint de paramètre conservé UNIQUEMENT s'il porte un FORMAT/EXEMPLE (cron, ISO8601, défaut,
/// slot `{{}}`…). Les descriptions redondantes avec le nom+type (« The URL to fetch ») sont jetées.
fn hint_param(spec: &serde_json::Value) -> Option<String> {
    let d = spec
        .get("description")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())?;
    let porteur = d.chars().any(|c| c.is_ascii_digit())
        || d.contains("ex:")
        || d.contains("ex ")
        || d.contains("ISO")
        || d.contains("{{")
        || d.contains("['")
        || d.contains("défaut")
        || d.contains("defaut");
    if !porteur {
        return None;
    }
    let one = d.split_whitespace().collect::<Vec<_>>().join(" ");
    Some(one.chars().take(60).collect())
}

/// Rend les outils en SIGNATURES compactes (style TypeScript) au lieu de JSON verbeux :
/// `nom(param: type, opt?: type) — description`. ~80% de tokens en moins que le JSON pretty,
/// dans un format que le modèle relie nativement à l'émission d'un `<tool_call>`.
fn signatures_outils(tools: &[serde_json::Value]) -> String {
    let mut out = String::new();
    for t in tools {
        let Some(name) = t
            .get("name")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
        else {
            continue;
        };
        let params = &t["parameters"];
        let req: Vec<&str> = params
            .get("required")
            .and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|x| x.as_str()).collect())
            .unwrap_or_default();
        let mut sig: Vec<String> = Vec::new();
        let mut hints: Vec<String> = Vec::new();
        if let Some(props) = params.get("properties").and_then(|v| v.as_object()) {
            // Paramètres requis d'abord (ordre de `required`), puis les optionnels.
            let mut keys: Vec<&str> = req.iter().copied().filter(|k| props.contains_key(*k)).collect();
            for k in props.keys() {
                if !keys.contains(&k.as_str()) {
                    keys.push(k.as_str());
                }
            }
            for k in keys {
                let spec = &props[k];
                let opt = if req.contains(&k) { "" } else { "?" };
                sig.push(format!("{k}{opt}: {}", type_court(spec)));
                if let Some(h) = hint_param(spec) {
                    hints.push(format!("{k}: {h}"));
                }
            }
        }
        let desc = t
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        let suffixe = if hints.is_empty() {
            String::new()
        } else {
            format!(" {{{}}}", hints.join("; "))
        };
        out.push_str(&format!("- {name}({}) — {desc}{suffixe}\n", sig.join(", ")));
    }
    out
}

fn section_outils(tools_schema: &serde_json::Value) -> String {
    let tools = match tools_schema.as_array() {
        Some(a) if !a.is_empty() => a,
        _ => return String::new(),
    };
    let sigs = signatures_outils(tools);
    format!(
        "## Outils disponibles\n\n\
         Signatures (style TypeScript). `?` = paramètre optionnel ; `a|b` = valeurs autorisées ; \
         `{{…}}` = précision de format. Tu DOIS émettre l'appel en JSON (voir ci-dessous).\n\n\
         ```\n{sigs}```\n\n\
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

pub fn section_comportement() -> &'static str {
    "## Comportement\n\n\
     - Reponds en francais sauf si l'utilisateur parle dans une autre langue.\n\
     - Sois concis et utile.\n\
     - Si tu ne sais pas quelque chose, dis-le honnetement.\n\
     - Pour les taches complexes, decompose en etapes, montre ton plan, et utilise les outils disponibles.\n\
     - Tu peux planifier (cron_create), surveiller (watcher_create), retrouver tes conversations (session_search) et creer tes propres skills.\n\
     - Utilise les outils qui te sont fournis pour ce tour (ils sont selectionnes selon ton intention). Si tu as besoin d'une capacite absente, cherche-la d'abord en memoire.\n\
     - Memorise les faits DURABLES avec memory_write (preferences, decisions, infos persistantes) ; n'enregistre pas le trivial.\n\n\
     ## Auto-amelioration : forge tes SKILLS et tes OUTILS\n\n\
     Tu apprends de tes experiences en les transformant en savoir reutilisable. DEUX choses distinctes :\n\n\
     ### SKILL = une PROCEDURE (le *comment*)\n\
     Un skill documente comment accomplir un type de tache (etapes, pieges, commandes exactes). Il ORCHESTRE des outils existants.\n\
     - QUAND en creer : APRES une tache complexe REUSSIE (>=2 outils enchaines, erreurs surmontees, approche corrigee qui a marche, workflow non-trivial decouvert).\n\
     - COMMENT : `skill_create(name, description, body, tools, scripts)`. Declare dans `tools`/`scripts` ce que le skill utilise (il est ainsi borne). Le corps = procedure pas-a-pas + pieges + commandes.\n\
     - ITERE : si tu utilises un skill et qu'il echoue ou est perime, `skill_patch(name, old, new)` IMMEDIATEMENT pour le corriger. C'est ainsi qu'un skill devient fiable.\n\
     - Scripts bundles : `skill_file_write(skill, path, content)` ecrit un script sous `skills/<nom>/scripts/`, que tu lances ensuite via `shell_exec`/`execute_code`.\n\n\
     ### OUTIL (plugin) = une CAPACITE atomique (le *quoi*)\n\
     Pour une action repetitive atomique (un verbe), forge un outil persistant : `plugin_create(name, description, command, schema, [script_path, script_content])`.\n\
     `command` = template shell avec {{slots}} (ex. `python plugins/scripts/x.py {{arg}}`). Il se recharge tout seul et devient appelable comme une abeille.\n\n\
     ### Regles\n\
     - SKILL pour une procedure, PLUGIN pour une capacite atomique. Ne confonds pas.\n\
     - Tu n'as pas tous les outils listes ce tour : vois le `Catalogue d'outils`, et appelle n'importe lequel via `tool_call` (ou `tool_search` pour chercher).\n\
     - Memorise les FAITS durables avec `memory_write` ; les PROCEDURES avec `skill_create`.\n\n"
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
        let prompt = build_system_prompt(&tools, None, None, None, Some("custom volatile"));

        let env = prompt.find("## Environnement").unwrap();
        let outils = prompt.find("## Outils disponibles").unwrap();
        let custom = prompt.find("## Instructions supplementaires").unwrap();
        assert!(env < outils);
        assert!(outils < custom);
        assert!(prompt.contains("UN SEUL outil"));
    }

    #[test]
    fn signatures_compactes_preservent_enums_et_hints() {
        let tools = serde_json::json!([
            {"name":"web_fetch","description":"Fetch a web page and return clean text.","origin":"builtin",
             "parameters":{"properties":{
                 "url":{"description":"The URL to fetch","type":"string"},
                 "render":{"description":"Si true, rendu headless (pages JS).","type":"boolean"}},
                 "required":["url"],"type":"object"}},
            {"name":"todo","description":"Liste de taches.","origin":"builtin",
             "parameters":{"properties":{
                 "action":{"description":"Action","enum":["add","done","list"],"type":"string"},
                 "id":{"description":"id pour done","type":"integer"}},
                 "required":["action"],"type":"object"}},
            {"name":"cron_create","description":"Crée une tâche planifiée.","origin":"builtin",
             "parameters":{"properties":{
                 "name":{"description":"Nom","type":"string"},
                 "cron_expr":{"description":"Expression cron (ex: '*/5 * * * *')","type":"string"}},
                 "required":["name"],"type":"object"}},
        ]);
        let arr = tools.as_array().unwrap();
        let sigs = signatures_outils(arr);
        // enum rendu inline, optionnel marqué `?`, hint de format conservé, requis sans `?`.
        assert!(sigs.contains("action: add|done|list"));
        assert!(sigs.contains("render?: bool"));
        assert!(sigs.contains("url: string"));
        assert!(sigs.contains("*/5 * * * *"));
        assert!(!sigs.contains("The URL to fetch")); // hint redondant jeté
        // Mesure du gain réel sur cet échantillon.
        let json = serde_json::to_string_pretty(&tools).unwrap();
        eprintln!(
            "[MESURE] JSON pretty: {} car (~{} tok) | signatures: {} car (~{} tok) | gain {:.0}%",
            json.len(),
            json.len() / 4,
            sigs.len(),
            sigs.len() / 4,
            100.0 * (1.0 - sigs.len() as f64 / json.len() as f64)
        );
    }
}
