//! run_script — pipeline d'outils en UN seul tour (inspiré des scripts-RPC de third-party).
//!
//! Le modèle fournit une liste d'étapes `[{tool, args}]` ; on les exécute séquentiellement
//! via le registre, **sans repasser par le LLM** entre chaque étape. La sortie de l'étape N
//! est injectable dans les args des étapes suivantes via le jeton `{{N}}` (1-based).
//! Gros gain de contexte/latence sur les pipelines (ex. web_search → web_fetch → file_write).

use crate::abeille::{Abeille, AbeilleRegistry, ContextExecution, NiveauDanger, ResultatAbeille};
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

pub struct RunScript {
    pub registry: Arc<AbeilleRegistry>,
}

/// `tool_search` — divulgation progressive (inspiré du `tool_search` d'third-party) : cherche un
/// outil par mots-clés parmi TOUS ceux enregistrés, pas seulement ceux injectés ce tour.
/// Tolérant FR↔EN (recherche par sous-chaîne sur nom+description). Lecture seule.
pub struct ToolSearch {
    pub registry: Arc<AbeilleRegistry>,
}

#[async_trait]
impl Abeille for ToolSearch {
    fn nom(&self) -> &str {
        "tool_search"
    }
    fn description(&self) -> &str {
        "Search ALL registered tools by keyword (beyond those listed this turn). Returns name + origin + description. Then call it with tool_call."
    }
    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": { "type": "string", "description": "Search keywords" },
                "limit": { "type": "integer", "description": "Max results (default 15)" }
            },
            "required": ["query"]
        })
    }
    fn niveau_danger(&self) -> NiveauDanger {
        NiveauDanger::Safe
    }
    async fn executer(
        &self,
        args: serde_json::Value,
        _ctx: &ContextExecution,
    ) -> Result<ResultatAbeille> {
        let q = args["query"].as_str().unwrap_or("").to_lowercase();
        let limit = args["limit"].as_u64().unwrap_or(15) as usize;
        let toks: Vec<String> = q
            .split(|c: char| !c.is_alphanumeric())
            .filter(|t| t.len() > 1)
            .map(|t| t.to_string())
            .collect();
        let schema = self.registry.schema_complet();
        let mut lignes = Vec::new();
        if let Some(tools) = schema.as_array() {
            for t in tools {
                let name = t["name"].as_str().unwrap_or("");
                let desc = t["description"].as_str().unwrap_or("");
                let origin = t["origin"].as_str().unwrap_or("builtin");
                let hay = format!("{name} {desc}").to_lowercase();
                let hit = toks.is_empty() || toks.iter().any(|tk| hay.contains(tk.as_str()));
                if hit {
                    let short: String = desc.chars().take(120).collect();
                    lignes.push(format!("- {name} ({origin}): {short}"));
                }
            }
        }
        if lignes.is_empty() {
            return Ok(ResultatAbeille::ok("No matching tools."));
        }
        lignes.truncate(limit);
        Ok(ResultatAbeille::ok(format!(
            "Matching tools (call with tool_call):\n{}",
            lignes.join("\n")
        )))
    }
}

/// `tool_call` — exécute N'IMPORTE quel outil enregistré par son nom, même s'il n'est pas
/// injecté ce tour (inspiré du pont `tool_call` d'third-party). Préserve la validation : refuse les
/// outils non-`Safe` (à appeler directement) et la récursion.
pub struct ToolCall {
    pub registry: Arc<AbeilleRegistry>,
}

#[async_trait]
impl Abeille for ToolCall {
    fn nom(&self) -> &str {
        "tool_call"
    }
    fn description(&self) -> &str {
        "Execute any registered tool by name, even if not listed this turn (discover via tool_search). \
         `tool` = tool name, `args` = its arguments. Sensitive tools (requiring approval) must be called directly, not via tool_call."
    }
    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "tool": { "type": "string", "description": "Tool name to execute" },
                "args": { "type": "object", "description": "Arguments for the target tool" }
            },
            "required": ["tool"]
        })
    }
    fn niveau_danger(&self) -> NiveauDanger {
        NiveauDanger::Safe
    }
    async fn executer(
        &self,
        args: serde_json::Value,
        ctx: &ContextExecution,
    ) -> Result<ResultatAbeille> {
        let tool = args["tool"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("'tool' field missing"))?;
        if matches!(tool, "tool_call" | "run_script" | "delegate") {
            return Ok(ResultatAbeille::err(format!(
                "Recursion not allowed via tool_call: {tool}"
            )));
        }
        match self.registry.get(tool) {
            None => Ok(ResultatAbeille::err(format!("Unknown tool: {tool}"))),
            Some(a) if a.niveau_danger() != NiveauDanger::Safe => {
                Ok(ResultatAbeille::err(format!(
                "'{tool}' requires approval: call it DIRECTLY (not via tool_call)."
            )))
            }
            Some(_) => {
                let inner = args
                    .get("args")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!({}));
                match self.registry.executer(tool, inner, ctx).await {
                    Ok(result) => Ok(result),
                    Err(err) => Ok(ResultatAbeille::err(format!(
                        "Tool execution error '{tool}': {err}"
                    ))),
                }
            }
        }
    }
}

/// Remplace `{{N}}` (1-based) par la sortie de l'étape N, récursivement dans les chaînes.
fn substitute_refs(value: &mut serde_json::Value, outputs: &[String]) {
    match value {
        serde_json::Value::String(s) => {
            for (idx, out) in outputs.iter().enumerate() {
                let token = format!("{{{{{}}}}}", idx + 1);
                if s.contains(&token) {
                    *s = s.replace(&token, out);
                }
            }
        }
        serde_json::Value::Array(arr) => arr.iter_mut().for_each(|v| substitute_refs(v, outputs)),
        serde_json::Value::Object(map) => {
            map.values_mut().for_each(|v| substitute_refs(v, outputs))
        }
        _ => {}
    }
}

#[async_trait]
impl Abeille for RunScript {
    fn nom(&self) -> &str {
        "run_script"
    }

    fn description(&self) -> &str {
        "Execute a SEQUENCE of tools in one turn without re-prompting the LLM between steps. \
         `steps` = list of {tool, args} objects. Step N output is injectable into later args via {{N}} (1-based). \
         Ideal for chaining e.g. web_search → web_fetch → file_write. (run_script and delegate are forbidden as steps.)"
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "steps": {
                    "type": "array",
                    "description": "Ordered list of steps",
                    "items": {
                        "type": "object",
                        "properties": {
                            "tool": { "type": "string", "description": "Tool name" },
                            "args": { "type": "object", "description": "Tool arguments (may contain {{N}} references)" }
                        },
                        "required": ["tool"]
                    }
                }
            },
            "required": ["steps"]
        })
    }

    fn niveau_danger(&self) -> NiveauDanger {
        NiveauDanger::NeedsApproval
    }

    async fn executer(
        &self,
        args: serde_json::Value,
        ctx: &ContextExecution,
    ) -> Result<ResultatAbeille> {
        let steps = args["steps"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("'steps' (array) missing"))?;
        if steps.is_empty() {
            return Ok(ResultatAbeille::err("steps is empty"));
        }
        if steps.len() > 12 {
            return Ok(ResultatAbeille::err("too many steps (max 12)"));
        }

        let mut outputs: Vec<String> = Vec::new();
        let mut report = String::new();

        for (i, step) in steps.iter().enumerate() {
            let tool = step["tool"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("step {}: 'tool' missing", i + 1))?;

            // Garde-fou : pas de récursion ni de délégation imbriquée.
            if tool == "run_script" || tool == "delegate" {
                report.push_str(&format!(
                    "── step {} ({tool}) ──\n(not allowed inside a pipeline)\n\n",
                    i + 1
                ));
                outputs.push(String::new());
                continue;
            }

            let mut step_args = step
                .get("args")
                .cloned()
                .unwrap_or_else(|| serde_json::json!({}));
            substitute_refs(&mut step_args, &outputs);

            let res = match self.registry.executer(tool, step_args, ctx).await {
                Ok(res) => res,
                Err(err) => ResultatAbeille::err(format!(
                    "Step {} ({tool}) execution error: {err}",
                    i + 1
                )),
            };
            let out = if res.success {
                res.output
            } else {
                format!("Error: {}", res.error.unwrap_or_default())
            };
            let preview: String = out.chars().take(1500).collect();
            report.push_str(&format!("── step {} ({tool}) ──\n{preview}\n\n", i + 1));
            let failed = !res.success;
            outputs.push(out);
            if failed {
                report.push_str("(pipeline stopped: step failed)\n");
                break;
            }
        }

        Ok(ResultatAbeille::ok(report))
    }
}
