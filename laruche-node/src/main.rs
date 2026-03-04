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
    extract::State,
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
use std::sync::Arc;
use sysinfo::System;
use tokio::sync::RwLock;
use tracing::{error, info};
use uuid::Uuid;

use std::collections::VecDeque;

const DASHBOARD_HTML: &str = include_str!("../../laruche-dashboard/src/templates/dashboard.html");

#[derive(Debug, Clone, Serialize)]
struct ActivityLogEntry {
    timestamp: String,
    level: String,
    tag: String,
    message: String,
}

struct AppState {
    manifest: RwLock<CognitiveManifest>,
    auth: RwLock<ProximityAuth>,
    queue: RwLock<RequestQueue>,
    listener: RwLock<LandListener>,
    config: NodeConfig,
    sys: RwLock<System>,
    activity_log: RwLock<VecDeque<ActivityLogEntry>>,
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

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            node_name: format!("laruche-{}", &Uuid::new_v4().to_string()[..6]),
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
    #[allow(dead_code)]
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
    capabilities: Vec<String>,
    /// Primary model running on this node (from LAND TXT record)
    model: Option<String>,
    tokens_per_sec: Option<f32>,
    queue_depth: Option<u32>,
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

#[derive(Debug, Serialize)]
struct OllamaModelInfo {
    name: String,
    size_gb: f64,
    digest: String,
}

#[derive(Debug, Serialize)]
struct ModelsResponse {
    models: Vec<OllamaModelInfo>,
    default_model: String,
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
        capabilities: manifest.capabilities.to_flags(),
        tokens_per_sec: manifest.performance.tokens_per_sec,
        memory_usage_pct: mem_pct,
        cpu_usage_pct: cpu_pct,
        memory_used_mb: used_mem_kb / 1024,
        memory_total_mb: total_mem_kb / 1024,
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
            capabilities: n.manifest.capabilities.iter().map(|c| c.to_string()).collect(),
            model: n.manifest.model.clone(),
            tokens_per_sec: n.manifest.tokens_per_sec,
            queue_depth: n.manifest.queue_depth,
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

    let total_mem_mb = sys.total_memory() / 1024;
    let local_model = state.config.capabilities.first().map(|c| c.model_name.clone());

    let mut total_tps = manifest.performance.tokens_per_sec;
    let total_vram = manifest.resources.vram_total_mb.unwrap_or(0);
    let total_ram = total_mem_mb;
    let mut total_queue = queue.depth() as u32;

    let mut node_infos = vec![DiscoveredNodeInfo {
        node_id: Some(manifest.node_id.to_string()),
        name: Some(manifest.node_name.clone()),
        host: manifest.api_endpoint.host.clone(),
        capabilities: manifest.capabilities.to_flags(),
        model: local_model,
        tokens_per_sec: Some(manifest.performance.tokens_per_sec),
        queue_depth: Some(queue.depth() as u32),
    }];

    for node in nodes.values() {
        if node.manifest.node_id == Some(manifest.node_id)
            || node.manifest.host == manifest.api_endpoint.host
        {
            continue;
        }
        total_tps += node.manifest.tokens_per_sec.unwrap_or(0.0);
        total_queue += node.manifest.queue_depth.unwrap_or(0);

        node_infos.push(DiscoveredNodeInfo {
            node_id: node.manifest.node_id.map(|id| id.to_string()),
            name: node.manifest.node_name.clone(),
            host: node.manifest.host.clone(),
            capabilities: node.manifest.capabilities.iter().map(|c| c.to_string()).collect(),
            model: node.manifest.model.clone(),
            tokens_per_sec: node.manifest.tokens_per_sec,
            queue_depth: node.manifest.queue_depth,
        });
    }

    Json(SwarmResponse {
        swarm_id: "collective-1".into(),
        total_nodes: node_infos.len(),
        collective_tps: total_tps,
        collective_queue: total_queue,
        total_vram_mb: total_vram,
        total_ram_mb: total_ram,
        nodes: node_infos,
    })
}

/// POST /infer - Inference endpoint (proxies to Ollama)
async fn post_infer(
    State(state): State<Arc<AppState>>,
    Json(req): Json<InferenceRequest>,
) -> Result<Json<InferenceResponse>, StatusCode> {
    let config = &state.config;
    let model = req.model.unwrap_or_else(|| config.default_model.clone());
    let start = std::time::Instant::now();

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

                // Log activity
                let prompt_preview: String = req.prompt.chars().take(40).collect();
                let log_msg = format!(
                    "Inférence {} - {} tokens en {}ms - \"{}...\"",
                    model, eval_count, latency, prompt_preview.replace('\n', " ")
                );
                
                let mut activity = state.activity_log.write().await;
                if activity.len() >= 50 {
                    activity.pop_front();
                }
                activity.push_back(ActivityLogEntry {
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    level: "log-ok".into(),
                    tag: "INFER".into(),
                    message: log_msg,
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
    let client = reqwest::Client::new();
    let url = format!("{}/api/tags", state.config.ollama_url);

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

                Ok(Json(ModelsResponse {
                    models,
                    default_model: state.config.default_model.clone(),
                }))
            }
            Err(_) => Err(StatusCode::BAD_GATEWAY),
        },
        Err(_) => Err(StatusCode::SERVICE_UNAVAILABLE),
    }
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

async fn health() -> &'static str {
    "OK"
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

    let mut listener = LandListener::new()?;
    let _discovered_nodes = listener.start()?;

    let mut sys = System::new_all();
    sys.refresh_all();

    let state = Arc::new(AppState {
        manifest: RwLock::new(manifest),
        auth: RwLock::new(ProximityAuth::new()),
        queue: RwLock::new(RequestQueue::new(QosPolicy::default())),
        listener: RwLock::new(listener),
        config: config.clone(),
        sys: RwLock::new(sys),
        activity_log: RwLock::new(VecDeque::with_capacity(50)),
    });

    let app = Router::new()
        .route("/", get(get_status))
        .route("/health", get(health))
        .route("/nodes", get(get_nodes))
        .route("/swarm", get(get_swarm))
        .route("/models", get(get_models))
        .route("/activity", get(get_activity))
        .route("/infer", post(post_infer))
        .route("/auth/request", post(post_auth_request))
        .route("/auth/approve", post(post_auth_approve))
        .route("/dashboard", get(dashboard))
        .layer(tower_http::cors::CorsLayer::permissive())
        .with_state(state.clone());

    // Background: refresh real metrics every 5s
    let update_state = state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
        let start_time = std::time::Instant::now();
        loop {
            interval.tick().await;

            {
                let mut sys = update_state.sys.write().await;
                sys.refresh_cpu_usage();
                sys.refresh_memory();
            }

            {
                let mut manifest = update_state.manifest.write().await;
                manifest.uptime_secs = start_time.elapsed().as_secs();
                manifest.timestamp = chrono::Utc::now();

                let sys = update_state.sys.read().await;
                manifest.resources.memory_used_mb = sys.used_memory() / 1024;
                manifest.resources.memory_total_mb = sys.total_memory() / 1024;
                manifest.resources.cpu_usage_pct = sys.global_cpu_usage();
            }
        }
    });

    let addr = format!("0.0.0.0:{}", config.api_port);
    info!(addr = %addr, "LaRuche API server starting");
    info!(
        dashboard = format!("http://localhost:{}/dashboard", config.api_port),
        "Embedded Dashboard available at"
    );

    let listener_tcp = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener_tcp, app).await?;

    Ok(())
}

fn load_config() -> Result<NodeConfig> {
    let config_path = std::env::var("LARUCHE_CONFIG").unwrap_or_else(|_| "laruche.toml".into());
    if std::path::Path::new(&config_path).exists() {
        info!(path = %config_path, "Loaded config from file");
    }

    // Support up to 2 capabilities via env vars:
    //   LARUCHE_CAP=llm  LARUCHE_MODEL=mistral
    //   LARUCHE_CAP2=code LARUCHE_MODEL2=deepseek-coder  (optional)
    let mut capabilities = vec![CapabilityConfig {
        capability: std::env::var("LARUCHE_CAP").unwrap_or_else(|_| "llm".into()),
        model_name: std::env::var("LARUCHE_MODEL").unwrap_or_else(|_| "mistral".into()),
        model_size: Some("7B".into()),
        quantization: Some("Q4_K_M".into()),
    }];

    if let (Ok(cap2), Ok(model2)) = (
        std::env::var("LARUCHE_CAP2"),
        std::env::var("LARUCHE_MODEL2"),
    ) {
        capabilities.push(CapabilityConfig {
            capability: cap2,
            model_name: model2,
            model_size: None,
            quantization: None,
        });
    }

    Ok(NodeConfig {
        node_name: std::env::var("LARUCHE_NAME")
            .unwrap_or_else(|_| format!("laruche-{}", &Uuid::new_v4().to_string()[..6])),
        tier: match std::env::var("LARUCHE_TIER").as_deref() {
            Ok("nano") => HardwareTier::Nano,
            Ok("pro") => HardwareTier::Pro,
            Ok("max") => HardwareTier::Max,
            _ => HardwareTier::Core,
        },
        ollama_url: std::env::var("OLLAMA_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:11434".into()),
        default_model: std::env::var("LARUCHE_MODEL").unwrap_or_else(|_| "mistral".into()),
        api_port: std::env::var("LARUCHE_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(land_protocol::DEFAULT_API_PORT),
        dashboard_port: std::env::var("LARUCHE_DASH_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(land_protocol::DEFAULT_DASHBOARD_PORT),
        capabilities,
    })
}
