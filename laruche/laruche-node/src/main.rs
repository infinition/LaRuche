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
mod secrets_vault;
mod sync;
mod systray;
mod tui;
mod config_api;
mod plugins_api;
mod voice_api;
mod profiles_api;
mod knowledge_api;
mod web;
mod slack_api;
mod local_api;
mod ws_chat;
mod discord_api;
mod channels_api;
mod auth_api;
mod events_api;
mod credentials_api;
mod settings_api;
mod doctor_api;

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

// Web asset serving (SPA shell, CSS, concatenated JS) and i18n language-file
// injection live in `web.rs` (handlers: web::spa_page / app_css / app_js / lang_file).
const PEER_FETCH_TIMEOUT_MS: u64 = 4000;
// Peer staleness window. MUST be > the mDNS re-announce interval (30s below),
// otherwise a peer "flickers": it goes stale between two announcements. 90s tolerates 2 missed announcements.
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
    /// Per-capability service selection (with source): survives restart.
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
    /// Curateur (auto-skills/tools) enabled from Settings: survives restart.
    #[serde(default)]
    curateur_actif: Option<bool>,
    /// "Home" channel (/sethome): default destination for proactive messages.
    #[serde(default)]
    home_channel: Option<String>,
    /// Dynamic tool selection (inject only relevant schemas: lighter prompt
    /// for small-context models). Survives restart.
    #[serde(default)]
    dynamic_tool_selection: Option<bool>,
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

/// Current service selection for a given capability (stt/tts/code/vlm/vla/llm...).
/// Goes beyond a plain model name: keeps the SOURCE (backend / node mesh)
/// for routing (e.g. voice dictation to the chosen STT, auto-TTS to the chosen TTS).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CapabilitySelection {
    capability: String,
    model: String,
    /// Backend/host (local label "llama.cpp"... or mesh node IP).
    backend: String,
    /// Remote Miel node id (None if local service).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    node_id: Option<String>,
    is_local: bool,
    /// Provider profile serving this capability (to resolve provider/base_url/key at runtime).
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

pub(crate) struct AppState {
    manifest: RwLock<CognitiveManifest>,
    auth: RwLock<ProximityAuth>,
    queue: RwLock<RequestQueue>,
    listener: RwLock<MielListener>,
    config: NodeConfig,
    /// Manually declared mesh services (P6)
    custom_services: RwLock<HashMap<String, CustomService>>,
    /// Per-capability default models (e.g. "llm" → "mistral", "code" → "qwen3-coder:30b")
    /// The "llm" key is the universal fallback for unspecified capabilities.
    default_models: RwLock<HashMap<String, String>>,
    /// Per-capability service selection (with source), for voice/code/vision routing.
    capability_selection: RwLock<HashMap<String, CapabilitySelection>>,
    /// Long-running missions ("La Reine"): metadata; the knowledge lives in the cognitive map.
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

/// Override a config's provider/model with the per-channel choice (Settings >
/// Channels). Channels without an override keep the global active model, so this
/// is always safe to call. Lets e.g. Telegram run a tool-reliable model while the
/// web chat uses a faster one.
async fn apply_channel_model(state: &AppState, channel: &str, config: &mut EssaimConfig) {
    let profiles = state.profiles.read().await;
    if let Some((profile, model)) = profiles::model_for_channel(&profiles, channel) {
        config.provider = profile.provider.clone();
        config.api_key = profile.api_key.clone();
        config.api_base = if profile.base_url.is_empty() {
            None
        } else {
            Some(profile.base_url.clone())
        };
        config.model = model.to_string();
    }
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

// -- Blueprints: parameterized cron automation templates ------------------------
// Built-in catalogue (laruche_essaim::blueprints::catalogue) + blueprints CREATED by
// the user, persisted in `blueprints.json`.

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

/// GET /api/blueprints - built-in catalogue + user blueprints.
async fn get_blueprints() -> Json<Vec<laruche_essaim::blueprints::Blueprint>> {
    let mut all = laruche_essaim::blueprints::catalogue();
    all.extend(load_user_blueprints());
    Json(all)
}

/// POST /api/blueprints - creates (or updates) a user blueprint. Body = Blueprint
/// {id, title, schedule_template, prompt_template, slots:[{name,label,default}]}.
async fn api_create_blueprint(
    Json(mut bp): Json<laruche_essaim::blueprints::Blueprint>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if bp.id.trim().is_empty() {
        // derive an id from the title
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
    // Forbid overwriting a built-in blueprint.
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

/// DELETE /api/blueprints/:id - deletes a user blueprint (built-ins are immutable).
async fn api_delete_blueprint(Path(id): Path<String>) -> Json<serde_json::Value> {
    let mut users = load_user_blueprints();
    let before = users.len();
    users.retain(|b| b.id != id);
    let removed = before - users.len();
    let _ = save_user_blueprints(&users);
    Json(serde_json::json!({ "status": "ok", "removed": removed }))
}

/// POST /api/blueprints/:id/instancier - instantiates a blueprint into a REAL cron.
/// Body = slot values: `{ "<slot>": "<value>", ... }` (or `{slots:{...}}`).
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
    // Accepts {slots:{...}} or a flat object of values.
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
                    // Route to peer: they have a lower queue
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
    // Resilient: if Ollama is down, do NOT fail the whole endpoint (otherwise the
    // "Mesh services" panel stays stuck). Just list 0 Ollama models + the mesh services.
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

    // Local OpenAI-compatible inference backends (llama.cpp, vLLM, LM Studio...).
    // Same logic as Ollama: list them and announce them on the mesh.
    {
        let detectes = local_inference::detecter_modeles_openai_compat(
            &local_inference::backends_openai_compat_par_defaut(),
        )
        .await;
        for m in detectes {
            // Avoid duplicates if the same model is already exposed locally (e.g. Ollama).
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
            // LOCAL backend (llama.cpp/vLLM/LM Studio): list ONLY what is actually
            // detected ALIVE (live detection above). A local profile whose backend is off
            // therefore does NOT appear (no more phantom models from a closed Ollama/llama.cpp).
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
        // Do not list SELF as a mesh peer: a node hears its own
        // mDNS announcement. Its local role is already shown in SWARM INTELLIGENCE.
        if node.manifest.node_id == Some(manifest.node_id)
            || node.manifest.host == manifest.api_endpoint.host
        {
            continue;
        }
        for cap_str in &node.manifest.capabilities {
            let cap = cap_str.to_string();
            // No longer skip the primary capabilities, since a Miel node
            // may well host "custom" LLM/VLM models (outside Ollama).
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
        message: "Awaiting physical approval. Press the button on the LaRuche box."
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
            // System logs (no user_id): visible to admin only, hidden from regular users
            // User's own logs: visible to that user
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

/// GET /api/voice/status - check STT/TTS service availability.
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

/// Fetches (with caching) a peer's public key via its /api/mesh/identity. Verifies that the
/// node_id announced by the IP matches the one declared (X-Miel-From).
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
        return None; // the IP does not match the declared node_id -> suspicious
    }
    cache.lock().unwrap().insert(node_id.to_string(), pk.clone());
    Some(pk)
}

/// VERIFIED node_id (ed25519 signature) of the caller of an inference request, or None.
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

    // Mesh ENFORCEMENT: a REMOTE caller (non-loopback) may only use this model according to
    // its visibility. The local node (loopback) is never blocked.
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
                return refus("Private model: not shared on the mesh.");
            }
            profiles::Visibilite::Restricted => {
                match verified_inference_caller(&headers, &addr).await {
                    Some(nid) if allowed.iter().any(|a| a == &nid) => {} // allowed
                    Some(_) => return refus("Ruche not authorized for this model (restricted)."),
                    None => return refus("Mesh identity required/invalid for a restricted model."),
                }
            }
            profiles::Visibilite::PublicProxy => {} // public: any mesh member
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
            None,
        ).await
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

    // Service explicitly chosen per capability (otherwise: first found, previous behavior).
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
    let mut stt_locked = false; // url locked by user selection
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

/// GET/POST /api/tools/config - enable/disable Abeilles for prompt injection/execution.
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

/// GET /api/memory/search?q=...&limit=8 - search cognitive memory.
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

/// POST /api/memory/write - write a durable memory item.
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
        "You must enrich the cognitive node '{}'.\nHere is the user's request: '{}'.\nRead the node with 'memory_read_node', perform the necessary research, then use 'memory_write' to add your findings to this node.",
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
                        format!("{}\n\n**LaRuche summary:**\n{}", prompt, result.summary);
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
                    let new_content = format!("{}\n\n**LaRuche error:**\n{}", prompt, e);
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

/// GET /api/memory/node/:id - read a cognitive-map node with children and active items.
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

/// POST /api/memory/node/move - reparents a node (drag&drop in the tree). body
/// `{node_id, new_parent}`; empty `new_parent` => root node. Moves the whole subtree
/// (id rename). Rejects system nodes and cycles (moving into its own subtree).
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
            serde_json::json!({ "status": "error", "error": "system node cannot be moved" }),
        ));
    }
    if new_id == old || new_id.starts_with(&format!("{old}.")) {
        return Ok(Json(
            serde_json::json!({ "status": "error", "error": "invalid move (cycle or identical)" }),
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

/// POST /api/memory/dream - trigger active memory consolidation.
async fn api_memory_dream(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let dream = state
        .memoire
        .dream()
        .await
        .unwrap_or_else(|e| serde_json::json!({ "error": e.to_string() }));
    Json(dream)
}

/// POST /api/memory/consolidate?node=<id> - ACTUALLY merges items (via the aux model).
/// With `node`: consolidates that node. Without: processes overloaded nodes (>=4 items). Old
/// items are soft-deleted (recoverable). This is what the "Consolidate" button triggers.
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

/// GET /api/memory/grep?q=<texte>&limit=30 - substring search in item content.
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

/// Phase 1 - DISK -> SQL sync: scans `skills/*/SKILL.md` and upserts each skill into
/// `capacities.skills.<slug>` (single item). Additive (does not delete SQL-only skills).
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
        let content = content.replace("\r\n", "\n"); // normalize (SQL in LF)
        if !content.contains("type: skill") {
            continue; // only real OKF skills
        }
        let Some(slug) = p.file_name().and_then(|x| x.to_str()).filter(|s| !s.is_empty()) else {
            continue;
        };
        let node_id = format!("capacities.skills.{slug}");
        // Replace the existing item (skill = single item).
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
        tracing::info!(count = n, "skills synchronized from disk (SKILL.md -> SQL)");
    }
    // Targeted purge of META-SKILLS from other agent frameworks (third-party/Claude Code/Codex...),
    // wrongly imported: they describe ANOTHER agent, not LaRuche. Explicit DENYLIST: definitely
    // NOT a disk diff "delete everything not on disk" (that would destroy skills
    // created by the agent or seeded in code, like arxiv_search / web_research). Hard-delete:
    // delete_node reparents to `orphans.*`, so we also delete the resulting orphan.
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
            continue; // absent -> nothing to do
        }
        if let Ok(r) = memoire.delete_node(&node_id).await {
            purges += 1;
            // delete_node moved it to orphans.<base>_<ts> -> hard-delete this orphan.
            if let Some(orphan) = r.get("relocated_to").and_then(|v| v.as_str()) {
                let _ = memoire.delete_node(orphan).await;
            }
        }
    }
    if purges > 0 {
        tracing::info!(count = purges, "meta-skills from other frameworks purged (denylist)");
    }
}

/// Imports a list of facts `{node_id, content}` into memory (exact dedup). (imported, skipped).
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
        // Exact dedup: if an identical item already exists in this node, skip.
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

/// GET /api/memory/export_changes?since=<ts> - facts (op=write) written since `since`, for
/// mesh federation (Lever 3, first slice). Excludes system/capacities projections.
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

/// POST /api/memory/import_changes {items:[{node_id,content}], source?} - applies facts (dedup).
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

/// POST /api/memory/mesh_pull {peer, since?} - pulls facts from a PEER node (Miel) and imports them
/// locally. First building block of the mesh's COLLECTIVE memory (Lever 3).
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
        return Json(serde_json::json!({ "error": "missing peer (e.g. http://192.168.1.20:8419)" }));
    }
    let since = body["since"].as_i64().unwrap_or(0);
    let url = format!("{peer}/api/memory/export_changes?since={since}");
    let data: serde_json::Value = match reqwest::get(&url).await {
        Ok(r) => match r.json().await {
            Ok(v) => v,
            Err(e) => return Json(serde_json::json!({ "error": format!("peer json: {e}") })),
        },
        Err(e) => return Json(serde_json::json!({ "error": format!("peer contact: {e}") })),
    };
    let empty: Vec<serde_json::Value> = vec![];
    let items = data["items"].as_array().unwrap_or(&empty);
    let src = format!("mesh:{peer}");
    let (imported, skipped) = importer_changes(&state, items, &src).await;
    Json(serde_json::json!({ "pulled_from": peer, "imported": imported, "skipped": skipped }))
}

/// GET /api/state/version - ts of the last memory mutation (P7 lite: the UI polls to know
/// whether to refresh, without a push channel).
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

/// Actor of a memory mutation based on its `src` (source/reason). UI -> User, otherwise LaRuche.
fn feed_actor(src: &str) -> &'static str {
    let s = src.trim().to_lowercase();
    if s.starts_with("ui") || s == "user" || s == "fabien" || s == "admin" {
        "User"
    } else {
        "LaRuche"
    }
}

/// Cleans an agent response for the Feed: removes protocol blocks (`<plan>`, `<tool_call>`,
/// `<think>`) - complete or truncated - and normalizes whitespace. Otherwise the Feed shows JSON/XML
/// unreadable to a human.
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
                None => out.truncate(i), // opening tag without closing -> cut the tail
            }
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

// ===================== Phase 4 - Mesh messaging (DM between instances/users) =====================
#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct InboxMessage {
    id: String,
    peer_id: String,
    peer_name: String,
    dir: String, // "in" (received) | "out" (sent)
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

/// GET /api/mesh/code - indicates whether a mesh code is configured (never the secret itself).
async fn api_mesh_code_get() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "set": sync::load_mesh_code().is_some() }))
}
/// POST /api/mesh/code {code} - sets/clears the shared mesh code (auth + encryption base).
async fn api_mesh_code_set(Json(body): Json<serde_json::Value>) -> Json<serde_json::Value> {
    let code = body["code"].as_str().unwrap_or("");
    sync::save_mesh_code(code);
    Json(serde_json::json!({ "status": "ok", "set": !code.trim().is_empty() }))
}

/// GET /api/mesh/identity - node_id + this node's ed25519 PUBLIC key (hex). Peers fetch it
/// and cache it to verify signatures (strong identity, `restricted`).
async fn api_mesh_identity() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "node_id": sync::my_node_id(), "pubkey": sync::my_pubkey_hex() }))
}

/// GET /api/mesh/whoami - identity of THIS instance (laruche ID + name).
async fn api_mesh_whoami(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let m = state.manifest.read().await;
    Json(serde_json::json!({ "id": m.node_id.to_string(), "name": m.node_name }))
}

/// GET /api/mesh/peers - other LaRuche instances discovered on the network (directory).
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

// --- Gap A - FEDERATION OF VERIFIED SKILLS BETWEEN NODES ----------------------------
// A swarm that learns collectively: when a node has (created/verified) a skill, the others
// can fetch it. Mechanics: each node ANNOUNCES its skills (slug + content hash),
// and SYNCHRONIZES by pulling from peers the skills it lacks (or whose hash differs).

/// Lists local skills on disk (`skills/<slug>/SKILL.md`) with a content hash.
fn lister_skills_locaux() -> Vec<(String, String, String)> {
    // (slug, hash, content)
    let mut out = Vec::new();
    let Ok(rd) = std::fs::read_dir("skills") else {
        return out;
    };
    for e in rd.flatten() {
        if !e.path().is_dir() {
            continue;
        }
        let slug = e.file_name().to_string_lossy().to_string();
        let md = e.path().join("SKILL.md");
        if let Ok(content) = std::fs::read_to_string(&md) {
            let hash = blake3::hash(content.as_bytes()).to_hex().to_string();
            out.push((slug, hash, content));
        }
    }
    out
}

/// GET /api/mesh/skills - announces THIS node's verified skills (slug + hash, without content).
async fn api_mesh_skills_list() -> Json<serde_json::Value> {
    let skills: Vec<serde_json::Value> = lister_skills_locaux()
        .into_iter()
        .map(|(slug, hash, _)| serde_json::json!({ "slug": slug, "hash": hash }))
        .collect();
    Json(serde_json::json!({ "skills": skills }))
}

/// GET /api/mesh/skills/:slug - returns a skill's SKILL.md content (for a peer to pull).
async fn api_mesh_skill_get(Path(slug): Path<String>) -> Json<serde_json::Value> {
    // Anti-traversal guard: slug = a single alphanumeric/_/- segment.
    if slug.is_empty() || !slug.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
        return Json(serde_json::json!({ "status": "error", "error": "invalid slug" }));
    }
    match std::fs::read_to_string(format!("skills/{slug}/SKILL.md")) {
        Ok(content) => Json(serde_json::json!({ "slug": slug, "content": content })),
        Err(_) => Json(serde_json::json!({ "status": "error", "error": "not found" })),
    }
}

/// POST /api/mesh/sync - pulls from all active peers the missing/different verified skills,
/// writes them to disk then re-indexes them in memory. Returns the report. **Additive**: overwrites
/// a local skill only if the remote hash differs; never deletes.
async fn api_mesh_skills_sync(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let locaux: std::collections::HashMap<String, String> = lister_skills_locaux()
        .into_iter()
        .map(|(slug, hash, _)| (slug, hash))
        .collect();

    let nodes = state.listener.read().await.get_nodes().await;
    let m = state.manifest.read().await;
    let self_id = m.node_id;
    let self_host = m.api_endpoint.host.clone();
    drop(m);

    let http = reqwest::Client::builder()
        .timeout(Duration::from_millis(PEER_FETCH_TIMEOUT_MS))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    let mut importes: Vec<String> = Vec::new();
    let mut vus_pairs = 0usize;

    for node in nodes.values() {
        if node.manifest.node_id == Some(self_id)
            || node.manifest.host == self_host
            || is_stale(node.last_seen)
        {
            continue;
        }
        let host = node.manifest.host.clone();
        let port = node.manifest.port.unwrap_or(miel_protocol::DEFAULT_API_PORT);
        let base = format!("http://{host}:{port}");
        vus_pairs += 1;

        // 1) peer announcement
        let Ok(resp) = http.get(format!("{base}/api/mesh/skills")).send().await else {
            continue;
        };
        let Ok(val) = resp.json::<serde_json::Value>().await else {
            continue;
        };
        let Some(liste) = val.get("skills").and_then(|s| s.as_array()) else {
            continue;
        };

        for sk in liste {
            let slug = sk.get("slug").and_then(|s| s.as_str()).unwrap_or("");
            let hash = sk.get("hash").and_then(|s| s.as_str()).unwrap_or("");
            if slug.is_empty()
                || !slug.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-')
            {
                continue;
            }
            // already up to date?
            if locaux.get(slug).map(|h| h == hash).unwrap_or(false) {
                continue;
            }
            // 2) pull the content
            let Ok(r) = http
                .get(format!("{base}/api/mesh/skills/{slug}"))
                .send()
                .await
            else {
                continue;
            };
            let Ok(body) = r.json::<serde_json::Value>().await else {
                continue;
            };
            let Some(content) = body.get("content").and_then(|c| c.as_str()) else {
                continue;
            };
            // 3) write to disk
            let dir = format!("skills/{slug}");
            let peer_name = node
                .manifest
                .node_name
                .clone()
                .unwrap_or_else(|| host.clone());
            if std::fs::create_dir_all(&dir).is_ok()
                && std::fs::write(format!("{dir}/SKILL.md"), content).is_ok()
            {
                importes.push(format!("{slug} ⇐ {peer_name}"));
                laruche_essaim::feed_journal::record(
                    "LaRuche",
                    "mesh",
                    "federated the skill",
                    format!("{slug} (from {peer_name})"),
                    chrono::Utc::now(),
                );
            }
        }
    }

    // 4) re-index disk -> SQL to make the pulled skills immediately usable.
    if !importes.is_empty() {
        sync_skills_disk_to_sql(&state.memoire).await;
    }

    Json(serde_json::json!({
        "status": "ok",
        "peers_scanned": vus_pairs,
        "imported": importes,
        "count": importes.len(),
    }))
}

/// POST /api/mesh/send {to_id, text} - sends a DM to a peer (resolves the host by ID, POSTs to its
/// /api/mesh/receive). Keeps a local copy (dir=out) for the conversation thread.
async fn api_mesh_send(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let to_id = body["to_id"].as_str().unwrap_or("").to_string();
    let text = body["text"].as_str().unwrap_or("").trim().to_string();
    if to_id.is_empty() || text.is_empty() {
        return Json(serde_json::json!({ "status": "error", "error": "to_id/text required" }));
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
        return Json(serde_json::json!({ "status": "error", "error": "peer not found" }));
    }
    let (my_id, my_name) = {
        let m = state.manifest.read().await;
        (m.node_id.to_string(), m.node_name.clone())
    };
    let client = reqwest::Client::new();
    let url = format!("http://{host}:8419/api/mesh/receive");
    // Encrypt the content if a mesh code is configured (otherwise plaintext, backward-compatible).
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

/// POST /api/mesh/receive {from_id, from_name, text} - receives a DM from another LaRuche.
/// Mesh-code auth if configured (otherwise open, historical LAN behavior).
async fn api_mesh_receive(
    headers: axum::http::HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    if let Some(false) = sync::mesh_auth_ok(&headers, "/api/mesh/receive") {
        return Json(serde_json::json!({ "status": "error", "error": "invalid mesh auth" }));
    }
    let from_id = body["from_id"].as_str().unwrap_or("unknown").to_string();
    let from_name = body["from_name"].as_str().unwrap_or("LaRuche").to_string();
    // Encrypted content (`enc`) -> decrypt; otherwise plaintext (`text`).
    let text = if let Some(enc) = body["enc"].as_str() {
        match sync::open(enc) {
            Some(t) => t.trim().to_string(),
            None => return Json(serde_json::json!({ "status": "error", "error": "decryption failed" })),
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

/// GET /api/inbox - all messages (the client groups by peer).
async fn api_inbox_get() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "messages": read_inbox() }))
}

/// POST /api/inbox/read {peer_id} - marks a peer's messages as read.
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

/// POST /api/feed/ask {text} - talks to LaRuche FROM the Feed. Runs on a dedicated "feed"
/// session (rolling context ~10 exchanges, isolated from the main chat), in the background; the response
/// appears in the Feed via activity_log on the next poll. Full agent capabilities (crons...).
async fn api_feed_ask(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let text = match body["text"].as_str().map(str::trim).filter(|s| !s.is_empty()) {
        Some(t) => t.to_string(),
        None => return Json(serde_json::json!({ "status": "error", "error": "empty text" })),
    };
    let st = state.clone();
    tokio::spawn(async move {
        // Dedicated feed session (deterministic id) -> rolling context, separate from the main chat.
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
        // Short rolling context (~10 exchanges = 20 messages): truncate the oldest.
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

/// GET /api/profile - user profile (node `system.user`, injected into LaRuche's context).
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

/// POST /api/profile {fiche} - replaces the user profile (single item). Source `ui-profile`
/// (User actor in the Feed). Only the user edits; the agent is forbidden (memory_write).
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

/// GET /api/feed?limit=N - UNIFIED activity stream for the global Feed pane: memory mutations
/// (with User/LaRuche actor + clickable ref) + agent inferences (activity_log), sorted recent->old.
async fn api_feed(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(q): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let limit = q
        .get("limit")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(200);
    let mut events: Vec<serde_json::Value> = Vec::new();

    // 1) Memory mutations (who added/deleted/modified what).
    if let Ok(muts) = state.memoire.mutations(Some(150)).await {
        if let Some(arr) = muts.get("mutations").and_then(|m| m.as_array()) {
            for m in arr {
                let op = m.get("op").and_then(|v| v.as_str()).unwrap_or("");
                let node = m.get("node_id").and_then(|v| v.as_str()).unwrap_or("");
                let ts = m.get("ts").and_then(|v| v.as_i64()).unwrap_or(0);
                let src = m.get("src").and_then(|v| v.as_str()).unwrap_or("");
                // System noise (non-activity): tool indexing + node (re)seed at boot
                // + disk<->SQL skill sync (delete+write per skill on each startup/watch ->
                // flooded the Feed with dozens of capacities.skills.* lines).
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
                    "write" if src == "consolidation" => "consolidated",
                    "write" => "added an item to",
                    "propose" => "proposed an item in",
                    "update" => "modified an item of",
                    "delete" => "deleted an item from",
                    "move" => "moved an item to",
                    "create_node" => "created the node",
                    "update_node" => "updated the node",
                    "rename_subtree" => "moved the subtree",
                    _ => "modified",
                };
                events.push(serde_json::json!({
                    "ts": ts, "actor": feed_actor(src), "kind": "memory",
                    "action": action, "object": node, "ref": node
                }));
            }
        }
    }

    // 2) Agent exchanges: the user's message (full_prompt) THEN LaRuche's response
    //    (full_response cleaned of protocol tags). Lets you see your own messages in the
    //    Feed, attributed to User, and a readable response (no raw <plan>/<tool_call>).
    {
        let logs = state.activity_log.read().await;
        for e in logs.iter() {
            // MILLISECONDS (rfc3339 has sub-second). Reverse-chronological Feed (recent on
            // TOP) -> within a turn, the RESPONSE (more recent) is placed 1 ms ABOVE the
            // question. You read: response, then its question below; next turn lower down.
            let ms = chrono::DateTime::parse_from_rfc3339(&e.timestamp)
                .map(|d| d.timestamp_millis())
                .unwrap_or(0);
            // a) User message (only for chat exchanges).
            if e.tag == "agent" {
                if let Some(prompt) = e.full_prompt.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
                    let clean = prompt.split("\n\n[SYSTEM]").next().unwrap_or(prompt).trim();
                    if !clean.is_empty() {
                        events.push(serde_json::json!({
                            "ts": ms, "actor": "User", "kind": "agent",
                            "action": "asked", "object": preview_text(clean, 160),
                            "full": clean, "ref": serde_json::Value::Null, "tag": e.tag
                        }));
                    }
                }
            }
            // b) LaRuche's response, cleaned (otherwise unreadable JSON/XML). Empty after cleaning
            //    (pure tool turn) -> we don't add a hollow "replied" event.
            let brut = e.full_response.as_deref().filter(|s| !s.is_empty()).unwrap_or(&e.message);
            let resp = nettoyer_reponse_feed(brut);
            if !resp.is_empty() {
                events.push(serde_json::json!({
                    "ts": ms + 1, "actor": "LaRuche", "kind": "agent",
                    "action": "replied", "object": preview_text(&resp, 160),
                    "full": resp, "ref": serde_json::Value::Null, "tag": e.tag
                }));
            }
        }
    }

    // 3) Executed crons (last run).
    {
        let cron = state.essaim_cron.read().await;
        for t in cron.list() {
            if let Some(lr) = t.last_run {
                events.push(serde_json::json!({
                    "ts": lr.timestamp(), "actor": "LaRuche", "kind": "cron",
                    "action": "ran the cron", "object": t.name, "ref": serde_json::Value::Null
                }));
            }
        }
    }
    // 4) Missions (last iteration).
    {
        let missions = state.missions.read().await;
        for m in missions.list() {
            if let Some(lr) = m.last_run.as_deref() {
                let ts = chrono::DateTime::parse_from_rfc3339(lr)
                    .map(|d| d.timestamp())
                    .unwrap_or(0);
                events.push(serde_json::json!({
                    "ts": ts, "actor": "LaRuche", "kind": "mission",
                    "action": "advanced the mission", "object": m.slug, "ref": serde_json::Value::Null
                }));
            }
        }
    }
    // 5) Triggered watchers (last detection).
    {
        let watchers = state.watchers.read().await;
        for w in watchers.list() {
            if let Some(lr) = w.last_run {
                events.push(serde_json::json!({
                    "ts": lr.timestamp(), "actor": "LaRuche", "kind": "watcher",
                    "action": "triggered the watcher", "object": w.name, "ref": serde_json::Value::Null
                }));
            }
        }
    }

    // 6) Direct messages (DM) from the mesh -> first building block of the global feed. Actor = the PEER (purple
    //    ruche) for received ones; Me for sent ones.
    for m in read_inbox() {
        let (actor, action, akind) = if m.dir == "out" {
            ("User".to_string(), format!("wrote to {}", m.peer_name), "user")
        } else {
            (m.peer_name.clone(), "wrote to you".to_string(), "peer")
        };
        events.push(serde_json::json!({
            "ts": m.ts, "actor": actor, "kind": "dm", "action": action,
            "object": preview_text(&m.text, 160), "full": m.text,
            "ref": serde_json::Value::Null, "actor_kind": akind
        }));
    }

    // Unify the unit: the mutations/cron/mission/watcher sections are in SECONDS, the agent
    // section in MILLISECONDS. Convert everything to ms (a ts < 1e12 = seconds -> x1000) for a
    // consistent sort (otherwise the agent events, 1000x larger, would crush everything).
    for e in events.iter_mut() {
        let t = e["ts"].as_i64().unwrap_or(0);
        if t > 0 && t < 1_000_000_000_000 {
            e["ts"] = serde_json::Value::from(t * 1000);
        }
    }
    // Normalize all `ts` to MILLISECONDS (some sources are in seconds: memory,
    // missions, watchers, crons). Without this, agent events (already in ms) ALWAYS floated
    // above the others, regardless of real time. Heuristic: ts < 1e12 -> seconds.
    // PERSISTENT system journal: creations (cron/watcher/mission/kanban) + curateur runs.
    // Survives restart (before: only executions via last_run appeared).
    for ev in laruche_essaim::feed_journal::recent(limit) {
        events.push(serde_json::json!({
            "ts": ev.ts, "actor": ev.actor, "kind": ev.kind,
            "action": ev.action, "object": ev.object, "ref": serde_json::Value::Null
        }));
    }

    for e in events.iter_mut() {
        if let Some(ts) = e.get("ts").and_then(|v| v.as_i64()) {
            if ts != 0 && ts < 1_000_000_000_000 {
                e["ts"] = serde_json::Value::from(ts * 1000);
            }
        }
    }
    events.sort_by(|a, b| {
        b["ts"].as_i64().unwrap_or(0).cmp(&a["ts"].as_i64().unwrap_or(0))
    });
    events.truncate(limit);
    Json(serde_json::json!({ "events": events }))
}

/// GET /api/system/prompt-defaults - default (hardcoded) texts of the editable sections,
/// to pre-fill the editor: the user sees and edits the full prompt (empty in DB =
/// this default is used). The `node_*` override REPLACES the corresponding section.
async fn api_system_prompt_defaults() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "identity": laruche_essaim::prompt::section_identite_stable(),
        "behavior": laruche_essaim::prompt::section_comportement(),
        "prompt_curateur": laruche_essaim::butinage_pont::prompt_curateur_defaut(),
        "prompt_extraction": laruche_essaim::butinage_pont::prompt_extraction_defaut(),
        "prompt_planning": laruche_essaim::prompt::section_planification(),
    }))
}

/// GET /api/memory/tree - lightweight cognitive-map snapshot for the SPA.
async fn api_memory_tree(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let health = state.memoire.health().await.unwrap_or(false);
    let dream = state
        .memoire
        .dream()
        .await
        .unwrap_or_else(|e| serde_json::json!({ "error": e.to_string() }));
    // Real tree from the backend (the Obsidian UI rebuilds the hierarchy on the
    // dotted ids). Fallback to the base roots if the backend does not list (sidecar).
    let mut nodes = state
        .memoire
        .list_nodes()
        .await
        .unwrap_or_else(|_| serde_json::json!([]));
    if nodes.as_array().map(|a| a.is_empty()).unwrap_or(true) {
        nodes = serde_json::json!([
            { "id": "people", "label": "People", "one_liner": "People and preferences" },
            { "id": "projects", "label": "Projects", "one_liner": "Active projects" },
            { "id": "decisions", "label": "Decisions", "one_liner": "Durable choices" },
            { "id": "capacities", "label": "Capacites", "one_liner": "Tools, plugins, MCP, skills" },
            { "id": "missions", "label": "Missions", "one_liner": "Long-running research" },
            { "id": "sessions", "label": "Sessions", "one_liner": "Conversational context" },
            { "id": "knowledge", "label": "Knowledge", "one_liner": "Imported knowledge" }
        ]);
    }
    // Mark system-managed nodes (tools.*/system.*): the UI shows a 🔒 and
    // the agent cannot mutate them (`noeud_reserve` guard). The admin can edit them.
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

/// GET /api/memory/stats - counts (items/nodes/mutations) for the SPA.
async fn api_memory_stats(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    Json(
        state
            .memoire
            .stats()
            .await
            .unwrap_or_else(|e| serde_json::json!({ "error": e.to_string() })),
    )
}

/// GET /api/memory/mutations?limit=50 - audit log of recent memory mutations.
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

/// GET /api/memory/export_okf?dir=okf-export&node=<id> - exports an OKF bundle into a server
/// folder. Optional `node` = exports only that node + its subtree (otherwise the whole map).
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

/// Recursively zips a folder's content into an in-memory buffer (entries relative to `base`).
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

/// GET /api/memory/export.zip?node=<id> - exports the OKF bundle and returns it as a
/// DOWNLOADABLE .zip (Content-Disposition), without writing anything durable on the project side.
/// Optional `node` = current node + subtree; otherwise the whole memory.
async fn api_memory_export_zip(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(q): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<axum::response::Response, StatusCode> {
    use axum::response::IntoResponse;
    let node = q.get("node").map(|s| s.as_str()).filter(|s| !s.is_empty());
    // Isolated temporary folder (cleaned up after reading).
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

/// POST /api/memory/import_okf?dir=okf-export - imports an OKF bundle.
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
    const CAPABILITY_HINT: &str = "\n\n[SYSTEM] You can schedule (cron_create), watch (watcher_create) and search your past conversations (session_search) yourself.";
    const AUTO_CONTINUE: &str = "Continue immediately with the next step of the plan";
    const OUTPUT_RECOVERY: &str = "Continue exactly from the interrupted response.";
    const FAILOVER_RECOVERY: &str = "The previous response was truncated twice.";

    if text.starts_with(AUTO_CONTINUE)
        || text.starts_with(OUTPUT_RECOVERY)
        || text.starts_with(FAILOVER_RECOVERY)
    {
        return None;
    }

    let text = text.strip_suffix(CAPABILITY_HINT).unwrap_or(text);
    let text = text.strip_prefix("/no_think\n").unwrap_or(text);
    if let Some((_, steering)) = text.split_once("\n") {
        if text.starts_with("[User steering injected during") {
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
        let raw = "Download this\n\n[SYSTEM] You can schedule (cron_create), watch (watcher_create) and search your past conversations (session_search) yourself.";
        assert_eq!(display_user_text(raw).as_deref(), Some("Download this"));
        assert!(display_user_text(
            "Continue immediately with the next step of the plan, without stopping."
        )
        .is_none());
    }

    #[test]
    fn assistant_display_keeps_plan_structured_and_hides_markup() {
        let message = laruche_essaim::Message::Assistant(
            "<plan>[{\"task\":\"Download\",\"status\":\"done\"}]</plan>\nFile ready.<tool_call>{}</tool_call>"
                .into(),
        );
        let display = session_message_for_client(&message).unwrap();
        assert_eq!(display["text"], "File ready.");
        assert_eq!(display["plan"][0]["task"], "Download");
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
            text: "I will fetch the page then analyze the result.".into(),
        });
        stats.apply_event(&ChatEvent::ToolCall {
            name: "web_fetch".into(),
            args: serde_json::json!({"url":"https://example.test/long-page"}),
            iteration: Some(1),
        });
        stats.apply_event(&ChatEvent::ToolResult {
            name: "web_fetch".into(),
            result: "content ".repeat(200),
            success: true,
            elapsed_ms: Some(42),
        });

        assert!(stats.messages >= 4);
        assert!(stats.used_tokens() > 65);
        assert!(stats.running);
    }
}

/// GET /api/sessions/:id/messages - get session messages (with ownership check).
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

/// GET /api/sessions/search?q=query - search across all sessions.
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

/// GET /api/sessions/:id/export - export a session as Markdown.
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

/// POST /api/sessions/:id/fork - fork (branch) a session (with ownership check).
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

/// GET /api/cron - list scheduled tasks.
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

/// POST /api/agents/spawn - launch a subagent dynamically.
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

/// POST /api/cron - create a scheduled task.
/// Body: {"name": "...", "prompt": "...", "cron_expr": "*/5 * * * *"} or {"fire_at": "ISO8601"}
async fn api_create_cron(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Admin only: cron tasks execute agent prompts
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

    let cron_name = task.name.clone();
    let id = {
        let mut cron = state.essaim_cron.write().await;
        cron.add(task)
    };
    laruche_essaim::feed_journal::record(
        "User",
        "cron",
        "created the scheduled task",
        cron_name,
        chrono::Utc::now(),
    );

    Ok(Json(
        serde_json::json!({"id": id.to_string(), "status": "created"}),
    ))
}

/// DELETE /api/cron/:id - remove a scheduled task.
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

/// POST /api/cron/:id/run - immediately runs a cron's prompt (spawn).
// --- Missions ("La Reine") --------------------------------------------------
/// GET /api/missions - lists long-running missions.
async fn api_list_missions(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    Json(serde_json::json!(state.missions.read().await.list()))
}

/// POST /api/missions - creates a mission. Body: {objective, slug?, cadence?}.
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
        return Json(serde_json::json!({"error": "objective required"}));
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
    let opt = |k: &str| {
        body[k]
            .as_str()
            .filter(|s| !s.trim().is_empty())
            .map(|s| s.to_string())
    };
    let m = missions::Mission {
        slug: slug.clone(),
        objective,
        cadence,
        profile_id: opt("profile_id"),
        model: opt("model"),
        channel: opt("channel"),
        status: "active".to_string(),
        iterations: 0,
        last_run: None,
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    state.missions.write().await.upsert(m);
    laruche_essaim::feed_journal::record(
        "User",
        "mission",
        "created the mission",
        slug.clone(),
        chrono::Utc::now(),
    );
    Json(serde_json::json!({"status": "ok", "slug": slug}))
}

/// Runs ONE mission iteration (reused by the API AND the cadence daemon): the agent reads
/// the accumulated state under `missions.<slug>`, advances one step and writes its findings there.
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
    let profile_id = mission.profile_id.clone();
    let model_override = mission.model.clone();
    let channel = mission.channel.clone();
    let run_state = state.clone();
    tokio::spawn(async move {
        // Mission provider/model (otherwise global default).
        let mut cfg = run_state.essaim_config.read().await.clone();
        if let Some(pid) = &profile_id {
            profiles_api::appliquer_profil(&run_state, &mut cfg, pid, model_override.as_deref()).await;
        } else if let Some(m) = &model_override {
            cfg.model = m.clone();
        }
        // Origin channel -> a cron created by the mission will reply there; also used as delivery target.
        cfg.origin_channel = channel.clone();
        // Anti-replication: a mission iteration does not create scheduled tasks.
        for t in ["cron_create", "watcher_create", "mission_create", "kanban_create"] {
            if !cfg.disabled_tools.iter().any(|d| d == t) {
                cfg.disabled_tools.push(t.to_string());
            }
        }
        let sessions_dir = std::path::Path::new("sessions");
        let mut session = Session::new_with_path(&cfg.model, sessions_dir);
        let (tx, mut rx) = broadcast::channel::<ChatEvent>(64);
        tokio::spawn(async move { while rx.recv().await.is_ok() {} });
        let result = boucle_react_memoire(
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
        // Deliver the report to the mission's channel (if set; otherwise silent background work).
        if let (Some(ch), Ok(bilan)) = (channel.as_ref(), &result) {
            let txt = bilan.trim();
            if !txt.is_empty() {
                livrer_telegram(ch, &format!("📋 Mission \"{slug}\" - iteration {iteration}:\n\n{txt}"))
                    .await;
            }
        }
    });
    iteration
}

/// Minimal text-message delivery to a Telegram channel (`telegram:<chat_id>`).
/// No-op if the channel is not Telegram or the bot is not configured.
async fn livrer_telegram(channel: &str, text: &str) {
    if !channel.starts_with("telegram") {
        return; // other channels: to extend (discord/slack) later
    }
    let chat_id = channel.strip_prefix("telegram:").unwrap_or("").trim();
    if chat_id.is_empty() {
        return;
    }
    let Ok(content) = std::fs::read_to_string(std::path::Path::new("channels-config.json")) else {
        return;
    };
    let Ok(cfg) = serde_json::from_str::<serde_json::Value>(&content) else {
        return;
    };
    let token = cfg["telegram"]["bot_token"].as_str().unwrap_or("");
    if token.is_empty() {
        return;
    }
    let client = reqwest::Client::new();
    let _ = client
        .post(format!("https://api.telegram.org/bot{}/sendMessage", token))
        .json(&serde_json::json!({ "chat_id": chat_id, "text": text }))
        .send()
        .await;
}

/// GET /api/butinage/carnets - lists UNFINISHED butinage notebooks (resumable).
async fn api_carnets_list() -> Json<serde_json::Value> {
    let dir = std::path::Path::new("sessions").join("butinage");
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for e in entries.flatten() {
            let p = e.path();
            if p.extension().and_then(|x| x.to_str()) != Some("json") {
                continue;
            }
            if let Ok(raw) = std::fs::read_to_string(&p) {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) {
                    let id = p
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("")
                        .trim_end_matches(".carnet.json")
                        .to_string();
                    out.push(serde_json::json!({
                        "id": id,
                        "mission": v.get("mission").and_then(|m| m.as_str()).unwrap_or(""),
                        "passe": v.get("passe").and_then(|m| m.as_u64()).unwrap_or(0),
                        "maj_le": v.get("maj_le").cloned().unwrap_or(serde_json::Value::Null),
                    }));
                }
            }
        }
    }
    Json(serde_json::json!({ "carnets": out }))
}

/// POST /api/butinage/carnets/:id/resume - RESUMES an unfinished notebook (background).
async fn api_carnet_resume(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let path = std::path::Path::new("sessions")
        .join("butinage")
        .join(format!("{id}.carnet.json"));
    if !path.exists() {
        return Json(serde_json::json!({ "error": "notebook not found" }));
    }
    let st = state.clone();
    let id_spawn = id.clone();
    tokio::spawn(async move {
        let cfg = st.essaim_config.read().await.clone();
        let (tx, mut rx) = broadcast::channel::<ChatEvent>(64);
        tokio::spawn(async move { while rx.recv().await.is_ok() {} });
        let memoire = Some(st.memoire.clone());
        match laruche_essaim::butinage_pont::reprendre_carnet(
            &path,
            &st.essaim_registry,
            &cfg,
            &tx,
            &memoire,
        )
        .await
        {
            Ok(txt) => {
                laruche_essaim::feed_journal::record(
                    "LaRuche",
                    "mission",
                    "resumed and finished a notebook",
                    id_spawn,
                    chrono::Utc::now(),
                );
                if let Some(ch) = cfg.home_channel.as_ref() {
                    livrer_telegram(ch, &format!("✅ Notebook resumed - finished:\n\n{}", txt.trim()))
                        .await;
                }
            }
            Err(e) => warn!(error = %e, "Notebook resume failed"),
        }
    });
    Json(serde_json::json!({ "status": "resuming", "id": id }))
}

/// POST /api/missions/:slug/run - triggers ONE iteration.
async fn api_run_mission(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(slug): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let Some(mission) = state.missions.read().await.get(&slug) else {
        return Json(serde_json::json!({"error": "mission not found"}));
    };
    let iteration = lancer_iteration_mission(state.clone(), mission).await;
    Json(serde_json::json!({"status": "started", "slug": slug, "iteration": iteration}))
}

/// Contents (items) of a memory node.
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

/// GET /api/missions/:slug/dossier - assembles the mission DOSSIER (synthesis + findings
/// + open questions, from the cognitive map) as markdown ready to read/export.
async fn api_mission_dossier(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(slug): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let Some(mission) = state.missions.read().await.get(&slug) else {
        return Json(serde_json::json!({"error": "mission not found"}));
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

    let mut md = format!("# Mission: {}\n\n", mission.objective);
    md.push_str(&format!(
        "_Iterations: {} - status: {}_\n\n",
        mission.iterations, mission.status
    ));
    if let Some(s) = synthese.last() {
        md.push_str(&format!("## Synthesis\n\n{}\n\n", s));
    }
    if !findings.is_empty() {
        md.push_str("## Findings\n\n");
        for f in &findings {
            md.push_str(&format!("- {}\n", f));
        }
        md.push('\n');
    }
    if !questions.is_empty() {
        md.push_str("## Open questions\n\n");
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

/// POST /api/missions/:slug - updates a mission (status pause/active/done, objective, cadence).
async fn api_update_mission(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(slug): axum::extract::Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let mut store = state.missions.write().await;
    let Some(mut m) = store.get(&slug) else {
        return Json(serde_json::json!({"error": "mission not found"}));
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

/// DELETE /api/missions/:slug - deletes a mission (the metadata; the knowledge stays in memory).
async fn api_delete_mission(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(slug): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let ok = state.missions.write().await.remove(&slug);
    Json(serde_json::json!({"status": if ok {"ok"} else {"not_found"}, "slug": slug}))
}

/// Level-2 orbit - DECOMPOSES a mission into parallel kanban tasks (one per open
/// question, otherwise an angle to cover). The kanban dispatcher executes them (research), each
/// task writing its findings into the mission's subtree. Skills are forged
/// automatically (background_review) each turn. Reuses everything that exists.
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
            "Cover the most important key angle of the objective still not addressed: {}",
            mission.objective
        )]
    } else {
        questions.into_iter().take(max_tasks).collect()
    };
    let mut board = state.kanban_board.write().await;
    let mut n = 0;
    for q in cibles {
        let desc = format!(
            "Mission \"{obj}\". Address this research question: \"{q}\".\n\
             Do thorough web research, then write your SOURCED findings via memory_write \
             under the node_id `{base}.findings` (one fact = one clear item). Be rigorous and factual.",
            obj = mission.objective,
            q = q,
            base = base
        );
        let task = board.create(
            format!("Mission {} - research", mission.slug),
            desc,
            None,
            None,
            None,
            None, // channel: the mission delivers its own result
        );
        board.change_status(task.id, laruche_kanban::TaskStatus::Ready);
        n += 1;
        if n >= max_tasks {
            break;
        }
    }
    n
}

/// POST /api/missions/:slug/decompose - splits the mission into parallel kanban tasks.
async fn api_decompose_mission(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(slug): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let Some(mission) = state.missions.read().await.get(&slug) else {
        return Json(serde_json::json!({"error": "mission not found"}));
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
        // Inject attached skills (same logic as the daemon), skipping
        // skills disabled via the Skills page slider.
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

/// PUT /api/cron/:id - updates a cron (editing / schedule shift).
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

// --- Skills (OKF in memory, capacities.skills.*) - Settings page ----------------

/// GET /api/skills - lists skills (name, description, enabled).
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
                // Load the content to extract the description.
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

/// GET /api/skills/:name - returns the full SKILL.md (OKF).
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

/// POST /api/skills - creates/updates a skill (body: {content} OKF, or {name, content}).
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
                serde_json::json!({"error": "invalid frontmatter (name/description required, type: skill)"}),
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

/// POST /api/skills/:name/toggle - enables/disables a skill (persisted).
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

/// DELETE /api/skills/:name - deletes the skill (node items) + cleans up the state.
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

/// GET /api/watchers - list watchers.
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

/// POST /api/watchers - create a watcher.
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
        channel: body["channel"]
            .as_str()
            .filter(|s| !s.trim().is_empty())
            .map(|s| s.to_string()),
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

    let log_name = watcher.name.clone();
    let mut registry = state.watchers.write().await;
    registry.add(watcher);
    drop(registry);
    laruche_essaim::feed_journal::record(
        "User",
        "watcher",
        "created the watcher",
        log_name,
        chrono::Utc::now(),
    );
    StatusCode::CREATED
}

/// PATCH /api/watchers/:id - updates a watcher's editable fields. Absent key =
/// field unchanged; model/profile_id set to "" = cleared.
async fn api_update_watcher(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    axum::extract::Json(body): axum::extract::Json<serde_json::Value>,
) -> StatusCode {
    let uuid = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => return StatusCode::BAD_REQUEST,
    };
    let watcher_type = body.get("watcher_type").and_then(|v| v.as_str()).map(|s| match s {
        "url" => laruche_watchers::WatcherType::Url,
        "log" => laruche_watchers::WatcherType::Log,
        _ => laruche_watchers::WatcherType::File,
    });
    let s = |k: &str| body.get(k).and_then(|v| v.as_str()).map(|v| v.to_string());
    // Key present -> update (empty value = clear for model/profile_id).
    let opt = |k: &str| {
        body.get(k)
            .map(|v| v.as_str().filter(|x| !x.is_empty()).map(|x| x.to_string()))
    };
    let mut registry = state.watchers.write().await;
    let ok = registry.update(
        &uuid,
        s("name"),
        watcher_type,
        s("target"),
        s("condition"),
        s("prompt"),
        body.get("active").and_then(|v| v.as_bool()),
        opt("model"),
        opt("profile_id"),
        opt("channel"),
    );
    if ok {
        StatusCode::OK
    } else {
        StatusCode::NOT_FOUND
    }
}

/// DELETE /api/watchers/:id - remove a watcher.
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

/// GET /api/kanban - list all tasks
async fn api_kanban_list(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let board = state.kanban_board.read().await;
    Json(serde_json::json!(board.list()))
}

/// POST /api/kanban - create task
async fn api_kanban_create(
    State(state): State<Arc<AppState>>,
    axum::extract::Json(body): axum::extract::Json<serde_json::Value>,
) -> StatusCode {
    let title = body["title"].as_str().unwrap_or("").to_string();
    let description = body["description"].as_str().unwrap_or("").to_string();
    let idempotency_key = body["idempotency_key"].as_str().map(|s| s.to_string());

    let profile_id = body["profile_id"].as_str().map(|s| s.to_string());
    let model = body["model"].as_str().map(|s| s.to_string());
    let channel = body["channel"]
        .as_str()
        .map(|s| s.to_string())
        .filter(|s| !s.trim().is_empty());
    let log_title = title.clone();
    let mut board = state.kanban_board.write().await;
    board.create(title, description, idempotency_key, profile_id, model, channel);
    drop(board);
    laruche_essaim::feed_journal::record(
        "User",
        "kanban",
        "created the kanban task",
        log_title,
        chrono::Utc::now(),
    );
    StatusCode::CREATED
}

/// GET /api/channels/known - known REAL channels (to populate the dropdowns).
/// Aggregates: home channel + cron channels + kanban default/tasks + watchers. Deduplicated.
async fn api_channels_known(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    use std::collections::BTreeSet;
    let mut set: BTreeSet<String> = BTreeSet::new();
    let mut push = |c: Option<String>| {
        if let Some(c) = c {
            let c = c.trim().to_string();
            if !c.is_empty() {
                set.insert(c);
            }
        }
    };
    let home = state.essaim_config.read().await.home_channel.clone();
    push(home.clone());
    for t in state.essaim_cron.read().await.list() {
        push(t.channel.clone());
    }
    {
        let board = state.kanban_board.read().await;
        push(board.default_channel());
        for t in board.list() {
            push(t.channel.clone());
        }
    }
    for w in state.watchers.read().await.list() {
        push(w.channel.clone());
    }
    Json(serde_json::json!({
        "channels": set.into_iter().collect::<Vec<_>>(),
        "home": home,
    }))
}

/// GET /api/kanban/default_channel - board's default channel.
async fn api_kanban_default_channel_get(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let ch = state.kanban_board.read().await.default_channel();
    Json(serde_json::json!({ "channel": ch }))
}

/// POST /api/kanban/default_channel {channel} - sets the board's default channel.
async fn api_kanban_default_channel_set(
    State(state): State<Arc<AppState>>,
    axum::extract::Json(body): axum::extract::Json<serde_json::Value>,
) -> StatusCode {
    let ch = body["channel"].as_str().map(|s| s.to_string());
    state.kanban_board.write().await.set_default_channel(ch);
    StatusCode::OK
}

/// PUT /api/kanban/:id/status - update status
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

/// PUT /api/kanban/:id - update title/description.
async fn api_kanban_update(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    axum::extract::Json(body): axum::extract::Json<serde_json::Value>,
) -> StatusCode {
    if let Ok(uuid) = Uuid::parse_str(&id) {
        let title = body["title"].as_str().map(|s| s.to_string());
        let description = body["description"].as_str().map(|s| s.to_string());
        let mut board = state.kanban_board.write().await;
        // Per-task channel: present in the body (even empty) -> apply it (empty = inherit default).
        if body.get("channel").is_some() {
            board.set_channel(uuid, body["channel"].as_str().map(|s| s.to_string()));
        }
        if board.update(uuid, title, description).is_some() {
            return StatusCode::OK;
        }
    }
    StatusCode::NOT_FOUND
}

/// POST /api/kanban/:id/dependency - block child by parent
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

// System diagnostics endpoint (health check and configuration validation) -> moved to doctor_api.rs

/// GET /api/onboarding - guided setup checklist.
// Runtime settings endpoints (channel/notify/permission/curateur config, secrets vault HTTP layer, MCP server RPC) -> moved to settings_api.rs

/// GET /api/config/channel-models: per-channel model overrides + the available
// Config/settings API handlers moved to config_api.rs (provider, channel models,
// runtime generation levers, compaction, context stats).

// Credential pool API (list, add, delete shared provider credentials) -> moved to credentials_api.rs
// Provider profiles + codex + active model + capabilities API -> moved to profiles_api.rs

// Event log endpoints (list recent events, export as NDJSON) -> moved to events_api.rs

// Authentication endpoints (passkey enroll/challenge, login/logout, password, model selection, QR scan, permanent link) -> moved to auth_api.rs

// Knowledge endpoints -> moved to knowledge_api.rs

// Channel bot management (start/stop/status) and Telegram bot runtime, plus shared channel query helpers -> moved to channels_api.rs

// Discord interaction webhook (slash command and interaction callbacks) -> moved to discord_api.rs

// Slack Events API (url_verification challenge, message and app_mention event callbacks) -> moved to slack_api.rs
// Local/system HTTP endpoints (cwd, local media, onboarding, file suggest, RPC, model preload, webhook) -> moved to local_api.rs
// WebSocket chat handler (interactive streaming chat over WS) and its event serializer helper -> moved to ws_chat.rs

// Voice pipeline (STT/TTS websocket) -> moved to voice_api.rs
// Plugins API (plugin CRUD + plugin file browser) -> moved to plugins_api.rs
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
    // PERSISTENT IDENTITY (identity.json). Without it, node_id = Uuid::new_v4() at EVERY startup:
    // the ruche appears as a NEW node to peers at every reboot (the old one expires) → this is
    // a direct cause of flapping. We load the saved ID, or persist the generated one.
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
                info!(node_id = %id, "Identity loaded (identity.json)");
            }
            None => {
                let _ = std::fs::write(
                    id_path,
                    serde_json::json!({ "node_id": manifest.node_id.to_string() }).to_string(),
                );
                info!(node_id = %manifest.node_id, "New identity persisted (identity.json)");
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

    // PRIVACY NOTE: we NO LONGER announce locally detected backends at startup. The mesh
    // should only expose explicitly public providers (`public_proxy`): it's the re-announce
    // loop (below) that rebuilds the capabilities from the public set only.

    // Feed journal (persistent): loads the history of system events at startup.
    laruche_essaim::feed_journal::init(std::path::PathBuf::from("feed-journal.ndjson"), 500);

    // Secrets vault: decrypts the at-rest file → in-memory view (never re-serialized).
    // Tools/providers substitute `${NAME}` with the real value without showing it to the LLM.
    laruche_essaim::secrets::init(secrets_vault::charger());

    // Gap D: USER HOOKS: loads `hooks.json` (pre/post-tool) if it exists.
    {
        let hooks = std::fs::read_to_string("hooks.json")
            .ok()
            .and_then(|s| serde_json::from_str::<Vec<laruche_essaim::hooks::Hook>>(&s).ok())
            .unwrap_or_default();
        if !hooks.is_empty() {
            eprintln!("🪝 {} user hook(s) loaded from hooks.json", hooks.len());
        }
        laruche_essaim::hooks::init(hooks);
    }

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
    // Overlay persisted state (takes priority: user's runtime choices)
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

    // Cleanup of duplicate profiles (historical bug in /api/models/use that created
    // duplicate "local-<host>" + OpenAI profiles with an empty base_url).
    {
        // 1) Remove OpenAI profiles with an empty base_url (broken, e.g. bogus "local-codex").
        profiles_cfg
            .profiles
            .retain(|_, p| !(p.provider == "openai" && p.base_url.trim().is_empty()));
        // 2) Merge profiles with identical (provider, base_url): keep the 1st (sorted order
        //    → "llamacpp-8001" before "local-llama.cpp"), recover its models, remove.
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
        // 3) Repair active_model if its profile was removed.
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
                "Duplicate profiles cleaned up at startup"
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
    // Wire the mesh signer: the inference path (laruche-essaim) signs its calls to a LAN peer
    // with this node's ed25519 identity → the peer can apply `restricted`.
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
    if let Some(c) = persistent.curateur_actif {
        essaim_config.curateur_actif = c;
    }
    if let Some(d) = persistent.dynamic_tool_selection {
        essaim_config.dynamic_tool_selection = d;
    }
    if persistent.home_channel.is_some() {
        essaim_config.home_channel = persistent.home_channel.clone();
    }
    if let Some(ref m) = persistent.permission_mode {
        if let Some(mode) = settings_api::permission_mode_from_str(m) {
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

    // Cognitive memory (laruche-memoire): env-selectable backend.
    //   LARUCHE_MEMOIRE_BACKEND=sidecar  → real paradigm on http://127.0.0.1:8765
    //   (default)                         → Rust in-memory NativeBackend (zero dependency)
    let memoire: Arc<dyn laruche_memoire::MemoireCognitive> =
        match std::env::var("LARUCHE_MEMOIRE_BACKEND").as_deref() {
            Ok("sidecar") => Arc::new(laruche_memoire::SidecarBackend::loopback()),
            Ok("sqlite") => match std::env::var("LARUCHE_EMBED_URL") {
                // With Ollama embedder → hybrid semantic recall.
                Ok(url) if !url.is_empty() => {
                    let model = std::env::var("LARUCHE_EMBED_MODEL")
                        .unwrap_or_else(|_| "nomic-embed-text".to_string());
                    Arc::new(
                        laruche_memoire::SqliteBackend::open_with_embedder(
                            "memoire.db",
                            Arc::new(laruche_memoire::OllamaEmbedder::new(url, model)),
                        )
                        .expect("opening memoire.db (SQLite+FTS5+embeddings)"),
                    )
                }
                // Without embedder → FTS5 lexical recall.
                _ => Arc::new(
                    laruche_memoire::SqliteBackend::open("memoire.db")
                        .expect("opening memoire.db (SQLite+FTS5)"),
                ),
            },
            _ => Arc::new(laruche_memoire::NativeBackend::new()),
        };
    laruche_essaim::abeilles::enregistrer_memoire(&essaim_registry, memoire.clone());
    // LLM consolidation (item merging): requires memory + config (aux model).
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
    // SELF-IMPROVEMENT tools (forge): skill_file_*, plugin_*, mcp_*. The main registry
    // is passed so plugin_create/delete reload in the right place.
    laruche_essaim::abeilles::enregistrer_forge(&essaim_registry, essaim_registry.clone());
    essaim_registry.enregistrer(Box::new(abeilles_local::AbeilleMeshSend));

    // Migration `tools.* → capacities.*` (idempotent, run at every boot but no-op afterwards).
    // The forged skills (real data) are PRESERVED; tools.abeilles (a mere projection)
    // is purged then recreated by the indexer under capacities.tools/plugins/mcp.
    match memoire
        .renommer_sous_arbre("tools.skills", "capacities.skills")
        .await
    {
        Ok(n) if n > 0 => tracing::info!(noeuds = n, "migration skills -> capacities.skills"),
        Ok(_) => {}
        Err(e) => tracing::warn!(error = %e, "skills migration skipped (backend without support)"),
    }
    let _ = memoire.supprimer_sous_arbre("tools").await; // purge the remaining legacy projection

    // Map nodes (virtual .md files). Created empty if absent (idempotent).
    // `capacities.*` = tool ecosystem (protected); `system.*` = editable prompt/SOUL base.
    for (id, label, desc) in [
        (
            "capacities",
            "Capacities",
            "Ecosystem: tools, plugins, MCP, skills",
        ),
        ("capacities.tools", "Tools", "Native tools (builtin)"),
        (
            "capacities.plugins",
            "Plugins",
            "Custom tools (JSON plugins)",
        ),
        (
            "capacities.mcp",
            "MCP",
            "Tools served by MCP servers",
        ),
        ("capacities.skills", "Skills", "Learned OKF procedures"),
        (
            "system",
            "System",
            "Editable sections of the system prompt (hot-reload, no restart)",
        ),
        (
            "system.prompt",
            "Identity",
            "Editable identity / persona (empty = code default)",
        ),
        (
            "system.behavior",
            "Behavior",
            "Editable behavior rules (empty = code default)",
        ),
        (
            "system.soul",
            "SOUL",
            "Injectable personalization layer (frontmatter enabled)",
        ),
        (
            "system.prompt_curateur",
            "Curateur Prompt",
            "Self-improvement curateur prompt (empty = code default, hot-reload)",
        ),
        (
            "system.prompt_extraction",
            "Consolidation Prompt",
            "Memory / escale consolidation prompt (empty = code default, hot-reload)",
        ),
        (
            "system.prompt_planning",
            "Planning Prompt",
            "Planning section of the system prompt (empty = code default, hot-reload)",
        ),
    ] {
        let _ = memoire
            .create_node(id, label, Some(desc), Some(1.0), None)
            .await;
    }

    // Default "web_research" skill (search→evaluate→fetch→synthesize procedure): seeded
    // once if absent, so web research goes beyond the snippets.
    {
        // Version marker: re-seed once if the old version (v1) is in place.
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
            let skill = "---\ntype: skill\nname: web_research\nversion: web_research-v2\ndescription: Deep multi-step web research (search, evaluate, FETCH the pages, synthesize with sources)\n---\n\n# Deep web research\n\n## When to use it\nAny request for up-to-date, factual or detailed info from the web (news, papers, docs, comparisons, scores, prices...).\n\n## Procedure (DO NOT loop on searching)\n1. ONE broad search: `web_deep_search` with a precise query.\n2. SPOT in the results the reliable and NON-blocked URLs (arxiv.org, blogs, official docs). Ignore domains that return 400/403/Forbidden.\n3. GO DEEPER: `web_fetch` on 1 to 3 of these URLs to read the FULL PAGE: that is where the detail is, not in the snippets.\n4. If a key piece of info is missing: ONE DIFFERENT refined search (never the same query), then re-fetch.\n5. SYNTHESIZE while citing the source URLs. Flag uncertainties/contradictions.\n\n## Strict rules\n- At most ~2 web_deep_search; beyond that, move to `web_fetch` on precise URLs.\n- NEVER re-run a query nearly identical to the previous one.\n- A page returns 400/403/Forbidden -> drop it, do not insist on it.\n- Always `web_fetch` at least one primary source (arxiv, official site) before concluding.\n- Memorize (memory_write) a useful durable fact if relevant.\n";
            let _ = memoire
                .write(
                    laruche_memoire::MemoryItem::new("capacities.skills.web_research", skill)
                        .with_source("seed"),
                )
                .await;
        }
    }

    // Index the tool registry into the map (capacities.*) RIGHT FROM startup, incrementally,
    // so any new tool is visible in memory and semantically retrievable.
    // (MCP tools, loaded below, are indexed on the 1st chat turn via the same call.)
    if let Err(e) =
        laruche_essaim::brain::indexer_abeilles_memoire(&essaim_registry, &memoire).await
    {
        tracing::warn!(error = %e, "tool indexing at startup skipped");
    }

    // Phase 1: flat-file layer: disk → SQL sync of skills (skills/<slug>/SKILL.md).
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
            "nomic-embed-text", // Default embedding model: user should pull it
        ),
    ));
    // Fix A: knowledge_add/knowledge_search REMOVED: it was a 2nd memory system
    // (flat KnowledgeBase/RAG) DUPLICATING the cognitive map. Everything now goes through
    // memory_write / memory_search (the cognitive memory = LaRuche's differentiator).
    let _ = &kb; // kb kept for rag.rs (legacy RAG), but no longer exposed as an agent tool.

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
        .route("/", get(web::spa_page))
        .route("/app.css", get(web::app_css))
        .route("/app.js", get(web::app_js))
        .route("/lang/:file", get(web::lang_file))
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
        .route("/api/events", get(events_api::api_get_events))
        .route("/api/events/export", get(events_api::api_export_events))
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
        .route("/dashboard", get(web::spa_page))
        .route("/chat", get(web::spa_page))
        .route("/control", get(web::spa_page))
        .route("/app", get(web::spa_page))
        .route("/ws/chat", get(ws_chat::ws_chat_handler))
        .route("/ws/audio", get(voice_api::ws_audio_handler))
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
        .route("/api/mesh/skills", get(api_mesh_skills_list))
        .route("/api/mesh/skills/:slug", get(api_mesh_skill_get))
        .route("/api/mesh/sync", post(api_mesh_skills_sync))
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
        .route("/api/webhook", post(local_api::api_webhook))
        .route("/api/preload", post(local_api::api_preload))
        .route("/api/rpc", post(local_api::api_rpc))
        .route("/api/files/suggest", get(local_api::api_files_suggest))
        .route("/api/onboarding", get(local_api::api_onboarding))
        .route("/api/cwd", get(local_api::api_get_cwd).post(local_api::api_set_cwd))
        .route("/api/media/local", get(local_api::api_media_local))
        .route(
            "/api/config/channels",
            get(settings_api::api_get_channels_config).post(settings_api::api_save_channels_config),
        )
        .route(
            "/api/config/notify",
            get(settings_api::api_get_notify_config).post(settings_api::api_set_notify_config),
        )
        .route(
            "/api/config/provider",
            get(config_api::api_get_provider_config).post(config_api::api_save_provider_config),
        )
        .route(
            "/api/config/channel-models",
            get(config_api::api_get_channel_models).post(config_api::api_save_channel_model),
        )
        .route("/api/context/stats", get(config_api::api_get_context_stats))
        .route(
            "/api/config/compaction",
            get(config_api::api_get_compaction_config).post(config_api::api_set_compaction_config),
        )
        .route(
            "/api/config/runtime",
            get(config_api::api_get_runtime_config).post(config_api::api_set_runtime_config),
        )
        .route(
            "/api/config/permission",
            get(settings_api::api_get_permission_config).post(settings_api::api_set_permission_config),
        )
        .route(
            "/api/config/curateur",
            get(settings_api::api_get_curateur_config).post(settings_api::api_set_curateur_config),
        )
        .route(
            "/api/secrets",
            get(settings_api::api_secrets_list).post(settings_api::api_secrets_set),
        )
        .route("/api/secrets/:name", axum::routing::delete(settings_api::api_secrets_delete))
        .route("/mcp", post(settings_api::api_mcp_server))
        .route(
            "/api/profiles",
            get(profiles_api::api_get_profiles).post(profiles_api::api_upsert_profile),
        )
        .route(
            "/api/credentials",
            get(credentials_api::api_get_credentials)
                .post(credentials_api::api_add_credential)
                .delete(credentials_api::api_delete_credential),
        )
        .route("/api/profiles/models", get(profiles_api::api_get_unified_models))
        .route("/api/profiles/active", post(profiles_api::api_set_active_model))
        .route("/api/profiles/:id/visibility", post(profiles_api::api_set_visibility))
        .route("/api/models/use", post(profiles_api::api_models_use))
        .route(
            "/api/capabilities/selection",
            get(profiles_api::api_capabilities_selection),
        )
        .route(
            "/api/missions",
            get(api_list_missions).post(api_create_mission),
        )
        .route("/api/missions/:slug/run", post(api_run_mission))
        .route("/api/butinage/carnets", get(api_carnets_list))
        .route("/api/butinage/carnets/:id/resume", post(api_carnet_resume))
        .route("/api/missions/:slug/dossier", get(api_mission_dossier))
        .route("/api/missions/:slug/decompose", post(api_decompose_mission))
        .route(
            "/api/missions/:slug",
            post(api_update_mission).delete(api_delete_mission),
        )
        .route(
            "/api/profiles/:id",
            axum::routing::delete(profiles_api::api_delete_profile),
        )
        .route("/api/services/register", post(api_register_service))
        .route(
            "/api/services/register/:name",
            axum::routing::delete(api_unregister_service),
        )
        .route("/api/auth/codex/status", get(profiles_api::api_codex_status))
        .route("/api/auth/codex/start", post(profiles_api::api_codex_start))
        .route("/api/auth/codex/logout", post(profiles_api::api_codex_logout))
        .route("/api/channels/start", post(channels_api::api_start_channel))
        .route("/api/channels/stop", post(channels_api::api_stop_channel))
        .route("/api/channels/status", get(channels_api::api_channels_status))
        .route(
            "/api/knowledge",
            get(knowledge_api::api_list_knowledge).post(knowledge_api::api_add_knowledge),
        )
        .route(
            "/api/knowledge/:id",
            axum::routing::delete(knowledge_api::api_delete_knowledge).put(knowledge_api::api_update_knowledge),
        )
        .route("/api/doctor", get(doctor_api::api_doctor))
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
            axum::routing::patch(api_update_watcher).delete(api_delete_watcher),
        )
        .route("/api/channels/known", get(api_channels_known))
        .route(
            "/api/kanban/default_channel",
            get(api_kanban_default_channel_get).post(api_kanban_default_channel_set),
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
            get(plugins_api::api_plugin_get)
                .post(plugins_api::api_plugin_save)
                .delete(plugins_api::api_plugin_delete),
        )
        .route("/api/plugin-files", get(plugins_api::api_plugin_files))
        .route(
            "/api/plugin-file/*path",
            get(plugins_api::api_plugin_file_get)
                .post(plugins_api::api_plugin_file_save)
                .delete(plugins_api::api_plugin_file_delete),
        )
        .route("/api/channels/discord/webhook", post(discord_api::api_discord_webhook))
        .route("/api/channels/slack/events", post(slack_api::api_slack_events))
        // Auth routes
        .route("/api/auth/enroll", post(auth_api::api_auth_enroll))
        .route("/api/auth/me", get(auth_api::api_auth_me))
        .route("/api/auth/challenge", get(auth_api::api_auth_challenge))
        .route("/api/auth/status/:id", get(auth_api::api_auth_status))
        .route("/api/auth/logout", post(auth_api::api_auth_logout))
        .route("/api/auth/login", post(auth_api::api_auth_login))
        .route("/api/auth/password", post(auth_api::api_auth_set_password))
        .route("/api/auth/model", post(auth_api::api_auth_set_model))
        .route("/auth/scan/:id", get(auth_api::auth_scan_challenge))
        .route("/auth/link/:user_id/:secret", get(auth_api::auth_permanent_link))
        .route("/login", get(web::spa_page))
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

    // Boot resume: purge stale butinage notebooks (crashed/abandoned missions)
    // and log the still-recent ones (potentially resumable). Successful missions already
    // deleted their notebook (see butinage_pont::executer).
    purger_carnets_au_boot();

    // Background: periodic memory dream (consolidation + dedup): anti-bloat hygiene.
    // Long interval (6 h by default), 1st pass deferred by 10 min so as not to load
    // startup. Disableable via LARUCHE_DREAM_INTERVAL_SECS=0.
    {
        let dream_state = state.clone();
        let secs: u64 = std::env::var("LARUCHE_DREAM_INTERVAL_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(6 * 3600);
        if secs > 0 {
            tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(600)).await;
                let mut interval = tokio::time::interval(std::time::Duration::from_secs(secs));
                loop {
                    interval.tick().await;
                    match dream_state.memoire.dream().await {
                        Ok(_) => info!("Periodic memory dream finished (consolidation + dedup)"),
                        Err(e) => warn!(error = %e, "Periodic memory dream failed"),
                    }
                }
            });
        }
    }

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
                    // Full resolution via the profile (provider + key + base_url + model).
                    profiles_api::appliquer_profil(&cron_state, &mut cron_config, &pid, model.as_deref()).await;
                } else if let Some(p) = provider {
                    // Legacy fallback: raw provider/model (key/URL from the active config).
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

                // ANTI-REPLICATION: a run TRIGGERED by a cron must NOT be able to create
                // other scheduled tasks (cron/watcher/mission/kanban). Otherwise a prompt like
                // "test message for the cron" recreates a cron → which re-fires →
                // infinite loop of phantom crons. We disable these tools for this run.
                for t in [
                    "cron_create", "cron_delete", "watcher_create", "mission_create",
                    "kanban_create",
                ] {
                    if !cron_config.disabled_tools.iter().any(|d| d == t) {
                        cron_config.disabled_tools.push(t.to_string());
                    }
                }
                // ANTI-RUNAWAY: a cron is a short, targeted task. We cap its passes
                // low (≤ 12): otherwise a vague prompt ("write a test message") loops
                // the agent up to the global cap (100): writes/re-reads/rewrites the log endlessly,
                // hence the "100-pass cap reached" and the spam.
                cron_config.max_iterations = cron_config.max_iterations.min(12);

                let current_model = cron_config.model.clone();
                let sessions_dir = std::path::Path::new("sessions");
                let mut session = Session::new_with_path(&current_model, sessions_dir);
                let (tx, mut rx) = broadcast::channel::<ChatEvent>(64);

                // Don't drop the receiver (drain)
                tokio::spawn(async move { while let Ok(_) = rx.recv().await {} });

                // Batch 10.B: injection of attached skills: loads each OKF SKILL.md
                // from capacities.skills.<name> and assembles it at the head of the prompt (skills
                // disabled via the Skills page slider = skipped).
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

                // Delivery channel: ONLY the task's own. NO home_channel fallback
                // (otherwise a channel-less test cron spams Telegram). A cron created FROM Telegram
                // already captures ctx.channel=telegram → "notify me" works; a cron created
                // in the UI without a channel stays silent (feed/UI only).
                let delivery_channel = channel.filter(|s| !s.is_empty());
                if let Some(ch) = delivery_channel {
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
                let (w_profile, w_model, w_channel) = {
                    let reg = watcher_state.watchers.read().await;
                    reg.list()
                        .into_iter()
                        .find(|w| w.id == watcher_id)
                        .map(|w| (w.profile_id.clone(), w.model.clone(), w.channel.clone()))
                        .unwrap_or((None, None, None))
                };
                let mut config = watcher_state.essaim_config.read().await.clone();
                if let Some(pid) = w_profile {
                    profiles_api::appliquer_profil(&watcher_state, &mut config, &pid, w_model.as_deref()).await;
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

                // Delivery: watcher channel → home channel.
                let livr_channel = match w_channel {
                    Some(c) => Some(c),
                    None => watcher_state.essaim_config.read().await.home_channel.clone(),
                };
                if let (Some(ch), Ok(res)) = (livr_channel, &result) {
                    livrer_telegram(&ch, &format!("🔔 Watcher triggered\n\n{}", res)).await;
                }

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

    // Background: periodic mDNS re-announce (P4): reflects the REAL models (active +
    // detected local backends + public_proxy providers), picks up backends started hot,
    // and fixes the announcement of the frozen default model.
    let mdns_broadcaster = broadcaster.clone();
    let mdns_state = state.clone();
    tokio::spawn(async move {
        // Re-announce every 30s (< PEER_STALE_SECS=90) → stable presence, no more flapping.
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
        interval.tick().await; // skip the immediate tick
        loop {
            interval.tick().await;
            let mut manifest = mdns_state.manifest.read().await.clone();
            manifest.capabilities = Default::default();
            // MESH PRIVACY: we announce ONLY what is EXPLICITLY public (`public_proxy`
            // providers). We NO LONGER auto-announce detected local backends (leak: a peer
            // saw all your llama.cpp), and the Agent's model is disclosed only if it is public.
            let public_models: std::collections::HashSet<String> = {
                let pcfg = mdns_state.profiles.read().await;
                pcfg.profiles
                    .iter()
                    .filter(|(_, p)| p.visibilite == profiles::Visibilite::PublicProxy)
                    .flat_map(|(_, p)| p.models.iter().cloned())
                    .collect()
            };
            // Agent = presence of an agent in the swarm. Model name hidden if not public.
            let active_model = get_llm_default(&mdns_state).await;
            let agent_model = if public_models.contains(&active_model) {
                active_model
            } else {
                "(private)".to_string()
            };
            manifest.capabilities.add(CapabilityInfo {
                capability: Capability::Agent,
                model_name: agent_model,
                model_size: None,
                quantization: None,
                max_context_length: Some(8192),
            });
            // public_proxy AND restricted providers → announced (gateway; key never broadcast).
            // restricted ones are visible (authorized peers must discover them) but access
            // is controlled at use time (P3 checks the caller's identity against allowed_peers).
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
                tracing::warn!(error = %e, "mDNS re-announce failed");
            }
        }
    });

    // Background: tick of long-running MISSIONS ("La Reine"): every 60s, launches an
    // iteration of active missions whose cron cadence is due (e.g. weekly research).
    let mission_state = state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        interval.tick().await; // skip the immediate tick
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
                tracing::info!(mission = %mission.slug, "Mission iteration (cadence)");
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
                // PLANNING board: we auto-execute ONLY the tasks
                // explicitly promoted to `Ready` (surgical selection →
                // `Running`). The `Todo` items created by the agent/user stay
                // visible until they are promoted (otherwise the daemon would
                // grab them all in 5 s → "empty" board). To launch a task:
                // drag it into the "Ready" column.
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
                    profiles_api::appliquer_profil(
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
                // Delivery: task channel → board default → home channel.
                let task_channel = board.effective_channel(kanban_task.id);
                drop(board);
                let livr_channel = match task_channel {
                    Some(c) => Some(c),
                    None => kanban_state.essaim_config.read().await.home_channel.clone(),
                };
                if let (Some(ch), Ok(res)) = (livr_channel, &result) {
                    livrer_telegram(&ch, &format!("✅ Kanban « {} »\n\n{}", kanban_task.title, res))
                        .await;
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

    // Background: Dream (auto on inactivity + background review)
    let dream_state = state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        let mut last_dreamed = std::time::Instant::now();
        loop {
            interval.tick().await;
            let last_activity = *dream_state.last_activity.read().await;
            let idle_duration = last_activity.elapsed();

            if idle_duration > std::time::Duration::from_secs(300) && last_dreamed < last_activity {
                tracing::info!("System idle for > 5min, triggering Dream mode...");
                let _ = dream_state.events.write().await.emit(
                    laruche_events::EventKind::SystemStatus,
                    "dream_task",
                    serde_json::json!({"status": "dreaming", "idle_secs": idle_duration.as_secs()}),
                );

                let memoire = dream_state.memoire.clone();
                if let Err(e) = memoire.dream().await {
                    tracing::warn!("Error during dream: {}", e);
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
            info!("Shutting down: saving persistent state...");
            save_persistent_state(&shutdown_state).await;
            std::process::exit(0);
        }
    });

    let addr = format!("0.0.0.0:{}", config.api_port);
    info!("LaRuche ready → http://localhost:{}", config.api_port);

    // Sync essaim config from active profile at startup
    profiles_api::sync_essaim_from_profiles(&state).await;

    // L3 (slice 2): AUTO memory SYNC from peer nodes (Miel), every 5 min: each
    // node pulls+dedups the others' facts → COLLECTIVE memory of the ruche, without cloud.
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

    // Phase 1.5: live WATCHER of SKILL.md: a modified file is re-synced to SQL
    // without reboot (8s poll, incremental by mtime).
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
                        continue; // 1st pass = init; the boot already synced everything
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
                                channels_api::run_telegram_bot(&token, &allowed, &state_for_tg).await;
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
                info!(cert = %cert_path, key = %key_path, "TLS enabled: starting HTTPS server");
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

        // TUI exited: save state and shutdown
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
                info!(cert = %cert_path, key = %key_path, "TLS enabled: starting HTTPS server");
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
                info!("Ctrl+C received: shutting down...");
            }
            _ = tray_shutdown_rx => {
                info!("Quit from system tray: shutting down...");
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

/// At startup: purges stale butinage notebooks (checkpoints of crashed/abandoned
/// missions, > 3 days) and logs the still-recent ones (potentially resumable).
/// Successful missions already delete their notebook at the end.
fn purger_carnets_au_boot() {
    let dir = std::path::Path::new("sessions").join("butinage");
    let entries = match std::fs::read_dir(&dir) {
        Ok(e) => e,
        Err(_) => return, // no folder = nothing to do
    };
    let max_age = std::time::Duration::from_secs(3 * 24 * 3600); // 3 days
    let now = std::time::SystemTime::now();
    let (mut purges, mut repris) = (0u32, 0u32);
    for e in entries.flatten() {
        let p = e.path();
        if p.extension().and_then(|x| x.to_str()) != Some("json") {
            continue;
        }
        let age = e
            .metadata()
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| now.duration_since(t).ok())
            .unwrap_or_default();
        if age > max_age {
            if std::fs::remove_file(&p).is_ok() {
                purges += 1;
            }
        } else {
            repris += 1;
        }
    }
    if purges > 0 || repris > 0 {
        info!(purges, repris, "Butinage notebooks: cleanup at startup");
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
            settings_api::permission_mode_to_str(state.essaim_config.read().await.permission_mode).to_string(),
        ),
        saved_at: chrono::Utc::now().to_rfc3339(),
        cookie_secret: Some(auth_user::cookie_secret_to_base64(&state.cookie_secret)),
        context_max_messages: Some(state.essaim_config.read().await.context_max_messages),
        compaction_threshold: Some(state.essaim_config.read().await.compaction_threshold),
        context_max_tokens: Some(state.essaim_config.read().await.context_max_tokens),
        curateur_actif: Some(state.essaim_config.read().await.curateur_actif),
        dynamic_tool_selection: Some(state.essaim_config.read().await.dynamic_tool_selection),
        home_channel: state.essaim_config.read().await.home_channel.clone(),
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
