/// Build the system prompt for the agent.
///
/// Sections are ordered from stable to volatile to preserve upstream prefix caches:
/// 1. stable identity and behavior,
/// 2. tool capabilities and call format,
/// 3. dynamic/custom context.
///
/// Assembles the system prompt from EDITABLE sections (loaded from the cognitive map,
/// hot-reloaded per turn) and LOCKED sections (machine-critical protocol, hardcoded).
///
/// - `identity_override` (`system.prompt` node): overrides the default identity.
/// - `behavior_override` (`system.behavior` node): overrides the default behavior.
/// - `custom_instructions` (`system.soul` node): additional instruction layer.
/// - Locked (never editable): tool list + `<tool_call>` format + `<plan>` format.
///   Editing these formats would break tool-calling, so they stay in code.
// Dependances injectees, toutes distinctes: les regrouper dans une structure ne
// deplacerait la liste que d un cran, en la faisant construire par chaque appelant.
#[allow(clippy::too_many_arguments)]
pub fn build_system_prompt(
    tools_schema: &serde_json::Value,
    protocole_texte: bool,
    identity_override: Option<&str>,
    behavior_override: Option<&str>,
    planning_override: Option<&str>,
    capability_index: Option<&str>,
    custom_instructions: Option<&str>,
    reactions_agent: bool,
) -> String {
    let mut prompt = String::new();
    let mut jalons: Vec<(&str, usize)> = Vec::new();
    // 1) Identity (editable) or hardcoded default.
    match identity_override {
        Some(o) if !o.trim().is_empty() => {
            prompt.push_str(o.trim());
            prompt.push_str("\n\n");
        }
        _ => prompt.push_str(&section_identite_stable()),
    }
    jalons.push(("identity", prompt.len()));
    // 2) LOCKED protocol + generated tools + capability index.
    prompt.push_str(&section_outils(tools_schema, protocole_texte));
    jalons.push(("tools", prompt.len()));
    push_capability_index(&mut prompt, capability_index);
    jalons.push(("catalog+skills", prompt.len()));
    match planning_override {
        Some(o) if !o.trim().is_empty() => {
            prompt.push_str(o.trim());
            prompt.push_str("\n\n");
        }
        _ => prompt.push_str(section_planification()),
    }
    jalons.push(("planning", prompt.len()));
    // 3) Behavior (editable) or hardcoded default.
    match behavior_override {
        Some(o) if !o.trim().is_empty() => {
            prompt.push_str(o.trim());
            prompt.push_str("\n\n");
        }
        _ => prompt.push_str(section_comportement()),
    }
    jalons.push(("behavior", prompt.len()));
    // 4) Additional instructions (SOUL).
    if let Some(instructions) = custom_instructions {
        prompt.push_str(&section_contexte_dynamique(instructions));
    }
    // 5) Secrets: expose the NAMES (never the values). The LLM references them via `${NAME}`
    //    in shell commands/scripts; the node substitutes the real value at execution time.
    jalons.push(("soul+profile", prompt.len()));
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
    // 6) Agent reactions, OFF unless the user turned them on: this is instruction
    //    budget spent on every turn for something decorative, so it stays a choice.
    if reactions_agent {
        prompt.push_str(&crate::reactions::consigne_prompt());
        prompt.push_str(
            "

",
        );
    }
    jalons.push(("secrets", prompt.len()));
    mesurer(&prompt, &jalons);
    prompt
}

/// Log what each section of the system prompt actually costs, once per build.
///
/// Without this, every budget decision is a guess. It is how a hint worth ~150
/// tokens got filtered out to "save context" while the six failed tool calls that
/// followed cost fifteen passes. Chars are converted with the usual ~4 chars per
/// token rule: precision does not matter here, orders of magnitude do.
fn mesurer(prompt: &str, jalons: &[(&str, usize)]) {
    if !tracing::enabled!(tracing::Level::DEBUG) {
        return;
    }
    let mut detail: Vec<String> = Vec::new();
    let mut precedent = 0usize;
    for (nom, fin) in jalons {
        let taille = fin.saturating_sub(precedent);
        precedent = *fin;
        if taille > 0 {
            detail.push(format!("{nom}={}t", taille / 4));
        }
    }
    tracing::debug!(
        total_chars = prompt.len(),
        total_tokens_approx = prompt.len() / 4,
        sections = %detail.join(" "),
        "system prompt budget"
    );
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

    // The working directory, stated once. Without it a model that needed a project path
    // invented one (observed live: `D:\laruche`), then reached for a whole-drive scan to
    // find what it had just made up. One line here removes both failures.
    let cwd = std::env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    format!(
        "You are an intelligent, helpful AI assistant powered by LaRuche. \
         You can reason step by step and use tools to accomplish tasks. \
         Always THINK and reply in the user's language (the language of their message), regardless of the \
         language of these instructions. Your reasoning is DISPLAYED to the user beside the answer, so it counts as part of the reply: reasoning in English at a French speaker is the same defect as answering in English.\n\n\
         ## Environment\n\
         - Operating system: {os_info}\n\
         - Working directory: {cwd}. Relative paths resolve HERE. Never guess a project \
         path and never scan a whole drive to find one.\n\
         - Act through tools. Never describe, summarise or invent an action you did not \
         actually perform: emit the call and wait for its result.\n\n"
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
    match spec
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("string")
    {
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
/// Format hint for one parameter, kept in the compact signature.
///
/// The rule is STRUCTURAL, not a guess about the wording. It used to emit a hint
/// only when the description contained a digit, "ex:", "ISO" or "défaut". The
/// description of `watcher_create.regles` (a whole rule-tree grammar) contains
/// none of those, so it was silently dropped and the model saw `regles?: object`
/// with no guidance at all: six failed attempts and a watcher that could never
/// fire. A parameter shaped like an object or an array can NEVER be guessed from
/// its name, so its documentation always travels.
fn hint_param(spec: &serde_json::Value) -> Option<String> {
    let d = spec
        .get("description")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())?;

    let compose = matches!(
        spec.get("type").and_then(|v| v.as_str()),
        Some("object") | Some("array")
    );
    if !compose {
        // Scalars: a name plus a type usually says it all. Keep the hint only when
        // it carries something unguessable (a default, an example, a format).
        let utile = d.chars().any(|c| c.is_ascii_digit())
            || d.contains("ex:")
            || d.contains("e.g.")
            || d.contains("ISO")
            || d.contains("{{")
            || d.contains("['")
            || d.contains("default")
            || d.contains("défaut")
            || d.contains("defaut");
        if !utile {
            return None;
        }
    }

    // Composite parameters get room for their shape; scalars stay tight. Cutting at
    // a word boundary with an ellipsis matters as much as the budget: the old
    // `chars().take(60)` produced "(default: 900 for url, 0 o", which reads as a
    // complete sentence and hides the fact that anything was removed.
    // One worked example is worth more than any amount of prose to a small model:
    // it can copy a shape it has seen, it cannot invent one it has only read about.
    // Declare it as `"example"` in the JSON Schema of the parameter.
    let exemple = spec.get("example").map(|e| match e.as_str() {
        Some(s) => s.to_string(),
        None => serde_json::to_string(e).unwrap_or_default(),
    });

    let budget = if compose { 240 } else { 80 };
    let one = d.split_whitespace().collect::<Vec<_>>().join(" ");
    let avec_exemple = |texte: String| -> Option<String> {
        Some(match &exemple {
            Some(ex) if !ex.trim().is_empty() => format!("{texte} e.g. {ex}"),
            _ => texte,
        })
    };
    if one.chars().count() <= budget {
        return avec_exemple(one);
    }
    let mut coupe = String::new();
    for mot in one.split(' ') {
        if coupe.chars().count() + mot.chars().count() + 1 > budget - 1 {
            break;
        }
        if !coupe.is_empty() {
            coupe.push(' ');
        }
        coupe.push_str(mot);
    }
    avec_exemple(format!("{coupe}…"))
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
            let mut keys: Vec<&str> = req
                .iter()
                .copied()
                .filter(|k| props.contains_key(*k))
                .collect();
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

/// Comma-separated tool names, for the native-protocol inventory.
///
/// The model still needs to KNOW what it has at a glance, to plan with; it does
/// not need the parameters repeated, because the native channel carries them.
fn noms_outils(tools: &[serde_json::Value]) -> String {
    let noms: Vec<&str> = tools
        .iter()
        .filter_map(|t| {
            t.get("name")
                .or_else(|| t.get("function").and_then(|f| f.get("name")))
                .and_then(|n| n.as_str())
        })
        .collect();
    noms.join(", ")
}

/// Tool section. `protocole_texte` renders the `<tool_call>` XML convention.
///
/// It must be OFF for a backend that carries tool calls natively. Sending both a
/// native `tools` array AND an instruction to emit XML gives the model two
/// contradictory protocols, and a confused model falls back on whatever template it
/// memorised at training time: deepseek started emitting Anthropic's placeholder
/// syntax verbatim, calling a tool literally named `$TOOL_NAME` with an argument
/// named `$PARAMETER_NAME`, until the sentinel stopped the loop.
///
/// Parsing `<tool_call>` from the text stays enabled everywhere regardless: we stop
/// ASKING for it, we do not stop ACCEPTING it, so the fallback still catches a model
/// that emits it spontaneously.
fn section_outils(tools_schema: &serde_json::Value, protocole_texte: bool) -> String {
    let tools = match tools_schema.as_array() {
        Some(a) if !a.is_empty() => a,
        _ => return String::new(),
    };
    let sigs = signatures_outils(tools);
    if !protocole_texte {
        // NATIVE mode: the `tools` array already carries every name, description
        // and parameter schema. Rendering the signatures here too pays twice for
        // the same information. Measured on a refused body: 6976 chars of
        // signatures on top of 12085 chars of native schemas, 23% of an 84436-byte
        // request spent saying it twice, against a gateway wall at ~80 KB.
        //
        // What the text block alone provides is the CALLING CONVENTION, which no
        // schema expresses, plus a glanceable inventory. Both are kept; only the
        // duplicated detail goes.
        let noms = noms_outils(tools);
        return format!(
            "## Available tools\n\n\
             {noms}\n\n\
             Their full signatures reach you through your native tool-calling channel: \
             read the parameters there, they are authoritative. Emit ONE tool call per \
             message, except for independent read-only calls or several `delegate` scouts, \
             which may share a message and run in parallel. A mutating call (write, shell, \
             delete) always travels alone. After a call, stop and wait for its result.\n\n"
        );
    }
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
         Rules:\n\
         - ONE tool per message, with one exception: independent READ-ONLY calls (several \
         web searches/reads) or several `delegate` scouts may share a message, each in its \
         own complete <tool_call>...</tool_call> block, and they run in parallel.\n\
         - A mutating tool (write, shell, delete) always travels alone.\n\
         - Stop your reply immediately after the last </tool_call> tag. Results arrive in the \
         next message; then call another tool or answer.\n\
         - If you don't need a tool, answer directly without a <tool_call> tag.\n\
         - After downloading, creating, modifying or moving a file, verify it with a tool.\n\n"
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
    // The language rule lives in the opening line and is repeated once at the very
    // tail of the context (volatile tier), where recency makes it stick. A third
    // copy here taught the model that instructions are repeated and can be skimmed.
    "## Behavior\n\n\
     - Be concise and useful.\n\
     - If you don't know something, say so honestly.\n\
     - For complex tasks, break them into steps, show your plan, and use the available tools.\n\
     - You can schedule, watch, run long missions, search your past conversations (session_search) and create your own skills. See the choice below.\n\
     - Use the tools provided for this turn (they are selected based on your intent). If you need a capability that isn't present, search for it in memory first.\n\
     - Memorize DURABLE facts with memory_write (preferences, decisions, persistent info); don't record trivia.\n\n\
     ## Choosing how something happens later\n\n\
     Four primitives. Ask what DECIDES, not what the work is. Picking wrong is the most \
     expensive mistake available to you.\n\
     - A CONDITION decides -> `watcher_create`. \"when X happens\", \"if Y is true\", \"warn me if\". \
     Its rule tree is evaluated mechanically and costs NOTHING while nothing happens. A `command` \
     watcher observes anything a CLI can answer: a lamp, a service, a container, free disk space.\n\
     - The CLOCK decides -> `cron_create`. \"every morning at 9\", \"each Monday\". The work itself is \
     the point and its timing is arbitrary.\n\
     - A GOAL needs many sessions -> `mission_create`. \"research X in depth, iterate over days\".\n\
     - The USER tracks work items -> `kanban_create`. Their board, not your scratchpad; `todo` is \
     yours, for the current mission.\n\
     NEVER poll with a cron. \"Has X happened yet\" is a WATCHER, even when a command is needed to \
     answer it: a cron wakes a whole model turn at every tick to look at something that did not \
     change, where a watcher rule costs nothing and fires the moment it is true.\n\n\
     ## Autonomy during missions\n\n\
     - FINISH the job yourself. NEVER advise the user to search or do it themselves - you have the tools, use them.\n\
     - NEVER ask permission to continue mid-mission (\"do you want me to...?\" is forbidden). Either act, or conclude with results. \
     Use `clarify` ONLY when truly blocked on information ONLY the user has.\n\
     - A failed access (403, paywall, captcha, empty result) is an OBSTACLE, not a conclusion: retry via web archives \
     (web.archive.org), search-engine caches, mirrors and alternate sources before abandoning that angle.\n\
     - A MISSING TOOL is an install to perform, not a mystery to investigate. `command not found` after one \
     check means it is not installed: install it (the skill's Install section, a package manager, the official \
     release), verify, then carry on. Never hunt a binary across the disk, and never hand the install back to \
     the user. A leftover config file is not proof of an install: it outlives the program it configured.\n\
     - If the user asks for a thorough/deep/exhaustive research IN ANY LANGUAGE (e.g. FR 'approfondie', ES 'exhaustiva/investigación', \
     IT 'approfondita', DE 'gründlich', PT 'aprofundada') - or a quick lookup proves insufficient - call `research_mode` \
     FIRST: it activates the deep-research protocol (parallel `delegate` scouts, at most 4, no premature conclusions).\n\n\
     ## Self-improvement: forge your SKILLS and your TOOLS\n\n\
     You learn from your experiences by turning them into reusable knowledge. TWO distinct things:\n\n\
     ### SKILL = a PROCEDURE (the *how*)\n\
     A skill documents how to accomplish a type of task (steps, pitfalls, exact commands). It ORCHESTRATES existing tools.\n\
     - WHEN to create one: AFTER a complex task SUCCEEDS (>=2 tools chained, errors overcome, a corrected approach that worked, a non-trivial workflow discovered).\n\
     - HOW: `skill_create(name, description, body, tools, scripts)`. List in `tools` the tools the procedure relies on: it grants nothing and forbids nothing, but when the skill is opened, any listed tool whose signature is missing from the turn is pointed out to you, so you reach it with `tool_call` instead of assuming it is gone. The body = step-by-step procedure + pitfalls + commands.\n\
     - ITERATE: if you use a skill and it fails or is stale, `skill_patch(name, old, new)` IMMEDIATELY to fix it. That's how a skill becomes reliable.\n\
     - Bundled scripts: `skill_file_write(skill, path, content)` writes a script under `skills/<name>/scripts/`, which you then run via `shell_exec`/`execute_code`. It stays inert until the skill is loaded, so it costs nothing when the skill is not in play.\n\n\
     ### TOOL (plugin) = an atomic CAPABILITY (the *what*)\n\
     For an atomic repetitive action (a verb), forge a persistent tool: `plugin_create(name, description, command, schema, [script_path, script_content])`.\n\
     `command` = a shell template with {{slots}}. `{{plugin_dir}}` expands to the plugin's own folder, so use `python {{plugin_dir}}/run.py {{arg}}` rather than a path relative to the working directory.\n\
     A plugin is registered next to the built-in tools from the moment it exists, callable without any skill being loaded.\n\n\
     ### Where these things live\n\
     - `skills/<name>/` holds SKILL.md and, if needed, `scripts/`. One folder per skill.\n\
     - `plugins/<name>/` holds plugin.json and the files it runs, `run.py` by convention. One folder per plugin: `plugin_delete` removes the folder whole, so never scatter a plugin's script elsewhere.\n\
     - A JSON dropped loose at the root of `plugins/` is NOT loaded. It must sit in its own folder.\n\
     - The repository's `scripts/` folder is maintenance tooling for humans. Never write your own scripts there.\n\n\
     ### Rules\n\
     - SKILL for a procedure, PLUGIN for an atomic capability. Don't mix them up. The test: must it be usable without knowing a skill exists? Then it is a plugin. Does it only make sense inside a procedure you are documenting? Then it is a skill script.\n\
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
        let prompt = build_system_prompt(
            &tools,
            true,
            None,
            None,
            None,
            None,
            Some("custom volatile"),
            false,
        );

        let env = prompt.find("## Environment").unwrap();
        let outils = prompt.find("## Available tools").unwrap();
        let custom = prompt.find("## Additional instructions").unwrap();
        assert!(env < outils);
        assert!(outils < custom);
        assert!(prompt.contains("ONE tool per message"));
        assert!(
            prompt.contains("run in parallel"),
            "multi-call fan-out documented"
        );
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

    /// A native backend must not be taught a second, conflicting call protocol.
    ///
    /// Sending a `tools` array AND "emit this XML block" gave deepseek two
    /// contradictory formats. It fell back on a template memorised at training time
    /// and called a tool literally named `$TOOL_NAME` with `$PARAMETER_NAME`, in a
    /// loop, until the sentinel killed the turn.
    /// Measured on a refused body: 6976 chars of TypeScript signatures on top of
    /// 12085 chars of native schemas, saying the same thing twice, in a request
    /// that a gateway cuts at ~80 KB. In native mode the schemas are
    /// authoritative, so the prompt keeps the inventory and the calling
    /// convention and drops the duplicated detail.
    #[test]
    fn le_mode_natif_ne_repete_pas_les_schemas_doutils() {
        let outils = serde_json::json!([
            { "name": "web_fetch",
              "description": "Fetch a web page and return its clean text.",
              "parameters": { "type": "object", "properties": {
                  "url": { "type": "string", "description": "The URL to fetch" } } } },
            { "name": "shell_exec",
              "description": "Execute a shell command.",
              "parameters": { "type": "object", "properties": {
                  "command": { "type": "string", "description": "The command" } } } }
        ]);
        let natif = section_outils(&outils, false);
        let texte = section_outils(&outils, true);

        // The inventory survives: the model must know what it holds.
        assert!(natif.contains("web_fetch") && natif.contains("shell_exec"));
        // The calling convention survives: no schema expresses it.
        assert!(natif.contains("ONE tool call per"));
        // The duplicated detail is gone.
        assert!(
            !natif.contains("The URL to fetch"),
            "parameter descriptions are still duplicated in native mode"
        );
        assert!(
            natif.len() < texte.len(),
            "native mode should be the cheaper one"
        );
    }

    #[test]
    fn le_protocole_texte_disparait_pour_les_backends_natifs() {
        let tools = serde_json::json!([
            { "name": "file_read", "description": "Read a file",
              "parameters": { "type": "object", "properties": { "path": { "type": "string" } } } }
        ]);

        let natif = section_outils(&tools, false);
        assert!(natif.contains("file_read"), "signatures stay");
        assert!(
            !natif.contains("tool_call"),
            "a native backend must never be shown the XML convention"
        );
        assert!(natif.contains("native tool-calling channel"));

        let texte = section_outils(&tools, true);
        assert!(texte.contains("file_read"));
        assert!(
            texte.contains("<tool_call>"),
            "the text rail must survive for local models"
        );
    }

    /// The rule-tree grammar of `watcher_create.regles` contains no digit, no "ex:",
    /// no "ISO": the old wording-based heuristic dropped it and the model saw
    /// `regles?: object` with nothing else. A composite parameter always keeps its
    /// documentation.
    #[test]
    fn hint_param_garde_toujours_les_parametres_composites() {
        let regles = serde_json::json!({
            "type": "object",
            "description": "COMPILED condition tree: deterministic predicates. Ops: et/ou/non, jour_semaine{jours}, heure_entre{de,a}, apparu, contient{motif}."
        });
        let h = hint_param(&regles).expect("an object parameter always keeps its hint");
        assert!(h.contains("heure_entre"), "the grammar must survive: {h}");

        // A scalar whose description teaches nothing stays out, as before.
        let trivial = serde_json::json!({ "type": "string", "description": "The path to read." });
        assert!(hint_param(&trivial).is_none());
        // A scalar carrying a default is worth keeping.
        let avec_defaut =
            serde_json::json!({ "type": "integer", "description": "Max items (default 8)" });
        assert!(hint_param(&avec_defaut).is_some());
    }

    /// `chars().take(60)` produced "(default: 900 for url, 0 o", which reads like a
    /// finished sentence and hides that anything was cut.
    #[test]
    fn hint_param_coupe_au_mot_et_signale_la_coupe() {
        let spec = serde_json::json!({
            "type": "string",
            "description": format!("default {}", "word ".repeat(80))
        });
        let h = hint_param(&spec).unwrap();
        assert!(h.ends_with('…'), "a truncated hint must say so: {h}");
        assert!(
            h.ends_with("word…") || h.ends_with(" …"),
            "cut on a word boundary: {h}"
        );
    }

    /// GUARD: everything we put in `tools` must round-trip as valid JSON.
    ///
    /// deepseek rejected two requests with parse errors pointing INSIDE our payload
    /// ("tools[16]...properties.?: key must be a string"). serde_json cannot emit a
    /// non-string key, so either the body was corrupted in transit or one of our
    /// schemas is not what we think. This pins our side down: if it ever fails, the
    /// fault is ours; if it passes, the wire is to blame.
    #[test]
    fn les_schemas_doutils_produisent_du_json_valide() {
        let registre = crate::abeille::AbeilleRegistry::new();
        crate::abeilles::enregistrer_abeilles_builtin(&registre);
        let schema = registre.schema_complet();
        let outils = schema.as_array().expect("the registry yields an array");
        assert!(
            outils.len() > 10,
            "guard would be vacuous: {} tools",
            outils.len()
        );

        let openai = crate::providers::convertir_tools_openai(outils);
        let brut = serde_json::to_string(&openai).expect("serialization must succeed");
        let relu: serde_json::Value =
            serde_json::from_str(&brut).expect("what we send must parse back");
        assert_eq!(relu, serde_json::Value::Array(openai.clone()));

        // Every property key must be a real string, at every depth.
        fn verifier(v: &serde_json::Value, chemin: &str) {
            match v {
                serde_json::Value::Object(map) => {
                    for (k, sous) in map {
                        assert!(!k.is_empty(), "empty key at {chemin}");
                        verifier(sous, &format!("{chemin}.{k}"));
                    }
                }
                serde_json::Value::Array(items) => {
                    for (i, sous) in items.iter().enumerate() {
                        verifier(sous, &format!("{chemin}[{i}]"));
                    }
                }
                _ => {}
            }
        }
        for (i, outil) in openai.iter().enumerate() {
            verifier(outil, &format!("tools[{i}]"));
        }
    }
}
