//! LaRuche Node Daemon
//!
//! The main process that runs on each LaRuche box. It:
//! 1. Broadcasts its Cognitive Manifest via LAND (mDNS)
//! 2. Listens for peer nodes (swarm)
//! 3. Exposes an inference API (proxying to Ollama)
//! 4. Manages authentication via Proof of Proximity
//! 5. Runs the web dashboard
//! 6. Exposes /models to list available Ollama models
//! 7. Reports real system metrics (CPU, RAM) via sysinfo

use anyhow::Result;
use axum::{
    extract::{ConnectInfo, State},
    http::StatusCode,
    response::{Html, Json},
    routing::{get, post},
    Router,
};
use land_protocol::{
    auth::ProximityAuth,
    capabilities::{Capability, CapabilityInfo},
    discovery::{LandBroadcaster, LandListener},
    manifest::{CognitiveManifest, HardwareTier},
    qos::{QosPolicy, RequestQueue},
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, collections::HashSet, fs, net::SocketAddr, path::PathBuf, sync::Arc, time::Duration};
use sysinfo::System;
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use uuid::Uuid;

use std::collections::VecDeque;

const DASHBOARD_HTML: &str = include_str!("../../laruche-dashboard/src/templates/dashboard.html");
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
    listener: RwLock<LandListener>,
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
            api_port: land_protocol::DEFAULT_API_PORT,
            dashboard_port: land_protocol::DEFAULT_DASHBOARD_PORT,
            capabilities: vec![CapabilityConfig {
                capability: "llm".into(),
                model_name: "mistral-7b".into(),
                model_size: Some("7B".into()),
                quantization: Some("Q4_K_M".into()),
            }],
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

#[derive(Debug, Serialize)]
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
    /// Primary model running on this node (from LAND TXT record)
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

/// Infer a LAND capability from a model name using heuristics.
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
            .unwrap_or(land_protocol::DEFAULT_API_PORT);

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
            .unwrap_or(land_protocol::DEFAULT_API_PORT);
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
        "family" => land_protocol::auth::TrustCircle::Family,
        "office" => land_protocol::auth::TrustCircle::Office,
        _ => land_protocol::auth::TrustCircle::Guest,
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
    });

    info!(capability = %capability, prev = %prev, new = %model_name, "Default model changed via API");

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

/// GET /activity - Recent inference and system activity
async fn get_activity(State(state): State<Arc<AppState>>) -> Json<ActivityResponse> {
    let logs = state.activity_log.read().await;
    Json(ActivityResponse {
        logs: logs.iter().cloned().collect(),
    })
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok" }))
}

async fn dashboard() -> Html<&'static str> {
    Html(DASHBOARD_HTML)
}

// ======================== Main ========================

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "laruche_node=info,land_protocol=info".into()),
        )
        .init();

    let config = load_config()?;

    info!(
        r#"
  ██╗      █████╗ ██████╗ ██╗   ██╗ ██████╗██╗  ██╗███████╗
  ██║     ██╔══██╗██╔══██╗██║   ██║██╔════╝██║  ██║██╔════╝
  ██║     ███████║██████╔╝██║   ██║██║     ███████║█████╗
  ██║     ██╔══██║██╔══██╗██║   ██║██║     ██╔══██║██╔══╝
  ███████╗██║  ██║██║  ██║╚██████╔╝╚██████╗██║  ██║███████╗
  ╚══════╝╚═╝  ╚═╝╚═╝  ╚═╝ ╚═════╝  ╚═════╝╚═╝  ╚═╝╚══════╝
  Branchez l'IA. C'est tout. • LAND Protocol v{}
    "#,
        land_protocol::PROTOCOL_VERSION
    );

    info!(name = %config.node_name, tier = ?config.tier, "Starting LaRuche node");

    let local_ip = land_protocol::get_local_ip();
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

    let mut broadcaster = LandBroadcaster::new()?;
    broadcaster.register(&manifest)?;
    let broadcaster = Arc::new(broadcaster);

    let mut listener = LandListener::new()?;
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
    });

    let app = Router::new()
        .route("/", get(get_status))
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
        .route("/dashboard", get(dashboard))
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

                let snapshot = MetricsSnapshot {
                    epoch_ms: chrono::Utc::now().timestamp_millis() as u64,
                    cpu_pct: sys.global_cpu_usage(),
                    ram_pct,
                    tokens_per_sec: manifest.performance.tokens_per_sec,
                    queue_depth,
                    node_count,
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
    info!(addr = %addr, "LaRuche API server starting");
    info!(
        dashboard = format!("http://localhost:{}/dashboard", config.api_port),
        "Embedded Dashboard available at"
    );

    let listener_tcp = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(
        listener_tcp,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

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
