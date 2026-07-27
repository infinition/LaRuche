//! Forge: **SELF-IMPROVEMENT** tools: the agent creates/edits its own **skill scripts**,
//! its **plugins** (forged tools) and its **MCP servers**. Granular by design
//! (simple schemas, reliable even on a small model). The skills (OKF docs)
//! live in the cognitive map and are managed by the `skill_*` abeilles in `memoire.rs`.

use crate::abeille::{
    Abeille, AbeilleRegistry, ContextExecution, NiveauDanger, ResultatAbeille, ToolOrigin,
};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// ASCII slug (a-z0-9_) from a free-form name.
fn slugify(name: &str) -> String {
    let mut s: String = name
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    while s.contains("__") {
        s = s.replace("__", "_");
    }
    s.trim_matches('_').to_string()
}

/// Safe path under `skills/<slug>/` (rejects `..` and absolute paths).
fn skill_file_path(skill: &str, rel: &str) -> Option<PathBuf> {
    if rel.is_empty() || rel.contains("..") || rel.starts_with('/') || rel.starts_with('\\') {
        return None;
    }
    let slug = slugify(skill);
    if slug.is_empty() {
        return None;
    }
    Some(PathBuf::from("skills").join(slug).join(rel))
}

// ─────────────────────────── Skill files (scripts/references) ───────────────────────────

pub struct SkillFileWrite;
#[async_trait]
impl Abeille for SkillFileWrite {
    fn nom(&self) -> &str {
        "skill_file_write"
    }
    fn description(&self) -> &str {
        "Create or overwrite a file in a skill bundle (script, reference, etc.). \
         The script is then run via shell_exec/execute_code. \
         Does NOT create a tool - use plugin_create for that."
    }
    fn schema(&self) -> serde_json::Value {
        json!({"type":"object","properties":{
            "skill":{"type":"string","description":"skill name"},
            "path":{"type":"string","description":"relative path under skills/<skill>/, e.g. scripts/run.py"},
            "content":{"type":"string"}},"required":["skill","path","content"]})
    }
    fn niveau_danger(&self) -> NiveauDanger {
        NiveauDanger::Safe
    }
    async fn executer(
        &self,
        args: serde_json::Value,
        _ctx: &ContextExecution,
    ) -> Result<ResultatAbeille> {
        let skill = args["skill"].as_str().unwrap_or("");
        let path = args["path"].as_str().unwrap_or("");
        let content = args["content"].as_str().unwrap_or("");
        let Some(full) = skill_file_path(skill, path) else {
            return Ok(ResultatAbeille::err("invalid path (.. not allowed)"));
        };
        if let Some(p) = full.parent() {
            let _ = std::fs::create_dir_all(p);
        }
        match std::fs::write(&full, content) {
            Ok(_) => Ok(ResultatAbeille::ok(format!("Written: {}", full.display()))),
            Err(e) => Ok(ResultatAbeille::err(format!("Write failed: {e}"))),
        }
    }
}

pub struct SkillFileRead;
#[async_trait]
impl Abeille for SkillFileRead {
    fn nom(&self) -> &str {
        "skill_file_read"
    }
    fn description(&self) -> &str {
        "Read a file from a skill bundle (skill, path)."
    }
    fn schema(&self) -> serde_json::Value {
        json!({"type":"object","properties":{"skill":{"type":"string"},"path":{"type":"string"}},"required":["skill","path"]})
    }
    fn niveau_danger(&self) -> NiveauDanger {
        NiveauDanger::Safe
    }
    async fn executer(
        &self,
        args: serde_json::Value,
        _ctx: &ContextExecution,
    ) -> Result<ResultatAbeille> {
        let Some(full) = skill_file_path(args["skill"].as_str().unwrap_or(""), args["path"].as_str().unwrap_or("")) else {
            return Ok(ResultatAbeille::err("invalid path"));
        };
        match std::fs::read_to_string(&full) {
            Ok(c) => Ok(ResultatAbeille::ok(c)),
            Err(e) => Ok(ResultatAbeille::err(format!("Read failed: {e}"))),
        }
    }
}

pub struct SkillFileDelete;
#[async_trait]
impl Abeille for SkillFileDelete {
    fn nom(&self) -> &str {
        "skill_file_delete"
    }
    fn description(&self) -> &str {
        "Delete a file from a skill bundle (skill, path)."
    }
    fn schema(&self) -> serde_json::Value {
        json!({"type":"object","properties":{"skill":{"type":"string"},"path":{"type":"string"}},"required":["skill","path"]})
    }
    fn niveau_danger(&self) -> NiveauDanger {
        NiveauDanger::Safe
    }
    async fn executer(
        &self,
        args: serde_json::Value,
        _ctx: &ContextExecution,
    ) -> Result<ResultatAbeille> {
        let Some(full) = skill_file_path(args["skill"].as_str().unwrap_or(""), args["path"].as_str().unwrap_or("")) else {
            return Ok(ResultatAbeille::err("invalid path"));
        };
        match std::fs::remove_file(&full) {
            Ok(_) => Ok(ResultatAbeille::ok(format!("Deleted: {}", full.display()))),
            Err(e) => Ok(ResultatAbeille::err(format!("Delete failed: {e}"))),
        }
    }
}

pub struct SkillFileList;
#[async_trait]
impl Abeille for SkillFileList {
    fn nom(&self) -> &str {
        "skill_file_list"
    }
    fn description(&self) -> &str {
        "List all files in a skill bundle (recursive)."
    }
    fn schema(&self) -> serde_json::Value {
        json!({"type":"object","properties":{"skill":{"type":"string"}},"required":["skill"]})
    }
    fn niveau_danger(&self) -> NiveauDanger {
        NiveauDanger::Safe
    }
    async fn executer(
        &self,
        args: serde_json::Value,
        _ctx: &ContextExecution,
    ) -> Result<ResultatAbeille> {
        let slug = slugify(args["skill"].as_str().unwrap_or(""));
        if slug.is_empty() {
            return Ok(ResultatAbeille::err("skill name required"));
        }
        let base = PathBuf::from("skills").join(&slug);
        let mut out = Vec::new();
        let mut stack = vec![base.clone()];
        while let Some(d) = stack.pop() {
            if let Ok(rd) = std::fs::read_dir(&d) {
                for e in rd.flatten() {
                    let p = e.path();
                    if p.is_dir() {
                        stack.push(p);
                    } else if let Ok(rel) = p.strip_prefix(&base) {
                        out.push(rel.to_string_lossy().replace('\\', "/"));
                    }
                }
            }
        }
        if out.is_empty() {
            Ok(ResultatAbeille::ok("(no files)"))
        } else {
            out.sort();
            Ok(ResultatAbeille::ok(out.join("\n")))
        }
    }
}

// ─────────────────────────────── Plugins (forged tools) ───────────────────────────────

pub struct PluginCreate {
    pub registry: Arc<AbeilleRegistry>,
}
#[async_trait]
impl Abeille for PluginCreate {
    fn nom(&self) -> &str {
        "plugin_create"
    }
    fn description(&self) -> &str {
        "Create a persistent tool (plugin) callable like any built-in. `command` = shell template \
         with {{slots}} (e.g. 'python plugins/scripts/x.py {{arg}}'). `schema` = JSON Schema for \
         the tool's arguments. Optional `script_path`+`script_content` to write the script inline. \
         Hot-reloads automatically. For a PROCEDURE (not a tool), use skill_create."
    }
    fn schema(&self) -> serde_json::Value {
        json!({"type":"object","properties":{
            "name":{"type":"string"},
            "description":{"type":"string"},
            "command":{"type":"string","description":"shell template with {{slots}}"},
            "schema":{"type":"object","description":"JSON Schema for the tool's arguments"},
            "script_path":{"type":"string","description":"optional: e.g. plugins/scripts/x.py"},
            "script_content":{"type":"string","description":"optional: script source code"}
        },"required":["name","description","command"]})
    }
    fn niveau_danger(&self) -> NiveauDanger {
        NiveauDanger::NeedsApproval
    }
    async fn executer(
        &self,
        args: serde_json::Value,
        _ctx: &ContextExecution,
    ) -> Result<ResultatAbeille> {
        let name = args["name"].as_str().unwrap_or("").trim();
        if name.is_empty() {
            return Ok(ResultatAbeille::err("name required"));
        }
        let slug = slugify(name);
        // Optional script (rejects ../).
        if let (Some(sp), Some(sc)) = (args["script_path"].as_str(), args["script_content"].as_str()) {
            if sp.contains("..") {
                return Ok(ResultatAbeille::err("invalid script_path"));
            }
            let p = PathBuf::from(sp);
            if let Some(parent) = p.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Err(e) = std::fs::write(&p, sc) {
                return Ok(ResultatAbeille::err(format!("Script write failed: {e}")));
            }
        }
        let def = json!({
            "name": name,
            "description": args["description"].as_str().unwrap_or(""),
            "parameters": args.get("schema").cloned().unwrap_or_else(|| json!({"type":"object","properties":{}})),
            "command": args["command"].as_str().unwrap_or(""),
        });
        let _ = std::fs::create_dir_all("plugins");
        let path = PathBuf::from("plugins").join(format!("{slug}.json"));
        if let Err(e) = std::fs::write(&path, serde_json::to_string_pretty(&def).unwrap_or_default()) {
            return Ok(ResultatAbeille::err(format!("Plugin write failed: {e}")));
        }
        // Hot-reload into the main registry.
        crate::abeilles::charger_plugins(Path::new("plugins"), &self.registry);
        Ok(ResultatAbeille::ok(format!(
            "Plugin `{name}` created and loaded ({}).",
            path.display()
        )))
    }
}

pub struct PluginList;
#[async_trait]
impl Abeille for PluginList {
    fn nom(&self) -> &str {
        "plugin_list"
    }
    fn description(&self) -> &str {
        "List all forged plugins present in plugins/*.json."
    }
    fn schema(&self) -> serde_json::Value {
        json!({"type":"object","properties":{}})
    }
    fn niveau_danger(&self) -> NiveauDanger {
        NiveauDanger::Safe
    }
    async fn executer(
        &self,
        _args: serde_json::Value,
        _ctx: &ContextExecution,
    ) -> Result<ResultatAbeille> {
        let mut out = Vec::new();
        if let Ok(rd) = std::fs::read_dir("plugins") {
            for e in rd.flatten() {
                let p = e.path();
                if p.extension().map(|x| x == "json").unwrap_or(false) {
                    if let Some(n) = p.file_stem() {
                        out.push(n.to_string_lossy().to_string());
                    }
                }
            }
        }
        out.sort();
        Ok(ResultatAbeille::ok(if out.is_empty() {
            "(no plugins)".to_string()
        } else {
            out.join("\n")
        }))
    }
}

pub struct PluginDelete {
    pub registry: Arc<AbeilleRegistry>,
}
#[async_trait]
impl Abeille for PluginDelete {
    fn nom(&self) -> &str {
        "plugin_delete"
    }
    fn description(&self) -> &str {
        "Delete a plugin (plugins/<name>.json) and remove it from the registry."
    }
    fn schema(&self) -> serde_json::Value {
        json!({"type":"object","properties":{"name":{"type":"string"}},"required":["name"]})
    }
    fn niveau_danger(&self) -> NiveauDanger {
        NiveauDanger::NeedsApproval
    }
    async fn executer(
        &self,
        args: serde_json::Value,
        _ctx: &ContextExecution,
    ) -> Result<ResultatAbeille> {
        let slug = slugify(args["name"].as_str().unwrap_or(""));
        if slug.is_empty() {
            return Ok(ResultatAbeille::err("name required"));
        }
        let path = PathBuf::from("plugins").join(format!("{slug}.json"));
        let _ = std::fs::remove_file(&path);
        // Clear custom plugins from the registry then reload the remaining ones.
        self.registry.supprimer_par_origine(ToolOrigin::Custom);
        crate::abeilles::charger_plugins(Path::new("plugins"), &self.registry);
        Ok(ResultatAbeille::ok(format!("Plugin `{slug}` deleted.")))
    }
}

// ─────────────────────────────── MCP (external servers) ───────────────────────────────

fn lire_mcp() -> serde_json::Value {
    std::fs::read_to_string("mcp_servers.json")
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| json!({ "mcpServers": {} }))
}
fn ecrire_mcp(v: &serde_json::Value) -> std::io::Result<()> {
    std::fs::write("mcp_servers.json", serde_json::to_string_pretty(v).unwrap_or_default())
}

pub struct McpAdd;
#[async_trait]
impl Abeille for McpAdd {
    fn nom(&self) -> &str {
        "mcp_add"
    }
    fn description(&self) -> &str {
        "Add or update an MCP server in mcp_servers.json. Takes effect on restart."
    }
    fn schema(&self) -> serde_json::Value {
        json!({"type":"object","properties":{
            "name":{"type":"string"},
            "command":{"type":"string"},
            "args":{"type":"array","items":{"type":"string"}}
        },"required":["name","command"]})
    }
    fn niveau_danger(&self) -> NiveauDanger {
        NiveauDanger::NeedsApproval
    }
    async fn executer(
        &self,
        args: serde_json::Value,
        _ctx: &ContextExecution,
    ) -> Result<ResultatAbeille> {
        let name = args["name"].as_str().unwrap_or("").trim();
        if name.is_empty() {
            return Ok(ResultatAbeille::err("name required"));
        }
        let mut v = lire_mcp();
        if !v["mcpServers"].is_object() {
            v["mcpServers"] = json!({});
        }
        v["mcpServers"][name] = json!({
            "command": args["command"].as_str().unwrap_or(""),
            "args": args.get("args").cloned().unwrap_or_else(|| json!([])),
        });
        match ecrire_mcp(&v) {
            Ok(_) => Ok(ResultatAbeille::ok(format!(
                "MCP server `{name}` registered (active on restart)."
            ))),
            Err(e) => Ok(ResultatAbeille::err(format!("Failed: {e}"))),
        }
    }
}

pub struct McpRemove;
#[async_trait]
impl Abeille for McpRemove {
    fn nom(&self) -> &str {
        "mcp_remove"
    }
    fn description(&self) -> &str {
        "Remove an MCP server from mcp_servers.json."
    }
    fn schema(&self) -> serde_json::Value {
        json!({"type":"object","properties":{"name":{"type":"string"}},"required":["name"]})
    }
    fn niveau_danger(&self) -> NiveauDanger {
        NiveauDanger::NeedsApproval
    }
    async fn executer(
        &self,
        args: serde_json::Value,
        _ctx: &ContextExecution,
    ) -> Result<ResultatAbeille> {
        let name = args["name"].as_str().unwrap_or("").trim();
        let mut v = lire_mcp();
        if let Some(obj) = v["mcpServers"].as_object_mut() {
            obj.remove(name);
        }
        match ecrire_mcp(&v) {
            Ok(_) => Ok(ResultatAbeille::ok(format!("MCP server `{name}` removed."))),
            Err(e) => Ok(ResultatAbeille::err(format!("Failed: {e}"))),
        }
    }
}

pub struct McpList;
#[async_trait]
impl Abeille for McpList {
    fn nom(&self) -> &str {
        "mcp_list"
    }
    fn description(&self) -> &str {
        "List configured MCP servers."
    }
    fn schema(&self) -> serde_json::Value {
        json!({"type":"object","properties":{}})
    }
    fn niveau_danger(&self) -> NiveauDanger {
        NiveauDanger::Safe
    }
    async fn executer(
        &self,
        _args: serde_json::Value,
        _ctx: &ContextExecution,
    ) -> Result<ResultatAbeille> {
        let v = lire_mcp();
        let noms: Vec<String> = v["mcpServers"]
            .as_object()
            .map(|o| o.keys().cloned().collect())
            .unwrap_or_default();
        Ok(ResultatAbeille::ok(if noms.is_empty() {
            "(no MCP servers)".to_string()
        } else {
            noms.join("\n")
        }))
    }
}

/// Registers the forge tools (self-improvement). `registry_arc` = the MAIN registry
/// (so plugin_create/delete reload in the right place).
pub fn enregistrer_forge(registry: &AbeilleRegistry, registry_arc: Arc<AbeilleRegistry>) {
    registry.enregistrer(Box::new(SkillFileWrite));
    registry.enregistrer(Box::new(SkillFileRead));
    registry.enregistrer(Box::new(SkillFileDelete));
    registry.enregistrer(Box::new(SkillFileList));
    registry.enregistrer(Box::new(PluginCreate {
        registry: registry_arc.clone(),
    }));
    registry.enregistrer(Box::new(PluginList));
    registry.enregistrer(Box::new(PluginDelete {
        registry: registry_arc,
    }));
    registry.enregistrer(Box::new(McpAdd));
    registry.enregistrer(Box::new(McpRemove));
    registry.enregistrer(Box::new(McpList));
    tracing::info!("Forge abeilles registered (skill_file_*, plugin_*, mcp_*)");
}
