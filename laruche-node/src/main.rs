//! LaRuche Node Daemon
//!
//! The main process that runs on each LaRuche box. It:
//! 1. Broadcasts its Cognitive Manifest via Miel (mDNS)
//! 2. Listens for peer nodes (swarm)
//! 3. Exposes an inference API (proxying to Ollama)
//! 4. Manages authentication via Proof of Proximity
//! 5. Runs the web dashboard
//! 6. Exposes /models to list available Ollama models
//! 7. Reports real system metrics (CPU, RAM) via sysinfo
//! 8. Exposes MCP server for external AI clients
//! 9. Discord & Slack channel integrations

mod auth_user;
mod mcp;
mod profiles;
mod sync;
mod systray;
mod tui;

use anyhow::Result;
use axum::{
    extract::{ConnectInfo, State, WebSocketUpgrade, ws},
    http::StatusCode,
    response::{Html, Json},
    routing::{get, post},
    Router,
};
use miel_protocol::{
    auth::ProximityAuth,
    capabilities::{Capability, CapabilityInfo},
    discovery::{MielBroadcaster, MielListener},
    manifest::{CognitiveManifest, HardwareTier},
    qos::{QosPolicy, RequestQueue},
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, collections::HashSet, fs, net::SocketAddr, path::PathBuf, sync::Arc, time::Duration};
use sysinfo::System;
use tokio::sync::{RwLock, broadcast};
use tracing::{error, info, warn};
use uuid::Uuid;

use laruche_essaim::{
    AbeilleRegistry, EssaimConfig, Session, ChatEvent,
    abeilles::{enregistrer_abeilles_builtin, enregistrer_delegation, charger_plugins},
    brain::{boucle_react, boucle_react_multimodal},
    cron::{CronScheduler, ScheduledTask},
};

use std::collections::VecDeque;

const SPA_HTML: &str = include_str!("../../laruche-dashboard/src/templates/spa.html");
const PEER_FETCH_TIMEOUT_MS: u64 = 4000;
const PEER_STALE_SECS: i64 = 45;
const MDNS_REANNOUNCE_INTERVAL_SECS: u64 = 2;
const ACTIVITY_LOG_LIMIT: usize = 120;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ActivityLogEntry {
    timestamp: String,
    level: String,
    tag: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    full_prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    full_response: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    model_used: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    tokens_generated: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    latency_ms: Option<u64>,
    /// Owner user ID (for filtering: users see only their own logs, admin sees all)
    #[serde(skip_serializing_if = "Option::is_none", default)]
    user_id: Option<Uuid>,
}

/// Persistent state saved to disk (survives restarts)
#[derive(Debug, Serialize, Deserialize, Default)]
struct PersistentState {
    /// Legacy single default model (kept for backward-compatible deserialization)
    #[serde(default)]
    default_model: Option<String>,
    /// Per-capability default models (new format)
    #[serde(default)]
    default_models: Option<HashMap<String, String>>,
    #[serde(default)]
    activity_log: Vec<ActivityLogEntry>,
    #[serde(default)]
    saved_at: String,
    /// BLAKE3 cookie secret (base64), shared across cluster
    #[serde(default)]
    cookie_secret: Option<String>,
}

const METRICS_HISTORY_LIMIT: usize = 360; // ~1 hour at 10s intervals
const NODE_EVENTS_LIMIT: usize = 200;

#[derive(Debug, Clone, Serialize)]
struct MetricsSnapshot {
    epoch_ms: u64,
    cpu_pct: f32,
    ram_pct: f32,
    tokens_per_sec: f32,
    queue_depth: u32,
    node_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    gpu_pct: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    vram_pct: Option<f32>,
}

#[derive(Debug, Clone, Serialize)]
struct NodeEvent {
    epoch_ms: u64,
    event_type: String,
    node_name: String,
}

#[derive(Debug, Serialize)]
struct MetricsHistoryResponse {
    snapshots: Vec<MetricsSnapshot>,
    events: Vec<NodeEvent>,
}

struct AppState {
    manifest: RwLock<CognitiveManifest>,
    auth: RwLock<ProximityAuth>,
    queue: RwLock<RequestQueue>,
    listener: RwLock<MielListener>,
    config: NodeConfig,
    /// Per-capability default models (e.g. "llm" → "mistral", "code" → "qwen3-coder:30b")
    /// The "llm" key is the universal fallback for unspecified capabilities.
    default_models: RwLock<HashMap<String, String>>,
    sys: RwLock<System>,
    activity_log: RwLock<VecDeque<ActivityLogEntry>>,
    /// Path to laruche-state.json for persistence
    state_file_path: PathBuf,
    /// Time-series metrics for charts
    metrics_history: RwLock<VecDeque<MetricsSnapshot>>,
    /// Node connect/disconnect events
    node_events: RwLock<VecDeque<NodeEvent>>,
    /// Track known node IDs for event detection
    known_node_ids: RwLock<HashSet<String>>,
    /// Essaim agent engine
    essaim_registry: Arc<AbeilleRegistry>,
    essaim_config: RwLock<EssaimConfig>,
    essaim_sessions: RwLock<HashMap<Uuid, Session>>,
    essaim_cron: RwLock<CronScheduler>,
    essaim_kb: Arc<tokio::sync::RwLock<laruche_essaim::rag::KnowledgeBase>>,
    /// Active channel bots (keyed by channel name)
    channel_handles: RwLock<HashMap<String, tokio::task::JoinHandle<()>>>,
    /// Provider profiles (multi-provider support)
    profiles: RwLock<profiles::ProfilesConfig>,
    /// Path to provider-profiles.json
    profiles_path: PathBuf,
    /// Registered users
    users: RwLock<HashMap<Uuid, auth_user::User>>,
    /// Pending login challenges (ephemeral, 60s TTL)
    auth_challenges: RwLock<HashMap<Uuid, auth_user::AuthChallenge>>,
    /// BLAKE3 key for signing auth cookies (shared across cluster)
    cookie_secret: [u8; 32],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct NodeConfig {
    node_name: String,
    tier: HardwareTier,
    ollama_url: String,
    default_model: String,
    api_port: u16,
    dashboard_port: u16,
    capabilities: Vec<CapabilityConfig>,
    /// LLM provider: "ollama" (default), "openai", "anthropic"
    #[serde(default)]
    provider: String,
    /// API key for cloud providers
    #[serde(default)]
    api_key: String,
    /// API base URL override
    #[serde(default)]
    api_base: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CapabilityConfig {
    capability: String,
    model_name: String,
    model_size: Option<String>,
    quantization: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct NodeConfigFile {
    node_name: Option<String>,
    tier: Option<HardwareTier>,
    ollama_url: Option<String>,
    default_model: Option<String>,
    api_port: Option<u16>,
    dashboard_port: Option<u16>,
    capabilities: Option<Vec<CapabilityConfig>>,
    provider: Option<String>,
    api_key: Option<String>,
    api_base: Option<String>,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            node_name: {
                let id = Uuid::new_v4().to_string();
                format!("laruche-{}", &id[..6])
            },
            tier: HardwareTier::Core,
            ollama_url: "http://127.0.0.1:11434".into(),
            default_model: "mistral".into(),
            api_port: miel_protocol::DEFAULT_API_PORT,
            dashboard_port: miel_protocol::DEFAULT_DASHBOARD_PORT,
            capabilities: vec![CapabilityConfig {
                capability: "llm".into(),
                model_name: "mistral-7b".into(),
                model_size: Some("7B".into()),
                quantization: Some("Q4_K_M".into()),
            }],
            provider: "ollama".into(),
            api_key: String::new(),
            api_base: None,
        }
    }
}

// ======================== API Types ========================

#[derive(Debug, Deserialize)]
struct InferenceRequest {
    prompt: String,
    model: Option<String>,
    capability: Option<String>,
    #[allow(dead_code)]
    #[serde(default = "default_qos")]
    qos: String,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
}

fn default_qos() -> String {
    "normal".into()
}

#[derive(Debug, Serialize, Deserialize)]
struct InferenceResponse {
    response: String,
    model: String,
    tokens_generated: u32,
    latency_ms: u64,
    node_name: String,
}

#[derive(Debug, Serialize)]
struct NodeStatus {
    node_id: String,
    node_name: String,
    tier: String,
    protocol_version: String,
    capabilities: Vec<String>,
    tokens_per_sec: f32,
    /// Real memory usage % from sysinfo
    memory_usage_pct: f32,
    /// Real CPU usage % from sysinfo
    cpu_usage_pct: f32,
    memory_used_mb: u64,
    memory_total_mb: u64,
    vram_used_mb: Option<u64>,
    vram_total_mb: Option<u64>,
    gpu_usage_pct: Option<f32>,
    temperature_c: Option<f32>,
    queue_depth: usize,
    uptime_secs: u64,
    swarm: SwarmStatus,
    auth: AuthStatus,
}

#[derive(Debug, Serialize)]
struct SwarmStatus {
    in_swarm: bool,
    peer_count: usize,
}

#[derive(Debug, Serialize)]
struct SwarmResponse {
    swarm_id: String,
    total_nodes: usize,
    collective_tps: f32,
    collective_queue: u32,
    total_vram_mb: u64,
    total_ram_mb: u64,
    estimated_speedup: f32,
    sharding_possible: bool,
    nodes: Vec<DiscoveredNodeInfo>,
}

#[derive(Debug, Serialize)]
struct AuthStatus {
    active_tokens: usize,
    pending_requests: usize,
}

#[derive(Debug, Serialize)]
struct DiscoveredNodesResponse {
    nodes: Vec<DiscoveredNodeInfo>,
}

#[derive(Debug, Serialize)]
struct DiscoveredNodeInfo {
    node_id: Option<String>,
    name: Option<String>,
    host: String,
    port: Option<u16>,
    capabilities: Vec<String>,
    /// Primary model running on this node (from Miel TXT record)
    model: Option<String>,
    tokens_per_sec: Option<f32>,
    queue_depth: Option<u32>,
    memory_used_mb: Option<u64>,
    memory_total_mb: Option<u64>,
    memory_usage_pct: Option<f32>,
    cpu_usage_pct: Option<f32>,
    vram_total_mb: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct AuthRequest {
    device_name: String,
    circle: String,
}

#[derive(Debug, Serialize)]
struct AuthPendingResponse {
    request_id: String,
    message: String,
    expires_in_secs: i64,
}

#[derive(Debug, Serialize, Deserialize)]
struct OllamaModelInfo {
    name: String,
    size_gb: f64,
    digest: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ModelsResponse {
    models: Vec<OllamaModelInfo>,
    default_model: String,
}

#[derive(Debug, Deserialize)]
struct PeerStatusResponse {
    node_name: String,
    capabilities: Vec<String>,
    tokens_per_sec: f32,
    queue_depth: usize,
    memory_used_mb: u64,
    memory_total_mb: u64,
    memory_usage_pct: f32,
    cpu_usage_pct: f32,
    vram_total_mb: Option<u64>,
}

#[derive(Debug, Serialize)]
struct SwarmModelInfo {
    host: String,
    node_name: String,
    node_id: Option<String>,
    name: String,
    size_gb: f64,
    digest: String,
    is_default: bool,
    is_local: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    capability: Option<String>,
}

#[derive(Debug, Serialize)]
struct SwarmModelsResponse {
    total_hosts: usize,
    models: Vec<SwarmModelInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    default_models: Option<HashMap<String, String>>,
}

/// Infer a Miel capability from a model name using heuristics.
/// Falls back to "llm" if no specific pattern is matched.
fn infer_capability_from_model_name(name: &str) -> String {
    let lower = name.to_lowercase();
    if lower.contains("coder") || lower.contains("codestral") || lower.contains("deepseek-coder")
        || lower.contains("starcoder") || lower.contains("code")
    {
        return "code".into();
    }
    if lower.contains("llava") || lower.contains("bakllava") || lower.contains("moondream")
        || lower.contains("minicpm-v") || lower.contains("vision")
    {
        return "vlm".into();
    }
    if lower.contains("whisper") || lower.contains("audio") {
        return "audio".into();
    }
    if lower.contains("nomic-embed") || lower.contains("mxbai-embed")
        || lower.contains("all-minilm") || lower.contains("embed")
    {
        return "embed".into();
    }
    if lower.contains("stable-diffusion") || lower.contains("sdxl") || lower.contains("dall") {
        return "image".into();
    }
    "llm".into()
}

/// Resolve capability for a model: first check CapabilityConfig mappings, then heuristic.
fn resolve_model_capability(model_name: &str, capabilities: &[CapabilityConfig]) -> String {
    // Check if any capability config explicitly maps this model
    for cap in capabilities {
        let cap_model = cap.model_name.to_lowercase();
        let check = model_name.to_lowercase();
        if check == cap_model || check.starts_with(&format!("{}:", cap_model)) || cap_model.starts_with(&check) {
            return normalize_capability_label(&cap.capability);
        }
    }
    infer_capability_from_model_name(model_name)
}

/// Read the "llm" default model from the per-capability map, falling back to config.
async fn get_llm_default(state: &AppState) -> String {
    // First check profiles (new system)
    let profiles = state.profiles.read().await;
    if !profiles.active_model.model.is_empty() {
        return profiles.active_model.model.clone();
    }
    drop(profiles);
    // Fallback to old default_models
    let dm = state.default_models.read().await;
    dm.get("llm")
        .cloned()
        .unwrap_or_else(|| state.config.default_model.clone())
}

/// Resolve a model for a given capability from the per-capability map.
async fn resolve_model_for_capability(state: &AppState, capability: Option<&str>) -> String {
    let cap = normalize_capability_label(capability.unwrap_or("llm"));
    let defaults = state.default_models.read().await;
    defaults
        .get(&cap)
        .or_else(|| defaults.get("llm"))
        .cloned()
        .unwrap_or_else(|| state.config.default_model.clone())
}

fn preview_text(input: &str, max_chars: usize) -> String {
    let flat = input.replace(['\n', '\r'], " ");
    let truncated: String = flat.chars().take(max_chars).collect();
    if flat.chars().count() > max_chars {
        format!("{truncated}...")
    } else {
        truncated
    }
}

fn normalize_capability_label(raw: &str) -> String {
    raw.strip_prefix("capability:")
        .unwrap_or(raw)
        .trim()
        .to_lowercase()
}

fn normalize_capabilities(caps: Vec<String>) -> Vec<String> {
    let mut normalized: Vec<String> = caps
        .into_iter()
        .map(|c| normalize_capability_label(&c))
        .filter(|c| !c.is_empty())
        .collect();
    normalized.sort();
    normalized.dedup();
    normalized
}

fn merge_capabilities(primary: Vec<String>, fallback: Vec<String>) -> Vec<String> {
    let mut merged = normalize_capabilities(primary);
    for cap in normalize_capabilities(fallback) {
        if !merged.contains(&cap) {
            merged.push(cap);
        }
    }
    merged.sort();
    merged.dedup();
    merged
}

fn format_host_for_url(host: &str) -> String {
    if host.contains(':') && !host.starts_with('[') && !host.ends_with(']') {
        format!("[{host}]")
    } else {
        host.to_string()
    }
}

fn endpoint_url(host: &str, port: u16, path: &str) -> String {
    let safe_host = format_host_for_url(host);
    format!("http://{safe_host}:{port}{path}")
}

fn is_stale(last_seen: chrono::DateTime<chrono::Utc>) -> bool {
    (chrono::Utc::now() - last_seen).num_seconds() > PEER_STALE_SECS
}

async fn fetch_peer_status(
    client: &reqwest::Client,
    host: &str,
    port: u16,
) -> Option<PeerStatusResponse> {
    let url = endpoint_url(host, port, "/");
    match client.get(url).send().await {
        Ok(resp) if resp.status().is_success() => resp.json::<PeerStatusResponse>().await.ok(),
        _ => None,
    }
}

async fn fetch_models_from_node(
    client: &reqwest::Client,
    host: &str,
    port: u16,
) -> Option<ModelsResponse> {
    let url = endpoint_url(host, port, "/models");
    match client.get(url).send().await {
        Ok(resp) if resp.status().is_success() => resp.json::<ModelsResponse>().await.ok(),
        _ => None,
    }
}

async fn fetch_local_models(
    ollama_url: &str,
    default_model: &str,
) -> Result<ModelsResponse, StatusCode> {
    let client = reqwest::Client::new();
    let url = format!("{ollama_url}/api/tags");

    match client.get(&url).send().await {
        Ok(resp) => match resp.json::<serde_json::Value>().await {
            Ok(body) => {
                let models: Vec<OllamaModelInfo> = body["models"]
                    .as_array()
                    .unwrap_or(&vec![])
                    .iter()
                    .map(|m| OllamaModelInfo {
                        name: m["name"].as_str().unwrap_or("unknown").to_string(),
                        size_gb: m["size"].as_f64().unwrap_or(0.0) / 1_073_741_824.0,
                        digest: m["digest"]
                            .as_str()
                            .unwrap_or("")
                            .chars()
                            .take(12)
                            .collect(),
                    })
                    .collect();

                Ok(ModelsResponse {
                    models,
                    default_model: default_model.to_string(),
                })
            }
            Err(_) => Err(StatusCode::BAD_GATEWAY),
        },
        Err(_) => Err(StatusCode::SERVICE_UNAVAILABLE),
    }
}

// ======================== Handlers ========================

/// GET / - Node status with real system metrics
async fn get_status(State(state): State<Arc<AppState>>) -> Json<NodeStatus> {
    let manifest = state.manifest.read().await;
    let auth = state.auth.read().await;
    let queue = state.queue.read().await;
    let listener = state.listener.read().await;
    let sys = state.sys.read().await;
    let nodes = listener.get_nodes().await;

    let cpu_pct = sys.global_cpu_usage();
    let used_mem_kb = sys.used_memory();
    let total_mem_kb = sys.total_memory();
    let mem_pct = if total_mem_kb > 0 {
        (used_mem_kb as f32 / total_mem_kb as f32) * 100.0
    } else {
        0.0
    };

    Json(NodeStatus {
        node_id: manifest.node_id.to_string(),
        node_name: manifest.node_name.clone(),
        tier: format!("{:?}", manifest.hardware_tier).to_lowercase(),
        protocol_version: manifest.protocol_version.clone(),
        capabilities: normalize_capabilities(manifest.capabilities.to_flags()),
        tokens_per_sec: manifest.performance.tokens_per_sec,
        memory_usage_pct: mem_pct,
        cpu_usage_pct: cpu_pct,
        memory_used_mb: used_mem_kb / 1024,
        memory_total_mb: total_mem_kb / 1024,
        vram_used_mb: manifest.resources.vram_used_mb,
        vram_total_mb: manifest.resources.vram_total_mb,
        gpu_usage_pct: manifest.resources.accelerator_usage_pct,
        temperature_c: manifest.resources.temperature_c,
        queue_depth: queue.depth(),
        uptime_secs: manifest.uptime_secs,
        swarm: SwarmStatus {
            in_swarm: manifest.swarm_info.in_swarm,
            peer_count: nodes.len(),
        },
        auth: AuthStatus {
            active_tokens: auth.list_tokens().len(),
            pending_requests: auth.list_pending().len(),
        },
    })
}

/// GET /nodes - List discovered nodes on the network (peers only)
async fn get_nodes(State(state): State<Arc<AppState>>) -> Json<DiscoveredNodesResponse> {
    let listener = state.listener.read().await;
    let nodes = listener.get_nodes().await;
    let manifest = state.manifest.read().await;

    let node_list: Vec<DiscoveredNodeInfo> = nodes
        .values()
        .filter(|n| {
            n.manifest.node_id != Some(manifest.node_id)
                && n.manifest.host != manifest.api_endpoint.host
        })
        .map(|n| DiscoveredNodeInfo {
            node_id: n.manifest.node_id.map(|id| id.to_string()),
            name: n.manifest.node_name.clone(),
            host: n.manifest.host.clone(),
            port: n.manifest.port,
            capabilities: normalize_capabilities(
                n.manifest
                    .capabilities
                    .iter()
                    .map(|c| c.to_string())
                    .collect(),
            ),
            model: n.manifest.model.clone(),
            tokens_per_sec: n.manifest.tokens_per_sec,
            queue_depth: n.manifest.queue_depth,
            memory_used_mb: None,
            memory_total_mb: None,
            memory_usage_pct: n.manifest.memory_usage_pct,
            cpu_usage_pct: None,
            vram_total_mb: None,
        })
        .collect();

    Json(DiscoveredNodesResponse { nodes: node_list })
}

/// GET /swarm - Collective intelligence status (all nodes including self)
async fn get_swarm(State(state): State<Arc<AppState>>) -> Json<SwarmResponse> {
    let listener = state.listener.read().await;
    let nodes = listener.get_nodes().await;
    let manifest = state.manifest.read().await;
    let queue = state.queue.read().await;
    let sys = state.sys.read().await;
    let http = reqwest::Client::builder()
        .timeout(Duration::from_millis(PEER_FETCH_TIMEOUT_MS))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    let total_mem_mb = sys.total_memory() / 1024;
    let used_mem_mb = sys.used_memory() / 1024;
    let local_mem_pct = if total_mem_mb > 0 {
        (used_mem_mb as f32 / total_mem_mb as f32) * 100.0
    } else {
        0.0
    };
    let local_cpu_pct = sys.global_cpu_usage();
    let local_model = Some(get_llm_default(&state).await);

    let mut total_tps = manifest.performance.tokens_per_sec;
    let mut total_vram = manifest.resources.vram_total_mb.unwrap_or(0);
    let mut total_ram = total_mem_mb;
    let mut total_queue = queue.depth() as u32;

    let mut node_infos = vec![DiscoveredNodeInfo {
        node_id: Some(manifest.node_id.to_string()),
        name: Some(manifest.node_name.clone()),
        host: manifest.api_endpoint.host.clone(),
        port: Some(manifest.api_endpoint.port),
        capabilities: normalize_capabilities(manifest.capabilities.to_flags()),
        model: local_model,
        tokens_per_sec: Some(manifest.performance.tokens_per_sec),
        queue_depth: Some(queue.depth() as u32),
        memory_used_mb: Some(used_mem_mb),
        memory_total_mb: Some(total_mem_mb),
        memory_usage_pct: Some(local_mem_pct),
        cpu_usage_pct: Some(local_cpu_pct),
        vram_total_mb: manifest.resources.vram_total_mb,
    }];

    for node in nodes.values() {
        if node.manifest.node_id == Some(manifest.node_id)
            || node.manifest.host == manifest.api_endpoint.host
        {
            continue;
        }
        if is_stale(node.last_seen) {
            continue;
        }

        let peer_port = node
            .manifest
            .port
            .unwrap_or(miel_protocol::DEFAULT_API_PORT);

        if let Some(peer_status) = fetch_peer_status(&http, &node.manifest.host, peer_port).await {
            total_tps += peer_status.tokens_per_sec;
            total_queue += peer_status.queue_depth as u32;
            total_ram += peer_status.memory_total_mb;
            total_vram += peer_status.vram_total_mb.unwrap_or(0);

            node_infos.push(DiscoveredNodeInfo {
                node_id: node.manifest.node_id.map(|id| id.to_string()),
                name: Some(peer_status.node_name),
                host: node.manifest.host.clone(),
                port: Some(peer_port),
                capabilities: merge_capabilities(
                    peer_status.capabilities,
                    node.manifest
                        .capabilities
                        .iter()
                        .map(|c| c.to_string())
                        .collect(),
                ),
                model: node.manifest.model.clone(),
                tokens_per_sec: Some(peer_status.tokens_per_sec),
                queue_depth: Some(peer_status.queue_depth as u32),
                memory_used_mb: Some(peer_status.memory_used_mb),
                memory_total_mb: Some(peer_status.memory_total_mb),
                memory_usage_pct: Some(peer_status.memory_usage_pct),
                cpu_usage_pct: Some(peer_status.cpu_usage_pct),
                vram_total_mb: peer_status.vram_total_mb,
            });
        } else {
            // Keep nodes visible in /swarm when discovered via mDNS, even if peer HTTP status
            // is temporarily unreachable.
            if let Some(tps) = node.manifest.tokens_per_sec {
                total_tps += tps;
            }
            if let Some(queue_depth) = node.manifest.queue_depth {
                total_queue += queue_depth;
            }

            node_infos.push(DiscoveredNodeInfo {
                node_id: node.manifest.node_id.map(|id| id.to_string()),
                name: node.manifest.node_name.clone(),
                host: node.manifest.host.clone(),
                port: node.manifest.port,
                capabilities: normalize_capabilities(
                    node.manifest
                        .capabilities
                        .iter()
                        .map(|c| c.to_string())
                        .collect(),
                ),
                model: node.manifest.model.clone(),
                tokens_per_sec: node.manifest.tokens_per_sec,
                queue_depth: node.manifest.queue_depth,
                memory_used_mb: None,
                memory_total_mb: None,
                memory_usage_pct: node.manifest.memory_usage_pct,
                cpu_usage_pct: None,
                vram_total_mb: None,
            });
        }
    }

    // Estimate speedup: ~85% efficiency per additional node
    let n = node_infos.len() as f32;
    let estimated_speedup = if n <= 1.0 { 1.0 } else { 1.0 + (n - 1.0) * 0.85 };
    let sharding_possible = node_infos.len() >= 2 && total_vram > 0;

    Json(SwarmResponse {
        swarm_id: "collective-1".into(),
        total_nodes: node_infos.len(),
        collective_tps: total_tps,
        collective_queue: total_queue,
        total_vram_mb: total_vram,
        total_ram_mb: total_ram,
        estimated_speedup,
        sharding_possible,
        nodes: node_infos,
    })
}

/// POST /infer - Inference endpoint (proxies to Ollama)
async fn post_infer(
    State(state): State<Arc<AppState>>,
    connect_info: Option<ConnectInfo<SocketAddr>>,
    Json(req): Json<InferenceRequest>,
) -> Result<Json<InferenceResponse>, StatusCode> {
    let config = &state.config;
    let model = match req.model {
        Some(m) if !m.trim().is_empty() => m,
        _ => resolve_model_for_capability(&state, req.capability.as_deref()).await,
    };
    let start = std::time::Instant::now();
    let requester_ip = connect_info
        .map(|ConnectInfo(addr)| addr.ip().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    // Swarm load balancing: check if a peer node has lower queue depth
    let _target_url = config.ollama_url.clone();
    let _target_node = config.node_name.clone();

    if std::env::var("ESSAIM_SWARM_LB").unwrap_or_default() == "1" {
        let listener = state.listener.read().await;
        let nodes = listener.get_nodes().await;
        let my_queue = state.queue.read().await.depth();

        for (_id, node) in &nodes {
            if is_stale(node.last_seen) { continue; }
            let caps: Vec<String> = node.manifest.capabilities.iter().map(|c| c.to_string()).collect();
            if !caps.iter().any(|c| c == "llm") { continue; }
            let peer_queue = node.manifest.queue_depth.unwrap_or(u32::MAX);
            if (peer_queue as usize) < my_queue.saturating_sub(2) {
                if let Some(port) = node.manifest.port {
                    // Route to peer — they have lower queue
                    let peer_url = format!("http://{}:{}", node.manifest.host, port);
                    tracing::info!(
                        from = %config.node_name,
                        to = ?node.manifest.node_name,
                        my_queue,
                        peer_queue,
                        "Swarm LB: routing to less busy peer"
                    );
                    // Forward the full request to the peer's /infer endpoint
                    let http = reqwest::Client::builder()
                        .timeout(std::time::Duration::from_secs(120))
                        .build()
                        .unwrap_or_else(|_| reqwest::Client::new());
                    match http.post(format!("{}/infer", peer_url))
                        .json(&serde_json::json!({
                            "prompt": req.prompt,
                            "model": &model,
                            "max_tokens": req.max_tokens,
                            "temperature": req.temperature,
                        }))
                        .send()
                        .await
                    {
                        Ok(resp) if resp.status().is_success() => {
                            if let Ok(body) = resp.json::<InferenceResponse>().await {
                                return Ok(Json(body));
                            }
                        }
                        _ => {} // Fallback to local
                    }
                }
            }
        }
    }

    let client = reqwest::Client::new();
    let ollama_req = serde_json::json!({
        "model": model,
        "prompt": req.prompt,
        "stream": false,
        "options": {
            "num_predict": req.max_tokens.unwrap_or(4096),
            "temperature": req.temperature.unwrap_or(0.7),
        }
    });

    match client
        .post(format!("{}/api/generate", config.ollama_url))
        .json(&ollama_req)
        .send()
        .await
    {
        Ok(resp) => {
            if let Ok(body) = resp.json::<serde_json::Value>().await {
                let response_text = body["response"]
                    .as_str()
                    .unwrap_or("(empty response)")
                    .to_string();

                let eval_count = body["eval_count"].as_u64().unwrap_or(0) as u32;
                let latency = start.elapsed().as_millis() as u64;

                if let Ok(mut manifest) = state.manifest.try_write() {
                    let eval_duration =
                        body["eval_duration"].as_f64().unwrap_or(1.0) / 1_000_000_000.0;
                    if eval_duration > 0.0 {
                        manifest.performance.tokens_per_sec =
                            eval_count as f32 / eval_duration as f32;
                    }
                    manifest.performance.avg_latency_ms = latency as f32;
                }

                // Log activity with requester IP and a short response preview.
                let prompt_preview = preview_text(&req.prompt, 60);
                let response_preview = preview_text(&response_text, 100);
                let log_msg = format!(
                    "Inference {} <- {} | {} tokens in {}ms | prompt: \"{}\" | response: \"{}\"",
                    model, requester_ip, eval_count, latency, prompt_preview, response_preview
                );

                let mut activity = state.activity_log.write().await;
                if activity.len() >= ACTIVITY_LOG_LIMIT {
                    activity.pop_front();
                }
                activity.push_back(ActivityLogEntry {
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    level: "log-ok".into(),
                    tag: "INFER".into(),
                    message: log_msg,
                    full_prompt: Some(req.prompt.clone()),
                    full_response: Some(response_text.clone()),
                    model_used: Some(model.clone()),
                    tokens_generated: Some(eval_count),
                    latency_ms: Some(latency),
                    user_id: None,
                });

                Ok(Json(InferenceResponse {
                    response: response_text,
                    model,
                    tokens_generated: eval_count,
                    latency_ms: latency,
                    node_name: config.node_name.clone(),
                }))
            } else {
                Err(StatusCode::BAD_GATEWAY)
            }
        }
        Err(e) => {
            error!("Ollama request failed: {e}");
            Err(StatusCode::BAD_GATEWAY)
        }
    }
}

/// GET /models - List available Ollama models on this node
async fn get_models(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ModelsResponse>, StatusCode> {
    let dm = get_llm_default(&state).await;
    fetch_local_models(&state.config.ollama_url, &dm)
        .await
        .map(Json)
}

/// GET /swarm/models - Aggregate models across local node and discovered peers
async fn get_swarm_models(
    State(state): State<Arc<AppState>>,
) -> Result<Json<SwarmModelsResponse>, StatusCode> {
    let listener = state.listener.read().await;
    let nodes = listener.get_nodes().await;
    let manifest = state.manifest.read().await;

    let http = reqwest::Client::builder()
        .timeout(Duration::from_millis(PEER_FETCH_TIMEOUT_MS))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    let mut models: Vec<SwarmModelInfo> = Vec::new();
    let mut hosts = HashSet::new();

    let dm = get_llm_default(&state).await;
    let local_models =
        fetch_local_models(&state.config.ollama_url, &dm).await?;
    hosts.insert(manifest.api_endpoint.host.clone());
    for m in local_models.models {
        let is_default =
            m.name == local_models.default_model || m.name.starts_with(&local_models.default_model);
        let cap = resolve_model_capability(&m.name, &state.config.capabilities);
        models.push(SwarmModelInfo {
            host: manifest.api_endpoint.host.clone(),
            node_name: manifest.node_name.clone(),
            node_id: Some(manifest.node_id.to_string()),
            name: m.name,
            size_gb: m.size_gb,
            digest: m.digest,
            is_default,
            is_local: true,
            capability: Some(cap),
        });
    }

    for node in nodes.values() {
        if node.manifest.node_id == Some(manifest.node_id)
            || node.manifest.host == manifest.api_endpoint.host
            || is_stale(node.last_seen)
        {
            continue;
        }

        let peer_port = node
            .manifest
            .port
            .unwrap_or(miel_protocol::DEFAULT_API_PORT);
        let Some(peer_models) = fetch_models_from_node(&http, &node.manifest.host, peer_port).await
        else {
            continue;
        };

        hosts.insert(node.manifest.host.clone());
        for m in peer_models.models {
            let is_default = m.name == peer_models.default_model
                || m.name.starts_with(&peer_models.default_model);
            let peer_cap = infer_capability_from_model_name(&m.name);
            models.push(SwarmModelInfo {
                host: node.manifest.host.clone(),
                node_name: node
                    .manifest
                    .node_name
                    .clone()
                    .unwrap_or_else(|| node.manifest.host.clone()),
                node_id: node.manifest.node_id.map(|id| id.to_string()),
                name: m.name,
                size_gb: m.size_gb,
                digest: m.digest,
                is_default,
                is_local: false,
                capability: Some(peer_cap),
            });
        }
    }

    // Add cloud provider models from profiles (non-Ollama)
    {
        let profiles = state.profiles.read().await;
        let active = &profiles.active_model;
        for (pid, profile) in &profiles.profiles {
            if profile.provider == "ollama" {
                continue; // already listed above
            }
            for model_name in &profile.models {
                let is_def = pid == &active.profile_id && model_name == &active.model;
                let cap = resolve_model_capability(model_name, &state.config.capabilities);
                models.push(SwarmModelInfo {
                    host: profile.provider.clone(),
                    node_name: profile.name.clone(),
                    node_id: None,
                    name: model_name.clone(),
                    size_gb: 0.0,
                    digest: String::new(),
                    is_default: is_def,
                    is_local: false,
                    capability: Some(cap),
                });
            }
        }
    }

    // Add Miel service nodes (STT, TTS, Agent) that are not Ollama-based
    for node in nodes.values() {
        if is_stale(node.last_seen) {
            continue;
        }
        for cap_str in &node.manifest.capabilities {
            let cap = cap_str.to_string();
            // Skip capabilities already covered by Ollama models
            if matches!(cap.as_str(), "llm" | "code" | "vlm" | "embed" | "image") {
                continue;
            }
            let _port = node.manifest.port.unwrap_or(0);
            let model_name = node.manifest.model.clone().unwrap_or_else(|| format!("{}-service", cap));
            let node_name = node.manifest.node_name.clone().unwrap_or_else(|| node.manifest.host.clone());

            // Avoid duplicates
            let already_listed = models.iter().any(|m| {
                m.capability.as_deref() == Some(&cap) && m.host == node.manifest.host
            });
            if already_listed {
                continue;
            }

            hosts.insert(node.manifest.host.clone());
            models.push(SwarmModelInfo {
                host: node.manifest.host.clone(),
                node_name,
                node_id: node.manifest.node_id.map(|id| id.to_string()),
                name: model_name,
                size_gb: 0.0,
                digest: String::new(),
                is_default: true,
                is_local: false,
                capability: Some(cap),
            });
        }
    }

    models.sort_by(|a, b| {
        a.capability
            .cmp(&b.capability)
            .then_with(|| a.name.cmp(&b.name))
            .then_with(|| a.host.cmp(&b.host))
            .then_with(|| a.node_name.cmp(&b.node_name))
    });

    // Read per-capability default models directly from runtime state
    let default_models = state.default_models.read().await.clone();

    Ok(Json(SwarmModelsResponse {
        total_hosts: hosts.len(),
        models,
        default_models: Some(default_models),
    }))
}

/// POST /auth/request - Request device authorization
async fn post_auth_request(
    State(state): State<Arc<AppState>>,
    Json(req): Json<AuthRequest>,
) -> Json<AuthPendingResponse> {
    let circle = match req.circle.as_str() {
        "family" => miel_protocol::auth::TrustCircle::Family,
        "office" => miel_protocol::auth::TrustCircle::Office,
        _ => miel_protocol::auth::TrustCircle::Guest,
    };

    let mut auth = state.auth.write().await;
    let pending = auth.request_auth(Uuid::new_v4(), req.device_name, circle);
    let expires_in = (pending.expires_at - chrono::Utc::now()).num_seconds();

    Json(AuthPendingResponse {
        request_id: pending.request_id.to_string(),
        message: "En attente d'approbation physique. Appuyez sur le bouton du boîtier LaRuche."
            .into(),
        expires_in_secs: expires_in,
    })
}

/// POST /auth/approve - Simulate physical button press (for POC)
async fn post_auth_approve(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut auth = state.auth.write().await;
    match auth.approve_pending() {
        Some(token) => Ok(Json(serde_json::json!({
            "status": "approved",
            "token_id": token.token_id.to_string(),
            "device_name": token.device_name,
            "circle": format!("{:?}", token.circle).to_lowercase(),
            "expires_at": token.expires_at,
        }))),
        None => Err(StatusCode::NOT_FOUND),
    }
}

#[derive(Debug, Deserialize)]
struct SetDefaultModelRequest {
    model: String,
    #[serde(default)]
    capability: Option<String>,
}

/// POST /config/default_model - Change the runtime default model
async fn post_set_default_model(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SetDefaultModelRequest>,
) -> Json<serde_json::Value> {
    let model_name = req.model.trim().to_string();
    if model_name.is_empty() {
        return Json(serde_json::json!({ "status": "error", "message": "model name cannot be empty" }));
    }

    let capability = normalize_capability_label(
        req.capability.as_deref().unwrap_or("llm"),
    );

    let prev = {
        let mut dm = state.default_models.write().await;
        let prev = dm.get(&capability).cloned().unwrap_or_default();
        dm.insert(capability.clone(), model_name.clone());
        prev
    };

    // Log the change
    let cap_label = if capability == "llm" { "".into() } else { format!(" ({capability})") };
    let mut activity = state.activity_log.write().await;
    if activity.len() >= ACTIVITY_LOG_LIMIT {
        activity.pop_front();
    }
    activity.push_back(ActivityLogEntry {
        timestamp: chrono::Utc::now().to_rfc3339(),
        level: "log-ok".into(),
        tag: "MODEL".into(),
        message: format!("Default{cap_label} model changed: {} → {}", prev, model_name),
        full_prompt: None,
        full_response: None,
        model_used: None,
        tokens_generated: None,
        latency_ms: None,
        user_id: None,
    });

    info!(capability = %capability, prev = %prev, new = %model_name, "Default model changed via API");

    // Also sync to essaim_config so the inference engine uses the new model
    if capability == "llm" {
        let mut ec = state.essaim_config.write().await;
        ec.model = model_name.clone();
    }

    // Persist state immediately after model change
    let save_ref = state.clone();
    tokio::spawn(async move { save_persistent_state(&save_ref).await });

    Json(serde_json::json!({
        "status": "ok",
        "capability": capability,
        "default_model": model_name,
        "previous": prev,
    }))
}

/// GET /config/default_model - Get the current runtime default model(s)
async fn get_default_model(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let dm = state.default_models.read().await;
    let llm_default = dm.get("llm").cloned().unwrap_or_else(|| state.config.default_model.clone());
    Json(serde_json::json!({
        "default_model": llm_default,
        "default_models": *dm,
    }))
}

#[derive(Debug, Serialize)]
struct ActivityResponse {
    logs: Vec<ActivityLogEntry>,
}

/// GET /activity - Recent activity (filtered by user; admin sees all)
async fn get_activity(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> Json<ActivityResponse> {
    let caller = auth_user::extract_user_from_headers(&headers, &state.cookie_secret);
    let is_admin = if let Some(uid) = caller {
        state.users.read().await.get(&uid).map(|u| u.role == auth_user::UserRole::Admin).unwrap_or(false)
    } else { false };

    let logs = state.activity_log.read().await;
    let filtered: Vec<ActivityLogEntry> = logs.iter().filter(|entry| {
        if is_admin { return true; }
        // System logs (no user_id) — visible to admin only, hide from regular users
        // User's own logs — visible to that user
        match (&entry.user_id, &caller) {
            (None, _) => entry.tag != "agent", // show system logs (heartbeat, model) but not other users' agent chats
            (Some(log_uid), Some(caller_uid)) => log_uid == caller_uid,
            (Some(_), None) => false, // not authenticated
        }
    }).cloned().collect();
    Json(ActivityResponse { logs: filtered })
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok" }))
}

/// GET /api/voice/status — check STT/TTS service availability.
async fn api_voice_status(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let listener = state.listener.read().await;
    let nodes = listener.get_nodes().await;

    let mut stt_available = false;
    let mut tts_available = false;
    let mut stt_url = String::new();
    let mut tts_url = String::new();

    for (_id, node) in &nodes {
        let caps: Vec<String> = node.manifest.capabilities.iter().map(|c| c.to_string()).collect();
        if caps.iter().any(|c| c == "stt") {
            stt_available = true;
            if let Some(port) = node.manifest.port {
                stt_url = format!("http://{}:{}", node.manifest.host, port);
            }
        }
        if caps.iter().any(|c| c == "tts") {
            tts_available = true;
            if let Some(port) = node.manifest.port {
                tts_url = format!("http://{}:{}", node.manifest.host, port);
            }
        }
    }

    Json(serde_json::json!({
        "stt": { "available": stt_available, "url": stt_url },
        "tts": { "available": tts_available, "url": tts_url },
    }))
}

async fn spa_page() -> Html<&'static str> {
    Html(SPA_HTML)
}

async fn api_list_tools(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    Json(state.essaim_registry.schema_complet())
}

/// List all sessions with metadata.
async fn api_list_sessions(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> Json<serde_json::Value> {
    let caller = auth_user::extract_user_from_headers(&headers, &state.cookie_secret);
    let sessions = state.essaim_sessions.read().await;
    let list: Vec<serde_json::Value> = sessions
        .values()
        .filter(|s| {
            // Show: user's own sessions + legacy sessions (no owner)
            s.user_id.is_none() || s.user_id == caller
        })
        .map(|s| {
            serde_json::json!({
                "id": s.id.to_string(),
                "title": s.title,
                "model": s.model,
                "messages": s.len(),
                "estimated_tokens": s.estimated_tokens(),
                "created_at": s.created_at.to_rfc3339(),
                "updated_at": s.updated_at.to_rfc3339(),
            })
        })
        .collect();
    Json(serde_json::json!(list))
}

/// Delete a session by ID (with ownership check).
async fn api_delete_session(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> StatusCode {
    let caller = auth_user::extract_user_from_headers(&headers, &state.cookie_secret);
    if let Ok(uuid) = Uuid::parse_str(&id) {
        let mut sessions = state.essaim_sessions.write().await;
        // Check ownership before deleting
        if let Some(session) = sessions.get(&uuid) {
            if session.user_id.is_some() && session.user_id != caller {
                warn!(session_id = %uuid, "Unauthorized session delete attempt");
                return StatusCode::FORBIDDEN;
            }
        }
        if sessions.remove(&uuid).is_some() {
            let path = std::path::PathBuf::from("sessions").join(format!("{}.json", uuid));
            let _ = std::fs::remove_file(path);
            info!(session_id = %uuid, "Session deleted");
            return StatusCode::OK;
        }
    }
    StatusCode::NOT_FOUND
}

/// GET /api/sessions/:id/messages — get session messages (with ownership check).
async fn api_get_session_messages(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let caller = auth_user::extract_user_from_headers(&headers, &state.cookie_secret);
    let uuid = Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let sessions = state.essaim_sessions.read().await;
    match sessions.get(&uuid) {
        Some(session) if session.user_id.is_some() && session.user_id != caller => {
            Err(StatusCode::FORBIDDEN)
        }
        Some(session) => {
            let messages: Vec<serde_json::Value> = session.messages.iter().map(|m| {
                match m {
                    laruche_essaim::Message::User(text) => serde_json::json!({"role": "user", "text": text}),
                    laruche_essaim::Message::UserMultimodal { text, images } => {
                        serde_json::json!({"role": "user", "text": text, "images": images.len()})
                    },
                    laruche_essaim::Message::Assistant(text) => {
                        // Strip tool_call blocks for display
                        let mut clean = text.clone();
                        while let Some(start) = clean.find("<tool_call>") {
                            if let Some(end) = clean.find("</tool_call>") {
                                clean = format!("{}{}", &clean[..start], &clean[end + "</tool_call>".len()..]);
                            } else {
                                clean.truncate(start);
                                break;
                            }
                        }
                        serde_json::json!({"role": "assistant", "text": clean.trim()})
                    },
                    laruche_essaim::Message::Observation { tool, result } => {
                        serde_json::json!({"role": "tool", "tool": tool, "text": result})
                    },
                    laruche_essaim::Message::System(text) => {
                        serde_json::json!({"role": "system", "text": text})
                    },
                    laruche_essaim::Message::ToolCall { name, args } => {
                        serde_json::json!({"role": "tool_call", "tool": name, "args": args})
                    },
                }
            }).collect();
            Ok(Json(serde_json::json!({
                "session_id": id,
                "title": session.title,
                "messages": messages,
            })))
        }
        None => {
            // Fallback: try loading from disk
            drop(sessions);
            let path = std::path::Path::new("sessions").join(format!("{}.json", id));
            if let Ok(session) = Session::charger(&path) {
                let messages: Vec<serde_json::Value> = session.messages.iter().map(|m| {
                    match m {
                        laruche_essaim::Message::User(t) => serde_json::json!({"role":"user","text":t}),
                        laruche_essaim::Message::UserMultimodal { text, .. } => serde_json::json!({"role":"user","text":text}),
                        laruche_essaim::Message::Assistant(t) => serde_json::json!({"role":"assistant","text":t}),
                        laruche_essaim::Message::Observation { tool, result } => serde_json::json!({"role":"tool","tool":tool,"text":result}),
                        laruche_essaim::Message::System(t) => serde_json::json!({"role":"system","text":t}),
                        laruche_essaim::Message::ToolCall { name, args } => serde_json::json!({"role":"tool_call","tool":name,"args":args}),
                    }
                }).collect();
                state.essaim_sessions.write().await.insert(uuid, session);
                Ok(Json(serde_json::json!({"session_id":id,"messages":messages})))
            } else {
                Err(StatusCode::NOT_FOUND)
            }
        }
    }
}

/// GET /api/sessions/search?q=query — search across all sessions.
async fn api_search_sessions(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let caller = auth_user::extract_user_from_headers(&headers, &state.cookie_secret);
    let query = params.get("q").map(|s| s.to_lowercase()).unwrap_or_default();
    if query.is_empty() {
        return Json(serde_json::json!([]));
    }

    let sessions = state.essaim_sessions.read().await;
    let mut results = Vec::new();

    for session in sessions.values() {
        // Only search user's own sessions + legacy
        if session.user_id.is_some() && session.user_id != caller { continue; }
        for msg in &session.messages {
            let text = match msg {
                laruche_essaim::Message::User(t) | laruche_essaim::Message::Assistant(t) => t.clone(),
                laruche_essaim::Message::UserMultimodal { text, .. } => text.clone(),
                _ => continue,
            };
            if text.to_lowercase().contains(&query) {
                let preview: String = text.chars().take(150).collect();
                results.push(serde_json::json!({
                    "session_id": session.id.to_string(),
                    "session_title": session.title,
                    "role": match msg {
                        laruche_essaim::Message::User(_) | laruche_essaim::Message::UserMultimodal { .. } => "user",
                        _ => "assistant",
                    },
                    "preview": preview,
                }));
                if results.len() >= 20 { break; }
            }
        }
        if results.len() >= 20 { break; }
    }

    Json(serde_json::json!(results))
}

/// GET /api/sessions/:id/export — export a session as Markdown.
// TODO: Add PDF export support (e.g. via printpdf or headless Chrome).
//       For now, only Markdown export is implemented.
async fn api_export_session(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<String, StatusCode> {
    let caller = auth_user::extract_user_from_headers(&headers, &state.cookie_secret);
    let uuid = Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let sessions = state.essaim_sessions.read().await;
    let session = sessions.get(&uuid).ok_or(StatusCode::NOT_FOUND)?;
    if session.user_id.is_some() && session.user_id != caller {
        return Err(StatusCode::FORBIDDEN);
    }

    let mut md = format!(
        "# {}\n\n*Session: {} | Model: {} | Date: {}*\n\n---\n\n",
        session.title.as_deref().unwrap_or("Conversation"),
        session.id,
        session.model,
        session.created_at.format("%Y-%m-%d %H:%M"),
    );

    for msg in &session.messages {
        match msg {
            laruche_essaim::Message::User(text) => {
                md.push_str(&format!("## User\n\n{}\n\n", text));
            }
            laruche_essaim::Message::UserMultimodal { text, images } => {
                md.push_str(&format!("## User\n\n{}\n\n*({} image(s) attached)*\n\n", text, images.len()));
            }
            laruche_essaim::Message::Assistant(text) => {
                // Strip tool_call tags
                let mut clean = text.clone();
                while let Some(s) = clean.find("<tool_call>") {
                    if let Some(e) = clean.find("</tool_call>") {
                        clean = format!("{}{}", &clean[..s], &clean[e + "</tool_call>".len()..]);
                    } else { clean.truncate(s); break; }
                }
                // Strip plan tags
                while let Some(s) = clean.find("<plan>") {
                    if let Some(e) = clean.find("</plan>") {
                        clean = format!("{}{}", &clean[..s], &clean[e + "</plan>".len()..]);
                    } else { clean.truncate(s); break; }
                }
                let clean = clean.trim();
                if !clean.is_empty() {
                    md.push_str(&format!("## Assistant\n\n{}\n\n", clean));
                }
            }
            laruche_essaim::Message::Observation { tool, result } => {
                md.push_str(&format!("> **Tool: {}**\n> ```\n> {}\n> ```\n\n",
                    tool, &result[..result.len().min(500)]));
            }
            _ => {}
        }
    }

    Ok(md)
}

/// POST /api/sessions/:id/fork — fork (branch) a session (with ownership check).
async fn api_fork_session(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let caller = auth_user::extract_user_from_headers(&headers, &state.cookie_secret);
    let uuid = Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let sessions_dir = std::path::Path::new("sessions");
    let current_model = state.essaim_config.read().await.model.clone();

    let mut sessions = state.essaim_sessions.write().await;
    let original = sessions.get(&uuid).ok_or(StatusCode::NOT_FOUND)?;
    if original.user_id.is_some() && original.user_id != caller {
        return Err(StatusCode::FORBIDDEN);
    }
    let mut forked = original.fork(&current_model, sessions_dir);
    // Inherit user_id from parent
    forked.user_id = caller;
    let forked_id = forked.id;

    if let Err(e) = forked.sauvegarder() {
        tracing::warn!(error = %e, "Failed to save forked session");
    }

    sessions.insert(forked_id, forked);

    Ok(Json(serde_json::json!({
        "id": forked_id.to_string(),
        "message": "Session forked successfully",
    })))
}

/// GET /api/cron — list scheduled tasks.
async fn api_list_cron(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let cron = state.essaim_cron.read().await;
    let tasks: Vec<serde_json::Value> = cron.list().iter().map(|t| {
        serde_json::json!({
            "id": t.id.to_string(),
            "name": t.name,
            "prompt": t.prompt,
            "cron_expr": t.cron_expr,
            "fire_at": t.fire_at,
            "enabled": t.enabled,
            "last_run": t.last_run,
            "run_count": t.run_count,
        })
    }).collect();
    Json(serde_json::json!(tasks))
}

/// POST /api/cron — create a scheduled task.
/// Body: {"name": "...", "prompt": "...", "cron_expr": "*/5 * * * *"} or {"fire_at": "ISO8601"}
async fn api_create_cron(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Admin only — cron tasks execute agent prompts
    let users = state.users.read().await;
    let (_, is_admin) = auth_user::check_admin(&headers, &state.cookie_secret, &users);
    drop(users);
    if !is_admin { return Err(StatusCode::FORBIDDEN); }
    let name = body["name"].as_str().unwrap_or("Unnamed task").to_string();
    let prompt = body["prompt"].as_str().ok_or(StatusCode::BAD_REQUEST)?.to_string();
    let cron_expr = body["cron_expr"].as_str().map(|s| s.to_string());
    let fire_at = body["fire_at"].as_str().and_then(|s| {
        chrono::DateTime::parse_from_rfc3339(s).ok().map(|dt| dt.with_timezone(&chrono::Utc))
    });

    let task = ScheduledTask {
        id: Uuid::new_v4(),
        name,
        prompt,
        cron_expr,
        fire_at,
        enabled: true,
        created_at: chrono::Utc::now(),
        last_run: None,
        run_count: 0,
    };

    let id = {
        let mut cron = state.essaim_cron.write().await;
        cron.add(task)
    };

    Ok(Json(serde_json::json!({"id": id.to_string(), "status": "created"})))
}

/// DELETE /api/cron/:id — remove a scheduled task.
async fn api_delete_cron(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> StatusCode {
    if let Ok(uuid) = Uuid::parse_str(&id) {
        let mut cron = state.essaim_cron.write().await;
        if cron.remove(&uuid) {
            return StatusCode::OK;
        }
    }
    StatusCode::NOT_FOUND
}

/// GET /api/doctor — system health check and configuration validation.
async fn api_doctor(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let mut checks = Vec::new();

    // Check Ollama connectivity
    let ec = state.essaim_config.read().await;
    let ollama_ok = reqwest::Client::new()
        .get(format!("{}/api/tags", ec.ollama_url))
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false);
    checks.push(serde_json::json!({
        "name": "Ollama",
        "status": if ollama_ok { "ok" } else { "error" },
        "detail": if ollama_ok { format!("Connected to {}", ec.ollama_url) }
                  else { format!("Cannot reach {}", ec.ollama_url) },
    }));

    // Check model availability
    checks.push(serde_json::json!({
        "name": "Model",
        "status": "ok",
        "detail": format!("Default model: {}", ec.model),
    }));
    let _ = ec;

    // Check Miel network
    let listener = state.listener.read().await;
    let nodes = listener.get_nodes().await;
    checks.push(serde_json::json!({
        "name": "Miel Network",
        "status": "ok",
        "detail": format!("{} peer(s) discovered", nodes.len()),
    }));

    // Check STT/TTS
    let mut stt_found = false;
    let mut tts_found = false;
    for (_id, node) in &nodes {
        let caps: Vec<String> = node.manifest.capabilities.iter().map(|c| c.to_string()).collect();
        if caps.iter().any(|c| c == "stt") { stt_found = true; }
        if caps.iter().any(|c| c == "tts") { tts_found = true; }
    }
    checks.push(serde_json::json!({
        "name": "STT Service",
        "status": if stt_found { "ok" } else { "warning" },
        "detail": if stt_found { "Available" } else { "Not found — voice input disabled" },
    }));
    checks.push(serde_json::json!({
        "name": "TTS Service",
        "status": if tts_found { "ok" } else { "warning" },
        "detail": if tts_found { "Available" } else { "Not found — voice output disabled" },
    }));

    // Check sessions directory
    let sessions_ok = std::path::Path::new("sessions").exists();
    checks.push(serde_json::json!({
        "name": "Sessions Storage",
        "status": if sessions_ok { "ok" } else { "warning" },
        "detail": if sessions_ok { "sessions/ directory exists" } else { "Will be created on first chat" },
    }));

    // Check plugins directory
    let plugins_dir = std::path::Path::new("plugins");
    let plugin_count = if plugins_dir.exists() {
        std::fs::read_dir(plugins_dir)
            .map(|entries| entries.filter(|e| e.as_ref().map(|e| e.path().extension().map_or(false, |ext| ext == "json")).unwrap_or(false)).count())
            .unwrap_or(0)
    } else { 0 };
    checks.push(serde_json::json!({
        "name": "Plugins",
        "status": "ok",
        "detail": format!("{} plugin(s) loaded", plugin_count),
    }));

    // Check Chrome for browser tools
    let chrome_found = if cfg!(windows) {
        std::path::Path::new(r"C:\Program Files\Google\Chrome\Application\chrome.exe").exists()
        || std::path::Path::new(r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe").exists()
    } else {
        which::which("google-chrome").is_ok() || which::which("chromium-browser").is_ok()
    };
    checks.push(serde_json::json!({
        "name": "Browser (Chrome/Edge)",
        "status": if chrome_found { "ok" } else { "warning" },
        "detail": if chrome_found { "Available for browser_navigate/screenshot" } else { "Not found — browser tools disabled" },
    }));

    // Check TLS configuration
    let tls_configured = std::env::var("LARUCHE_TLS_CERT").is_ok()
        && std::env::var("LARUCHE_TLS_KEY").is_ok();
    checks.push(serde_json::json!({
        "name": "TLS/HTTPS",
        "status": if tls_configured { "ok" } else { "warning" },
        "detail": if tls_configured { "TLS enabled" } else { "Not configured — using plain HTTP" },
    }));

    // Abeilles count
    checks.push(serde_json::json!({
        "name": "Abeilles (Tools)",
        "status": "ok",
        "detail": format!("{} tools registered", state.essaim_registry.noms().len()),
    }));

    let all_ok = checks.iter().all(|c| c["status"].as_str() != Some("error"));

    Json(serde_json::json!({
        "status": if all_ok { "healthy" } else { "unhealthy" },
        "checks": checks,
        "version": "0.2.0",
        "protocol": "Miel",
    }))
}

/// GET /api/onboarding — guided setup checklist.
/// GET /api/config/channels — read channel configuration.
async fn api_get_channels_config() -> Json<serde_json::Value> {
    let path = std::path::Path::new("channels-config.json");
    if path.exists() {
        if let Ok(content) = std::fs::read_to_string(path) {
            if let Ok(config) = serde_json::from_str::<serde_json::Value>(&content) {
                return Json(config);
            }
        }
    }
    Json(serde_json::json!({
        "telegram": {"bot_token": "", "allowed_chats": "", "enabled": false},
        "discord": {"bot_token": "", "allowed_channels": "", "enabled": false},
        "slack": {"bot_token": "", "app_token": "", "enabled": false},
    }))
}

/// POST /api/config/channels — save channel configuration.
async fn api_save_channels_config(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> StatusCode {
    let users = state.users.read().await;
    let (_, is_admin) = auth_user::check_admin(&headers, &state.cookie_secret, &users);
    drop(users);
    if !is_admin { return StatusCode::FORBIDDEN; }
    let path = std::path::Path::new("channels-config.json");
    match serde_json::to_string_pretty(&body) {
        Ok(json) => {
            if std::fs::write(path, json).is_ok() {
                StatusCode::OK
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
        Err(_) => StatusCode::BAD_REQUEST,
    }
}

/// GET /api/config/provider — get current LLM provider settings.
async fn api_get_provider_config(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let ec = state.essaim_config.read().await;
    Json(serde_json::json!({
        "provider": ec.provider,
        "api_key_set": !ec.api_key.is_empty(),
        "api_base": ec.api_base,
        "model": ec.model,
        "ollama_url": ec.ollama_url,
    }))
}

/// POST /api/config/provider — update LLM provider settings at runtime.
async fn api_save_provider_config(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let mut cg = state.essaim_config.write().await;
    if let Some(provider) = body["provider"].as_str() {
        let p = provider.to_lowercase();
        if matches!(p.as_str(), "ollama" | "openai" | "anthropic") {
            cg.provider = p;
        }
    }
    if let Some(key) = body["api_key"].as_str() {
        cg.api_key = key.to_string();
    }
    if body.get("api_base").is_some() {
        cg.api_base = body["api_base"]
            .as_str()
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty());
    }
    if let Some(model) = body["model"].as_str() {
        if !model.is_empty() {
            cg.model = model.to_string();
        }
    }
    if let Some(url) = body["ollama_url"].as_str() {
        if !url.is_empty() {
            cg.ollama_url = url.to_string();
        }
    }
    let result = serde_json::json!({
        "status": "ok",
        "provider": cg.provider,
        "model": cg.model,
    });
    drop(cg);
    Json(result)
}

// ======================== Provider Profiles API ========================

/// GET /api/profiles — list all profiles.
async fn api_get_profiles(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Require auth to access profiles (contain API keys)
    auth_user::extract_user_from_headers(&headers, &state.cookie_secret)
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let cfg = state.profiles.read().await;
    // Mask API keys: show only last 4 chars
    let mut profiles_map = serde_json::to_value(&cfg.profiles).unwrap_or_default();
    if let Some(obj) = profiles_map.as_object_mut() {
        for (_id, profile) in obj.iter_mut() {
            if let Some(key) = profile.get("api_key").and_then(|k| k.as_str()) {
                if key.len() > 4 {
                    let masked = format!("{}...{}", &key[..4], &key[key.len()-4..]);
                    profile["api_key"] = serde_json::json!(masked);
                }
            }
        }
    }
    Ok(Json(serde_json::json!({
        "profiles": profiles_map,
        "active_model": cfg.active_model,
    })))
}

/// POST /api/profiles — create or update a profile (auth required).
async fn api_upsert_profile(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    auth_user::extract_user_from_headers(&headers, &state.cookie_secret)
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let id = match body["id"].as_str() {
        Some(id) if !id.is_empty() => id.to_string(),
        _ => return Ok(Json(serde_json::json!({"error": "missing id"}))),
    };
    let provider = body["provider"].as_str().unwrap_or("ollama").to_string();
    let name = body["name"].as_str().unwrap_or(&id).to_string();
    let base_url = body["base_url"].as_str().unwrap_or("").to_string();
    let api_key = body["api_key"].as_str().unwrap_or("").to_string();
    let models: Vec<String> = body["models"]
        .as_array()
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
        .unwrap_or_default();

    let profile = profiles::ProviderProfile {
        provider,
        name: name.clone(),
        base_url,
        api_key,
        models,
    };

    let mut cfg = state.profiles.write().await;
    cfg.profiles.insert(id.clone(), profile);

    // Auto-discover Ollama models if provider is ollama
    if cfg.profiles.get(&id).map(|p| p.provider.as_str()) == Some("ollama") {
        let base = cfg.profiles[&id].base_url.clone();
        drop(cfg);
        let models = profiles::discover_ollama_models(&base).await;
        let mut cfg = state.profiles.write().await;
        if !models.is_empty() {
            if let Some(p) = cfg.profiles.get_mut(&id) {
                p.models = models;
            }
        }
        let _ = profiles::save_profiles(&state.profiles_path, &cfg);
        drop(cfg);
    } else {
        let _ = profiles::save_profiles(&state.profiles_path, &cfg);
        drop(cfg);
    }

    // Sync essaim config from active profile
    sync_essaim_from_profiles(&state).await;

    Ok(Json(serde_json::json!({"status": "ok", "id": id, "name": name})))
}

/// DELETE /api/profiles/:id — delete a profile (auth required).
async fn api_delete_profile(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    auth_user::extract_user_from_headers(&headers, &state.cookie_secret)
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let mut cfg = state.profiles.write().await;
    if cfg.profiles.remove(&id).is_some() {
        // If we deleted the active profile, fall back to first available
        if cfg.active_model.profile_id == id {
            if let Some(first_id) = cfg.profiles.keys().next().cloned() {
                let first_model = cfg.profiles[&first_id]
                    .models
                    .first()
                    .cloned()
                    .unwrap_or_default();
                cfg.active_model = profiles::ActiveModel {
                    profile_id: first_id,
                    model: first_model,
                };
            }
        }
        let _ = profiles::save_profiles(&state.profiles_path, &cfg);
        drop(cfg);
        sync_essaim_from_profiles(&state).await;
        Ok(Json(serde_json::json!({"status": "ok"})))
    } else {
        Ok(Json(serde_json::json!({"error": "profile not found"})))
    }
}

/// GET /api/profiles/models — unified model list across all profiles.
async fn api_get_unified_models(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    // Refresh Ollama models before returning
    let mut cfg = state.profiles.write().await;
    profiles::refresh_ollama_profiles(&mut cfg).await;
    let _ = profiles::save_profiles(&state.profiles_path, &cfg);
    let models = profiles::build_unified_models(&cfg);
    let active = cfg.active_model.clone();
    drop(cfg);
    Json(serde_json::json!({
        "models": models,
        "active": active,
    }))
}

/// POST /api/profiles/active — set the active model.
async fn api_set_active_model(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let profile_id = match body["profile_id"].as_str() {
        Some(id) => id.to_string(),
        None => return Json(serde_json::json!({"error": "missing profile_id"})),
    };
    let model = match body["model"].as_str() {
        Some(m) => m.to_string(),
        None => return Json(serde_json::json!({"error": "missing model"})),
    };

    let mut cfg = state.profiles.write().await;
    if !cfg.profiles.contains_key(&profile_id) {
        return Json(serde_json::json!({"error": "profile not found"}));
    }
    cfg.active_model = profiles::ActiveModel {
        profile_id: profile_id.clone(),
        model: model.clone(),
    };
    let _ = profiles::save_profiles(&state.profiles_path, &cfg);
    drop(cfg);

    // Sync to essaim config
    sync_essaim_from_profiles(&state).await;

    Json(serde_json::json!({"status": "ok", "profile_id": profile_id, "model": model}))
}

/// Sync the active profile into EssaimConfig so brain.rs picks it up.
async fn sync_essaim_from_profiles(state: &Arc<AppState>) {
    let cfg = state.profiles.read().await;
    let (provider, model, api_key, api_base, ollama_url) =
        profiles::active_to_essaim_fields(&cfg);
    drop(cfg);

    let mut ec = state.essaim_config.write().await;
    ec.provider = provider;
    ec.model = model;
    ec.api_key = api_key;
    ec.api_base = api_base;
    ec.ollama_url = ollama_url;
}

// ======================== Auth Endpoints ========================

/// POST /api/auth/enroll — Create a new user identity.
async fn api_auth_enroll(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Result<(axum::http::StatusCode, axum::http::HeaderMap, Json<serde_json::Value>), StatusCode> {
    let display_name = body["display_name"]
        .as_str()
        .unwrap_or("Utilisateur")
        .trim();
    if display_name.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    // First user ever registered becomes admin, others are regular users
    let role = {
        let users = state.users.read().await;
        if users.is_empty() {
            auth_user::UserRole::Admin
        } else {
            auth_user::UserRole::User
        }
    };
    let password = body["password"].as_str().filter(|p| !p.is_empty());
    let user = auth_user::create_user(display_name, role, password);
    let users_dir = std::path::Path::new("users");
    if let Err(e) = auth_user::save_user(&user, users_dir) {
        warn!(error = %e, "Failed to save user");
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    // Build permanent auth link QR
    let manifest = state.manifest.read().await;
    let host = manifest.api_endpoint.host.clone();
    let port = manifest.api_endpoint.port;
    drop(manifest);

    let auth_url = auth_user::build_auth_link(&host, port, user.id, &user.auth_secret);
    let qr_svg = auth_user::generate_qr_svg(&auth_url);

    // Set auth cookie
    let cookie_value = auth_user::create_auth_cookie(user.id, &state.cookie_secret);
    let mut headers = axum::http::HeaderMap::new();
    headers.insert(
        axum::http::header::SET_COOKIE,
        format!(
            "laruche_auth={}; HttpOnly; Path=/; SameSite=Lax; Max-Age=2592000",
            cookie_value
        )
        .parse()
        .unwrap(),
    );

    // Store user in memory
    state.users.write().await.insert(user.id, user.clone());
    // Sync to peers
    let sync_state = state.clone();
    let sync_user = user.clone();
    tokio::spawn(async move { sync::push_user_to_peers(&sync_user, &sync_state).await; });

    info!(user_id = %user.id, name = %user.display_name, "New user enrolled");

    Ok((
        axum::http::StatusCode::OK,
        headers,
        Json(serde_json::json!({
            "user_id": user.id.to_string(),
            "display_name": user.display_name,
            "role": user.role,
            "qr_svg": qr_svg,
            "auth_url": auth_url,
        })),
    ))
}

/// GET /api/auth/me — Return current user info (from cookie).
async fn api_auth_me(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let user_id = auth_user::extract_user_from_headers(&headers, &state.cookie_secret)
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let users = state.users.read().await;
    let user = users.get(&user_id).ok_or(StatusCode::UNAUTHORIZED)?;
    Ok(Json(serde_json::json!({
        "user_id": user.id.to_string(),
        "display_name": user.display_name,
        "role": user.role,
        "created_at": user.created_at.to_rfc3339(),
    })))
}

/// GET /api/auth/challenge — Generate ephemeral login QR.
async fn api_auth_challenge(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    // Cleanup expired challenges
    {
        let mut challenges = state.auth_challenges.write().await;
        challenges.retain(|_, c| !c.is_expired());
    }

    let challenge = auth_user::AuthChallenge::new();
    let challenge_id = challenge.challenge_id;

    let manifest = state.manifest.read().await;
    let host = manifest.api_endpoint.host.clone();
    let port = manifest.api_endpoint.port;
    drop(manifest);
    let scan_url = auth_user::build_challenge_url(&host, port, challenge_id);

    let qr_svg = auth_user::generate_qr_svg(&scan_url);

    state.auth_challenges.write().await.insert(challenge_id, challenge);

    Json(serde_json::json!({
        "challenge_id": challenge_id.to_string(),
        "qr_svg": qr_svg,
        "expires_in": 60,
    }))
}

/// GET /api/auth/status/:id — Poll challenge status.
async fn api_auth_status(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let challenge_id = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => return Json(serde_json::json!({"status": "invalid"})),
    };

    let challenges = state.auth_challenges.read().await;
    match challenges.get(&challenge_id) {
        Some(c) if c.is_expired() => {
            Json(serde_json::json!({"status": "expired"}))
        }
        Some(c) if c.resolved_user_id.is_some() => {
            let user_id = c.resolved_user_id.unwrap();
            let users = state.users.read().await;
            let display_name = users
                .get(&user_id)
                .map(|u| u.display_name.clone())
                .unwrap_or_default();
            let token = auth_user::create_auth_cookie(user_id, &state.cookie_secret);
            Json(serde_json::json!({
                "status": "authenticated",
                "token": token,
                "user_id": user_id.to_string(),
                "display_name": display_name,
            }))
        }
        Some(_) => {
            Json(serde_json::json!({"status": "pending"}))
        }
        None => {
            Json(serde_json::json!({"status": "not_found"}))
        }
    }
}

/// GET /auth/scan/:challenge_id — Phone scans this to resolve challenge.
async fn auth_scan_challenge(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(challenge_id_str): axum::extract::Path<String>,
    headers: axum::http::HeaderMap,
) -> axum::response::Html<String> {
    let challenge_id = match Uuid::parse_str(&challenge_id_str) {
        Ok(u) => u,
        Err(_) => return axum::response::Html("<h1>Invalid challenge</h1>".into()),
    };

    // Extract user from phone's cookie
    let user_id = match auth_user::extract_user_from_headers(&headers, &state.cookie_secret) {
        Some(uid) => uid,
        None => {
            return axum::response::Html(format!(
                r#"<!DOCTYPE html>
<html><head><meta name="viewport" content="width=device-width,initial-scale=1">
<style>body{{background:#1a1a2e;color:#e0e0e0;font-family:system-ui;display:flex;align-items:center;justify-content:center;min-height:100vh;margin:0}}
.card{{background:#16213e;padding:2rem;border-radius:16px;text-align:center;max-width:320px}}
h2{{color:#ffbf00}}</style></head>
<body><div class="card">
<h2>Non authentifie</h2>
<p>Ouvrez d'abord votre lien d'enrollment sur ce telephone.</p>
</div></body></html>"#
            ));
        }
    };

    // Resolve the challenge
    let mut challenges = state.auth_challenges.write().await;
    if let Some(challenge) = challenges.get_mut(&challenge_id) {
        if challenge.is_expired() {
            return axum::response::Html(format!(
                r#"<!DOCTYPE html>
<html><head><meta name="viewport" content="width=device-width,initial-scale=1">
<style>body{{background:#1a1a2e;color:#e0e0e0;font-family:system-ui;display:flex;align-items:center;justify-content:center;min-height:100vh;margin:0}}
.card{{background:#16213e;padding:2rem;border-radius:16px;text-align:center;max-width:320px}}
h2{{color:#ef4444}}</style></head>
<body><div class="card">
<h2>QR expire</h2>
<p>Retournez sur le navigateur et rafraichissez le QR code.</p>
</div></body></html>"#
            ));
        }
        challenge.resolved_user_id = Some(user_id);
    }
    drop(challenges);

    let users = state.users.read().await;
    let display_name = users
        .get(&user_id)
        .map(|u| u.display_name.clone())
        .unwrap_or_else(|| "Utilisateur".into());

    info!(user_id = %user_id, name = %display_name, "Login challenge resolved via QR scan");

    axum::response::Html(format!(
        r#"<!DOCTYPE html>
<html><head><meta name="viewport" content="width=device-width,initial-scale=1">
<style>body{{background:#1a1a2e;color:#e0e0e0;font-family:system-ui;display:flex;align-items:center;justify-content:center;min-height:100vh;margin:0}}
.card{{background:#16213e;padding:2rem;border-radius:16px;text-align:center;max-width:320px}}
h2{{color:#22c55e}}.check{{font-size:3rem;margin-bottom:1rem}}</style></head>
<body><div class="card">
<div class="check">&#x2714;</div>
<h2>Connecte !</h2>
<p>Bienvenue <strong>{}</strong>.<br>Vous pouvez fermer cet onglet.</p>
</div></body></html>"#,
        display_name
    ))
}

/// GET /auth/link/:user_id/:secret — Permanent auth link (from enrollment QR).
async fn auth_permanent_link(
    State(state): State<Arc<AppState>>,
    axum::extract::Path((user_id_str, secret)): axum::extract::Path<(String, String)>,
) -> Result<(axum::http::StatusCode, axum::http::HeaderMap, axum::response::Html<String>), StatusCode> {
    let user_id = Uuid::parse_str(&user_id_str).map_err(|_| StatusCode::BAD_REQUEST)?;
    let users = state.users.read().await;
    let user = users.get(&user_id).ok_or(StatusCode::NOT_FOUND)?;

    if user.auth_secret != secret {
        return Err(StatusCode::FORBIDDEN);
    }

    let display_name = user.display_name.clone();
    drop(users);

    // Set auth cookie on this device (phone)
    let cookie_value = auth_user::create_auth_cookie(user_id, &state.cookie_secret);
    let mut headers = axum::http::HeaderMap::new();
    headers.insert(
        axum::http::header::SET_COOKIE,
        format!(
            "laruche_auth={}; HttpOnly; Path=/; SameSite=Lax; Max-Age=2592000",
            cookie_value
        )
        .parse()
        .unwrap(),
    );

    // Also check if there's a pending challenge to resolve
    // (phone scans enrollment QR which also resolves any open challenge)
    {
        let mut challenges = state.auth_challenges.write().await;
        for (_, challenge) in challenges.iter_mut() {
            if !challenge.is_expired() && challenge.resolved_user_id.is_none() {
                challenge.resolved_user_id = Some(user_id);
                break; // resolve the first pending one
            }
        }
    }

    info!(user_id = %user_id, name = %display_name, "Auth via permanent link");

    Ok((
        axum::http::StatusCode::OK,
        headers,
        axum::response::Html(format!(
            r#"<!DOCTYPE html>
<html><head><meta name="viewport" content="width=device-width,initial-scale=1">
<style>body{{background:#1a1a2e;color:#e0e0e0;font-family:system-ui;display:flex;align-items:center;justify-content:center;min-height:100vh;margin:0}}
.card{{background:#16213e;padding:2rem;border-radius:16px;text-align:center;max-width:320px}}
h2{{color:#ffbf00}}.bee{{font-size:3rem;margin-bottom:1rem}}</style></head>
<body><div class="card">
<div class="bee">&#x1F41D;</div>
<h2>Identite confirmee</h2>
<p>Bienvenue <strong>{}</strong>.<br>Ce telephone est maintenant votre cle d'acces LaRuche.</p>
</div></body></html>"#,
            display_name
        )),
    ))
}

/// POST /api/auth/logout — Clear auth cookie.
async fn api_auth_logout() -> (axum::http::StatusCode, axum::http::HeaderMap) {
    let mut headers = axum::http::HeaderMap::new();
    headers.insert(
        axum::http::header::SET_COOKIE,
        "laruche_auth=; HttpOnly; Path=/; SameSite=Lax; Max-Age=0"
            .parse()
            .unwrap(),
    );
    (axum::http::StatusCode::OK, headers)
}

/// POST /api/auth/login — Login with display_name + password.
async fn api_auth_login(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Result<(axum::http::StatusCode, axum::http::HeaderMap, Json<serde_json::Value>), StatusCode> {
    let name = body["display_name"].as_str().unwrap_or("").trim();
    let password = body["password"].as_str().unwrap_or("");
    if name.is_empty() || password.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let users = state.users.read().await;
    let user = auth_user::find_user_by_name(&users, name).ok_or(StatusCode::UNAUTHORIZED)?;

    match &user.password_hash {
        Some(hash) if auth_user::verify_password(password, hash) => {
            let cookie_value = auth_user::create_auth_cookie(user.id, &state.cookie_secret);
            let mut headers = axum::http::HeaderMap::new();
            headers.insert(
                axum::http::header::SET_COOKIE,
                format!("laruche_auth={}; HttpOnly; Path=/; SameSite=Lax; Max-Age=2592000", cookie_value)
                    .parse().unwrap(),
            );
            info!(user_id = %user.id, name = %user.display_name, "Login via password");
            Ok((
                axum::http::StatusCode::OK,
                headers,
                Json(serde_json::json!({
                    "user_id": user.id.to_string(),
                    "display_name": user.display_name,
                    "role": user.role,
                })),
            ))
        }
        _ => {
            warn!(name = %name, "Failed login attempt");
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}

/// POST /api/auth/password — Set or change password (requires auth).
async fn api_auth_set_password(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let user_id = auth_user::extract_user_from_headers(&headers, &state.cookie_secret)
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let password = body["password"].as_str().unwrap_or("");
    if password.len() < 4 {
        return Ok(Json(serde_json::json!({"error": "Password must be at least 4 characters"})));
    }

    let mut users = state.users.write().await;
    if let Some(user) = users.get_mut(&user_id) {
        user.password_hash = Some(auth_user::hash_password(password));
        let users_dir = std::path::Path::new("users");
        let _ = auth_user::save_user(user, users_dir);
        info!(user_id = %user_id, "Password set/changed");
        Ok(Json(serde_json::json!({"status": "ok"})))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// POST /api/auth/model — Set per-user preferred model (doesn't touch global config).
async fn api_auth_set_model(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let user_id = auth_user::extract_user_from_headers(&headers, &state.cookie_secret)
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let model = body["model"].as_str().unwrap_or("").to_string();
    let provider = body["provider"].as_str().map(|s| s.to_string());

    let mut users = state.users.write().await;
    if let Some(user) = users.get_mut(&user_id) {
        user.preferred_model = if model.is_empty() { None } else { Some(model.clone()) };
        user.preferred_provider = provider;
        let users_dir = std::path::Path::new("users");
        let _ = auth_user::save_user(user, users_dir);
        Ok(Json(serde_json::json!({"status": "ok", "model": model})))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

// ======================== Knowledge Endpoints ========================

/// GET /api/knowledge — list knowledge base entries.
async fn api_list_knowledge(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> Json<serde_json::Value> {
    let caller = auth_user::extract_user_from_headers(&headers, &state.cookie_secret);
    let is_admin = if let Some(uid) = caller {
        state.users.read().await.get(&uid).map(|u| u.role == auth_user::UserRole::Admin).unwrap_or(false)
    } else { false };
    let kb = state.essaim_kb.read().await;
    let entries: Vec<serde_json::Value> = kb.entries.iter()
        .filter(|e| {
            // Admin sees all, users see global + own
            is_admin || e.user_id.is_none() || e.user_id == caller
        })
        .map(|e| {
        serde_json::json!({
            "id": e.id,
            "text": e.text,
            "source": e.source,
            "created_at": e.created_at,
            "user_id": e.user_id,
        })
    }).collect();
    Json(serde_json::json!({
        "count": entries.len(),
        "entries": entries,
    }))
}

/// POST /api/knowledge — add a knowledge entry.
async fn api_add_knowledge(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let caller = auth_user::extract_user_from_headers(&headers, &state.cookie_secret);
    let text = body["text"].as_str().ok_or(StatusCode::BAD_REQUEST)?;
    let source = body["source"].as_str();
    // Admin entries are global (user_id=None), user entries are private
    let is_admin = if let Some(uid) = caller {
        state.users.read().await.get(&uid).map(|u| u.role == auth_user::UserRole::Admin).unwrap_or(false)
    } else { false };
    let entry_user_id = if is_admin { None } else { caller };

    let mut kb = state.essaim_kb.write().await;
    match kb.add_with_user(text, source, entry_user_id).await {
        Ok(id) => Ok(Json(serde_json::json!({"id": id, "status": "added"}))),
        Err(e) => Ok(Json(serde_json::json!({"error": e.to_string()}))),
    }
}

/// PUT /api/knowledge/:id — update a knowledge entry.
async fn api_update_knowledge(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let text = body["text"].as_str().unwrap_or("");
    let source = body["source"].as_str();
    if text.is_empty() {
        return Json(serde_json::json!({"error": "text is required"}));
    }
    let mut kb = state.essaim_kb.write().await;
    match kb.update(&id, text, source).await {
        Ok(true) => Json(serde_json::json!({"status": "updated", "id": id})),
        Ok(false) => Json(serde_json::json!({"error": "Entry not found"})),
        Err(e) => Json(serde_json::json!({"error": e.to_string()})),
    }
}

/// DELETE /api/knowledge/:id — remove a knowledge entry.
async fn api_delete_knowledge(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> StatusCode {
    let mut kb = state.essaim_kb.write().await;
    if kb.remove(&id) { StatusCode::OK } else { StatusCode::NOT_FOUND }
}

/// POST /api/channels/start — start a channel bot.
/// Body: {"channel": "telegram"}
async fn api_start_channel(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let channel = body["channel"].as_str().unwrap_or("");

    // Check if already running
    {
        let handles = state.channel_handles.read().await;
        if handles.contains_key(channel) {
            return Json(serde_json::json!({"status": "already_running", "channel": channel}));
        }
    }

    // Load config
    let config_path = std::path::Path::new("channels-config.json");
    let config: serde_json::Value = if config_path.exists() {
        std::fs::read_to_string(config_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        return Json(serde_json::json!({"status": "error", "message": "No channels-config.json found. Configure in Settings > Channels."}));
    };

    match channel {
        "telegram" => {
            let token = config["telegram"]["bot_token"].as_str().unwrap_or("").to_string();
            let allowed = config["telegram"]["allowed_chats"].as_str().unwrap_or("").to_string();
            if token.is_empty() {
                return Json(serde_json::json!({"status": "error", "message": "No Telegram bot token configured"}));
            }
            let state_clone = state.clone();
            let handle = tokio::spawn(async move {
                run_telegram_bot(&token, &allowed, &state_clone).await;
            });
            state.channel_handles.write().await.insert("telegram".into(), handle);
            info!("Telegram bot started");
            Json(serde_json::json!({"status": "started", "channel": "telegram"}))
        }
        _ => Json(serde_json::json!({"status": "error", "message": format!("Unknown channel: {}", channel)})),
    }
}

/// POST /api/channels/stop — stop a channel bot.
async fn api_stop_channel(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let channel = body["channel"].as_str().unwrap_or("");
    let mut handles = state.channel_handles.write().await;
    if let Some(handle) = handles.remove(channel) {
        handle.abort();
        info!(channel = channel, "Channel bot stopped");
        Json(serde_json::json!({"status": "stopped", "channel": channel}))
    } else {
        Json(serde_json::json!({"status": "not_running", "channel": channel}))
    }
}

/// GET /api/channels/status — check which channels are running.
async fn api_channels_status(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let handles = state.channel_handles.read().await;
    let running: Vec<&String> = handles.keys().collect();
    Json(serde_json::json!({"running": running}))
}

/// Telegram bot — runs as a background task within the server.
async fn run_telegram_bot(token: &str, allowed_chats: &str, state: &Arc<AppState>) {
    let api = format!("https://api.telegram.org/bot{}", token);
    let client = reqwest::Client::builder().timeout(std::time::Duration::from_secs(60)).build().unwrap();
    let allowed: Vec<String> = allowed_chats.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();

    let mut offset: i64 = 0;
    let mut processed_ids: std::collections::HashSet<i64> = std::collections::HashSet::new();
    info!("Telegram bot polling started");

    loop {
        let url = format!("{}/getUpdates?offset={}&timeout=30", api, offset);
        match client.get(&url).send().await {
            Ok(resp) => {
                if let Ok(data) = resp.json::<serde_json::Value>().await {
                    if let Some(updates) = data["result"].as_array() {
                        // Advance offset immediately to prevent duplicate processing
                        if let Some(last) = updates.last() {
                            offset = last["update_id"].as_i64().unwrap_or(0) + 1;
                            // Confirm offset with Telegram (quick call, no wait)
                            let _ = client.get(format!("{}/getUpdates?offset={}&timeout=0", api, offset))
                                .send().await;
                        }

                        for update in updates {
                            let update_id = update["update_id"].as_i64().unwrap_or(0);
                            if processed_ids.contains(&update_id) { continue; }
                            processed_ids.insert(update_id);
                            // Keep set small — only remember last 100
                            if processed_ids.len() > 100 {
                                let min = *processed_ids.iter().min().unwrap_or(&0);
                                processed_ids.remove(&min);
                            }

                            let chat_id = update["message"]["chat"]["id"].as_i64().unwrap_or(0);
                            let text = update["message"]["text"].as_str().unwrap_or("");
                            let user = update["message"]["from"]["first_name"].as_str().unwrap_or("?");

                            if text.is_empty() || chat_id == 0 { continue; }

                            // Check allowlist
                            if !allowed.is_empty() && !allowed.contains(&chat_id.to_string()) {
                                let _ = client.post(format!("{}/sendMessage", api))
                                    .json(&serde_json::json!({"chat_id": chat_id, "text": "Access denied."}))
                                    .send().await;
                                continue;
                            }

                            info!(user = user, chat_id = chat_id, text = &text[..text.len().min(50)], "Telegram message");

                            // Get or create LaRuche user for this Telegram chat_id
                            let tg_user_id = {
                                let tg_name = format!("telegram:{}", chat_id);
                                let users = state.users.read().await;
                                if let Some(u) = auth_user::find_user_by_name(&users, &tg_name) {
                                    u.id
                                } else {
                                    drop(users);
                                    let display = format!("{} (Telegram)", user);
                                    let new_user = auth_user::create_user(&display, auth_user::UserRole::User, None);
                                    let uid = new_user.id;
                                    let _ = auth_user::save_user(&new_user, std::path::Path::new("users"));
                                    state.users.write().await.insert(uid, new_user);
                                    info!(chat_id = chat_id, user_id = %uid, "Auto-created Telegram user");
                                    uid
                                }
                            };

                            // Send typing
                            let _ = client.post(format!("{}/sendChatAction", api))
                                .json(&serde_json::json!({"chat_id": chat_id, "action": "typing"}))
                                .send().await;

                            // Query agent with current default model
                            let current_model = get_llm_default(state).await;
                            let sessions_dir = std::path::Path::new("sessions");
                            let session_id = Uuid::new_v4();
                            let mut session = Session::new_with_id(session_id, &current_model, sessions_dir);
                            session.user_id = Some(tg_user_id);
                            let (tx, _rx) = broadcast::channel::<ChatEvent>(64);

                            let mut config = state.essaim_config.read().await.clone();
                            config.model = current_model;

                            let result = boucle_react(
                                text,
                                &mut session,
                                &state.essaim_registry,
                                &config,
                                &tx,
                            ).await;

                            let response = match result {
                                Ok(r) => {
                                    // Clean tags
                                    let mut clean = r;
                                    while let Some(s) = clean.find("<tool_call>") {
                                        if let Some(e) = clean.find("</tool_call>") { clean = format!("{}{}", &clean[..s], &clean[e+"</tool_call>".len()..]); }
                                        else { clean.truncate(s); break; }
                                    }
                                    while let Some(s) = clean.find("<plan>") {
                                        if let Some(e) = clean.find("</plan>") { clean = format!("{}{}", &clean[..s], &clean[e+"</plan>".len()..]); }
                                        else { clean.truncate(s); break; }
                                    }
                                    clean.trim().to_string()
                                }
                                Err(e) => format!("Error: {}", e),
                            };

                            // Send response (split if > 4000 chars)
                            let chunks: Vec<String> = response.chars().collect::<Vec<_>>()
                                .chunks(4000).map(|c| c.iter().collect()).collect();
                            for chunk in chunks {
                                let _ = client.post(format!("{}/sendMessage", api))
                                    .json(&serde_json::json!({
                                        "chat_id": chat_id,
                                        "text": chunk,
                                        "parse_mode": "Markdown",
                                    }))
                                    .send().await;
                            }

                            info!(user = user, response_len = response.len(), "Telegram replied");
                        }
                    }
                }
            }
            Err(e) => {
                warn!(error = %e, "Telegram polling error");
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            }
        }
    }
}

/// Helper: run agent query and return cleaned response text.
async fn run_agent_query(state: &Arc<AppState>, text: &str) -> String {
    let current_model = get_llm_default(state).await;
    let sessions_dir = std::path::Path::new("sessions");
    let session_id = Uuid::new_v4();
    let mut session = Session::new_with_id(session_id, &current_model, sessions_dir);
    let (tx, _rx) = broadcast::channel::<ChatEvent>(64);

    let mut config = state.essaim_config.read().await.clone();
    config.model = current_model;

    let result = boucle_react(
        text,
        &mut session,
        &state.essaim_registry,
        &config,
        &tx,
    ).await;

    match result {
        Ok(r) => {
            let mut clean = r;
            while let Some(s) = clean.find("<tool_call>") {
                if let Some(e) = clean.find("</tool_call>") {
                    clean = format!("{}{}", &clean[..s], &clean[e + "</tool_call>".len()..]);
                } else {
                    clean.truncate(s);
                    break;
                }
            }
            while let Some(s) = clean.find("<plan>") {
                if let Some(e) = clean.find("</plan>") {
                    clean = format!("{}{}", &clean[..s], &clean[e + "</plan>".len()..]);
                } else {
                    clean.truncate(s);
                    break;
                }
            }
            clean.trim().to_string()
        }
        Err(e) => format!("Error: {}", e),
    }
}

// ======================== Discord Webhook ========================

/// POST /api/channels/discord/webhook — receive Discord Interactions (slash commands).
/// Discord sends interactions as POST requests to the configured endpoint URL.
/// Interaction types:
///   1 = PING (verification), 2 = APPLICATION_COMMAND (slash command),
///   3 = MESSAGE_COMPONENT, 4 = APPLICATION_COMMAND_AUTOCOMPLETE
async fn api_discord_webhook(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let interaction_type = body["type"].as_u64().unwrap_or(0);

    match interaction_type {
        // Type 1: PING — Discord verification handshake
        1 => {
            info!("Discord: PING received (verification)");
            Json(serde_json::json!({"type": 1}))
        }
        // Type 2: APPLICATION_COMMAND — slash command
        2 => {
            let command_name = body["data"]["name"].as_str().unwrap_or("");
            let user = body["member"]["user"]["username"]
                .as_str()
                .or_else(|| body["user"]["username"].as_str())
                .unwrap_or("unknown");

            // Extract the user's input from the command options
            let input = body["data"]["options"]
                .as_array()
                .and_then(|opts| {
                    opts.iter()
                        .find(|o| o["name"].as_str() == Some("prompt") || o["name"].as_str() == Some("message"))
                        .and_then(|o| o["value"].as_str())
                })
                .unwrap_or("");

            if input.is_empty() {
                return Json(serde_json::json!({
                    "type": 4,
                    "data": {
                        "content": "Please provide a prompt. Usage: `/ask <your question>`"
                    }
                }));
            }

            info!(user = user, command = command_name, input = &input[..input.len().min(50)], "Discord slash command");

            // Run agent query
            let response = run_agent_query(&state, input).await;

            // Truncate if needed (Discord max: 2000 chars)
            let truncated = if response.len() > 1990 {
                format!("{}...", &response[..1990])
            } else {
                response
            };

            // Type 4 = CHANNEL_MESSAGE_WITH_SOURCE
            Json(serde_json::json!({
                "type": 4,
                "data": {
                    "content": truncated
                }
            }))
        }
        // Unknown interaction type
        _ => {
            warn!(interaction_type = interaction_type, "Discord: unknown interaction type");
            Json(serde_json::json!({"type": 1}))
        }
    }
}

// ======================== Slack Events ========================

/// POST /api/channels/slack/events — receive Slack Events API callbacks.
/// Handles:
///   - `url_verification` challenge (required by Slack during setup)
///   - `event_callback` with `message` and `app_mention` events
async fn api_slack_events(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let event_type = body["type"].as_str().unwrap_or("");

    match event_type {
        // Slack URL verification challenge
        "url_verification" => {
            let challenge = body["challenge"].as_str().unwrap_or("");
            info!("Slack: URL verification challenge");
            Json(serde_json::json!({"challenge": challenge}))
        }
        // Actual event callbacks
        "event_callback" => {
            let event = &body["event"];
            let event_subtype = event["type"].as_str().unwrap_or("");
            let subtype = event["subtype"].as_str();

            // Ignore bot messages to prevent loops
            if event.get("bot_id").is_some() || subtype == Some("bot_message") {
                return Json(serde_json::json!({"ok": true}));
            }

            let text = event["text"].as_str().unwrap_or("");
            let channel = event["channel"].as_str().unwrap_or("");
            let user = event["user"].as_str().unwrap_or("unknown");

            if text.is_empty() || channel.is_empty() {
                return Json(serde_json::json!({"ok": true}));
            }

            match event_subtype {
                "message" | "app_mention" => {
                    info!(user = user, channel = channel, event_type = event_subtype, text = &text[..text.len().min(50)], "Slack event");

                    // Strip bot mention (e.g., "<@U123456> what is Rust?" -> "what is Rust?")
                    let clean_text = if text.starts_with('<') {
                        text.find('>').map(|i| text[i + 1..].trim()).unwrap_or(text)
                    } else {
                        text
                    };

                    if clean_text.is_empty() {
                        return Json(serde_json::json!({"ok": true}));
                    }

                    // Run agent query
                    let response = run_agent_query(&state, clean_text).await;

                    // Post reply via Slack API
                    let config_path = std::path::Path::new("channels-config.json");
                    if let Ok(content) = std::fs::read_to_string(config_path) {
                        if let Ok(config) = serde_json::from_str::<serde_json::Value>(&content) {
                            let bot_token = config["slack"]["bot_token"].as_str().unwrap_or("");
                            if !bot_token.is_empty() {
                                let http = reqwest::Client::new();
                                let _ = http
                                    .post("https://slack.com/api/chat.postMessage")
                                    .header("Authorization", format!("Bearer {}", bot_token))
                                    .json(&serde_json::json!({
                                        "channel": channel,
                                        "text": response,
                                    }))
                                    .send()
                                    .await;
                                info!(channel = channel, response_len = response.len(), "Slack replied");
                            }
                        }
                    }
                }
                _ => {
                    // Ignore other event types
                }
            }

            Json(serde_json::json!({"ok": true}))
        }
        _ => {
            warn!(event_type = event_type, "Slack: unknown event type");
            Json(serde_json::json!({"ok": true}))
        }
    }
}

/// GET /api/cwd — get current working directory.
async fn api_get_cwd() -> Json<serde_json::Value> {
    let cwd = std::env::current_dir().unwrap_or_default().display().to_string();
    Json(serde_json::json!({"cwd": cwd}))
}

/// POST /api/cwd — set current working directory.
async fn api_set_cwd(
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let path = body["cwd"].as_str().unwrap_or("");
    if path.is_empty() {
        return Json(serde_json::json!({"error": "cwd is required"}));
    }
    let p = std::path::Path::new(path);
    if !p.exists() || !p.is_dir() {
        return Json(serde_json::json!({"error": format!("Directory not found: {}", path)}));
    }
    match std::env::set_current_dir(p) {
        Ok(()) => {
            info!(cwd = path, "Working directory changed");
            Json(serde_json::json!({"cwd": path, "status": "ok"}))
        }
        Err(e) => Json(serde_json::json!({"error": e.to_string()})),
    }
}

async fn api_onboarding(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let mut steps = Vec::new();

    // 1. Ollama installed?
    let ec = state.essaim_config.read().await;
    let ollama_ok = reqwest::Client::new()
        .get(format!("{}/api/tags", ec.ollama_url))
        .timeout(std::time::Duration::from_secs(3))
        .send().await
        .map(|r| r.status().is_success()).unwrap_or(false);
    steps.push(serde_json::json!({
        "step": 1, "title": "Ollama",
        "done": ollama_ok,
        "instruction": if ollama_ok { "Ollama est connecte." }
            else { "Installer Ollama: https://ollama.com/download" },
    }));

    // 2. LLM model configured?
    steps.push(serde_json::json!({
        "step": 2, "title": "Modele LLM",
        "done": ollama_ok,
        "instruction": format!("Modele actuel: {}. Pour Gemma 4: ollama pull gemma4:e4b", ec.model),
    }));
    let _ = ec;

    // 3. Embedding model for RAG?
    steps.push(serde_json::json!({
        "step": 3, "title": "Modele Embeddings (RAG)",
        "done": false,
        "instruction": "Pour le RAG: ollama pull nomic-embed-text",
    }));

    // 4. Voice services?
    let listener = state.listener.read().await;
    let nodes = listener.get_nodes().await;
    let has_stt = nodes.values().any(|n| n.manifest.capabilities.iter().any(|c| c.to_string() == "stt"));
    let has_tts = nodes.values().any(|n| n.manifest.capabilities.iter().any(|c| c.to_string() == "tts"));
    steps.push(serde_json::json!({
        "step": 4, "title": "Services vocaux (STT/TTS)",
        "done": has_stt && has_tts,
        "instruction": if has_stt && has_tts { "STT et TTS disponibles." }
            else { "Lancer: cd laruche-voix && python -m src.stt_service && python -m src.tts_service" },
    }));

    // 5. Chrome for browser tools?
    let has_chrome = if cfg!(windows) {
        std::path::Path::new(r"C:\Program Files\Google\Chrome\Application\chrome.exe").exists()
    } else {
        which::which("google-chrome").is_ok()
    };
    steps.push(serde_json::json!({
        "step": 5, "title": "Chrome/Edge (outils navigateur)",
        "done": has_chrome,
        "instruction": if has_chrome { "Chrome detecte." } else { "Installer Chrome pour browser_navigate/screenshot." },
    }));

    let done_count = steps.iter().filter(|s| s["done"].as_bool().unwrap_or(false)).count();

    Json(serde_json::json!({
        "progress": format!("{}/{}", done_count, steps.len()),
        "complete": done_count == steps.len(),
        "steps": steps,
    }))
}

/// GET /api/files/suggest?q=partial_path — autocomplete file paths.
async fn api_files_suggest(
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let query = params.get("q").map(|s| s.as_str()).unwrap_or(".");
    let path = std::path::Path::new(query);

    // Determine the directory to list and the prefix to match
    let (dir, prefix) = if path.is_dir() {
        (path.to_path_buf(), String::new())
    } else {
        let parent = path.parent().unwrap_or(std::path::Path::new("."));
        let prefix = path.file_name()
            .map(|f| f.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        (parent.to_path_buf(), prefix)
    };

    let mut suggestions = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten().take(20) {
            let name = entry.file_name().to_string_lossy().to_string();
            if prefix.is_empty() || name.to_lowercase().starts_with(&prefix) {
                let full_path = entry.path().display().to_string();
                let is_dir = entry.path().is_dir();
                suggestions.push(serde_json::json!({
                    "name": name,
                    "path": full_path,
                    "is_dir": is_dir,
                }));
            }
        }
    }

    Json(serde_json::json!(suggestions))
}

/// POST /api/rpc — Remote Procedure Call between Miel nodes.
/// Body: {"method": "infer|status|tools|ping", "params": {...}}
/// Allows nodes to invoke capabilities on each other.
async fn api_rpc(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let method = body["method"].as_str().unwrap_or("");
    let params = &body["params"];

    match method {
        "ping" => {
            let manifest = state.manifest.read().await;
            Json(serde_json::json!({
                "result": "pong",
                "node": state.config.node_name,
                "uptime_secs": manifest.uptime_secs,
            }))
        }
        "tools" => {
            Json(serde_json::json!({
                "result": state.essaim_registry.noms(),
            }))
        }
        "status" => {
            let manifest = state.manifest.read().await;
            Json(serde_json::json!({
                "result": {
                    "node_name": manifest.node_name,
                    "tier": format!("{:?}", manifest.hardware_tier),
                    "cpu_pct": manifest.resources.cpu_usage_pct,
                    "memory_used_mb": manifest.resources.memory_used_mb,
                    "tokens_per_sec": manifest.performance.tokens_per_sec,
                    "queue_depth": manifest.performance.queue_depth,
                }
            }))
        }
        "execute_tool" => {
            let tool_name = params["name"].as_str().unwrap_or("");
            let tool_args = params["arguments"].clone();
            let ctx = laruche_essaim::ContextExecution::default();
            match state.essaim_registry.executer(tool_name, tool_args, &ctx).await {
                Ok(result) => Json(serde_json::json!({
                    "result": {
                        "success": result.success,
                        "output": result.output,
                        "error": result.error,
                    }
                })),
                Err(e) => Json(serde_json::json!({ "error": e.to_string() })),
            }
        }
        _ => Json(serde_json::json!({
            "error": format!("Unknown RPC method: '{}'. Available: ping, tools, status, execute_tool", method),
        })),
    }
}

/// POST /api/preload — preload a model into Ollama VRAM.
/// Sends a minimal generate request to warm up the model.
async fn api_preload(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let default_model = state.essaim_config.read().await.model.clone();
    let model = body["model"]
        .as_str()
        .unwrap_or(&default_model)
        .to_string();

    info!(model = %model, "Preloading model into Ollama");
    let start = std::time::Instant::now();

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    // Ollama loads the model on first request. Send a minimal prompt.
    let result = client
        .post(format!("{}/api/generate", state.config.ollama_url))
        .json(&serde_json::json!({
            "model": model,
            "prompt": "",
            "stream": false,
            "options": { "num_predict": 1 },
            "keep_alive": "10m",
        }))
        .send()
        .await;

    let elapsed = start.elapsed().as_millis() as u64;

    match result {
        Ok(resp) if resp.status().is_success() => {
            info!(model = %model, elapsed_ms = elapsed, "Model preloaded");
            Json(serde_json::json!({
                "status": "loaded",
                "model": model,
                "elapsed_ms": elapsed,
            }))
        }
        Ok(resp) => {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            warn!(model = %model, status = %status, "Preload failed");
            Json(serde_json::json!({
                "status": "error",
                "error": format!("Ollama {}: {}", status, &body[..body.len().min(200)]),
            }))
        }
        Err(e) => {
            warn!(model = %model, error = %e, "Preload failed");
            Json(serde_json::json!({
                "status": "error",
                "error": e.to_string(),
            }))
        }
    }
}

/// POST /api/webhook — trigger the agent via HTTP (for external integrations).
/// Body: {"prompt": "...", "model": "optional-model-override"}
/// Returns: {"response": "...", "session_id": "..."}
async fn api_webhook(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let prompt = body["prompt"].as_str().ok_or(StatusCode::BAD_REQUEST)?;
    let model_override = body["model"].as_str().map(|s| s.to_string());

    // Use current dynamic default model, not initial config
    let current_model = get_llm_default(&state).await;
    let sessions_dir = std::path::Path::new("sessions");
    let session_id = uuid::Uuid::new_v4();
    let mut session = Session::new_with_id(session_id, &current_model, sessions_dir);

    let mut config = state.essaim_config.read().await.clone();
    config.model = model_override.unwrap_or(current_model);

    let (tx, mut rx) = broadcast::channel::<ChatEvent>(256);

    let result = boucle_react(
        prompt,
        &mut session,
        &state.essaim_registry,
        &config,
        &tx,
    )
    .await;

    // Collect events for the response
    drop(tx);
    let mut tools_used: Vec<serde_json::Value> = Vec::new();
    let mut plan_items: Vec<serde_json::Value> = Vec::new();
    while let Ok(event) = rx.try_recv() {
        match event {
            ChatEvent::ToolCall { name, args, .. } => {
                tools_used.push(serde_json::json!({"name": name, "args": args}));
            }
            ChatEvent::ToolResult { name, success, elapsed_ms, .. } => {
                if let Some(last) = tools_used.last_mut() {
                    if last["name"].as_str() == Some(&name) {
                        last["success"] = serde_json::json!(success);
                        last["elapsed_ms"] = serde_json::json!(elapsed_ms);
                    }
                }
            }
            ChatEvent::Plan { items } => {
                plan_items = items.iter().map(|i| serde_json::json!({"task": i.task, "status": i.status})).collect();
            }
            _ => {}
        }
    }

    // Save session
    session.auto_title();
    let _ = session.sauvegarder();
    // Sync to peers
    let sync_state = state.clone();
    let sync_session = session.clone();
    tokio::spawn(async move { sync::push_session_to_peers(&sync_session, &sync_state).await; });
    state.essaim_sessions.write().await.insert(session_id, session);

    match result {
        Ok(response) => Ok(Json(serde_json::json!({
            "response": response,
            "session_id": session_id.to_string(),
            "tools_used": tools_used,
            "plan": plan_items,
        }))),
        Err(e) => Ok(Json(serde_json::json!({
            "error": e.to_string(),
            "session_id": session_id.to_string(),
        }))),
    }
}

/// WebSocket handler for the chat interface.
/// Protocol:
///   Client → {"type":"message","text":"..."} or {"type":"message","text":"...","session_id":"uuid"}
///   Server → {"type":"token","text":"..."} / {"type":"tool_call",...} / {"type":"done",...} / {"type":"error",...}
async fn ws_chat_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> axum::response::Response {
    let user_id = auth_user::extract_user_from_headers(&headers, &state.cookie_secret);
    ws.on_upgrade(move |socket| ws_chat_connection(socket, state, user_id))
}

async fn ws_chat_connection(socket: ws::WebSocket, state: Arc<AppState>, auth_user_id: Option<Uuid>) {
    use futures_util::{SinkExt, StreamExt};

    let (mut sender, mut receiver) = socket.split();

    while let Some(Ok(msg)) = receiver.next().await {
        let text = match msg {
            ws::Message::Text(t) => t.to_string(),
            ws::Message::Close(_) => break,
            _ => continue,
        };

        // Parse incoming message
        let incoming: serde_json::Value = match serde_json::from_str(&text) {
            Ok(v) => v,
            Err(_) => {
                let _ = sender
                    .send(ws::Message::Text(
                        serde_json::json!({"type":"error","message":"Invalid JSON"}).to_string().into(),
                    ))
                    .await;
                continue;
            }
        };

        let user_text = match incoming["text"].as_str() {
            Some(t) if !t.trim().is_empty() => t.to_string(),
            _ => continue,
        };

        // Get or create session
        let session_id = incoming["session_id"]
            .as_str()
            .and_then(|s| Uuid::parse_str(s).ok());

        let sessions_dir = std::path::Path::new("sessions");
        let current_model_ws = state.essaim_config.read().await.model.clone();
        let mut sessions = state.essaim_sessions.write().await;
        let session_id = session_id.unwrap_or_else(|| {
            let id = Uuid::new_v4();
            let mut s = Session::new_with_id(id, &current_model_ws, sessions_dir);
            s.user_id = auth_user_id;
            sessions.insert(id, s);
            id
        });
        if !sessions.contains_key(&session_id) {
            let mut s = Session::new_with_id(session_id, &current_model_ws, sessions_dir);
            s.user_id = auth_user_id;
            sessions.insert(session_id, s);
        }
        drop(sessions);

        // Send session_id back so the client can persist it
        let _ = sender
            .send(ws::Message::Text(
                serde_json::json!({"type":"session","session_id": session_id.to_string()}).to_string().into(),
            ))
            .await;

        // Model override from client
        let model_override = incoming["model"].as_str().map(|s| s.to_string());

        // Parse images from client message (base64 array)
        let images: Vec<String> = incoming["images"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        // Create broadcast channel for events + approval channel
        let (tx, mut rx) = broadcast::channel::<ChatEvent>(256);
        let (approval_tx, approval_rx) = tokio::sync::mpsc::channel::<laruche_essaim::ApprovalResponse>(4);

        // Extract session, run ReAct, then put it back
        let state_clone = state.clone();
        let ws_user_id = auth_user_id;
        let user_text_clone = user_text.clone();
        let tx_clone = tx.clone();
        let react_handle = tokio::spawn(async move {
            let sessions_dir = std::path::Path::new("sessions");
            let ec_snapshot = state_clone.essaim_config.read().await.clone();
            let mut session = {
                let mut sessions = state_clone.essaim_sessions.write().await;
                sessions.remove(&session_id).unwrap_or_else(|| {
                    Session::new_with_id(session_id, &ec_snapshot.model, sessions_dir)
                })
            };

            let mut config = ec_snapshot;
            if let Some(ref model) = model_override {
                config.model = model.clone();
            }

            let result = boucle_react_multimodal(
                &user_text_clone,
                &mut session,
                &state_clone.essaim_registry,
                &config,
                &tx_clone,
                images,
                Some(approval_rx),
            )
            .await;

            // Log to activity (visible in dashboard)
            {
                let now = chrono::Utc::now().to_rfc3339();
                let mut activity = state_clone.activity_log.write().await;
                if activity.len() >= ACTIVITY_LOG_LIMIT { activity.pop_front(); }
                activity.push_back(ActivityLogEntry {
                    timestamp: now,
                    level: if result.is_ok() { "info" } else { "error" }.into(),
                    tag: "agent".into(),
                    message: format!("Agent chat: {}", preview_text(&user_text_clone, 60)),
                    full_prompt: Some(user_text_clone.clone()),
                    full_response: result.as_ref().ok().map(|r| preview_text(r, 200)),
                    model_used: Some(config.model.clone()),
                    tokens_generated: None,
                    latency_ms: None,
                    user_id: ws_user_id,
                });
            }

            if let Err(e) = &result {
                let _ = tx_clone.send(ChatEvent::Error {
                    message: e.to_string(),
                });
            }

            // Auto-title and save session
            session.auto_title();
            if let Err(e) = session.sauvegarder() {
                tracing::warn!(error = %e, "Failed to save session");
            }
            // Sync to peers
            let sync_s = session.clone();
            let sync_st = state_clone.clone();
            tokio::spawn(async move { sync::push_session_to_peers(&sync_s, &sync_st).await; });

            // Put session back
            let mut sessions = state_clone.essaim_sessions.write().await;
            sessions.insert(session_id, session);
        });

        // Forward events to WebSocket + listen for approvals from client
        let mut done = false;
        while !done {
            tokio::select! {
                // Events from the ReAct loop → send to client
                event_result = rx.recv() => {
                    match event_result {
                        Ok(event) => {
                            let json = serde_json::to_string(&event).unwrap_or_default();
                            if sender.send(ws::Message::Text(json.into())).await.is_err() {
                                done = true;
                            }
                            if matches!(event, ChatEvent::Done { .. } | ChatEvent::Error { .. }) {
                                done = true;
                            }
                        }
                        Err(broadcast::error::RecvError::Closed) => { done = true; }
                        Err(broadcast::error::RecvError::Lagged(_)) => { continue; }
                    }
                }
                // Incoming messages from client (approvals)
                msg_result = receiver.next() => {
                    match msg_result {
                        Some(Ok(ws::Message::Text(text))) => {
                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text.to_string()) {
                                if json["type"].as_str() == Some("approval") {
                                    let resp = laruche_essaim::ApprovalResponse {
                                        tool_call_id: json["tool_call_id"].as_str().unwrap_or("").to_string(),
                                        approved: json["approved"].as_bool().unwrap_or(false),
                                    };
                                    let _ = approval_tx.send(resp).await;
                                }
                            }
                        }
                        Some(Ok(ws::Message::Close(_))) | None => { done = true; }
                        _ => {}
                    }
                }
            }
        }

        let _ = react_handle.await;
    }
}

// ======================== Voice Pipeline ========================

/// WebSocket handler for voice: receives audio, returns audio.
/// Protocol:
///   Client → binary (PCM 16kHz 16-bit mono) or JSON {"type":"config","stt_url":"...","tts_url":"..."}
///   Server → binary (WAV audio) or JSON {"type":"transcript","text":"..."} / {"type":"error",...}
async fn ws_audio_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> axum::response::Response {
    ws.on_upgrade(move |socket| ws_audio_connection(socket, state))
}

async fn ws_audio_connection(socket: ws::WebSocket, state: Arc<AppState>) {
    use futures_util::{SinkExt, StreamExt};

    let (mut sender, mut receiver) = socket.split();

    // Default STT/TTS endpoints — can be overridden by client config message
    let mut stt_url = "http://127.0.0.1:8421".to_string();
    let mut tts_url = "http://127.0.0.1:8422".to_string();

    // Try to discover STT/TTS nodes from Miel listener
    {
        let listener = state.listener.read().await;
        let nodes = listener.get_nodes().await;
        for (_id, node) in &nodes {
            let caps: Vec<String> = node.manifest.capabilities.iter().map(|c| c.to_string()).collect();
            let host = &node.manifest.host;
            if caps.iter().any(|c| c == "stt") {
                if let Some(port) = node.manifest.port {
                    stt_url = format!("http://{}:{}", host, port);
                    info!(stt_url = %stt_url, "Discovered STT node via Miel");
                }
            }
            if caps.iter().any(|c| c == "tts") {
                if let Some(port) = node.manifest.port {
                    tts_url = format!("http://{}:{}", host, port);
                    info!(tts_url = %tts_url, "Discovered TTS node via Miel");
                }
            }
        }
    }

    let _ = sender.send(ws::Message::Text(
        serde_json::json!({"type": "ready", "stt_url": &stt_url, "tts_url": &tts_url}).to_string().into()
    )).await;

    let client = reqwest::Client::new();

    while let Some(Ok(msg)) = receiver.next().await {
        match msg {
            ws::Message::Binary(audio_data) => {
                // Step 1: Send audio to STT service
                let stt_result = client
                    .post(format!("{}/transcribe", stt_url))
                    .multipart(
                        reqwest::multipart::Form::new()
                            .part("file", reqwest::multipart::Part::bytes(audio_data.to_vec())
                                .file_name("audio.wav")
                                .mime_str("audio/wav").unwrap())
                    )
                    .send()
                    .await;

                let transcript = match stt_result {
                    Ok(resp) => {
                        match resp.json::<serde_json::Value>().await {
                            Ok(json) => json["text"].as_str().unwrap_or("").to_string(),
                            Err(e) => {
                                let _ = sender.send(ws::Message::Text(
                                    serde_json::json!({"type":"error","message":format!("STT parse error: {}", e)}).to_string().into()
                                )).await;
                                continue;
                            }
                        }
                    }
                    Err(e) => {
                        let _ = sender.send(ws::Message::Text(
                            serde_json::json!({"type":"error","message":format!("STT unavailable: {}", e)}).to_string().into()
                        )).await;
                        continue;
                    }
                };

                if transcript.is_empty() {
                    continue;
                }

                // Send transcript to client
                let _ = sender.send(ws::Message::Text(
                    serde_json::json!({"type":"transcript","text":&transcript}).to_string().into()
                )).await;

                // Step 2: Run through ReAct agent
                let sessions_dir = std::path::Path::new("sessions");
                let audio_config = state.essaim_config.read().await.clone();
                let mut session = Session::new_with_path(&audio_config.model, sessions_dir);
                let (tx, _rx) = broadcast::channel::<ChatEvent>(64);

                let agent_result = boucle_react(
                    &transcript,
                    &mut session,
                    &state.essaim_registry,
                    &audio_config,
                    &tx,
                ).await;

                let response_text = match agent_result {
                    Ok(text) => text,
                    Err(e) => {
                        let _ = sender.send(ws::Message::Text(
                            serde_json::json!({"type":"error","message":format!("Agent error: {}", e)}).to_string().into()
                        )).await;
                        continue;
                    }
                };

                // Send text response
                let _ = sender.send(ws::Message::Text(
                    serde_json::json!({"type":"response","text":&response_text}).to_string().into()
                )).await;

                // Step 3: Send response to TTS service
                let tts_result = client
                    .post(format!("{}/synthesize", tts_url))
                    .json(&serde_json::json!({"text": &response_text}))
                    .send()
                    .await;

                match tts_result {
                    Ok(resp) if resp.status().is_success() => {
                        if let Ok(audio_bytes) = resp.bytes().await {
                            let _ = sender.send(ws::Message::Binary(audio_bytes.to_vec().into())).await;
                        }
                    }
                    Ok(resp) => {
                        let _ = sender.send(ws::Message::Text(
                            serde_json::json!({"type":"error","message":format!("TTS error: {}", resp.status())}).to_string().into()
                        )).await;
                    }
                    Err(e) => {
                        let _ = sender.send(ws::Message::Text(
                            serde_json::json!({"type":"error","message":format!("TTS unavailable: {}", e)}).to_string().into()
                        )).await;
                    }
                }
            }
            ws::Message::Text(text) => {
                // Config messages
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                    if json["type"].as_str() == Some("config") {
                        if let Some(url) = json["stt_url"].as_str() {
                            stt_url = url.to_string();
                        }
                        if let Some(url) = json["tts_url"].as_str() {
                            tts_url = url.to_string();
                        }
                    }
                }
            }
            ws::Message::Close(_) => break,
            _ => {}
        }
    }
}

// ======================== Main ========================

#[tokio::main]
async fn main() -> Result<()> {
    let use_tui = !std::env::args().any(|a| a == "--no-tui");

    let tui_log_rx = if use_tui {
        // Layered subscriber: TUI captures logs + optional stderr fallback
        use tracing_subscriber::layer::SubscriberExt;
        use tracing_subscriber::util::SubscriberInitExt;
        let (tui_buf, rx) = tui::TuiLogBuffer::new();
        let tui_layer = tui::TuiTracingLayer::new(tui_buf.sender());
        let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "laruche_node=info,miel_protocol=info,laruche_essaim=info".into());
        tracing_subscriber::registry()
            .with(env_filter)
            .with(tui_layer)
            .init();
        Some(rx)
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "laruche_node=info,miel_protocol=info".into()),
            )
            .init();
        None
    };

    let config = load_config()?;

    info!(
        r#"
  ██╗      █████╗ ██████╗ ██╗   ██╗ ██████╗██╗  ██╗███████╗
  ██║     ██╔══██╗██╔══██╗██║   ██║██╔════╝██║  ██║██╔════╝
  ██║     ███████║██████╔╝██║   ██║██║     ███████║█████╗
  ██║     ██╔══██║██╔══██╗██║   ██║██║     ██╔══██║██╔══╝
  ███████╗██║  ██║██║  ██║╚██████╔╝╚██████╗██║  ██║███████╗
  ╚══════╝╚═╝  ╚═╝╚═╝  ╚═╝ ╚═════╝  ╚═════╝╚═╝  ╚═╝╚══════╝
  Branchez l'IA. C'est tout. • Miel Protocol v{}
    "#,
        miel_protocol::PROTOCOL_VERSION
    );

    info!(name = %config.node_name, tier = ?config.tier, "Starting LaRuche node");

    let local_ip = miel_protocol::get_local_ip();
    info!(ip = %local_ip, "Detected local IP");

    let mut manifest = CognitiveManifest::new(config.node_name.clone(), config.tier);
    manifest.api_endpoint.host = local_ip;
    manifest.api_endpoint.port = config.api_port;
    manifest.api_endpoint.dashboard_port = config.dashboard_port;

    for cap_config in &config.capabilities {
        if let Some(cap) = Capability::from_flag(&cap_config.capability) {
            manifest.capabilities.add(CapabilityInfo {
                capability: cap,
                model_name: cap_config.model_name.clone(),
                model_size: cap_config.model_size.clone(),
                quantization: cap_config.quantization.clone(),
                max_context_length: Some(8192),
            });
            info!(capability = %cap, model = %cap_config.model_name, "Registered capability");
        }
    }

    // This node is also an agent (Essaim)
    manifest.capabilities.add(CapabilityInfo {
        capability: Capability::Agent,
        model_name: config.default_model.clone(),
        model_size: None,
        quantization: None,
        max_context_length: Some(8192),
    });
    info!(capability = "agent", "Registered Essaim agent capability");

    let mut broadcaster = MielBroadcaster::new()?;
    broadcaster.register(&manifest)?;
    let broadcaster = Arc::new(broadcaster);

    let mut listener = MielListener::new()?;
    let _discovered_nodes = listener.start()?;

    let mut sys = System::new_all();
    sys.refresh_all();

    // Load persistent state (activity log, default model) from previous session
    let state_file_path = resolve_state_file_path();
    let persistent = load_persistent_state(&state_file_path);

    // Build initial per-capability default models map:
    // 1) Start from config capabilities
    // 2) Overlay with persisted runtime choices from last session
    let mut initial_defaults: HashMap<String, String> = HashMap::new();
    for cap in &config.capabilities {
        let cap_name = normalize_capability_label(&cap.capability);
        initial_defaults.entry(cap_name).or_insert_with(|| cap.model_name.clone());
    }
    // Ensure "llm" is always present
    initial_defaults
        .entry("llm".into())
        .or_insert_with(|| config.default_model.clone());
    // Overlay persisted state (takes priority — user's runtime choices)
    if let Some(persisted_map) = persistent.default_models {
        for (k, v) in persisted_map {
            if !v.is_empty() {
                initial_defaults.insert(k, v);
            }
        }
    } else if let Some(dm) = persistent.default_model.filter(|m| !m.is_empty()) {
        // Legacy migration: single default_model → "llm" entry
        initial_defaults.insert("llm".into(), dm);
    }

    // Pre-populate activity log from persistent state
    let mut initial_log = VecDeque::with_capacity(ACTIVITY_LOG_LIMIT);
    for entry in persistent.activity_log.into_iter().rev().take(ACTIVITY_LOG_LIMIT) {
        initial_log.push_front(entry);
    }

    // Load provider profiles (multi-provider support)
    let profiles_path = PathBuf::from("provider-profiles.json");
    let mut profiles_cfg = profiles::load_profiles(&profiles_path);

    // Migrate old single-provider config into profiles if no profiles exist beyond default
    if profiles_cfg.profiles.len() <= 1 && !config.provider.is_empty() && config.provider != "ollama" {
        let migrated_id = format!("{}-migrated", config.provider);
        profiles_cfg.profiles.insert(
            migrated_id.clone(),
            profiles::ProviderProfile {
                provider: config.provider.clone(),
                name: config.provider.clone(),
                base_url: config.api_base.clone().unwrap_or_else(|| match config.provider.as_str() {
                    "openai" => "https://api.openai.com".to_string(),
                    "anthropic" => "https://api.anthropic.com".to_string(),
                    _ => String::new(),
                }),
                api_key: config.api_key.clone(),
                models: vec![config.default_model.clone()],
            },
        );
        profiles_cfg.active_model = profiles::ActiveModel {
            profile_id: migrated_id,
            model: config.default_model.clone(),
        };
        let _ = profiles::save_profiles(&profiles_path, &profiles_cfg);
        info!("Migrated legacy provider config into profiles");
    }

    // Auto-discover Ollama models for ollama profiles at startup
    profiles::refresh_ollama_profiles(&mut profiles_cfg).await;
    let _ = profiles::save_profiles(&profiles_path, &profiles_cfg);

    // Derive EssaimConfig from active profile
    let (prof_provider, prof_model, prof_api_key, prof_api_base, prof_ollama_url) =
        profiles::active_to_essaim_fields(&profiles_cfg);

    // Initialize Essaim (agent engine)
    let mut essaim_registry = AbeilleRegistry::new();
    enregistrer_abeilles_builtin(&mut essaim_registry);
    let essaim_config = EssaimConfig {
        ollama_url: prof_ollama_url,
        model: prof_model,
        provider: prof_provider,
        api_key: prof_api_key,
        api_base: prof_api_base,
        ..EssaimConfig::default()
    };

    // Create a sub-registry for delegation (contains all tools except delegate itself)
    let sub_registry = Arc::new({
        let mut r = AbeilleRegistry::new();
        enregistrer_abeilles_builtin(&mut r);
        r
    });
    enregistrer_delegation(&mut essaim_registry, sub_registry, essaim_config.clone());

    // Load dynamic plugins from plugins/ directory
    charger_plugins(std::path::Path::new("plugins"), &mut essaim_registry);

    // Initialize RAG knowledge base
    let kb = Arc::new(tokio::sync::RwLock::new(
        laruche_essaim::rag::KnowledgeBase::new(
            std::path::Path::new("knowledge-base.json"),
            &config.ollama_url,
            "nomic-embed-text", // Default embedding model — user should pull it
        ),
    ));
    essaim_registry.enregistrer(Box::new(
        laruche_essaim::abeilles::knowledge::KnowledgeAdd { kb: kb.clone() },
    ));
    essaim_registry.enregistrer(Box::new(
        laruche_essaim::abeilles::knowledge::KnowledgeSearch { kb: kb.clone() },
    ));

    // Load existing sessions from disk
    let mut loaded_sessions: HashMap<Uuid, Session> = HashMap::new();
    let sessions_dir = std::path::Path::new("sessions");
    if sessions_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(sessions_dir) {
            for entry in entries.flatten() {
                if entry.path().extension().map_or(false, |e| e == "json") {
                    match Session::charger(&entry.path()) {
                        Ok(session) => {
                            tracing::debug!(session_id = %session.id, title = ?session.title, "Loaded session");
                            loaded_sessions.insert(session.id, session);
                        }
                        Err(e) => {
                            warn!(path = %entry.path().display(), error = %e, "Failed to load session");
                        }
                    }
                }
            }
        }
    }
    info!(count = loaded_sessions.len(), "Sessions loaded from disk");

    // Load users from disk
    let users_dir = std::path::Path::new("users");
    let loaded_users = auth_user::load_all_users(users_dir);
    if !loaded_users.is_empty() {
        info!(count = loaded_users.len(), "Users loaded from disk");
    }

    // Load or generate cookie secret (persisted in laruche-state.json)
    let cookie_secret = if let Some(ref hex) = persistent.cookie_secret {
        auth_user::cookie_secret_from_base64(hex).unwrap_or_else(|| {
            let s = auth_user::generate_cookie_secret();
            info!("Generated new cookie secret (stored was invalid)");
            s
        })
    } else {
        let s = auth_user::generate_cookie_secret();
        info!("Generated new cookie secret");
        s
    };

    let state = Arc::new(AppState {
        manifest: RwLock::new(manifest),
        auth: RwLock::new(ProximityAuth::new()),
        queue: RwLock::new(RequestQueue::new(QosPolicy::default())),
        listener: RwLock::new(listener),
        default_models: RwLock::new(initial_defaults),
        config: config.clone(),
        sys: RwLock::new(sys),
        activity_log: RwLock::new(initial_log),
        state_file_path,
        metrics_history: RwLock::new(VecDeque::with_capacity(METRICS_HISTORY_LIMIT)),
        node_events: RwLock::new(VecDeque::with_capacity(NODE_EVENTS_LIMIT)),
        known_node_ids: RwLock::new(HashSet::new()),
        essaim_registry: Arc::new(essaim_registry),
        essaim_config: RwLock::new(essaim_config),
        essaim_sessions: RwLock::new(loaded_sessions),
        essaim_cron: RwLock::new(CronScheduler::new(std::path::Path::new("cron-tasks.json"))),
        essaim_kb: kb.clone(),
        channel_handles: RwLock::new(HashMap::new()),
        profiles: RwLock::new(profiles_cfg),
        profiles_path,
        users: RwLock::new(loaded_users),
        auth_challenges: RwLock::new(HashMap::new()),
        cookie_secret,
    });

    let app = Router::new()
        .route("/", get(spa_page))
        .route("/api/status", get(get_status))
        .route("/health", get(health))
        .route("/nodes", get(get_nodes))
        .route("/swarm", get(get_swarm))
        .route("/swarm/models", get(get_swarm_models))
        .route("/models", get(get_models))
        .route("/activity", get(get_activity))
        .route("/infer", post(post_infer))
        .route("/auth/request", post(post_auth_request))
        .route("/auth/approve", post(post_auth_approve))
        .route("/config/default_model", get(get_default_model).post(post_set_default_model))
        .route("/metrics/history", get(get_metrics_history))
        .route("/dashboard", get(spa_page))
        .route("/chat", get(spa_page))
        .route("/control", get(spa_page))
        .route("/app", get(spa_page))
        .route("/ws/chat", get(ws_chat_handler))
        .route("/ws/audio", get(ws_audio_handler))
        .route("/api/tools", get(api_list_tools))
        .route("/api/sessions", get(api_list_sessions))
        .route("/api/sessions/search", get(api_search_sessions))
        .route("/api/sessions/:id/messages", get(api_get_session_messages))
        .route("/api/voice/status", get(api_voice_status))
        .route("/api/webhook", post(api_webhook))
        .route("/api/preload", post(api_preload))
        .route("/api/rpc", post(api_rpc))
        .route("/api/files/suggest", get(api_files_suggest))
        .route("/api/onboarding", get(api_onboarding))
        .route("/api/cwd", get(api_get_cwd).post(api_set_cwd))
        .route("/api/config/channels", get(api_get_channels_config).post(api_save_channels_config))
        .route("/api/config/provider", get(api_get_provider_config).post(api_save_provider_config))
        .route("/api/profiles", get(api_get_profiles).post(api_upsert_profile))
        .route("/api/profiles/models", get(api_get_unified_models))
        .route("/api/profiles/active", post(api_set_active_model))
        .route("/api/profiles/:id", axum::routing::delete(api_delete_profile))
        .route("/api/channels/start", post(api_start_channel))
        .route("/api/channels/stop", post(api_stop_channel))
        .route("/api/channels/status", get(api_channels_status))
        .route("/api/knowledge", get(api_list_knowledge).post(api_add_knowledge))
        .route("/api/knowledge/:id", axum::routing::delete(api_delete_knowledge).put(api_update_knowledge))
        .route("/api/doctor", get(api_doctor))
        .route("/api/sessions/:id/export", get(api_export_session))
        .route("/api/sessions/:id/fork", post(api_fork_session))
        .route("/api/sessions/:id", axum::routing::delete(api_delete_session))
        .route("/api/cron", get(api_list_cron).post(api_create_cron))
        .route("/api/cron/:id", axum::routing::delete(api_delete_cron))
        .route("/api/mcp", post(mcp::api_mcp_handler))
        .route("/api/channels/discord/webhook", post(api_discord_webhook))
        .route("/api/channels/slack/events", post(api_slack_events))
        // Auth routes
        .route("/api/auth/enroll", post(api_auth_enroll))
        .route("/api/auth/me", get(api_auth_me))
        .route("/api/auth/challenge", get(api_auth_challenge))
        .route("/api/auth/status/:id", get(api_auth_status))
        .route("/api/auth/logout", post(api_auth_logout))
        .route("/api/auth/login", post(api_auth_login))
        .route("/api/auth/password", post(api_auth_set_password))
        .route("/api/auth/model", post(api_auth_set_model))
        .route("/auth/scan/:id", get(auth_scan_challenge))
        .route("/auth/link/:user_id/:secret", get(auth_permanent_link))
        .route("/login", get(spa_page))
        // Internal sync routes (peer-to-peer)
        .route("/api/internal/sync/session", post(sync::handle_session_sync))
        .route("/api/internal/sync/user", post(sync::handle_user_sync))
        .route("/api/internal/sync/bulk", get(sync::handle_bulk_sync))
        .layer(
            tower_http::cors::CorsLayer::new()
                .allow_origin(tower_http::cors::AllowOrigin::any())
                .allow_methods(tower_http::cors::Any)
                .allow_headers(tower_http::cors::Any),
        )
        .with_state(state.clone());

    // Background: refresh real metrics + re-announce mDNS + periodic save
    let update_state = state.clone();
    let bg_broadcaster = broadcaster.clone();
    tokio::spawn(async move {
        let mut interval =
            tokio::time::interval(std::time::Duration::from_secs(MDNS_REANNOUNCE_INTERVAL_SECS));
        let start_time = std::time::Instant::now();
        let mut tick_count: u64 = 0;
        loop {
            interval.tick().await;
            tick_count += 1;

            {
                let mut sys = update_state.sys.write().await;
                sys.refresh_cpu_usage();
                sys.refresh_memory();
            }

            // Periodic save every 60 seconds (30 ticks at 2s interval)
            if tick_count % 30 == 0 {
                save_persistent_state(&update_state).await;
            }

            {
                let queue_depth = update_state.queue.read().await.depth() as u32;
                let mut manifest = update_state.manifest.write().await;
                manifest.uptime_secs = start_time.elapsed().as_secs();
                manifest.timestamp = chrono::Utc::now();

                let sys = update_state.sys.read().await;
                manifest.resources.memory_used_mb = sys.used_memory() / 1024;
                manifest.resources.memory_total_mb = sys.total_memory() / 1024;
                manifest.resources.cpu_usage_pct = sys.global_cpu_usage();
                manifest.performance.queue_depth = queue_depth;

                // GPU/VRAM metrics via nvidia-smi (every 10 ticks = 20 seconds)
                if tick_count % 10 == 0 {
                    if let Ok(output) = std::process::Command::new("nvidia-smi")
                        .args(["--query-gpu=utilization.gpu,memory.used,memory.total,temperature.gpu", "--format=csv,noheader,nounits"])
                        .output()
                    {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        let parts: Vec<&str> = stdout.trim().split(',').map(|s| s.trim()).collect();
                        if parts.len() >= 4 {
                            manifest.resources.accelerator_usage_pct = parts[0].parse::<f32>().ok();
                            manifest.resources.vram_used_mb = parts[1].parse::<u64>().ok();
                            manifest.resources.vram_total_mb = parts[2].parse::<u64>().ok();
                            manifest.resources.temperature_c = parts[3].parse::<f32>().ok();
                        }
                    }
                }

                // Re-announce via mDNS so listeners refresh last_seen
                if let Err(e) = bg_broadcaster.update(&manifest) {
                    tracing::warn!("mDNS re-announce failed: {}", e);
                }
            }

            // Collect metrics snapshot every 5 ticks (10 seconds)
            if tick_count % 5 == 0 {
                let manifest = update_state.manifest.read().await;
                let sys = update_state.sys.read().await;
                let queue_depth = update_state.queue.read().await.depth() as u32;
                let total_mem = sys.total_memory();
                let used_mem = sys.used_memory();
                let ram_pct = if total_mem > 0 { (used_mem as f32 / total_mem as f32) * 100.0 } else { 0.0 };

                // Count nodes from listener
                let listener = update_state.listener.read().await;
                let nodes = listener.get_nodes().await;
                let node_count = nodes.len() + 1; // +1 for self

                let gpu_pct = manifest.resources.accelerator_usage_pct;
                let vram_pct = match (manifest.resources.vram_used_mb, manifest.resources.vram_total_mb) {
                    (Some(used), Some(total)) if total > 0 => Some((used as f32 / total as f32) * 100.0),
                    _ => None,
                };

                let snapshot = MetricsSnapshot {
                    epoch_ms: chrono::Utc::now().timestamp_millis() as u64,
                    cpu_pct: sys.global_cpu_usage(),
                    ram_pct,
                    tokens_per_sec: manifest.performance.tokens_per_sec,
                    queue_depth,
                    node_count,
                    gpu_pct,
                    vram_pct,
                };

                let mut history = update_state.metrics_history.write().await;
                if history.len() >= METRICS_HISTORY_LIMIT {
                    history.pop_front();
                }
                history.push_back(snapshot);

                // Detect node connect/disconnect events
                let current_ids: HashSet<String> = nodes.keys().map(|k| k.to_string()).collect();
                let mut known = update_state.known_node_ids.write().await;
                let now_ms = chrono::Utc::now().timestamp_millis() as u64;

                // New nodes (connected)
                for id in current_ids.difference(&known) {
                    if let Some(node) = nodes.get(id.as_str()) {
                        let name = node.manifest.node_name.clone().unwrap_or_else(|| id.clone());
                        let mut events = update_state.node_events.write().await;
                        if events.len() >= NODE_EVENTS_LIMIT { events.pop_front(); }
                        events.push_back(NodeEvent {
                            epoch_ms: now_ms,
                            event_type: "connected".into(),
                            node_name: name,
                        });
                        // Bulk sync from new peer
                        let peer_host = node.manifest.host.clone();
                        let peer_port = node.manifest.port.unwrap_or(miel_protocol::DEFAULT_API_PORT);
                        let sync_state = update_state.clone();
                        tokio::spawn(async move {
                            sync::fetch_bulk_from_peer(&peer_host, peer_port, &sync_state).await;
                        });
                    }
                }
                // Removed nodes (disconnected)
                for id in known.difference(&current_ids) {
                    let mut events = update_state.node_events.write().await;
                    if events.len() >= NODE_EVENTS_LIMIT { events.pop_front(); }
                    events.push_back(NodeEvent {
                        epoch_ms: now_ms,
                        event_type: "disconnected".into(),
                        node_name: id.clone(),
                    });
                }
                *known = current_ids;
            }
        }
    });

    // Background: Auth challenge cleanup (every 30 seconds)
    let challenge_state = state.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
            let mut challenges = challenge_state.auth_challenges.write().await;
            let before = challenges.len();
            challenges.retain(|_, c| !c.is_expired());
            let removed = before - challenges.len();
            if removed > 0 {
                tracing::debug!(removed, "Expired auth challenges cleaned up");
            }
        }
    });

    // Background: Ollama heartbeat (every 60 seconds)
    let heartbeat_state = state.clone();
    tokio::spawn(async move {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .unwrap();
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        let mut was_down = false;
        loop {
            interval.tick().await;
            let url = format!("{}/api/tags", heartbeat_state.essaim_config.read().await.ollama_url);
            match client.get(&url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    if was_down {
                        info!("Ollama heartbeat: recovered (back online)");
                        let mut activity = heartbeat_state.activity_log.write().await;
                        if activity.len() >= ACTIVITY_LOG_LIMIT { activity.pop_front(); }
                        activity.push_back(ActivityLogEntry {
                            timestamp: chrono::Utc::now().to_rfc3339(),
                            level: "info".into(),
                            tag: "heartbeat".into(),
                            message: "Ollama recovered".into(),
                            full_prompt: None, full_response: None,
                            model_used: None, tokens_generated: None, latency_ms: None,
                            user_id: None,
                        });
                        was_down = false;
                    }
                }
                _ => {
                    if !was_down {
                        warn!("Ollama heartbeat: DOWN (not responding)");
                        let mut activity = heartbeat_state.activity_log.write().await;
                        if activity.len() >= ACTIVITY_LOG_LIMIT { activity.pop_front(); }
                        activity.push_back(ActivityLogEntry {
                            timestamp: chrono::Utc::now().to_rfc3339(),
                            level: "error".into(),
                            tag: "heartbeat".into(),
                            message: "Ollama is not responding".into(),
                            full_prompt: None, full_response: None,
                            model_used: None, tokens_generated: None, latency_ms: None,
                            user_id: None,
                        });
                        was_down = true;
                    }
                }
            }
        }
    });

    // Background: Cron task checker (every 30 seconds)
    let cron_state = state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
        loop {
            interval.tick().await;
            let due_tasks = {
                let mut cron = cron_state.essaim_cron.write().await;
                cron.check_due_tasks()
            };
            for (task_id, prompt) in due_tasks {
                info!(task_id = %task_id, "Executing scheduled task");
                let current_model = get_llm_default(&cron_state).await;
                let sessions_dir = std::path::Path::new("sessions");
                let mut session = Session::new_with_path(&current_model, sessions_dir);
                let (tx, _rx) = broadcast::channel::<ChatEvent>(64);
                let mut cron_config = cron_state.essaim_config.read().await.clone();
                cron_config.model = current_model;
                let result = boucle_react(
                    &prompt,
                    &mut session,
                    &cron_state.essaim_registry,
                    &cron_config,
                    &tx,
                ).await;
                match &result {
                    Ok(response) => {
                        info!(task_id = %task_id, response_len = response.len(), "Scheduled task completed");
                    }
                    Err(e) => {
                        warn!(task_id = %task_id, error = %e, "Scheduled task failed");
                    }
                }
                // Log to activity
                let now = chrono::Utc::now().to_rfc3339();
                let mut activity = cron_state.activity_log.write().await;
                if activity.len() >= ACTIVITY_LOG_LIMIT { activity.pop_front(); }
                activity.push_back(ActivityLogEntry {
                    timestamp: now,
                    level: if result.is_ok() { "info" } else { "error" }.into(),
                    tag: "cron".into(),
                    message: format!("Cron task: {}", preview_text(&prompt, 60)),
                    full_prompt: Some(prompt),
                    full_response: result.ok().map(|r| preview_text(&r, 200)),
                    model_used: Some(cron_config.model.clone()),
                    tokens_generated: None,
                    latency_ms: None,
                    user_id: None,
                });
            }
        }
    });

    // Graceful shutdown: save state on Ctrl+C
    let shutdown_state = state.clone();
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            info!("Shutting down — saving persistent state...");
            save_persistent_state(&shutdown_state).await;
            std::process::exit(0);
        }
    });

    let addr = format!("0.0.0.0:{}", config.api_port);
    info!("LaRuche ready → http://localhost:{}", config.api_port);

    // Sync essaim config from active profile at startup
    sync_essaim_from_profiles(&state).await;

    // Auto-start channels if configured
    {
        let config_path = std::path::Path::new("channels-config.json");
        if config_path.exists() {
            if let Ok(content) = std::fs::read_to_string(config_path) {
                if let Ok(channels_config) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(tg_token) = channels_config["telegram"]["bot_token"].as_str() {
                        if !tg_token.is_empty() && channels_config["telegram"]["enabled"].as_bool().unwrap_or(false) {
                            let allowed = channels_config["telegram"]["allowed_chats"].as_str().unwrap_or("").to_string();
                            let token = tg_token.to_string();
                            let state_for_tg = state.clone();
                            let handle = tokio::spawn(async move {
                                run_telegram_bot(&token, &allowed, &state_for_tg).await;
                            });
                            state.channel_handles.write().await.insert("telegram".into(), handle);
                            info!("Telegram bot auto-started from config");
                        }
                    }
                }
            }
        }
    }

    // TLS support: if LARUCHE_TLS_CERT and LARUCHE_TLS_KEY are set, use HTTPS
    let tls_cert = std::env::var("LARUCHE_TLS_CERT").ok();
    let tls_key = std::env::var("LARUCHE_TLS_KEY").ok();

    if use_tui {
        // Spawn server in background, run TUI in foreground
        let tui_state = state.clone();
        tokio::spawn(async move {
            if let (Some(cert_path), Some(key_path)) = (tls_cert, tls_key) {
                info!(cert = %cert_path, key = %key_path, "TLS enabled — starting HTTPS server");
                let tls_config = axum_server::tls_rustls::RustlsConfig::from_pem_file(&cert_path, &key_path)
                    .await
                    .expect("Failed to load TLS certificate/key");
                let _ = axum_server::bind_rustls(addr.parse().expect("Invalid bind address"), tls_config)
                    .serve(app.into_make_service_with_connect_info::<SocketAddr>())
                    .await;
            } else {
                let listener_tcp = tokio::net::TcpListener::bind(&addr).await.expect("Failed to bind");
                let _ = axum::serve(
                    listener_tcp,
                    app.into_make_service_with_connect_info::<SocketAddr>(),
                )
                .await;
            }
        });

        // Run TUI (blocks until user presses 'q')
        if let Some(rx) = tui_log_rx {
            tui::run_tui(tui_state.clone(), rx).await?;
        }

        // TUI exited — save state and shutdown
        save_persistent_state(&tui_state).await;
    } else {
        // --no-tui mode: spawn server + system tray (Windows)
        let (tray_shutdown_tx, tray_shutdown_rx) = tokio::sync::oneshot::channel::<()>();

        // Spawn systray on a dedicated OS thread (requires win32 message pump)
        let tray_port = config.api_port;
        std::thread::spawn(move || {
            systray::run_systray(tray_port, tray_shutdown_tx);
        });

        // Spawn HTTP server
        tokio::spawn(async move {
            if let (Some(cert_path), Some(key_path)) = (tls_cert, tls_key) {
                info!(cert = %cert_path, key = %key_path, "TLS enabled — starting HTTPS server");
                let tls_config = axum_server::tls_rustls::RustlsConfig::from_pem_file(&cert_path, &key_path)
                    .await
                    .expect("Failed to load TLS certificate/key");
                let _ = axum_server::bind_rustls(addr.parse().expect("Invalid bind address"), tls_config)
                    .serve(app.into_make_service_with_connect_info::<SocketAddr>())
                    .await;
            } else {
                let listener_tcp = tokio::net::TcpListener::bind(&addr).await.expect("Failed to bind");
                let _ = axum::serve(
                    listener_tcp,
                    app.into_make_service_with_connect_info::<SocketAddr>(),
                )
                .await;
            }
        });

        // Wait for either Ctrl+C or tray "Quit"
        let save_state = state.clone();
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                info!("Ctrl+C received — shutting down...");
            }
            _ = tray_shutdown_rx => {
                info!("Quit from system tray — shutting down...");
            }
        }
        save_persistent_state(&save_state).await;
    }

    Ok(())
}

fn parse_tier(value: &str) -> Option<HardwareTier> {
    match value.to_ascii_lowercase().as_str() {
        "nano" => Some(HardwareTier::Nano),
        "core" => Some(HardwareTier::Core),
        "pro" => Some(HardwareTier::Pro),
        "max" => Some(HardwareTier::Max),
        _ => None,
    }
}

fn parse_env_capabilities(default_model: &str) -> Option<Vec<CapabilityConfig>> {
    let cap1 = std::env::var("LARUCHE_CAP").ok()?;
    let model1 = std::env::var("LARUCHE_MODEL").unwrap_or_else(|_| default_model.to_string());

    let mut caps = vec![CapabilityConfig {
        capability: cap1,
        model_name: model1,
        model_size: None,
        quantization: None,
    }];

    if let Ok(cap2) = std::env::var("LARUCHE_CAP2") {
        let model2 = std::env::var("LARUCHE_MODEL2").unwrap_or_else(|_| default_model.to_string());
        caps.push(CapabilityConfig {
            capability: cap2,
            model_name: model2,
            model_size: None,
            quantization: None,
        });
    }

    Some(caps)
}

/// GET /metrics/history - Time-series metrics for dashboard charts
async fn get_metrics_history(
    State(state): State<Arc<AppState>>,
) -> Json<MetricsHistoryResponse> {
    let snapshots = state.metrics_history.read().await;
    let events = state.node_events.read().await;
    Json(MetricsHistoryResponse {
        snapshots: snapshots.iter().cloned().collect(),
        events: events.iter().cloned().collect(),
    })
}

// ── Persistence ──────────────────────────────────────────────────────

fn resolve_state_file_path() -> PathBuf {
    if let Ok(dir) = std::env::var("LARUCHE_DATA_DIR") {
        PathBuf::from(dir).join("laruche-state.json")
    } else {
        PathBuf::from("laruche-state.json")
    }
}

fn load_persistent_state(path: &std::path::Path) -> PersistentState {
    match std::fs::read_to_string(path) {
        Ok(raw) => {
            match serde_json::from_str::<PersistentState>(&raw) {
                Ok(s) => {
                    info!(path = %path.display(), entries = s.activity_log.len(), "Loaded persistent state");
                    s
                }
                Err(e) => {
                    warn!(path = %path.display(), error = %e, "Failed to parse state file, starting fresh");
                    PersistentState::default()
                }
            }
        }
        Err(_) => {
            info!(path = %path.display(), "No state file found, starting fresh");
            PersistentState::default()
        }
    }
}

async fn save_persistent_state(state: &Arc<AppState>) {
    let logs = state.activity_log.read().await;
    let dm = state.default_models.read().await;
    let llm_default = dm.get("llm").cloned();
    let persistent = PersistentState {
        default_model: llm_default, // backward compat
        default_models: Some(dm.clone()),
        activity_log: logs.iter().cloned().collect(),
        saved_at: chrono::Utc::now().to_rfc3339(),
        cookie_secret: Some(auth_user::cookie_secret_to_base64(&state.cookie_secret)),
    };
    drop(logs);
    drop(dm);

    let json = match serde_json::to_string_pretty(&persistent) {
        Ok(j) => j,
        Err(e) => {
            warn!(error = %e, "Failed to serialize state");
            return;
        }
    };

    let tmp_path = state.state_file_path.with_extension("json.tmp");
    if let Err(e) = tokio::fs::write(&tmp_path, &json).await {
        warn!(error = %e, "Failed to write state temp file");
        return;
    }
    if let Err(e) = tokio::fs::rename(&tmp_path, &state.state_file_path).await {
        warn!(error = %e, "Failed to rename state file");
    }
}

fn load_config() -> Result<NodeConfig> {
    let config_path = std::env::var("LARUCHE_CONFIG").unwrap_or_else(|_| "laruche.toml".into());
    let mut config = NodeConfig::default();

    if std::path::Path::new(&config_path).exists() {
        let raw = fs::read_to_string(&config_path)?;
        let file_cfg: NodeConfigFile = toml::from_str(&raw)?;

        if let Some(v) = file_cfg.node_name {
            config.node_name = v;
        }
        if let Some(v) = file_cfg.tier {
            config.tier = v;
        }
        if let Some(v) = file_cfg.ollama_url {
            config.ollama_url = v;
        }
        if let Some(v) = file_cfg.default_model {
            config.default_model = v;
        }
        if let Some(v) = file_cfg.api_port {
            config.api_port = v;
        }
        if let Some(v) = file_cfg.dashboard_port {
            config.dashboard_port = v;
        }
        if let Some(v) = file_cfg.capabilities {
            config.capabilities = v;
        }
        if let Some(v) = file_cfg.provider {
            config.provider = v;
        }
        if let Some(v) = file_cfg.api_key {
            config.api_key = v;
        }
        if let Some(v) = file_cfg.api_base {
            config.api_base = Some(v);
        }

        info!(path = %config_path, "Loaded config file");
    }

    // Environment variables override config file values (with warnings)
    if let Ok(v) = std::env::var("LARUCHE_NAME") {
        info!(env = "LARUCHE_NAME", value = %v, "Env override: node_name");
        config.node_name = v;
    }
    if let Ok(v) = std::env::var("LARUCHE_TIER") {
        if let Some(tier) = parse_tier(&v) {
            info!(env = "LARUCHE_TIER", value = %v, "Env override: tier");
            config.tier = tier;
        }
    }
    if let Ok(v) = std::env::var("OLLAMA_URL") {
        info!(env = "OLLAMA_URL", value = %v, "Env override: ollama_url");
        config.ollama_url = v;
    }
    if let Ok(v) = std::env::var("LARUCHE_MODEL") {
        info!(env = "LARUCHE_MODEL", value = %v, "Env override: default_model");
        config.default_model = v;
    }
    if let Ok(v) = std::env::var("LARUCHE_PORT") {
        if let Ok(port) = v.parse::<u16>() {
            info!(env = "LARUCHE_PORT", value = %v, "Env override: api_port");
            config.api_port = port;
        }
    }
    if let Ok(v) = std::env::var("LARUCHE_DASH_PORT") {
        if let Ok(port) = v.parse::<u16>() {
            info!(env = "LARUCHE_DASH_PORT", value = %v, "Env override: dashboard_port");
            config.dashboard_port = port;
        }
    }

    if let Ok(v) = std::env::var("LARUCHE_PROVIDER") {
        info!(env = "LARUCHE_PROVIDER", value = %v, "Env override: provider");
        config.provider = v;
    }
    if let Ok(v) = std::env::var("LARUCHE_API_KEY") {
        info!(env = "LARUCHE_API_KEY", "Env override: api_key (redacted)");
        config.api_key = v;
    }
    if let Ok(v) = std::env::var("LARUCHE_API_BASE") {
        info!(env = "LARUCHE_API_BASE", value = %v, "Env override: api_base");
        config.api_base = Some(v);
    }

    if let Some(caps) = parse_env_capabilities(&config.default_model) {
        info!("Env override: capabilities from LARUCHE_CAP/LARUCHE_MODEL");
        config.capabilities = caps;
    }

    if config.capabilities.is_empty() {
        config.capabilities = vec![CapabilityConfig {
            capability: "llm".into(),
            model_name: config.default_model.clone(),
            model_size: Some("7B".into()),
            quantization: Some("Q4_K_M".into()),
        }];
    }

    for cap in &mut config.capabilities {
        cap.capability = normalize_capability_label(&cap.capability);
    }

    Ok(config)
}
