//! WebSocket chat handler (interactive streaming chat over WS) and its event serializer helper - split out of main.rs.

use crate::*;
use axum::extract::State;
use std::sync::Arc;

/// Serializes a ChatEvent to JSON, injecting the originating `session_id`.
/// Essential so the frontend routes each event to ITS conversation (and does not
/// mix up jobs from different conversations running in parallel).
fn event_json_avec_session(event: &laruche_essaim::ChatEvent, session_id: Uuid) -> String {
    let mut v = serde_json::to_value(event).unwrap_or_else(|_| serde_json::json!({}));
    if let Some(obj) = v.as_object_mut() {
        obj.insert(
            "session_id".to_string(),
            serde_json::Value::String(session_id.to_string()),
        );
    }
    serde_json::to_string(&v).unwrap_or_default()
}

/// WebSocket handler for the chat interface.
/// Protocol:
///   Client → {"type":"message","text":"..."} or {"type":"message","text":"...","session_id":"uuid"}
///   Server → {"type":"token","text":"...","session_id":"uuid"} / {"type":"tool_call",...} / {"type":"done",...}
pub(crate) async fn ws_chat_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> axum::response::Response {
    let user_id = params.get("user_id").and_then(|s| Uuid::parse_str(s).ok());
    ws.on_upgrade(move |socket| ws_chat_connection(socket, state, user_id))
}

pub(crate) async fn ws_chat_connection(
    socket: ws::WebSocket,
    state: Arc<AppState>,
    auth_user_id: Option<Uuid>,
) {
    use futures_util::{SinkExt, StreamExt};

    let (mut sender, mut receiver) = socket.split();

    // Pending message: deposited by the relay loop when a NEW `message` arrives
    // while a run is running (the user switched conversations and wrote again). We let
    // the current run continue detached and treat this message as a new run.
    let mut pending_text: Option<String> = None;
    // Idle relay. Between two turns the socket used to wait on the client and nothing
    // else, so anything the server pushed in the meantime was broadcast to zero
    // receivers and silently dropped: this is why "Send LaRuche back to work" answered
    // 200, did the work, saved the rewritten answer, and showed absolutely nothing until
    // a reload. The last session's channel stays subscribed here, so a push that arrives
    // between turns still reaches the browser.
    let mut veille: Option<(Uuid, broadcast::Receiver<laruche_essaim::ChatEvent>)> = None;
    loop {
        let text = if let Some(p) = pending_text.take() {
            p
        } else {
            let mut recu: Option<String> = None;
            while recu.is_none() {
                match &mut veille {
                    Some((sid, rx_veille)) => {
                        let sid = *sid;
                        tokio::select! {
                            client = receiver.next() => match client {
                                Some(Ok(ws::Message::Text(t))) => recu = Some(t.to_string()),
                                Some(Ok(ws::Message::Close(_))) | None => return,
                                Some(Ok(_)) => {}
                                Some(Err(_)) => return,
                            },
                            ev = rx_veille.recv() => match ev {
                                Ok(event) => {
                                    update_active_context_stats(&state, sid, &event).await;
                                    let json = event_json_avec_session(&event, sid);
                                    if sender.send(ws::Message::Text(json)).await.is_err() {
                                        return;
                                    }
                                }
                                // Lagged: the client missed a burst but the channel is
                                // still good, so keep relaying instead of going deaf.
                                Err(broadcast::error::RecvError::Lagged(_)) => {}
                                Err(broadcast::error::RecvError::Closed) => { veille = None; }
                            },
                        }
                    }
                    None => match receiver.next().await {
                        Some(Ok(ws::Message::Text(t))) => recu = Some(t.to_string()),
                        Some(Ok(ws::Message::Close(_))) | None => return,
                        Some(Ok(_)) => {}
                        Some(Err(_)) => return,
                    },
                }
            }
            match recu {
                Some(t) => t,
                None => return,
            }
        };

        // Parse incoming message
        let incoming: serde_json::Value = match serde_json::from_str(&text) {
            Ok(v) => v,
            Err(_) => {
                let _ = sender
                    .send(ws::Message::Text(
                        serde_json::json!({"type":"error","message":"Invalid JSON"}).to_string(),
                    ))
                    .await;
                continue;
            }
        };

        let msg_type = incoming["type"].as_str().unwrap_or("message");

        // Handle "subscribe": reattach to an existing running session
        if msg_type == "subscribe" {
            let sessions_dir = std::path::Path::new("sessions");
            let mut sessions = state.essaim_sessions.write().await;
            if let Some(session_id_str) = incoming["session_id"].as_str() {
                if let Ok(id) = Uuid::parse_str(session_id_str) {
                    // Try to load from disk if not in memory
                    if let std::collections::hash_map::Entry::Vacant(e) = sessions.entry(id) {
                        if let Ok(loaded) = laruche_essaim::Session::charger(
                            &sessions_dir.join(format!("{}.json", id)),
                        ) {
                            e.insert(loaded);
                        }
                    }
                    if let Some(session) = sessions.get_mut(&id) {
                        if let Some(tx) = &session.event_tx {
                            let mut rx = tx.subscribe();
                            drop(sessions);
                            let _ = sender.send(ws::Message::Text(serde_json::json!({"type":"session","session_id": id.to_string()}).to_string())).await;
                            // Enter the broadcast loop: relay events to the reattached client
                            let mut done = false;
                            while !done {
                                tokio::select! {
                                    event_result = rx.recv() => {
                                        if let Ok(event) = event_result {
                                            update_active_context_stats(&state, id, &event).await;
                                            let json = event_json_avec_session(&event, id);
                                            if sender.send(ws::Message::Text(json)).await.is_err() {
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
                        "message": "No active task: the request will be relaunched."
                    })
                    .to_string(),
                ))
                .await;
            continue;
        }

        let user_text = match incoming["text"].as_str() {
            Some(t) if !t.trim().is_empty() => t.to_string(),
            _ => continue,
        };
        // No capability reminder appended here any more. The Behavior section of the
        // system prompt already states it, and cron_create / watcher_create /
        // mission_create / session_search are listed with their signatures. Stapling
        // it to EVERY message cost ~35 tokens a turn, and it was stored verbatim:
        // episodes were named `test_system_you_can` and their content carried the
        // marker, which then came back through recall, several times per prompt.
        let user_text_for_agent =
            inject_no_think(&user_text, incoming["no_think"].as_bool().unwrap_or(false));

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
        sessions.entry(session_id).or_insert_with(|| {
            let mut s = Session::new_with_id(session_id, &current_model_ws, sessions_dir);
            s.user_id = auth_user_id;
            s
        });

        // Immediate persistence: save right after creating (before agent runs)
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
                    .to_string(),
            ))
            .await;

        // Model override from client
        let model_override = incoming["model"].as_str().map(|s| s.to_string());
        // Profile (provider) override from client: the model dropdown sends the
        // selected profile id so we can switch provider/base_url/api_key, not
        // just the model name (otherwise a Codex model would go to llama.cpp).
        let profile_override = incoming["provider"].as_str().map(|s| s.to_string());
        // Explicit capability for the turn (e.g. "code" to code) → resolves a dedicated model.
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
        // Le panneau flottant de la page pilotee repond dans ce meme canal: ce
        // qui y est tape est une intervention en cours de route, exactement ce
        // que le steering sait deja faire.
        laruche_essaim::abeilles::navigateur::brancher_pilotage(steer_tx.clone());
        laruche_essaim::abeilles::ordinateur::brancher_pilotage(steer_tx.clone());
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

            // Makes the session visible IMMEDIATELY (even before the response) + persists it to
            // disk: it appears in Sessions and survives an F5 (the agent itself already runs
            // in the background in this tokio::spawn detached from the WebSocket).
            let run_fini = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            {
                let mut snapshot = session.clone();
                snapshot.ajouter_user(&user_text_log);
                // Title right away (not only at the end of the run), so the Sessions
                // list never shows "Untitled" while the agent works.
                snapshot.auto_title();
                let _ = snapshot.sauvegarder();
                state_clone.active_context_stats.write().await.insert(
                    session_id,
                    ActiveContextStats::from_session(&snapshot, true),
                );
                state_clone
                    .essaim_sessions
                    .write()
                    .await
                    .insert(session_id, snapshot.clone());

                // Live mirror: while the engine runs it mutates a LOCAL session, so the
                // shared map + disk would only hold the initial prompt until the end of
                // the run, and a page refresh lost the whole workflow view. This task
                // appends the run's key events (plan, tool calls, results, thoughts) to
                // the shared copy and checkpoints it, so /api/sessions/:id/messages
                // restores the work-in-progress at any moment. The engine's final
                // reconciliation replaces the mirror wholesale (run_fini guard), so
                // duplicates never survive the run.
                let mirror_state = state_clone.clone();
                let mut mirror_rx = tx_clone.subscribe();
                let mirror_fini = run_fini.clone();
                let mut mirror = snapshot;
                tokio::spawn(async move {
                    use laruche_essaim::ChatEvent as Ev;
                    use std::sync::atomic::Ordering;
                    let mut last_save = std::time::Instant::now();
                    loop {
                        match mirror_rx.recv().await {
                            Ok(event) => {
                                let important = match &event {
                                    Ev::ToolCall { name, args, .. } => {
                                        if name == "plan" {
                                            continue;
                                        }
                                        mirror.ajouter_tool_call(name, args.clone());
                                        true
                                    }
                                    Ev::ToolResult { name, result, .. } => {
                                        mirror.ajouter_observation(name, result);
                                        true
                                    }
                                    Ev::Plan { items } => match serde_json::to_string(items) {
                                        Ok(json) => {
                                            mirror.ajouter_thought("plan", "plan", &json);
                                            true
                                        }
                                        Err(_) => continue,
                                    },
                                    Ev::Thought { phase, kind, text } => {
                                        mirror.ajouter_thought(phase, kind, text);
                                        false
                                    }
                                    Ev::Done { .. } | Ev::Error { .. } => break,
                                    _ => continue,
                                };
                                if important
                                    || last_save.elapsed() > std::time::Duration::from_secs(2)
                                {
                                    // Flag checked INSIDE the lock: once the run task has
                                    // published the authoritative session, the mirror must
                                    // never overwrite it with a stale copy.
                                    let mut sessions = mirror_state.essaim_sessions.write().await;
                                    if mirror_fini.load(Ordering::Relaxed) {
                                        break;
                                    }
                                    sessions.insert(session_id, mirror.clone());
                                    let _ = mirror.sauvegarder();
                                    drop(sessions);
                                    last_save = std::time::Instant::now();
                                }
                            }
                            Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                            Err(_) => break,
                        }
                    }
                });
            }

            let mut config = ec_snapshot;
            // List of reachable mesh hives → injected into the context (the agent can `mesh_send`).
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
                                "- {} - {}",
                                n.manifest
                                    .node_name
                                    .clone()
                                    .unwrap_or_else(|| "ruche".into()),
                                id
                            )
                        })
                    })
                    .collect();
                if !lignes.is_empty() {
                    config.mesh_peers_hint = Some(lignes.join("\n"));
                }
            }
            // Resolve the selected profile → provider/base_url/api_key.
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
            // Explicit capability (e.g. "code") without a profile override → model dedicated to this capability.
            if profile_override.is_none() {
                if let Some(ref cap) = capability_override {
                    if cap != "llm" {
                        profiles_api::appliquer_capacite(&state_clone, &mut config, cap).await;
                    }
                }
            }
            if let Some(ref model) = model_override {
                config.model = model.clone();
            }

            // LaReine memory gate: when on, the curateur's writes become proposals
            // (the mode decides whether the Reine auto-applies the safe ones).
            {
                let rs = reine_api::charger_reine_settings();
                config.reine.queue_gate = rs.queue_gate;
                config.reine.mode = rs.mode;
                // Tier 3: the supervisor watches the live butinage loop for stalls.
                config.reine.tier_supervision = rs.tier_supervision;
                // Mirror the gate into the process-global so self-created skills are
                // held for approval (used by the skill_create tool).
                laruche_essaim::reine_queue::definir_gate(rs.queue_gate);
            }

            // LaRuche answering a person: the only actor the user can already see, but the
            // indicator names its model and channel like every other.
            let _garde = ouvrir_travail(
                &state_clone,
                "laruche",
                "chat",
                &config,
                config.origin_channel.clone().or(Some("web".to_string())),
            );
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

            // Auto-title, then publish the authoritative session. The disk save and
            // the map insert happen under the sessions lock, AFTER raising run_fini:
            // the live mirror checks that flag inside the same lock, so a lagging
            // mirror event can never overwrite the reconciled session (map or disk).
            session.auto_title();
            {
                let mut sessions = state_clone.essaim_sessions.write().await;
                run_fini.store(true, std::sync::atomic::Ordering::Relaxed);
                if let Err(e) = session.sauvegarder() {
                    tracing::warn!(error = %e, "Failed to save session");
                }
                sessions.insert(session_id, session.clone());
            }
            // Sync to peers
            let sync_s = session.clone();
            let sync_st = state_clone.clone();
            tokio::spawn(async move {
                sync::push_session_to_peers(&sync_s, &sync_st).await;
            });
            state_clone.active_context_stats.write().await.insert(
                session_id,
                ActiveContextStats::from_session(&session, false),
            );

            // CURATEUR (butinage engine): auto-creation/patch of VERIFIED skills & tools,
            // in the BACKGROUND. OPT-IN (disabled by default) so as not to pollute the library.
            // Activation: Settings toggle (config.curateur_actif, persistent) OR env RUCHE_CURATEUR=1.
            // Conservative (most missions => nothing).
            let curateur_on =
                config.curateur_actif || std::env::var("RUCHE_CURATEUR").as_deref() == Ok("1");
            if laruche_essaim::butinage_pont::moteur_butinage_actif()
                && curateur_on
                && session.messages.len() >= 6
            {
                let msgs = session.messages.clone();
                let reg = state_clone.essaim_registry.clone();
                let cfg = config.clone();
                let txc = tx_clone.clone();
                let mem = Some(state_clone.memoire.clone());
                let cur_state = state_clone.clone();
                tokio::spawn(async move {
                    // The curateur reviews the finished exchange on its own time. Declared
                    // here rather than inside the essaim crate, which knows nothing of the
                    // node's state; the guard covers the whole pass either way.
                    let _garde = ouvrir_travail(&cur_state, "curateur", "review", &cfg, None);
                    laruche_essaim::butinage_pont::lancer_curateur_arriere_plan(
                        msgs,
                        reg,
                        cfg.clone(),
                        txc,
                        mem,
                    )
                    .await;
                });
            }

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
                            // Le meme evenement nourrit le panneau de la page
                            // quand une page est pilotee. Sans page ouverte,
                            // c'est un test d'un booleen et rien d'autre.
                            laruche_essaim::abeilles::navigateur::narrer(&event);
                            laruche_essaim::abeilles::ordinateur::narrer(&event);
                            let json = event_json_avec_session(&event, session_id);
                            if sender.send(ws::Message::Text(json)).await.is_err() {
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
                                laruche_essaim::ChatEvent::Done { full_response } => {
                                    let _ = state.events.write().await.emit(
                                        laruche_events::EventKind::AgentFinished,
                                        &actor,
                                        serde_json::json!({ "session_id": session_id, "status": "done" })
                                    );
                                    // LaReine Tier 1 review runs AFTER Done is forwarded, so the answer
                                    // is shown first and the turn completes; the verdict and any rewrite
                                    // trickle in afterwards. No-op unless enabled.
                                    if reine_api::review_active() {
                                        // The review (and visible rework) streams to a local channel,
                                        // relayed live to this WebSocket until the `__reine_end__`
                                        // sentinel. The rework thus appears in the chat as it happens.
                                        let (rtx, mut rrx) =
                                            tokio::sync::broadcast::channel::<laruche_essaim::ChatEvent>(256);
                                        let revue = reine_api::revue_complete(
                                            &state,
                                            session_id,
                                            &user_text,
                                            full_response,
                                            rtx,
                                        );
                                        let relay = async {
                                            while let Ok(ev) = rrx.recv().await {
                                                if let laruche_essaim::ChatEvent::Status { message } = &ev {
                                                    if message == "__reine_end__" {
                                                        break;
                                                    }
                                                }
                                                let _ = sender
                                                    .send(ws::Message::Text(
                                                        event_json_avec_session(&ev, session_id),
                                                    ))
                                                    .await;
                                            }
                                        };
                                        tokio::join!(revue, relay);
                                    }
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
                                                    "message": "Steering received: applied at the next step."
                                                }).to_string()
                                            )).await;
                                        }
                                        Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {
                                            let _ = sender.send(ws::Message::Text(
                                                serde_json::json!({
                                                    "type": "steer_rejected",
                                                    "reason": "queue_full",
                                                    "text": steer_text,
                                                    "message": "Too many pending steers: wait for the next step."
                                                }).to_string()
                                            )).await;
                                        }
                                        Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
                                            let _ = sender.send(ws::Message::Text(
                                                serde_json::json!({
                                                    "type": "steer_rejected",
                                                    "reason": "run_finished",
                                                    "text": steer_text,
                                                    "message": "The task just finished: resend this message as a new request."
                                                }).to_string()
                                            )).await;
                                        }
                                    }
                                } else if json["type"].as_str() == Some("stop") {
                                    // Stop requested: abort the agent task. The session was already
                                    // saved as a snapshot (with the user message) BEFORE the run,
                                    // so only the in-progress response is dropped, not the session.
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
                                                "message": "Generation interrupted."
                                            })
                                            .to_string(),
                                        ))
                                        .await;
                                    done = true;
                                } else if json["type"].as_str() == Some("message") {
                                    // New message during a run (often ANOTHER conversation):
                                    // we let THIS run continue detached (react_handle keeps going, its session
                                    // is re-inserted at the end) and ask the outer loop to handle it.
                                    pending_text = Some(text.to_string());
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

        // Le tour est fini: ce qui serait tape dans la page apres coup n'a plus
        // de destination, et le panneau le dira plutot que de l'avaler.
        laruche_essaim::abeilles::navigateur::debrancher_pilotage();
        laruche_essaim::abeilles::ordinateur::debrancher_pilotage();

        // Stay tuned to this session until the next turn: LaReine sending the work back,
        // or any other background push, has somewhere to arrive.
        veille = Some((session_id, tx.subscribe()));

        // let _ = react_handle.await; (Detached to allow background running without blocking WS cleanup)
    }
}
