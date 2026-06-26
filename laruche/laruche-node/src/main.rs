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

mod abeilles_local;
mod auth_user;
mod local_inference;
mod mcp;
mod missions;
mod profiles;
mod sync;
mod systray;
mod tui;

use anyhow::Result;
use axum::{
    extract::{ws, ConnectInfo, Path, Query, State, WebSocketUpgrade},
    http::StatusCode,
    response::{Html, IntoResponse, Json},
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
use std::{
    collections::HashMap, collections::HashSet, fs, net::SocketAddr, path::PathBuf, sync::Arc,
    time::Duration,
};
use sysinfo::System;
use tokio::sync::{broadcast, RwLock};
use tracing::{error, info, warn};
use uuid::Uuid;

use laruche_essaim::{
    abeilles::{charger_plugins, enregistrer_abeilles_builtin, enregistrer_delegation},
    brain::{boucle_react_memoire, boucle_react_memoire_multimodal},
    cron::{CronScheduler, ScheduledTask},
    mcp_client::charger_mcp_servers,
    AbeilleRegistry, ChatEvent, EssaimConfig, Session,
};

use std::collections::VecDeque;

const SPA_HTML: &str = include_str!("../../laruche-dashboard/src/templates/spa.html");
const PEER_FETCH_TIMEOUT_MS: u64 = 4000;
// Fenêtre de péremption d'un pair. DOIT être > l'intervalle de ré-annonce mDNS (30s ci-dessous),
// sinon un pair « clignote » : il devient périmé entre deux annonces. 90s = tolère 2 annonces ratées.
const PEER_STALE_SECS: i64 = 90;
const MDNS_REANNOUNCE_INTERVAL_SECS: u64 = 2;
const ACTIVITY_LOG_LIMIT: usize = 400;

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
    /// Sélection de service par capacité (avec source) — survit au redémarrage.
    #[serde(default)]
    capability_selection: Option<HashMap<String, CapabilitySelection>>,
    #[serde(default)]
    activity_log: Vec<ActivityLogEntry>,
    #[serde(default)]
    disabled_tools: Vec<String>,
    #[serde(default)]
    disabled_skills: Vec<String>,
    /// Permission mode ("default" | "plan" | "acceptEdits" | "auto" | "bubble").
    #[serde(default)]
    permission_mode: Option<String>,
    #[serde(default)]
    saved_at: String,
    /// BLAKE3 cookie secret (base64), shared across cluster
    #[serde(default)]
    cookie_secret: Option<String>,
    #[serde(default)]
    context_max_messages: Option<usize>,
    #[serde(default)]
    context_max_tokens: Option<u32>,
    #[serde(default)]
    compaction_threshold: Option<f32>,
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

/// Sélection courante d'un service pour une capacité donnée (stt/tts/code/vlm/vla/llm…).
/// Permet d'aller au-delà du simple nom de modèle : on garde la SOURCE (backend / node mesh)
/// pour router (ex. la dictée vocale vers le STT choisi, l'auto-TTS vers le TTS choisi).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CapabilitySelection {
    capability: String,
    model: String,
    /// Backend/host (label local "llama.cpp"… ou IP du nœud mesh).
    backend: String,
    /// Id du nœud Miel distant (None si service local).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    node_id: Option<String>,
    is_local: bool,
    /// Profil provider qui sert cette capacité (pour résoudre provider/base_url/clé au runtime).
    profile_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomService {
    pub name: String,
    pub capability: String,
    pub url: String,
    pub protocol: String,
}

#[derive(Debug, Clone, Default)]
struct ActiveContextStats {
    messages: u32,
    base_tokens: u32,
    streamed_chars: usize,
    extra_tokens: u32,
    streaming_response_open: bool,
    running: bool,
}

impl ActiveContextStats {
    fn from_session(session: &Session, running: bool) -> Self {
        Self {
            messages: session.len() as u32,
            base_tokens: session.estimated_tokens() as u32,
            streamed_chars: 0,
            extra_tokens: 0,
            streaming_response_open: false,
            running,
        }
    }

    fn used_tokens(&self) -> u32 {
        self.base_tokens
            .saturating_add((self.streamed_chars / 4) as u32)
            .saturating_add(self.extra_tokens)
    }

    fn apply_event(&mut self, event: &ChatEvent) {
        match event {
            ChatEvent::Token { text } => {
                if !text.is_empty() {
                    self.streamed_chars = self.streamed_chars.saturating_add(text.len());
                    self.streaming_response_open = true;
                    self.running = true;
                }
            }
            ChatEvent::ToolCall { name, args, .. } => {
                if self.streaming_response_open {
                    self.messages = self.messages.saturating_add(1);
                    self.streaming_response_open = false;
                }
                self.messages = self.messages.saturating_add(1);
                self.extra_tokens = self
                    .extra_tokens
                    .saturating_add(approx_context_tokens(&format!("{name}{args}")));
                self.running = true;
            }
            ChatEvent::ToolResult { name, result, .. } => {
                self.messages = self.messages.saturating_add(1);
                self.extra_tokens = self
                    .extra_tokens
                    .saturating_add(approx_context_tokens(&format!("{name}{result}")));
                self.running = true;
            }
            ChatEvent::Usage {
                input_tokens,
                output_tokens,
                ..
            } => {
                let usage_total = input_tokens.saturating_add(*output_tokens);
                if usage_total > self.used_tokens() {
                    self.base_tokens = usage_total;
                    self.streamed_chars = 0;
                    self.extra_tokens = 0;
                }
            }
            ChatEvent::Done { full_response } => {
                if self.streaming_response_open || !full_response.is_empty() {
                    self.messages = self.messages.saturating_add(1);
                    self.streaming_response_open = false;
                }
                self.running = false;
            }
            ChatEvent::Error { .. } => {
                self.running = false;
            }
            _ => {}
        }
    }
}

fn approx_context_tokens(text: &str) -> u32 {
    if text.is_empty() {
        0
    } else {
        ((text.len() + 3) / 4) as u32
    }
}

async fn update_active_context_stats(
    state: &Arc<AppState>,
    session_id: Uuid,
    event: &ChatEvent,
) {
    let mut stats_by_session = state.active_context_stats.write().await;
    let stats = stats_by_session
        .entry(session_id)
        .or_insert_with(|| ActiveContextStats {
            running: true,
            ..ActiveContextStats::default()
        });

    stats.apply_event(event);
}

struct AppState {
    manifest: RwLock<CognitiveManifest>,
    auth: RwLock<ProximityAuth>,
    queue: RwLock<RequestQueue>,
    listener: RwLock<MielListener>,
    config: NodeConfig,
    /// Services mesh déclarés manuellement (P6)
    custom_services: RwLock<HashMap<String, CustomService>>,
    /// Per-capability default models (e.g. "llm" → "mistral", "code" → "qwen3-coder:30b")
    /// The "llm" key is the universal fallback for unspecified capabilities.
    default_models: RwLock<HashMap<String, String>>,
    /// Sélection de service par capacité (avec source), pour le routing voix/code/vision.
    capability_selection: RwLock<HashMap<String, CapabilitySelection>>,
    /// Missions au long cours (« La Reine ») — métadonnée ; le savoir vit dans la carte cognitive.
    missions: RwLock<missions::MissionStore>,
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
    memoire: Arc<dyn laruche_memoire::MemoireCognitive>,
    essaim_sessions: Arc<RwLock<HashMap<Uuid, Session>>>,
    active_context_stats: Arc<RwLock<HashMap<Uuid, ActiveContextStats>>>,
    essaim_cron: Arc<RwLock<CronScheduler>>,
    watchers: Arc<RwLock<laruche_watchers::WatchersRegistry>>,
    kanban_board: Arc<RwLock<laruche_kanban::KanbanBoard>>,
    essaim_kb: Arc<tokio::sync::RwLock<laruche_essaim::rag::KnowledgeBase>>,
    events: Arc<RwLock<laruche_events::EventBus>>,
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
    /// Credential pool for multiple API keys per provider
    credential_pool: Arc<RwLock<laruche_essaim::credential_pool::CredentialPool>>,
    /// Path to credentials.json
    credentials_path: PathBuf,
    /// Last activity timestamp to trigger Dream mode
    last_activity: RwLock<std::time::Instant>,
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
    if lower.contains("coder")
        || lower.contains("codestral")
        || lower.contains("deepseek-coder")
        || lower.contains("starcoder")
        || lower.contains("code")
    {
        return "code".into();
    }
    if lower.contains("llava")
        || lower.contains("bakllava")
        || lower.contains("moondream")
        || lower.contains("minicpm-v")
        || lower.contains("vision")
    {
        return "vlm".into();
    }
    if lower.contains("whisper") || lower.contains("audio") {
        return "audio".into();
    }
    if lower.contains("nomic-embed")
        || lower.contains("mxbai-embed")
        || lower.contains("all-minilm")
        || lower.contains("embed")
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
        if check == cap_model
            || check.starts_with(&format!("{}:", cap_model))
            || cap_model.starts_with(&check)
        {
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

fn inject_no_think(prompt: &str, no_think: bool) -> String {
    if no_think && !prompt.trim_start().starts_with("/no_think") {
        format!("/no_think\n{prompt}")
    } else {
        prompt.to_string()
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

// ── Blueprints : modèles d'automatisations cron paramétrés ──────────────────────
// Catalogue intégré (laruche_essaim::blueprints::catalogue) + blueprints CRÉÉS par
// l'utilisateur, persistés dans `blueprints.json`.

fn load_user_blueprints() -> Vec<laruche_essaim::blueprints::Blueprint> {
    std::fs::read_to_string("blueprints.json")
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_user_blueprints(bps: &[laruche_essaim::blueprints::Blueprint]) -> std::io::Result<()> {
    std::fs::write(
        "blueprints.json",
        serde_json::to_string_pretty(bps).unwrap_or_else(|_| "[]".into()),
    )
}

/// GET /api/blueprints — catalogue intégré + blueprints utilisateur.
async fn get_blueprints() -> Json<Vec<laruche_essaim::blueprints::Blueprint>> {
    let mut all = laruche_essaim::blueprints::catalogue();
    all.extend(load_user_blueprints());
    Json(all)
}

/// POST /api/blueprints — crée (ou met à jour) un blueprint utilisateur. Body = Blueprint
/// {id, title, schedule_template, prompt_template, slots:[{name,label,default}]}.
async fn api_create_blueprint(
    Json(mut bp): Json<laruche_essaim::blueprints::Blueprint>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if bp.id.trim().is_empty() {
        // dérive un id depuis le titre
        let slug: String = bp
            .title
            .to_lowercase()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect();
        let slug = slug.trim_matches('-').to_string();
        bp.id = if slug.is_empty() {
            format!("bp-{}", Uuid::new_v4())
        } else {
            slug
        };
    }
    // Interdit d'écraser un blueprint intégré.
    if laruche_essaim::blueprints::catalogue()
        .iter()
        .any(|b| b.id == bp.id)
    {
        return Err(StatusCode::CONFLICT);
    }
    let mut users = load_user_blueprints();
    users.retain(|b| b.id != bp.id); // upsert
    users.push(bp.clone());
    save_user_blueprints(&users).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({ "status": "ok", "id": bp.id })))
}

/// DELETE /api/blueprints/:id — supprime un blueprint utilisateur (les intégrés sont immuables).
async fn api_delete_blueprint(Path(id): Path<String>) -> Json<serde_json::Value> {
    let mut users = load_user_blueprints();
    let before = users.len();
    users.retain(|b| b.id != id);
    let removed = before - users.len();
    let _ = save_user_blueprints(&users);
    Json(serde_json::json!({ "status": "ok", "removed": removed }))
}

/// POST /api/blueprints/:id/instancier — instancie un blueprint en un VRAI cron.
/// Body = valeurs des slots : `{ "<slot>": "<valeur>", ... }` (ou `{slots:{...}}`).
async fn instancier_blueprint(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut all = laruche_essaim::blueprints::catalogue();
    all.extend(load_user_blueprints());
    let Some(bp) = all.into_iter().find(|b| b.id == id) else {
        return Err(StatusCode::NOT_FOUND);
    };
    // Accepte {slots:{...}} ou un objet plat de valeurs.
    let src = body.get("slots").filter(|v| v.is_object()).unwrap_or(&body);
    let mut valeurs: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    if let Some(obj) = src.as_object() {
        for (k, v) in obj {
            if let Some(s) = v.as_str() {
                valeurs.insert(k.clone(), s.to_string());
            }
        }
    }
    let (name, cron_expr, prompt) = laruche_essaim::blueprints::instancier(&bp, &valeurs);
    let task = ScheduledTask {
        id: Uuid::new_v4(),
        name,
        prompt,
        cron_expr: Some(cron_expr),
        fire_at: None,
        channel: None,
        provider: None,
        model: None,
        profile_id: None,
        skills: vec![],
        enabled: true,
        created_at: chrono::Utc::now(),
        last_run: None,
        run_count: 0,
    };
    let cron_id = {
        let mut cron = state.essaim_cron.write().await;
        cron.add(task)
    };
    Ok(Json(
        serde_json::json!({ "status": "ok", "cron_id": cron_id }),
    ))
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
    let estimated_speedup = if n <= 1.0 {
        1.0
    } else {
        1.0 + (n - 1.0) * 0.85
    };
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
            if is_stale(node.last_seen) {
                continue;
            }
            let caps: Vec<String> = node
                .manifest
                .capabilities
                .iter()
                .map(|c| c.to_string())
                .collect();
            if !caps.iter().any(|c| c == "llm") {
                continue;
            }
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
                    match http
                        .post(format!("{}/infer", peer_url))
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
    let max_tokens_val = req.max_tokens.unwrap_or(0);
    let num_predict = if max_tokens_val > 0 {
        serde_json::json!(max_tokens_val)
    } else {
        serde_json::Value::Null
    };
    let ollama_req = serde_json::json!({
        "model": model,
        "prompt": req.prompt,
        "stream": false,
        "options": {
            "num_predict": num_predict,
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
    // Résilient : si Ollama est down, on n'échoue PAS tout l'endpoint (sinon le panneau
    // « Services du mesh » reste bloqué). On liste juste 0 modèle Ollama + les services mesh.
    let local_models = fetch_local_models(&state.config.ollama_url, &dm)
        .await
        .unwrap_or_else(|_| ModelsResponse {
            models: Vec::new(),
            default_model: dm.clone(),
        });
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

    // Backends d'inférence locaux OpenAI-compatibles (llama.cpp, vLLM, LM Studio…).
    // Même logique que pour Ollama : on les liste et on les annonce sur le mesh.
    {
        let detectes = local_inference::detecter_modeles_openai_compat(
            &local_inference::backends_openai_compat_par_defaut(),
        )
        .await;
        for m in detectes {
            // Évite le doublon si le même modèle est déjà exposé localement (ex. Ollama).
            if models.iter().any(|x| x.is_local && x.name == m.name) {
                continue;
            }
            let cap = resolve_model_capability(&m.name, &state.config.capabilities);
            models.push(SwarmModelInfo {
                host: m.backend,
                node_name: format!("{} (local)", m.base_url),
                node_id: None,
                name: m.name,
                size_gb: 0.0,
                digest: String::new(),
                is_default: false,
                is_local: true,
                capability: Some(cap),
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
            let is_profile_local =
                profile.base_url.contains("127.0.0.1") || profile.base_url.contains("localhost");
            // Backend LOCAL (llama.cpp/vLLM/LM Studio) : on ne liste QUE ce qui est réellement
            // détecté EN VIE (détection live plus haut). Un profil local dont le backend est éteint
            // n'apparaît donc PAS (fini les modèles fantômes d'un Ollama/llama.cpp fermé).
            if is_profile_local {
                continue;
            }
            for model_name in &profile.models {
                let is_def = pid == &active.profile_id && model_name == &active.model;
                let cap = resolve_model_capability(model_name, &state.config.capabilities);
                models.push(SwarmModelInfo {
                    host: profile.base_url.clone(), // Use base_url instead of provider for better tracking
                    node_name: profile.name.clone(),
                    node_id: None,
                    name: model_name.clone(),
                    size_gb: 0.0,
                    digest: String::new(),
                    is_default: is_def,
                    is_local: is_profile_local,
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
        // Ne pas se lister SOI-MÊME comme un pair du mesh : un nœud entend sa propre
        // annonce mDNS. Son rôle local est déjà affiché dans SWARM INTELLIGENCE.
        if node.manifest.node_id == Some(manifest.node_id)
            || node.manifest.host == manifest.api_endpoint.host
        {
            continue;
        }
        for cap_str in &node.manifest.capabilities {
            let cap = cap_str.to_string();
            // On ne saute plus les capacités principales, car un noeud Miel
            // peut très bien héberger des modèles LLM/VLM "custom" (hors Ollama).
            let _port = node.manifest.port.unwrap_or(0);
            let model_name = node
                .manifest
                .model
                .clone()
                .unwrap_or_else(|| format!("{}-service", cap));
            let node_name = node
                .manifest
                .node_name
                .clone()
                .unwrap_or_else(|| node.manifest.host.clone());

            // Avoid duplicates
            let already_listed = models
                .iter()
                .any(|m| m.capability.as_deref() == Some(&cap) && m.host == node.manifest.host);
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

    // Add custom services registered manually (P6)
    {
        let custom = state.custom_services.read().await;
        for (_name, service) in custom.iter() {
            models.push(SwarmModelInfo {
                host: service.url.clone(), // using url as host for custom
                node_name: format!("{} (custom)", service.name),
                node_id: None,
                name: service.name.clone(),
                size_gb: 0.0,
                digest: String::new(),
                is_default: false,
                is_local: true, // We treat them as local proxy
                capability: Some(service.capability.clone()),
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
        return Json(
            serde_json::json!({ "status": "error", "message": "model name cannot be empty" }),
        );
    }

    let capability = normalize_capability_label(req.capability.as_deref().unwrap_or("llm"));

    let prev = {
        let mut dm = state.default_models.write().await;
        let prev = dm.get(&capability).cloned().unwrap_or_default();
        dm.insert(capability.clone(), model_name.clone());
        prev
    };

    // Log the change
    let cap_label = if capability == "llm" {
        "".into()
    } else {
        format!(" ({capability})")
    };
    let mut activity = state.activity_log.write().await;
    if activity.len() >= ACTIVITY_LOG_LIMIT {
        activity.pop_front();
    }
    activity.push_back(ActivityLogEntry {
        timestamp: chrono::Utc::now().to_rfc3339(),
        level: "log-ok".into(),
        tag: "MODEL".into(),
        message: format!(
            "Default{cap_label} model changed: {} → {}",
            prev, model_name
        ),
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
async fn get_default_model(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let dm = state.default_models.read().await;
    let llm_default = dm
        .get("llm")
        .cloned()
        .unwrap_or_else(|| state.config.default_model.clone());
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
        state
            .users
            .read()
            .await
            .get(&uid)
            .map(|u| u.role == auth_user::UserRole::Admin)
            .unwrap_or(false)
    } else {
        false
    };

    let logs = state.activity_log.read().await;
    let filtered: Vec<ActivityLogEntry> = logs
        .iter()
        .filter(|entry| {
            if is_admin {
                return true;
            }
            // System logs (no user_id) — visible to admin only, hide from regular users
            // User's own logs — visible to that user
            match (&entry.user_id, &caller) {
                (None, _) => entry.tag != "agent", // show system logs (heartbeat, model) but not other users' agent chats
                (Some(log_uid), Some(caller_uid)) => log_uid == caller_uid,
                (Some(_), None) => false, // not authenticated
            }
        })
        .cloned()
        .collect();
    Json(ActivityResponse { logs: filtered })
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok" }))
}

/// GET /api/voice/status — check STT/TTS service availability.
// --- P6: Custom Services Register ---
#[derive(Deserialize)]
pub struct RegisterServiceReq {
    pub name: String,
    pub capability: String,
    pub url: String,
    pub protocol: String,
}

async fn api_register_service(
    State(state): State<Arc<AppState>>,
    axum::extract::Json(req): axum::extract::Json<RegisterServiceReq>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if req.name.is_empty() || req.url.is_empty() || req.capability.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    let mut custom = state.custom_services.write().await;
    custom.insert(
        req.name.clone(),
        CustomService {
            name: req.name.clone(),
            capability: req.capability.clone(),
            url: req.url.clone(),
            protocol: req.protocol.clone(),
        },
    );

    // P4 periodic loop will pick this up for mDNS if public_proxy (or auto-announce)
    Ok(Json(serde_json::json!({ "success": true })))
}

async fn api_unregister_service(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut custom = state.custom_services.write().await;
    if custom.remove(&name).is_some() {
        Ok(Json(serde_json::json!({ "success": true })))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

// --- P3: OpenAI-compatible /v1/chat/completions ---
#[derive(Deserialize)]
pub struct OpenAiChatReq {
    pub model: String,
    pub messages: Vec<serde_json::Value>,
    #[serde(default)]
    pub stream: bool,
    #[serde(default)]
    pub temperature: Option<f32>,
    #[serde(default)]
    pub max_tokens: Option<u32>,
}

/// Récupère (avec cache) la clé publique d'un pair via son /api/mesh/identity. Vérifie que le
/// node_id annoncé par l'IP correspond bien à celui déclaré (X-Miel-From).
async fn peer_pubkey(node_id: &str, ip: &str) -> Option<String> {
    use std::sync::{Mutex, OnceLock};
    static CACHE: OnceLock<Mutex<std::collections::HashMap<String, String>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(std::collections::HashMap::new()));
    if let Some(pk) = cache.lock().unwrap().get(node_id) {
        return Some(pk.clone());
    }
    let url = format!("http://{ip}:8419/api/mesh/identity");
    let resp: serde_json::Value = reqwest::Client::new()
        .get(&url)
        .timeout(std::time::Duration::from_secs(3))
        .send()
        .await
        .ok()?
        .json()
        .await
        .ok()?;
    let nid = resp.get("node_id").and_then(|v| v.as_str())?;
    let pk = resp.get("pubkey").and_then(|v| v.as_str())?.to_string();
    if nid != node_id {
        return None; // l'IP ne correspond pas au node_id déclaré → suspect
    }
    cache.lock().unwrap().insert(node_id.to_string(), pk.clone());
    Some(pk)
}

/// node_id VÉRIFIÉ (signature ed25519) de l'appelant d'une requête d'inférence, ou None.
async fn verified_inference_caller(
    headers: &axum::http::HeaderMap,
    addr: &std::net::SocketAddr,
) -> Option<String> {
    let from = headers.get("X-Miel-From").and_then(|v| v.to_str().ok())?.to_string();
    let pubkey = peer_pubkey(&from, &addr.ip().to_string()).await?;
    sync::verified_caller(headers, "/v1/chat/completions", &pubkey)
}

async fn api_v1_chat_completions(
    State(state): State<Arc<AppState>>,
    axum::extract::ConnectInfo(addr): axum::extract::ConnectInfo<std::net::SocketAddr>,
    headers: axum::http::HeaderMap,
    axum::extract::Json(req): axum::extract::Json<OpenAiChatReq>,
) -> axum::response::Response {
    use axum::response::IntoResponse;
    use futures_util::StreamExt;

    // 1. Resolve provider based on req.model or active_model
    let mut provider_id = "ollama".to_string();
    let mut api_key = "".to_string();
    let mut api_base = None;
    let ollama_url = state.config.ollama_url.clone();
    let mut vis = profiles::Visibilite::Prive;
    let mut allowed: Vec<String> = Vec::new();

    {
        let profiles = state.profiles.read().await;
        let mut found = false;
        for (_pid, profile) in &profiles.profiles {
            if profile.models.contains(&req.model) {
                provider_id = profile.provider.clone();
                api_key = profile.api_key.clone();
                api_base = Some(profile.base_url.clone());
                vis = profile.visibilite;
                allowed = profile.allowed_peers.clone();
                found = true;
                break;
            }
        }
        if !found {
            // fallback to active model provider if it matches
            let active = &profiles.active_model;
            if let Some(p) = profiles.profiles.get(&active.profile_id) {
                provider_id = p.provider.clone();
                api_key = p.api_key.clone();
                api_base = Some(p.base_url.clone());
                vis = p.visibilite;
                allowed = p.allowed_peers.clone();
            }
        }
    }

    // ENFORCEMENT mesh : un appelant DISTANT (non-loopback) ne peut utiliser ce modèle que selon
    // sa visibilité. Le node local (loopback) n'est jamais bloqué.
    if !addr.ip().is_loopback() {
        let refus = |msg: &str| {
            (
                axum::http::StatusCode::FORBIDDEN,
                axum::Json(serde_json::json!({ "error": { "message": msg, "type": "forbidden" } })),
            )
                .into_response()
        };
        match vis {
            profiles::Visibilite::Prive => {
                return refus("Modèle privé : non partagé sur le mesh.");
            }
            profiles::Visibilite::Restricted => {
                match verified_inference_caller(&headers, &addr).await {
                    Some(nid) if allowed.iter().any(|a| a == &nid) => {} // autorisé
                    Some(_) => return refus("Ruche non autorisée pour ce modèle (restricted)."),
                    None => return refus("Identité mesh requise/invalide pour un modèle restricted."),
                }
            }
            profiles::Visibilite::PublicProxy => {} // public : tout membre du mesh
        }
    }

    let temp = req.temperature.unwrap_or(0.7);
    let max_t = req.max_tokens.unwrap_or(2048);

    match laruche_essaim::providers::provider_chat_stream(
        &provider_id,
        &req.model,
        &req.messages,
        temp,
        max_t,
        &api_key,
        api_base.as_deref(),
        &ollama_url,
    )
    .await
    {
        Ok(mut stream) => {
            if req.stream {
                let model_name = req.model.clone();
                let stream_res = stream.map(move |chunk| {
                    let json_str = serde_json::json!({
                        "id": "chatcmpl-mesh",
                        "object": "chat.completion.chunk",
                        "created": 0,
                        "model": model_name.clone(),
                        "choices": [{
                            "index": 0,
                            "delta": {
                                "content": chunk.text
                            },
                            "finish_reason": if chunk.done { Some("stop") } else { None::<&str> }
                        }]
                    })
                    .to_string();

                    let mut out = format!("data: {}\n\n", json_str);
                    if chunk.done {
                        out.push_str("data: [DONE]\n\n");
                    }
                    Ok::<_, std::convert::Infallible>(axum::body::Bytes::from(out))
                });
                axum::response::Response::builder()
                    .header("Content-Type", "text/event-stream")
                    .header("Cache-Control", "no-cache")
                    .body(axum::body::Body::from_stream(stream_res))
                    .unwrap()
            } else {
                let mut full_text = String::new();
                while let Some(chunk) = stream.next().await {
                    full_text.push_str(&chunk.text);
                }
                let res = serde_json::json!({
                    "id": "chatcmpl-mesh",
                    "object": "chat.completion",
                    "created": 0,
                    "model": req.model,
                    "choices": [{
                        "index": 0,
                        "message": {
                            "role": "assistant",
                            "content": full_text
                        },
                        "finish_reason": "stop"
                    }]
                });
                axum::Json(res).into_response()
            }
        }
        Err(e) => {
            let res = serde_json::json!({
                "error": { "message": e.to_string(), "type": "server_error" }
            });
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(res),
            )
                .into_response()
        }
    }
}

async fn api_voice_status(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let listener = state.listener.read().await;
    let nodes = listener.get_nodes().await;

    // Service explicitement choisi par capacité (sinon : premier trouvé, comportement d'avant).
    let (want_stt, want_tts, stt_model, tts_model) = {
        let sel = state.capability_selection.read().await;
        (
            sel.get("stt").and_then(|s| s.node_id.clone()),
            sel.get("tts").and_then(|s| s.node_id.clone()),
            sel.get("stt").map(|s| s.model.clone()).unwrap_or_default(),
            sel.get("tts").map(|s| s.model.clone()).unwrap_or_default(),
        )
    };

    let mut stt_available = false;
    let mut tts_available = false;
    let mut stt_url = String::new();
    let mut tts_url = String::new();
    let mut stt_locked = false; // url verrouillée par la sélection utilisateur
    let mut tts_locked = false;

    for (_id, node) in &nodes {
        let nid = node.manifest.node_id.map(|x| x.to_string());
        let caps: Vec<String> = node
            .manifest
            .capabilities
            .iter()
            .map(|c| c.to_string())
            .collect();
        let url = node
            .manifest
            .port
            .map(|p| format!("http://{}:{}", node.manifest.host, p));

        if caps.iter().any(|c| c == "stt") {
            stt_available = true;
            if want_stt.is_some() && nid == want_stt {
                if let Some(u) = &url {
                    stt_url = u.clone();
                }
                stt_locked = true;
            } else if !stt_locked && stt_url.is_empty() {
                if let Some(u) = &url {
                    stt_url = u.clone();
                }
            }
        }
        if caps.iter().any(|c| c == "tts") {
            tts_available = true;
            if want_tts.is_some() && nid == want_tts {
                if let Some(u) = &url {
                    tts_url = u.clone();
                }
                tts_locked = true;
            } else if !tts_locked && tts_url.is_empty() {
                if let Some(u) = &url {
                    tts_url = u.clone();
                }
            }
        }
    }

    Json(serde_json::json!({
        "stt": { "available": stt_available, "url": stt_url, "selected_model": stt_model, "is_selected": stt_locked },
        "tts": { "available": tts_available, "url": tts_url, "selected_model": tts_model, "is_selected": tts_locked },
    }))
}

async fn spa_page() -> Html<&'static str> {
    Html(SPA_HTML)
}

async fn api_list_tools(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let disabled = state.essaim_config.read().await.disabled_tools.clone();
    let tools = match state.essaim_registry.schema_complet() {
        serde_json::Value::Array(items) => items
            .into_iter()
            .map(|mut tool| {
                if let Some(name) = tool
                    .get("name")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string)
                {
                    let enabled = !disabled.iter().any(|t| t == &name);
                    if let Some(obj) = tool.as_object_mut() {
                        obj.insert("enabled".to_string(), serde_json::json!(enabled));
                        if let Some(abeille) = state.essaim_registry.get(&name) {
                            obj.insert(
                                "danger".to_string(),
                                serde_json::to_value(abeille.niveau_danger())
                                    .unwrap_or_else(|_| serde_json::json!("safe")),
                            );
                            obj.insert(
                                "origin".to_string(),
                                serde_json::to_value(abeille.origin())
                                    .unwrap_or_else(|_| serde_json::json!("builtin")),
                            );
                        }
                    }
                }
                tool
            })
            .collect(),
        _ => Vec::new(),
    };
    Json(serde_json::Value::Array(tools))
}

/// GET/POST /api/tools/config — enable/disable Abeilles for prompt injection/execution.
async fn api_get_tools_config(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let disabled = state.essaim_config.read().await.disabled_tools.clone();
    Json(serde_json::json!({ "disabled_tools": disabled }))
}

async fn api_save_tools_config(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let disabled = body["disabled_tools"]
        .as_array()
        .ok_or(StatusCode::BAD_REQUEST)?
        .iter()
        .filter_map(|v| v.as_str().map(str::trim))
        .filter(|name| !name.is_empty() && state.essaim_registry.get(name).is_some())
        .map(str::to_string)
        .collect::<Vec<_>>();

    {
        let mut cfg = state.essaim_config.write().await;
        cfg.disabled_tools = disabled.clone();
    }
    save_persistent_state(&state).await;
    Ok(Json(
        serde_json::json!({ "status": "ok", "disabled_tools": disabled }),
    ))
}

/// GET /api/memory/search?q=...&limit=8 — search cognitive memory.
async fn api_memory_search(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let query = params.get("q").map(String::as_str).unwrap_or("").trim();
    if query.is_empty() {
        return Ok(Json(serde_json::json!({
            "query": query,
            "raw": { "nodes": [], "items": [] },
            "prompt_text": ""
        })));
    }
    let limit = params
        .get("limit")
        .and_then(|v| v.parse::<u8>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(8);
    match state
        .memoire
        .search(
            query,
            laruche_memoire::SearchOpts {
                depth: None,
                limit: Some(limit),
            },
        )
        .await
    {
        Ok(pack) => {
            let prompt_text = pack.to_prompt_text();
            Ok(Json(serde_json::json!({
                "query": query,
                "raw": pack.raw,
                "prompt_text": prompt_text
            })))
        }
        Err(e) => Ok(Json(serde_json::json!({ "error": e.to_string() }))),
    }
}

/// POST /api/memory/write — write a durable memory item.
async fn api_memory_write(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let node_id = body["node_id"]
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or(StatusCode::BAD_REQUEST)?;
    let content = body["content"]
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or(StatusCode::BAD_REQUEST)?;
    let propose = body["propose"].as_bool().unwrap_or(false);

    let mut item = laruche_memoire::MemoryItem::new(node_id, content);
    if let Some(source) = body["source"]
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        item = item.with_source(source);
    }
    if let Some(tags) = body["tags"].as_array() {
        let tags = tags
            .iter()
            .filter_map(|v| v.as_str().map(str::trim))
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect();
        item = item.with_tags(tags);
    }

    let result = if propose {
        state.memoire.propose_write(item).await
    } else {
        state.memoire.write(item).await
    };
    match result {
        Ok(value) => {
            let _ = state.events.write().await.emit(
                laruche_events::EventKind::MemorySaved,
                "api_memory",
                &serde_json::json!({ "node_id": node_id, "content": content, "propose": propose }),
            );
            Ok(Json(serde_json::json!({ "status": "ok", "result": value })))
        }
        Err(e) => Ok(Json(
            serde_json::json!({ "status": "error", "error": e.to_string() }),
        )),
    }
}

/// POST /api/memory/enrich - Spawn an agent to enrich a node
async fn api_memory_enrich(
    State(state): State<Arc<AppState>>,
    _headers: axum::http::HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let node_id = body["node_id"]
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or(StatusCode::BAD_REQUEST)?
        .to_string();
    let prompt = body["prompt"]
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or(StatusCode::BAD_REQUEST)?
        .to_string();
    let item_id = body["item_id"].as_str().map(|s| s.to_string());

    let mut config = state.essaim_config.read().await.clone();
    if let Some(review_model) = &config.review_model {
        if !review_model.trim().is_empty() {
            config.model = review_model.clone();
        }
    }

    let agent_id = uuid::Uuid::new_v4();
    let registry = state.essaim_registry.clone();
    let state_clone = state.clone();

    let task = format!(
        "Tu dois enrichir le noeud cognitif '{}'.\nVoici la requête de l'utilisateur : '{}'.\nLis le noeud avec 'memory_read_node', effectue les recherches nécessaires, puis utilise 'memory_write' pour ajouter tes trouvailles dans ce noeud.",
        node_id, prompt
    );
    let context = Some(node_id.to_string());

    tokio::spawn(async move {
        tracing::info!(agent_id = %agent_id, task = %task, "Subagent spawned for memory enrichment");
        let _ = state_clone.events.write().await.emit(
            laruche_events::EventKind::AgentStarted,
            "api_memory_enrich",
            serde_json::json!({ "agent_id": agent_id, "node_id": node_id, "item_id": item_id }),
        );

        match laruche_essaim::subagent::lancer_sous_agent(
            &task,
            context.as_deref(),
            registry,
            &config,
        )
        .await
        {
            Ok(result) => {
                tracing::info!(agent_id = %agent_id, "Memory enrichment agent finished");
                if let Some(id) = item_id {
                    let new_content =
                        format!("{}\n\n**Synthèse LaRuche :**\n{}", prompt, result.summary);
                    let _ = state_clone.memoire.update_item(&id, &new_content).await;
                    let _ = state_clone.events.write().await.emit(
                        laruche_events::EventKind::AgentFinished,
                        "api_memory_enrich",
                        serde_json::json!({ "agent_id": agent_id, "item_id": id, "status": "ok" }),
                    );
                }
            }
            Err(e) => {
                tracing::error!(agent_id = %agent_id, error = %e, "Memory enrichment agent failed");
                if let Some(id) = item_id {
                    let new_content = format!("{}\n\n**Erreur LaRuche :**\n{}", prompt, e);
                    let _ = state_clone.memoire.update_item(&id, &new_content).await;
                    let _ = state_clone.events.write().await.emit(
                        laruche_events::EventKind::AgentFinished,
                        "api_memory_enrich",
                        serde_json::json!({ "agent_id": agent_id, "item_id": id, "status": "error" }),
                    );
                }
            }
        }
    });

    Ok(Json(
        serde_json::json!({ "status": "ok", "agent_id": agent_id }),
    ))
}

/// GET /api/memory/node/:id — read a cognitive-map node with children and active items.
async fn api_memory_node(
    State(state): State<Arc<AppState>>,
    Path(node_id): Path<String>,
) -> Json<serde_json::Value> {
    match state.memoire.read_node(&node_id).await {
        Ok(value) => Json(serde_json::json!({ "status": "ok", "node": value })),
        Err(e) => Json(serde_json::json!({ "status": "error", "error": e.to_string() })),
    }
}

async fn api_memory_update(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let item_id = body["item_id"]
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or(StatusCode::BAD_REQUEST)?;
    let content = body["content"]
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or(StatusCode::BAD_REQUEST)?;
    match state.memoire.update_item(item_id, content).await {
        Ok(value) => Ok(Json(serde_json::json!({ "status": "ok", "result": value }))),
        Err(e) => Ok(Json(
            serde_json::json!({ "status": "error", "error": e.to_string() }),
        )),
    }
}

async fn api_memory_delete(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let item_id = body["item_id"]
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or(StatusCode::BAD_REQUEST)?;
    let reason = body["reason"].as_str();
    match state.memoire.delete_item(item_id, reason).await {
        Ok(value) => Ok(Json(serde_json::json!({ "status": "ok", "result": value }))),
        Err(e) => Ok(Json(
            serde_json::json!({ "status": "error", "error": e.to_string() }),
        )),
    }
}

async fn api_memory_node_delete(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let node_id = body["node_id"]
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or(StatusCode::BAD_REQUEST)?;
    match state.memoire.delete_node(node_id).await {
        Ok(value) => Ok(Json(serde_json::json!({ "status": "ok", "result": value }))),
        Err(e) => Ok(Json(
            serde_json::json!({ "status": "error", "error": e.to_string() }),
        )),
    }
}

async fn api_memory_node_create(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let node_id = body["node_id"].as_str().unwrap_or("");
    let label = body["label"].as_str().unwrap_or("");
    let one_liner = body["one_liner"].as_str();
    let importance = body["importance"].as_f64().map(|f| f as f32);
    let source = body["source"].as_str();
    if node_id.is_empty() || label.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    match state
        .memoire
        .create_node(node_id, label, one_liner, importance, source)
        .await
    {
        Ok(value) => Ok(Json(serde_json::json!({ "status": "ok", "result": value }))),
        Err(e) => Ok(Json(
            serde_json::json!({ "status": "error", "error": e.to_string() }),
        )),
    }
}

async fn api_memory_node_update(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let node_id = body["node_id"].as_str().unwrap_or("");
    if node_id.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    let label = body["label"].as_str();
    let one_liner = body["one_liner"].as_str();
    let importance = body["importance"].as_f64().map(|f| f as f32);

    match state
        .memoire
        .update_node(node_id, label, one_liner, importance)
        .await
    {
        Ok(value) => Ok(Json(serde_json::json!({ "status": "ok", "result": value }))),
        Err(e) => Ok(Json(
            serde_json::json!({ "status": "error", "error": e.to_string() }),
        )),
    }
}

/// POST /api/memory/node/move — reparente un nœud (drag&drop dans l'arbre). body
/// `{node_id, new_parent}` ; `new_parent` vide ⇒ nœud racine. Déplace tout le sous-arbre
/// (renommage d'id). Refuse les nœuds système et les cycles (déplacer dans son propre sous-arbre).
async fn api_memory_node_move(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let old = body["node_id"]
        .as_str()
        .map(|s| s.trim().trim_matches('.'))
        .filter(|s| !s.is_empty())
        .ok_or(StatusCode::BAD_REQUEST)?;
    let new_parent = body["new_parent"]
        .as_str()
        .unwrap_or("")
        .trim()
        .trim_matches('.');
    let last = old.rsplit('.').next().unwrap_or(old);
    let new_id = if new_parent.is_empty() {
        last.to_string()
    } else {
        format!("{new_parent}.{last}")
    };
    let prot = |s: &str| {
        s == "system"
            || s == "capacities"
            || s.starts_with("system.")
            || s.starts_with("capacities.")
    };
    if prot(old) || prot(&new_id) {
        return Ok(Json(
            serde_json::json!({ "status": "error", "error": "noeud systeme non deplacable" }),
        ));
    }
    if new_id == old || new_id.starts_with(&format!("{old}.")) {
        return Ok(Json(
            serde_json::json!({ "status": "error", "error": "deplacement invalide (cycle ou identique)" }),
        ));
    }
    match state.memoire.renommer_sous_arbre(old, &new_id).await {
        Ok(n) => Ok(Json(
            serde_json::json!({ "status": "ok", "result": { "moved_to": new_id, "nodes": n } }),
        )),
        Err(e) => Ok(Json(
            serde_json::json!({ "status": "error", "error": e.to_string() }),
        )),
    }
}

async fn api_memory_move(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let item_id = body["item_id"]
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or(StatusCode::BAD_REQUEST)?;
    let node_id = body["node_id"]
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or(StatusCode::BAD_REQUEST)?;
    match state.memoire.move_item(item_id, node_id).await {
        Ok(value) => Ok(Json(serde_json::json!({ "status": "ok", "result": value }))),
        Err(e) => Ok(Json(
            serde_json::json!({ "status": "error", "error": e.to_string() }),
        )),
    }
}

async fn api_memory_review(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let item_id = body["item_id"]
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or(StatusCode::BAD_REQUEST)?;
    let action = body["action"]
        .as_str()
        .map(str::trim)
        .filter(|s| matches!(*s, "accept" | "reject"))
        .ok_or(StatusCode::BAD_REQUEST)?;
    let reason = body["reason"].as_str();
    match state.memoire.review_item(item_id, action, reason).await {
        Ok(value) => {
            let _ = state.events.write().await.emit(
                laruche_events::EventKind::MemoryReviewed,
                "api_memory",
                &serde_json::json!({ "item_id": item_id, "action": action }),
            );
            Ok(Json(serde_json::json!({ "status": "ok", "result": value })))
        }
        Err(e) => Ok(Json(
            serde_json::json!({ "status": "error", "error": e.to_string() }),
        )),
    }
}

async fn api_memory_proposed(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let limit = params
        .get("limit")
        .and_then(|v| v.parse::<u8>().ok())
        .filter(|v| *v > 0);
    match state.memoire.list_proposed(limit).await {
        Ok(value) => Json(serde_json::json!({ "status": "ok", "result": value })),
        Err(e) => Json(serde_json::json!({ "status": "error", "error": e.to_string() })),
    }
}

async fn api_memory_suggest(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let query = params.get("q").map(String::as_str).unwrap_or("").trim();
    let limit = params
        .get("limit")
        .and_then(|v| v.parse::<u8>().ok())
        .filter(|v| *v > 0);
    match state.memoire.suggest_nodes(query, limit).await {
        Ok(value) => Json(serde_json::json!({ "status": "ok", "result": value })),
        Err(e) => Json(serde_json::json!({ "status": "error", "error": e.to_string() })),
    }
}

/// POST /api/memory/dream — trigger active memory consolidation.
async fn api_memory_dream(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let dream = state
        .memoire
        .dream()
        .await
        .unwrap_or_else(|e| serde_json::json!({ "error": e.to_string() }));
    Json(dream)
}

/// POST /api/memory/consolidate?node=<id> — fusionne RÉELLEMENT les items (via le modèle aux).
/// Avec `node` : consolide ce nœud. Sans : passe sur les nœuds surchargés (≥4 items). Les anciens
/// items sont soft-deleted (récupérables). C'est ce que le bouton « Consolider » déclenche.
async fn api_memory_consolidate(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(q): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let config = state.essaim_config.read().await.clone();
    let node = q.get("node").map(|s| s.as_str()).filter(|s| !s.is_empty());
    let res = match node {
        Some(n) => laruche_essaim::brain::consolider_node(&state.memoire, &config, n).await,
        None => laruche_essaim::brain::consolider_memoire(&state.memoire, &config).await,
    };
    Json(res.unwrap_or_else(|e| serde_json::json!({ "error": e.to_string() })))
}

/// GET /api/memory/grep?q=<texte>&limit=30 — recherche par sous-chaîne dans le contenu des items.
async fn api_memory_grep(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(q): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let pattern = q.get("q").cloned().unwrap_or_default();
    let limit = q.get("limit").and_then(|s| s.parse::<u8>().ok());
    Json(
        state
            .memoire
            .grep(&pattern, limit)
            .await
            .unwrap_or_else(|e| serde_json::json!({ "error": e.to_string() })),
    )
}

/// Phase 1 — sync DISQUE → SQL : scanne `skills/*/SKILL.md` et upsert chaque skill dans
/// `capacities.skills.<slug>` (item unique). Additif (ne supprime pas les skills SQL-only).
async fn sync_skills_disk_to_sql(memoire: &Arc<dyn laruche_memoire::MemoireCognitive>) {
    let dir = std::path::Path::new("skills");
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    let mut n = 0usize;
    for e in rd.flatten() {
        let p = e.path();
        if !p.is_dir() {
            continue;
        }
        let Ok(content) = std::fs::read_to_string(p.join("SKILL.md")) else {
            continue;
        };
        let content = content.replace("\r\n", "\n"); // normalise (SQL en LF)
        if !content.contains("type: skill") {
            continue; // seulement les vrais skills OKF
        }
        let Some(slug) = p.file_name().and_then(|x| x.to_str()).filter(|s| !s.is_empty()) else {
            continue;
        };
        let node_id = format!("capacities.skills.{slug}");
        // Remplace l'item existant (skill = item unique).
        if let Ok(node) = memoire.read_node(&node_id).await {
            if let Some(items) = node.get("items").and_then(|i| i.as_array()) {
                for it in items {
                    if let Some(id) = it.get("id").and_then(|x| x.as_str()) {
                        let _ = memoire.delete_item(id, Some("skill-file-sync")).await;
                    }
                }
            }
        }
        let _ = memoire
            .write(
                laruche_memoire::MemoryItem::new(node_id, content).with_source("skill-file"),
            )
            .await;
        n += 1;
    }
    if n > 0 {
        tracing::info!(count = n, "skills synchronises depuis disque (SKILL.md -> SQL)");
    }
    // Purge ciblée des MÉTA-SKILLS d'autres frameworks d'agents (third-party/Claude Code/Codex…),
    // importés à tort : ils décrivent un AUTRE agent, pas LaRuche. DENYLIST explicite — surtout
    // PAS un diff disque « supprime tout ce qui n'est pas sur disque » (ça détruirait les skills
    // créés par l'agent ou seedés en code, comme arxiv_search / web_research). Hard-delete :
    // delete_node reparente vers `orphans.*`, donc on supprime aussi l'orphelin résultant.
    const META_SKILLS_A_PURGER: &[&str] = &[
        "third-party agent",
        "third-party agent-skill-authoring",
        "claude-code",
        "codex",
        "opencode",
    ];
    let mut purges = 0usize;
    for slug in META_SKILLS_A_PURGER {
        let node_id = format!("capacities.skills.{slug}");
        if memoire.read_node(&node_id).await.is_err() {
            continue; // absent → rien à faire
        }
        if let Ok(r) = memoire.delete_node(&node_id).await {
            purges += 1;
            // delete_node a déplacé vers orphans.<base>_<ts> → hard-delete cet orphelin.
            if let Some(orphan) = r.get("relocated_to").and_then(|v| v.as_str()) {
                let _ = memoire.delete_node(orphan).await;
            }
        }
    }
    if purges > 0 {
        tracing::info!(count = purges, "meta-skills d'autres frameworks purges (denylist)");
    }
}

/// Importe une liste de faits `{node_id, content}` dans la mémoire (dédup exact). (imported, skipped).
async fn importer_changes(
    state: &Arc<AppState>,
    items: &[serde_json::Value],
    src: &str,
) -> (usize, usize) {
    let (mut imported, mut skipped) = (0usize, 0usize);
    for it in items {
        let node = it["node_id"].as_str().unwrap_or("").trim();
        let content = it["content"].as_str().unwrap_or("");
        if node.is_empty() || content.trim().is_empty() {
            continue;
        }
        // Dédup exact : si un item identique existe déjà dans ce nœud, on saute.
        let exists = state
            .memoire
            .grep(content, Some(8))
            .await
            .ok()
            .and_then(|g| {
                g["items"].as_array().map(|a| {
                    a.iter().any(|x| {
                        x["node_id"].as_str() == Some(node) && x["content"].as_str() == Some(content)
                    })
                })
            })
            .unwrap_or(false);
        if exists {
            skipped += 1;
            continue;
        }
        let _ = state
            .memoire
            .write(
                laruche_memoire::MemoryItem::new(node.to_string(), content.to_string())
                    .with_source(src),
            )
            .await;
        imported += 1;
    }
    (imported, skipped)
}

/// GET /api/memory/export_changes?since=<ts> — faits (op=write) écrits depuis `since`, pour la
/// fédération mesh (Levier 3, 1re tranche). Exclut projections système/capacities.
async fn api_memory_export_changes(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(q): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let since = q.get("since").and_then(|s| s.parse::<i64>().ok()).unwrap_or(0);
    let muts = state
        .memoire
        .mutations(Some(250))
        .await
        .unwrap_or_else(|_| serde_json::json!({}));
    let items: Vec<serde_json::Value> = muts["mutations"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter(|m| {
                    m["op"].as_str() == Some("write")
                        && m["ts"].as_i64().unwrap_or(0) > since
                        && {
                            let n = m["node_id"].as_str().unwrap_or("");
                            !n.starts_with("capacities") && !n.starts_with("system")
                        }
                })
                .map(|m| serde_json::json!({ "node_id": m["node_id"], "content": m["content"], "ts": m["ts"] }))
                .collect()
        })
        .unwrap_or_default();
    let count = items.len();
    Json(serde_json::json!({ "items": items, "count": count }))
}

/// POST /api/memory/import_changes {items:[{node_id,content}], source?} — applique des faits (dédup).
async fn api_memory_import_changes(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let src = body["source"].as_str().unwrap_or("mesh").to_string();
    let empty: Vec<serde_json::Value> = vec![];
    let items = body["items"].as_array().unwrap_or(&empty);
    let (imported, skipped) = importer_changes(&state, items, &src).await;
    Json(serde_json::json!({ "imported": imported, "skipped": skipped }))
}

/// POST /api/memory/mesh_pull {peer, since?} — tire les faits d'un node PAIR (Miel) et les importe
/// localement. Première brique de la mémoire COLLECTIVE du mesh (Levier 3).
async fn api_memory_mesh_pull(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let peer = body["peer"]
        .as_str()
        .unwrap_or("")
        .trim()
        .trim_end_matches('/')
        .to_string();
    if peer.is_empty() {
        return Json(serde_json::json!({ "error": "peer manquant (ex. http://192.168.1.20:8419)" }));
    }
    let since = body["since"].as_i64().unwrap_or(0);
    let url = format!("{peer}/api/memory/export_changes?since={since}");
    let data: serde_json::Value = match reqwest::get(&url).await {
        Ok(r) => match r.json().await {
            Ok(v) => v,
            Err(e) => return Json(serde_json::json!({ "error": format!("json pair: {e}") })),
        },
        Err(e) => return Json(serde_json::json!({ "error": format!("contact pair: {e}") })),
    };
    let empty: Vec<serde_json::Value> = vec![];
    let items = data["items"].as_array().unwrap_or(&empty);
    let src = format!("mesh:{peer}");
    let (imported, skipped) = importer_changes(&state, items, &src).await;
    Json(serde_json::json!({ "pulled_from": peer, "imported": imported, "skipped": skipped }))
}

/// GET /api/state/version — ts de la dernière mutation mémoire (P7 lite : l'UI poll pour savoir
/// si rafraîchir, sans canal push).
async fn api_state_version(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let v = state
        .memoire
        .mutations(Some(1))
        .await
        .ok()
        .and_then(|m| {
            m["mutations"]
                .as_array()
                .and_then(|a| a.first())
                .and_then(|x| x["ts"].as_i64())
        })
        .unwrap_or(0);
    Json(serde_json::json!({ "version": v }))
}

/// Acteur d'une mutation mémoire d'après son `src` (source/raison). UI → User, sinon LaRuche.
fn feed_actor(src: &str) -> &'static str {
    let s = src.trim().to_lowercase();
    if s.starts_with("ui") || s == "user" || s == "fabien" || s == "admin" {
        "User"
    } else {
        "LaRuche"
    }
}

/// Nettoie une réponse d'agent pour le Feed : retire les blocs protocole (`<plan>`, `<tool_call>`,
/// `<think>`) — complets ou tronqués — et normalise les espaces. Sinon le Feed affiche du JSON/XML
/// illisible pour un humain.
fn nettoyer_reponse_feed(s: &str) -> String {
    let mut out = s.to_string();
    for (open, close) in [
        ("<plan>", "</plan>"),
        ("<tool_call>", "</tool_call>"),
        ("<think>", "</think>"),
    ] {
        loop {
            let Some(i) = out.find(open) else { break };
            match out[i..].find(close) {
                Some(j_rel) => {
                    let j = i + j_rel + close.len();
                    out.replace_range(i..j, " ");
                }
                None => out.truncate(i), // tag ouvrant sans fermant → coupe la queue
            }
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

// ===================== Phase 4 — Messagerie mesh (DM inter-instances/users) =====================
#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct InboxMessage {
    id: String,
    peer_id: String,
    peer_name: String,
    dir: String, // "in" (reçu) | "out" (envoyé)
    text: String,
    ts: i64,
    read: bool,
}
fn inbox_path() -> std::path::PathBuf {
    std::path::PathBuf::from("inbox.json")
}
fn read_inbox() -> Vec<InboxMessage> {
    std::fs::read_to_string(inbox_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}
fn write_inbox(msgs: &[InboxMessage]) {
    if let Ok(s) = serde_json::to_string_pretty(msgs) {
        let _ = std::fs::write(inbox_path(), s);
    }
}
fn append_inbox(m: InboxMessage) {
    let mut v = read_inbox();
    v.push(m);
    if v.len() > 1000 {
        let drop_n = v.len() - 1000;
        v.drain(0..drop_n);
    }
    write_inbox(&v);
}

/// GET /api/mesh/code — indique si un code de mesh est configuré (jamais le secret lui-même).
async fn api_mesh_code_get() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "set": sync::load_mesh_code().is_some() }))
}
/// POST /api/mesh/code {code} — définit/efface le code de mesh partagé (auth + base de chiffrement).
async fn api_mesh_code_set(Json(body): Json<serde_json::Value>) -> Json<serde_json::Value> {
    let code = body["code"].as_str().unwrap_or("");
    sync::save_mesh_code(code);
    Json(serde_json::json!({ "status": "ok", "set": !code.trim().is_empty() }))
}

/// GET /api/mesh/identity — node_id + clé PUBLIQUE ed25519 (hex) de ce nœud. Les pairs la
/// récupèrent et la mettent en cache pour vérifier les signatures (identité forte, `restricted`).
async fn api_mesh_identity() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "node_id": sync::my_node_id(), "pubkey": sync::my_pubkey_hex() }))
}

/// GET /api/mesh/whoami — identité de CETTE instance (ID laruche + nom).
async fn api_mesh_whoami(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let m = state.manifest.read().await;
    Json(serde_json::json!({ "id": m.node_id.to_string(), "name": m.node_name }))
}

/// GET /api/mesh/peers — autres instances LaRuche découvertes sur le réseau (annuaire).
async fn api_mesh_peers(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let listener = state.listener.read().await;
    let nodes = listener.get_nodes().await;
    let m = state.manifest.read().await;
    let peers: Vec<serde_json::Value> = nodes
        .values()
        .filter(|n| {
            n.manifest.node_id != Some(m.node_id) && n.manifest.host != m.api_endpoint.host
        })
        .filter_map(|n| {
            n.manifest.node_id.map(|id| {
                serde_json::json!({
                    "id": id.to_string(),
                    "name": n.manifest.node_name,
                    "host": n.manifest.host,
                })
            })
        })
        .collect();
    Json(serde_json::json!({ "peers": peers }))
}

/// POST /api/mesh/send {to_id, text} — envoie un DM à un pair (résout l'hôte par ID, POST sur son
/// /api/mesh/receive). Garde une copie locale (dir=out) pour le fil de conversation.
async fn api_mesh_send(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let to_id = body["to_id"].as_str().unwrap_or("").to_string();
    let text = body["text"].as_str().unwrap_or("").trim().to_string();
    if to_id.is_empty() || text.is_empty() {
        return Json(serde_json::json!({ "status": "error", "error": "to_id/text requis" }));
    }
    let (host, peer_name) = {
        let listener = state.listener.read().await;
        let nodes = listener.get_nodes().await;
        nodes
            .values()
            .find(|n| n.manifest.node_id.map(|i| i.to_string()).as_deref() == Some(to_id.as_str()))
            .map(|n| (n.manifest.host.clone(), n.manifest.node_name.clone().unwrap_or_default()))
            .unwrap_or((String::new(), to_id.clone()))
    };
    if host.is_empty() {
        return Json(serde_json::json!({ "status": "error", "error": "pair introuvable" }));
    }
    let (my_id, my_name) = {
        let m = state.manifest.read().await;
        (m.node_id.to_string(), m.node_name.clone())
    };
    let client = reqwest::Client::new();
    let url = format!("http://{host}:8419/api/mesh/receive");
    // Chiffre le contenu si un code de mesh est configuré (sinon clair, rétro-compatible).
    let payload = match sync::seal(&text) {
        Some(enc) => serde_json::json!({ "from_id": my_id, "from_name": my_name, "enc": enc }),
        None => serde_json::json!({ "from_id": my_id, "from_name": my_name, "text": text }),
    };
    let ok = sync::sign_request(client.post(&url), "/api/mesh/receive")
        .json(&payload)
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false);
    append_inbox(InboxMessage {
        id: Uuid::new_v4().to_string(),
        peer_id: to_id,
        peer_name,
        dir: "out".into(),
        text,
        ts: chrono::Utc::now().timestamp(),
        read: true,
    });
    Json(serde_json::json!({ "status": if ok { "ok" } else { "local_only" } }))
}

/// POST /api/mesh/receive {from_id, from_name, text} — réception d'un DM d'un autre LaRuche.
/// Auth code-de-mesh si configuré (sinon ouvert, comportement historique sur LAN).
async fn api_mesh_receive(
    headers: axum::http::HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    if let Some(false) = sync::mesh_auth_ok(&headers, "/api/mesh/receive") {
        return Json(serde_json::json!({ "status": "error", "error": "auth mesh invalide" }));
    }
    let from_id = body["from_id"].as_str().unwrap_or("inconnu").to_string();
    let from_name = body["from_name"].as_str().unwrap_or("LaRuche").to_string();
    // Contenu chiffré (`enc`) → déchiffre ; sinon clair (`text`).
    let text = if let Some(enc) = body["enc"].as_str() {
        match sync::open(enc) {
            Some(t) => t.trim().to_string(),
            None => return Json(serde_json::json!({ "status": "error", "error": "déchiffrement échoué" })),
        }
    } else {
        body["text"].as_str().unwrap_or("").trim().to_string()
    };
    if text.is_empty() {
        return Json(serde_json::json!({ "status": "error" }));
    }
    append_inbox(InboxMessage {
        id: Uuid::new_v4().to_string(),
        peer_id: from_id,
        peer_name: from_name,
        dir: "in".into(),
        text,
        ts: chrono::Utc::now().timestamp(),
        read: false,
    });
    Json(serde_json::json!({ "status": "ok" }))
}

/// GET /api/inbox — tous les messages (le client regroupe par pair).
async fn api_inbox_get() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "messages": read_inbox() }))
}

/// POST /api/inbox/read {peer_id} — marque comme lus les messages d'un pair.
async fn api_inbox_read(Json(body): Json<serde_json::Value>) -> Json<serde_json::Value> {
    let peer = body["peer_id"].as_str().unwrap_or("");
    let mut v = read_inbox();
    for m in v.iter_mut() {
        if m.peer_id == peer {
            m.read = true;
        }
    }
    write_inbox(&v);
    Json(serde_json::json!({ "status": "ok" }))
}

/// POST /api/feed/ask {text} — parle à LaRuche DEPUIS le Feed. Tourne sur une session « feed »
/// dédiée (contexte roulant ~10 échanges, isolé du chat principal), en arrière-plan ; la réponse
/// apparaît dans le Feed via activity_log au prochain poll. Capacités agent complètes (crons…).
async fn api_feed_ask(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let text = match body["text"].as_str().map(str::trim).filter(|s| !s.is_empty()) {
        Some(t) => t.to_string(),
        None => return Json(serde_json::json!({ "status": "error", "error": "texte vide" })),
    };
    let st = state.clone();
    tokio::spawn(async move {
        // Session feed dédiée (id déterministe) → contexte roulant, séparé du chat principal.
        let feed_id = Uuid::from_u128(0xFEED_0000_0000_0000_0000_0000_0000_0001);
        let sessions_dir = std::path::Path::new("sessions");
        let model = st.essaim_config.read().await.model.clone();
        let mut session = {
            let mut sessions = st.essaim_sessions.write().await;
            sessions
                .remove(&feed_id)
                .unwrap_or_else(|| Session::new_with_id(feed_id, &model, sessions_dir))
        };
        let (tx, _rx) = tokio::sync::broadcast::channel::<laruche_essaim::ChatEvent>(256);
        let cfg = st.essaim_config.read().await.clone();
        let result = boucle_react_memoire(
            &text,
            &mut session,
            &st.essaim_registry,
            &cfg,
            &tx,
            st.memoire.clone(),
        )
        .await;
        // Contexte roulant court (~10 échanges = 20 messages) : on tronque le plus ancien.
        if session.messages.len() > 20 {
            let drop_n = session.messages.len() - 20;
            session.messages.drain(0..drop_n);
        }
        {
            let now = chrono::Utc::now().to_rfc3339();
            let mut activity = st.activity_log.write().await;
            if activity.len() >= ACTIVITY_LOG_LIMIT {
                activity.pop_front();
            }
            activity.push_back(ActivityLogEntry {
                timestamp: now,
                level: if result.is_ok() { "info" } else { "error" }.into(),
                tag: "agent".into(),
                message: format!("Feed: {}", preview_text(&text, 60)),
                full_prompt: Some(text.clone()),
                full_response: result.as_ref().ok().map(|r| preview_text(r, 4000)),
                model_used: Some(cfg.model.clone()),
                tokens_generated: None,
                latency_ms: None,
                user_id: None,
            });
        }
        let _ = session.sauvegarder();
        st.essaim_sessions.write().await.insert(feed_id, session);
    });
    Json(serde_json::json!({ "status": "ok" }))
}

/// GET /api/profile — fiche utilisateur (nœud `system.user`, injectée au contexte de LaRuche).
async fn api_profile_get(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let fiche = state
        .memoire
        .read_node("system.user")
        .await
        .ok()
        .and_then(|n| {
            n.get("items").and_then(|i| i.as_array()).and_then(|a| {
                a.iter().rev().find_map(|it| {
                    it.get("content").and_then(|c| c.as_str()).map(str::to_string)
                })
            })
        })
        .unwrap_or_default();
    Json(serde_json::json!({ "fiche": fiche }))
}

/// POST /api/profile {fiche} — remplace la fiche utilisateur (item unique). Source `ui-profile`
/// (acteur User dans le Feed). Seul l'utilisateur édite ; l'agent en est interdit (memory_write).
async fn api_profile_save(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let fiche = body["fiche"].as_str().unwrap_or("").trim().to_string();
    if let Ok(node) = state.memoire.read_node("system.user").await {
        if let Some(items) = node.get("items").and_then(|i| i.as_array()) {
            for it in items {
                if let Some(id) = it.get("id").and_then(|x| x.as_str()) {
                    let _ = state.memoire.delete_item(id, Some("ui-profile")).await;
                }
            }
        }
    }
    if !fiche.is_empty() {
        let _ = state
            .memoire
            .write(
                laruche_memoire::MemoryItem::new("system.user", fiche).with_source("ui-profile"),
            )
            .await;
    }
    Json(serde_json::json!({ "status": "ok" }))
}

/// GET /api/feed?limit=N — flux d'activité UNIFIÉ pour le volet Feed global : mutations mémoire
/// (avec acteur User/LaRuche + ref cliquable) + inférences agent (activity_log), trié récent→ancien.
async fn api_feed(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(q): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let limit = q
        .get("limit")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(200);
    let mut events: Vec<serde_json::Value> = Vec::new();

    // 1) Mutations mémoire (qui a ajouté/supprimé/modifié quoi).
    if let Ok(muts) = state.memoire.mutations(Some(150)).await {
        if let Some(arr) = muts.get("mutations").and_then(|m| m.as_array()) {
            for m in arr {
                let op = m.get("op").and_then(|v| v.as_str()).unwrap_or("");
                let node = m.get("node_id").and_then(|v| v.as_str()).unwrap_or("");
                let ts = m.get("ts").and_then(|v| v.as_i64()).unwrap_or(0);
                let src = m.get("src").and_then(|v| v.as_str()).unwrap_or("");
                // Bruit système (non-activité) : indexation d'outils + (re)seed des nœuds au boot
                // + sync disque↔SQL des skills (delete+write par skill à chaque démarrage/watch →
                // floodait le Feed de dizaines de lignes capacities.skills.*).
                if matches!(
                    src,
                    "tool-registry" | "seed" | "skill-file" | "skill-file-sync" | "skill-file-watch"
                ) {
                    continue;
                }
                if (op == "create_node" || op == "update_node")
                    && (node.starts_with("system") || node.starts_with("capacities"))
                {
                    continue;
                }
                let action = match op {
                    "write" if src == "consolidation" => "a consolidé",
                    "write" => "a ajouté un item dans",
                    "propose" => "a proposé un item dans",
                    "update" => "a modifié un item de",
                    "delete" => "a supprimé un item de",
                    "move" => "a déplacé un item vers",
                    "create_node" => "a créé le nœud",
                    "update_node" => "a mis à jour le nœud",
                    "rename_subtree" => "a déplacé le sous-arbre",
                    _ => "a modifié",
                };
                events.push(serde_json::json!({
                    "ts": ts, "actor": feed_actor(src), "kind": "memory",
                    "action": action, "object": node, "ref": node
                }));
            }
        }
    }

    // 2) Échanges agent : le message de l'utilisateur (full_prompt) PUIS la réponse de LaRuche
    //    (full_response nettoyée des tags protocole). Permet de voir ses propres messages dans le
    //    Feed, attribués à User, et une réponse lisible (pas de <plan>/<tool_call> bruts).
    {
        let logs = state.activity_log.read().await;
        for e in logs.iter() {
            // MILLISECONDES (le rfc3339 a la sous-seconde) → ordre correct entre tours d'une même
            // minute. Dans un tour, la question est placée 1 ms AU-DESSUS de sa réponse (lecture
            // Q→A par tour, tours en ordre antéchronologique).
            let ms = chrono::DateTime::parse_from_rfc3339(&e.timestamp)
                .map(|d| d.timestamp_millis())
                .unwrap_or(0);
            // a) Message utilisateur (uniquement pour les échanges de chat).
            if e.tag == "agent" {
                if let Some(prompt) = e.full_prompt.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
                    let clean = prompt.split("\n\n[SYSTEM]").next().unwrap_or(prompt).trim();
                    if !clean.is_empty() {
                        events.push(serde_json::json!({
                            "ts": ms + 1, "actor": "User", "kind": "agent",
                            "action": "a demandé", "object": preview_text(clean, 160),
                            "full": clean, "ref": serde_json::Value::Null, "tag": e.tag
                        }));
                    }
                }
            }
            // b) Réponse de LaRuche, nettoyée (sinon JSON/XML illisible). Vide après nettoyage
            //    (tour purement outil) → on n'ajoute pas d'événement « a répondu » creux.
            let brut = e.full_response.as_deref().filter(|s| !s.is_empty()).unwrap_or(&e.message);
            let resp = nettoyer_reponse_feed(brut);
            if !resp.is_empty() {
                events.push(serde_json::json!({
                    "ts": ms, "actor": "LaRuche", "kind": "agent",
                    "action": "a répondu", "object": preview_text(&resp, 160),
                    "full": resp, "ref": serde_json::Value::Null, "tag": e.tag
                }));
            }
        }
    }

    // 3) Crons exécutés (dernière exécution).
    {
        let cron = state.essaim_cron.read().await;
        for t in cron.list() {
            if let Some(lr) = t.last_run {
                events.push(serde_json::json!({
                    "ts": lr.timestamp(), "actor": "LaRuche", "kind": "cron",
                    "action": "a exécuté le cron", "object": t.name, "ref": serde_json::Value::Null
                }));
            }
        }
    }
    // 4) Missions (dernière itération).
    {
        let missions = state.missions.read().await;
        for m in missions.list() {
            if let Some(lr) = m.last_run.as_deref() {
                let ts = chrono::DateTime::parse_from_rfc3339(lr)
                    .map(|d| d.timestamp())
                    .unwrap_or(0);
                events.push(serde_json::json!({
                    "ts": ts, "actor": "LaRuche", "kind": "mission",
                    "action": "a avancé la mission", "object": m.slug, "ref": serde_json::Value::Null
                }));
            }
        }
    }
    // 5) Watchers déclenchés (dernière détection).
    {
        let watchers = state.watchers.read().await;
        for w in watchers.list() {
            if let Some(lr) = w.last_run {
                events.push(serde_json::json!({
                    "ts": lr.timestamp(), "actor": "LaRuche", "kind": "watcher",
                    "action": "a déclenché le watcher", "object": w.name, "ref": serde_json::Value::Null
                }));
            }
        }
    }

    // 6) Messages directs (DM) du mesh → 1re brique du feed global. Acteur = le PAIR (ruche
    //    violette) pour les reçus ; Moi pour les envoyés.
    for m in read_inbox() {
        let (actor, action, akind) = if m.dir == "out" {
            ("User".to_string(), format!("a écrit à {}", m.peer_name), "user")
        } else {
            (m.peer_name.clone(), "vous a écrit".to_string(), "peer")
        };
        events.push(serde_json::json!({
            "ts": m.ts, "actor": actor, "kind": "dm", "action": action,
            "object": preview_text(&m.text, 160), "full": m.text,
            "ref": serde_json::Value::Null, "actor_kind": akind
        }));
    }

    // Unifie l'unité : les sections mutations/cron/mission/watcher sont en SECONDES, la section
    // agent en MILLISECONDES. On passe tout en ms (un ts < 1e12 = secondes → ×1000) pour un tri
    // cohérent (sinon les events agent, 1000× plus grands, écraseraient tout).
    for e in events.iter_mut() {
        let t = e["ts"].as_i64().unwrap_or(0);
        if t > 0 && t < 1_000_000_000_000 {
            e["ts"] = serde_json::Value::from(t * 1000);
        }
    }
    events.sort_by(|a, b| {
        b["ts"].as_i64().unwrap_or(0).cmp(&a["ts"].as_i64().unwrap_or(0))
    });
    events.truncate(limit);
    Json(serde_json::json!({ "events": events }))
}

/// GET /api/system/prompt-defaults — textes par défaut (codés en dur) des sections éditables,
/// pour pré-remplir l'éditeur : l'utilisateur voit et modifie le prompt complet (vide en DB =
/// ce défaut est utilisé). Le `node_*` override REMPLACE la section correspondante.
async fn api_system_prompt_defaults() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "identity": laruche_essaim::prompt::section_identite_stable(),
        "behavior": laruche_essaim::prompt::section_comportement(),
    }))
}

/// GET /api/memory/tree — lightweight cognitive-map snapshot for the SPA.
async fn api_memory_tree(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let health = state.memoire.health().await.unwrap_or(false);
    let dream = state
        .memoire
        .dream()
        .await
        .unwrap_or_else(|e| serde_json::json!({ "error": e.to_string() }));
    // Vraie arborescence depuis le backend (l'UI Obsidian reconstruit la hiérarchie sur les
    // ids pointés). Fallback sur les racines de base si le backend ne liste pas (sidecar).
    let mut nodes = state
        .memoire
        .list_nodes()
        .await
        .unwrap_or_else(|_| serde_json::json!([]));
    if nodes.as_array().map(|a| a.is_empty()).unwrap_or(true) {
        nodes = serde_json::json!([
            { "id": "people", "label": "People", "one_liner": "Personnes et preferences" },
            { "id": "projects", "label": "Projects", "one_liner": "Projets actifs" },
            { "id": "decisions", "label": "Decisions", "one_liner": "Choix durables" },
            { "id": "capacities", "label": "Capacites", "one_liner": "Outils, plugins, MCP, skills" },
            { "id": "missions", "label": "Missions", "one_liner": "Recherches au long cours" },
            { "id": "sessions", "label": "Sessions", "one_liner": "Contexte conversationnel" },
            { "id": "knowledge", "label": "Knowledge", "one_liner": "Savoirs importes" }
        ]);
    }
    // Marque les nœuds gérés par le système (tools.*/system.*) : l'UI affiche un 🔒 et
    // l'agent ne peut pas les muter (garde-fou `noeud_reserve`). L'admin peut les éditer.
    if let Some(arr) = nodes.as_array_mut() {
        for n in arr.iter_mut() {
            if let Some(id) = n.get("id").and_then(|v| v.as_str()) {
                let prot = id == "capacities"
                    || id == "system"
                    || id == "tools"
                    || id.starts_with("capacities.")
                    || id.starts_with("system.")
                    || id.starts_with("tools.");
                n["protected"] = serde_json::json!(prot);
            }
        }
    }
    Json(serde_json::json!({
        "health": health,
        "nodes": nodes,
        "review": dream.get("suggestions").cloned().unwrap_or_else(|| serde_json::json!([])),
        "audit": dream
    }))
}

/// GET /api/memory/stats — counts (items/nodes/mutations) for the SPA.
async fn api_memory_stats(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    Json(
        state
            .memoire
            .stats()
            .await
            .unwrap_or_else(|e| serde_json::json!({ "error": e.to_string() })),
    )
}

/// GET /api/memory/mutations?limit=50 — audit log of recent memory mutations.
async fn api_memory_mutations(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(q): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let limit = q.get("limit").and_then(|s| s.parse::<u8>().ok());
    Json(
        state
            .memoire
            .mutations(limit)
            .await
            .unwrap_or_else(|e| serde_json::json!({ "error": e.to_string() })),
    )
}

/// GET /api/memory/export_okf?dir=okf-export&node=<id> — exporte en bundle OKF dans un dossier
/// serveur. `node` optionnel = n'exporte que ce nœud + son sous-arbre (sinon toute la carte).
async fn api_memory_export_okf(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(q): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let dir = q
        .get("dir")
        .cloned()
        .unwrap_or_else(|| "okf-export".to_string());
    let node = q.get("node").map(|s| s.as_str()).filter(|s| !s.is_empty());
    match state
        .memoire
        .export_okf(std::path::Path::new(&dir), node)
        .await
    {
        Ok(n) => Json(serde_json::json!({ "ok": true, "files": n, "dir": dir })),
        Err(e) => Json(serde_json::json!({ "ok": false, "error": e.to_string() })),
    }
}

/// Zippe récursivement le contenu d'un dossier dans un buffer mémoire (entrées relatives à `base`).
fn zip_dir_to_bytes(base: &std::path::Path) -> std::io::Result<Vec<u8>> {
    use std::io::{Cursor, Write};
    let mut buf = Vec::new();
    {
        let mut zw = zip::ZipWriter::new(Cursor::new(&mut buf));
        let opts: zip::write::FileOptions<()> =
            zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        let mut stack = vec![base.to_path_buf()];
        while let Some(d) = stack.pop() {
            for entry in std::fs::read_dir(&d)? {
                let path = entry?.path();
                if path.is_dir() {
                    stack.push(path);
                } else if let Ok(rel) = path.strip_prefix(base) {
                    let name = rel.to_string_lossy().replace('\\', "/");
                    zw.start_file(name, opts)
                        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
                    zw.write_all(&std::fs::read(&path)?)?;
                }
            }
        }
        zw.finish()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    }
    Ok(buf)
}

/// GET /api/memory/export.zip?node=<id> — exporte le bundle OKF et le renvoie en .zip
/// TÉLÉCHARGEABLE (Content-Disposition), sans rien écrire durablement côté projet.
/// `node` optionnel = nœud courant + sous-arbre ; sinon toute la mémoire.
async fn api_memory_export_zip(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(q): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<axum::response::Response, StatusCode> {
    use axum::response::IntoResponse;
    let node = q.get("node").map(|s| s.as_str()).filter(|s| !s.is_empty());
    // Dossier temporaire isolé (nettoyé après lecture).
    let tmp = std::env::temp_dir().join(format!("laruche-okf-{}", Uuid::new_v4()));
    let result = state
        .memoire
        .export_okf(&tmp, node)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR);
    let bytes =
        result.and_then(|_| zip_dir_to_bytes(&tmp).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR));
    let _ = std::fs::remove_dir_all(&tmp); // best-effort cleanup
    let bytes = bytes?;
    let fname = match node {
        Some(n) => format!("okf-{}.zip", n.replace('.', "_")),
        None => "okf-memoire.zip".to_string(),
    };
    Ok((
        [
            (
                axum::http::header::CONTENT_TYPE,
                "application/zip".to_string(),
            ),
            (
                axum::http::header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{fname}\""),
            ),
        ],
        bytes,
    )
        .into_response())
}

/// POST /api/memory/import_okf?dir=okf-export — importe un bundle OKF.
async fn api_memory_import_okf(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(q): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let dir = q
        .get("dir")
        .cloned()
        .unwrap_or_else(|| "okf-export".to_string());
    match state.memoire.import_okf(std::path::Path::new(&dir)).await {
        Ok(n) => Json(serde_json::json!({ "ok": true, "imported": n, "dir": dir })),
        Err(e) => Json(serde_json::json!({ "ok": false, "error": e.to_string() })),
    }
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

fn strip_display_tag_blocks(text: &str, tag: &str) -> String {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let mut clean = text.to_string();
    while let Some(start) = clean.find(&open) {
        if let Some(end) = clean[start + open.len()..].find(&close) {
            let end = start + open.len() + end + close.len();
            clean.replace_range(start..end, "");
        } else {
            clean.truncate(start);
            break;
        }
    }
    clean
}

/// Removes instructions injected for the ReAct loop from the user-facing transcript.
fn display_user_text(text: &str) -> Option<String> {
    const CAPABILITY_HINT: &str = "\n\n[SYSTEM] Tu peux planifier (cron_create), surveiller (watcher_create) et chercher dans tes conversations passées (session_search) toi-même.";
    const AUTO_CONTINUE: &str = "Continue immédiatement l'étape suivante du plan";
    const OUTPUT_RECOVERY: &str = "Continue exactement la reponse interrompue.";
    const FAILOVER_RECOVERY: &str = "La reponse precedente a ete tronquee deux fois.";

    if text.starts_with(AUTO_CONTINUE)
        || text.starts_with(OUTPUT_RECOVERY)
        || text.starts_with(FAILOVER_RECOVERY)
    {
        return None;
    }

    let text = text.strip_suffix(CAPABILITY_HINT).unwrap_or(text);
    let text = text.strip_prefix("/no_think\n").unwrap_or(text);
    if let Some((_, steering)) = text.split_once("\n") {
        if text.starts_with("[Steering utilisateur injecte pendant") {
            return Some(steering.to_string());
        }
    }
    Some(text.to_string())
}

/// Converts the durable ReAct transcript to the clean presentation transcript.
/// Internal tool and plan tags remain in storage for the agent, while the UI gets
/// plain assistant text plus the latest structured plan for the left-hand workflow.
fn session_message_for_client(message: &laruche_essaim::Message) -> Option<serde_json::Value> {
    match message {
        laruche_essaim::Message::User(text) => {
            display_user_text(text).map(|text| serde_json::json!({"role": "user", "text": text}))
        }
        laruche_essaim::Message::UserMultimodal { text, attachments } => {
            let text = display_user_text(text)?;
            let att_meta: Vec<serde_json::Value> = attachments
                .iter()
                .map(|a| {
                    serde_json::json!({
                        "kind": a.kind,
                        "mime_type": a.mime_type,
                        "filename": a.filename,
                        "data": if a.kind == "image" { a.data.clone() } else { String::new() }
                    })
                })
                .collect();
            Some(serde_json::json!({
                "role": "user",
                "text": text,
                "attachments": att_meta
            }))
        }
        laruche_essaim::Message::Assistant(text) => {
            let plan = laruche_essaim::brain::parse_plan(text)
                .and_then(|items| serde_json::to_value(items).ok());
            let clean = strip_display_tag_blocks(
                &strip_display_tag_blocks(&strip_display_tag_blocks(text, "tool_call"), "plan"),
                "think",
            );
            let mut value = serde_json::json!({"role": "assistant", "text": clean.trim()});
            if let Some(plan) = plan {
                value["plan"] = plan;
            }
            Some(value)
        }
        laruche_essaim::Message::Thought { phase, kind, text } => Some(serde_json::json!({
            "role": "thought",
            "phase": phase,
            "kind": kind,
            "text": text,
        })),
        laruche_essaim::Message::PromptDebug {
            payload,
            model,
            provider,
        } => Some(serde_json::json!({
            "role": "prompt_debug",
            "payload": payload,
            "model": model,
            "provider": provider,
        })),
        laruche_essaim::Message::Observation { tool, result, .. } => {
            Some(serde_json::json!({"role": "tool", "tool": tool, "text": result}))
        }
        laruche_essaim::Message::ToolCall { name, args } => {
            Some(serde_json::json!({"role": "tool_call", "tool": name, "args": args}))
        }
        // System/compaction notes are model context, never visible chat messages.
        laruche_essaim::Message::System(_) => None,
    }
}

#[cfg(test)]
mod session_display_tests {
    use super::*;

    #[test]
    fn user_display_hides_agent_only_instructions() {
        let raw = "Télécharge ceci\n\n[SYSTEM] Tu peux planifier (cron_create), surveiller (watcher_create) et chercher dans tes conversations passées (session_search) toi-même.";
        assert_eq!(display_user_text(raw).as_deref(), Some("Télécharge ceci"));
        assert!(display_user_text(
            "Continue immédiatement l'étape suivante du plan, sans t'arrêter."
        )
        .is_none());
    }

    #[test]
    fn assistant_display_keeps_plan_structured_and_hides_markup() {
        let message = laruche_essaim::Message::Assistant(
            "<plan>[{\"task\":\"Télécharger\",\"status\":\"done\"}]</plan>\nFichier prêt.<tool_call>{}</tool_call>"
                .into(),
        );
        let display = session_message_for_client(&message).unwrap();
        assert_eq!(display["text"], "Fichier prêt.");
        assert_eq!(display["plan"][0]["task"], "Télécharger");
    }

    #[test]
    fn active_context_stats_progressent_pendant_les_outils() {
        let mut stats = ActiveContextStats {
            messages: 1,
            base_tokens: 65,
            running: true,
            ..ActiveContextStats::default()
        };

        stats.apply_event(&ChatEvent::Token {
            text: "Je vais chercher la page puis analyser le resultat.".into(),
        });
        stats.apply_event(&ChatEvent::ToolCall {
            name: "web_fetch".into(),
            args: serde_json::json!({"url":"https://example.test/long-page"}),
            iteration: Some(1),
        });
        stats.apply_event(&ChatEvent::ToolResult {
            name: "web_fetch".into(),
            result: "contenu ".repeat(200),
            success: true,
            elapsed_ms: Some(42),
        });

        assert!(stats.messages >= 4);
        assert!(stats.used_tokens() > 65);
        assert!(stats.running);
    }
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
            let messages: Vec<serde_json::Value> = session
                .messages
                .iter()
                .filter_map(session_message_for_client)
                .collect();
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
                let messages: Vec<serde_json::Value> = session
                    .messages
                    .iter()
                    .filter_map(session_message_for_client)
                    .collect();
                state.essaim_sessions.write().await.insert(uuid, session);
                Ok(Json(
                    serde_json::json!({"session_id":id,"messages":messages}),
                ))
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
    let query = params
        .get("q")
        .map(|s| s.to_lowercase())
        .unwrap_or_default();
    if query.is_empty() {
        return Json(serde_json::json!([]));
    }

    let sessions = state.essaim_sessions.read().await;
    let mut results = Vec::new();

    for session in sessions.values() {
        // Only search user's own sessions + legacy
        if session.user_id.is_some() && session.user_id != caller {
            continue;
        }
        for msg in &session.messages {
            let text = match msg {
                laruche_essaim::Message::User(t) | laruche_essaim::Message::Assistant(t) => {
                    t.clone()
                }
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
                if results.len() >= 20 {
                    break;
                }
            }
        }
        if results.len() >= 20 {
            break;
        }
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
            laruche_essaim::Message::UserMultimodal { text, attachments } => {
                md.push_str(&format!(
                    "## User\n\n{}\n\n*({} attachment(s) attached)*\n\n",
                    text,
                    attachments.len()
                ));
            }
            laruche_essaim::Message::Assistant(text) => {
                // Strip tool_call tags
                let mut clean = text.clone();
                while let Some(s) = clean.find("<tool_call>") {
                    if let Some(e) = clean.find("</tool_call>") {
                        clean = format!("{}{}", &clean[..s], &clean[e + "</tool_call>".len()..]);
                    } else {
                        clean.truncate(s);
                        break;
                    }
                }
                // Strip plan tags
                while let Some(s) = clean.find("<plan>") {
                    if let Some(e) = clean.find("</plan>") {
                        clean = format!("{}{}", &clean[..s], &clean[e + "</plan>".len()..]);
                    } else {
                        clean.truncate(s);
                        break;
                    }
                }
                let clean = clean.trim();
                if !clean.is_empty() {
                    md.push_str(&format!("## Assistant\n\n{}\n\n", clean));
                }
            }
            laruche_essaim::Message::Observation { tool, result, .. } => {
                md.push_str(&format!(
                    "> **Tool: {}**\n> ```\n> {}\n> ```\n\n",
                    tool,
                    &result[..result.len().min(500)]
                ));
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
async fn api_list_cron(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let cron = state.essaim_cron.read().await;
    let tasks: Vec<serde_json::Value> = cron
        .list()
        .iter()
        .map(|t| {
            serde_json::json!({
                "id": t.id.to_string(),
                "name": t.name,
                "prompt": t.prompt,
                "cron_expr": t.cron_expr,
                "fire_at": t.fire_at,
                "enabled": t.enabled,
                "last_run": t.last_run,
                "run_count": t.run_count,
                "channel": t.channel.clone(),
                "provider": t.provider.clone(),
                "model": t.model.clone(),
                "skills": t.skills.clone(),
            })
        })
        .collect();
    Json(serde_json::json!(tasks))
}

#[derive(Debug, serde::Deserialize)]
struct SpawnAgentRequest {
    task: String,
    context: Option<String>,
    recursion_depth: Option<u32>,
    max_iterations: Option<usize>,
    budget: Option<f32>,
}

#[derive(Debug, serde::Serialize)]
struct SpawnAgentResponse {
    agent_id: String,
    status: String,
}

/// POST /api/agents/spawn — launch a subagent dynamically.
async fn api_spawn_subagent(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<SpawnAgentRequest>,
) -> Result<Json<SpawnAgentResponse>, (StatusCode, Json<serde_json::Value>)> {
    let caller = auth_user::extract_user_from_headers(&headers, &state.cookie_secret);
    if caller.is_none() {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "Unauthorized"})),
        ));
    }

    if payload.task.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "task is required"})),
        ));
    }

    if let Some(depth) = payload.recursion_depth {
        if depth > 3 {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "recursion depth too high (max 3)"})),
            ));
        }
    }

    if let Some(iters) = payload.max_iterations {
        if iters == 0 || iters > 20 {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "max_iterations must be between 1 and 20"})),
            ));
        }
    }

    if let Some(b) = payload.budget {
        if b <= 0.0 {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "budget must be positive"})),
            ));
        }
    }

    let agent_id = Uuid::new_v4();
    let mut config = state.essaim_config.read().await.clone();

    if let Some(iters) = payload.max_iterations {
        config.max_iterations = iters;
    }

    let registry = state.essaim_registry.clone();
    let state_clone = state.clone();
    let task_clone = payload.task.clone();
    let context_clone = payload.context.clone();

    tokio::spawn(async move {
        tracing::info!(agent_id = %agent_id, task = %task_clone, "Subagent spawned via API");
        let _ = state_clone.events.write().await.emit(
            laruche_events::EventKind::AgentStarted,
            "api_spawn",
            serde_json::json!({ "agent_id": agent_id, "task": task_clone }),
        );

        match laruche_essaim::subagent::lancer_sous_agent(
            &task_clone,
            context_clone.as_deref(),
            registry,
            &config,
        )
        .await
        {
            Ok(result) => {
                tracing::info!(agent_id = %agent_id, "Subagent finished successfully");
                let _ = state_clone.events.write().await.emit(
                    laruche_events::EventKind::AgentFinished,
                    "api_spawn",
                    serde_json::json!({ "agent_id": agent_id, "result": result }),
                );
            }
            Err(e) => {
                tracing::error!(agent_id = %agent_id, error = %e, "Subagent failed");
                let mut activity = state_clone.activity_log.write().await;
                if activity.len() >= ACTIVITY_LOG_LIMIT {
                    activity.pop_front();
                }
                activity.push_back(ActivityLogEntry {
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    level: "error".into(),
                    tag: "subagent".into(),
                    message: format!("Subagent {} failed: {}", agent_id, e),
                    full_prompt: Some(task_clone.clone()),
                    full_response: None,
                    model_used: Some(config.model.clone()),
                    tokens_generated: None,
                    latency_ms: None,
                    user_id: caller,
                });
            }
        }
    });

    Ok(Json(SpawnAgentResponse {
        agent_id: agent_id.to_string(),
        status: "spawned".into(),
    }))
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
    if !is_admin {
        return Err(StatusCode::FORBIDDEN);
    }
    let name = body["name"].as_str().unwrap_or("Unnamed task").to_string();
    let prompt = body["prompt"]
        .as_str()
        .ok_or(StatusCode::BAD_REQUEST)?
        .to_string();
    let cron_expr = body["cron_expr"].as_str().map(|s| s.to_string());
    let fire_at = body["fire_at"].as_str().and_then(|s| {
        chrono::DateTime::parse_from_rfc3339(s)
            .ok()
            .map(|dt| dt.with_timezone(&chrono::Utc))
    });
    let channel = body["channel"]
        .as_str()
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());
    let provider = body["provider"]
        .as_str()
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());
    let model = body["model"]
        .as_str()
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());
    let skills: Vec<String> = body["skills"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    let profile_id = body["profile_id"].as_str().map(|s| s.to_string());
    let task = ScheduledTask {
        id: Uuid::new_v4(),
        name,
        prompt,
        cron_expr,
        fire_at,
        channel,
        provider,
        model,
        profile_id,
        skills,
        enabled: true,
        created_at: chrono::Utc::now(),
        last_run: None,
        run_count: 0,
    };

    let id = {
        let mut cron = state.essaim_cron.write().await;
        cron.add(task)
    };

    Ok(Json(
        serde_json::json!({"id": id.to_string(), "status": "created"}),
    ))
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

/// POST /api/cron/:id/run — exécute immédiatement le prompt d'un cron (spawn).
// ─── Missions (« La Reine ») ────────────────────────────────────────────────
/// GET /api/missions — liste les missions au long cours.
async fn api_list_missions(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    Json(serde_json::json!(state.missions.read().await.list()))
}

/// POST /api/missions — crée une mission. Body: {objective, slug?, cadence?}.
async fn api_create_mission(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let objective = body["objective"]
        .as_str()
        .unwrap_or_default()
        .trim()
        .to_string();
    if objective.is_empty() {
        return Json(serde_json::json!({"error": "objective requis"}));
    }
    let slug = body["slug"]
        .as_str()
        .map(missions::slugify)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| missions::slugify(&objective));
    let cadence = body["cadence"]
        .as_str()
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.to_string());
    let m = missions::Mission {
        slug: slug.clone(),
        objective,
        cadence,
        status: "active".to_string(),
        iterations: 0,
        last_run: None,
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    state.missions.write().await.upsert(m);
    Json(serde_json::json!({"status": "ok", "slug": slug}))
}

/// Lance UNE itération de mission (réutilisé par l'API ET le daemon de cadence) : l'agent lit
/// l'état capitalisé sous `missions.<slug>`, avance d'une étape et y écrit ses trouvailles.
async fn lancer_iteration_mission(state: Arc<AppState>, mission: missions::Mission) -> u32 {
    let slug = mission.slug.clone();
    let node_id = format!("missions.{}", slug);
    let etat = match state.memoire.read_node(&node_id).await {
        Ok(v) => v["items"]
            .as_array()
            .map(|its| {
                its.iter()
                    .filter_map(|i| i["content"].as_str())
                    .take(25)
                    .collect::<Vec<_>>()
                    .join("\n- ")
            })
            .unwrap_or_default(),
        Err(_) => String::new(),
    };
    let prompt = missions::prompt_iteration(&mission, &etat);
    let iteration = mission.iterations + 1;
    let run_state = state.clone();
    tokio::spawn(async move {
        let cfg = run_state.essaim_config.read().await.clone();
        let sessions_dir = std::path::Path::new("sessions");
        let mut session = Session::new_with_path(&cfg.model, sessions_dir);
        let (tx, mut rx) = broadcast::channel::<ChatEvent>(64);
        tokio::spawn(async move { while rx.recv().await.is_ok() {} });
        let _ = boucle_react_memoire(
            &prompt,
            &mut session,
            &run_state.essaim_registry,
            &cfg,
            &tx,
            run_state.memoire.clone(),
        )
        .await;
        run_state
            .missions
            .write()
            .await
            .mark_run(&slug, chrono::Utc::now().to_rfc3339());
    });
    iteration
}

/// POST /api/missions/:slug/run — déclenche UNE itération.
async fn api_run_mission(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(slug): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let Some(mission) = state.missions.read().await.get(&slug) else {
        return Json(serde_json::json!({"error": "mission introuvable"}));
    };
    let iteration = lancer_iteration_mission(state.clone(), mission).await;
    Json(serde_json::json!({"status": "started", "slug": slug, "iteration": iteration}))
}

/// Contenus (items) d'un nœud mémoire.
fn items_of(node: &serde_json::Value) -> Vec<String> {
    node["items"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|i| i["content"].as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

/// GET /api/missions/:slug/dossier — assemble le DOSSIER de la mission (synthèse + trouvailles
/// + questions ouvertes, depuis la carte cognitive) en markdown prêt à lire/exporter.
async fn api_mission_dossier(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(slug): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let Some(mission) = state.missions.read().await.get(&slug) else {
        return Json(serde_json::json!({"error": "mission introuvable"}));
    };
    let base = format!("missions.{}", slug);
    let read = |suffix: &str| {
        let n = format!("{base}.{suffix}");
        let mem = state.memoire.clone();
        async move {
            mem.read_node(&n)
                .await
                .ok()
                .as_ref()
                .map(items_of)
                .unwrap_or_default()
        }
    };
    let synthese = read("synthese").await;
    let findings = read("findings").await;
    let questions = read("questions").await;

    let mut md = format!("# Mission : {}\n\n", mission.objective);
    md.push_str(&format!(
        "_Itérations : {} · statut : {}_\n\n",
        mission.iterations, mission.status
    ));
    if let Some(s) = synthese.last() {
        md.push_str(&format!("## Synthèse\n\n{}\n\n", s));
    }
    if !findings.is_empty() {
        md.push_str("## Trouvailles\n\n");
        for f in &findings {
            md.push_str(&format!("- {}\n", f));
        }
        md.push('\n');
    }
    if !questions.is_empty() {
        md.push_str("## Questions ouvertes\n\n");
        for q in &questions {
            md.push_str(&format!("- {}\n", q));
        }
    }
    Json(serde_json::json!({
        "slug": slug,
        "objective": mission.objective,
        "iterations": mission.iterations,
        "findings": findings.len(),
        "questions": questions.len(),
        "markdown": md,
    }))
}

/// POST /api/missions/:slug — met à jour une mission (status pause/active/done, objectif, cadence).
async fn api_update_mission(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(slug): axum::extract::Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let mut store = state.missions.write().await;
    let Some(mut m) = store.get(&slug) else {
        return Json(serde_json::json!({"error": "mission introuvable"}));
    };
    if let Some(s) = body["status"].as_str() {
        m.status = s.to_string();
    }
    if let Some(o) = body["objective"].as_str().filter(|o| !o.trim().is_empty()) {
        m.objective = o.to_string();
    }
    if body.get("cadence").is_some() {
        m.cadence = body["cadence"]
            .as_str()
            .filter(|c| !c.trim().is_empty())
            .map(String::from);
    }
    store.upsert(m);
    Json(serde_json::json!({"status": "ok", "slug": slug}))
}

/// DELETE /api/missions/:slug — supprime une mission (la métadonnée ; le savoir reste en mémoire).
async fn api_delete_mission(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(slug): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let ok = state.missions.write().await.remove(&slug);
    Json(serde_json::json!({"status": if ok {"ok"} else {"not_found"}, "slug": slug}))
}

/// Orbite niveau 2 — DÉCOMPOSE une mission en tâches kanban parallèles (une par question
/// ouverte, sinon un angle à couvrir). Le dispatcher kanban les exécute (recherche), chaque
/// tâche écrivant ses trouvailles dans le sous-arbre de la mission. Les skills sont forgés
/// automatiquement (background_review) à chaque tour. Réutilise tout l'existant.
async fn decomposer_mission(
    state: &Arc<AppState>,
    mission: &missions::Mission,
    max_tasks: usize,
) -> usize {
    let base = format!("missions.{}", mission.slug);
    let questions = state
        .memoire
        .read_node(&format!("{base}.questions"))
        .await
        .ok()
        .as_ref()
        .map(items_of)
        .unwrap_or_default();
    let cibles: Vec<String> = if questions.is_empty() {
        vec![format!(
            "Couvrir l'angle clé le plus important encore non traité de l'objectif : {}",
            mission.objective
        )]
    } else {
        questions.into_iter().take(max_tasks).collect()
    };
    let mut board = state.kanban_board.write().await;
    let mut n = 0;
    for q in cibles {
        let desc = format!(
            "Mission « {obj} ». Traite cette question de recherche : « {q} ».\n\
             Fais une recherche web approfondie, puis écris tes trouvailles SOURCÉES via memory_write \
             sous le node_id `{base}.findings` (un fait = un item clair). Sois rigoureux et factuel.",
            obj = mission.objective,
            q = q,
            base = base
        );
        let task = board.create(
            format!("Mission {} — recherche", mission.slug),
            desc,
            None,
            None,
            None,
        );
        board.change_status(task.id, laruche_kanban::TaskStatus::Ready);
        n += 1;
        if n >= max_tasks {
            break;
        }
    }
    n
}

/// POST /api/missions/:slug/decompose — éclate la mission en tâches kanban parallèles.
async fn api_decompose_mission(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(slug): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let Some(mission) = state.missions.read().await.get(&slug) else {
        return Json(serde_json::json!({"error": "mission introuvable"}));
    };
    let n = decomposer_mission(&state, &mission, 4).await;
    Json(serde_json::json!({"status": "ok", "slug": slug, "tasks_created": n}))
}

async fn api_run_cron(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let uuid = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => return Json(serde_json::json!({"error": "bad id"})),
    };
    let task = {
        let cron = state.essaim_cron.read().await;
        cron.get(&uuid)
    };
    let Some(task) = task else {
        return Json(serde_json::json!({"error": "not found"}));
    };
    let run_state = state.clone();
    tokio::spawn(async move {
        let mut cfg = run_state.essaim_config.read().await.clone();
        if let Some(p) = task.provider.clone() {
            cfg.provider = p;
        }
        cfg.model = task.model.clone().unwrap_or_else(|| cfg.model.clone());
        // Injection des skills attachés (même logique que le daemon), en sautant
        // les skills désactivés via le slider de la page Skills.
        let disabled_sk = cfg.disabled_skills.clone();
        let mut skills_charges: Vec<(String, String)> = Vec::new();
        for skill_name in task.skills.iter().filter(|s| !disabled_sk.contains(s)) {
            let node_id = laruche_skills::skill_node_id(skill_name);
            if let Ok(node) = run_state.memoire.read_node(&node_id).await {
                if let Some(items) = node["items"].as_array() {
                    if let Some(body) = items
                        .iter()
                        .rev()
                        .find_map(|it| it["content"].as_str().filter(|c| c.contains("type: skill")))
                    {
                        skills_charges.push((skill_name.clone(), body.to_string()));
                    }
                }
            }
        }
        let prompt =
            laruche_essaim::orchestration::assembler_prompt_skills(&task.prompt, &skills_charges);
        let sessions_dir = std::path::Path::new("sessions");
        let mut session = Session::new_with_path(&cfg.model, sessions_dir);
        let (tx, mut rx) = broadcast::channel::<ChatEvent>(64);
        tokio::spawn(async move { while rx.recv().await.is_ok() {} });
        let _ = boucle_react_memoire(
            &prompt,
            &mut session,
            &run_state.essaim_registry,
            &cfg,
            &tx,
            run_state.memoire.clone(),
        )
        .await;
    });
    Json(serde_json::json!({"status": "started"}))
}

/// PUT /api/cron/:id — met à jour un cron (édition / décalage d'horaire).
async fn api_update_cron(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let uuid = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => return Json(serde_json::json!({"error": "bad id"})),
    };
    let mut cron = state.essaim_cron.write().await;
    let Some(mut task) = cron.get(&uuid) else {
        return Json(serde_json::json!({"error": "not found"}));
    };
    if let Some(v) = body["name"].as_str() {
        task.name = v.to_string();
    }
    if let Some(v) = body["prompt"].as_str() {
        task.prompt = v.to_string();
    }
    if body.get("cron_expr").is_some() {
        task.cron_expr = body["cron_expr"]
            .as_str()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
    }
    if body.get("fire_at").is_some() {
        task.fire_at = body["fire_at"]
            .as_str()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc));
    }
    if body.get("channel").is_some() {
        task.channel = body["channel"]
            .as_str()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
    }
    if body.get("provider").is_some() {
        task.provider = body["provider"]
            .as_str()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
    }
    if body.get("model").is_some() {
        task.model = body["model"]
            .as_str()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
    }
    if let Some(arr) = body["skills"].as_array() {
        task.skills = arr
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();
    }
    if let Some(b) = body["enabled"].as_bool() {
        task.enabled = b;
    }
    cron.replace(task);
    Json(serde_json::json!({"status": "ok"}))
}

// ─── Skills (OKF en mémoire, capacities.skills.*) — page Settings ────────────────

/// GET /api/skills — liste les skills (nom, description, enabled).
async fn api_list_skills(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let disabled = state.essaim_config.read().await.disabled_skills.clone();
    let mut out: Vec<serde_json::Value> = Vec::new();
    if let Ok(root) = state.memoire.read_node("capacities.skills").await {
        if let Some(children) = root["children"].as_array() {
            for child in children {
                let id = child["id"].as_str().or_else(|| child["node_id"].as_str());
                let Some(id) = id else { continue };
                let name = id
                    .strip_prefix("capacities.skills.")
                    .unwrap_or(id)
                    .to_string();
                // Charge le contenu pour extraire la description.
                let mut description = child["label"].as_str().unwrap_or("").to_string();
                if let Ok(node) = state.memoire.read_node(id).await {
                    if let Some(items) = node["items"].as_array() {
                        if let Some(body) = items.iter().rev().find_map(|it| {
                            it["content"].as_str().filter(|c| c.contains("type: skill"))
                        }) {
                            if let Ok(sk) = laruche_skills::Skill::parse(body) {
                                description = sk.meta.description.clone();
                            }
                        }
                    }
                }
                out.push(serde_json::json!({
                    "name": name,
                    "description": description,
                    "enabled": !disabled.iter().any(|d| d == &name),
                }));
            }
        }
    }
    out.sort_by(|a, b| {
        a["name"]
            .as_str()
            .unwrap_or("")
            .cmp(b["name"].as_str().unwrap_or(""))
    });
    Json(serde_json::json!(out))
}

/// GET /api/skills/:name — retourne le SKILL.md (OKF) complet.
async fn api_get_skill(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let node_id = laruche_skills::skill_node_id(&name);
    if let Ok(node) = state.memoire.read_node(&node_id).await {
        if let Some(items) = node["items"].as_array() {
            if let Some(body) = items
                .iter()
                .rev()
                .find_map(|it| it["content"].as_str().filter(|c| c.contains("type: skill")))
            {
                return Json(serde_json::json!({"name": name, "content": body}));
            }
        }
    }
    Json(serde_json::json!({"error": "not found"}))
}

/// POST /api/skills — crée/met à jour un skill (body: {content} OKF, ou {name, content}).
async fn api_upsert_skill(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    if auth_user::extract_user_from_headers(&headers, &state.cookie_secret).is_none() {
        return Json(serde_json::json!({"error": "unauthorized"}));
    }
    let content = body["content"].as_str().unwrap_or("");
    let sk = match laruche_skills::Skill::parse(content) {
        Ok(s) if !s.meta.name.trim().is_empty() => s,
        _ => {
            return Json(
                serde_json::json!({"error": "frontmatter invalide (name/description requis, type: skill)"}),
            )
        }
    };
    let node_id = laruche_skills::skill_node_id(&sk.meta.name);
    match state
        .memoire
        .write(laruche_memoire::MemoryItem::new(node_id, content).with_source("skills-ui"))
        .await
    {
        Ok(_) => Json(serde_json::json!({"status": "ok", "name": sk.meta.name})),
        Err(e) => Json(serde_json::json!({"error": format!("{e}")})),
    }
}

/// POST /api/skills/:name/toggle — active/désactive un skill (persisté).
async fn api_toggle_skill(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    if auth_user::extract_user_from_headers(&headers, &state.cookie_secret).is_none() {
        return Json(serde_json::json!({"error": "unauthorized"}));
    }
    let enabled = {
        let mut cfg = state.essaim_config.write().await;
        if let Some(pos) = cfg.disabled_skills.iter().position(|d| d == &name) {
            cfg.disabled_skills.remove(pos);
            true
        } else {
            cfg.disabled_skills.push(name.clone());
            false
        }
    };
    save_persistent_state(&state).await;
    Json(serde_json::json!({"status": "ok", "name": name, "enabled": enabled}))
}

/// DELETE /api/skills/:name — supprime le skill (items du nœud) + nettoie l'état.
async fn api_delete_skill(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    if auth_user::extract_user_from_headers(&headers, &state.cookie_secret).is_none() {
        return Json(serde_json::json!({"error": "unauthorized"}));
    }
    let node_id = laruche_skills::skill_node_id(&name);
    let _ = state.memoire.delete_node(&node_id).await;
    {
        let mut cfg = state.essaim_config.write().await;
        cfg.disabled_skills.retain(|d| d != &name);
    }
    save_persistent_state(&state).await;
    Json(serde_json::json!({"status": "ok"}))
}

/// GET /api/watchers — list watchers.
async fn api_list_watchers(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let registry = state.watchers.read().await;
    let watchers: Vec<serde_json::Value> = registry
        .list()
        .iter()
        .map(|w| {
            serde_json::json!({
                "id": w.id,
                "name": w.name,
                "watcher_type": w.watcher_type,
                "target": w.target,
                "condition": w.condition,
                "prompt": w.prompt,
                "active": w.active,
                "run_count": w.run_count,
                "profile_id": w.profile_id,
                "model": w.model,
            })
        })
        .collect();
    Json(serde_json::json!(watchers))
}

/// POST /api/watchers — create a watcher.
async fn api_create_watcher(
    State(state): State<Arc<AppState>>,
    axum::extract::Json(body): axum::extract::Json<serde_json::Value>,
) -> StatusCode {
    let name = body["name"]
        .as_str()
        .unwrap_or("Unnamed Watcher")
        .to_string();
    let prompt = body["prompt"]
        .as_str()
        .unwrap_or("Analyze this change")
        .to_string();
    let target = body["target"].as_str().unwrap_or("").to_string();
    let condition = body["condition"].as_str().unwrap_or("").to_string();
    let w_type_str = body["watcher_type"].as_str().unwrap_or("file");

    let watcher_type = match w_type_str {
        "url" => laruche_watchers::WatcherType::Url,
        "log" => laruche_watchers::WatcherType::Log,
        _ => laruche_watchers::WatcherType::File,
    };

    let watcher = laruche_watchers::Watcher {
        id: Uuid::new_v4(),
        name,
        watcher_type,
        target,
        condition,
        prompt,
        active: true,
        created_at: chrono::Utc::now(),
        last_run: None,
        run_count: 0,
        last_state: None,
        model: body["model"]
            .as_str()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string()),
        profile_id: body["profile_id"]
            .as_str()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string()),
    };

    let mut registry = state.watchers.write().await;
    registry.add(watcher);
    StatusCode::CREATED
}

/// DELETE /api/watchers/:id — remove a watcher.
async fn api_delete_watcher(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> StatusCode {
    if let Ok(uuid) = Uuid::parse_str(&id) {
        let mut registry = state.watchers.write().await;
        if registry.remove(&uuid) {
            return StatusCode::OK;
        }
    }
    StatusCode::NOT_FOUND
}

/// GET /api/kanban — list all tasks
async fn api_kanban_list(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let board = state.kanban_board.read().await;
    Json(serde_json::json!(board.list()))
}

/// POST /api/kanban — create task
async fn api_kanban_create(
    State(state): State<Arc<AppState>>,
    axum::extract::Json(body): axum::extract::Json<serde_json::Value>,
) -> StatusCode {
    let title = body["title"].as_str().unwrap_or("").to_string();
    let description = body["description"].as_str().unwrap_or("").to_string();
    let idempotency_key = body["idempotency_key"].as_str().map(|s| s.to_string());

    let profile_id = body["profile_id"].as_str().map(|s| s.to_string());
    let model = body["model"].as_str().map(|s| s.to_string());
    let mut board = state.kanban_board.write().await;
    board.create(title, description, idempotency_key, profile_id, model);
    StatusCode::CREATED
}

/// PUT /api/kanban/:id/status — update status
async fn api_kanban_update_status(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    axum::extract::Json(body): axum::extract::Json<serde_json::Value>,
) -> StatusCode {
    if let Ok(uuid) = Uuid::parse_str(&id) {
        let status_str = body["status"].as_str().unwrap_or("");
        let status = match status_str {
            "Triage" => laruche_kanban::TaskStatus::Triage,
            "Todo" => laruche_kanban::TaskStatus::Todo,
            "Ready" => laruche_kanban::TaskStatus::Ready,
            "Running" => laruche_kanban::TaskStatus::Running,
            "Blocked" => laruche_kanban::TaskStatus::Blocked,
            "Done" => laruche_kanban::TaskStatus::Done,
            "Archived" => laruche_kanban::TaskStatus::Archived,
            _ => return StatusCode::BAD_REQUEST,
        };
        let mut board = state.kanban_board.write().await;
        if board.change_status(uuid, status) {
            return StatusCode::OK;
        }
    }
    StatusCode::NOT_FOUND
}

/// PUT /api/kanban/:id — update title/description.
async fn api_kanban_update(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    axum::extract::Json(body): axum::extract::Json<serde_json::Value>,
) -> StatusCode {
    if let Ok(uuid) = Uuid::parse_str(&id) {
        let title = body["title"].as_str().map(|s| s.to_string());
        let description = body["description"].as_str().map(|s| s.to_string());
        let mut board = state.kanban_board.write().await;
        if board.update(uuid, title, description).is_some() {
            return StatusCode::OK;
        }
    }
    StatusCode::NOT_FOUND
}

/// POST /api/kanban/:id/dependency — block child by parent
async fn api_kanban_add_dependency(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    axum::extract::Json(body): axum::extract::Json<serde_json::Value>,
) -> StatusCode {
    if let Ok(child_uuid) = Uuid::parse_str(&id) {
        if let Some(parent_str) = body["parent_id"].as_str() {
            if let Ok(parent_uuid) = Uuid::parse_str(parent_str) {
                let mut board = state.kanban_board.write().await;
                if board.add_dependency(child_uuid, parent_uuid) {
                    return StatusCode::OK;
                }
            }
        }
    }
    StatusCode::NOT_FOUND
}

/// DELETE /api/kanban/:id
async fn api_kanban_delete(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> StatusCode {
    if let Ok(uuid) = Uuid::parse_str(&id) {
        let mut board = state.kanban_board.write().await;
        if board.remove(&uuid) {
            return StatusCode::OK;
        }
    }
    StatusCode::NOT_FOUND
}

/// GET /api/doctor — system health check and configuration validation.
async fn api_doctor(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
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
        let caps: Vec<String> = node
            .manifest
            .capabilities
            .iter()
            .map(|c| c.to_string())
            .collect();
        if caps.iter().any(|c| c == "stt") {
            stt_found = true;
        }
        if caps.iter().any(|c| c == "tts") {
            tts_found = true;
        }
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
            .map(|entries| {
                entries
                    .filter(|e| {
                        e.as_ref()
                            .map(|e| e.path().extension().map_or(false, |ext| ext == "json"))
                            .unwrap_or(false)
                    })
                    .count()
            })
            .unwrap_or(0)
    } else {
        0
    };
    checks.push(serde_json::json!({
        "name": "Plugins",
        "status": "ok",
        "detail": format!("{} plugin(s) loaded", plugin_count),
    }));

    // Check Chrome for browser tools
    let chrome_found = if cfg!(windows) {
        std::path::Path::new(r"C:\Program Files\Google\Chrome\Application\chrome.exe").exists()
            || std::path::Path::new(r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe")
                .exists()
    } else {
        which::which("google-chrome").is_ok() || which::which("chromium-browser").is_ok()
    };
    checks.push(serde_json::json!({
        "name": "Browser (Chrome/Edge)",
        "status": if chrome_found { "ok" } else { "warning" },
        "detail": if chrome_found { "Available for browser_navigate/screenshot" } else { "Not found — browser tools disabled" },
    }));

    // Check TLS configuration
    let tls_configured =
        std::env::var("LARUCHE_TLS_CERT").is_ok() && std::env::var("LARUCHE_TLS_KEY").is_ok();
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
    if !is_admin {
        return StatusCode::FORBIDDEN;
    }
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

async fn api_get_notify_config() -> Json<serde_json::Value> {
    let path = std::path::Path::new("channels-config.json");
    if path.exists() {
        if let Ok(content) = std::fs::read_to_string(path) {
            if let Ok(config) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(notify) = config.get("notify") {
                    return Json(notify.clone());
                }
            }
        }
    }
    Json(serde_json::json!({
        "enabled": false
    }))
}

async fn api_set_notify_config(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> StatusCode {
    let users = state.users.read().await;
    let (_, is_admin) = auth_user::check_admin(&headers, &state.cookie_secret, &users);
    drop(users);
    if !is_admin {
        return StatusCode::FORBIDDEN;
    }
    let path = std::path::Path::new("channels-config.json");
    let mut config: serde_json::Value = if path.exists() {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|c| serde_json::from_str(&c).ok())
            .unwrap_or_else(|| serde_json::json!({}))
    } else {
        serde_json::json!({})
    };
    config["notify"] = body;
    if std::fs::write(
        path,
        serde_json::to_string_pretty(&config).unwrap_or_default(),
    )
    .is_ok()
    {
        StatusCode::OK
    } else {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}

// ─── Mode de permission (Always ask / Auto / Plan…) ─────────────────────────

fn permission_mode_to_str(m: laruche_essaim::PermissionMode) -> &'static str {
    use laruche_essaim::PermissionMode::*;
    match m {
        Default => "default",
        Plan => "plan",
        AcceptEdits => "acceptEdits",
        Auto => "auto",
        Bubble => "bubble",
    }
}

fn permission_mode_from_str(s: &str) -> Option<laruche_essaim::PermissionMode> {
    use laruche_essaim::PermissionMode::*;
    match s.trim().to_lowercase().as_str() {
        "default" => Some(Default),
        "plan" => Some(Plan),
        "acceptedits" | "accept_edits" => Some(AcceptEdits),
        "auto" | "yolo" => Some(Auto),
        "bubble" | "always" | "ask" => Some(Bubble),
        _ => None,
    }
}

/// GET /api/config/permission — current permission mode + available options.
async fn api_get_permission_config(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let mode = state.essaim_config.read().await.permission_mode;
    Json(serde_json::json!({
        "mode": permission_mode_to_str(mode),
        "modes": [
            {"id": "default",     "label": "Demander si nécessaire (défaut)"},
            {"id": "acceptEdits", "label": "Accepter les éditions de fichiers"},
            {"id": "plan",        "label": "Plan — lecture seule"},
            {"id": "bubble",      "label": "Toujours demander"},
            {"id": "auto",        "label": "Tout autoriser (ignorer permissions)"},
        ],
    }))
}

/// POST /api/config/permission — set the permission mode (auth required, persisted).
async fn api_set_permission_config(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    auth_user::extract_user_from_headers(&headers, &state.cookie_secret)
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let mode_str = body["mode"].as_str().unwrap_or("");
    let mode = permission_mode_from_str(mode_str).ok_or(StatusCode::BAD_REQUEST)?;
    {
        let mut ec = state.essaim_config.write().await;
        ec.permission_mode = mode;
    }
    save_persistent_state(&state).await;
    Ok(Json(serde_json::json!({
        "status": "ok",
        "mode": permission_mode_to_str(mode),
    })))
}

/// GET /api/config/provider — get current LLM provider settings.
async fn api_get_provider_config(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let ec = state.essaim_config.read().await;
    Json(serde_json::json!({
        "provider": ec.provider,
        "api_key_set": !ec.api_key.is_empty(),
        "api_base": ec.api_base,
        "model": ec.model,
        "ollama_url": ec.ollama_url,
        "fallback_models": ec.fallback_models.join(", "),
        "max_tokens": ec.max_tokens,
        "temperature": ec.temperature,
    }))
}

async fn api_get_context_stats(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let ec = state.essaim_config.read().await;
    let max_messages = ec.context_max_messages;
    let max_tokens = ec.context_max_tokens;

    let session_id = params.get("session_id");
    let (messages, used_tokens) = if let Some(sid_str) = session_id {
        if let Ok(sid) = uuid::Uuid::parse_str(sid_str) {
            let session_stats = state
                .essaim_sessions
                .read()
                .await
                .get(&sid)
                .map(|s| (s.messages.len() as u32, s.estimated_tokens() as u32))
                .unwrap_or((0, 0));
            let active_stats = state.active_context_stats.read().await.get(&sid).cloned();
            if let Some(active) = active_stats {
                if active.running {
                    (
                        active.messages.max(session_stats.0),
                        active.used_tokens().max(session_stats.1),
                    )
                } else {
                    session_stats
                }
            } else {
                session_stats
            }
        } else {
            (0, 0)
        }
    } else {
        (0, 0)
    };

    let ratio = if max_tokens > 0 {
        used_tokens as f32 / max_tokens as f32
    } else {
        0.0
    };

    Json(serde_json::json!({
        "used": messages,
        "max_messages": max_messages,
        "used_tokens": used_tokens,
        "max_tokens": max_tokens,
        "ratio": ratio,
        "messages": messages
    }))
}

async fn api_get_compaction_config(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let ec = state.essaim_config.read().await;
    Json(serde_json::json!({
        "context_max_messages": ec.context_max_messages,
        "compaction_threshold": ec.compaction_threshold
    }))
}

async fn api_set_compaction_config(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    auth_user::extract_user_from_headers(&headers, &state.cookie_secret)
        .ok_or(StatusCode::UNAUTHORIZED)?;

    {
        let mut ec = state.essaim_config.write().await;
        if let Some(max) = body["context_max_messages"].as_u64() {
            ec.context_max_messages = max as usize;
        }
        if let Some(threshold) = body["compaction_threshold"].as_f64() {
            ec.compaction_threshold = threshold as f32;
        }
    }

    save_persistent_state(&state).await;

    Ok(Json(serde_json::json!({
        "status": "ok"
    })))
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
    if let Some(fm) = body["fallback_models"].as_str() {
        cg.fallback_models = fm
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
    }
    if let Some(mt) = body["max_tokens"].as_u64() {
        cg.max_tokens = mt as u32;
    }
    if let Some(t) = body["temperature"].as_f64() {
        cg.temperature = t as f32;
    }
    if body.get("review_model").is_some() {
        cg.review_model = body["review_model"]
            .as_str()
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty());
    }
    let result = serde_json::json!({
        "status": "ok",
        "provider": cg.provider,
        "model": cg.model,
    });
    drop(cg);
    Json(result)
}

// ======================== Credential Pool API ========================

/// GET /api/credentials
async fn api_get_credentials(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<serde_json::Value>, StatusCode> {
    auth_user::extract_user_from_headers(&headers, &state.cookie_secret)
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let pool = state.credential_pool.read().await;
    Ok(Json(serde_json::json!({
        "credentials": pool.entries
    })))
}

/// POST /api/credentials
async fn api_add_credential(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    auth_user::extract_user_from_headers(&headers, &state.cookie_secret)
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let provider = body["provider"].as_str().unwrap_or("").trim().to_string();
    let api_key = body["api_key"].as_str().unwrap_or("").trim().to_string();
    let label = body["label"].as_str().map(|s| s.trim().to_string());

    if provider.is_empty() || api_key.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let mut entry =
        laruche_essaim::credential_pool::CredentialEntry::new(&provider, &api_key, None);
    entry.label = label;

    {
        let mut pool = state.credential_pool.write().await;
        pool.entries.push(entry);
        let _ = std::fs::write(
            &state.credentials_path,
            serde_json::to_string_pretty(&*pool).unwrap(),
        );
    }

    Ok(Json(serde_json::json!({"status": "ok"})))
}

/// DELETE /api/credentials
async fn api_delete_credential(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    auth_user::extract_user_from_headers(&headers, &state.cookie_secret)
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let provider = body["provider"].as_str().unwrap_or("").trim();
    let api_key = body["api_key"].as_str().unwrap_or("").trim();

    {
        let mut pool = state.credential_pool.write().await;
        let initial_len = pool.entries.len();
        pool.entries
            .retain(|e| !(e.provider == provider && e.api_key == api_key));
        if pool.entries.len() < initial_len {
            let _ = std::fs::write(
                &state.credentials_path,
                serde_json::to_string_pretty(&*pool).unwrap(),
            );
        }
    }

    Ok(Json(serde_json::json!({"status": "ok"})))
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
                    let masked = format!("{}...{}", &key[..4], &key[key.len() - 4..]);
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
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    let max_context_length = body["max_context_length"]
        .as_u64()
        .map(|v| v as u32)
        .unwrap_or_else(|| match provider.as_str() {
            "anthropic" => 200000,
            "codex" => 128000,
            "openai" => 128000,
            _ => 32768,
        });

    let profile = profiles::ProviderProfile {
        provider,
        name: name.clone(),
        base_url,
        api_key,
        models,
        visibilite: Default::default(), allowed_peers: Vec::new(),
        max_context_length,
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

    Ok(Json(
        serde_json::json!({"status": "ok", "id": id, "name": name}),
    ))
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

// ─── Auth ChatGPT Codex (OAuth abonnement) pour l'UI web ────────────────────
//
// Le flux device code est asynchrone : `start` lance la connexion en tâche de
// fond et renvoie immédiatement l'URL + le code à afficher ; le frontend poll
// ensuite `status` jusqu'à `connected`. À la réussite, un profil provider
// "codex" est auto-créé pour un usage en un clic.

#[derive(Clone, Serialize, Default)]
struct CodexLoginStatus {
    phase: String, // idle | pending | connected | error
    verification_url: String,
    user_code: String,
    message: String,
    account_id: Option<String>,
}

fn codex_login_cell() -> &'static std::sync::Mutex<CodexLoginStatus> {
    static CELL: std::sync::OnceLock<std::sync::Mutex<CodexLoginStatus>> =
        std::sync::OnceLock::new();
    CELL.get_or_init(|| std::sync::Mutex::new(CodexLoginStatus::default()))
}

fn codex_set_status(f: impl FnOnce(&mut CodexLoginStatus)) {
    if let Ok(mut s) = codex_login_cell().lock() {
        f(&mut s);
    }
}

/// Modèles supportés par Codex avec un compte ChatGPT (abonnement).
/// NB: les variantes `*-codex` (gpt-5.4-codex…) sont rejetées en 400 sur ce
/// backend — seuls les modèles "chat" généraux sont acceptés.
const CODEX_CHATGPT_MODELS: &[&str] = &["gpt-5.5", "gpt-5.4", "gpt-5.4-mini"];

/// Auto-crée (ou met à jour) le profil provider "codex" (ChatGPT abonnement).
async fn ensure_codex_profile(state: &Arc<AppState>) {
    let id = "codex-chatgpt";
    let models: Vec<String> = CODEX_CHATGPT_MODELS.iter().map(|s| s.to_string()).collect();
    let mut cfg = state.profiles.write().await;
    match cfg.profiles.get_mut(id) {
        Some(p) => {
            // Profil existant : on rafraîchit la liste de modèles + base URL
            // (corrige un profil créé avec d'anciens modèles non supportés).
            p.provider = "codex".to_string();
            p.base_url = laruche_essaim::codex_auth::DEFAULT_CODEX_BASE_URL.to_string();
            p.models = models;
        }
        None => {
            cfg.profiles.insert(
                id.to_string(),
                profiles::ProviderProfile {
                    provider: "codex".to_string(),
                    name: "ChatGPT Codex".to_string(),
                    base_url: laruche_essaim::codex_auth::DEFAULT_CODEX_BASE_URL.to_string(),
                    api_key: String::new(),
                    models,
                    visibilite: Default::default(), allowed_peers: Vec::new(),
                    max_context_length: 128000,
                },
            );
        }
    }
    let _ = profiles::save_profiles(&state.profiles_path, &cfg);
}

/// GET /api/auth/codex/status — état de connexion ChatGPT Codex.
async fn api_codex_status(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<serde_json::Value>, StatusCode> {
    auth_user::extract_user_from_headers(&headers, &state.cookie_secret)
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let live = codex_login_cell()
        .lock()
        .map(|s| s.clone())
        .unwrap_or_default();
    // Un login en cours (pending) ou en erreur prime sur l'état stocké.
    if live.phase == "pending" || live.phase == "error" {
        return Ok(Json(serde_json::to_value(&live).unwrap_or_default()));
    }
    // Sinon, refléter les tokens persistés.
    match laruche_essaim::codex_auth::read_codex_tokens() {
        Some(t) => {
            let acct = laruche_essaim::codex_auth::account_id_from_token(&t.access_token);
            Ok(Json(serde_json::json!({
                "phase": "connected",
                "account_id": acct,
                "expiring": laruche_essaim::codex_auth::access_token_is_expiring(&t.access_token, 60),
            })))
        }
        None => Ok(Json(serde_json::json!({"phase": "idle"}))),
    }
}

/// POST /api/auth/codex/start — démarre le flux device code, renvoie URL + code.
async fn api_codex_start(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<serde_json::Value>, StatusCode> {
    auth_user::extract_user_from_headers(&headers, &state.cookie_secret)
        .ok_or(StatusCode::UNAUTHORIZED)?;

    codex_set_status(|s| {
        *s = CodexLoginStatus {
            phase: "pending".into(),
            message: "Initialisation...".into(),
            ..Default::default()
        };
    });

    let (tx, rx) = tokio::sync::oneshot::channel::<(String, String)>();
    let state_bg = state.clone();
    tokio::spawn(async move {
        let res = laruche_essaim::codex_auth::device_code_login(move |url, code| {
            codex_set_status(|s| {
                s.phase = "pending".into();
                s.verification_url = url.to_string();
                s.user_code = code.to_string();
                s.message = "En attente de connexion dans le navigateur...".into();
            });
            let _ = tx.send((url.to_string(), code.to_string()));
        })
        .await;
        match res {
            Ok(tokens) => {
                let _ = laruche_essaim::codex_auth::save_codex_tokens(&tokens);
                let acct = laruche_essaim::codex_auth::account_id_from_token(&tokens.access_token);
                ensure_codex_profile(&state_bg).await;
                codex_set_status(|s| {
                    s.phase = "connected".into();
                    s.account_id = acct;
                    s.message = "Connecté !".into();
                });
            }
            Err(e) => {
                codex_set_status(|s| {
                    s.phase = "error".into();
                    s.message = format!("{e}");
                });
            }
        }
    });

    // Attendre brièvement que la 1re requête renvoie le code à afficher.
    match tokio::time::timeout(std::time::Duration::from_secs(25), rx).await {
        Ok(Ok((url, code))) => Ok(Json(serde_json::json!({
            "phase": "pending",
            "verification_url": url,
            "user_code": code,
        }))),
        _ => {
            let live = codex_login_cell()
                .lock()
                .map(|s| s.clone())
                .unwrap_or_default();
            let msg = if live.message.is_empty() {
                "Impossible d'obtenir le code, réessayez.".to_string()
            } else {
                live.message
            };
            Ok(Json(serde_json::json!({
                "phase": if live.phase == "error" { "error" } else { "pending" },
                "message": msg,
            })))
        }
    }
}

/// POST /api/auth/codex/logout — supprime les tokens Codex stockés.
async fn api_codex_logout(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<serde_json::Value>, StatusCode> {
    auth_user::extract_user_from_headers(&headers, &state.cookie_secret)
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let _ = laruche_essaim::codex_auth::clear_codex_tokens();
    codex_set_status(|s| *s = CodexLoginStatus::default());
    Ok(Json(serde_json::json!({"phase": "idle"})))
}

/// GET /api/profiles/models — unified model list across all profiles.
async fn api_get_unified_models(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
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

/// POST /api/profiles/:id/visibility — bascule la visibilité mesh d'un provider.
async fn api_set_visibility(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let vis = match body["visibility"].as_str() {
        Some("public_proxy") => profiles::Visibilite::PublicProxy,
        Some("restricted") => profiles::Visibilite::Restricted,
        Some("prive") => profiles::Visibilite::Prive,
        _ => {
            return Json(serde_json::json!(
                {"error": "visibility must be 'prive' | 'public_proxy' | 'restricted'"}
            ))
        }
    };
    let allowed: Vec<String> = body["allowed_peers"]
        .as_array()
        .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();
    let mut cfg = state.profiles.write().await;
    match cfg.profiles.get_mut(&id) {
        Some(p) => {
            p.visibilite = vis;
            if vis == profiles::Visibilite::Restricted {
                p.allowed_peers = allowed;
            }
        }
        None => return Json(serde_json::json!({"error": "profile not found"})),
    }
    let _ = profiles::save_profiles(&state.profiles_path, &cfg);
    Json(serde_json::json!({"status": "ok", "id": id, "visibility": body["visibility"]}))
}

/// POST /api/models/use — sélection 2-clics d'un modèle (local ou mesh) pour sa capacité.
async fn api_models_use(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let name = body["name"].as_str().unwrap_or_default().to_string();
    if name.is_empty() {
        return Json(serde_json::json!({"error": "missing model name"}));
    }
    let host = body["host"].as_str().unwrap_or_default().to_string();
    let capability = body["capability"].as_str().unwrap_or("llm").to_lowercase();
    let node_id = body["node_id"].as_str().filter(|s| !s.is_empty());
    let base_url_in = body["base_url"].as_str().map(|s| s.to_string());

    let (provider, base_url, profile_id, disp) = if let Some(nid) = node_id {
        let burl = base_url_in.clone().unwrap_or_else(|| host.clone());
        (
            "miel".to_string(),
            burl,
            format!("miel-{nid}"),
            format!("{host} (mesh)"),
        )
    } else if host == "ollama" {
        (
            "ollama".to_string(),
            state.config.ollama_url.clone(),
            "ollama-local".to_string(),
            "Ollama Local".to_string(),
        )
    } else {
        let burl = base_url_in.clone().unwrap_or_else(|| {
            local_inference::backends_openai_compat_par_defaut()
                .into_iter()
                .find(|b| b.label == host)
                .map(|b| b.base_url)
                .unwrap_or_default()
        });
        (
            "openai".to_string(),
            burl,
            format!("local-{host}"),
            format!("{host} (local)"),
        )
    };

    // Dédup : si un profil existant sert déjà ce modèle, on le RÉUTILISE (évite les
    // doublons "local-llama.cpp" vs "llamacpp-8001", ou un faux "local-codex").
    let existing_id = {
        let cfg = state.profiles.read().await;
        cfg.profiles
            .iter()
            .find(|(_, p)| p.models.iter().any(|m| m == &name))
            .map(|(id, _)| id.clone())
    };
    let profile_id = existing_id.unwrap_or(profile_id);

    {
        let mut cfg = state.profiles.write().await;
        // Crée le profil SEULEMENT s'il n'existe pas (sinon on n'écrase ni son
        // provider, ni sa base_url, ni sa clé — on ajoute juste le modèle).
        let prof =
            cfg.profiles
                .entry(profile_id.clone())
                .or_insert_with(|| profiles::ProviderProfile {
                    provider: provider.clone(),
                    name: disp.clone(),
                    base_url: base_url.clone(),
                    api_key: String::new(),
                    models: vec![],
                    visibilite: profiles::Visibilite::Prive, allowed_peers: Vec::new(),
                    max_context_length: 128000,
                });
        if !prof.models.contains(&name) {
            prof.models.push(name.clone());
        }
        // On ne change le LLM de chat actif QUE pour "llm"/"agent".
        if capability == "llm" || capability == "agent" {
            cfg.active_model = profiles::ActiveModel {
                profile_id: profile_id.clone(),
                model: name.clone(),
            };
        }
        let _ = profiles::save_profiles(&state.profiles_path, &cfg);
    }
    state
        .default_models
        .write()
        .await
        .insert(capability.clone(), name.clone());
    state.capability_selection.write().await.insert(
        capability.clone(),
        CapabilitySelection {
            capability: capability.clone(),
            model: name.clone(),
            backend: host.clone(),
            node_id: node_id.map(|s| s.to_string()),
            is_local: node_id.is_none(),
            profile_id: profile_id.clone(),
        },
    );

    sync_essaim_from_profiles(&state).await;
    save_persistent_state(&state).await;
    Json(
        serde_json::json!({"status": "ok", "profile_id": profile_id, "model": name, "capability": capability}),
    )
}

/// GET /api/capabilities/selection — sélection courante de service par capacité.
async fn api_capabilities_selection(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let sel = state.capability_selection.read().await;
    Json(serde_json::json!({ "selection": serde_json::to_value(&*sel).unwrap_or_default() }))
}

/// `(profile_id, model)` choisi pour une capacité (ex. "code"), s'il existe.
async fn capability_profile(state: &Arc<AppState>, capability: &str) -> Option<(String, String)> {
    let sel = state.capability_selection.read().await;
    sel.get(capability)
        .map(|s| (s.profile_id.clone(), s.model.clone()))
}

/// Applique provider + clé + base_url + modèle d'un **profil** sur `config`.
/// Résolution unique réutilisée (chat capacité, cron, watcher, kanban).
async fn appliquer_profil(
    state: &Arc<AppState>,
    config: &mut EssaimConfig,
    profile_id: &str,
    model: Option<&str>,
) {
    let profiles = state.profiles.read().await;
    if let Some(p) = profiles.profiles.get(profile_id) {
        config.provider = p.provider.clone();
        config.api_key = p.api_key.clone();
        if p.provider == "ollama" {
            config.ollama_url = p.base_url.clone();
            config.api_base = None;
        } else {
            config.api_base = Some(p.base_url.clone());
        }
        if let Some(m) = model {
            config.model = m.to_string();
        } else if let Some(first) = p.models.first() {
            config.model = first.clone();
        }
    }
}

/// Applique le profil servant `capability` (s'il y a une sélection).
async fn appliquer_capacite(state: &Arc<AppState>, config: &mut EssaimConfig, capability: &str) {
    if let Some((pid, model)) = capability_profile(state, capability).await {
        appliquer_profil(state, config, &pid, Some(&model)).await;
    }
}

/// Sync the active profile into EssaimConfig so brain.rs picks it up.
async fn sync_essaim_from_profiles(state: &Arc<AppState>) {
    let cfg = state.profiles.read().await;
    let (provider, model, api_key, api_base, ollama_url, max_context_length) = profiles::active_to_essaim_fields(&cfg);
    drop(cfg);

    let mut ec = state.essaim_config.write().await;
    ec.provider = provider;
    ec.model = model;
    ec.api_key = api_key;
    ec.api_base = api_base;
    ec.ollama_url = ollama_url;
    ec.context_max_tokens = max_context_length;
}

// ======================== Events Endpoints ========================

async fn api_get_events(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> Json<Vec<laruche_events::Event>> {
    let since_id = params
        .get("since")
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);
    let events = state.events.read().await.since(since_id);
    Json(events)
}

async fn api_export_events(
    State(state): State<Arc<AppState>>,
) -> Result<String, axum::http::StatusCode> {
    let ndjson = state
        .events
        .read()
        .await
        .to_ndjson()
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(ndjson)
}

// ======================== Auth Endpoints ========================

/// POST /api/auth/enroll — Create a new user identity.
async fn api_auth_enroll(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Result<
    (
        axum::http::StatusCode,
        axum::http::HeaderMap,
        Json<serde_json::Value>,
    ),
    StatusCode,
> {
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
    tokio::spawn(async move {
        sync::push_user_to_peers(&sync_user, &sync_state).await;
    });

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
async fn api_auth_challenge(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
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

    state
        .auth_challenges
        .write()
        .await
        .insert(challenge_id, challenge);

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
        Some(c) if c.is_expired() => Json(serde_json::json!({"status": "expired"})),
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
        Some(_) => Json(serde_json::json!({"status": "pending"})),
        None => Json(serde_json::json!({"status": "not_found"})),
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
) -> Result<
    (
        axum::http::StatusCode,
        axum::http::HeaderMap,
        axum::response::Html<String>,
    ),
    StatusCode,
> {
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
) -> Result<
    (
        axum::http::StatusCode,
        axum::http::HeaderMap,
        Json<serde_json::Value>,
    ),
    StatusCode,
> {
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
                format!(
                    "laruche_auth={}; HttpOnly; Path=/; SameSite=Lax; Max-Age=2592000",
                    cookie_value
                )
                .parse()
                .unwrap(),
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
        return Ok(Json(
            serde_json::json!({"error": "Password must be at least 4 characters"}),
        ));
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
        user.preferred_model = if model.is_empty() {
            None
        } else {
            Some(model.clone())
        };
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
        state
            .users
            .read()
            .await
            .get(&uid)
            .map(|u| u.role == auth_user::UserRole::Admin)
            .unwrap_or(false)
    } else {
        false
    };
    let kb = state.essaim_kb.read().await;
    let entries: Vec<serde_json::Value> = kb
        .entries
        .iter()
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
        })
        .collect();
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
        state
            .users
            .read()
            .await
            .get(&uid)
            .map(|u| u.role == auth_user::UserRole::Admin)
            .unwrap_or(false)
    } else {
        false
    };
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
    if kb.remove(&id) {
        StatusCode::OK
    } else {
        StatusCode::NOT_FOUND
    }
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
        return Json(
            serde_json::json!({"status": "error", "message": "No channels-config.json found. Configure in Settings > Channels."}),
        );
    };

    match channel {
        "telegram" => {
            let token = config["telegram"]["bot_token"]
                .as_str()
                .unwrap_or("")
                .to_string();
            let allowed = config["telegram"]["allowed_chats"]
                .as_str()
                .unwrap_or("")
                .to_string();
            if token.is_empty() {
                return Json(
                    serde_json::json!({"status": "error", "message": "No Telegram bot token configured"}),
                );
            }
            let state_clone = state.clone();
            let handle = tokio::spawn(async move {
                run_telegram_bot(&token, &allowed, &state_clone).await;
            });
            state
                .channel_handles
                .write()
                .await
                .insert("telegram".into(), handle);
            info!("Telegram bot started");
            Json(serde_json::json!({"status": "started", "channel": "telegram"}))
        }
        _ => Json(
            serde_json::json!({"status": "error", "message": format!("Unknown channel: {}", channel)}),
        ),
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
async fn api_channels_status(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let handles = state.channel_handles.read().await;
    let running: Vec<&String> = handles.keys().collect();
    Json(serde_json::json!({"running": running}))
}

/// Telegram bot — runs as a background task within the server.
async fn run_telegram_bot(token: &str, allowed_chats: &str, state: &Arc<AppState>) {
    let api = format!("https://api.telegram.org/bot{}", token);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .unwrap();
    let allowed: Vec<String> = allowed_chats
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let mut offset: i64 = 0;
    let mut processed_ids: std::collections::HashSet<i64> = std::collections::HashSet::new();
    let mut tg_sessions: std::collections::HashMap<i64, Uuid> = std::collections::HashMap::new();
    let active_steers: Arc<
        tokio::sync::RwLock<std::collections::HashMap<i64, tokio::sync::mpsc::Sender<String>>>,
    > = Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new()));
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
                            let _ = client
                                .get(format!("{}/getUpdates?offset={}&timeout=0", api, offset))
                                .send()
                                .await;
                        }

                        for update in updates {
                            let update_id = update["update_id"].as_i64().unwrap_or(0);
                            if processed_ids.contains(&update_id) {
                                continue;
                            }
                            processed_ids.insert(update_id);
                            // Keep set small — only remember last 100
                            if processed_ids.len() > 100 {
                                let min = *processed_ids.iter().min().unwrap_or(&0);
                                processed_ids.remove(&min);
                            }

                            let chat_id = update["message"]["chat"]["id"].as_i64().unwrap_or(0);
                            let text = update["message"]["text"].as_str().unwrap_or("");
                            let user = update["message"]["from"]["first_name"]
                                .as_str()
                                .unwrap_or("?");

                            if text.is_empty() || chat_id == 0 {
                                continue;
                            }

                            // Check allowlist
                            if !allowed.is_empty() && !allowed.contains(&chat_id.to_string()) {
                                let _ = client.post(format!("{}/sendMessage", api))
                                    .json(&serde_json::json!({"chat_id": chat_id, "text": "Access denied."}))
                                    .send().await;
                                continue;
                            }

                            if text == "/start" || text == "/clear" || text == "/reset" {
                                tg_sessions.insert(chat_id, Uuid::new_v4());
                                let _ = client
                                    .post(format!("{}/sendMessage", api))
                                    .json(&serde_json::json!({
                                        "chat_id": chat_id,
                                        "text": "Nouvelle conversation commencée. L'historique de la session a été réinitialisé.",
                                        "parse_mode": "Markdown",
                                    }))
                                    .send()
                                    .await;
                                continue;
                            }

                            // Check for active steering
                            let mut steers_lock = active_steers.write().await;
                            if let Some(steer_tx) = steers_lock.get(&chat_id) {
                                match steer_tx.try_send(text.to_string()) {
                                    Ok(()) => {
                                        let _ = client
                                            .post(format!("{}/sendMessage", api))
                                            .json(&serde_json::json!({
                                                "chat_id": chat_id,
                                                "text": "Orientation reçue — appliquée à la prochaine étape.",
                                            }))
                                            .send()
                                            .await;
                                    }
                                    Err(_) => {
                                        let _ = client
                                            .post(format!("{}/sendMessage", api))
                                            .json(&serde_json::json!({
                                                "chat_id": chat_id,
                                                "text": "La tâche vient de se terminer : envoie ce message comme nouvelle demande.",
                                            }))
                                            .send()
                                            .await;
                                    }
                                }
                                continue;
                            }

                            // Setup steering for new task
                            let (steer_tx, steer_rx) = tokio::sync::mpsc::channel::<String>(100);
                            steers_lock.insert(chat_id, steer_tx);
                            drop(steers_lock);

                            info!(
                                user = user,
                                chat_id = chat_id,
                                text = &text[..text.len().min(50)],
                                "Telegram message"
                            );

                            // Get or create LaRuche user for this Telegram chat_id
                            let tg_user_id = {
                                let tg_name = format!("telegram:{}", chat_id);
                                let users = state.users.read().await;
                                if let Some(u) = auth_user::find_user_by_name(&users, &tg_name) {
                                    u.id
                                } else {
                                    drop(users);
                                    let new_user = auth_user::create_user(
                                        &tg_name,
                                        auth_user::UserRole::User,
                                        None,
                                    );
                                    let uid = new_user.id;
                                    let _ = auth_user::save_user(
                                        &new_user,
                                        std::path::Path::new("users"),
                                    );
                                    state.users.write().await.insert(uid, new_user);
                                    info!(chat_id = chat_id, user_id = %uid, "Auto-created Telegram user");
                                    uid
                                }
                            };

                            // Telegram efface l'indicateur "typing…" après quelques secondes.
                            // Le maintenir pendant tout le tour évite l'impression que le bot a
                            // abandonné la demande pendant un appel outil ou une réponse longue.
                            let (typing_stop, mut typing_stopped) =
                                tokio::sync::watch::channel(false);
                            let typing_client = client.clone();
                            let typing_api = api.clone();
                            let typing_task = tokio::spawn(async move {
                                let mut ticker =
                                    tokio::time::interval(std::time::Duration::from_secs(4));
                                loop {
                                    tokio::select! {
                                        _ = ticker.tick() => {
                                            if let Err(error) = typing_client
                                                .post(format!("{}/sendChatAction", typing_api))
                                                .json(&serde_json::json!({"chat_id": chat_id, "action": "typing"}))
                                                .send()
                                                .await
                                            {
                                                tracing::debug!(error = %error, chat_id, "Telegram typing update failed");
                                            }
                                        }
                                        changed = typing_stopped.changed() => {
                                            if changed.is_err() || *typing_stopped.borrow() {
                                                break;
                                            }
                                        }
                                    }
                                }
                            });

                            // Query agent with current default model
                            let current_model = get_llm_default(state).await;
                            let sessions_dir = std::path::Path::new("sessions");

                            let session_id =
                                *tg_sessions.entry(chat_id).or_insert_with(Uuid::new_v4);
                            let mut session = if let Ok(mut loaded) =
                                Session::charger(&sessions_dir.join(format!("{}.json", session_id)))
                            {
                                loaded.model = current_model.clone();
                                loaded
                            } else {
                                Session::new_with_id(session_id, &current_model, sessions_dir)
                            };
                            session.user_id = Some(tg_user_id);
                            let (tx, _rx) = broadcast::channel::<ChatEvent>(64);

                            let mut config = state.essaim_config.read().await.clone();
                            config.model = current_model;

                            let state_clone = state.clone();
                            let client_clone = client.clone();
                            let api_clone = api.clone();
                            let text_clone = text.to_string();
                            let user_clone = user.to_string();
                            let active_steers_clone = active_steers.clone();

                            tokio::spawn(async move {
                                let result = boucle_react_memoire_multimodal(
                                    &text_clone,
                                    &mut session,
                                    &state_clone.essaim_registry,
                                    &config,
                                    &tx,
                                    state_clone.memoire.clone(),
                                    vec![],
                                    None,
                                    Some(steer_rx),
                                )
                                .await;
                                let _ = typing_stop.send(true);
                                let _ = typing_task.await;

                                let mut response = match result {
                                    Ok(r) => {
                                        let mut clean = r;
                                        while let Some(s) = clean.find("<tool_call>") {
                                            if let Some(e) = clean.find("</tool_call>") {
                                                clean = format!(
                                                    "{}{}",
                                                    &clean[..s],
                                                    &clean[e + "</tool_call>".len()..]
                                                );
                                            } else {
                                                clean.truncate(s);
                                                break;
                                            }
                                        }
                                        while let Some(s) = clean.find("<plan>") {
                                            if let Some(e) = clean.find("</plan>") {
                                                clean = format!(
                                                    "{}{}",
                                                    &clean[..s],
                                                    &clean[e + "</plan>".len()..]
                                                );
                                            } else {
                                                clean.truncate(s);
                                                break;
                                            }
                                        }
                                        clean.trim().to_string()
                                    }
                                    Err(e) => format!("Error: {}", e),
                                };
                                if response.trim().is_empty() {
                                    response =
                                        "✅ Terminé. Aucune réponse textuelle supplémentaire."
                                            .to_string();
                                }

                                let chunks: Vec<String> = response
                                    .chars()
                                    .collect::<Vec<_>>()
                                    .chunks(4000)
                                    .map(|c| c.iter().collect())
                                    .collect();
                                for chunk in chunks {
                                    if let Err(error) = send_telegram_text(
                                        &client_clone,
                                        &api_clone,
                                        chat_id,
                                        &chunk,
                                    )
                                    .await
                                    {
                                        tracing::error!(error = %error, chat_id, "Telegram final response failed to send");
                                    }
                                }

                                let _ = session.sauvegarder();
                                state_clone
                                    .essaim_sessions
                                    .write()
                                    .await
                                    .insert(session.id, session.clone());

                                tracing::info!(
                                    user = user_clone,
                                    response_len = response.len(),
                                    "Telegram replied"
                                );

                                active_steers_clone.write().await.remove(&chat_id);
                            });
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

/// Sends a plain Telegram message and treats API rejections as real errors.
///
/// Agent output is intentionally not parsed as Telegram Markdown: ordinary code snippets,
/// paths and tool output frequently contain unbalanced Markdown markers.
async fn send_telegram_text(
    client: &reqwest::Client,
    api: &str,
    chat_id: i64,
    text: &str,
) -> Result<()> {
    let response = client
        .post(format!("{api}/sendMessage"))
        .json(&serde_json::json!({
            "chat_id": chat_id,
            "text": text,
        }))
        .send()
        .await?;
    if response.status().is_success() {
        return Ok(());
    }
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    Err(anyhow::anyhow!(
        "Telegram sendMessage rejected ({status}): {body}"
    ))
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

    let result = boucle_react_memoire(
        text,
        &mut session,
        &state.essaim_registry,
        &config,
        &tx,
        state.memoire.clone(),
    )
    .await;

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
                        .find(|o| {
                            o["name"].as_str() == Some("prompt")
                                || o["name"].as_str() == Some("message")
                        })
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

            info!(
                user = user,
                command = command_name,
                input = &input[..input.len().min(50)],
                "Discord slash command"
            );

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
            warn!(
                interaction_type = interaction_type,
                "Discord: unknown interaction type"
            );
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
                    info!(
                        user = user,
                        channel = channel,
                        event_type = event_subtype,
                        text = &text[..text.len().min(50)],
                        "Slack event"
                    );

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
                                info!(
                                    channel = channel,
                                    response_len = response.len(),
                                    "Slack replied"
                                );
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
    let cwd = std::env::current_dir()
        .unwrap_or_default()
        .display()
        .to_string();
    Json(serde_json::json!({"cwd": cwd}))
}

/// POST /api/cwd — set current working directory.
async fn api_set_cwd(Json(body): Json<serde_json::Value>) -> Json<serde_json::Value> {
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

/// GET /api/media/local?path=... — serves an explicitly selected local media file.
///
/// The route is intentionally confined to the current workspace. `media_present`
/// applies the same restriction before it ever produces a local-media card.
#[derive(Deserialize)]
struct LocalMediaQuery {
    path: String,
}

async fn api_media_local(
    Query(query): Query<LocalMediaQuery>,
) -> Result<axum::response::Response, StatusCode> {
    const MAX_LOCAL_MEDIA_BYTES: u64 = 250 * 1024 * 1024;

    let root = std::env::current_dir()
        .ok()
        .and_then(|path| std::fs::canonicalize(path).ok())
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let path = std::fs::canonicalize(&query.path).map_err(|_| StatusCode::NOT_FOUND)?;
    if !path.starts_with(&root) {
        return Err(StatusCode::FORBIDDEN);
    }
    let metadata = std::fs::metadata(&path).map_err(|_| StatusCode::NOT_FOUND)?;
    if !metadata.is_file() {
        return Err(StatusCode::NOT_FOUND);
    }
    if metadata.len() > MAX_LOCAL_MEDIA_BYTES {
        return Err(StatusCode::PAYLOAD_TOO_LARGE);
    }
    let bytes = tokio::fs::read(&path)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mime = local_media_mime(&path);
    Ok((
        [
            (axum::http::header::CONTENT_TYPE, mime),
            (axum::http::header::CACHE_CONTROL, "no-store"),
        ],
        bytes,
    )
        .into_response())
}

fn local_media_mime(path: &std::path::Path) -> &'static str {
    match path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "avif" => "image/avif",
        "pdf" => "application/pdf",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "ogv" => "video/ogg",
        "mov" => "video/quicktime",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "ogg" => "audio/ogg",
        "m4a" => "audio/mp4",
        "flac" => "audio/flac",
        _ => "application/octet-stream",
    }
}

async fn api_onboarding(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let mut steps = Vec::new();

    // 1. Ollama installed?
    let ec = state.essaim_config.read().await;
    let ollama_ok = reqwest::Client::new()
        .get(format!("{}/api/tags", ec.ollama_url))
        .timeout(std::time::Duration::from_secs(3))
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false);
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
    let has_stt = nodes.values().any(|n| {
        n.manifest
            .capabilities
            .iter()
            .any(|c| c.to_string() == "stt")
    });
    let has_tts = nodes.values().any(|n| {
        n.manifest
            .capabilities
            .iter()
            .any(|c| c.to_string() == "tts")
    });
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

    let done_count = steps
        .iter()
        .filter(|s| s["done"].as_bool().unwrap_or(false))
        .count();

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
        let prefix = path
            .file_name()
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
    *state.last_activity.write().await = std::time::Instant::now();
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
        "tools" => Json(serde_json::json!({
            "result": state.essaim_registry.noms(),
        })),
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
            match state
                .essaim_registry
                .executer(tool_name, tool_args, &ctx)
                .await
            {
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
    let model = body["model"].as_str().unwrap_or(&default_model).to_string();

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
    let prompt_for_agent = inject_no_think(prompt, body["no_think"].as_bool().unwrap_or(false));
    let model_override = body["model"].as_str().map(|s| s.to_string());

    // Use current dynamic default model, not initial config
    let current_model = get_llm_default(&state).await;
    let sessions_dir = std::path::Path::new("sessions");
    let session_id = uuid::Uuid::new_v4();
    let mut session = Session::new_with_id(session_id, &current_model, sessions_dir);

    let mut config = state.essaim_config.read().await.clone();
    config.model = model_override.unwrap_or(current_model);

    let (tx, mut rx) = broadcast::channel::<ChatEvent>(256);

    let result = boucle_react_memoire(
        &prompt_for_agent,
        &mut session,
        &state.essaim_registry,
        &config,
        &tx,
        state.memoire.clone(),
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
            ChatEvent::ToolResult {
                name,
                success,
                elapsed_ms,
                ..
            } => {
                if let Some(last) = tools_used.last_mut() {
                    if last["name"].as_str() == Some(&name) {
                        last["success"] = serde_json::json!(success);
                        last["elapsed_ms"] = serde_json::json!(elapsed_ms);
                    }
                }
            }
            ChatEvent::Plan { items } => {
                plan_items = items
                    .iter()
                    .map(|i| serde_json::json!({"task": i.task, "status": i.status}))
                    .collect();
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
    tokio::spawn(async move {
        sync::push_session_to_peers(&sync_session, &sync_state).await;
    });
    state
        .essaim_sessions
        .write()
        .await
        .insert(session_id, session);

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
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> axum::response::Response {
    let user_id = params.get("user_id").and_then(|s| Uuid::parse_str(s).ok());
    ws.on_upgrade(move |socket| ws_chat_connection(socket, state, user_id))
}

async fn ws_chat_connection(
    socket: ws::WebSocket,
    state: Arc<AppState>,
    auth_user_id: Option<Uuid>,
) {
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
                        serde_json::json!({"type":"error","message":"Invalid JSON"})
                            .to_string()
                            .into(),
                    ))
                    .await;
                continue;
            }
        };

        let msg_type = incoming["type"].as_str().unwrap_or("message");

        // Handle "subscribe" — reattach to an existing running session
        if msg_type == "subscribe" {
            let sessions_dir = std::path::Path::new("sessions");
            let mut sessions = state.essaim_sessions.write().await;
            if let Some(session_id_str) = incoming["session_id"].as_str() {
                if let Ok(id) = Uuid::parse_str(session_id_str) {
                    // Try to load from disk if not in memory
                    if !sessions.contains_key(&id) {
                        if let Ok(loaded) = laruche_essaim::Session::charger(
                            &sessions_dir.join(format!("{}.json", id)),
                        ) {
                            sessions.insert(id, loaded);
                        }
                    }
                    if let Some(session) = sessions.get_mut(&id) {
                        if let Some(tx) = &session.event_tx {
                            let mut rx = tx.subscribe();
                            drop(sessions);
                            let _ = sender.send(ws::Message::Text(serde_json::json!({"type":"session","session_id": id.to_string()}).to_string().into())).await;
                            // Enter the broadcast loop — relay events to the reattached client
                            let mut done = false;
                            while !done {
                                tokio::select! {
                                    event_result = rx.recv() => {
                                        if let Ok(event) = event_result {
                                            update_active_context_stats(&state, id, &event).await;
                                            let json = serde_json::to_string(&event).unwrap_or_default();
                                            if sender.send(ws::Message::Text(json.into())).await.is_err() {
                                                done = true;
                                            }
                                        } else {
                                            done = true;
                                        }
                                    }
                                    client_msg_opt = receiver.next() => {
                                        match client_msg_opt {
                                            Some(Ok(ws::Message::Close(_))) | None => { done = true; }
                                            _ => {}
                                        }
                                    }
                                }
                            }
                            continue; // go back to outer loop
                        }
                    }
                }
            }
            continue;
        }

        if msg_type == "steer" {
            let _ = sender
                .send(ws::Message::Text(
                    serde_json::json!({
                        "type": "steer_rejected",
                        "reason": "no_active_run",
                        "text": incoming["text"].as_str().unwrap_or(""),
                        "message": "Aucune tâche active : la demande va être relancée."
                    })
                    .to_string()
                    .into(),
                ))
                .await;
            continue;
        }

        let user_text = match incoming["text"].as_str() {
            Some(t) if !t.trim().is_empty() => t.to_string(),
            _ => continue,
        };
        let mut user_text_for_agent =
            inject_no_think(&user_text, incoming["no_think"].as_bool().unwrap_or(false));
        user_text_for_agent = format!("{}\n\n[SYSTEM] Tu peux planifier (cron_create), surveiller (watcher_create) et chercher dans tes conversations passées (session_search) toi-même.", user_text_for_agent);

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

        // Immediate persistence — save right after creating (before agent runs)
        if let Some(s) = sessions.get(&session_id) {
            let _ = s.sauvegarder();
        }

        // Create or reuse broadcast channel
        let (tx, mut rx) = if let Some(s) = sessions.get_mut(&session_id) {
            if let Some(existing_tx) = &s.event_tx {
                (existing_tx.clone(), existing_tx.subscribe())
            } else {
                let (new_tx, new_rx) =
                    tokio::sync::broadcast::channel::<laruche_essaim::ChatEvent>(256);
                s.event_tx = Some(new_tx.clone());
                (new_tx, new_rx)
            }
        } else {
            let (new_tx, new_rx) =
                tokio::sync::broadcast::channel::<laruche_essaim::ChatEvent>(256);
            (new_tx, new_rx)
        };

        drop(sessions);

        // Send session_id back so the client can persist it
        let _ = sender
            .send(ws::Message::Text(
                serde_json::json!({"type":"session","session_id": session_id.to_string()})
                    .to_string()
                    .into(),
            ))
            .await;

        // Model override from client
        let model_override = incoming["model"].as_str().map(|s| s.to_string());
        // Profile (provider) override from client — the model dropdown sends the
        // selected profile id so we can switch provider/base_url/api_key, not
        // just the model name (sinon un modèle Codex partirait vers llama.cpp).
        let profile_override = incoming["provider"].as_str().map(|s| s.to_string());
        // Capacité explicite du tour (ex. "code" pour coder) → résout un modèle dédié.
        let capability_override = incoming["capability"].as_str().map(|s| s.to_lowercase());

        // Parse attachments from client message
        let mut attachments = match incoming.get("attachments") {
            Some(v) => {
                serde_json::from_value::<Vec<laruche_essaim::session::Attachment>>(v.clone())
                    .unwrap_or_default()
            }
            None => vec![],
        };
        // Fallback for older UI versions sending `images: ["base64..."]`
        if attachments.is_empty() {
            if let Some(imgs) = incoming["images"].as_array() {
                for img in imgs {
                    if let Some(s) = img.as_str() {
                        attachments.push(laruche_essaim::session::Attachment {
                            kind: "image".to_string(),
                            mime_type: "image/jpeg".to_string(),
                            data: s.to_string(),
                            filename: None,
                        });
                    }
                }
            }
        }

        // Create approval channel
        let (approval_tx, approval_rx) =
            tokio::sync::mpsc::channel::<laruche_essaim::ApprovalResponse>(4);

        // Extract session, run ReAct, then put it back
        let state_clone = state.clone();
        let ws_user_id = auth_user_id;
        let actor = ws_user_id
            .map(|u| u.to_string())
            .unwrap_or_else(|| "user".to_string());

        let _ = state.events.write().await.emit(
            laruche_events::EventKind::AgentStarted,
            &actor,
            serde_json::json!({ "session_id": session_id, "prompt": user_text }),
        );
        let user_text_log = user_text.clone();
        let user_text_clone = user_text_for_agent.clone();
        let tx_clone = tx.clone();

        let (steer_tx, steer_rx) = tokio::sync::mpsc::channel::<String>(100);
        let actor_react = actor.clone();

        let react_handle = tokio::spawn(async move {
            let sessions_dir = std::path::Path::new("sessions");
            let ec_snapshot = state_clone.essaim_config.read().await.clone();
            let mut session = {
                let mut sessions = state_clone.essaim_sessions.write().await;
                sessions.remove(&session_id).unwrap_or_else(|| {
                    Session::new_with_id(session_id, &ec_snapshot.model, sessions_dir)
                })
            };

            // Rend la session visible IMMÉDIATEMENT (avant même la réponse) + la persiste sur
            // disque : elle apparaît dans Sessions et survit à un F5 (l'agent, lui, tourne déjà
            // en tâche de fond dans ce tokio::spawn détaché du WebSocket).
            {
                let mut snapshot = session.clone();
                snapshot.ajouter_user(&user_text_log);
                let _ = snapshot.sauvegarder();
                state_clone.active_context_stats.write().await.insert(
                    session_id,
                    ActiveContextStats::from_session(&snapshot, true),
                );
                state_clone
                    .essaim_sessions
                    .write()
                    .await
                    .insert(session_id, snapshot);
            }

            let mut config = ec_snapshot;
            // Liste des ruches du mesh joignables → injectée au contexte (l'agent peut `mesh_send`).
            {
                let listener = state_clone.listener.read().await;
                let nodes = listener.get_nodes().await;
                let me = state_clone.manifest.read().await.node_id;
                let lignes: Vec<String> = nodes
                    .values()
                    .filter(|n| n.manifest.node_id != Some(me))
                    .filter_map(|n| {
                        n.manifest.node_id.map(|id| {
                            format!(
                                "- {} — {}",
                                n.manifest.node_name.clone().unwrap_or_else(|| "ruche".into()),
                                id
                            )
                        })
                    })
                    .collect();
                if !lignes.is_empty() {
                    config.mesh_peers_hint = Some(lignes.join("\n"));
                }
            }
            // Résoudre le profil sélectionné → provider/base_url/api_key.
            if let Some(ref pid) = profile_override {
                let profiles = state_clone.profiles.read().await;
                if let Some(p) = profiles.profiles.get(pid) {
                    config.provider = p.provider.clone();
                    config.api_key = p.api_key.clone();
                    if p.provider == "ollama" {
                        config.ollama_url = p.base_url.clone();
                        config.api_base = None;
                    } else {
                        config.api_base = Some(p.base_url.clone());
                    }
                }
            }
            // Capacité explicite (ex. "code") sans override de profil → modèle dédié à cette capacité.
            if profile_override.is_none() {
                if let Some(ref cap) = capability_override {
                    if cap != "llm" {
                        appliquer_capacite(&state_clone, &mut config, cap).await;
                    }
                }
            }
            if let Some(ref model) = model_override {
                config.model = model.clone();
            }

            let result = boucle_react_memoire_multimodal(
                &user_text_clone,
                &mut session,
                &state_clone.essaim_registry,
                &config,
                &tx_clone,
                state_clone.memoire.clone(),
                attachments,
                Some(approval_rx),
                Some(steer_rx),
            )
            .await;

            // Log to activity (visible in dashboard)
            {
                let now = chrono::Utc::now().to_rfc3339();
                let mut activity = state_clone.activity_log.write().await;
                if activity.len() >= ACTIVITY_LOG_LIMIT {
                    activity.pop_front();
                }
                activity.push_back(ActivityLogEntry {
                    timestamp: now,
                    level: if result.is_ok() { "info" } else { "error" }.into(),
                    tag: "agent".into(),
                    message: format!("Agent chat: {}", preview_text(&user_text_log, 60)),
                    full_prompt: Some(user_text_log.clone()),
                    full_response: result.as_ref().ok().map(|r| preview_text(r, 4000)),
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
            tokio::spawn(async move {
                sync::push_session_to_peers(&sync_s, &sync_st).await;
            });

            // Put session back
            let mut sessions = state_clone.essaim_sessions.write().await;
            sessions.insert(session_id, session.clone());
            drop(sessions);
            state_clone.active_context_stats.write().await.insert(
                session_id,
                ActiveContextStats::from_session(&session, false),
            );

            // Notify globally that session finished
            let last_msg = session
                .messages
                .last()
                .map(|m| match m {
                    laruche_essaim::Message::Assistant(t) | laruche_essaim::Message::User(t) => {
                        t.clone()
                    }
                    _ => String::new(),
                })
                .unwrap_or_default();
            let preview = if last_msg.len() > 100 {
                format!("{}...", &last_msg[..97])
            } else {
                last_msg
            };
            let _ = state_clone.events.write().await.emit(
                laruche_events::EventKind::SessionFinished,
                &actor_react,
                serde_json::json!({ "session_id": session_id, "preview": preview }),
            );
        });

        // Forward events to WebSocket + listen for approvals from client
        let mut done = false;
        while !done {
            tokio::select! {
                // Events from the ReAct loop → send to client
                event_result = rx.recv() => {
                    match event_result {
                        Ok(event) => {
                            update_active_context_stats(&state, session_id, &event).await;
                            let json = serde_json::to_string(&event).unwrap_or_default();
                            if sender.send(ws::Message::Text(json.into())).await.is_err() {
                                done = true;
                            }
                            match &event {
                                laruche_essaim::ChatEvent::ToolCall { name, args, .. } => {
                                    let _ = state.events.write().await.emit(
                                        laruche_events::EventKind::ToolCall,
                                        &actor,
                                        serde_json::json!({ "session_id": session_id, "tool": name, "args": args })
                                    );
                                }
                                laruche_essaim::ChatEvent::ToolResult { name, result, success, .. } => {
                                    let _ = state.events.write().await.emit(
                                        laruche_events::EventKind::ToolResult,
                                        &actor,
                                        serde_json::json!({ "session_id": session_id, "tool": name, "result": preview_text(result, 200), "success": success })
                                    );
                                }
                                laruche_essaim::ChatEvent::Done { .. } => {
                                    let _ = state.events.write().await.emit(
                                        laruche_events::EventKind::AgentFinished,
                                        &actor,
                                        serde_json::json!({ "session_id": session_id, "status": "done" })
                                    );
                                    done = true;
                                }
                                laruche_essaim::ChatEvent::Error { message } => {
                                    let _ = state.events.write().await.emit(
                                        laruche_events::EventKind::AgentFinished,
                                        &actor,
                                        serde_json::json!({ "session_id": session_id, "status": "error", "error": message })
                                    );
                                    done = true;
                                }
                                _ => {}
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
                                } else if json["type"].as_str() == Some("steer") {
                                    let steer_text = json["text"].as_str().unwrap_or("").trim();
                                    if steer_text.is_empty() {
                                        continue;
                                    }
                                    match steer_tx.try_send(steer_text.to_string()) {
                                        Ok(()) => {
                                            let _ = sender.send(ws::Message::Text(
                                                serde_json::json!({
                                                    "type": "steer_ack",
                                                    "text": steer_text,
                                                    "message": "Orientation reçue — appliquée à la prochaine étape."
                                                }).to_string().into()
                                            )).await;
                                        }
                                        Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {
                                            let _ = sender.send(ws::Message::Text(
                                                serde_json::json!({
                                                    "type": "steer_rejected",
                                                    "reason": "queue_full",
                                                    "text": steer_text,
                                                    "message": "Trop d'orientations en attente : attends la prochaine étape."
                                                }).to_string().into()
                                            )).await;
                                        }
                                        Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
                                            let _ = sender.send(ws::Message::Text(
                                                serde_json::json!({
                                                    "type": "steer_rejected",
                                                    "reason": "run_finished",
                                                    "text": steer_text,
                                                    "message": "La tâche vient de se terminer : renvoie ce message comme nouvelle demande."
                                                }).to_string().into()
                                            )).await;
                                        }
                                    }
                                } else if json["type"].as_str() == Some("stop") {
                                    // Stop demandé : on abort la tâche agent. La session a déjà été
                                    // sauvegardée en snapshot (avec le message user) AVANT le run,
                                    // donc seule la réponse en cours est abandonnée — pas la session.
                                    react_handle.abort();
                                    if let Some(stats) =
                                        state.active_context_stats.write().await.get_mut(&session_id)
                                    {
                                        stats.running = false;
                                    }
                                    let _ = state.events.write().await.emit(
                                        laruche_events::EventKind::AgentFinished,
                                        &actor,
                                        serde_json::json!({ "session_id": session_id, "status": "stopped" }),
                                    );
                                    let _ = sender
                                        .send(ws::Message::Text(
                                            serde_json::json!({
                                                "type": "stopped",
                                                "session_id": session_id.to_string(),
                                                "message": "Génération interrompue."
                                            })
                                            .to_string()
                                            .into(),
                                        ))
                                        .await;
                                    done = true;
                                }
                            }
                        }
                        Some(Ok(ws::Message::Close(_))) | None => { done = true; }
                        _ => {}
                    }
                }
            }
        }

        // let _ = react_handle.await; (Detached to allow background running without blocking WS cleanup)
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
            let caps: Vec<String> = node
                .manifest
                .capabilities
                .iter()
                .map(|c| c.to_string())
                .collect();
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

    let _ = sender
        .send(ws::Message::Text(
            serde_json::json!({"type": "ready", "stt_url": &stt_url, "tts_url": &tts_url})
                .to_string()
                .into(),
        ))
        .await;

    let client = reqwest::Client::new();

    while let Some(Ok(msg)) = receiver.next().await {
        match msg {
            ws::Message::Binary(audio_data) => {
                // Step 1: Send audio to STT service
                let stt_result = client
                    .post(format!("{}/transcribe", stt_url))
                    .multipart(
                        reqwest::multipart::Form::new().part(
                            "file",
                            reqwest::multipart::Part::bytes(audio_data.to_vec())
                                .file_name("audio.wav")
                                .mime_str("audio/wav")
                                .unwrap(),
                        ),
                    )
                    .send()
                    .await;

                let transcript = match stt_result {
                    Ok(resp) => match resp.json::<serde_json::Value>().await {
                        Ok(json) => json["text"].as_str().unwrap_or("").to_string(),
                        Err(e) => {
                            let _ = sender.send(ws::Message::Text(
                                    serde_json::json!({"type":"error","message":format!("STT parse error: {}", e)}).to_string().into()
                                )).await;
                            continue;
                        }
                    },
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
                let _ = sender
                    .send(ws::Message::Text(
                        serde_json::json!({"type":"transcript","text":&transcript})
                            .to_string()
                            .into(),
                    ))
                    .await;

                // Step 2: Run through ReAct agent
                let sessions_dir = std::path::Path::new("sessions");
                let audio_config = state.essaim_config.read().await.clone();
                let mut session = Session::new_with_path(&audio_config.model, sessions_dir);
                let (tx, _rx) = broadcast::channel::<ChatEvent>(64);

                let agent_result = boucle_react_memoire(
                    &transcript,
                    &mut session,
                    &state.essaim_registry,
                    &audio_config,
                    &tx,
                    state.memoire.clone(),
                )
                .await;

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
                let _ = sender
                    .send(ws::Message::Text(
                        serde_json::json!({"type":"response","text":&response_text})
                            .to_string()
                            .into(),
                    ))
                    .await;

                // Step 3: Send response to TTS service
                let tts_result = client
                    .post(format!("{}/synthesize", tts_url))
                    .json(&serde_json::json!({"text": &response_text}))
                    .send()
                    .await;

                match tts_result {
                    Ok(resp) if resp.status().is_success() => {
                        if let Ok(audio_bytes) = resp.bytes().await {
                            let _ = sender
                                .send(ws::Message::Binary(audio_bytes.to_vec().into()))
                                .await;
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

// ======================== Plugins API ========================

async fn api_plugin_get(
    axum::extract::Path(name): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let path = std::path::Path::new("plugins").join(format!("{}.json", name));
    if !path.exists() {
        return Err(StatusCode::NOT_FOUND);
    }
    let content = tokio::fs::read_to_string(&path)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({ "content": content })))
}

async fn api_plugin_save(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(name): axum::extract::Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let content = body["content"].as_str().ok_or(StatusCode::BAD_REQUEST)?;
    let path = std::path::Path::new("plugins").join(format!("{}.json", name));
    tokio::fs::create_dir_all("plugins").await.ok();
    tokio::fs::write(&path, content)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Reload plugins
    let plugins_dir = std::path::Path::new("plugins");
    laruche_essaim::abeilles::plugins::charger_plugins(plugins_dir, &state.essaim_registry);

    Ok(Json(serde_json::json!({ "status": "ok", "name": name })))
}

async fn api_plugin_delete(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let path = std::path::Path::new("plugins").join(format!("{}.json", name));
    if path.exists() {
        tokio::fs::remove_file(&path)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    // Reload plugins
    let plugins_dir = std::path::Path::new("plugins");
    laruche_essaim::abeilles::plugins::charger_plugins(plugins_dir, &state.essaim_registry);

    Ok(Json(serde_json::json!({ "status": "ok", "name": name })))
}

// ======================== Main ========================

/// GET /api/mcp/servers
async fn api_mcp_list_servers(
    State(_state): State<Arc<AppState>>,
) -> axum::Json<serde_json::Value> {
    let path = std::path::Path::new("mcp_servers.json");
    if path.exists() {
        if let Ok(content) = std::fs::read_to_string(path) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                return axum::Json(json);
            }
        }
    }
    axum::Json(serde_json::json!({ "mcpServers": {} }))
}

/// POST /api/mcp/servers/:name
async fn api_mcp_save_server(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(name): axum::extract::Path<String>,
    axum::extract::Json(body): axum::extract::Json<serde_json::Value>,
) -> axum::Json<serde_json::Value> {
    let command = body["command"].as_str().unwrap_or("").to_string();
    let mut args = vec![];
    if let Some(args_arr) = body["args"].as_array() {
        for a in args_arr {
            if let Some(s) = a.as_str() {
                args.push(s.to_string());
            }
        }
    }

    let path = std::path::Path::new("mcp_servers.json");
    let mut servers: laruche_essaim::mcp_client::McpServersFile = if path.exists() {
        let content = std::fs::read_to_string(path).unwrap_or_default();
        serde_json::from_str(&content).unwrap_or_else(|_| {
            laruche_essaim::mcp_client::McpServersFile {
                mcpServers: std::collections::HashMap::new(),
            }
        })
    } else {
        laruche_essaim::mcp_client::McpServersFile {
            mcpServers: std::collections::HashMap::new(),
        }
    };

    servers.mcpServers.insert(
        name.clone(),
        laruche_essaim::mcp_client::McpServerConfig { command, args },
    );

    if let Ok(json) = serde_json::to_string_pretty(&servers) {
        let _ = std::fs::write(path, json);
    }

    // Reload all MCP tools
    state
        .essaim_registry
        .supprimer_par_origine(laruche_essaim::abeille::ToolOrigin::Mcp);
    let _ = laruche_essaim::mcp_client::charger_mcp_servers(path, &state.essaim_registry).await;

    axum::Json(serde_json::json!({ "status": "ok", "name": name }))
}

/// DELETE /api/mcp/servers/:name
async fn api_mcp_delete_server(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> axum::Json<serde_json::Value> {
    let path = std::path::Path::new("mcp_servers.json");
    if path.exists() {
        if let Ok(content) = std::fs::read_to_string(path) {
            if let Ok(mut servers) =
                serde_json::from_str::<laruche_essaim::mcp_client::McpServersFile>(&content)
            {
                servers.mcpServers.remove(&name);
                if let Ok(json) = serde_json::to_string_pretty(&servers) {
                    let _ = std::fs::write(path, json);
                }
            }
        }
    }

    // Reload all MCP tools
    state
        .essaim_registry
        .supprimer_par_origine(laruche_essaim::abeille::ToolOrigin::Mcp);
    let _ = laruche_essaim::mcp_client::charger_mcp_servers(path, &state.essaim_registry).await;

    axum::Json(serde_json::json!({ "status": "deleted", "name": name }))
}

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

    info!(name = %config.node_name, tier = ?config.tier, "Starting LaRuche node");

    let local_ip = miel_protocol::get_local_ip();
    info!(ip = %local_ip, "Detected local IP");

    let mut manifest = CognitiveManifest::new(config.node_name.clone(), config.tier);
    // IDENTITÉ PERSISTANTE (identity.json). Sans ça, node_id = Uuid::new_v4() à CHAQUE démarrage :
    // la ruche apparaît comme un NOUVEAU nœud aux pairs à chaque reboot (l'ancien expire) → c'est
    // une cause directe du flapping. On charge l'ID sauvegardé, ou on persiste celui généré.
    {
        let id_path = std::path::Path::new("identity.json");
        let saved = std::fs::read_to_string(id_path)
            .ok()
            .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
            .and_then(|v| v.get("node_id").and_then(|x| x.as_str()).map(String::from))
            .and_then(|s| Uuid::parse_str(&s).ok());
        match saved {
            Some(id) => {
                manifest.node_id = id;
                info!(node_id = %id, "Identite chargee (identity.json)");
            }
            None => {
                let _ = std::fs::write(
                    id_path,
                    serde_json::json!({ "node_id": manifest.node_id.to_string() }).to_string(),
                );
                info!(node_id = %manifest.node_id, "Nouvelle identite persistee (identity.json)");
            }
        }
    }
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

    // NOTE confidentialité : on N'annonce PLUS les backends locaux détectés au démarrage. Le mesh
    // ne doit exposer que les providers explicitement publics (`public_proxy`) — c'est la boucle
    // de re-annonce (plus bas) qui reconstruit les capacités à partir du public uniquement.

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
        initial_defaults
            .entry(cap_name)
            .or_insert_with(|| cap.model_name.clone());
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
    for entry in persistent
        .activity_log
        .into_iter()
        .rev()
        .take(ACTIVITY_LOG_LIMIT)
    {
        initial_log.push_front(entry);
    }

    // Load provider profiles (multi-provider support)
    let profiles_path = PathBuf::from("provider-profiles.json");
    let mut profiles_cfg = profiles::load_profiles(&profiles_path);

    // Migrate old single-provider config into profiles if no profiles exist beyond default
    if profiles_cfg.profiles.len() <= 1
        && !config.provider.is_empty()
        && config.provider != "ollama"
    {
        let migrated_id = format!("{}-migrated", config.provider);
        profiles_cfg.profiles.insert(
            migrated_id.clone(),
            profiles::ProviderProfile {
                provider: config.provider.clone(),
                name: config.provider.clone(),
                base_url: config.api_base.clone().unwrap_or_else(|| {
                    match config.provider.as_str() {
                        "openai" => "https://api.openai.com".to_string(),
                        "anthropic" => "https://api.anthropic.com".to_string(),
                        _ => String::new(),
                    }
                }),
                api_key: config.api_key.clone(),
                models: vec![config.default_model.clone()],
                visibilite: Default::default(), allowed_peers: Vec::new(),
                max_context_length: match config.provider.as_str() {
                    "anthropic" => 200000,
                    "openai" => 128000,
                    _ => 32768,
                },
            },
        );
        profiles_cfg.active_model = profiles::ActiveModel {
            profile_id: migrated_id,
            model: config.default_model.clone(),
        };
        let _ = profiles::save_profiles(&profiles_path, &profiles_cfg);
        info!("Migrated legacy provider config into profiles");
    }

    // Auto-discover local models at startup.
    profiles::refresh_ollama_profiles(&mut profiles_cfg).await;
    profiles::ensure_llamacpp_8001_profile(&mut profiles_cfg).await;

    // Nettoyage des doublons de profils (bug historique de /api/models/use qui créait
    // des "local-<host>" en double + des profils OpenAI à base_url vide).
    {
        // 1) Retirer les profils OpenAI à base_url vide (cassés, ex. faux "local-codex").
        profiles_cfg
            .profiles
            .retain(|_, p| !(p.provider == "openai" && p.base_url.trim().is_empty()));
        // 2) Fusionner les profils (provider, base_url) identiques : garder le 1er (ordre
        //    trié → "llamacpp-8001" avant "local-llama.cpp"), récupérer ses modèles, supprimer.
        let mut ids: Vec<String> = profiles_cfg.profiles.keys().cloned().collect();
        ids.sort();
        let mut seen: std::collections::HashMap<(String, String), String> =
            std::collections::HashMap::new();
        let mut to_remove: Vec<String> = Vec::new();
        for id in ids {
            let (prov, url) = {
                let p = &profiles_cfg.profiles[&id];
                (p.provider.clone(), p.base_url.clone())
            };
            if url.trim().is_empty() {
                continue;
            }
            if let Some(keep) = seen.get(&(prov.clone(), url.clone())).cloned() {
                let models = profiles_cfg.profiles[&id].models.clone();
                if let Some(kp) = profiles_cfg.profiles.get_mut(&keep) {
                    for m in models {
                        if !kp.models.contains(&m) {
                            kp.models.push(m);
                        }
                    }
                }
                to_remove.push(id);
            } else {
                seen.insert((prov, url), id);
            }
        }
        for id in &to_remove {
            profiles_cfg.profiles.remove(id);
        }
        // 3) Réparer active_model si son profil a été supprimé.
        if !profiles_cfg
            .profiles
            .contains_key(&profiles_cfg.active_model.profile_id)
        {
            let m = profiles_cfg.active_model.model.clone();
            let found = profiles_cfg
                .profiles
                .iter()
                .find(|(_, p)| p.models.contains(&m))
                .map(|(id, _)| id.clone());
            if let Some(id) = found {
                profiles_cfg.active_model.profile_id = id;
            } else {
                let fallback = profiles_cfg
                    .profiles
                    .iter()
                    .find(|(_, p)| !p.models.is_empty())
                    .map(|(id, p)| (id.clone(), p.models[0].clone()));
                if let Some((id, model)) = fallback {
                    profiles_cfg.active_model = profiles::ActiveModel {
                        profile_id: id,
                        model,
                    };
                }
            }
        }
        if !to_remove.is_empty() {
            tracing::info!(
                removed = to_remove.len(),
                "Profils doublons nettoyés au démarrage"
            );
        }
    }

    let _ = profiles::save_profiles(&profiles_path, &profiles_cfg);

    // Derive EssaimConfig from active profile
    let (prof_provider, prof_model, prof_api_key, prof_api_base, prof_ollama_url, prof_max_context_len) =
        profiles::active_to_essaim_fields(&profiles_cfg);

    let cron_arc = Arc::new(RwLock::new(CronScheduler::new(std::path::Path::new(
        "cron-tasks.json",
    ))));
    let watchers_arc = Arc::new(RwLock::new(laruche_watchers::WatchersRegistry::new(
        std::path::Path::new("watchers.json"),
    )));
    let kanban_arc = Arc::new(RwLock::new(laruche_kanban::KanbanBoard::new(
        std::path::Path::new("kanban.json"),
    )));
    // Initialize Essaim (agent engine)
    let essaim_registry = Arc::new(AbeilleRegistry::new());
    enregistrer_abeilles_builtin(&essaim_registry);
    // Branche le signer mesh : le chemin d'inférence (laruche-essaim) signe ses appels vers un pair
    // LAN avec l'identité ed25519 de ce nœud → le pair pourra appliquer `restricted`.
    laruche_essaim::providers::set_mesh_signer(std::sync::Arc::new(|path: &str| {
        sync::sign_headers(path)
    }));
    essaim_registry.enregistrer(Box::new(abeilles_local::AbeilleCronCreate {
        cron_store: cron_arc.clone(),
    }));
    essaim_registry.enregistrer(Box::new(abeilles_local::AbeilleCronList {
        cron_store: cron_arc.clone(),
    }));
    essaim_registry.enregistrer(Box::new(abeilles_local::AbeilleCronDelete {
        cron_store: cron_arc.clone(),
    }));
    essaim_registry.enregistrer(Box::new(abeilles_local::AbeilleWatcherCreate {
        watcher_store: watchers_arc.clone(),
    }));
    essaim_registry.enregistrer(Box::new(abeilles_local::AbeilleWatcherList {
        watcher_store: watchers_arc.clone(),
    }));
    essaim_registry.enregistrer(Box::new(abeilles_local::AbeilleWatcherDelete {
        watcher_store: watchers_arc.clone(),
    }));
    essaim_registry.enregistrer(Box::new(abeilles_local::AbeilleKanbanCreate {
        kanban_board: kanban_arc.clone(),
    }));
    essaim_registry.enregistrer(Box::new(abeilles_local::AbeilleKanbanList {
        kanban_board: kanban_arc.clone(),
    }));
    let mut essaim_config = EssaimConfig {
        ollama_url: prof_ollama_url,
        model: prof_model,
        provider: prof_provider,
        api_key: prof_api_key,
        api_base: prof_api_base,
        context_max_tokens: prof_max_context_len,
        disabled_tools: persistent.disabled_tools.clone(),
        disabled_skills: persistent.disabled_skills.clone(),
        ..EssaimConfig::default()
    };
    if let Some(max) = persistent.context_max_messages {
        essaim_config.context_max_messages = max;
    }
    if let Some(tok) = persistent.context_max_tokens {
        essaim_config.context_max_tokens = tok;
    }
    if let Some(th) = persistent.compaction_threshold {
        essaim_config.compaction_threshold = th;
    }
    if let Some(ref m) = persistent.permission_mode {
        if let Some(mode) = permission_mode_from_str(m) {
            essaim_config.permission_mode = mode;
        }
    }

    // Create a sub-registry for delegation (contains all tools except delegate itself)
    let sub_registry = Arc::new({
        let mut r = AbeilleRegistry::new();
        enregistrer_abeilles_builtin(&mut r);
        r
    });
    enregistrer_delegation(&essaim_registry, sub_registry, essaim_config.clone());

    // Mémoire cognitive (laruche-memoire) — backend sélectionnable par env.
    //   LARUCHE_MEMOIRE_BACKEND=sidecar  → vrai paradigm sur http://127.0.0.1:8765
    //   (défaut)                          → NativeBackend Rust en mémoire (zéro dépendance)
    let memoire: Arc<dyn laruche_memoire::MemoireCognitive> =
        match std::env::var("LARUCHE_MEMOIRE_BACKEND").as_deref() {
            Ok("sidecar") => Arc::new(laruche_memoire::SidecarBackend::loopback()),
            Ok("sqlite") => match std::env::var("LARUCHE_EMBED_URL") {
                // Avec embedder Ollama → recall sémantique hybride.
                Ok(url) if !url.is_empty() => {
                    let model = std::env::var("LARUCHE_EMBED_MODEL")
                        .unwrap_or_else(|_| "nomic-embed-text".to_string());
                    Arc::new(
                        laruche_memoire::SqliteBackend::open_with_embedder(
                            "memoire.db",
                            Arc::new(laruche_memoire::OllamaEmbedder::new(url, model)),
                        )
                        .expect("ouverture memoire.db (SQLite+FTS5+embeddings)"),
                    )
                }
                // Sans embedder → recall lexical FTS5.
                _ => Arc::new(
                    laruche_memoire::SqliteBackend::open("memoire.db")
                        .expect("ouverture memoire.db (SQLite+FTS5)"),
                ),
            },
            _ => Arc::new(laruche_memoire::NativeBackend::new()),
        };
    laruche_essaim::abeilles::enregistrer_memoire(&essaim_registry, memoire.clone());
    // Consolidation LLM (fusion d'items) : nécessite mémoire + config (modèle aux).
    essaim_registry.enregistrer(Box::new(
        laruche_essaim::abeilles::memoire::MemoireConsolidate {
            mem: memoire.clone(),
            config: essaim_config.clone(),
        },
    ));

    // Load dynamic plugins from plugins/ directory
    charger_plugins(std::path::Path::new("plugins"), &essaim_registry);
    essaim_registry.enregistrer(Box::new(
        laruche_essaim::abeilles::reload_plugins::ReloadPluginsTool {
            registry: essaim_registry.clone(),
        },
    ));
    // Outils d'AUTO-AMELIORATION (forge) : skill_file_*, plugin_*, mcp_*. Le registre principal
    // est passe pour que plugin_create/delete rechargent au bon endroit.
    laruche_essaim::abeilles::enregistrer_forge(&essaim_registry, essaim_registry.clone());
    essaim_registry.enregistrer(Box::new(abeilles_local::AbeilleMeshSend));

    // Migration `tools.* → capacities.*` (idempotente, jouée à chaque boot mais no-op après).
    // Les skills forgés (vraies données) sont PRÉSERVÉS ; tools.abeilles (simple projection)
    // est purgé puis recréé par l'indexeur sous capacities.tools/plugins/mcp.
    match memoire
        .renommer_sous_arbre("tools.skills", "capacities.skills")
        .await
    {
        Ok(n) if n > 0 => tracing::info!(noeuds = n, "migration skills -> capacities.skills"),
        Ok(_) => {}
        Err(e) => tracing::warn!(error = %e, "migration skills ignoree (backend sans support)"),
    }
    let _ = memoire.supprimer_sous_arbre("tools").await; // purge la projection legacy restante

    // Nœuds de la carte (fichiers .md virtuels). Créés vides si absents (idempotent).
    // `capacities.*` = écosystème d'outils (protégé) ; `system.*` = socle prompt/SOUL éditable.
    for (id, label, desc) in [
        (
            "capacities",
            "Capacites",
            "Ecosysteme : outils, plugins, MCP, skills",
        ),
        ("capacities.tools", "Outils", "Abeilles natives (builtin)"),
        (
            "capacities.plugins",
            "Plugins",
            "Outils custom (plugins JSON)",
        ),
        (
            "capacities.mcp",
            "MCP",
            "Outils servis par des serveurs MCP",
        ),
        ("capacities.skills", "Skills", "Procedures OKF apprises"),
        (
            "system",
            "Systeme",
            "Sections editables du system prompt (hot-reload, sans redemarrage)",
        ),
        (
            "system.prompt",
            "Identite",
            "Identite / persona editable (vide = defaut code)",
        ),
        (
            "system.behavior",
            "Comportement",
            "Regles de comportement editables (vide = defaut code)",
        ),
        (
            "system.soul",
            "SOUL",
            "Couche de personnalisation injectable (frontmatter enabled)",
        ),
    ] {
        let _ = memoire
            .create_node(id, label, Some(desc), Some(1.0), None)
            .await;
    }

    // Skill par défaut « web_research » (procédure search→évalue→fetch→synthèse) — seedé une
    // seule fois s'il est absent, pour que la recherche web aille au-delà des snippets.
    {
        // Marqueur de version : re-seed une fois si l'ancienne version (v1) est en place.
        let present = memoire
            .read_node("capacities.skills.web_research")
            .await
            .ok()
            .and_then(|n| {
                n.get("items").and_then(|i| i.as_array()).map(|a| {
                    a.iter().any(|it| {
                        it.get("content")
                            .and_then(|c| c.as_str())
                            .map(|c| c.contains("web_research-v2"))
                            .unwrap_or(false)
                    })
                })
            })
            .unwrap_or(false);
        if !present {
            let skill = "---\ntype: skill\nname: web_research\nversion: web_research-v2\ndescription: Recherche web approfondie multi-etapes (cherche, evalue, FETCH les pages, synthetise avec sources)\n---\n\n# Recherche web approfondie\n\n## Quand l'utiliser\nToute demande d'infos a jour, factuelles ou detaillees sur le web (actu, papiers, docs, comparatifs, scores, prix...).\n\n## Procedure (NE PAS boucler sur la recherche)\n1. UNE recherche large : `web_deep_search` avec une requete precise.\n2. REPERE dans les resultats les URLs fiables et NON bloquees (arxiv.org, blogs, docs officielles). Ignore les domaines qui renvoient 400/403/Forbidden.\n3. APPROFONDIS : `web_fetch` sur 1 a 3 de ces URLs pour lire la PAGE COMPLETE — c'est la qu'est le detail, pas dans les snippets.\n4. Si une info cle manque : UNE recherche affinee DIFFERENTE (jamais la meme requete), puis re-fetch.\n5. SYNTHETISE en citant les URLs sources. Signale incertitudes/contradictions.\n\n## Regles strictes\n- Maximum ~2 web_deep_search ; au-dela, passe a `web_fetch` sur des URLs precises.\n- Ne relance JAMAIS une requete quasi-identique a la precedente.\n- Une page renvoie 400/403/Forbidden -> abandonne-la, n'insiste pas dessus.\n- Toujours `web_fetch` au moins une source primaire (arxiv, site officiel) avant de conclure.\n- Memorise (memory_write) un fait durable utile si pertinent.\n";
            let _ = memoire
                .write(
                    laruche_memoire::MemoryItem::new("capacities.skills.web_research", skill)
                        .with_source("seed"),
                )
                .await;
        }
    }

    // Indexe le registre d'outils dans la carte (capacities.*) DÈS le démarrage — incrémental —
    // pour que tout nouvel outil soit visible en mémoire et récupérable sémantiquement.
    // (Les outils MCP, chargés plus bas, seront indexés au 1er tour de chat via le même appel.)
    if let Err(e) =
        laruche_essaim::brain::indexer_abeilles_memoire(&essaim_registry, &memoire).await
    {
        tracing::warn!(error = %e, "indexation outils au demarrage ignoree");
    }

    // Phase 1 — couche flat-file : sync disque → SQL des skills (skills/<slug>/SKILL.md).
    sync_skills_disk_to_sql(&memoire).await;

    // Load MCP servers
    let (_count, mcp_clients) =
        charger_mcp_servers(std::path::Path::new("mcp_servers.json"), &essaim_registry).await;
    let mcp_clients = Arc::new(mcp_clients);
    essaim_registry.enregistrer(Box::new(
        laruche_essaim::abeilles::mcp_resources::McpListResources {
            clients: mcp_clients.clone(),
        },
    ));
    essaim_registry.enregistrer(Box::new(
        laruche_essaim::abeilles::mcp_resources::McpReadResource {
            clients: mcp_clients.clone(),
        },
    ));

    // Initialize RAG knowledge base
    let kb = Arc::new(tokio::sync::RwLock::new(
        laruche_essaim::rag::KnowledgeBase::new(
            std::path::Path::new("knowledge-base.json"),
            &config.ollama_url,
            "nomic-embed-text", // Default embedding model — user should pull it
        ),
    ));
    // Fix A — knowledge_add/knowledge_search RETIRÉS : c'était un 2e système de mémoire
    // (KnowledgeBase plate/RAG) en DOUBLON de la carte cognitive. Tout passe désormais par
    // memory_write / memory_search (la mémoire cognitive = le différenciateur de LaRuche).
    let _ = &kb; // kb conservé pour rag.rs (RAG hérité), mais plus exposé comme outil agent.

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

    let sessions_arc = Arc::new(RwLock::new(loaded_sessions));
    essaim_registry.enregistrer(Box::new(abeilles_local::AbeilleSessionSearch {
        sessions_store: sessions_arc.clone(),
    }));

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

    // Load or create CredentialPool
    let credentials_path = std::path::PathBuf::from("credentials.json");
    let pool_data = if credentials_path.exists() {
        std::fs::read_to_string(&credentials_path)
            .ok()
            .and_then(|data| {
                serde_json::from_str::<laruche_essaim::credential_pool::CredentialPool>(&data).ok()
            })
            .unwrap_or_else(|| laruche_essaim::credential_pool::CredentialPool::default())
    } else {
        laruche_essaim::credential_pool::CredentialPool::default()
    };
    let credential_pool = Arc::new(RwLock::new(pool_data));

    let state = Arc::new(AppState {
        manifest: RwLock::new(manifest),
        auth: RwLock::new(ProximityAuth::new()),
        queue: RwLock::new(RequestQueue::new(QosPolicy::default())),
        listener: RwLock::new(listener),
        default_models: RwLock::new(initial_defaults),
        custom_services: RwLock::new(HashMap::new()),
        capability_selection: RwLock::new(
            persistent.capability_selection.clone().unwrap_or_default(),
        ),
        missions: RwLock::new(missions::MissionStore::new(std::path::Path::new(
            "missions.json",
        ))),
        config: config.clone(),
        sys: RwLock::new(sys),
        activity_log: RwLock::new(initial_log),
        state_file_path,
        metrics_history: RwLock::new(VecDeque::with_capacity(METRICS_HISTORY_LIMIT)),
        node_events: RwLock::new(VecDeque::with_capacity(NODE_EVENTS_LIMIT)),
        known_node_ids: RwLock::new(HashSet::new()),
        essaim_registry: essaim_registry.clone(),
        essaim_config: RwLock::new({
            essaim_config.credential_pool = Some(credential_pool.clone());
            essaim_config
        }),
        memoire,
        essaim_sessions: sessions_arc.clone(),
        active_context_stats: Arc::new(RwLock::new(HashMap::new())),
        essaim_cron: cron_arc.clone(),
        watchers: watchers_arc.clone(),
        kanban_board: kanban_arc.clone(),
        essaim_kb: kb.clone(),
        events: Arc::new(RwLock::new(laruche_events::EventBus::new())),
        channel_handles: RwLock::new(HashMap::new()),
        profiles: RwLock::new(profiles_cfg),
        profiles_path,
        users: RwLock::new(loaded_users),
        auth_challenges: RwLock::new(HashMap::new()),
        cookie_secret,
        credential_pool: credential_pool.clone(),
        credentials_path,
        last_activity: RwLock::new(std::time::Instant::now()),
    });

    let app = Router::new()
        .route("/", get(spa_page))
        .route("/api/status", get(get_status))
        .route(
            "/api/blueprints",
            get(get_blueprints).post(api_create_blueprint),
        )
        .route(
            "/api/blueprints/:id",
            axum::routing::delete(api_delete_blueprint),
        )
        .route("/api/blueprints/:id/instancier", post(instancier_blueprint))
        .route("/api/events", get(api_get_events))
        .route("/api/events/export", get(api_export_events))
        .route("/health", get(health))
        .route("/nodes", get(get_nodes))
        .route("/swarm", get(get_swarm))
        .route("/swarm/models", get(get_swarm_models))
        .route("/models", get(get_models))
        .route("/activity", get(get_activity))
        .route("/infer", post(post_infer))
        .route("/v1/chat/completions", post(api_v1_chat_completions))
        .route("/auth/request", post(post_auth_request))
        .route("/auth/approve", post(post_auth_approve))
        .route(
            "/config/default_model",
            get(get_default_model).post(post_set_default_model),
        )
        .route("/metrics/history", get(get_metrics_history))
        .route("/dashboard", get(spa_page))
        .route("/chat", get(spa_page))
        .route("/control", get(spa_page))
        .route("/app", get(spa_page))
        .route("/ws/chat", get(ws_chat_handler))
        .route("/ws/audio", get(ws_audio_handler))
        .route("/api/tools", get(api_list_tools))
        .route(
            "/api/tools/config",
            get(api_get_tools_config).post(api_save_tools_config),
        )
        .route("/api/memory/search", get(api_memory_search))
        .route("/api/memory/node/:id", get(api_memory_node))
        .route("/api/memory/suggest", get(api_memory_suggest))
        .route("/api/memory/proposed", get(api_memory_proposed))
        .route("/api/memory/write", post(api_memory_write))
        .route("/api/memory/enrich", post(api_memory_enrich))
        .route("/api/memory/update", post(api_memory_update))
        .route("/api/memory/delete", post(api_memory_delete))
        .route("/api/memory/node/create", post(api_memory_node_create))
        .route("/api/memory/node/update", post(api_memory_node_update))
        .route("/api/memory/node/move", post(api_memory_node_move))
        .route("/api/memory/node/delete", post(api_memory_node_delete))
        .route("/api/memory/move", post(api_memory_move))
        .route("/api/memory/review", post(api_memory_review))
        .route("/api/memory/dream", post(api_memory_dream))
        .route("/api/memory/consolidate", post(api_memory_consolidate))
        .route("/api/feed", get(api_feed))
        .route("/api/feed/ask", post(api_feed_ask))
        .route("/api/mesh/whoami", get(api_mesh_whoami))
        .route("/api/mesh/identity", get(api_mesh_identity))
        .route("/api/mesh/code", get(api_mesh_code_get).post(api_mesh_code_set))
        .route("/api/mesh/peers", get(api_mesh_peers))
        .route("/api/mesh/send", post(api_mesh_send))
        .route("/api/mesh/receive", post(api_mesh_receive))
        .route("/api/inbox", get(api_inbox_get))
        .route("/api/inbox/read", post(api_inbox_read))
        .route("/api/profile", get(api_profile_get).post(api_profile_save))
        .route("/api/memory/grep", get(api_memory_grep))
        .route("/api/memory/export_changes", get(api_memory_export_changes))
        .route("/api/memory/import_changes", post(api_memory_import_changes))
        .route("/api/memory/mesh_pull", post(api_memory_mesh_pull))
        .route("/api/state/version", get(api_state_version))
        .route("/api/memory/tree", get(api_memory_tree))
        .route(
            "/api/system/prompt-defaults",
            get(api_system_prompt_defaults),
        )
        .route("/api/memory/stats", get(api_memory_stats))
        .route("/api/memory/mutations", get(api_memory_mutations))
        .route("/api/memory/export_okf", get(api_memory_export_okf))
        .route("/api/memory/export.zip", get(api_memory_export_zip))
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
        .route("/api/media/local", get(api_media_local))
        .route(
            "/api/config/channels",
            get(api_get_channels_config).post(api_save_channels_config),
        )
        .route(
            "/api/config/notify",
            get(api_get_notify_config).post(api_set_notify_config),
        )
        .route(
            "/api/config/provider",
            get(api_get_provider_config).post(api_save_provider_config),
        )
        .route("/api/context/stats", get(api_get_context_stats))
        .route(
            "/api/config/compaction",
            get(api_get_compaction_config).post(api_set_compaction_config),
        )
        .route(
            "/api/config/permission",
            get(api_get_permission_config).post(api_set_permission_config),
        )
        .route(
            "/api/profiles",
            get(api_get_profiles).post(api_upsert_profile),
        )
        .route(
            "/api/credentials",
            get(api_get_credentials)
                .post(api_add_credential)
                .delete(api_delete_credential),
        )
        .route("/api/profiles/models", get(api_get_unified_models))
        .route("/api/profiles/active", post(api_set_active_model))
        .route("/api/profiles/:id/visibility", post(api_set_visibility))
        .route("/api/models/use", post(api_models_use))
        .route(
            "/api/capabilities/selection",
            get(api_capabilities_selection),
        )
        .route(
            "/api/missions",
            get(api_list_missions).post(api_create_mission),
        )
        .route("/api/missions/:slug/run", post(api_run_mission))
        .route("/api/missions/:slug/dossier", get(api_mission_dossier))
        .route("/api/missions/:slug/decompose", post(api_decompose_mission))
        .route(
            "/api/missions/:slug",
            post(api_update_mission).delete(api_delete_mission),
        )
        .route(
            "/api/profiles/:id",
            axum::routing::delete(api_delete_profile),
        )
        .route("/api/services/register", post(api_register_service))
        .route(
            "/api/services/register/:name",
            axum::routing::delete(api_unregister_service),
        )
        .route("/api/auth/codex/status", get(api_codex_status))
        .route("/api/auth/codex/start", post(api_codex_start))
        .route("/api/auth/codex/logout", post(api_codex_logout))
        .route("/api/channels/start", post(api_start_channel))
        .route("/api/channels/stop", post(api_stop_channel))
        .route("/api/channels/status", get(api_channels_status))
        .route(
            "/api/knowledge",
            get(api_list_knowledge).post(api_add_knowledge),
        )
        .route(
            "/api/knowledge/:id",
            axum::routing::delete(api_delete_knowledge).put(api_update_knowledge),
        )
        .route("/api/doctor", get(api_doctor))
        .route("/api/sessions/:id/export", get(api_export_session))
        .route("/api/sessions/:id/fork", post(api_fork_session))
        .route(
            "/api/sessions/:id",
            axum::routing::delete(api_delete_session),
        )
        .route("/api/agents/spawn", post(api_spawn_subagent))
        .route("/api/cron", get(api_list_cron).post(api_create_cron))
        .route(
            "/api/cron/:id",
            axum::routing::delete(api_delete_cron).put(api_update_cron),
        )
        .route("/api/cron/:id/run", post(api_run_cron))
        .route("/api/skills", get(api_list_skills).post(api_upsert_skill))
        .route(
            "/api/skills/:name",
            get(api_get_skill).delete(api_delete_skill),
        )
        .route("/api/skills/:name/toggle", post(api_toggle_skill))
        .route(
            "/api/watchers",
            get(api_list_watchers).post(api_create_watcher),
        )
        .route(
            "/api/watchers/:id",
            axum::routing::delete(api_delete_watcher),
        )
        .route("/api/kanban", get(api_kanban_list).post(api_kanban_create))
        .route(
            "/api/kanban/:id",
            axum::routing::delete(api_kanban_delete).put(api_kanban_update),
        )
        .route(
            "/api/kanban/:id/status",
            axum::routing::put(api_kanban_update_status),
        )
        .route(
            "/api/kanban/:id/dependency",
            post(api_kanban_add_dependency),
        )
        .route("/api/memory/import_okf", post(api_memory_import_okf))
        .route("/api/mcp", post(mcp::api_mcp_handler))
        .route("/api/mcp/servers", get(api_mcp_list_servers))
        .route(
            "/api/mcp/servers/:name",
            post(api_mcp_save_server).delete(api_mcp_delete_server),
        )
        .route(
            "/api/plugins/:name",
            get(api_plugin_get)
                .post(api_plugin_save)
                .delete(api_plugin_delete),
        )
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
        .route(
            "/api/internal/sync/session",
            post(sync::handle_session_sync),
        )
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
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(
            MDNS_REANNOUNCE_INTERVAL_SECS,
        ));
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
                        .args([
                            "--query-gpu=utilization.gpu,memory.used,memory.total,temperature.gpu",
                            "--format=csv,noheader,nounits",
                        ])
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
                let ram_pct = if total_mem > 0 {
                    (used_mem as f32 / total_mem as f32) * 100.0
                } else {
                    0.0
                };

                // Count nodes from listener
                let listener = update_state.listener.read().await;
                let nodes = listener.get_nodes().await;
                let node_count = nodes.len() + 1; // +1 for self

                let gpu_pct = manifest.resources.accelerator_usage_pct;
                let vram_pct = match (
                    manifest.resources.vram_used_mb,
                    manifest.resources.vram_total_mb,
                ) {
                    (Some(used), Some(total)) if total > 0 => {
                        Some((used as f32 / total as f32) * 100.0)
                    }
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
                        let name = node
                            .manifest
                            .node_name
                            .clone()
                            .unwrap_or_else(|| id.clone());
                        let mut events = update_state.node_events.write().await;
                        if events.len() >= NODE_EVENTS_LIMIT {
                            events.pop_front();
                        }
                        events.push_back(NodeEvent {
                            epoch_ms: now_ms,
                            event_type: "connected".into(),
                            node_name: name,
                        });
                        // Bulk sync from new peer
                        let peer_host = node.manifest.host.clone();
                        let peer_port = node
                            .manifest
                            .port
                            .unwrap_or(miel_protocol::DEFAULT_API_PORT);
                        let sync_state = update_state.clone();
                        tokio::spawn(async move {
                            sync::fetch_bulk_from_peer(&peer_host, peer_port, &sync_state).await;
                        });
                    }
                }
                // Removed nodes (disconnected)
                for id in known.difference(&current_ids) {
                    let mut events = update_state.node_events.write().await;
                    if events.len() >= NODE_EVENTS_LIMIT {
                        events.pop_front();
                    }
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
            let url = format!(
                "{}/api/tags",
                heartbeat_state.essaim_config.read().await.ollama_url
            );
            match client.get(&url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    if was_down {
                        info!("Ollama heartbeat: recovered (back online)");
                        let mut activity = heartbeat_state.activity_log.write().await;
                        if activity.len() >= ACTIVITY_LOG_LIMIT {
                            activity.pop_front();
                        }
                        activity.push_back(ActivityLogEntry {
                            timestamp: chrono::Utc::now().to_rfc3339(),
                            level: "info".into(),
                            tag: "heartbeat".into(),
                            message: "Ollama recovered".into(),
                            full_prompt: None,
                            full_response: None,
                            model_used: None,
                            tokens_generated: None,
                            latency_ms: None,
                            user_id: None,
                        });
                        was_down = false;
                    }
                }
                _ => {
                    if !was_down {
                        let profiles = heartbeat_state.profiles.read().await;
                        let has_ollama = profiles.profiles.values().any(|p| p.provider == "ollama");
                        drop(profiles);
                        if !has_ollama {
                            was_down = true;
                            continue;
                        }

                        warn!("Ollama heartbeat: DOWN (not responding)");
                        let mut activity = heartbeat_state.activity_log.write().await;
                        if activity.len() >= ACTIVITY_LOG_LIMIT {
                            activity.pop_front();
                        }
                        activity.push_back(ActivityLogEntry {
                            timestamp: chrono::Utc::now().to_rfc3339(),
                            level: "error".into(),
                            tag: "heartbeat".into(),
                            message: "Ollama is not responding".into(),
                            full_prompt: None,
                            full_response: None,
                            model_used: None,
                            tokens_generated: None,
                            latency_ms: None,
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
                let due = cron.check_due_tasks();
                due.into_iter()
                    .map(|(id, prompt)| {
                        let mut channel = None;
                        let mut provider = None;
                        let mut model = None;
                        let mut profile_id = None;
                        let mut skills = Vec::new();
                        for t in cron.list() {
                            if t.id == id {
                                channel = t.channel.clone();
                                provider = t.provider.clone();
                                model = t.model.clone();
                                profile_id = t.profile_id.clone();
                                skills = t.skills.clone();
                                break;
                            }
                        }
                        (id, prompt, channel, provider, model, profile_id, skills)
                    })
                    .collect::<Vec<_>>()
            };
            for (task_id, prompt, channel, provider, model, profile_id, skills) in due_tasks {
                info!(task_id = %task_id, "Executing scheduled task");
                let _ = cron_state.events.write().await.emit(
                    laruche_events::EventKind::AgentStarted,
                    "cron_dispatcher",
                    serde_json::json!({ "task_id": task_id, "prompt": prompt }),
                );

                let mut cron_config = cron_state.essaim_config.read().await.clone();
                if let Some(pid) = profile_id {
                    // Résolution complète via le profil (provider + clé + base_url + modèle).
                    appliquer_profil(&cron_state, &mut cron_config, &pid, model.as_deref()).await;
                } else if let Some(p) = provider {
                    // Fallback hérité : provider/modèle bruts (clé/URL du config actif).
                    cron_config.provider = p;
                    if let Some(m) = model {
                        cron_config.model = m;
                    } else {
                        cron_config.model = get_llm_default(&cron_state).await;
                    }
                } else if let Some(m) = model {
                    cron_config.model = m;
                } else {
                    cron_config.model = get_llm_default(&cron_state).await;
                }

                let current_model = cron_config.model.clone();
                let sessions_dir = std::path::Path::new("sessions");
                let mut session = Session::new_with_path(&current_model, sessions_dir);
                let (tx, mut rx) = broadcast::channel::<ChatEvent>(64);

                // Ne pas jeter le receiver (drain)
                tokio::spawn(async move { while let Ok(_) = rx.recv().await {} });

                // Lot 10.B — injection des skills attachés : charge chaque SKILL.md OKF
                // depuis capacities.skills.<nom> et l'assemble en tête du prompt (skills
                // désactivés via le slider de la page Skills = sautés).
                let disabled_sk = cron_config.disabled_skills.clone();
                let mut skills_charges: Vec<(String, String)> = Vec::new();
                for skill_name in skills.iter().filter(|s| !disabled_sk.contains(s)) {
                    let node_id = laruche_skills::skill_node_id(skill_name);
                    if let Ok(node) = cron_state.memoire.read_node(&node_id).await {
                        if let Some(items) = node["items"].as_array() {
                            if let Some(body) = items.iter().rev().find_map(|it| {
                                it["content"].as_str().filter(|c| c.contains("type: skill"))
                            }) {
                                skills_charges.push((skill_name.clone(), body.to_string()));
                            }
                        }
                    }
                }
                let prompt = laruche_essaim::orchestration::assembler_prompt_skills(
                    &prompt,
                    &skills_charges,
                );

                let result = boucle_react_memoire(
                    &prompt,
                    &mut session,
                    &cron_state.essaim_registry,
                    &cron_config,
                    &tx,
                    cron_state.memoire.clone(),
                )
                .await;

                match &result {
                    Ok(response) => {
                        info!(task_id = %task_id, response_len = response.len(), "Scheduled task completed");
                    }
                    Err(e) => {
                        warn!(task_id = %task_id, error = %e, "Scheduled task failed");
                    }
                }

                if let Some(ch) = channel.filter(|s| !s.is_empty()) {
                    if ch.starts_with("telegram") {
                        let chat_id = ch.strip_prefix("telegram:").unwrap_or("").trim();
                        let config_path = std::path::Path::new("channels-config.json");
                        if let Ok(content) = std::fs::read_to_string(config_path) {
                            if let Ok(config) = serde_json::from_str::<serde_json::Value>(&content)
                            {
                                let token = config["telegram"]["bot_token"].as_str().unwrap_or("");
                                let target_chat = if chat_id.is_empty() {
                                    let chats_str =
                                        config["telegram"]["allowed_chats"].as_str().unwrap_or("");
                                    chats_str.split(',').next().unwrap_or("").trim()
                                } else {
                                    chat_id
                                };
                                if !token.is_empty() && !target_chat.is_empty() {
                                    let msg = match &result {
                                        Ok(r) => format!("🤖 *Cron Task*\n\n{}", r),
                                        Err(e) => format!("❌ *Cron Failed*\n\n{}", e),
                                    };
                                    let client = reqwest::Client::new();
                                    let _ = client
                                        .post(&format!(
                                            "https://api.telegram.org/bot{}/sendMessage",
                                            token
                                        ))
                                        .json(&serde_json::json!({
                                            "chat_id": target_chat,
                                            "text": msg,
                                            "parse_mode": "Markdown"
                                        }))
                                        .send()
                                        .await;
                                }
                            }
                        }
                    }
                } else {
                    // Log to activity
                    let now = chrono::Utc::now().to_rfc3339();
                    let mut activity = cron_state.activity_log.write().await;
                    if activity.len() >= ACTIVITY_LOG_LIMIT {
                        activity.pop_front();
                    }
                    activity.push_back(ActivityLogEntry {
                        timestamp: now,
                        level: if result.is_ok() { "info" } else { "error" }.into(),
                        tag: "cron".into(),
                        message: format!("Cron task: {}", preview_text(&prompt, 60)),
                        full_prompt: Some(prompt),
                        full_response: result.ok().map(|r| preview_text(&r, 4000)),
                        model_used: Some(cron_config.model.clone()),
                        tokens_generated: None,
                        latency_ms: None,
                        user_id: None,
                    });
                }
            }
        }
    });

    // Background: Watchers task checker (every 10 seconds)
    let watcher_state = state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(10));
        loop {
            interval.tick().await;
            let triggered = {
                let mut registry = watcher_state.watchers.write().await;
                registry.check_triggered_watchers().await
            };
            for (watcher_id, prompt, context) in triggered {
                info!(watcher_id = %watcher_id, "Executing watcher task");
                let _ = watcher_state.events.write().await.emit(
                    laruche_events::EventKind::WatcherFired,
                    "watcher_dispatcher",
                    serde_json::json!({ "watcher_id": watcher_id, "prompt": prompt, "context": context })
                );
                let current_model = get_llm_default(&watcher_state).await;
                let sessions_dir = std::path::Path::new("sessions");
                let mut session = Session::new_with_path(&current_model, sessions_dir);
                let (tx, _rx) = broadcast::channel::<ChatEvent>(64);
                let (w_profile, w_model) = {
                    let reg = watcher_state.watchers.read().await;
                    reg.list()
                        .into_iter()
                        .find(|w| w.id == watcher_id)
                        .map(|w| (w.profile_id.clone(), w.model.clone()))
                        .unwrap_or((None, None))
                };
                let mut config = watcher_state.essaim_config.read().await.clone();
                if let Some(pid) = w_profile {
                    appliquer_profil(&watcher_state, &mut config, &pid, w_model.as_deref()).await;
                } else {
                    config.model = current_model;
                }

                let full_prompt = format!("[CONTEXT: {}]\n\n{}", context, prompt);
                let result = boucle_react_memoire(
                    &full_prompt,
                    &mut session,
                    &watcher_state.essaim_registry,
                    &config,
                    &tx,
                    watcher_state.memoire.clone(),
                )
                .await;

                let now = chrono::Utc::now().to_rfc3339();
                let mut activity = watcher_state.activity_log.write().await;
                if activity.len() >= ACTIVITY_LOG_LIMIT {
                    activity.pop_front();
                }
                activity.push_back(ActivityLogEntry {
                    timestamp: now,
                    level: if result.is_ok() { "info" } else { "error" }.into(),
                    tag: "watcher".into(),
                    message: format!("Watcher task: {}", preview_text(&prompt, 60)),
                    full_prompt: Some(full_prompt),
                    full_response: result.ok().map(|r| preview_text(&r, 4000)),
                    model_used: Some(config.model.clone()),
                    tokens_generated: None,
                    latency_ms: None,
                    user_id: None,
                });
            }
        }
    });

    // Background: re-annonce mDNS périodique (P4) — reflète les modèles RÉELS (actif +
    // backends locaux détectés + providers public_proxy), capte les backends lancés à chaud,
    // et corrige l'annonce du modèle par défaut figé.
    let mdns_broadcaster = broadcaster.clone();
    let mdns_state = state.clone();
    tokio::spawn(async move {
        // Ré-annonce toutes les 30s (< PEER_STALE_SECS=90) → présence stable, fini le flapping.
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
        interval.tick().await; // saute le tick immédiat
        loop {
            interval.tick().await;
            let mut manifest = mdns_state.manifest.read().await.clone();
            manifest.capabilities = Default::default();
            // CONFIDENTIALITÉ MESH : on n'annonce QUE ce qui est EXPLICITEMENT public (providers
            // `public_proxy`). On n'auto-annonce PLUS les backends locaux détectés (fuite : un pair
            // voyait tous tes llama.cpp), et le modèle de l'Agent n'est divulgué que s'il est public.
            let public_models: std::collections::HashSet<String> = {
                let pcfg = mdns_state.profiles.read().await;
                pcfg.profiles
                    .iter()
                    .filter(|(_, p)| p.visibilite == profiles::Visibilite::PublicProxy)
                    .flat_map(|(_, p)| p.models.iter().cloned())
                    .collect()
            };
            // Agent = présence d'un agent dans l'essaim. Nom du modèle masqué si non public.
            let active_model = get_llm_default(&mdns_state).await;
            let agent_model = if public_models.contains(&active_model) {
                active_model
            } else {
                "(privé)".to_string()
            };
            manifest.capabilities.add(CapabilityInfo {
                capability: Capability::Agent,
                model_name: agent_model,
                model_size: None,
                quantization: None,
                max_context_length: Some(8192),
            });
            // Providers public_proxy ET restricted → annoncés (passerelle ; clé jamais diffusée).
            // Les restricted sont visibles (les pairs autorisés doivent les découvrir) mais l'accès
            // est contrôlé à l'usage (P3 vérifie l'identité de l'appelant contre allowed_peers).
            {
                let pcfg = mdns_state.profiles.read().await;
                for (_, p) in pcfg
                    .profiles
                    .iter()
                    .filter(|(_, p)| p.visibilite != profiles::Visibilite::Prive)
                {
                    for model in &p.models {
                        let cap = resolve_model_capability(model, &mdns_state.config.capabilities);
                        if let Some(c) = Capability::from_flag(&cap) {
                            manifest.capabilities.add(CapabilityInfo {
                                capability: c,
                                model_name: model.clone(),
                                model_size: None,
                                quantization: None,
                                max_context_length: Some(8192),
                            });
                        }
                    }
                }
            }
            {
                let mut m = mdns_state.manifest.write().await;
                *m = manifest.clone();
            }
            if let Err(e) = mdns_broadcaster.update(&manifest) {
                tracing::warn!(error = %e, "re-annonce mDNS échouée");
            }
        }
    });

    // Background: tick des MISSIONS au long cours (« La Reine ») — toutes les 60s, lance une
    // itération des missions actives dont la cadence cron est due (ex. recherche hebdo).
    let mission_state = state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        interval.tick().await; // saute le tick immédiat
        loop {
            interval.tick().await;
            let now = chrono::Utc::now();
            let dues: Vec<missions::Mission> = {
                let store = mission_state.missions.read().await;
                store
                    .list()
                    .into_iter()
                    .filter(|m| {
                        m.status == "active"
                            && m.cadence.as_deref().is_some_and(|c| {
                                let last = m
                                    .last_run
                                    .as_deref()
                                    .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                                    .map(|d| d.with_timezone(&chrono::Utc));
                                laruche_essaim::cron::should_fire_cron(c, last, now)
                            })
                    })
                    .collect()
            };
            for mission in dues {
                tracing::info!(mission = %mission.slug, "Itération de mission (cadence)");
                lancer_iteration_mission(mission_state.clone(), mission).await;
            }
        }
    });

    // Background: Kanban Dispatcher (every 5 seconds)
    let kanban_state = state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
        loop {
            interval.tick().await;

            let task_opt = {
                let mut board = kanban_state.kanban_board.write().await;
                // Board de PLANIFICATION : on n'auto-exécute QUE les tâches
                // explicitement promues en `Ready` (sélection chirurgicale →
                // `Running`). Les `Todo` créées par l'agent/l'utilisateur restent
                // visibles tant qu'elles ne sont pas promises (sinon le daemon les
                // happait toutes en 5 s → board « vide »). Pour lancer une tâche :
                // la glisser dans la colonne « Ready ».
                let ready = board
                    .list()
                    .into_iter()
                    .find(|t| t.status == laruche_kanban::TaskStatus::Ready);
                if let Some(t) = ready {
                    board.change_status(t.id, laruche_kanban::TaskStatus::Running);
                    Some(t)
                } else {
                    None
                }
            };

            if let Some(kanban_task) = task_opt {
                info!(task_id = %kanban_task.id, "Executing Kanban task");
                let _ = kanban_state.events.write().await.emit(
                    laruche_events::EventKind::KanbanTask,
                    "kanban_dispatcher",
                    serde_json::json!({ "task_id": kanban_task.id, "title": kanban_task.title }),
                );
                let current_model = get_llm_default(&kanban_state).await;
                let sessions_dir = std::path::Path::new("sessions");
                let mut session = Session::new_with_path(&current_model, sessions_dir);
                let (tx, _rx) = broadcast::channel::<ChatEvent>(64);
                let mut config = kanban_state.essaim_config.read().await.clone();
                if let Some(pid) = &kanban_task.profile_id {
                    appliquer_profil(
                        &kanban_state,
                        &mut config,
                        pid,
                        kanban_task.model.as_deref(),
                    )
                    .await;
                } else {
                    config.model = current_model;
                }

                let prompt = format!(
                    "[KANBAN TASK: {}]\n{}",
                    kanban_task.title, kanban_task.description
                );
                let result = boucle_react_memoire(
                    &prompt,
                    &mut session,
                    &kanban_state.essaim_registry,
                    &config,
                    &tx,
                    kanban_state.memoire.clone(),
                )
                .await;

                // Update board
                let mut board = kanban_state.kanban_board.write().await;
                match &result {
                    Ok(res) => {
                        board.complete(kanban_task.id, res.clone());
                    }
                    Err(e) => {
                        board.complete(kanban_task.id, format!("ERROR: {}", e));
                        board.change_status(kanban_task.id, laruche_kanban::TaskStatus::Blocked);
                    }
                }

                // Log
                let now = chrono::Utc::now().to_rfc3339();
                let mut activity = kanban_state.activity_log.write().await;
                if activity.len() >= ACTIVITY_LOG_LIMIT {
                    activity.pop_front();
                }
                activity.push_back(ActivityLogEntry {
                    timestamp: now,
                    level: if result.is_ok() { "info" } else { "error" }.into(),
                    tag: "kanban".into(),
                    message: format!("Kanban task: {}", preview_text(&kanban_task.title, 60)),
                    full_prompt: Some(prompt),
                    full_response: result.ok().map(|r| preview_text(&r, 4000)),
                    model_used: Some(config.model.clone()),
                    tokens_generated: None,
                    latency_ms: None,
                    user_id: None,
                });
            }
        }
    });

    // Background: Dream (auto à l'inactivité + background review)
    let dream_state = state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        let mut last_dreamed = std::time::Instant::now();
        loop {
            interval.tick().await;
            let last_activity = *dream_state.last_activity.read().await;
            let idle_duration = last_activity.elapsed();

            if idle_duration > std::time::Duration::from_secs(300) && last_dreamed < last_activity {
                tracing::info!("Système inactif depuis > 5min, déclenchement du mode Dream...");
                let _ = dream_state.events.write().await.emit(
                    laruche_events::EventKind::SystemStatus,
                    "dream_task",
                    serde_json::json!({"status": "dreaming", "idle_secs": idle_duration.as_secs()}),
                );

                let memoire = dream_state.memoire.clone();
                if let Err(e) = memoire.dream().await {
                    tracing::warn!("Erreur lors du dream: {}", e);
                }

                last_dreamed = std::time::Instant::now();

                let _ = dream_state.events.write().await.emit(
                    laruche_events::EventKind::SystemStatus,
                    "dream_task",
                    serde_json::json!({"status": "idle"}),
                );
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

    // L3 (tranche 2) — SYNC mémoire AUTO depuis les nodes pairs (Miel), toutes les 5 min : chaque
    // node tire+dédupe les faits des autres → mémoire COLLECTIVE de la ruche, sans cloud.
    {
        let sync_state = state.clone();
        tokio::spawn(async move {
            let mut last_sync: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(300));
            loop {
                interval.tick().await;
                let peers: Vec<String> = {
                    let l = sync_state.listener.read().await;
                    l.get_nodes()
                        .await
                        .into_iter()
                        .map(|(_, n)| n.manifest.host)
                        .collect()
                };
                for host in peers {
                    if host.trim().is_empty() {
                        continue;
                    }
                    let since = *last_sync.get(&host).unwrap_or(&0);
                    let url = format!("http://{host}:8419/api/memory/export_changes?since={since}");
                    let Ok(resp) = reqwest::get(&url).await else {
                        continue;
                    };
                    let Ok(data) = resp.json::<serde_json::Value>().await else {
                        continue;
                    };
                    let empty: Vec<serde_json::Value> = vec![];
                    let items = data["items"].as_array().unwrap_or(&empty);
                    if items.is_empty() {
                        continue;
                    }
                    let maxts = items
                        .iter()
                        .filter_map(|i| i["ts"].as_i64())
                        .max()
                        .unwrap_or(since);
                    let (imp, _) =
                        importer_changes(&sync_state, items, &format!("mesh:{host}")).await;
                    last_sync.insert(host.clone(), maxts.max(since));
                    if imp > 0 {
                        tracing::info!(peer = %host, imported = imp, "mesh memory auto-sync");
                    }
                }
            }
        });
    }

    // Phase 1.5 — WATCHER live des SKILL.md : un fichier modifié est re-synchronisé vers SQL
    // sans reboot (poll 8s, incrémental par mtime).
    {
        let w_state = state.clone();
        tokio::spawn(async move {
            let mut mtimes: std::collections::HashMap<String, std::time::SystemTime> =
                std::collections::HashMap::new();
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(8));
            let mut first = true;
            loop {
                interval.tick().await;
                let Ok(rd) = std::fs::read_dir("skills") else {
                    continue;
                };
                for e in rd.flatten() {
                    let p = e.path();
                    if !p.is_dir() {
                        continue;
                    }
                    let md = p.join("SKILL.md");
                    let Ok(mt) = std::fs::metadata(&md).and_then(|m| m.modified()) else {
                        continue;
                    };
                    let Some(key) = p.file_name().and_then(|x| x.to_str()).map(String::from) else {
                        continue;
                    };
                    let changed = mtimes.get(&key).map(|prev| *prev != mt).unwrap_or(true);
                    mtimes.insert(key.clone(), mt);
                    if first || !changed {
                        continue; // 1er tour = init ; le boot a déjà tout synchronisé
                    }
                    let Ok(content) = std::fs::read_to_string(&md) else {
                        continue;
                    };
                    let content = content.replace("\r\n", "\n");
                    if !content.contains("type: skill") {
                        continue;
                    }
                    let node_id = format!("capacities.skills.{key}");
                    if let Ok(node) = w_state.memoire.read_node(&node_id).await {
                        if let Some(items) = node.get("items").and_then(|i| i.as_array()) {
                            for it in items {
                                if let Some(id) = it.get("id").and_then(|x| x.as_str()) {
                                    let _ =
                                        w_state.memoire.delete_item(id, Some("skill-file-watch")).await;
                                }
                            }
                        }
                    }
                    let _ = w_state
                        .memoire
                        .write(
                            laruche_memoire::MemoryItem::new(node_id, content)
                                .with_source("skill-file"),
                        )
                        .await;
                    tracing::info!(skill = %key, "skill re-synchronise (watcher SKILL.md)");
                }
                first = false;
            }
        });
    }

    let notifier_state = state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
        let mut last_seen_id = 0;
        loop {
            interval.tick().await;
            let config_path = std::path::Path::new("channels-config.json");
            if !config_path.exists() {
                continue;
            }
            let config: serde_json::Value = match std::fs::read_to_string(config_path) {
                Ok(c) => serde_json::from_str(&c).unwrap_or_default(),
                Err(_) => continue,
            };

            let notify_enabled = config["notify"]["enabled"].as_bool().unwrap_or(false);
            if !notify_enabled {
                let evs = notifier_state.events.read().await.since(last_seen_id);
                if let Some(last) = evs.last() {
                    last_seen_id = last.id;
                }
                continue;
            }

            let evs = notifier_state.events.read().await.since(last_seen_id);
            for ev in evs {
                last_seen_id = last_seen_id.max(ev.id);
                if matches!(
                    ev.kind,
                    laruche_events::EventKind::AgentFinished
                        | laruche_events::EventKind::WatcherFired
                ) {
                    let token = config["telegram"]["bot_token"].as_str().unwrap_or("");
                    let chats_str = config["telegram"]["allowed_chats"].as_str().unwrap_or("");
                    let first_chat = chats_str.split(',').next().unwrap_or("").trim();
                    if !token.is_empty() && !first_chat.is_empty() {
                        let msg = format!(
                            "🔔 *LaRuche Notification*\n\n*Event:* `{:?}`\n*Actor:* `{}`",
                            ev.kind, ev.actor
                        );
                        let client = reqwest::Client::new();
                        let _ = client
                            .post(&format!(
                                "https://api.telegram.org/bot{}/sendMessage",
                                token
                            ))
                            .json(&serde_json::json!({
                                "chat_id": first_chat,
                                "text": msg,
                                "parse_mode": "Markdown"
                            }))
                            .send()
                            .await;
                    }
                }
            }
        }
    });

    info!("Starting MCP servers if configured...");
    // Auto-start channels if configured
    {
        let config_path = std::path::Path::new("channels-config.json");
        if config_path.exists() {
            if let Ok(content) = std::fs::read_to_string(config_path) {
                if let Ok(channels_config) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(tg_token) = channels_config["telegram"]["bot_token"].as_str() {
                        if !tg_token.is_empty()
                            && channels_config["telegram"]["enabled"]
                                .as_bool()
                                .unwrap_or(false)
                        {
                            let allowed = channels_config["telegram"]["allowed_chats"]
                                .as_str()
                                .unwrap_or("")
                                .to_string();
                            let token = tg_token.to_string();
                            let state_for_tg = state.clone();
                            let handle = tokio::spawn(async move {
                                run_telegram_bot(&token, &allowed, &state_for_tg).await;
                            });
                            state
                                .channel_handles
                                .write()
                                .await
                                .insert("telegram".into(), handle);
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
                let tls_config =
                    axum_server::tls_rustls::RustlsConfig::from_pem_file(&cert_path, &key_path)
                        .await
                        .expect("Failed to load TLS certificate/key");
                let _ = axum_server::bind_rustls(
                    addr.parse().expect("Invalid bind address"),
                    tls_config,
                )
                .serve(app.into_make_service_with_connect_info::<SocketAddr>())
                .await;
            } else {
                let listener_tcp = tokio::net::TcpListener::bind(&addr)
                    .await
                    .expect("Failed to bind");
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
                let tls_config =
                    axum_server::tls_rustls::RustlsConfig::from_pem_file(&cert_path, &key_path)
                        .await
                        .expect("Failed to load TLS certificate/key");
                let _ = axum_server::bind_rustls(
                    addr.parse().expect("Invalid bind address"),
                    tls_config,
                )
                .serve(app.into_make_service_with_connect_info::<SocketAddr>())
                .await;
            } else {
                let listener_tcp = tokio::net::TcpListener::bind(&addr)
                    .await
                    .expect("Failed to bind");
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
async fn get_metrics_history(State(state): State<Arc<AppState>>) -> Json<MetricsHistoryResponse> {
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
        Ok(raw) => match serde_json::from_str::<PersistentState>(&raw) {
            Ok(s) => {
                info!(path = %path.display(), entries = s.activity_log.len(), "Loaded persistent state");
                s
            }
            Err(e) => {
                warn!(path = %path.display(), error = %e, "Failed to parse state file, starting fresh");
                PersistentState::default()
            }
        },
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
        capability_selection: Some(state.capability_selection.read().await.clone()),
        activity_log: logs.iter().cloned().collect(),
        disabled_tools: state.essaim_config.read().await.disabled_tools.clone(),
        disabled_skills: state.essaim_config.read().await.disabled_skills.clone(),
        permission_mode: Some(
            permission_mode_to_str(state.essaim_config.read().await.permission_mode).to_string(),
        ),
        saved_at: chrono::Utc::now().to_rfc3339(),
        cookie_secret: Some(auth_user::cookie_secret_to_base64(&state.cookie_secret)),
        context_max_messages: Some(state.essaim_config.read().await.context_max_messages),
        compaction_threshold: Some(state.essaim_config.read().await.compaction_threshold),
        context_max_tokens: Some(state.essaim_config.read().await.context_max_tokens),
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

// Trigger rebuild

// Trigger rebuild 2

// Trigger rebuild 3

// Trigger rebuild 4

// Trigger rebuild 5

// Trigger rebuild 6

// Trigger rebuild 7

// Trigger rebuild 8

// Trigger rebuild 9

// Trigger rebuild 10

// Trigger rebuild 11

// Trigger rebuild 12

// Trigger rebuild 13

// Trigger rebuild 14

// Trigger rebuild 15

// Trigger rebuild 16

// Trigger rebuild 17

// Trigger rebuild 18

// Trigger rebuild 19

// Trigger rebuild 20

// Trigger rebuild 21
