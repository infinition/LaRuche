//! Forge — outils d'**AUTO-AMÉLIORATION** : l'agent crée/édite ses propres **scripts de skill**,
//! ses **plugins** (outils forgés) et ses **serveurs MCP**. Inspiré de `skill_manage` d'third-party,
//! mais granulaire (schémas simples → fiable même sur petit modèle). Les skills (docs OKF) eux
//! vivent dans la carte cognitive et sont gérés par les abeilles `skill_*` de `memoire.rs`.

use crate::abeille::{
    Abeille, AbeilleRegistry, ContextExecution, NiveauDanger, ResultatAbeille, ToolOrigin,
};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Slug ascii (a-z0-9_) depuis un nom libre.
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

/// Chemin sûr sous `skills/<slug>/` (refuse `..` et chemins absolus).
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

// ─────────────────────────── Fichiers de skill (scripts/références) ───────────────────────────

pub struct SkillFileWrite;
#[async_trait]
impl Abeille for SkillFileWrite {
    fn nom(&self) -> &str {
        "skill_file_write"
    }
    fn description(&self) -> &str {
        "Cree OU ecrase un fichier bundle d'un skill (script, reference...). Ex: \
         skill_file_write(skill='arxiv', path='scripts/search.py', content='...'). Le script \
         s'execute ensuite via shell_exec/execute_code. NE cree PAS un outil (pour ca: plugin_create)."
    }
    fn schema(&self) -> serde_json::Value {
        json!({"type":"object","properties":{
            "skill":{"type":"string","description":"nom du skill"},
            "path":{"type":"string","description":"chemin relatif sous skills/<skill>/, ex. scripts/run.py"},
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
            return Ok(ResultatAbeille::err("chemin invalide (.. interdit)"));
        };
        if let Some(p) = full.parent() {
            let _ = std::fs::create_dir_all(p);
        }
        match std::fs::write(&full, content) {
            Ok(_) => Ok(ResultatAbeille::ok(format!("Ecrit: {}", full.display()))),
            Err(e) => Ok(ResultatAbeille::err(format!("Echec ecriture: {e}"))),
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
        "Lit un fichier bundle d'un skill (skill, path)."
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
            return Ok(ResultatAbeille::err("chemin invalide"));
        };
        match std::fs::read_to_string(&full) {
            Ok(c) => Ok(ResultatAbeille::ok(c)),
            Err(e) => Ok(ResultatAbeille::err(format!("Lecture impossible: {e}"))),
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
        "Supprime un fichier bundle d'un skill (skill, path)."
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
            return Ok(ResultatAbeille::err("chemin invalide"));
        };
        match std::fs::remove_file(&full) {
            Ok(_) => Ok(ResultatAbeille::ok(format!("Supprime: {}", full.display()))),
            Err(e) => Ok(ResultatAbeille::err(format!("Echec suppression: {e}"))),
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
        "Liste les fichiers bundles d'un skill (recursif)."
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
            return Ok(ResultatAbeille::err("skill manquant"));
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
            Ok(ResultatAbeille::ok("(aucun fichier)"))
        } else {
            out.sort();
            Ok(ResultatAbeille::ok(out.join("\n")))
        }
    }
}

// ─────────────────────────────── Plugins (outils forgés) ───────────────────────────────

pub struct PluginCreate {
    pub registry: Arc<AbeilleRegistry>,
}
#[async_trait]
impl Abeille for PluginCreate {
    fn nom(&self) -> &str {
        "plugin_create"
    }
    fn description(&self) -> &str {
        "Forge un OUTIL persistant (plugin) appelable comme une abeille. `command` = template \
         shell avec {{slots}} (ex. 'python plugins/scripts/x.py {{arg}}'). `schema` = JSON Schema \
         des arguments. Optionnel `script_path`+`script_content` pour ecrire le script appele. \
         Recharge automatiquement. Pour une PROCEDURE (pas un outil), utilise skill_create."
    }
    fn schema(&self) -> serde_json::Value {
        json!({"type":"object","properties":{
            "name":{"type":"string"},
            "description":{"type":"string"},
            "command":{"type":"string","description":"template shell avec {{slots}}"},
            "schema":{"type":"object","description":"JSON Schema des arguments du tool"},
            "script_path":{"type":"string","description":"optionnel: ex. plugins/scripts/x.py"},
            "script_content":{"type":"string","description":"optionnel: contenu du script"}
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
            return Ok(ResultatAbeille::err("name manquant"));
        }
        let slug = slugify(name);
        // Script optionnel (refuse ../).
        if let (Some(sp), Some(sc)) = (args["script_path"].as_str(), args["script_content"].as_str()) {
            if sp.contains("..") {
                return Ok(ResultatAbeille::err("script_path invalide"));
            }
            let p = PathBuf::from(sp);
            if let Some(parent) = p.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Err(e) = std::fs::write(&p, sc) {
                return Ok(ResultatAbeille::err(format!("Echec ecriture script: {e}")));
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
            return Ok(ResultatAbeille::err(format!("Echec ecriture plugin: {e}")));
        }
        // Recharge à chaud dans le registre principal.
        crate::abeilles::charger_plugins(Path::new("plugins"), &self.registry);
        Ok(ResultatAbeille::ok(format!(
            "Plugin `{name}` cree et charge ({}).",
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
        "Liste les plugins (outils forges) presents dans plugins/*.json."
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
            "(aucun plugin)".to_string()
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
        "Supprime un plugin (plugins/<nom>.json) et le retire du registre."
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
            return Ok(ResultatAbeille::err("name manquant"));
        }
        let path = PathBuf::from("plugins").join(format!("{slug}.json"));
        let _ = std::fs::remove_file(&path);
        // Vide les plugins custom du registre puis recharge ceux qui restent.
        self.registry.supprimer_par_origine(ToolOrigin::Custom);
        crate::abeilles::charger_plugins(Path::new("plugins"), &self.registry);
        Ok(ResultatAbeille::ok(format!("Plugin `{slug}` supprime.")))
    }
}

// ─────────────────────────────── MCP (serveurs externes) ───────────────────────────────

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
        "Ajoute (ou met a jour) un serveur MCP dans mcp_servers.json. Prend effet au redemarrage."
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
            return Ok(ResultatAbeille::err("name manquant"));
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
                "Serveur MCP `{name}` enregistre (actif au redemarrage)."
            ))),
            Err(e) => Ok(ResultatAbeille::err(format!("Echec: {e}"))),
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
        "Retire un serveur MCP de mcp_servers.json."
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
            Ok(_) => Ok(ResultatAbeille::ok(format!("Serveur MCP `{name}` retire."))),
            Err(e) => Ok(ResultatAbeille::err(format!("Echec: {e}"))),
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
        "Liste les serveurs MCP configures."
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
            "(aucun serveur MCP)".to_string()
        } else {
            noms.join("\n")
        }))
    }
}

/// Enregistre les outils de forge (auto-amélioration). `registry_arc` = le registre PRINCIPAL
/// (pour que plugin_create/delete rechargent au bon endroit).
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
