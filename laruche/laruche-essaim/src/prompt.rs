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
    planning_override: Option<&str>,
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
    match planning_override {
        Some(o) if !o.trim().is_empty() => {
            prompt.push_str(o.trim());
            prompt.push_str("\n\n");
        }
        _ => prompt.push_str(section_planification()),
    }
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
    // 5) Secrets : on expose les NOMS (jamais les valeurs). Le LLM les référence par `${NOM}`
    //    dans les commandes shell/scripts ; le node substitue la vraie valeur à l'exécution.
    let noms = crate::secrets::noms();
    if !noms.is_empty() {
        prompt.push_str(&format!(
            "\n## Available secrets\nThe user has stored secrets (API keys, tokens, webhook URLs). \
             You NEVER know their value — only the name. To use one in a shell_exec command, a \
             script or a URL, write `${{NAME}}` OR `@@NAME` (short form; the user often writes it \
             this way, e.g. `@@webhook_test1`): the system substitutes the real value at execution \
             time (never displayed). If the user writes `@@NAME`, it's a reference to that secret — \
             pass it through to the tool as-is (don't try to guess it).\n\
             Secrets: {}\n\n",
            noms.join(", ")
        ));
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
        "Windows (use cmd/PowerShell commands, NOT bash/sh)"
    } else if cfg!(target_os = "macos") {
        "macOS (use bash/zsh commands)"
    } else {
        "Linux (use bash/sh commands)"
    };

    format!(
        "You are an intelligent, helpful AI assistant powered by LaRuche. \
         You can reason step by step and use tools to accomplish tasks.\n\n\
         ## Environment\n\
         - Operating system: {os_info}\n\
         - You MUST always use your tools (<tool_call>) to act. NEVER simulate an action.\n\
         - If asked to create a file, use the file_write tool.\n\
         - If asked to run a command, use shell_exec.\n\
         - Never invent the result of an action. Always call the matching tool.\n\n"
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
        "## Available tools\n\n\
         Signatures (TypeScript style). `?` = optional parameter; `a|b` = allowed values; \
         `{{…}}` = format hint. You MUST emit the call as JSON (see below).\n\n\
         ```\n{sigs}```\n\n\
         ## How to use a tool\n\n\
         To call a tool, include an XML block in your reply with this exact format:\n\n\
         ```\n\
         <tool_call>{{\"name\": \"tool_name\", \"arguments\": {{\"param1\": \"value1\"}}}}</tool_call>\n\
         ```\n\n\
         STRICT rules:\n\
         - You may call ONLY ONE tool per message.\n\
         - After writing the </tool_call> tag, you MUST stop your reply immediately.\n\
         - You will receive the tool result in the next message, then you can call another tool or answer.\n\
         - If you don't need a tool, answer directly without a <tool_call> tag.\n\
         - NEVER simulate a tool result. ALWAYS call it.\n\
         - When you download, create, modify or move a file/folder, verify its existence afterward with a tool.\n\
         - For shell_exec on Windows, use cmd.exe or PowerShell commands, NOT bash.\n\n"
    )
}

pub fn section_planification() -> &'static str {
    "## Planning\n\n\
     When the user asks for a complex task (several steps), \
     you MUST first lay out a plan before acting.\n\n\
     To display your plan, use this XML tag at the start of your reply:\n\n\
     ```\n\
     <plan>\n\
     [{\"task\": \"Description of step 1\", \"status\": \"in_progress\"},\n\
      {\"task\": \"Description of step 2\", \"status\": \"pending\"},\n\
      {\"task\": \"Description of step 3\", \"status\": \"pending\"}]\n\
     </plan>\n\
     ```\n\n\
     Valid statuses are: `pending`, `in_progress`, `done`.\n\
     On each new iteration, update the plan by changing the statuses.\n\
     Use a plan for tasks with 2+ steps. For simple questions, answer directly.\n\n"
}

pub fn section_comportement() -> &'static str {
    "## Behavior\n\n\
     - Reply in the user's language (match the language they write in).\n\
     - Be concise and useful.\n\
     - If you don't know something, say so honestly.\n\
     - For complex tasks, break them into steps, show your plan, and use the available tools.\n\
     - You can schedule (cron_create), watch (watcher_create), retrieve your conversations (session_search) and create your own skills.\n\
     - Use the tools provided for this turn (they are selected based on your intent). If you need a capability that isn't present, search for it in memory first.\n\
     - Memorize DURABLE facts with memory_write (preferences, decisions, persistent info); don't record trivia.\n\n\
     ## Self-improvement: forge your SKILLS and your TOOLS\n\n\
     You learn from your experiences by turning them into reusable knowledge. TWO distinct things:\n\n\
     ### SKILL = a PROCEDURE (the *how*)\n\
     A skill documents how to accomplish a type of task (steps, pitfalls, exact commands). It ORCHESTRATES existing tools.\n\
     - WHEN to create one: AFTER a complex task SUCCEEDS (>=2 tools chained, errors overcome, a corrected approach that worked, a non-trivial workflow discovered).\n\
     - HOW: `skill_create(name, description, body, tools, scripts)`. Declare in `tools`/`scripts` what the skill uses (this scopes it). The body = step-by-step procedure + pitfalls + commands.\n\
     - ITERATE: if you use a skill and it fails or is stale, `skill_patch(name, old, new)` IMMEDIATELY to fix it. That's how a skill becomes reliable.\n\
     - Bundled scripts: `skill_file_write(skill, path, content)` writes a script under `skills/<name>/scripts/`, which you then run via `shell_exec`/`execute_code`.\n\n\
     ### TOOL (plugin) = an atomic CAPABILITY (the *what*)\n\
     For an atomic repetitive action (a verb), forge a persistent tool: `plugin_create(name, description, command, schema, [script_path, script_content])`.\n\
     `command` = a shell template with {{slots}} (e.g. `python plugins/scripts/x.py {{arg}}`). It reloads itself and becomes callable like a native tool.\n\n\
     ### Rules\n\
     - SKILL for a procedure, PLUGIN for an atomic capability. Don't mix them up.\n\
     - You don't have every tool listed this turn: see the `Tool Catalog`, and call any of them via `tool_call` (or `tool_search` to search).\n\
     - Memorize durable FACTS with `memory_write`; PROCEDURES with `skill_create`.\n\n"
}

fn section_contexte_dynamique(instructions: &str) -> String {
    format!("## Additional instructions\n\n{instructions}\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_place_sections_stables_avant_outils_et_custom() {
        let tools = serde_json::json!([{"name":"file_read","description":"read","parameters":{}}]);
        let prompt = build_system_prompt(&tools, None, None, None, None, Some("custom volatile"));

        let env = prompt.find("## Environment").unwrap();
        let outils = prompt.find("## Available tools").unwrap();
        let custom = prompt.find("## Additional instructions").unwrap();
        assert!(env < outils);
        assert!(outils < custom);
        assert!(prompt.contains("ONLY ONE tool"));
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
