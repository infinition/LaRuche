/// Build the system prompt for the agent.
///
/// Sections are ordered from stable to volatile to preserve upstream prefix caches:
/// 1. stable identity and behavior,
/// 2. tool capabilities and call format,
/// 3. dynamic/custom context.
/// Assembles the system prompt from EDITABLE sections (loaded from the cognitive map,
/// hot-reloaded per turn) and LOCKED sections (machine-critical protocol, hardcoded).
///
/// - `identity_override` (`system.prompt` node): overrides the default identity.
/// - `behavior_override` (`system.behavior` node): overrides the default behavior.
/// - `custom_instructions` (`system.soul` node): additional instruction layer.
/// - Locked (never editable): tool list + `<tool_call>` format + `<plan>` format.
///   Editing these formats would break tool-calling, so they stay in code.
pub fn build_system_prompt(
    tools_schema: &serde_json::Value,
    identity_override: Option<&str>,
    behavior_override: Option<&str>,
    planning_override: Option<&str>,
    capability_index: Option<&str>,
    custom_instructions: Option<&str>,
) -> String {
    let mut prompt = String::new();
    // 1) Identity (editable) or hardcoded default.
    match identity_override {
        Some(o) if !o.trim().is_empty() => {
            prompt.push_str(o.trim());
            prompt.push_str("\n\n");
        }
        _ => prompt.push_str(&section_identite_stable()),
    }
    // 2) LOCKED protocol + generated tools + capability index.
    prompt.push_str(&section_outils(tools_schema));
    push_capability_index(&mut prompt, capability_index);
    match planning_override {
        Some(o) if !o.trim().is_empty() => {
            prompt.push_str(o.trim());
            prompt.push_str("\n\n");
        }
        _ => prompt.push_str(section_planification()),
    }
    // 3) Behavior (editable) or hardcoded default.
    match behavior_override {
        Some(o) if !o.trim().is_empty() => {
            prompt.push_str(o.trim());
            prompt.push_str("\n\n");
        }
        _ => prompt.push_str(&section_comportement()),
    }
    // 4) Additional instructions (SOUL).
    if let Some(instructions) = custom_instructions {
        prompt.push_str(&section_contexte_dynamique(instructions));
    }
    // 5) Secrets: expose the NAMES (never the values). The LLM references them via `${NAME}`
    //    in shell commands/scripts; the node substitutes the real value at execution time.
    let noms = crate::secrets::noms();
    if !noms.is_empty() {
        prompt.push_str(&format!(
            "\n## Available secrets\nThe user has stored secrets (API keys, tokens, webhook URLs). \
             You NEVER know their value, only the name. To use one in a shell_exec command, a \
             script or a URL, write `${{NAME}}` OR `@@NAME` (short form; the user often writes it \
             this way, e.g. `@@webhook_test1`): the system substitutes the real value at execution \
             time (never displayed). If the user writes `@@NAME`, it's a reference to that secret - \
             pass it through to the tool as-is (don't try to guess it).\n\
             Secrets: {}\n\n",
            noms.join(", ")
        ));
    }
    prompt
}

/// Compact capability catalog (names per family): the LLM knows what EXISTS beyond the
/// schemas injected this turn, and can reach everything via `tool_call`. Stable, cacheable.
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
         You can reason step by step and use tools to accomplish tasks. \
         Always reply in the user's language (the language of their message), regardless of the \
         language of these instructions.\n\n\
         ## Environment\n\
         - Operating system: {os_info}\n\
         - You MUST always use your tools (<tool_call>) to act. NEVER simulate an action.\n\
         - If asked to create a file, use the file_write tool.\n\
         - If asked to run a command, use shell_exec.\n\
         - Never invent the result of an action. Always call the matching tool.\n\n"
    )
}

/// Short type of a parameter: enum > typed array > abbreviated primitive. Format familiar to the model.
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

/// Parameter hint kept ONLY if it carries a FORMAT/EXAMPLE (cron, ISO8601, default,
/// `{{}}` slot, etc.). Descriptions redundant with name+type ("The URL to fetch") are dropped.
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

/// Renders tools as compact SIGNATURES (TypeScript style) instead of verbose JSON:
/// `name(param: type, opt?: type) - description`. ~80% fewer tokens than pretty JSON,
/// in a format the model natively associates with emitting a `<tool_call>`.
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
            // Required parameters first (order of `required`), then the optional ones.
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
        out.push_str(&format!("- {name}({}) - {desc}{suffixe}\n", sig.join(", ")));
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
         - Default: ONE tool per message. EXCEPTION - independent READ-ONLY calls (several \
         web searches/reads) or several `delegate` scouts MAY be emitted in the SAME message, \
         each in its own complete <tool_call>...</tool_call> block: they run in parallel.\n\
         - NEVER combine a mutating tool (write, shell, delete) with other calls: emit it alone.\n\
         - After writing the last </tool_call> tag, you MUST stop your reply immediately.\n\
         - You will receive the tool results in the next message, then you can call another tool or answer.\n\
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
     - LANGUAGE: ALWAYS reply in the SAME language as the user's message (French in -> French out, etc.). \
     These instructions are written in English, but your replies must be in the user's language, NOT English. \
     This rule overrides everything else.\n\
     - Be concise and useful.\n\
     - If you don't know something, say so honestly.\n\
     - For complex tasks, break them into steps, show your plan, and use the available tools.\n\
     - You can schedule (cron_create), watch (watcher_create), retrieve your conversations (session_search) and create your own skills.\n\
     - Use the tools provided for this turn (they are selected based on your intent). If you need a capability that isn't present, search for it in memory first.\n\
     - Memorize DURABLE facts with memory_write (preferences, decisions, persistent info); don't record trivia.\n\n\
     ## Autonomy during missions\n\n\
     - FINISH the job yourself. NEVER advise the user to search or do it themselves - you have the tools, use them.\n\
     - NEVER ask permission to continue mid-mission (\"do you want me to...?\" is forbidden). Either act, or conclude with results. \
     Use `clarify` ONLY when truly blocked on information ONLY the user has.\n\
     - A failed access (403, paywall, captcha, empty result) is an OBSTACLE, not a conclusion: retry via web archives \
     (web.archive.org), search-engine caches, mirrors and alternate sources before abandoning that angle.\n\
     - If the user asks for a thorough/deep/exhaustive research - or a quick lookup proves insufficient - call `research_mode` \
     FIRST: it activates the deep-research protocol (parallel `delegate` scouts, one per angle, no premature conclusions).\n\n\
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
        assert!(prompt.contains("ONE tool per message"));
        assert!(prompt.contains("run in parallel"), "multi-call fan-out documented");
        assert!(prompt.contains("Autonomy during missions"));
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
        // enum rendered inline, optional marked `?`, format hint kept, required without `?`.
        assert!(sigs.contains("action: add|done|list"));
        assert!(sigs.contains("render?: bool"));
        assert!(sigs.contains("url: string"));
        assert!(sigs.contains("*/5 * * * *"));
        assert!(!sigs.contains("The URL to fetch")); // redundant hint dropped
        // Measure the actual gain on this sample.
        let json = serde_json::to_string_pretty(&tools).unwrap();
        eprintln!(
            "[MEASURE] JSON pretty: {} chars (~{} tok) | signatures: {} chars (~{} tok) | gain {:.0}%",
            json.len(),
            json.len() / 4,
            sigs.len(),
            sigs.len() / 4,
            100.0 * (1.0 - sigs.len() as f64 / json.len() as f64)
        );
    }
}
