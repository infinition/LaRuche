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
mod arbitre_memoire;
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
mod voice_config;
mod totp;
mod state;
mod helpers;
mod router;

pub(crate) use state::*;
pub(crate) use helpers::*;

use anyhow::Result;
use axum::{
    extract::{ws, WebSocketUpgrade},
    http::StatusCode,
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
const MDNS_REANNOUNCE_INTERVAL_SECS: u64 = 2;


/// Best-effort local LAN IP (for the cert SAN list), via a UDP connect trick. No packet
/// is actually sent; the OS just picks the outbound interface address.
fn detect_local_ip() -> Option<String> {
    let sock = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    sock.connect("8.8.8.8:80").ok()?;
    sock.local_addr().ok().map(|a| a.ip().to_string())
}

/// Ensure a self-signed cert + key exist on disk (generated once), returning their
/// paths. The SANs cover localhost, 127.0.0.1 and the detected LAN IP so HTTPS works
/// from other devices. Browsers warn on a self-signed cert: accept it once.
fn ensure_self_signed_cert() -> Option<(String, String)> {
    let cert_path = "laruche-cert.pem";
    let key_path = "laruche-key.pem";
    if std::path::Path::new(cert_path).exists() && std::path::Path::new(key_path).exists() {
        return Some((cert_path.to_string(), key_path.to_string()));
    }
    let mut sans = vec!["localhost".to_string(), "127.0.0.1".to_string()];
    if let Some(ip) = detect_local_ip() {
        if !sans.contains(&ip) {
            sans.push(ip);
        }
    }
    let certified = rcgen::generate_simple_self_signed(sans).ok()?;
    std::fs::write(cert_path, certified.cert.pem()).ok()?;
    std::fs::write(key_path, certified.key_pair.serialize_pem()).ok()?;
    info!(cert = cert_path, "generated self-signed TLS certificate for HTTPS");
    Some((cert_path.to_string(), key_path.to_string()))
}

// Blueprint endpoints (list/create/delete parameterized cron automation templates, instantiate) -> moved to blueprints_api.rs

// ======================== Handlers ========================

// Node and swarm API (status, discovered nodes, swarm view, inference, model lists, auth request/approve, default model, activity feed, health, service register) -> moved to swarm_api.rs
// OpenAI-compatible chat completions endpoint with signed peer verification -> moved to openai_api.rs

// System status endpoints (voice STT/TTS availability, metrics history) -> moved to status_api.rs


// Tool registry endpoints (list tools, get/save tool enablement config) -> moved to tools_api.rs

// Cognitive memory CRUD (search, write, enrich, node create/update/move/delete, review, dream, consolidate, grep) -> moved to memory_crud_api.rs

// Memory change-sync (disk-to-SQL skill sync, OKF change import/export, mesh pull) and state version endpoint -> moved to changes_api.rs

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

/// Serve `app` on `addr`, with optional TLS. A bad/unreadable cert pair no longer panics
/// the server task: it logs and falls back to plain HTTP so the node stays reachable.
async fn serve_with_optional_tls(app: axum::Router, addr: String, tls: Option<(String, String)>) {
    let make = app.into_make_service_with_connect_info::<SocketAddr>();
    if let Some((cert, key)) = tls {
        info!(cert = %cert, key = %key, "TLS enabled: starting HTTPS server");
        match axum_server::tls_rustls::RustlsConfig::from_pem_file(&cert, &key).await {
            Ok(cfg) => match addr.parse::<SocketAddr>() {
                Ok(bind_addr) => {
                    let _ = axum_server::bind_rustls(bind_addr, cfg).serve(make).await;
                }
                Err(e) => error!(error = %e, addr = %addr, "Invalid bind address for HTTPS"),
            },
            Err(e) => {
                error!(error = %e, "Failed to load TLS cert/key; falling back to HTTP");
                match tokio::net::TcpListener::bind(&addr).await {
                    Ok(l) => { let _ = axum::serve(l, make).await; }
                    Err(e) => error!(error = %e, addr = %addr, "Failed to bind HTTP fallback"),
                }
            }
        }
    } else {
        match tokio::net::TcpListener::bind(&addr).await {
            Ok(l) => { let _ = axum::serve(l, make).await; }
            Err(e) => error!(error = %e, addr = %addr, "Failed to bind HTTP listener"),
        }
    }
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
            Ok("sqlite") => {
                // Embedder ALWAYS wired (semantic recall by default): LARUCHE_EMBED_URL
                // (Ollama `/api/embed` OR llama.cpp/OpenAI-compat `/v1/embeddings` —
                // format auto-detected), falling back to the local Ollama default.
                // HttpEmbedder opens a circuit breaker when the server is down, so a
                // missing embedder costs ~nothing and recall degrades to FTS5.
                let url = std::env::var("LARUCHE_EMBED_URL")
                    .ok()
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| "http://127.0.0.1:11434".to_string());
                let model = std::env::var("LARUCHE_EMBED_MODEL")
                    .unwrap_or_else(|_| "nomic-embed-text".to_string());
                info!(url = %url, model = %model, "memory embedder: semantic recall active (auto-detected format)");
                Arc::new(
                    laruche_memoire::SqliteBackend::open_with_embedder(
                        "memoire.db",
                        Arc::new(laruche_memoire::HttpEmbedder::new(url, model)),
                    )
                    .expect("opening memoire.db (SQLite+FTS5+embeddings)"),
                )
            }
            _ => Arc::new(laruche_memoire::NativeBackend::new()),
        };
    // Backfill: items written while the embedder was down get their embeddings
    // (semantic recall would otherwise never see them). Deferred a little so the
    // local embed server has time to come up; the breaker makes failures cheap.
    {
        let mem_bf = memoire.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(20)).await;
            if let Ok(n) = mem_bf.backfill_embeddings(500).await {
                if n > 0 {
                    info!(items = n, "memory: missing embeddings backfilled");
                }
            }
        });
    }
    // Write-time contradiction arbiter (aux LLM): resolves near-miss updates cosine
    // cannot (e.g. "4070 Ti" -> "5080" ~0.71). No-op on backends without arbiter support;
    // any LLM failure keeps both facts (never destructive). Opt-out via LARUCHE_MEMOIRE_ARBITRE=0.
    if std::env::var("LARUCHE_MEMOIRE_ARBITRE").as_deref() != Ok("0") {
        memoire.definir_arbitre(std::sync::Arc::new(
            arbitre_memoire::ArbitreLLM::depuis_config(&essaim_config),
        ));
    }
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
    let mut loaded_users = auth_user::load_all_users(users_dir);
    let deduped = auth_user::dedupe_users(&mut loaded_users, users_dir);
    if deduped > 0 {
        info!(removed = deduped, "Deduplicated legacy duplicate accounts (old enroll bug)");
    }
    if !loaded_users.is_empty() {
        info!(count = loaded_users.len(), "Users loaded from disk");
    }

    // Load or generate cookie secret (persisted in laruche-state.json).
    // The fingerprint log is the auth debugging anchor: if it CHANGES between two
    // boots, every session cookie is invalidated (that is the "re-login every
    // launch" symptom) and the state file is not persisting/loading correctly.
    let cookie_secret = if let Some(ref hex) = persistent.cookie_secret {
        match auth_user::cookie_secret_from_base64(hex) {
            Some(s) => {
                info!(
                    fingerprint = %&auth_user::cookie_secret_to_base64(&s)[..8],
                    "Cookie secret loaded from laruche-state.json (sessions survive restarts)"
                );
                s
            }
            None => {
                let s = auth_user::generate_cookie_secret();
                warn!(
                    stored_len = hex.len(),
                    fingerprint = %&auth_user::cookie_secret_to_base64(&s)[..8],
                    "Stored cookie secret INVALID -> regenerated: every session cookie is now invalid (re-login required)"
                );
                s
            }
        }
    } else {
        let s = auth_user::generate_cookie_secret();
        warn!(
            fingerprint = %&auth_user::cookie_secret_to_base64(&s)[..8],
            "No cookie secret in laruche-state.json -> generated: previous sessions are invalid"
        );
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

    // Persist the state RIGHT AWAY: the shutdown save only runs on a clean exit
    // (Ctrl+C / tray Quit). Closing the console window kills the process without
    // saving — a cookie secret generated this boot would then never be written,
    // and the NEXT boot would regenerate it, invalidating every session cookie
    // ("re-login on every launch"). Saving here makes the secret durable no
    // matter how the process dies.
    save_persistent_state(&state).await;

    // Mirror the saved LaReine gate into the process-global at boot, so self-created
    // skills are held for approval even before the first chat turn (cron/curateur).
    laruche_essaim::reine_queue::definir_gate(reine_api::charger_reine_settings().queue_gate);

    let app = router::build_router(state.clone());

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
                        // Bulk sync ONLY from full LaRuche peers (llm/agent). Voice-only
                        // nodes (tts/stt) and other capability services do not serve the
                        // /api/internal/sync/* endpoints, so syncing them just 404s and
                        // spams the logs (plus a Windows asyncio ConnectionReset).
                        let is_full_peer = node
                            .manifest
                            .capabilities
                            .iter()
                            .any(|c| matches!(c, Capability::Llm | Capability::Agent));
                        if is_full_peer {
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
                        Ok(report) => {
                            // Duplicate suggestions become actionable proposals in the
                            // Reine queue (critical class: a human click runs the dedup).
                            // Overloaded/orphan suggestions stay advisory for now: they
                            // have no mechanical apply and need an agent mission.
                            let mut enqueued = 0usize;
                            for s in report
                                .get("suggestions")
                                .and_then(|v| v.as_array())
                                .map(|a| a.as_slice())
                                .unwrap_or(&[])
                            {
                                if s.get("kind").and_then(|k| k.as_str()) != Some("duplicate") {
                                    continue;
                                }
                                let (Some(node_id), Some(message)) = (
                                    s.get("node_id").and_then(|v| v.as_str()),
                                    s.get("message").and_then(|v| v.as_str()),
                                ) else {
                                    continue;
                                };
                                if laruche_essaim::reine_queue::proposer_hygiene(node_id, message) {
                                    enqueued += 1;
                                }
                            }
                            info!(
                                proposals = enqueued,
                                "Periodic memory dream finished (consolidation + dedup)"
                            );
                        }
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

    // Bind to loopback ONLY by default (single-user local app). Serving on the whole
    // network is an explicit opt-in (LARUCHE_BIND_LAN=1) since not every route requires
    // the session cookie; the choice is loudly logged so it is never accidental.
    let bind_lan = std::env::var("LARUCHE_BIND_LAN").as_deref() == Ok("1");
    let bind_ip = if bind_lan { "0.0.0.0" } else { "127.0.0.1" };
    let addr = format!("{bind_ip}:{}", config.api_port);
    let scheme = if std::env::var("LARUCHE_HTTPS").as_deref() == Ok("1")
        || std::env::var("LARUCHE_TLS_CERT").map(|s| !s.is_empty()).unwrap_or(false)
    {
        "https"
    } else {
        "http"
    };
    if bind_lan {
        warn!(
            "LARUCHE_BIND_LAN=1: API exposed on the whole LAN ({bind_ip}:{}). Ensure auth \
             is configured; anyone on the network can reach it.",
            config.api_port
        );
    }
    info!("LaRuche ready → {scheme}://localhost:{}", config.api_port);

    // Sync essaim config from active profile at startup
    profiles_api::sync_essaim_from_profiles(&state).await;

    // L3 (slice 2): AUTO memory SYNC from peer nodes (Miel), every 5 min: each
    // node pulls+dedups the others' facts → COLLECTIVE memory of the ruche, without cloud.
    // OFF by default and OPT-IN (LARUCHE_MESH_MEMORY_SYNC=1). The pull does not yet
    // verify per-peer signatures, so a peer's facts are trusted as-is; imported facts
    // are provenance-tagged (source=mesh:<peer>) and treated as REFERENCE DATA (never
    // instructions) by the recall framing. Sync stays opt-in until mutual peer
    // verification lands (tracked in the roadmap).
    if std::env::var("LARUCHE_MESH_MEMORY_SYNC").as_deref() == Ok("1") {
        info!("LARUCHE_MESH_MEMORY_SYNC=1: collective memory sync enabled (pulls facts from LAN peers every 5 min).");
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
                        // Only full LaRuche peers (llm/agent) serve /api/memory/*; skip
                        // voice-only nodes (tts/stt) so we do not query a host that runs
                        // only a TTS/STT service.
                        .filter(|(_, n)| {
                            n.manifest.capabilities.iter().any(|c| {
                                matches!(c, Capability::Llm | Capability::Agent)
                            })
                        })
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

    // rustls 0.23 requires a process-wide crypto provider before any TLS server starts;
    // with both ring and aws-lc-rs in the tree it cannot auto-pick, so install ring here.
    // Without this, bind_rustls panics inside its spawned task and HTTPS silently never
    // starts (the app keeps serving plain HTTP).
    let _ = rustls::crypto::ring::default_provider().install_default();

    // TLS: explicit LARUCHE_TLS_CERT/KEY win. Otherwise LARUCHE_HTTPS=1 auto-generates a
    // self-signed cert (localhost + 127.0.0.1 + LAN IP), so the browser microphone works
    // from other devices on the network (a secure context), not just localhost.
    let (tls_cert, tls_key) = {
        let cert = std::env::var("LARUCHE_TLS_CERT").ok().filter(|s| !s.is_empty());
        let key = std::env::var("LARUCHE_TLS_KEY").ok().filter(|s| !s.is_empty());
        if cert.is_some() && key.is_some() {
            (cert, key)
        } else if std::env::var("LARUCHE_HTTPS").as_deref() == Ok("1") {
            match ensure_self_signed_cert() {
                Some((c, k)) => (Some(c), Some(k)),
                None => {
                    error!("LARUCHE_HTTPS=1 but self-signed cert generation failed; serving HTTP");
                    (None, None)
                }
            }
        } else {
            (None, None)
        }
    };

    if use_tui {
        // Spawn server in background, run TUI in foreground
        let tui_state = state.clone();
        tokio::spawn(async move {
            serve_with_optional_tls(app, addr, tls_cert.zip(tls_key)).await;
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
            serve_with_optional_tls(app, addr, tls_cert.zip(tls_key)).await;
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
