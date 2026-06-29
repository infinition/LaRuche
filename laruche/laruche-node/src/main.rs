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
mod kanban_api;
mod watchers_api;
mod skills_api;
mod missions_api;
mod sessions_api;
mod memory_api;
mod feed_api;
mod mesh_api;
mod changes_api;
mod memory_crud_api;
mod tools_api;
mod openai_api;
mod swarm_api;
mod mcp_api;
mod status_api;
mod blueprints_api;
mod reine_api;

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
pub(crate) struct MetricsHistoryResponse {
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

// Blueprint endpoints (list/create/delete parameterized cron automation templates, instantiate) -> moved to blueprints_api.rs

// ======================== Handlers ========================

// Node and swarm API (status, discovered nodes, swarm view, inference, model lists, auth request/approve, default model, activity feed, health, service register) -> moved to swarm_api.rs
// OpenAI-compatible chat completions endpoint with signed peer verification -> moved to openai_api.rs

// System status endpoints (voice STT/TTS availability, metrics history) -> moved to status_api.rs


// Tool registry endpoints (list tools, get/save tool enablement config) -> moved to tools_api.rs

// Cognitive memory CRUD (search, write, enrich, node create/update/move/delete, review, dream, consolidate, grep) -> moved to memory_crud_api.rs

// Memory change-sync (disk-to-SQL skill sync, OKF change import/export, mesh pull) and state version endpoint -> moved to changes_api.rs

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

// Mesh messaging (Phase 4 DM between instances/users): identity/peers, mesh skills sync, send/receive, local inbox storage -> moved to mesh_api.rs

// Feed endpoints (feed poll, ask LaRuche from the feed, profile get/save, system prompt defaults) -> moved to feed_api.rs

// Memory endpoints (cognitive-map tree, stats, mutations, OKF and zip export, OKF import) -> moved to memory_api.rs

// Session endpoints (list, delete, messages, search, export, fork) and the client-facing message display helpers with their tests -> moved to sessions_api.rs

// Mission, cron and subagent endpoints (cron CRUD/run, mission CRUD/run/decompose, subagent spawn, notebooks, mission iteration runtime) -> moved to missions_api.rs

// --- Skills (OKF in memory, capacities.skills.*) - Settings page ----------------

/// GET /api/skills - lists skills (name, description, enabled).
// Skill endpoints (list, get, upsert, toggle, delete agent skills) -> moved to skills_api.rs

// Watcher endpoints (list, create, update, delete file/event watchers) -> moved to watchers_api.rs

// Kanban board endpoints (task list/create/update/status/dependency/delete, default channel, known channels) -> moved to kanban_api.rs

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

// MCP server registry endpoints (list, save, delete configured MCP servers) -> moved to mcp_api.rs

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
        (
            "system.prompt_reine",
            "LaReine Prompt",
            "LaReine supervisor rubric (empty = code default, hot-reload)",
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
    changes_api::sync_skills_disk_to_sql(&memoire).await;

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

    // Mirror the saved LaReine gate into the process-global at boot, so self-created
    // skills are held for approval even before the first chat turn (cron/curateur).
    laruche_essaim::reine_queue::definir_gate(reine_api::charger_reine_settings().queue_gate);

    let app = Router::new()
        .route("/", get(web::spa_page))
        .route("/app.css", get(web::app_css))
        .route("/app.js", get(web::app_js))
        .route("/lang/:file", get(web::lang_file))
        .route("/api/status", get(swarm_api::get_status))
        .route(
            "/api/blueprints",
            get(blueprints_api::get_blueprints).post(blueprints_api::api_create_blueprint),
        )
        .route(
            "/api/blueprints/:id",
            axum::routing::delete(blueprints_api::api_delete_blueprint),
        )
        .route("/api/blueprints/:id/instancier", post(blueprints_api::instancier_blueprint))
        .route("/api/events", get(events_api::api_get_events))
        .route("/api/events/export", get(events_api::api_export_events))
        .route("/health", get(swarm_api::health))
        .route("/nodes", get(swarm_api::get_nodes))
        .route("/swarm", get(swarm_api::get_swarm))
        .route("/swarm/models", get(swarm_api::get_swarm_models))
        .route("/models", get(swarm_api::get_models))
        .route("/activity", get(swarm_api::get_activity))
        .route("/infer", post(swarm_api::post_infer))
        .route("/v1/chat/completions", post(openai_api::api_v1_chat_completions))
        .route("/auth/request", post(swarm_api::post_auth_request))
        .route("/auth/approve", post(swarm_api::post_auth_approve))
        .route(
            "/config/default_model",
            get(swarm_api::get_default_model).post(swarm_api::post_set_default_model),
        )
        .route("/metrics/history", get(status_api::get_metrics_history))
        .route("/dashboard", get(web::spa_page))
        .route("/chat", get(web::spa_page))
        .route("/control", get(web::spa_page))
        .route("/app", get(web::spa_page))
        .route("/ws/chat", get(ws_chat::ws_chat_handler))
        .route("/ws/audio", get(voice_api::ws_audio_handler))
        .route("/api/tools", get(tools_api::api_list_tools))
        .route(
            "/api/tools/config",
            get(tools_api::api_get_tools_config).post(tools_api::api_save_tools_config),
        )
        .route("/api/memory/search", get(memory_crud_api::api_memory_search))
        .route("/api/memory/node/:id", get(memory_crud_api::api_memory_node))
        .route("/api/memory/suggest", get(memory_crud_api::api_memory_suggest))
        .route("/api/memory/proposed", get(memory_crud_api::api_memory_proposed))
        .route("/api/memory/write", post(memory_crud_api::api_memory_write))
        .route("/api/memory/enrich", post(memory_crud_api::api_memory_enrich))
        .route("/api/memory/update", post(memory_crud_api::api_memory_update))
        .route("/api/memory/delete", post(memory_crud_api::api_memory_delete))
        .route("/api/memory/node/create", post(memory_crud_api::api_memory_node_create))
        .route("/api/memory/node/update", post(memory_crud_api::api_memory_node_update))
        .route("/api/memory/node/move", post(memory_crud_api::api_memory_node_move))
        .route("/api/memory/node/delete", post(memory_crud_api::api_memory_node_delete))
        .route("/api/memory/move", post(memory_crud_api::api_memory_move))
        .route("/api/memory/review", post(memory_crud_api::api_memory_review))
        .route("/api/memory/dream", post(memory_crud_api::api_memory_dream))
        .route("/api/memory/consolidate", post(memory_crud_api::api_memory_consolidate))
        .route("/api/feed", get(feed_api::api_feed))
        .route("/api/feed/ask", post(feed_api::api_feed_ask))
        .route("/api/mesh/whoami", get(mesh_api::api_mesh_whoami))
        .route("/api/mesh/identity", get(mesh_api::api_mesh_identity))
        .route("/api/mesh/code", get(mesh_api::api_mesh_code_get).post(mesh_api::api_mesh_code_set))
        .route("/api/mesh/peers", get(mesh_api::api_mesh_peers))
        .route("/api/mesh/skills", get(mesh_api::api_mesh_skills_list))
        .route("/api/mesh/skills/:slug", get(mesh_api::api_mesh_skill_get))
        .route("/api/mesh/sync", post(mesh_api::api_mesh_skills_sync))
        .route("/api/mesh/send", post(mesh_api::api_mesh_send))
        .route("/api/mesh/receive", post(mesh_api::api_mesh_receive))
        .route("/api/inbox", get(mesh_api::api_inbox_get))
        .route("/api/inbox/read", post(mesh_api::api_inbox_read))
        .route("/api/profile", get(feed_api::api_profile_get).post(feed_api::api_profile_save))
        .route("/api/memory/grep", get(memory_crud_api::api_memory_grep))
        .route("/api/memory/export_changes", get(changes_api::api_memory_export_changes))
        .route("/api/memory/import_changes", post(changes_api::api_memory_import_changes))
        .route("/api/memory/mesh_pull", post(changes_api::api_memory_mesh_pull))
        .route("/api/state/version", get(changes_api::api_state_version))
        .route("/api/memory/tree", get(memory_api::api_memory_tree))
        .route(
            "/api/system/prompt-defaults",
            get(feed_api::api_system_prompt_defaults),
        )
        .route("/api/memory/stats", get(memory_api::api_memory_stats))
        .route("/api/memory/mutations", get(memory_api::api_memory_mutations))
        .route("/api/memory/export_okf", get(memory_api::api_memory_export_okf))
        .route("/api/memory/export.zip", get(memory_api::api_memory_export_zip))
        .route("/api/sessions", get(sessions_api::api_list_sessions))
        .route("/api/sessions/search", get(sessions_api::api_search_sessions))
        .route("/api/sessions/:id/messages", get(sessions_api::api_get_session_messages))
        .route("/api/voice/status", get(status_api::api_voice_status))
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
            "/api/config/reine",
            get(reine_api::api_get_reine_config).post(reine_api::api_set_reine_config),
        )
        .route("/api/reine/proposals", get(reine_api::api_list_proposals))
        .route(
            "/api/reine/proposals/apply-safe",
            post(reine_api::api_approve_safe),
        )
        .route(
            "/api/reine/proposals/:id/approve",
            post(reine_api::api_approve_proposal),
        )
        .route(
            "/api/reine/proposals/:id/reject",
            post(reine_api::api_reject_proposal),
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
        .route("/api/profiles/:id/test", post(profiles_api::api_test_profile))
        .route("/api/models/use", post(profiles_api::api_models_use))
        .route(
            "/api/capabilities/selection",
            get(profiles_api::api_capabilities_selection),
        )
        .route(
            "/api/missions",
            get(missions_api::api_list_missions).post(missions_api::api_create_mission),
        )
        .route("/api/missions/:slug/run", post(missions_api::api_run_mission))
        .route("/api/butinage/carnets", get(missions_api::api_carnets_list))
        .route("/api/butinage/carnets/:id/resume", post(missions_api::api_carnet_resume))
        .route("/api/missions/:slug/dossier", get(missions_api::api_mission_dossier))
        .route("/api/missions/:slug/decompose", post(missions_api::api_decompose_mission))
        .route(
            "/api/missions/:slug",
            post(missions_api::api_update_mission).delete(missions_api::api_delete_mission),
        )
        .route(
            "/api/profiles/:id",
            axum::routing::delete(profiles_api::api_delete_profile),
        )
        .route("/api/services/register", post(swarm_api::api_register_service))
        .route(
            "/api/services/register/:name",
            axum::routing::delete(swarm_api::api_unregister_service),
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
        .route("/api/sessions/:id/export", get(sessions_api::api_export_session))
        .route("/api/sessions/:id/fork", post(sessions_api::api_fork_session))
        .route(
            "/api/sessions/:id",
            axum::routing::delete(sessions_api::api_delete_session),
        )
        .route("/api/agents/spawn", post(missions_api::api_spawn_subagent))
        .route("/api/cron", get(missions_api::api_list_cron).post(missions_api::api_create_cron))
        .route(
            "/api/cron/:id",
            axum::routing::delete(missions_api::api_delete_cron).put(missions_api::api_update_cron),
        )
        .route("/api/cron/:id/run", post(missions_api::api_run_cron))
        .route("/api/skills", get(skills_api::api_list_skills).post(skills_api::api_upsert_skill))
        .route(
            "/api/skills/:name",
            get(skills_api::api_get_skill).delete(skills_api::api_delete_skill),
        )
        .route("/api/skills/:name/toggle", post(skills_api::api_toggle_skill))
        .route(
            "/api/watchers",
            get(watchers_api::api_list_watchers).post(watchers_api::api_create_watcher),
        )
        .route(
            "/api/watchers/:id",
            axum::routing::patch(watchers_api::api_update_watcher).delete(watchers_api::api_delete_watcher),
        )
        .route("/api/channels/known", get(kanban_api::api_channels_known))
        .route(
            "/api/kanban/default_channel",
            get(kanban_api::api_kanban_default_channel_get).post(kanban_api::api_kanban_default_channel_set),
        )
        .route("/api/kanban", get(kanban_api::api_kanban_list).post(kanban_api::api_kanban_create))
        .route(
            "/api/kanban/:id",
            axum::routing::delete(kanban_api::api_kanban_delete).put(kanban_api::api_kanban_update),
        )
        .route(
            "/api/kanban/:id/status",
            axum::routing::put(kanban_api::api_kanban_update_status),
        )
        .route(
            "/api/kanban/:id/dependency",
            post(kanban_api::api_kanban_add_dependency),
        )
        .route("/api/memory/import_okf", post(memory_api::api_memory_import_okf))
        .route("/api/mcp", post(mcp::api_mcp_handler))
        .route("/api/mcp/servers", get(mcp_api::api_mcp_list_servers))
        .route(
            "/api/mcp/servers/:name",
            post(mcp_api::api_mcp_save_server).delete(mcp_api::api_mcp_delete_server),
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
                    missions_api::livrer_telegram(&ch, &format!("🔔 Watcher triggered\n\n{}", res)).await;
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
                missions_api::lancer_iteration_mission(mission_state.clone(), mission).await;
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
                    missions_api::livrer_telegram(&ch, &format!("✅ Kanban « {} »\n\n{}", kanban_task.title, res))
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
                        changes_api::importer_changes(&sync_state, items, &format!("mesh:{host}")).await;
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
// get_metrics_history -> moved to status_api.rs

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
