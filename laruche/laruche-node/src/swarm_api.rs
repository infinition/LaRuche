//! Node and swarm API (status, discovered nodes, swarm view, inference, model lists, auth request/approve, default model, activity feed, health, service register) - split out of main.rs.

use crate::*;
use axum::extract::State;
use axum::response::Json;
use axum::http::StatusCode;
use std::sync::Arc;

/// GET / - Node status with real system metrics
pub(crate) async fn get_status(State(state): State<Arc<AppState>>) -> Json<NodeStatus> {
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
pub(crate) async fn get_nodes(State(state): State<Arc<AppState>>) -> Json<DiscoveredNodesResponse> {
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
pub(crate) async fn get_swarm(State(state): State<Arc<AppState>>) -> Json<SwarmResponse> {
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

/// GET /models - List available Ollama models on this node
pub(crate) async fn get_models(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ModelsResponse>, StatusCode> {
    let dm = get_llm_default(&state).await;
    fetch_local_models(&state.config.ollama_url, &dm)
        .await
        .map(Json)
}

/// GET /swarm/models - Aggregate models across local node and discovered peers
pub(crate) async fn get_swarm_models(
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

    // Voice services (tts/stt) share THIS host, so the same-host filter above skips them,
    // and mDNS is flaky on Windows. Probe the default local voice ports directly so the
    // local TTS/STT ALWAYS shows up in the mesh services panel and the voice selectors.
    for (port, cap) in [(8422u16, "tts"), (8421u16, "stt")] {
        if models.iter().any(|m| m.capability.as_deref() == Some(cap)) {
            continue;
        }
        if let Some(backend) = crate::profiles_api::probe_voice_backend(port).await {
            hosts.insert("127.0.0.1".to_string());
            models.push(SwarmModelInfo {
                host: "127.0.0.1".to_string(),
                node_name: "Local voice".to_string(),
                node_id: None,
                name: format!("{cap}-{backend}"),
                size_gb: 0.0,
                digest: String::new(),
                is_default: false,
                is_local: true,
                capability: Some(cap.to_string()),
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
pub(crate) async fn post_auth_request(
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

/// POST /auth/approve - approve the pending device-auth request. Restricted to an
/// authenticated admin (the trusted operator), so anyone on the network cannot mint a
/// trust token by hitting this route.
pub(crate) async fn post_auth_approve(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if !auth_user::require_admin(&state, &headers).await {
        return Err(StatusCode::UNAUTHORIZED);
    }
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
pub(crate) struct SetDefaultModelRequest {
    model: String,
    #[serde(default)]
    capability: Option<String>,
}

/// POST /config/default_model - Change the runtime default model
pub(crate) async fn post_set_default_model(
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
pub(crate) async fn get_default_model(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
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
pub(crate) struct ActivityResponse {
    logs: Vec<ActivityLogEntry>,
}

/// GET /activity - Recent activity (filtered by user; admin sees all)
pub(crate) async fn get_activity(
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

pub(crate) async fn health() -> Json<serde_json::Value> {
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

pub(crate) async fn api_register_service(
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

pub(crate) async fn api_unregister_service(
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

