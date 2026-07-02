//! Background jobs (metrics refresh, schedulers, dispatchers, file watchers) - split out of main.rs.
//!
//! Each function wraps one background task moved verbatim from main(); main() calls
//! them in the same order as before the split.

use crate::*;

const MDNS_REANNOUNCE_INTERVAL_SECS: u64 = 2;

// Background: refresh real metrics + re-announce mDNS + periodic save
pub(crate) fn spawn_metrics_refresh(state: &Arc<AppState>, broadcaster: &Arc<MielBroadcaster>) {
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
}

// Background: Auth challenge cleanup (every 30 seconds)
pub(crate) fn spawn_auth_challenge_cleanup(state: &Arc<AppState>) {
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
}

// Background: periodic memory dream (consolidation + dedup): anti-bloat hygiene.
// Long interval (6 h by default), 1st pass deferred by 10 min so as not to load
// startup. Disableable via LARUCHE_DREAM_INTERVAL_SECS=0.
pub(crate) fn spawn_periodic_dream(state: &Arc<AppState>) {
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
}

// Background: Ollama heartbeat (every 60 seconds)
pub(crate) fn spawn_ollama_heartbeat(state: &Arc<AppState>) {
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
}

// Background: Cron task checker (every 30 seconds)
pub(crate) fn spawn_cron_checker(state: &Arc<AppState>) {
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
}

// Background: Watchers task checker (every 10 seconds)
pub(crate) fn spawn_watchers_checker(state: &Arc<AppState>) {
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
}

// Background: periodic mDNS re-announce (P4): reflects the REAL models (active +
// detected local backends + public_proxy providers), picks up backends started hot,
// and fixes the announcement of the frozen default model.
pub(crate) fn spawn_mdns_reannounce(state: &Arc<AppState>, broadcaster: &Arc<MielBroadcaster>) {
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
}

// Background: tick of long-running MISSIONS ("La Reine"): every 60s, launches an
// iteration of active missions whose cron cadence is due (e.g. weekly research).
pub(crate) fn spawn_missions_tick(state: &Arc<AppState>) {
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
}

// Background: Kanban Dispatcher (every 5 seconds)
pub(crate) fn spawn_kanban_dispatcher(state: &Arc<AppState>) {
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
}

// Background: Dream (auto on inactivity + background review)
pub(crate) fn spawn_idle_dream(state: &Arc<AppState>) {
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
}

// L3 (slice 2): AUTO memory SYNC from peer nodes (Miel), every 5 min: each
// node pulls+dedups the others' facts → COLLECTIVE memory of the ruche, without cloud.
// OFF by default and OPT-IN (LARUCHE_MESH_MEMORY_SYNC=1). The pull does not yet
// verify per-peer signatures, so a peer's facts are trusted as-is; imported facts
// are provenance-tagged (source=mesh:<peer>) and treated as REFERENCE DATA (never
// instructions) by the recall framing. Sync stays opt-in until mutual peer
// verification lands (tracked in the roadmap).
pub(crate) fn spawn_mesh_memory_sync(state: &Arc<AppState>) {
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
}

// Phase 1.5: live WATCHER of SKILL.md: a modified file is re-synced to SQL
// without reboot (8s poll, incremental by mtime).
pub(crate) fn spawn_skill_file_watcher(state: &Arc<AppState>) {
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
}

// Telegram notifier: forwards AgentFinished/WatcherFired events when notify is enabled.
pub(crate) fn spawn_event_notifier(state: &Arc<AppState>) {
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
}

// Auto-start channels if configured (channels-config.json): currently the Telegram bot.
pub(crate) async fn autostart_channels(state: &Arc<AppState>) {
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
}
