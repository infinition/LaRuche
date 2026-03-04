//! LaRuche Node Daemon
//!
//! The main process that runs on each LaRuche box. It:
//! 1. Broadcasts its Cognitive Manifest via LAND
//! 2. Listens for peer nodes (swarm)
//! 3. Exposes an inference API (proxying to Ollama)
//! 4. Manages authentication via Proof of Proximity
//! 5. Runs the web dashboard

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
use tokio::sync::RwLock;
use tracing::{info, error};
use uuid::Uuid;

// Embed the dashboard HTML at compile time
const DASHBOARD_HTML: &str = include_str!("../../laruche-dashboard/src/templates/dashboard.html");

/// Shared application state.
struct AppState {
    manifest: RwLock<CognitiveManifest>,
    auth: RwLock<ProximityAuth>,
    queue: RwLock<RequestQueue>,
    listener: RwLock<LandListener>,
    config: NodeConfig,
}

/// Node configuration (loaded from config file or env vars).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct NodeConfig {
    /// Human-friendly name for this node
    node_name: String,

    /// Hardware tier
    tier: HardwareTier,

    /// Ollama backend URL
    ollama_url: String,

    /// Default model to use
    default_model: String,

    /// API listen port
    api_port: u16,

    /// Dashboard listen port
    dashboard_port: u16,

    /// Available capabilities
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
    memory_usage_pct: f32,
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

// ======================== Handlers ========================

/// GET / - Node status
async fn get_status(State(state): State<Arc<AppState>>) -> Json<NodeStatus> {
    let manifest = state.manifest.read().await;
    let auth = state.auth.read().await;
    let queue = state.queue.read().await;
    let listener = state.listener.read().await;
    let nodes = listener.get_nodes().await;

    Json(NodeStatus {
        node_id: manifest.node_id.to_string(),
        node_name: manifest.node_name.clone(),
        tier: format!("{:?}", manifest.hardware_tier).to_lowercase(),
        protocol_version: manifest.protocol_version.clone(),
        capabilities: manifest.capabilities.to_flags(),
        tokens_per_sec: manifest.performance.tokens_per_sec,
        memory_usage_pct: if manifest.resources.memory_total_mb > 0 {
            (manifest.resources.memory_used_mb as f32 / manifest.resources.memory_total_mb as f32) * 100.0
        } else { 0.0 },
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

/// GET /nodes - List discovered nodes on the network
async fn get_nodes(State(state): State<Arc<AppState>>) -> Json<DiscoveredNodesResponse> {
    let listener = state.listener.read().await;
    let nodes = listener.get_nodes().await;

    let node_list: Vec<DiscoveredNodeInfo> = nodes.values().map(|n| {
        DiscoveredNodeInfo {
            node_id: n.manifest.node_id.map(|id| id.to_string()),
            name: n.manifest.node_name.clone(),
            host: n.manifest.host.clone(),
            capabilities: n.manifest.capabilities.iter().map(|c| c.to_string()).collect(),
            tokens_per_sec: n.manifest.tokens_per_sec,
            queue_depth: n.manifest.queue_depth,
        }
    }).collect();

    Json(DiscoveredNodesResponse { nodes: node_list })
}

/// GET /swarm - Collective intelligence status
async fn get_swarm(State(state): State<Arc<AppState>>) -> Json<SwarmResponse> {
    let listener = state.listener.read().await;
    let nodes = listener.get_nodes().await;
    let manifest = state.manifest.read().await;
    let queue = state.queue.read().await;

    let mut total_tps = manifest.performance.tokens_per_sec;
    let mut total_vram = manifest.resources.vram_total_mb.unwrap_or(0);
    let mut total_ram = manifest.resources.memory_total_mb;
    let mut total_queue = queue.depth() as u32;
    
    let mut node_infos = vec![DiscoveredNodeInfo {
        node_id: Some(manifest.node_id.to_string()),
        name: Some(manifest.node_name.clone()),
        host: manifest.api_endpoint.host.clone(),
        capabilities: manifest.capabilities.to_flags(),
        tokens_per_sec: Some(manifest.performance.tokens_per_sec),
        queue_depth: Some(queue.depth() as u32),
    }];

    for node in nodes.values() {
        total_tps += node.manifest.tokens_per_sec.unwrap_or(0.0);
        total_vram += 0; // Partial manifest doesn't have vram yet
        total_ram += 0; 
        total_queue += node.manifest.queue_depth.unwrap_or(0);
        
        node_infos.push(DiscoveredNodeInfo {
            node_id: node.manifest.node_id.map(|id| id.to_string()),
            name: node.manifest.node_name.clone(),
            host: node.manifest.host.clone(),
            capabilities: node.manifest.capabilities.iter().map(|c| c.to_string()).collect(),
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

/// POST /infer - Inference endpoint
async fn post_infer(
    State(state): State<Arc<AppState>>,
    Json(req): Json<InferenceRequest>,
) -> Result<Json<InferenceResponse>, StatusCode> {
    let config = &state.config;
    let model = req.model.unwrap_or_else(|| config.default_model.clone());
    let start = std::time::Instant::now();

    // Call Ollama backend
    let client = reqwest::Client::new();
    let ollama_req = serde_json::json!({
        "model": model,
        "prompt": req.prompt,
        "stream": false,
        "options": {
            "num_predict": req.max_tokens.unwrap_or(256),
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

                // Update manifest performance metrics
                if let Ok(mut manifest) = state.manifest.try_write() {
                    let eval_duration = body["eval_duration"].as_f64().unwrap_or(1.0) / 1_000_000_000.0;
                    if eval_duration > 0.0 {
                        manifest.performance.tokens_per_sec = eval_count as f32 / eval_duration as f32;
                    }
                }

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
        message: "En attente d'approbation physique. Appuyez sur le bouton du boîtier LaRuche.".into(),
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

/// GET /health - Health check
async fn health() -> &'static str {
    "OK"
}

/// GET /dashboard - Embedded dashboard
async fn dashboard() -> Html<&'static str> {
    Html(DASHBOARD_HTML)
}

// ======================== Main ========================

#[tokio::main]
async fn main() -> Result<()> {
    // Init tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "laruche_node=info,land_protocol=info".into()),
        )
        .init();

    // Load config (from file or defaults)
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

    // Build manifest
    let local_ip = land_protocol::get_local_ip();
    info!(ip = %local_ip, "Detected local IP");
    
    let mut manifest = CognitiveManifest::new(config.node_name.clone(), config.tier);
    manifest.api_endpoint.host = local_ip;
    manifest.api_endpoint.port = config.api_port;
    manifest.api_endpoint.dashboard_port = config.dashboard_port;

    // Register capabilities
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

    // Start LAND broadcaster
    let mut broadcaster = LandBroadcaster::new()?;
    broadcaster.register(&manifest)?;

    // Start LAND listener (discover peers)
    let mut listener = LandListener::new()?;
    let _discovered_nodes = listener.start()?;

    // Build app state
    let state = Arc::new(AppState {
        manifest: RwLock::new(manifest),
        auth: RwLock::new(ProximityAuth::new()),
        queue: RwLock::new(RequestQueue::new(QosPolicy::default())),
        listener: RwLock::new(listener),
        config: config.clone(),
    });

    // Build API router
    let app = Router::new()
        .route("/", get(get_status))
        .route("/health", get(health))
        .route("/nodes", get(get_nodes))
        .route("/swarm", get(get_swarm))
        .route("/infer", post(post_infer))
        .route("/auth/request", post(post_auth_request))
        .route("/auth/approve", post(post_auth_approve))
        .route("/dashboard", get(dashboard))
        .layer(
            tower_http::cors::CorsLayer::permissive()
        )
        .with_state(state.clone());

    // Spawn manifest update loop (refresh metrics every 5 seconds)
    let update_state = state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
        let start_time = std::time::Instant::now();
        loop {
            interval.tick().await;
            let mut manifest = update_state.manifest.write().await;
            manifest.uptime_secs = start_time.elapsed().as_secs();
            manifest.timestamp = chrono::Utc::now();
            // TODO: Read actual system metrics (CPU, RAM, temp)
        }
    });

    // Start server
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
    // Try to load from laruche.toml, fall back to defaults
    let config_path = std::env::var("LARUCHE_CONFIG").unwrap_or_else(|_| "laruche.toml".into());

    if std::path::Path::new(&config_path).exists() {
        let _content = std::fs::read_to_string(&config_path)?;
        // Simple TOML-like parsing for POC (use toml crate in production)
        info!(path = %config_path, "Loaded config from file");
    }

    // For POC, use env vars or defaults
    let config = NodeConfig {
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
        default_model: std::env::var("LARUCHE_MODEL")
            .unwrap_or_else(|_| "mistral".into()),
        api_port: std::env::var("LARUCHE_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(land_protocol::DEFAULT_API_PORT),
        dashboard_port: std::env::var("LARUCHE_DASH_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(land_protocol::DEFAULT_DASHBOARD_PORT),
        capabilities: vec![
            CapabilityConfig {
                capability: std::env::var("LARUCHE_CAP").unwrap_or_else(|_| "llm".into()),
                model_name: std::env::var("LARUCHE_MODEL").unwrap_or_else(|_| "mistral-7b".into()),
                model_size: Some("7B".into()),
                quantization: Some("Q4_K_M".into()),
            },
        ],
    };

    Ok(config)
}
