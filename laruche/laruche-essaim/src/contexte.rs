//! Turn context assembly: dynamic tool selection, the ReAct loop entry points
//! (pre-retrieval, working-set assembly, skills catalog/recall), and the
//! facade forwarding into the butinage engine.

use crate::abeille::AbeilleRegistry;
use crate::config::EssaimConfig;
use crate::curation::{curer_memoire, extraire_skill_memoire};
use crate::evenements::{ApprovalReceiver, ChatEvent, SteerReceiver};
use crate::session::Session;
use anyhow::Result;
use laruche_memoire::{MemoireCognitive, SearchOpts};
use std::collections::HashSet;
use std::sync::Arc;

fn tool_disabled(config: &EssaimConfig, name: &str) -> bool {
    config.disabled_tools.iter().any(|t| t == name)
}

fn filtered_tool_schema(registry: &AbeilleRegistry, config: &EssaimConfig) -> serde_json::Value {
    match registry.schema_complet() {
        serde_json::Value::Array(tools) => serde_json::Value::Array(
            tools
                .into_iter()
                .filter(|tool| {
                    tool.get("name")
                        .and_then(serde_json::Value::as_str)
                        .map(|name| !tool_disabled(config, name))
                        .unwrap_or(true)
                })
                .collect(),
        ),
        other => other,
    }
}

/// Lever 2 - core ESSENTIALS always injected (stable, cacheable). Covers ~90% of
/// common tasks so the agent is NEVER blocked (memory, web, shell, files,
/// control). The dynamic queue (`relevant_tools`) adds niche tools by intent
/// (cron, watcher, git, lsp, calendar, image, mixture...). 12 tools vs ~30 before.
const SEMANTIC_CORE: &[&str] = &[
    // Memory & loop control
    "memory_search",
    "memory_write",
    "clarify",
    "todo",
    "run_script",
    "skill_view",
    // Universal tool discovery (retrieval failsafe) - always present.
    "tool_search",
    "tool_call",
    // Common actions - always useful (otherwise the agent can do nothing)
    "web_deep_search",
    "web_fetch",
    "shell_exec",
    "file_read",
    "file_write",
    "file_edit",
    "file_list",
    // Tool creation and reloading
    "reload_plugins",
    // Long background jobs
    "submit_job",
    "check_job_status",
    // Deep research: mode self-declaration + parallel scout fan-out. ALWAYS present:
    // dynamic selection must never strip the orchestration tools right when a
    // narrow-context model starts a research mission.
    "research_mode",
    "delegate",
    // Findings ledger: compaction-proof fact recording. ALWAYS present - a fact not
    // recorded before compaction may be lost from the synthesis.
    "finding",
];

const CORE_TOOL_NAMES: &[&str] = &[
    "memory_search",
    "memory_write",
    "memory_update_item",
    "memory_delete",
    "memory_move_item",
    "memory_review",
    "memory_list_proposed",
    "memory_suggest_nodes",
    "memory_tree",
    "memory_delete_node",
    "memory_create_node",
    "memory_update_node",
    "skill_list",
    "skill_view",
    "file_read",
    "read_extract",
    "file_list",
    "file_write",
    "file_edit",
    "file_search",
    "shell_exec",
    "execute_code",
    "todo",
    "cron_create",
    "cron_list",
    "cron_delete",
    "watcher_create",
    "watcher_list",
    "watcher_delete",
    "session_search",
    "web_deep_search",
    "clarify",
    "run_script",
    "delegate",
    "research_mode",
    "finding",
    "mixture_of_agents",
];

fn tool_name(tool: &serde_json::Value) -> Option<&str> {
    tool.get("name").and_then(serde_json::Value::as_str)
}

fn tool_tokens(text: &str) -> HashSet<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|t| t.len() > 2)
        .map(str::to_string)
        .collect()
}

fn tool_score(
    prompt_tokens: &HashSet<String>,
    prompt_lower: &str,
    tool: &serde_json::Value,
) -> i32 {
    let name = tool_name(tool).unwrap_or_default();
    let mut score = if CORE_TOOL_NAMES.contains(&name) {
        2
    } else {
        0
    };
    if prompt_lower.contains(name) || name.split('_').any(|part| prompt_lower.contains(part)) {
        score += 8;
    }
    let haystack = serde_json::to_string(tool).unwrap_or_default();
    let hay_tokens = tool_tokens(&haystack);
    score += prompt_tokens.intersection(&hay_tokens).count() as i32;
    score
}

/// Return the tool schema to inject for this user prompt.
/// COMPACT index of all capabilities (names by family) for the stable prompt tier:
/// the LLM knows what EXISTS even beyond the tools injected this turn, and can reach anything via
/// `tool_call`. Stable within the session -> cacheable.
pub fn build_capability_index(registry: &AbeilleRegistry, exclude: &HashSet<&str>) -> String {
    let schema = registry.schema_complet();
    let Some(tools) = schema.as_array() else {
        return String::new();
    };
    // Native: NAMES only (~70). Plugins + MCP: NAME - DESCRIPTION (few, custom capabilities).
    // `exclude` = tools ALREADY detailed this turn (section `## Available tools`) -> we do NOT
    // repeat them here, otherwise the same tools are injected twice (signatures + names).
    let mut builtin: Vec<&str> = Vec::new();
    let mut plugins: Vec<(&str, String)> = Vec::new();
    let mut mcp: Vec<(&str, String)> = Vec::new();
    for t in tools {
        let Some(name) = t["name"].as_str().filter(|n| !n.is_empty()) else {
            continue;
        };
        if exclude.contains(name) {
            continue;
        }
        match t["origin"].as_str().unwrap_or("builtin") {
            "custom" => plugins.push((
                name,
                resumer_description(t["description"].as_str().unwrap_or("")),
            )),
            "mcp" => mcp.push((
                name,
                resumer_description(t["description"].as_str().unwrap_or("")),
            )),
            _ => builtin.push(name),
        }
    }
    if builtin.is_empty() && plugins.is_empty() && mcp.is_empty() {
        return String::new();
    }
    builtin.sort_unstable();
    plugins.sort_by(|a, b| a.0.cmp(b.0));
    mcp.sort_by(|a, b| a.0.cmp(b.0));
    let ligne = |out: &mut String, n: &str, d: &str| {
        if d.is_empty() {
            out.push_str(&format!("  - {n}\n"));
        } else {
            out.push_str(&format!("  - {n} - {d}\n"));
        }
    };
    let mut out = String::from(
        "## Tool Catalog\n\nALL tools below are available, even if their schema isn't listed this \
         turn. To use one that isn't in your list: call `tool_call` with `tool` = its name (or \
         `tool_search` to search by keywords).\n",
    );
    if !builtin.is_empty() {
        out.push_str(&format!("- Native tools: {}\n", builtin.join(", ")));
    }
    if !plugins.is_empty() {
        out.push_str("- Plugins:\n");
        for (n, d) in &plugins {
            ligne(&mut out, n, d);
        }
    }
    if !mcp.is_empty() {
        out.push_str("- MCP:\n");
        for (n, d) in &mcp {
            ligne(&mut out, n, d);
        }
    }
    out.push('\n');
    out
}

pub fn schema_outils_pour_prompt(
    registry: &AbeilleRegistry,
    config: &EssaimConfig,
    prompt: &str,
) -> serde_json::Value {
    let enabled = filtered_tool_schema(registry, config);
    if !config.dynamic_tool_selection {
        return enabled;
    }

    let serde_json::Value::Array(tools) = enabled else {
        return enabled;
    };

    // Lever 2 - SEMANTIC selection: minimal core + tools retrieved by intent
    // (from the cognitive map). For a "Hi" -> only the core. For "search the
    // web" -> core + web_*. Constant context cost, unlimited capabilities.
    if let Some(relevant) = &config.relevant_tools {
        let keep: HashSet<&str> = SEMANTIC_CORE
            .iter()
            .copied()
            .chain(relevant.iter().map(String::as_str))
            .collect();
        return serde_json::Value::Array(
            tools
                .into_iter()
                .filter(|t| tool_name(t).map(|n| keep.contains(n)).unwrap_or(false))
                .collect(),
        );
    }

    if tools.len() <= config.tool_selection_limit {
        return serde_json::Value::Array(tools);
    }

    // STABLE profile: query-INDEPENDENT selection (core + deterministic alpha fill).
    // Identical every turn -> cached prefix. Small AND stable.
    if config.stable_toolset {
        let available: HashSet<String> = tools
            .iter()
            .filter_map(|tool| tool_name(tool).map(str::to_string))
            .collect();
        let total_limit = config
            .tool_selection_limit
            .max(CORE_TOOL_NAMES.len())
            .min(tools.len());
        let mut selected: HashSet<String> = CORE_TOOL_NAMES
            .iter()
            .filter(|n| available.contains(**n))
            .map(|n| (*n).to_string())
            .collect();
        let mut names: Vec<String> = available.into_iter().collect();
        names.sort();
        for n in names {
            if selected.len() >= total_limit {
                break;
            }
            selected.insert(n);
        }
        return serde_json::Value::Array(
            tools
                .into_iter()
                .filter(|tool| {
                    tool_name(tool)
                        .map(|name| selected.contains(name))
                        .unwrap_or(false)
                })
                .collect(),
        );
    }

    let prompt_lower = prompt.to_lowercase();
    let prompt_tokens = tool_tokens(&prompt_lower);
    let total_limit = config
        .tool_selection_limit
        .max(CORE_TOOL_NAMES.len())
        .min(tools.len());
    let available: HashSet<String> = tools
        .iter()
        .filter_map(|tool| tool_name(tool).map(str::to_string))
        .collect();
    let mut selected = HashSet::new();
    for name in CORE_TOOL_NAMES {
        if available.contains(*name) {
            selected.insert((*name).to_string());
        }
    }

    let mut scored: Vec<(i32, String)> = tools
        .iter()
        .filter_map(|tool| {
            tool_name(tool).map(|name| {
                (
                    tool_score(&prompt_tokens, &prompt_lower, tool),
                    name.to_string(),
                )
            })
        })
        .collect();
    // Stats (modèle, outil): reliability is a TIEBREAK between equally relevant
    // tools - availability re-ranking only, never an exhortation in the prompt
    // (doctrine: usage signals re-rank, never decide). Unknown (< 3 tries) = neutral.
    let stats = crate::stats_outils::globales();
    let fiab = |name: &str| -> i64 {
        stats
            .fiabilite(&config.model, name)
            .map(|f| (f * 1000.0) as i64)
            .unwrap_or(500)
    };
    scored.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then_with(|| fiab(&b.1).cmp(&fiab(&a.1)))
            .then_with(|| a.1.cmp(&b.1))
    });
    for (score, name) in scored {
        if selected.len() >= total_limit {
            break;
        }
        if score > 0 {
            selected.insert(name);
        }
    }

    // ε-greedy cold start: a FORGED tool (origin `custom`, e.g. built by the
    // curateur) that this model has never tried can never earn a track record if
    // the selection always ignores it. Roughly 1 turn in 4 (deterministic on the
    // prompt: no rand, replayable), give ONE never-tried custom tool a seat.
    if selected.len() < total_limit {
        let mut jamais: Vec<&str> = tools
            .iter()
            .filter(|t| t.get("origin").and_then(|o| o.as_str()) == Some("custom"))
            .filter_map(|t| tool_name(t))
            .filter(|n| !selected.contains(*n) && stats.essais(&config.model, n) == 0)
            .collect();
        if !jamais.is_empty() {
            jamais.sort_unstable();
            let h = {
                use std::hash::{Hash, Hasher};
                let mut h = std::collections::hash_map::DefaultHasher::new();
                prompt.hash(&mut h);
                h.finish() as usize
            };
            if h % 4 == 0 {
                selected.insert(jamais[h % jamais.len()].to_string());
            }
        }
    }

    serde_json::Value::Array(
        tools
            .into_iter()
            .filter(|tool| {
                tool_name(tool)
                    .map(|name| selected.contains(name))
                    .unwrap_or(false)
            })
            .collect(),
    )
}

/// Detects requests where the user explicitly expects a long,
/// exploratory search with several successive strategies. In this mode,
/// an early negative conclusion must not stop the loop.
pub fn demande_recherche_longue(prompt: &str) -> bool {
    let p = prompt.to_lowercase();
    // Keyword FALLBACK only: the reliable channel is the model's own `research_mode`
    // declaration (intercepted by the butinage engine, cycle::analyser). Keep this list
    // broad - a missed match means a 1-search "deep research" (observed: "recherche
    // approfondie" was absent and the agent concluded after a single query).
    [
        "ne t'arrete pas",
        "ne t'arrête pas",
        "jusqu'a",
        "jusqu’à",
        "jusqu'à",
        "tant que",
        "tréfonds",
        "trefonds",
        "très profonde",
        "tres profonde",
        "recherche profonde",
        "recherche très profonde",
        "recherche tres profonde",
        "pendant des heures",
        "des heures",
        "longue recherche",
        "deep research",
        "approfondi", // couvre approfondi/approfondie/approfondir
        "exhaustif",
        "exhaustive",
        "fouillée",
        "fouillee",
        "creuse à fond",
        "creuse a fond",
        "de fond en comble",
        "thorough",
        "in depth",
        "in-depth",
        "deep dive",
        // Multilingue (ES/IT/PT/DE) : le raccourci mots-clés reste un filet ; le canal
        // fiable est `research_mode` que le modèle auto-déclare (language-agnostic).
        "exhaustiv",      // ES/IT/PT exhaustiva/exhaustivo + EN déjà couvert
        "investigaci",    // ES investigación / investigacion
        "investigazione", // IT
        "aprofund",       // PT/ES aprofundada
        "gründlich",      // DE
        "ausführlich",    // DE
        "approfondit",    // IT approfondita
    ]
    .iter()
    .any(|m| p.contains(m))
}

/// The main ReAct loop.
///
/// Flow:
/// 1. Build system prompt with tools schema
/// 2. Stream LLM response (with thinking separation)
/// 3. Handle stop reason: end_turn -> done, tool_use -> execute + loop
/// 4. Auto-compact context if too large
/// 5. Failover to fallback model on error
///
/// Run the ReAct loop (convenience wrapper without images or approval).
// Dependances injectees, toutes distinctes: les regrouper dans une structure ne
// deplacerait la liste que d un cran, en la faisant construire par chaque appelant.
#[allow(clippy::too_many_arguments)]
pub async fn boucle_react(
    prompt_utilisateur: &str,
    session: &mut Session,
    registry: &AbeilleRegistry,
    config: &EssaimConfig,
    tx: &tokio::sync::broadcast::Sender<ChatEvent>,
) -> Result<String> {
    boucle_react_multimodal(
        prompt_utilisateur,
        session,
        registry,
        config,
        tx,
        vec![],
        None,
    )
    .await
}

/// ReAct loop with **automatic cognitive memory** (P2 of the fusion).
///
/// - **Pre-retrieval**: before reasoning, search memory for the memories
///   relevant to the user's intent and inject them into the system
///   instructions. The agent "remembers" without being told to call a tool.
/// - **Post-curation**: after the response, an auxiliary call extracts durable facts
///   and writes them to memory (best-effort, silent on failure -
///   background-review style).
///
/// Backend-agnostic: `SidecarBackend` (paradigm) or `NativeBackend` (Rust), same.
pub async fn boucle_react_memoire(
    prompt_utilisateur: &str,
    session: &mut Session,
    registry: &AbeilleRegistry,
    config: &EssaimConfig,
    tx: &tokio::sync::broadcast::Sender<ChatEvent>,
    memoire: Arc<dyn MemoireCognitive>,
) -> Result<String> {
    boucle_react_memoire_multimodal(
        prompt_utilisateur,
        session,
        registry,
        config,
        tx,
        memoire,
        vec![],
        None,
        None,
    )
    .await
}

/// **Lever 1 - WORKING-SET assembler (first slice).** Instead of a fixed top-N, retrieve
/// broadly then keep the most relevant memories **under a character budget** (~ tokens).
/// The prompt stays stable and small; info is *retrieved* on demand, not accumulated.
/// Foundation: to be enriched (activation/atlas, "recent" + "active node" sources, real token budget).
/// True when a node's one-liner is the auto-generated filler rather than a real
/// summary. Both spellings exist in the wild (the generator changed language).
fn one_liner_generique(one: &str) -> bool {
    let l = one.trim().to_lowercase();
    l.starts_with("subnode of") || l.starts_with("sous-noeud de") || l.starts_with("sous-nœud de")
}

async fn assembler_working_set(
    memoire: &Arc<dyn MemoireCognitive>,
    prompt: &str,
    budget_chars: usize,
) -> Option<(String, Vec<(String, String)>)> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut lignes: Vec<String> = Vec::new();
    // (item_id, content) of the recalled evidence, for post-answer reinforcement.
    let mut rappeles: Vec<(String, String)> = Vec::new();

    // Source 1 - RELEVANCE (semantic/lexical).
    // Filter out INFRASTRUCTURE nodes (`system.*` = sections of the system prompt itself;
    // `capacities.*` = tool/skill catalog): injecting them here DUPLICATED the
    // Behavior section in the "memories" and flooded the working set with `capacities.tools.*` bullets.
    // Relevant skills arrive via a dedicated channel (augmenter_ephemere_avec_skills).
    if let Ok(pack) = memoire
        .search(
            prompt,
            SearchOpts {
                depth: None,
                limit: Some(16),
                sans_trace: true, // hebbian level 2: weight added after use, via renforcer()
            },
        )
        .await
    {
        // `system.*`/`capacities.*` = infrastructure (prompt sections, tool catalog);
        // `orphans.*` = deleted nodes awaiting purge (never relevant as a "memory").
        let infra = |id: &str| {
            id.starts_with("system") || id.starts_with("capacities") || id.starts_with("orphans")
        };
        // Activated nodes (one-liners), excluding infrastructure.
        if let Some(nodes) = pack.raw.get("nodes").and_then(|v| v.as_array()) {
            for n in nodes {
                let id = n
                    .get("id")
                    .or_else(|| n.get("label"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if id.is_empty() || infra(id) {
                    continue;
                }
                let one = n
                    .get("one_liner")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .trim();
                // A bullet with an EMPTY one-liner is pure noise (the name alone, e.g. `decisions.2`, says nothing).
                // The node's real content is injected via its items below - skip the bullet.
                // Auto-generated placeholders say exactly as much as an empty string
                // ("episodes.2026_07_21.test - Subnode of episodes.2026_07_21"): nine such
                // bullets were costing ~200 tokens per turn for zero information.
                if one.is_empty() || one_liner_generique(one) {
                    continue;
                }
                let l = format!("• {id} - {one}");
                if seen.insert(l.trim().to_string()) {
                    lignes.push(l);
                }
            }
        }
        // Evidence items (real content), excluding infrastructure.
        if let Some(items) = pack
            .raw
            .get("items")
            .or_else(|| pack.raw.get("evidence"))
            .and_then(|v| v.as_array())
        {
            for it in items {
                let node = it
                    .get("node_id")
                    .or_else(|| it.get("node"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if infra(node) {
                    continue;
                }
                if let Some(content) = it
                    .get("content")
                    .or_else(|| it.get("text"))
                    .and_then(|v| v.as_str())
                {
                    let l = format!("- {}", content.trim());
                    if !content.trim().is_empty() && seen.insert(l.trim().to_string()) {
                        lignes.push(l);
                        // Hebbian level 2: remember WHICH items were recalled, so
                        // only the ones actually used in the answer get weight.
                        if let Some(id) = it.get("id").and_then(|v| v.as_str()) {
                            rappeles.push((id.to_string(), content.trim().to_string()));
                        }
                    }
                }
            }
        }
    }

    // Source 2 - RECENCY (last facts written, excluding system/tools): approximates activation.
    if let Ok(muts) = memoire.mutations(Some(40)).await {
        if let Some(arr) = muts["mutations"].as_array() {
            for m in arr.iter() {
                if m["op"].as_str() != Some("write") {
                    continue;
                }
                let n = m["node_id"].as_str().unwrap_or("");
                if n.starts_with("capacities") || n.starts_with("system") {
                    continue;
                }
                if let Some(c) = m["content"].as_str() {
                    let key = format!("recent:{}", c.trim());
                    if !c.trim().is_empty() && seen.insert(key) {
                        lignes.push(format!("- {} (recent)", c.trim()));
                    }
                }
            }
        }
    }

    if lignes.is_empty() {
        return None;
    }
    // Selection by character BUDGET (~ tokens): keep in order up to the limit.
    let mut out = String::new();
    for l in lignes {
        if out.len() + l.len() + 1 > budget_chars {
            break;
        }
        out.push_str(&l);
        out.push('\n');
    }
    let out = out.trim_end().to_string();
    if out.is_empty() {
        None
    } else {
        Some((out, rappeles))
    }
}

/// Hebbian level 2, pure decision: which recalled items did the answer actually
/// USE? Deterministic lexical overlap: an item is used when at least two of its
/// significant tokens (>=5 chars, case-insensitive) appear in the answer, or one
/// for very short items. Cheap, testable, and honest enough: unused recalls stop
/// gaining weight, so noise no longer climbs the ranking by mere co-occurrence.
pub(crate) fn rappels_utilises(rappeles: &[(String, String)], reponse: &str) -> Vec<String> {
    // Significant tokens: >=4 chars (so model numbers like `5080` or `vram`
    // count) minus the most common fr/en filler words of that length.
    const VIDES: &[&str] = &[
        "pour", "dans", "avec", "sans", "sont", "vous", "nous", "mais", "plus", "tout",
        "toute", "comme", "leur", "elle", "cette", "fait", "etre", "être", "aussi", "donc",
        "alors", "meme", "même", "bien", "peut", "vers", "chez", "this", "that", "with",
        "from", "your", "have", "will", "been", "were", "they", "than", "then", "when",
        "what", "which", "there", "their", "about", "would", "could",
    ];
    let jetons = |s: &str| -> HashSet<String> {
        s.to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|t| t.chars().count() >= 4 && !VIDES.contains(t))
            .map(String::from)
            .collect()
    };
    let rep = jetons(reponse);
    if rep.is_empty() {
        return Vec::new();
    }
    rappeles
        .iter()
        .filter(|(_, contenu)| {
            let it = jetons(contenu);
            if it.is_empty() {
                return false;
            }
            let communs = it.intersection(&rep).count();
            communs >= 2 || (it.len() <= 3 && communs >= 1)
        })
        .map(|(id, _)| id.clone())
        .collect()
}

/// Multimodal variant of [`boucle_react_memoire`] for the WebSocket UI:
/// keeps images and approval requests while enabling memory.
// Dependances injectees, toutes distinctes: les regrouper dans une structure ne
// deplacerait la liste que d un cran, en la faisant construire par chaque appelant.
#[allow(clippy::too_many_arguments)]
pub async fn boucle_react_memoire_multimodal(
    prompt_utilisateur: &str,
    session: &mut Session,
    registry: &AbeilleRegistry,
    config: &EssaimConfig,
    tx: &tokio::sync::broadcast::Sender<ChatEvent>,
    memoire: Arc<dyn MemoireCognitive>,
    attachments: Vec<crate::session::Attachment>,
    approval_rx: Option<ApprovalReceiver>,
    steer_rx: Option<SteerReceiver>,
) -> Result<String> {
    // 1) Pre-retrieval: inject relevant memories into a cloned config.
    let mut cfg = config.clone();
    cfg.dynamic_tool_selection = true;
    cfg.stable_toolset = true; // stable profile -> cached prefix (combine with trailing memory #1)
    if let Err(e) = indexer_abeilles_memoire(registry, &memoire).await {
        tracing::warn!(error = %e, "Abeille memory indexing skipped");
    }
    // Lever 2 - semantic tools: keep only the core + the Abeilles relevant to
    // the intent (instead of injecting ~30 schemas every turn). Empty for a "Hi".
    let mut abeilles_pertinentes =
        recuperer_abeilles_pertinentes(&memoire, prompt_utilisateur, 6).await;
    // Lexical retrieval failsafe (FR<->EN, accents): force injection of explicitly
    // named tools + the memory box when the intent is "view/organize one's memory".
    for t in outils_forces_par_intention(registry, prompt_utilisateur) {
        if !abeilles_pertinentes.contains(&t) {
            abeilles_pertinentes.push(t);
        }
    }
    {
        // Transparency (UI): the list actually injected = core + retrieved.
        let mut injectes: Vec<String> = SEMANTIC_CORE.iter().map(|s| s.to_string()).collect();
        for t in &abeilles_pertinentes {
            if !injectes.contains(t) {
                injectes.push(t.clone());
            }
        }
        let _ = tx.send(ChatEvent::ToolsSelected { tools: injectes });
    }
    cfg.relevant_tools = Some(abeilles_pertinentes);

    // Editable system base + SOUL: they live in the cognitive map under `system.*`
    // (virtual .md files, OKF format with `enabled` frontmatter). Loaded per turn;
    // if absent/disabled -> fall back to the hardcoded default prompt.
    cfg.system_prompt_override = charger_doc_systeme(&memoire, "system.prompt").await;
    cfg.behavior_override = charger_doc_systeme(&memoire, "system.behavior").await;
    cfg.planning_override = charger_doc_systeme(&memoire, "system.prompt_planning").await;
    if let Some(soul) = charger_doc_systeme(&memoire, "system.soul").await {
        cfg.custom_instructions = Some(soul);
    }
    // User profile (locked node `system.user`): editable by the user ONLY (via their
    // profile), never by the agent (memory_write guard). Injected into context so LaRuche
    // "knows" the user. Read through the same door as the SOUL, so `enabled: false` in
    // its frontmatter switches the injection off. A profile without frontmatter, which is
    // every profile written before this, counts as enabled.
    if let Some(fiche) = charger_doc_systeme(&memoire, "system.user").await {
        let bloc = format!("## About the user (profile they provided)\n{fiche}");
        cfg.custom_instructions = Some(match cfg.custom_instructions.take() {
            Some(s) => format!("{s}\n\n{bloc}"),
            None => bloc,
        });
    }
    // Index of available skills (always present -> the model knows its full repertoire).
    // DYNAMIC skill catalog when the context is narrow (same condition as dynamic tool
    // selection): the semantic DB lists only the relevant skills + a pointer.
    let dyn_skills =
        cfg.dynamic_tool_selection || cfg.context_max_tokens <= cfg.dynamic_context_threshold;
    cfg.skills_index = construire_index_skills(&memoire, prompt_utilisateur, dyn_skills).await;

    // Pre-retrieval -> trailing EPHEMERAL context (NOT in the system prompt:
    // keeps the prefix stable -> hot prefix cache).
    // Lever 1 (first slice): BUDGETED working set instead of a fixed top-N.
    let mut rappels_du_tour: Vec<(String, String)> = Vec::new();
    let ephemeral = match assembler_working_set(&memoire, prompt_utilisateur, 2400).await {
        Some((recall, rappeles)) => {
            let _ = tx.send(ChatEvent::Status {
                message: format!("Memory: working set {} chars.", recall.len()),
            });
            rappels_du_tour = rappeles;
            Some(recall)
        }
        None => None,
    };

    // Automatic recall of learned skills (learning loop): injected into the
    // trailing context with memory, and signaled via SkillApplied.
    let ephemeral =
        augmenter_ephemere_avec_skills(&memoire, prompt_utilisateur, ephemeral, tx).await;

    // Current date/time injected into the VOLATILE (trailing) context - not in the stable
    // prefix, so the prefix cache isn't invalidated every turn. Standard practice: without
    // it the LLM doesn't know "what day it is" (crons, "tomorrow", memory freshness).
    let ephemeral = {
        let entete = format!("[Current date and time: {}]", horodatage_local());
        Some(match ephemeral {
            Some(e) => format!("{entete}\n{e}"),
            None => entete,
        })
    };

    // Snapshot of the number of tools already called (to measure the complexity of THIS turn).
    let tools_avant = compter_tool_calls(session);

    // "NEW MISSION" barrier if the session already has history.
    // Prevents the model from confusing the new request with the old plan.
    let ephemeral = if tools_avant > 0 {
        let barrier = format!(
            "[NEW MISSION - IGNORE the previous plan and steps. \
             This is a new, independent task.]\n{}",
            ephemeral.clone().unwrap_or_default()
        );
        Some(barrier)
    } else {
        ephemeral
    };

    // Normal loop, memory injected as trailing ephemeral context (core unchanged).
    let reponse = boucle_react_multimodal_ext(
        prompt_utilisateur,
        session,
        registry,
        &cfg,
        tx,
        attachments,
        approval_rx,
        steer_rx,
        ephemeral,
        Some(memoire.clone()),
    )
    .await?;

    // Hebbian level 2: of everything recalled into the working set, reinforce
    // ONLY the items whose content actually irrigated the answer (deterministic
    // overlap). Best-effort, cheap SQL.
    if !rappels_du_tour.is_empty() {
        let utilises = rappels_utilises(&rappels_du_tour, &reponse);
        if !utilises.is_empty() {
            if let Ok(n) = memoire.renforcer(&utilises).await {
                tracing::debug!(utilises = n, rappeles = rappels_du_tour.len(), "hebbian level 2");
            }
        }
    }

    // 3) Best-effort background review: the response is already rendered. The reviewer receives
    //    neither the session nor the Abeille registry, only the memory/skill accesses below.
    let n_outils_tour = compter_tool_calls(session).saturating_sub(tools_avant);
    let (user_owned, resp_owned, cfg_owned, mem_owned, tx_owned) = (
        prompt_utilisateur.to_string(),
        reponse.clone(),
        config.clone(),
        memoire.clone(),
        tx.clone(),
    );
    tokio::spawn(async move {
        crate::background_review::run_background_review(
            curer_memoire(&user_owned, &resp_owned, &cfg_owned, &mem_owned),
            extraire_skill_memoire(
                &user_owned,
                &resp_owned,
                &cfg_owned,
                &mem_owned,
                &tx_owned,
                n_outils_tour,
            ),
        )
        .await;
    });

    // LaReine Tier 1 review runs node-side on `Done` (it resolves LaReine's own
    // provider and emits the verdict before the turn closes). See `reine_api`.

    Ok(reponse)
}

/// Capability family of a tool based on its origin (builtin/custom/mcp).
fn famille_capacite(origin: &str) -> &'static str {
    match origin {
        "custom" => "capacities.plugins",
        "mcp" => "capacities.mcp",
        _ => "capacities.tools", // builtin + default
    }
}

/// Index (reconcile) the tool registry into the cognitive map under `capacities.*`,
/// routed by origin: builtin->`capacities.tools`, custom->`capacities.plugins`, mcp->`capacities.mcp`.
/// Incremental: writes only the missing tools. Called at startup AND on the 1st chat turn
/// (failsafe), so any new tool from the code surfaces in memory.
/// Marker of the current projection format. An item written before the schema was
/// fenced and indented does not contain it.
const MARQUEUR_PROJECTION: &str = "```json";

/// Rewrites, in place, the projected items still carrying the pre-fence format.
///
/// The projection is incremental: a tool already indexed is skipped, so a format change
/// would only ever reach tools registered afterwards and the existing 88 would stay
/// unreadable forever. Probed on one node first, so a database already up to date pays
/// a single read and nothing else.
async fn rafraichir_projections(
    registry: &AbeilleRegistry,
    memoire: &Arc<dyn MemoireCognitive>,
) -> usize {
    let schema = registry.schema_complet();
    let Some(tools) = schema.as_array() else {
        return 0;
    };
    let mut refaits = 0usize;
    let mut sonde_faite = false;

    for tool in tools {
        let name = tool["name"].as_str().unwrap_or("");
        if name.is_empty() {
            continue;
        }
        let origin = tool["origin"].as_str().unwrap_or("builtin");
        let node_id = format!("{}.{name}", famille_capacite(origin));
        let Ok(node) = memoire.read_node(&node_id).await else {
            continue;
        };
        let Some(items) = node["items"].as_array() else {
            continue;
        };
        for item in items {
            let (Some(item_id), Some(contenu)) = (
                item["id"].as_str(),
                item["content"].as_str(),
            ) else {
                continue;
            };
            if !contenu.starts_with("Tool `") || contenu.contains(MARQUEUR_PROJECTION) {
                continue;
            }
            let description = tool["description"].as_str().unwrap_or("");
            let neuf = format!(
                "Tool `{name}` ({origin}): {description}\n\nSchema:\n\n```json\n{}\n```",
                serde_json::to_string_pretty(tool).unwrap_or_default()
            );
            if memoire.update_item(item_id, &neuf).await.is_ok() {
                refaits += 1;
            }
        }
        // One node inspected and nothing to redo: the whole projection is current.
        if !sonde_faite {
            sonde_faite = true;
            if refaits == 0 {
                return 0;
            }
        }
    }
    if refaits > 0 {
        tracing::info!(items = refaits, "Tool projections rewritten in the current format");
    }
    refaits
}

pub async fn indexer_abeilles_memoire(
    registry: &AbeilleRegistry,
    memoire: &Arc<dyn MemoireCognitive>,
) -> Result<()> {
    rafraichir_projections(registry, memoire).await;
    // INCREMENTAL reconciliation: ids already indexed under the 3 tool families.
    let mut deja: std::collections::HashSet<String> = std::collections::HashSet::new();
    for parent in ["capacities.tools", "capacities.plugins", "capacities.mcp"] {
        if let Ok(node) = memoire.read_node(parent).await {
            if let Some(children) = node["children"].as_array() {
                for child in children {
                    if let Some(id) = child["id"].as_str().or_else(|| child["node_id"].as_str()) {
                        deja.insert(id.to_string());
                    }
                }
            }
        }
    }

    let schema = registry.schema_complet();
    let Some(tools) = schema.as_array() else {
        return Ok(());
    };

    let mut ajoutes = 0usize;
    for tool in tools {
        let name = tool["name"].as_str().unwrap_or("unknown");
        let origin = tool["origin"].as_str().unwrap_or("builtin");
        let node_id = format!("{}.{name}", famille_capacite(origin));
        if deja.contains(&node_id) {
            continue; // already indexed -> no duplicate
        }
        let description = tool["description"].as_str().unwrap_or("");
        // Fenced and indented: the schema used to be one compact line glued to the
        // description, so the memory view rendered it as a wall of prose and a reader
        // could not tell an argument from a sentence. The fence also stops a `{` from
        // being read as markdown.
        let content = format!(
            "Tool `{name}` ({origin}): {description}\n\nSchema:\n\n```json\n{}\n```",
            serde_json::to_string_pretty(tool).unwrap_or_default()
        );
        let _ = memoire
            .write(laruche_memoire::MemoryItem::new(node_id, content).with_source("tool-registry"))
            .await;
        ajoutes += 1;
    }

    // Reconcile DELETIONS for VOLATILE families (plugins/mcp): pure projections
    // of the registry. Any node `capacities.{plugins,mcp}.<name>` with no matching tool = a removed
    // capability (e.g. deleted plugin) -> remove it. Guard: only reconcile a family IF
    // the registry contains at least one (avoids purging everything at boot before MCPs load).
    let mut valides: std::collections::HashSet<String> = std::collections::HashSet::new();
    let (mut a_plugins, mut a_mcp) = (false, false);
    for tool in tools {
        let name = tool["name"].as_str().unwrap_or("");
        let origin = tool["origin"].as_str().unwrap_or("builtin");
        valides.insert(format!("{}.{name}", famille_capacite(origin)));
        match origin {
            "custom" => a_plugins = true,
            "mcp" => a_mcp = true,
            _ => {}
        }
    }
    for (parent, actif) in [("capacities.plugins", a_plugins), ("capacities.mcp", a_mcp)] {
        if !actif {
            continue;
        }
        if let Ok(node) = memoire.read_node(parent).await {
            if let Some(children) = node["children"].as_array() {
                for child in children {
                    let Some(id) = child["id"].as_str().or_else(|| child["node_id"].as_str())
                    else {
                        continue;
                    };
                    if valides.contains(id) {
                        continue;
                    }
                    if let Ok(r) = memoire.delete_node(id).await {
                        if let Some(orphan) = r.get("relocated_to").and_then(|v| v.as_str()) {
                            let _ = memoire.delete_node(orphan).await; // hard-delete the orphan
                        }
                    }
                }
            }
        }
    }

    if ajoutes > 0 {
        // A LOG, not a memory. It used to be written as an item on the capacities.tools
        // root, one per startup, piling up forever in a node a search can hand back as
        // if it were a souvenir. It says nothing the tool nodes do not already say.
        tracing::info!(
            total = tools.len(),
            added = ajoutes,
            "Capability index updated"
        );
    }
    Ok(())
}

/// Retrieve the OKF skills relevant to the query (automatic recall of the
/// learning loop): items under `capacities.skills.*` (frontmatter `type: skill`)
/// close to the user prompt.
/// Lever 2 - retrieve the NAMES of Abeilles relevant to the intent, from the cognitive
/// map (`tools.abeilles.*`, indexed by `indexer_abeilles_memoire`). Empty if nothing is
/// relevant (e.g. a greeting) -> only the core is injected.
/// Tools to inject unconditionally, regardless of semantic retrieval (which misses on FR<->EN
/// and accents). (1) Any tool whose **exact name** appears in the prompt
/// ("use memory_tree"). (2) The **memory toolbox** whenever the intent is about
/// viewing/organizing/cleaning/merging memory or nodes. Returns only actually
/// registered names.
fn outils_forces_par_intention(registry: &AbeilleRegistry, prompt: &str) -> Vec<String> {
    let p = prompt.to_lowercase();
    let noms = registry.noms();
    let mut forces: Vec<String> = Vec::new();

    // (1) Tools cited explicitly by name.
    for nom in &noms {
        if p.contains(nom.to_lowercase().as_str()) {
            forces.push(nom.clone());
        }
    }

    // (2) Cognitive memory management intent.
    const MOTS_MEMOIRE: &[&str] = &[
        "memoire",
        "mémoire",
        "node",
        "noeud",
        "nœud",
        "carte cognitive",
        "range",
        "ranger",
        "rangé",
        "nettoie",
        "nettoy",
        "fusionn",
        "organise",
        "rganise", // (re)organise / reorganise
        "souvenir",
    ];
    if MOTS_MEMOIRE.iter().any(|m| p.contains(m)) {
        const BOITE_MEMOIRE: &[&str] = &[
            "memory_tree",
            "memory_search",
            "memory_write",
            "memory_create_node",
            "memory_update_node",
            "memory_delete_node",
            "memory_move_item",
            "memory_update_item",
            "memory_delete",
            "memory_stats",
        ];
        for t in BOITE_MEMOIRE {
            if noms.iter().any(|n| n.as_str() == *t) {
                forces.push((*t).to_string());
            }
        }
    }

    // (3) Capability CREATION intent (skill / tool / plugin) -> forge box.
    const MOTS_FORGE: &[&str] = &[
        "skill",
        "outil",
        "plugin",
        "forge",
        "crée",
        "cree",
        "créer",
        "creer",
        "automatise",
        "script",
        "procédure",
        "procedure",
    ];
    if MOTS_FORGE.iter().any(|m| p.contains(m)) {
        const BOITE_FORGE: &[&str] = &[
            "skill_create",
            "skill_patch",
            "skill_view",
            "skill_list",
            "skill_file_write",
            "plugin_create",
            "plugin_list",
        ];
        for t in BOITE_FORGE {
            if noms.iter().any(|n| n.as_str() == *t) {
                forces.push((*t).to_string());
            }
        }
    }

    forces
}

/// Readable local timestamp for prompt injection (e.g. "21/06/2026 14:32").
/// Neutral format (no day/month names -> avoids English in a FR prompt).
/// Clock line for the volatile tier, in English like the rest of the prompt.
///
/// `27/07/2026 00:29` alone was routinely ignored: it is ambiguous (day/month or
/// month/day depending on what the model saw at training time), it carries no
/// timezone, and above all no WEEKDAY, which rules like `jour_semaine` need. The
/// spelled-out weekday plus the ISO form together are what small models honour.
fn horodatage_local() -> String {
    let now = chrono::Local::now();
    format!(
        "{} ({})",
        now.format("%A %-d %B %Y, %H:%M local time"),
        now.format("%Y-%m-%dT%H:%M:%S%:z")
    )
}

/// Separate the OKF frontmatter (`--- ... ---`) from the body and read the `enabled` flag
/// (default on). Returns `(active, body)`.
fn parser_frontmatter_enabled(content: &str) -> (bool, String) {
    let c = content.trim_start();
    if let Some(rest) = c.strip_prefix("---") {
        if let Some(end) = rest.find("\n---") {
            let fm = &rest[..end];
            let body = &rest[end + 4..];
            let desactive = fm.lines().any(|l| {
                let l = l.trim().to_lowercase().replace(' ', "");
                l == "enabled:false" || l == "active:false"
            });
            return (!desactive, body.trim_start_matches('\n').to_string());
        }
    }
    (true, content.to_string())
}

/// Load a system document (`system.prompt`, `system.soul`) from the cognitive map:
/// take the node's last item, read its frontmatter. Returns the body if enabled and non-empty.
pub async fn charger_doc_systeme(
    memoire: &Arc<dyn MemoireCognitive>,
    node_id: &str,
) -> Option<String> {
    let node = memoire.read_node(node_id).await.ok()?;
    let items = node.get("items")?.as_array()?;
    let content = items
        .iter()
        .rev()
        .find_map(|it| it.get("content").and_then(|c| c.as_str()))?;
    let (actif, corps) = parser_frontmatter_enabled(content);
    if actif && !corps.trim().is_empty() {
        Some(corps.trim().to_string())
    } else {
        None
    }
}

async fn recuperer_abeilles_pertinentes(
    memoire: &Arc<dyn MemoireCognitive>,
    query: &str,
    limit: usize,
) -> Vec<String> {
    // Scoped to the tools subtree so the abeilles aren't crowded out by
    // memory content (notes, projects...) in the ranking.
    let pack = match memoire
        .search(
            &format!("capacities {query}"),
            SearchOpts {
                depth: Some(2),
                limit: Some(20),
                sans_trace: false,
            },
        )
        .await
    {
        Ok(p) => p,
        Err(_) => return Vec::new(),
    };
    let mut out: Vec<String> = Vec::new();
    if let Some(items) = pack.raw["items"].as_array() {
        for item in items {
            let node_id = item
                .get("node_id")
                .or_else(|| item.get("node"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            // Tools = families capacities.tools / capacities.plugins / capacities.mcp (not skills).
            let name = [
                "capacities.tools.",
                "capacities.plugins.",
                "capacities.mcp.",
            ]
            .iter()
            .find_map(|p| node_id.strip_prefix(p));
            if let Some(name) = name {
                if !name.is_empty() && !out.iter().any(|n| n == name) {
                    out.push(name.to_string());
                    if out.len() >= limit {
                        break;
                    }
                }
            }
        }
    }
    out
}

/// Lexical gate: a skill recalled by fuzzy match is injected only if the query shares
/// a significant token (>=4 chars) with its name or description. Prevents an off-topic skill
/// from being injected on a vague query (e.g. `google-workspace` on "and about the 6?"). Web
/// searches go through the FORCED path (`intention_recherche`): this gate doesn't
/// penalize them.
fn skill_pertinent_lexical(query: &str, name: &str, content: &str) -> bool {
    let q = query.to_lowercase();
    let tokens: Vec<&str> = q
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| t.chars().count() >= 4)
        .collect();
    if tokens.is_empty() {
        return false; // query too vague -> no fuzzy skill
    }
    let desc = yaml_frontmatter_field(content, "description").unwrap_or_default();
    let haystack = format!("{name} {desc}").to_lowercase();
    tokens.iter().any(|t| haystack.contains(t))
}

/// Reduce a skill description to a short, readable summary for the index:
/// 1) pattern `Summary - details` -> keep the summary (before the separator);
/// 2) otherwise, the first sentence if it fits;
/// 3) soft cap ~80 chars, cut at the word boundary (never mid-word), `…` if truncated.
pub(crate) fn resumer_description(desc: &str) -> String {
    let d = desc.split_whitespace().collect::<Vec<_>>().join(" ");
    let base = if let Some(i) = d.find(" - ") {
        &d[..i]
    } else if let Some(i) = d.find(". ") {
        &d[..i]
    } else {
        d.as_str()
    }
    .trim();
    if base.chars().count() <= 80 {
        return base.to_string();
    }
    let mut cut = String::new();
    for w in base.split(' ') {
        if cut.chars().count() + w.chars().count() + 1 > 78 {
            break;
        }
        if !cut.is_empty() {
            cut.push(' ');
        }
        cut.push_str(w);
    }
    format!("{cut}…")
}

/// COMPACT index of ALL available skills (`name - description`), always injected in the
/// stable prefix. Without it, imported skills are invisible to the model (it only sees one
/// via fuzzy recall). The full body stays on demand via `skill_view(name)` - progressive
/// disclosure. Built in ONE `search` (query-independent: all skills
/// contain `type: skill`).
async fn construire_index_skills(
    memoire: &Arc<dyn MemoireCognitive>,
    query: &str,
    dynamic: bool,
) -> Option<String> {
    // ENUMERATE the node tree, never `search()`. Both branches of the search
    // deliberately exclude `capacities.%` (they are system projections with their
    // own injection channel, and their long bodies used to drown real facts in
    // recall). Building the catalog on top of that search meant asking a channel
    // to return exactly what it is designed to filter out: it always answered
    // nothing, the section vanished from the prompt, and the model could not know
    // a single one of its 73 skills existed. `skill_list` reads the tree, so do we.
    let racine = memoire.read_node("capacities.skills").await.ok()?;
    let enfants = racine["children"].as_array()?;

    let mut lignes: Vec<(String, String)> = Vec::new();
    let mut vus: HashSet<String> = HashSet::new();
    for enfant in enfants {
        let Some(node_id) = enfant
            .get("id")
            .or_else(|| enfant.get("node_id"))
            .and_then(|v| v.as_str())
        else {
            continue;
        };
        // Displayed name = SLUG (node_id suffix): the identifier that `skill_view(name)` resolves.
        // (The frontmatter name may differ, e.g. `arxiv-search` vs node `arxiv_search`.)
        let Some(name) = node_id.strip_prefix("capacities.skills.").map(str::to_string) else {
            continue;
        };
        if name.is_empty() || name.contains('.') {
            continue; // direct skills only
        }
        if !vus.insert(name.clone()) {
            continue;
        }
        // The child entry carries no description, so read the skill document itself.
        // Local SQLite reads, and the result lives in the stable prefix.
        let Ok(node) = memoire.read_node(node_id).await else {
            continue;
        };
        let Some(doc) = node["items"].as_array().and_then(|items| {
            items.iter().rev().find_map(|it| {
                let c = it["content"].as_str()?;
                c.contains("type: skill").then_some(c)
            })
        }) else {
            // Node without a skill document: an empty shell left behind by the old
            // `-` vs `_` mismatch. Advertising a name that `skill_view` cannot open
            // is worse than not advertising it.
            continue;
        };
        let desc = yaml_frontmatter_field(doc, "description")
            .map(|d| resumer_description(&d))
            .unwrap_or_default();
        lignes.push((name, desc));
    }
    if lignes.is_empty() {
        return None;
    }
    let total = lignes.len();
    lignes.sort();

    // The compact catalog (name + one-line description, ~2k tokens for ~70 skills)
    // is cheap; it is the skill BODIES that stay lazy (`skill_view` on demand).
    // Filtering the LIST itself by query tokens made most skills invisible to the
    // model on every real request (the user only saw 0-12 entries). So: FULL
    // catalog for any non-trivial request, whatever the context width; `dynamic`
    // only keeps the smalltalk shortcut (pointer alone, zero listing).
    if dynamic {
        let q = query.split("[SYSTEM]").next().unwrap_or(query).to_lowercase();
        if requete_triviale(&q) {
            return Some(format!(
                "## Available skills\n\n{total} reusable skill procedures are available - call \
                 `skill_list` to browse them or `skill_view(name)` to read one.\n\n"
            ));
        }
    }

    let mut out = String::from(
        "## Available skills\n\nReusable procedures. To apply the full procedure of one, \
         call `skill_view(name)`.\n",
    );
    for (n, d) in lignes {
        if d.is_empty() {
            out.push_str(&format!("- {n}\n"));
        } else {
            out.push_str(&format!("- {n} - {d}\n"));
        }
    }
    out.push('\n');
    Some(out)
}

async fn recuperer_skills_pertinents(
    memoire: &Arc<dyn MemoireCognitive>,
    query: &str,
    limit: usize,
) -> Vec<(String, String)> {
    let pack = match memoire
        .search(
            &format!("capacities.skills {query}"),
            SearchOpts {
                depth: Some(2),
                limit: Some(8),
                sans_trace: false,
            },
        )
        .await
    {
        Ok(p) => p,
        Err(_) => return Vec::new(),
    };
    let mut out: Vec<(String, String)> = Vec::new();
    if let Some(items) = pack.raw["items"].as_array() {
        for item in items {
            let node_id = item
                .get("node_id")
                .or_else(|| item.get("node"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            if !node_id.starts_with("capacities.skills.") {
                continue;
            }
            let content = item
                .get("content")
                .or_else(|| item.get("text"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            if !content.contains("type: skill") {
                continue;
            }
            let name = yaml_frontmatter_field(content, "name")
                .unwrap_or_else(|| node_id.trim_start_matches("capacities.skills.").to_string());
            if out.iter().any(|(n, _)| n == &name) {
                continue;
            }
            // Relevance gate: ignore off-topic fuzzy matches (noise on a vague query).
            if !skill_pertinent_lexical(query, &name, content) {
                continue;
            }
            out.push((name, content.to_string()));
            if out.len() >= limit {
                break;
            }
        }
    }
    out
}

/// Format the recalled skills at the head of the trailing context and emit `SkillApplied`
/// for each. Pure (testable without backend or LLM).
fn formater_et_signaler_skills(
    skills: &[(String, String)],
    ephemeral: Option<String>,
    tx: &tokio::sync::broadcast::Sender<ChatEvent>,
) -> Option<String> {
    if skills.is_empty() {
        return ephemeral;
    }
    // Per-skill budget: imported skills can weigh 10-20 KB. Injecting the full
    // body would drown the working set (seen in prod: a ~15 KB CLI reference skill on a
    // "world models" query). Cap at ~1600 chars + a skill_view pointer for detail (progressive
    // disclosure: the LLM reads the summary and calls skill_view if it needs everything).
    const BUDGET_SKILL: usize = 1600;
    let mut bloc = String::from("# Learned skills applicable to this task\n\n");
    for (name, body) in skills {
        let _ = tx.send(ChatEvent::SkillApplied { name: name.clone() });
        let nom = name.trim();
        let corps_complet = body.trim();
        let corps = if corps_complet.chars().count() > BUDGET_SKILL {
            let tronque: String = corps_complet.chars().take(BUDGET_SKILL).collect();
            format!("{tronque}\n\n… (truncated - `skill_view(\"{nom}\")` for the full procedure)")
        } else {
            corps_complet.to_string()
        };
        bloc.push_str(&format!("## Skill: {nom}\n{corps}\n\n---\n\n"));
    }
    Some(match ephemeral {
        Some(mem) => format!("{bloc}{mem}"),
        None => bloc,
    })
}

/// Web search intent (triggers the `web_research` skill unconditionally).
fn intention_recherche(query: &str) -> bool {
    let p = query.to_lowercase();
    const MOTS: &[&str] = &[
        "recherche",
        "approfond",
        "cherche",
        "renseigne",
        "trouve",
        "web",
        "internet",
        "actu",
        "news",
        "source",
        "documente",
        "veille",
        "compare",
        "qui est",
        "quoi de neuf",
    ];
    MOTS.iter().any(|m| p.contains(m))
}

/// Load a skill's OKF body (last `type: skill` item of the node).
async fn charger_skill_corps(memoire: &Arc<dyn MemoireCognitive>, node_id: &str) -> Option<String> {
    let node = memoire.read_node(node_id).await.ok()?;
    let items = node.get("items")?.as_array()?;
    items
        .iter()
        .rev()
        .find_map(|it| it.get("content").and_then(|c| c.as_str()))
        .filter(|c| c.contains("type: skill"))
        .map(|c| c.to_string())
}

/// Automatic recall: find the relevant skills, inject them into the trailing
/// ephemeral context and signal them. On a search intent, FORCE the `web_research`
/// skill (otherwise the "web_deep_search in a loop" reflex wins).
/// True if the message is smalltalk (greeting/thanks/"test"...) - no skill body
/// should be injected (the `## Available skills` catalog suffices, `skill_view` on demand).
pub(crate) fn requete_triviale(q: &str) -> bool {
    let q = q.trim().to_lowercase();
    let toks: Vec<&str> = q
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .collect();
    const SMALLTALK: &[&str] = &[
        "salut", "bonjour", "bonsoir", "coucou", "hello", "hi", "hey", "yo", "test", "merci",
        "thanks", "thx", "ok", "oui", "non", "cc", "slt", "ca", "ça", "va", "comment", "vas",
        "tu", "bien", "yep", "nope", "stp", "svp",
    ];
    !toks.is_empty() && toks.iter().all(|t| SMALLTALK.contains(t))
}

async fn augmenter_ephemere_avec_skills(
    memoire: &Arc<dyn MemoireCognitive>,
    query: &str,
    ephemeral: Option<String>,
    tx: &tokio::sync::broadcast::Sender<ChatEvent>,
) -> Option<String> {
    // The relevance query = the REAL user message, WITHOUT the `[SYSTEM] ...` suffix added by the
    // channel (it contains "plan/monitor/search" which matched cron_manager and
    // force-injected web_research EVERY TURN = wasted context). And no skill body for
    // smalltalk: the names+descriptions catalog suffices, the model uses `skill_view` as needed.
    let query = query.split("[SYSTEM]").next().unwrap_or(query).trim();
    if requete_triviale(query) {
        return ephemeral;
    }
    // Lazy default: inject only the single MOST relevant skill body. The full names+descriptions
    // catalog is always present, and the model pulls any other skill on demand via skill_view.
    // Eagerly injecting several bodies wasted context for little gain.
    let mut skills = recuperer_skills_pertinents(memoire, query, 1).await;
    // The web-research skill, whichever separator its folder uses. The node id was
    // hardcoded with an underscore, and `skill_node_id` PRESERVES hyphens: the day the
    // folder was renamed `web-research`, the two stopped meeting and this injection
    // silently became a no-op on every search. Candidates cover both spellings.
    const SKILL_RECHERCHE: &str = "web-research";
    let deja_la = |n: &str| n.replace('_', "-") == SKILL_RECHERCHE;
    if intention_recherche(query) && !skills.iter().any(|(n, _)| deja_la(n)) {
        for node in crate::abeilles::skill_node_id_candidates(SKILL_RECHERCHE) {
            if let Some(body) = charger_skill_corps(memoire, &node).await {
                skills.insert(0, (SKILL_RECHERCHE.to_string(), body));
                break;
            }
        }
    }
    formater_et_signaler_skills(&skills, ephemeral, tx)
}

/// Count the tool calls recorded in the session.
fn compter_tool_calls(session: &Session) -> usize {
    session
        .messages
        .iter()
        .filter(|m| matches!(m, crate::session::Message::ToolCall { .. }))
        .count()
}

#[cfg(test)]
mod apprentissage_tests {
    use super::*;

    #[test]
    fn hebbien_2_ne_renforce_que_les_rappels_utilises() {
        let rappeles = vec![
            (
                "itm_1".to_string(),
                // The GPU model has to stay: the match is on shared tokens with the
                // answer below, and stripping it makes the item look unused.
                "Alex utilise une carte graphique RTX 5080 avec 16 Go de VRAM".to_string(),
            ),
            (
                "itm_2".to_string(),
                "Le chat de la voisine s'appelle Filou et adore les croquettes".to_string(),
            ),
            ("itm_3".to_string(), "Broken Sword".to_string()), // short item: 1 shared token suffices
        ];
        let reponse = "Ta config actuelle repose sur la RTX 5080 et ses 16 Go de VRAM, \
                       largement suffisante pour les recherches sur Broken Sword.";
        let utilises = rappels_utilises(&rappeles, reponse);
        assert!(utilises.contains(&"itm_1".to_string()), "{utilises:?}");
        assert!(utilises.contains(&"itm_3".to_string()), "{utilises:?}");
        assert!(
            !utilises.contains(&"itm_2".to_string()),
            "unused recall must NOT gain weight: {utilises:?}"
        );
        // Empty answer reinforces nothing.
        assert!(rappels_utilises(&rappeles, "").is_empty());
    }

    #[test]
    fn rappel_skill_injecte_et_signale() {
        let (tx, mut rx) = tokio::sync::broadcast::channel(16);
        let skills = vec![(
            "veille-ia".to_string(),
            "---\ntype: skill\nname: veille-ia\n---\n## Step: utilise web_deep_search".to_string(),
        )];
        let out = formater_et_signaler_skills(&skills, Some("souvenir X".into()), &tx)
            .expect("non-empty context when a skill is recalled");
        assert!(out.contains("## Skill: veille-ia"), "skill injected");
        assert!(
            out.contains("souvenir X"),
            "memory preserved after the skills block"
        );
        match rx.try_recv() {
            Ok(ChatEvent::SkillApplied { name }) => assert_eq!(name, "veille-ia"),
            other => panic!("expected SkillApplied, got {other:?}"),
        }
    }

    #[test]
    fn sans_skill_le_contexte_reste_inchange() {
        let (tx, _rx) = tokio::sync::broadcast::channel(4);
        assert_eq!(
            formater_et_signaler_skills(&[], Some("m".into()), &tx),
            Some("m".to_string())
        );
        assert_eq!(formater_et_signaler_skills(&[], None, &tx), None);
    }
}

pub async fn boucle_react_multimodal(
    prompt_utilisateur: &str,
    session: &mut Session,
    registry: &AbeilleRegistry,
    config: &EssaimConfig,
    tx: &tokio::sync::broadcast::Sender<ChatEvent>,
    attachments: Vec<crate::session::Attachment>,
    approval_rx: Option<ApprovalReceiver>,
) -> Result<String> {
    boucle_react_multimodal_ext(
        prompt_utilisateur,
        session,
        registry,
        config,
        tx,
        attachments,
        approval_rx,
        None,
        None,
        None,
    )
    .await
}

/// The main ReAct loop.
/// Supports multimodal, approval, and a **trailing ephemeral context**
/// (memory) injected AFTER the history: the system prompt (prefix) stays stable
/// -> hot upstream prefix cache.
// Dependances injectees, toutes distinctes: les regrouper dans une structure ne
// deplacerait la liste que d un cran, en la faisant construire par chaque appelant.
#[allow(clippy::too_many_arguments)]
pub async fn boucle_react_multimodal_ext(
    prompt_utilisateur: &str,
    session: &mut Session,
    registry: &AbeilleRegistry,
    config: &EssaimConfig,
    tx: &tokio::sync::broadcast::Sender<ChatEvent>,
    attachments: Vec<crate::session::Attachment>,
    approval_rx: Option<ApprovalReceiver>,
    steer_rx: Option<SteerReceiver>,
    ephemeral_context: Option<String>,
    memoire: Option<Arc<dyn laruche_memoire::MemoireCognitive>>,
) -> Result<String> {
    // The legacy ReAct loop was REMOVED (2026-07-02) once butinage became the
    // default engine. This facade forwards every entry point (chat, channels,
    // missions, Reine reruns) to the bridge; RUCHE_MOTEUR=brain only logs a
    // deprecation warning now (see moteur_butinage_actif).
    let _ = crate::butinage_pont::moteur_butinage_actif();
    crate::butinage_pont::executer(
        prompt_utilisateur,
        session,
        registry,
        config,
        tx,
        &ephemeral_context,
        &memoire,
        steer_rx,
        &attachments,
        approval_rx,
    )
    .await
}

/// Separate the OKF frontmatter (`--- ... ---`) from the body and read a field's raw value.
pub(crate) fn yaml_frontmatter_field(markdown: &str, key: &str) -> Option<String> {
    let rest = markdown.trim_start().strip_prefix("---")?;
    let end = rest.find("\n---")?;
    let mut lignes = rest[..end].lines().peekable();
    while let Some(line) = lignes.next() {
        // Ignore lines without `:` (the 1st line after `---` is empty). Do NOT `?` here: it
        // made ALL parsing fail at the empty line -> name/description always None.
        let Some((k, v)) = line.split_once(':') else {
            continue;
        };
        if k.trim() != key {
            continue;
        }
        let v = v.trim();
        // YAML BLOCK SCALAR: the value lives on the following indented lines, and the
        // marker alone is not it. Returning it verbatim published a literal ">-" as the
        // description of every skill written that way, i.e. the whole prompt catalog.
        if matches!(v, ">" | ">-" | ">+" | "|" | "|-" | "|+") {
            let plie = v.starts_with('>');
            let mut morceaux: Vec<String> = Vec::new();
            while let Some(suite) = lignes.peek() {
                // The block ends at the first line that is blank or not indented.
                if suite.trim().is_empty() || !suite.starts_with([' ', '\t']) {
                    break;
                }
                morceaux.push(suite.trim().to_string());
                lignes.next();
            }
            // Folded (`>`) joins with spaces, literal (`|`) keeps the line breaks.
            return Some(morceaux.join(if plie { " " } else { "\n" }));
        }
        return Some(v.trim_matches('"').trim_matches('\'').to_string());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yaml_frontmatter_deplie_les_scalaires_de_bloc() {
        // Regression: `description: >-` returned the MARKER, so every skill written
        // that way published a literal ">-" as its description in the prompt catalog.
        let md = "---\ntype: skill\nname: cognitive-memory\ndescription: >-\n  Store and \
                  curate lasting facts.\nprerequisites:\n  commands: [jq]\n---\n\n# Body";
        assert_eq!(
            yaml_frontmatter_field(md, "description").as_deref(),
            Some("Store and curate lasting facts.")
        );
        // The block must stop at the next unindented key, not swallow it.
        assert_eq!(
            yaml_frontmatter_field(md, "name").as_deref(),
            Some("cognitive-memory")
        );

        // Folded over several lines joins with spaces; literal keeps the breaks.
        let plie = "---\ndescription: >-\n  one\n  two\n---\n";
        assert_eq!(yaml_frontmatter_field(plie, "description").as_deref(), Some("one two"));
        let litteral = "---\ndescription: |\n  one\n  two\n---\n";
        assert_eq!(yaml_frontmatter_field(litteral, "description").as_deref(), Some("one\ntwo"));

        // Inline form keeps working, quotes stripped.
        let inline = "---\ndescription: \"Plain one liner\"\n---\n";
        assert_eq!(
            yaml_frontmatter_field(inline, "description").as_deref(),
            Some("Plain one liner")
        );
    }

    #[test]
    fn yaml_frontmatter_lit_apres_ligne_vide() {
        // Regression: the 1st line after `---` is empty, must NOT break parsing.
        let md = "---\ntype: skill\nname: arxiv-search\ndescription: Recherche de papiers sur arxiv.org\n---\n\n# Corps";
        assert_eq!(
            yaml_frontmatter_field(md, "name").as_deref(),
            Some("arxiv-search")
        );
        assert_eq!(
            yaml_frontmatter_field(md, "description").as_deref(),
            Some("Recherche de papiers sur arxiv.org")
        );
        assert_eq!(yaml_frontmatter_field(md, "type").as_deref(), Some("skill"));
    }

    #[test]
    fn resumer_description_garde_le_resume_avant_tiret() {
        // Pattern "Summary - details": keep the summary.
        let comfyui = "Generate images, video, and audio with ComfyUI - install, launch, manage nodes/models, run workflows with parameter injection. Uses the official API.";
        assert_eq!(
            resumer_description(comfyui),
            "Generate images, video, and audio with ComfyUI"
        );
        // Without a dash: first sentence.
        let plan = "Plan mode: write an actionable markdown plan to .laruche/plans/, no execution. Bite-sized tasks, exact paths, complete code.";
        assert_eq!(
            resumer_description(plan),
            "Plan mode: write an actionable markdown plan to .laruche/plans/, no execution"
        );
        // Already short: unchanged.
        assert_eq!(
            resumer_description("Recherche de papiers sur arxiv.org via des requêtes structurées"),
            "Recherche de papiers sur arxiv.org via des requêtes structurées"
        );
        // Long without a separator: cut at the word + ellipsis, never > ~80.
        let long = "mot ".repeat(40);
        let r = resumer_description(&long);
        assert!(r.ends_with('…') && r.chars().count() <= 80);
    }
}
