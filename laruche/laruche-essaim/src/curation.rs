//! Post-turn memory curation: durable-fact extraction, contradiction detection,
//! node consolidation, and the auto-learning of reusable OKF skills from
//! successful trajectories.

use crate::config::EssaimConfig;
use crate::contexte::yaml_frontmatter_field;
use crate::evenements::ChatEvent;
use crate::providers::provider_chat_stream;
use anyhow::Result;
use futures_util::StreamExt;
use laruche_memoire::{MemoireCognitive, MemoryItem, SearchOpts};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
struct MemFact {
    node_id: String,
    content: String,
    #[serde(default)]
    confidence: Option<f32>,
    #[serde(default)]
    source: Option<String>,
}

/// Extract the first JSON array from a text (tolerates surrounding chatter).
pub fn extraire_json_array(s: &str) -> Option<String> {
    let start = s.find('[')?;
    let end = s.rfind(']')?;
    (end > start).then(|| s[start..=end].to_string())
}

/// Fix C - validates a node_id before a memory write: non-empty, no '|' or space, last
/// segment != placeholder 'x', and hierarchical (prefix.name - not a root node like "system").
pub fn node_id_valide(node_id: &str) -> bool {
    let id = node_id.trim();
    if id.is_empty() || id.contains('|') || id.contains(' ') || !id.contains('.') {
        return false;
    }
    // Reserved branches: `system.*` holds the editable prompt sections and
    // `capacities.*`/`tools.*` mirror the registry, all rewritten from elsewhere. The
    // memory_write tool already refuses them (`noeud_reserve`); the curator wrote
    // straight to the store and went around that door, which is how three notes about
    // cron_create ended up sitting in the system root.
    let racine = id.split('.').next().unwrap_or("");
    if matches!(racine, "system" | "capacities" | "tools" | "orphans") {
        return false;
    }
    let last = id.rsplit('.').next().unwrap_or("");
    !last.is_empty() && last != "x"
}

/// Post-curation: an auxiliary LLM call extracts durable facts -> memory.
pub(crate) async fn curer_memoire(
    user: &str,
    assistant: &str,
    config: &EssaimConfig,
    memoire: &Arc<dyn MemoireCognitive>,
) -> Result<()> {
    let sys = "You are a memory extractor. From the exchange, return ONLY a \
        JSON array of the DURABLE facts to memorize (stable preferences, decisions, \
        persistent info about the user or projects). Each element: \
        {\"node_id\":\"<prefixe>.<nom>\",\"content\":\"...\",\"confidence\":0.0-1.0,\"source\":\"...\"} \
        where <prefixe> is people, projects or decisions (e.g. people.alex, projects.laruche, \
        decisions.archi). The node_id must contain NEITHER a space NOR the character '|', \
        and NEVER uses 'x' as a name (those are examples). \
        'confidence': your certainty level (1.0 = certain, 0.5 = guess). \
        'source': where the info comes from (e.g. 'user said', 'web_search', 'analysis'). \
        If nothing durable, return []. No text outside the JSON.";
    let messages = vec![
        serde_json::json!({ "role": "system", "content": sys }),
        serde_json::json!({ "role": "user", "content": format!("User: {user}\nAssistant: {assistant}") }),
    ];
    let mut stream = provider_chat_stream(
        &config.provider,
        config.aux_model.as_deref().unwrap_or(&config.model),
        &messages,
        0.0,
        512,
        &crate::secrets::substituer(&config.api_key),
        config.api_base.as_deref(),
            &config.ollama_url,
            None,
        ).await?;
    let mut out = String::new();
    while let Some(chunk) = stream.next().await {
        out.push_str(&chunk.text);
    }

    if let Some(js) = extraire_json_array(&out) {
        if let Ok(items) = serde_json::from_str::<Vec<MemFact>>(&js) {
            for f in items {
                // Fix C - anti-pollution guard: reject empty node_ids, the
                // placeholders (people.x|projects.x|...), '|'/spaces and the names 'x'.
                if !node_id_valide(&f.node_id) || f.content.trim().is_empty() {
                    continue;
                }
                let mut item = MemoryItem::new(f.node_id, f.content).with_source("auto-curation");
                if let Some(conf) = f.confidence {
                    item.confidence = Some(conf.clamp(0.0, 1.0));
                }
                if let Some(src) = f.source {
                    item.source = Some(src);
                }
                // When LaReine's queue gate is on, the write becomes a proposal in the
                // backlog (approved by a human) instead of being applied directly.
                let _ = crate::reine_queue::proposer_memoire(
                    memoire,
                    item,
                    config.reine.queue_gate,
                    &config.reine.mode,
                    "curateur",
                )
                .await;
            }
        }
    }
    Ok(())
}

/// Checks whether a new fact contradicts existing facts in memory.
/// Writes a note under `contradictions.*` if a contradiction is detected.
pub async fn detecter_contradictions(
    nouveau_contenu: &str,
    memoire: &Arc<dyn MemoireCognitive>,
) -> Result<()> {
    let pack = memoire
        .search(
            nouveau_contenu,
            SearchOpts {
                depth: None,
                limit: Some(5),
                sans_trace: false,
            },
        )
        .await?;

    let Some(items) = pack
        .raw
        .get("items")
        .or_else(|| pack.raw.get("evidence"))
        .and_then(|v| v.as_array())
    else {
        return Ok(());
    };

    for item in items {
        let existing_content = item
            .get("content")
            .or_else(|| item.get("text"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if existing_content.is_empty() || existing_content == nouveau_contenu {
            continue;
        }
        let existing_lower = existing_content.to_lowercase();
        let nouveau_lower = nouveau_contenu.to_lowercase();

        if (existing_lower.contains("ne ") && !nouveau_lower.contains("ne "))
            || (!existing_lower.contains("ne ") && nouveau_lower.contains("ne "))
        {
            let node_id = item
                .get("node_id")
                .or_else(|| item.get("node"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let contradiction = format!(
                "CONTRADICTION DETECTED:\n- Old ({}): {existing_content}\n- New: {nouveau_contenu}\n\
                 To resolve: one of the two is incorrect or contextual.",
                node_id
            );
            let _ = memoire
                .write(
                    MemoryItem::new(
                        format!(
                            "contradictions.auto.{}",
                            uuid::Uuid::new_v4()
                                .to_string()
                                .split('-')
                                .next()
                                .unwrap_or("x")
                        ),
                        contradiction,
                    )
                    .with_source("contradiction-detector"),
                )
                .await;
            tracing::warn!(
                existing = existing_content,
                nouveau = nouveau_contenu,
                "Memory contradiction detected"
            );
        }
    }
    Ok(())
}

/// Consolidate ONE node: merge/dedupe its items into a minimal set via the aux model,
/// then replace (old ones **soft-deleted** -> recoverable via the audit). Only acts if there's a
/// real gain (fewer items). Skips `system.*`/`capacities.*` (handled as single items elsewhere).
pub async fn consolider_node(
    memoire: &Arc<dyn MemoireCognitive>,
    config: &EssaimConfig,
    node_id: &str,
) -> Result<serde_json::Value> {
    if node_id.starts_with("system") || node_id.starts_with("capacities") {
        return Ok(serde_json::json!({ "node_id": node_id, "skipped": "system node" }));
    }
    let node = memoire.read_node(node_id).await?;
    let items: Vec<(String, String)> = node
        .get("items")
        .and_then(|i| i.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|it| {
                    Some((
                        it.get("id").and_then(|x| x.as_str())?.to_string(),
                        it.get("content").and_then(|x| x.as_str())?.to_string(),
                    ))
                })
                .collect()
        })
        .unwrap_or_default();
    if items.len() < 2 {
        return Ok(
            serde_json::json!({ "node_id": node_id, "items": items.len(), "unchanged": true }),
        );
    }
    let liste = items
        .iter()
        .enumerate()
        .map(|(i, (_, c))| format!("{}. {}", i + 1, c))
        .collect::<Vec<_>>()
        .join("\n");
    let sys = "You consolidate a node's memory. You are given a list of facts/notes. \
        Merge duplicates and redundancies, KEEP all distinct information, rephrase clearly. \
        Return ONLY a JSON array of consolidated items: [{\"content\":\"...\"}]. \
        Aim for the minimum (often 1 to 3 for a person/project/synthesis). No text outside the JSON.";
    let messages = vec![
        serde_json::json!({ "role": "system", "content": sys }),
        serde_json::json!({ "role": "user", "content": format!("Node: {node_id}\nItems:\n{liste}") }),
    ];
    let mut stream = provider_chat_stream(
        &config.provider,
        config.aux_model.as_deref().unwrap_or(&config.model),
        &messages,
        0.0,
        1400,
        &crate::secrets::substituer(&config.api_key),
        config.api_base.as_deref(),
            &config.ollama_url,
            None,
        ).await?;
    let mut out = String::new();
    while let Some(chunk) = stream.next().await {
        out.push_str(&chunk.text);
    }
    let Some(js) = extraire_json_array(&out) else {
        return Ok(serde_json::json!({ "node_id": node_id, "error": "no JSON" }));
    };
    let arr: Vec<serde_json::Value> = serde_json::from_str(&js).unwrap_or_default();
    let news: Vec<String> = arr
        .iter()
        .filter_map(|v| {
            v.get("content")
                .and_then(|c| c.as_str())
                .map(|s| s.trim().to_string())
        })
        .filter(|s| !s.is_empty())
        .collect();
    // Safety: only replace IF there's a real gain (otherwise touch nothing).
    if news.is_empty() || news.len() >= items.len() {
        return Ok(
            serde_json::json!({ "node_id": node_id, "items": items.len(), "unchanged": true }),
        );
    }
    for (id, _) in &items {
        let _ = memoire.delete_item(id, Some("consolidation")).await;
    }
    for c in &news {
        let _ = memoire
            .write(MemoryItem::new(node_id.to_string(), c.clone()).with_source("consolidation"))
            .await;
    }
    Ok(serde_json::json!({ "node_id": node_id, "before": items.len(), "after": news.len() }))
}

/// Consolidate memory: spot loaded nodes (>=4 items, excluding system/capacities) and pass
/// them to `consolider_node`. Bounded in node count per run (LLM cost).
pub async fn consolider_memoire(
    memoire: &Arc<dyn MemoireCognitive>,
    config: &EssaimConfig,
) -> Result<serde_json::Value> {
    let mut cibles: Vec<String> = Vec::new();
    if let Ok(sugg) = memoire.suggest_nodes("", Some(200)).await {
        if let Some(nodes) = sugg.get("nodes").and_then(|n| n.as_array()) {
            for n in nodes {
                let id = n.get("id").and_then(|x| x.as_str()).unwrap_or("");
                let count = n.get("item_count").and_then(|x| x.as_u64()).unwrap_or(0);
                if count >= 4 && !id.starts_with("system") && !id.starts_with("capacities") {
                    cibles.push(id.to_string());
                }
            }
        }
    }
    cibles.truncate(12);
    let mut rapport = Vec::new();
    for id in cibles {
        if let Ok(r) = consolider_node(memoire, config, &id).await {
            rapport.push(r);
        }
    }
    Ok(serde_json::json!({ "consolidated": rapport.len(), "details": rapport }))
}

pub(crate) async fn extraire_skill_memoire(
    user: &str,
    assistant: &str,
    config: &EssaimConfig,
    memoire: &Arc<dyn MemoireCognitive>,
    tx: &tokio::sync::broadcast::Sender<ChatEvent>,
    n_outils: usize,
) -> Result<()> {
    // Anti-noise gating: skill only if a complex (multi-tool) trajectory succeeded.
    if !trajectoire_merite_skill(user, assistant, n_outils) {
        return Ok(());
    }
    // UNIFIED format with skill_create (build_skill_okf): type/name/description/tools + body.
    let sys = "You are a skill extractor. If the exchange contains a REUSABLE procedure, \
        return ONLY an OKF Markdown document with this EXACT frontmatter: \
        ---\\ntype: skill\\nname: <short-slug>\\ndescription: <10-50 chars, ultra-concise, \
        explicit, starts with a verb in the infinitive>\\ntools: [tools used]\\n--- \
        then a body: '# Title', '## When to use it', '## Procedure' \
        (numbered steps + exact commands), '## Pitfalls'. \
        NOTE on `description`: injected into the LLM context every turn \
        - max 50 chars, explicit (e.g. \\\"search web news\\\"). \
        NOTE on `tools`: list only REAL LaRuche tools \
        (file_read, file_write, file_edit, shell_exec, execute_code, \
        run_script, web_search, web_deep_search, web_fetch, delegate, \
        memory_search, memory_write, cron_create, watcher_create, \
        submit_job, check_job_status, spawn_specialist). \
        If a needed tool doesn't exist, put it in '## Pitfalls' as \
        \\\"tool to create: my_script.py\\\" but NOT in `tools`. \
        NEVER extract a skill from a DIAGNOSTIC DEAD-END or self-investigation: a mission \
        where the agent was confused, hunting for the source of something (a reminder, cron, \
        notification, unexpected state) or troubleshooting LaRuche's own internals is a one-off \
        investigation, NOT a reusable procedure - return NO_SKILL (never 'diagnose_*' or \
        'find_source_*' meta-skills). \
        If nothing generalizable, return NO_SKILL. No text outside the document.";
    let messages = vec![
        serde_json::json!({ "role": "system", "content": sys }),
        serde_json::json!({ "role": "user", "content": format!("User: {user}\nAssistant: {assistant}") }),
    ];
    let mut stream = provider_chat_stream(
        &config.provider,
        config.aux_model.as_deref().unwrap_or(&config.model),
        &messages,
        0.0,
        1400,
        &crate::secrets::substituer(&config.api_key),
        config.api_base.as_deref(),
            &config.ollama_url,
            None,
        ).await?;
    let mut out = String::new();
    while let Some(chunk) = stream.next().await {
        out.push_str(&chunk.text);
    }

    let Some(okf) = extraire_okf_skill(&out) else {
        return Ok(());
    };
    let Some(name) = yaml_frontmatter_field(&okf, "name") else {
        return Ok(());
    };
    let node_id = skill_node_id(&name);
    if let Some(existing) = trouver_skill_existant(memoire, &node_id, &name, &okf).await? {
        memoire.update_item(&existing.item_id, &okf).await?;
        tracing::info!(
            item_id = %existing.item_id,
            node_id = %existing.node_id,
            "existing OKF skill updated"
        );
        return Ok(());
    }

    let _ = memoire
        .propose_write(
            MemoryItem::new(node_id, okf)
                .with_source("auto-skill")
                .with_tags(vec!["skill".to_string(), "okf".to_string()]),
        )
        .await;
    // Learning loop: signal that a skill was just born (UI -> toast + review queue).
    let _ = tx.send(ChatEvent::SkillProposed { name: name.clone() });
    tracing::info!(skill = %name, "OKF skill proposed (auto-learning)");
    Ok(())
}

#[derive(Debug, Clone)]
struct SkillHit {
    item_id: String,
    node_id: String,
}

async fn trouver_skill_existant(
    memoire: &Arc<dyn MemoireCognitive>,
    node_id: &str,
    name: &str,
    okf: &str,
) -> Result<Option<SkillHit>> {
    // Step 1: EXACT match on the node_id.
    if let Ok(node) = memoire.read_node(node_id).await {
        if let Some(hit) = skill_hit_from_items(node["items"].as_array()) {
            return Ok(Some(hit));
        }
    }

    // Step 2: semantic search fallback but verify the node_id
    // matches EXACTLY. Without this, "web-recherche-profonde" would go under "web-research".
    let description = yaml_frontmatter_field(okf, "description").unwrap_or_default();
    let query = format!("capacities.skills {name} {description}");
    let pack = memoire
        .search(
            &query,
            SearchOpts {
                depth: Some(2),
                limit: Some(5),
                sans_trace: false,
            },
        )
        .await?;
    match skill_hit_from_items(pack.raw["items"].as_array()) {
        Some(hit) if hit.node_id == node_id => Ok(Some(hit)),
        _ => Ok(None), // No exact match -> new skill, new node
    }
}

fn skill_hit_from_items(items: Option<&Vec<serde_json::Value>>) -> Option<SkillHit> {
    items?.iter().find_map(|item| {
        let node_id = item
            .get("node_id")
            .or_else(|| item.get("node"))
            .and_then(serde_json::Value::as_str)?;
        if !node_id.starts_with("capacities.skills.") {
            return None;
        }
        let content = item
            .get("content")
            .or_else(|| item.get("text"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
        if !content.contains("type: skill") {
            return None;
        }
        let item_id = item
            .get("id")
            .or_else(|| item.get("item_id"))
            .and_then(serde_json::Value::as_str)?;
        Some(SkillHit {
            item_id: item_id.to_string(),
            node_id: node_id.to_string(),
        })
    })
}

fn extraire_okf_skill(text: &str) -> Option<String> {
    let cleaned = text
        .trim()
        .trim_start_matches("```markdown")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    let start = cleaned.find("---")?;
    let rest = &cleaned[start + 3..];
    let end_rel = rest.find("\n---")?;
    let frontmatter = &cleaned[start + 3..start + 3 + end_rel];
    if !frontmatter.lines().any(|line| {
        let line = line.trim();
        line == "type: skill" || line == "type: \"skill\""
    }) {
        return None;
    }
    Some(cleaned[start..].trim().to_string())
}

/// "Successful complex" trajectory: at least 2 tools chained in the turn and a
/// non-trivial response. This is the condition for a skill to be worth extracting
/// (a skill = a reusable procedure, so typically multi-step).
fn trajectoire_merite_skill(user: &str, reponse: &str, n_outils: usize) -> bool {
    n_outils >= 2 && user.trim().len() >= 12 && reponse.trim().len() >= 120
}

/// Was a private third copy that mangled `-` into `_`, so a skill the curator
/// created landed on a different node than the one the disk sync and the reader
/// used. Now the same function as everywhere else.
use crate::abeilles::skill_node_id;

#[cfg(test)]
mod apprentissage_tests {
    use super::*;

    #[test]
    fn gating_trajectoire_anti_bruit() {
        // No tool -> never a skill, even with a long response.
        assert!(!trajectoire_merite_skill(
            "une demande assez longue",
            &"x".repeat(250),
            0
        ));
        // A single tool -> trajectory too simple for a skill.
        assert!(!trajectoire_merite_skill(
            "une demande assez longue",
            &"x".repeat(250),
            1
        ));
        // >=2 tools chained + substantial response -> skill warranted.
        assert!(trajectoire_merite_skill(
            "une demande assez longue",
            &"x".repeat(250),
            2
        ));
        // 2 tools but trivial response -> no.
        assert!(!trajectoire_merite_skill("ok", "court", 2));
    }
}

#[cfg(test)]
mod tests_node_id {
    use super::node_id_valide;

    #[test]
    fn accepte_les_branches_ou_le_curateur_a_le_droit_d_ecrire() {
        for id in ["people.fabien", "projects.laruche", "decisions.archi", "episodes.2026.mission"] {
            assert!(node_id_valide(id), "{id} devrait etre accepte");
        }
    }

    #[test]
    fn refuse_les_branches_reservees() {
        // Rewritten from elsewhere: the prompt sections and the registry mirrors.
        for id in ["system.notes", "system.prompt", "capacities.tools.x2", "tools.web_fetch", "orphans.vieux_1"] {
            assert!(!node_id_valide(id), "{id} devrait etre refuse");
        }
    }

    #[test]
    fn refuse_les_placeholders_et_les_racines_nues() {
        for id in ["", "system", "people", "people.x", "people alex", "a|b.c"] {
            assert!(!node_id_valide(id), "{id:?} devrait etre refuse");
        }
    }

    #[test]
    fn un_prefixe_qui_ressemble_a_une_racine_reservee_reste_valide() {
        // Segment comparison, not string prefix.
        assert!(node_id_valide("systemes.reseau"));
        assert!(node_id_valide("toolsmith.notes"));
    }
}
