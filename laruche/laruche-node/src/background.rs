//! Background jobs (metrics refresh, schedulers, dispatchers, file watchers) - split out of main.rs.
//!
//! Each function wraps one background task moved verbatim from main(); main() calls
//! them in the same order as before the split.

use crate::*;

/// Cadence de rafraichissement des metriques locales (CPU, VRAM, file d'attente).
///
/// Rien a voir avec le mDNS, malgre le nom que portait cette constante. Elle
/// s'appelait `MDNS_REANNOUNCE_INTERVAL_SECS` alors qu'elle pilotait cette boucle-ci,
/// pendant que la vraie reannonce codait 30 en dur ailleurs. Le nom seul a suffi a
/// masquer une double annonce pendant longtemps.
const METRICS_REFRESH_INTERVAL_SECS: u64 = 2;

/// Cadence de reannonce mDNS. Confortablement sous `PEER_STALE_SECS` (90 s), pour
/// qu'un pair ne soit jamais evince entre deux annonces.
const MDNS_REANNOUNCE_INTERVAL_SECS: u64 = 30;

// Background: refresh real metrics + periodic save.
//
// N'annonce PLUS sur le mDNS. Cette boucle appelait `broadcaster.update()` toutes les
// deux secondes avec le manifeste BRUT, tandis que `spawn_mdns_reannounce` annonce
// toutes les 30 s une version filtree qui n'expose que les profils explicitement
// publics. Les deux tournaient, donc le contenu annonce alternait: les modeles
// apparaissaient et disparaissaient au rythme des deux boucles, et les backends
// locaux que la version filtree masque volontairement etaient reannonces malgre tout
// toutes les deux secondes.
pub(crate) fn spawn_metrics_refresh(state: &Arc<AppState>, broadcaster: &Arc<MielBroadcaster>) {
    let update_state = state.clone();
    // Conserve pour la reannonce IMMEDIATE lors d'un changement (voir plus bas): un
    // modele qu'on vient de rendre public doit apparaitre tout de suite, pas dans 30 s.
    let _ = broadcaster;
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(
            METRICS_REFRESH_INTERVAL_SECS,
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
            if tick_count.is_multiple_of(30) {
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
                if tick_count.is_multiple_of(10) {
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

                // Pas de reannonce ici: une seule boucle annonce, et c'est celle qui
                // filtre. `last_seen` des pairs est rafraichi par ses 30 s, largement
                // sous le seuil d'eviction de 90 s.
            }

            // Collect metrics snapshot every 5 ticks (10 seconds)
            if tick_count.is_multiple_of(5) {
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
                        log_activite(
                            &heartbeat_state,
                            "info",
                            "heartbeat",
                            "Ollama recovered".into(),
                            None,
                        )
                        .await;
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
                        log_activite(
                            &heartbeat_state,
                            "error",
                            "heartbeat",
                            "Ollama is not responding".into(),
                            None,
                        )
                        .await;
                        was_down = true;
                    }
                }
            }
        }
    });
}

// Background: Cron task checker (every 30 seconds)
/// Writes the outcome of a scheduled run into the cognitive memory.
///
/// The `memory` delivery channel: instead of pushing the answer to a chat service, the
/// run leaves a trace where LaRuche will find it again. It is the only channel needing no
/// token and no configuration, and the only one whose result the agent can later recall
/// on its own, which is what a recurring watch is usually for.
///
/// Lands under `episodes.<date>.<slug>`, the convention the engine already uses for a
/// mission's episodes, so a scheduled run and a conversation write to the same place.
pub(crate) async fn livrer_en_memoire(
    state: &Arc<AppState>,
    origine: &str,
    titre: &str,
    resultat: Result<&str, String>,
) {
    let date = chrono::Local::now().format("%Y-%m-%d");
    let slug: String = titre
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect::<String>()
        .trim_matches('_')
        .split('_')
        .filter(|s| !s.is_empty())
        .take(6)
        .collect::<Vec<_>>()
        .join("_");
    let slug = if slug.is_empty() { origine.to_string() } else { slug };
    let node_id = format!("episodes.{date}.{slug}");

    let contenu = match resultat {
        Ok(r) => format!("**{titre}** ({origine})\n\n{r}"),
        Err(e) => format!("**{titre}** ({origine}) a echoue\n\n{e}"),
    };
    if let Err(e) = state
        .memoire
        .write(laruche_memoire::MemoryItem::new(node_id, contenu).with_source(origine))
        .await
    {
        warn!(error = %e, "memory delivery failed");
    }
}

/// Reads a channel spec `nom` or `nom:cible`, e.g. `telegram`, `discord:123`, `slack:#veille`.
fn decoupe_canal(canal: &str) -> (&str, &str) {
    match canal.split_once(':') {
        Some((nom, cible)) => (nom, cible.trim()),
        None => (canal, ""),
    }
}

/// First entry of a comma-separated allow list, used when the spec names no target.
/// Read a channel token, resolving a vault reference.
///
/// A token may be stored literally, or as `${NAME}` pointing at a Secrets entry - the
/// same two modes as a provider API key. Without this the reference would be sent to
/// Telegram verbatim and the bot would simply fail to authenticate.
fn jeton_canal(bloc: &serde_json::Value, champ: &str) -> String {
    laruche_essaim::secrets::substituer(bloc[champ].as_str().unwrap_or(""))
}

fn premiere_cible(cfg: &serde_json::Value, cle: &str) -> String {
    cfg[cle]
        .as_str()
        .unwrap_or("")
        .split(',')
        .next()
        .unwrap_or("")
        .trim()
        .to_string()
}

/// Delivers the outcome of a scheduled run on the requested channel.
///
/// Delivery used to be a single `starts_with("telegram")` branch written inline in the
/// cron loop: a task set to Discord ran, produced its answer, and dropped it. The picker
/// offered a channel the server could not honour, and Slack was not even offered although
/// it was configured. One function now knows the four, and it is called from one place so
/// watchers and research can reuse it.
pub(crate) async fn livrer_resultat(
    state: &Arc<AppState>,
    canal: &str,
    origine: &str,
    titre: &str,
    resultat: Result<&str, String>,
) {
    if canal == crate::CANAL_MEMOIRE {
        livrer_en_memoire(state, origine, titre, resultat).await;
        return;
    }
    let texte = match &resultat {
        Ok(r) => format!("**{titre}**\n\n{r}"),
        Err(e) => format!("**{titre}** a echoue\n\n{e}"),
    };
    livrer_message(canal, &texte).await;
}

/// Sends a ready-made message on a chat channel.
///
/// Split out of [`livrer_resultat`] so the watchers, the kanban and the missions reach the
/// same three services: all six of their call sites went through a helper that returned
/// early for anything other than Telegram, with a comment promising the rest for later.
pub(crate) async fn livrer_message(canal: &str, texte: &str) {
    let (nom, cible) = decoupe_canal(canal);
    let Ok(contenu) = std::fs::read_to_string("channels-config.json") else {
        warn!(channel = canal, "no channels-config.json: nothing delivered");
        return;
    };
    let Ok(cfg) = serde_json::from_str::<serde_json::Value>(&contenu) else {
        warn!("channels-config.json unreadable: nothing delivered");
        return;
    };
    let client = reqwest::Client::new();

    match nom {
        "telegram" => {
            let bloc = &cfg["telegram"];
            let token = &jeton_canal(bloc, "bot_token");
            let chat = if cible.is_empty() {
                premiere_cible(bloc, "allowed_chats")
            } else {
                cible.to_string()
            };
            if token.is_empty() || chat.is_empty() {
                warn!("telegram not configured: nothing delivered");
                return;
            }
            let _ = client
                .post(format!("https://api.telegram.org/bot{token}/sendMessage"))
                .json(&serde_json::json!({
                    "chat_id": chat, "text": texte, "parse_mode": "Markdown"
                }))
                .send()
                .await;
        }
        "discord" => {
            let bloc = &cfg["discord"];
            let token = &jeton_canal(bloc, "bot_token");
            let salon = if cible.is_empty() {
                premiere_cible(bloc, "allowed_channels")
            } else {
                cible.to_string()
            };
            if token.is_empty() || salon.is_empty() {
                warn!("discord not configured: nothing delivered");
                return;
            }
            let _ = client
                .post(format!(
                    "https://discord.com/api/v10/channels/{salon}/messages"
                ))
                .header("Authorization", format!("Bot {token}"))
                .json(&serde_json::json!({ "content": texte }))
                .send()
                .await;
        }
        "slack" => {
            let bloc = &cfg["slack"];
            let token = &jeton_canal(bloc, "bot_token");
            if token.is_empty() || cible.is_empty() {
                // Slack has no allow list to fall back on: without an explicit channel
                // there is nowhere to post, and guessing one would be worse than saying so.
                warn!("slack: no target channel, nothing delivered");
                return;
            }
            let _ = client
                .post("https://slack.com/api/chat.postMessage")
                .header("Authorization", format!("Bearer {token}"))
                .json(&serde_json::json!({ "channel": cible, "text": texte }))
                .send()
                .await;
        }
        autre => warn!(channel = autre, "unknown delivery channel"),
    }
}

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
                        let mut nom = String::new();
                        let mut channel = None;
                        let mut provider = None;
                        let mut model = None;
                        let mut profile_id = None;
                        let mut skills = Vec::new();
                        for t in cron.list() {
                            if t.id == id {
                                nom = t.name.clone();
                                channel = t.channel.clone();
                                provider = t.provider.clone();
                                model = t.model.clone();
                                profile_id = t.profile_id.clone();
                                skills = t.skills.clone();
                                break;
                            }
                        }
                        (id, nom, prompt, channel, provider, model, profile_id, skills)
                    })
                    .collect::<Vec<_>>()
            };
            for (task_id, nom, prompt, channel, provider, model, profile_id, skills) in due_tasks {
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
                tokio::spawn(async move { while rx.recv().await.is_ok() {} });

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

                // Visible in the status bar for as long as the run lasts.
                let _garde = ouvrir_travail(
                    &cron_state,
                    "cron",
                    if nom.is_empty() { "scheduled task" } else { &nom },
                    &cron_config,
                    channel.clone(),
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
                // One place that knows how to deliver, instead of a single branch for
                // Telegram and silence for everything the picker offered.
                let delivery_channel = channel.filter(|s| !s.is_empty());
                if let Some(ch) = delivery_channel {
                    livrer_resultat(
                        &cron_state,
                        &ch,
                        "cron",
                        &preview_text(&prompt, 60),
                        match &result {
                            Ok(r) => Ok(r.as_str()),
                            Err(e) => Err(e.to_string()),
                        },
                    )
                    .await;
                    let _ = &prompt;
                } else {
                    log_activite_riche(
                        &cron_state,
                        if result.is_ok() { "info" } else { "error" },
                        "cron",
                        format!("Cron task: {}", preview_text(&prompt, 60)),
                        Some(prompt),
                        result.ok().map(|r| preview_text(&r, 4000)),
                        Some(cron_config.model.clone()),
                        None,
                    )
                    .await;
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
            for d in triggered {
                let (watcher_id, prompt, context) = (d.id, d.prompt, d.contexte);
                let current_model = get_llm_default(&watcher_state).await;
                let (w_profile, w_model, w_channel, w_name) = {
                    let reg = watcher_state.watchers.read().await;
                    reg.list()
                        .into_iter()
                        .find(|w| w.id == watcher_id)
                        .map(|w| (w.profile_id.clone(), w.model.clone(), w.channel.clone(), w.name.clone()))
                        .unwrap_or((None, None, None, String::new()))
                };
                let mut config = watcher_state.essaim_config.read().await.clone();
                if let Some(pid) = w_profile {
                    profiles_api::appliquer_profil(&watcher_state, &mut config, &pid, w_model.as_deref()).await;
                } else {
                    config.model = current_model.clone();
                }

                // LLM gate, two sources: the residual llm_check question of a
                // compiled-rules watcher (deterministic prefix already passed), or
                // the legacy free-text condition. One tiny call with the current
                // datetime in hand. Fail-open: an unusable gate must not silence
                // an alert.
                let question_gate: Option<String> = d
                    .question_llm
                    .clone()
                    .or_else(|| if d.semantique { Some(d.condition.clone()) } else { None });
                if let Some(q) = question_gate {
                    if !condition_satisfaite(&config, &q, &context).await {
                        info!(watcher_id = %watcher_id, "Watcher event rejected by the condition gate");
                        continue;
                    }
                }

                // A fire leaves a trace in LaRuche itself, whatever channel carries
                // the message away. Without this the feed only ever recorded the
                // CREATION of a watcher: one that had fired three times looked, from
                // the interface, exactly like one that had never fired at all, and the
                // only proof of it lived in a Telegram thread. Recorded here, above the
                // match, so that a fourth action cannot be added that forgets it.
                if !matches!(d.action, laruche_watchers::Action::Aucune) {
                    laruche_essaim::feed_journal::record(
                        if w_name.is_empty() { "watcher" } else { &w_name },
                        "watcher",
                        "fired",
                        preview_text(&context, 160),
                        chrono::Utc::now(),
                    );
                }

                // Two of the three actions never touch a model. A fire used to cost a
                // full agentic mission whatever the job was: a whole turn, paid and
                // slow, to write "the file is gone", which it could also get wrong.
                match &d.action {
                    laruche_watchers::Action::Notifier => {
                        let livr = match &w_channel {
                            Some(c) => Some(c.clone()),
                            None => watcher_state.essaim_config.read().await.home_channel.clone(),
                        };
                        if let Some(ch) = livr {
                            missions_api::livrer_telegram(&ch, &format!("🔔 {context}")).await;
                        }
                        log_activite_riche(
                            &watcher_state, "info", "watcher",
                            format!("Watcher notified: {}", preview_text(&context, 60)),
                            None, Some(preview_text(&context, 500)), None, None,
                        )
                        .await;
                        continue;
                    }
                    laruche_watchers::Action::Commande { commande } => {
                        // A watcher that ACTS: the lamp came on after midnight, turn it
                        // off. Same platform split as the command watcher, and the same
                        // refusal list, which lives in the watcher crate.
                        let sortie = executer_action_commande(commande).await;
                        let livr = match &w_channel {
                            Some(c) => Some(c.clone()),
                            None => watcher_state.essaim_config.read().await.home_channel.clone(),
                        };
                        if let Some(ch) = livr {
                            missions_api::livrer_telegram(
                                &ch,
                                &format!("⚙️ {context}\n\n{}", preview_text(&sortie, 500)),
                            )
                            .await;
                        }
                        log_activite_riche(
                            &watcher_state, "info", "watcher",
                            format!("Watcher action: {}", preview_text(commande, 60)),
                            Some(commande.clone()), Some(preview_text(&sortie, 2000)), None, None,
                        )
                        .await;
                        continue;
                    }
                    // A pure sensor. Its verdict was already published before we got
                    // here, which is the only thing it exists for.
                    laruche_watchers::Action::Aucune => continue,
                    laruche_watchers::Action::Agent => {}
                }

                info!(watcher_id = %watcher_id, "Executing watcher task");
                let _ = watcher_state.events.write().await.emit(
                    laruche_events::EventKind::WatcherFired,
                    "watcher_dispatcher",
                    serde_json::json!({ "watcher_id": watcher_id, "prompt": prompt, "context": context })
                );
                let sessions_dir = std::path::Path::new("sessions");
                let mut session = Session::new_with_path(&current_model, sessions_dir);
                let (tx, _rx) = broadcast::channel::<ChatEvent>(64);

                let full_prompt = format!("[CONTEXT: {}]\n\n{}", context, prompt);
                let _garde = ouvrir_travail(
                    &watcher_state,
                    "watcher",
                    if w_name.is_empty() { "watcher" } else { &w_name },
                    &config,
                    w_channel.clone(),
                );
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

                log_activite_riche(
                    &watcher_state,
                    if result.is_ok() { "info" } else { "error" },
                    "watcher",
                    format!("Watcher task: {}", preview_text(&prompt, 60)),
                    Some(full_prompt),
                    result.ok().map(|r| preview_text(&r, 4000)),
                    Some(config.model.clone()),
                    None,
                )
                .await;
            }
        }
    });
}

/// Semantic condition gate for file/url watchers: one small LLM call with the
/// observed event and the CURRENT LOCAL DATETIME, answering YES/NO. Fail-open on
/// provider or parse trouble (a broken gate must never silence an alert; the
/// fire cooldown already bounds the noise).
async fn condition_satisfaite(
    config: &laruche_essaim::EssaimConfig,
    condition: &str,
    contexte: &str,
) -> bool {
    use futures_util::StreamExt;
    let maintenant = chrono::Local::now().format("%A %Y-%m-%d %H:%M");
    let invite = format!(
        "You are the trigger gate of a monitoring watcher.\n\
         Current local datetime: {maintenant}\n\
         Observed event: {contexte}\n\
         User condition: \"{condition}\"\n\
         Does the observed event satisfy the condition RIGHT NOW? Take dates, days \
         of week and durations into account when the condition mentions them.\n\
         Answer STRICTLY with YES or NO."
    );
    let messages = vec![serde_json::json!({ "role": "user", "content": invite })];
    let mut stream = match laruche_essaim::providers::provider_chat_stream(
        &config.provider,
        &config.model,
        &messages,
        0.0,
        8,
        &laruche_essaim::secrets::substituer(&config.api_key),
        config.api_base.as_deref(),
        &config.ollama_url,
        None,
    )
    .await
    {
        Ok(s) => s,
        Err(e) => {
            warn!(error = %e, "watcher condition gate unavailable, firing anyway");
            return true;
        }
    };
    let mut texte = String::new();
    while let Some(chunk) = stream.next().await {
        texte.push_str(&chunk.text);
    }
    let rep = texte.to_uppercase();
    if rep.contains("NO") && !rep.contains("YES") {
        false
    } else {
        // YES, or unusable output: fail-open.
        true
    }
}

// Background: periodic mDNS re-announce (P4): reflects the REAL models (active +
// detected local backends + public_proxy providers), picks up backends started hot,
// and fixes the announcement of the frozen default model.
pub(crate) fn spawn_mdns_reannounce(state: &Arc<AppState>, broadcaster: &Arc<MielBroadcaster>) {
    let mdns_broadcaster = broadcaster.clone();
    let mdns_state = state.clone();
    tokio::spawn(async move {
        // SEULE boucle d'annonce du noeud. Sous PEER_STALE_SECS (90 s), donc un pair
        // n'est jamais evince entre deux annonces, et assez espacee pour ne pas
        // inonder le reseau: la version precedente annoncait toutes les 2 s.
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(
            MDNS_REANNOUNCE_INTERVAL_SECS,
        ));
        // On NE saute PAS le premier tick. `register()` au demarrage annonce le
        // manifeste brut, capacites comprises; il faut que la version filtree le
        // remplace tout de suite, sinon les backends locaux restent exposes pendant
        // les 30 premieres secondes de chaque demarrage.
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

// Background: repartiteur Kanban.
//
// Le delai entre deux releves est relu A CHAQUE TOUR plutot que fige a la
// creation de la boucle: un reglage qui n'agit qu'apres un redemarrage n'est
// pas un reglage, c'est un piege. Le regler a 2 secondes doit se sentir tout de
// suite.
/// La releve de la colonne A faire.
///
/// Elle ne lance rien: elle promeut les taches de A faire vers Pret, et le
/// repartiteur ci-dessous les prend une par une avec le profil de chacune.
/// Un seul chemin d'execution, donc une seule facon de se tromper.
///
/// Le tour de boucle est d'une minute, quelle que soit la cadence reglee: c'est
/// l'echeance qui decide, pas le rythme du reveil, et une minute suffit pour
/// une cadence qui se compte en heures.
pub(crate) fn spawn_kanban_todo_sweeper(state: &Arc<AppState>) {
    let etat = state.clone();
    tokio::spawn(async move {
        let mut tic = tokio::time::interval(std::time::Duration::from_secs(60));
        tic.tick().await; // le premier tick est immediat: on le laisse passer
        loop {
            tic.tick().await;
            let maintenant = chrono::Utc::now();
            let du = etat.kanban_board.read().await.todo_est_du(maintenant);
            if !du {
                continue;
            }
            let n = etat.kanban_board.write().await.promouvoir_todo(maintenant);
            if n > 0 {
                info!(taches = n, "Kanban: colonne A faire relevee vers Pret");
                let _ = etat.events.write().await.emit(
                    laruche_events::EventKind::AgentStarted,
                    "kanban_todo_sweeper",
                    serde_json::json!({ "promues": n }),
                );
            }
        }
    });
}

pub(crate) fn spawn_kanban_dispatcher(state: &Arc<AppState>) {
    let kanban_state = state.clone();
    tokio::spawn(async move {
        loop {
            let attente = kanban_state.kanban_board.read().await.delai_secs();
            tokio::time::sleep(std::time::Duration::from_secs(attente)).await;

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
                let _garde = ouvrir_travail(
                    &kanban_state,
                    "kanban",
                    &kanban_task.title,
                    &config,
                    None,
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

                log_activite_riche(
                    &kanban_state,
                    if result.is_ok() { "info" } else { "error" },
                    "kanban",
                    format!("Kanban task: {}", preview_text(&kanban_task.title, 60)),
                    Some(prompt),
                    result.ok().map(|r| preview_text(&r, 4000)),
                    Some(config.model.clone()),
                    None,
                )
                .await;
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
                    let token = &jeton_canal(&config["telegram"], "bot_token");
                    let chats_str = config["telegram"]["allowed_chats"].as_str().unwrap_or("");
                    let first_chat = chats_str.split(',').next().unwrap_or("").trim();
                    if !token.is_empty() && !first_chat.is_empty() {
                        let msg = format!(
                            "🔔 *LaRuche Notification*\n\n*Event:* `{:?}`\n*Actor:* `{}`",
                            ev.kind, ev.actor
                        );
                        let client = reqwest::Client::new();
                        let _ = client
                            .post(format!(
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
                    let tg_resolu = jeton_canal(&channels_config["telegram"], "bot_token");
                    if let Some(tg_token) = Some(tg_resolu.as_str()).filter(|t| !t.is_empty()) {
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

/// Run a watcher's action command and return its combined output.
///
/// Cross-platform on purpose: PowerShell on Windows, `sh` elsewhere, the same split the
/// command watcher uses. A watcher that acts is the point of the feature, so it must
/// behave identically on the three systems rather than being a Windows-only trick.
///
/// Bounded: an action that hangs would block the dispatcher for every other watcher.
async fn executer_action_commande(commande: &str) -> String {
    const TIMEOUT_ACTION_SECS: u64 = 30;
    // The refusal list lives in the watcher crate, so an action created through any
    // path gets the same guard. An action mutates by design, which is exactly why it
    // must not be looser than an observation.
    if let Some(motif) = laruche_watchers::commande_refusee_publique(commande) {
        return format!("refused for safety (forbidden pattern '{motif}')");
    }
    let futur = async {
        let sortie = if cfg!(windows) {
            tokio::process::Command::new("powershell")
                .args(["-NoProfile", "-NonInteractive", "-Command", commande])
                .output()
                .await
        } else {
            tokio::process::Command::new("sh")
                .args(["-c", commande])
                .output()
                .await
        };
        match sortie {
            Ok(o) => {
                let mut t = String::from_utf8_lossy(&o.stdout).to_string();
                let e = String::from_utf8_lossy(&o.stderr);
                if !e.trim().is_empty() {
                    if !t.is_empty() {
                        t.push('\n');
                    }
                    t.push_str(&e);
                }
                if t.trim().is_empty() {
                    t = format!("done (exit {})", o.status.code().unwrap_or(-1));
                }
                t
            }
            Err(e) => format!("failed: {e}"),
        }
    };
    match tokio::time::timeout(std::time::Duration::from_secs(TIMEOUT_ACTION_SECS), futur).await {
        Ok(t) => t,
        Err(_) => format!("timed out after {TIMEOUT_ACTION_SECS}s"),
    }
}

// ── Bin (`orphans.*`) ────────────────────────────────────────────────────────────────
// `delete_node` never destroys: it relocates the subtree under
// `orphans.<name>_<unix_ts>`. Nothing reads that bin back, so without a purge it grows
// forever and clutters the tree. It empties itself after a delay instead.

/// Deletion timestamp carried by a bin entry id (`orphans.<name>_<unix_ts>`).
///
/// The node's `created_at` cannot serve here: relocation keeps the ORIGINAL creation
/// date, so a node created months ago and deleted today would look expired on the spot.
/// The suffix is the only field that records WHEN the deletion happened.
fn horodatage_corbeille(id: &str) -> Option<i64> {
    let reste = id.strip_prefix("orphans.")?;
    // Top-level entry only: a descendant goes with its parent.
    if reste.contains('.') {
        return None;
    }
    let (_, ts) = reste.rsplit_once('_')?;
    ts.parse::<i64>().ok()
}

/// Background: empties the bin of everything deleted more than `LARUCHE_TRASH_TTL_SECS`
/// ago (7 days by default, 0 disables). Checked every 6 h, first pass after 5 min.
pub(crate) fn spawn_purge_corbeille(state: &Arc<AppState>) {
    let ttl: i64 = std::env::var("LARUCHE_TRASH_TTL_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(7 * 24 * 3600);
    if ttl <= 0 {
        return;
    }
    let purge_state = state.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(300)).await;
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(6 * 3600));
        loop {
            interval.tick().await;
            let maintenant = chrono::Utc::now().timestamp();
            let Ok(noeuds) = purge_state.memoire.list_nodes().await else {
                continue;
            };
            let perimes: Vec<String> = noeuds
                .as_array()
                .map(|a| a.as_slice())
                .unwrap_or(&[])
                .iter()
                .filter_map(|n| n.get("id").or_else(|| n.get("node_id"))?.as_str())
                .filter(|id| {
                    horodatage_corbeille(id).is_some_and(|ts| maintenant - ts >= ttl)
                })
                .map(str::to_string)
                .collect();
            let mut vides = 0usize;
            for id in &perimes {
                // Targeting `orphans.*` takes the hard-delete branch: gone for good.
                if purge_state.memoire.delete_node(id).await.is_ok() {
                    vides += 1;
                }
            }
            if vides > 0 {
                info!(entries = vides, ttl_secs = ttl, "Bin purged");
            }
        }
    });
}

#[cfg(test)]
mod tests_corbeille {
    use super::horodatage_corbeille;

    #[test]
    fn lit_l_horodatage_de_suppression_dans_l_identifiant() {
        assert_eq!(horodatage_corbeille("orphans.projects_1753660800"), Some(1753660800));
        // A name carrying underscores of its own: only the last chunk is the stamp.
        assert_eq!(
            horodatage_corbeille("orphans.mon_vieux_noeud_1753660800"),
            Some(1753660800)
        );
    }

    #[test]
    fn ignore_ce_qui_n_est_pas_une_entree_de_corbeille() {
        assert_eq!(horodatage_corbeille("orphans"), None);
        assert_eq!(horodatage_corbeille("projects.alpha"), None);
        // A descendant is removed with its parent, never on its own.
        assert_eq!(horodatage_corbeille("orphans.projects_1753660800.sub"), None);
        // Legacy entry with no stamp: left alone rather than purged on a guess.
        assert_eq!(horodatage_corbeille("orphans.projects"), None);
    }
}

#[cfg(test)]
mod tests_livraison {
    use super::{decoupe_canal, premiere_cible};

    #[test]
    fn lit_le_canal_et_sa_cible() {
        assert_eq!(decoupe_canal("telegram"), ("telegram", ""));
        assert_eq!(decoupe_canal("telegram:12345"), ("telegram", "12345"));
        assert_eq!(decoupe_canal("discord:987"), ("discord", "987"));
        // Slack channel names carry a #, which must survive intact.
        assert_eq!(decoupe_canal("slack:#veille"), ("slack", "#veille"));
        assert_eq!(decoupe_canal("memory"), ("memory", ""));
    }

    #[test]
    fn prend_la_premiere_cible_autorisee() {
        let cfg = serde_json::json!({ "allowed_chats": " 111 , 222 ,333 " });
        assert_eq!(premiere_cible(&cfg, "allowed_chats"), "111");
        let vide = serde_json::json!({});
        assert_eq!(premiere_cible(&vide, "allowed_chats"), "");
    }
}
