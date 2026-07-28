//! Mission, cron and subagent endpoints (cron CRUD/run, mission CRUD/run/decompose, subagent spawn, notebooks, mission iteration runtime) - split out of main.rs.

use crate::*;
use axum::extract::State;
use axum::response::Json;
use axum::http::StatusCode;
use std::sync::Arc;

/// GET /api/cron - list scheduled tasks.
pub(crate) async fn api_list_cron(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let cron = state.essaim_cron.read().await;
    let mut tasks: Vec<serde_json::Value> = cron
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
    drop(cron);
    // Mission cadences appear in the same list (kind="mission"): one truthful
    // view of everything scheduled. They are managed from the Missions page /
    // mission_* tools, so the UI renders them read-only here.
    let store = state.missions.read().await;
    // A finished mission has no schedule anymore: only active/paused cadenced
    // missions appear (paused shows as disabled, like a disabled cron).
    for m in store
        .list()
        .iter()
        .filter(|m| m.cadence.is_some() && m.status != "done")
    {
        tasks.push(serde_json::json!({
            "id": format!("mission:{}", m.slug),
            "name": format!("Mission: {}", m.objective),
            "cron_expr": m.cadence,
            "enabled": m.status == "active",
            "last_run": m.last_run,
            "run_count": m.iterations,
            "channel": m.channel.clone(),
            "kind": "mission",
        }));
    }
    Json(serde_json::json!(tasks))
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct SpawnAgentRequest {
    task: String,
    context: Option<String>,
    recursion_depth: Option<u32>,
    max_iterations: Option<usize>,
    budget: Option<f32>,
}

#[derive(Debug, serde::Serialize)]
pub(crate) struct SpawnAgentResponse {
    agent_id: String,
    status: String,
}

/// POST /api/agents/spawn - launch a subagent dynamically.
pub(crate) async fn api_spawn_subagent(
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
        let _garde = ouvrir_travail(&state_clone, "sous-agent", &task_clone, &config, None);
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
pub(crate) async fn api_create_cron(
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
pub(crate) async fn api_delete_cron(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> StatusCode {
    if !auth_user::require_admin(&state, &headers).await {
        return StatusCode::FORBIDDEN;
    }
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
pub(crate) async fn api_list_missions(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    Json(serde_json::json!(state.missions.read().await.list()))
}

/// POST /api/missions - creates a mission. Body: {objective, slug?, cadence?}.
pub(crate) async fn api_create_mission(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    if !auth_user::require_admin(&state, &headers).await {
        return Json(serde_json::json!({"error": "unauthorized (admin required)"}));
    }
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
    // Give the research a real workspace before it runs. The iteration prompt already
    // orders the agent to write under `missions.<slug>.findings`, `.questions` and
    // `.synthese`, but nothing created them: the node appeared only if the model happened
    // to call memory_write, so a mission could run and leave no visible trace in the tree.
    // Created up front, the workspace is imposed rather than hoped for, and the user sees
    // where the work is going from the moment the mission exists.
    {
        let racine = format!("missions.{slug}");
        let _ = state
            .memoire
            .create_node(
                &racine,
                &objective,
                Some("Research workspace: this mission capitalizes here"),
                Some(0.8),
                Some("mission"),
            )
            .await;
        for (suffixe, label, resume) in [
            ("findings", "Findings", "Lasting sourced facts, one per item"),
            ("questions", "Open questions", "What is still unresolved"),
            ("synthese", "Synthesis", "Readable overview of the case so far"),
        ] {
            let _ = state
                .memoire
                .create_node(
                    &format!("{racine}.{suffixe}"),
                    label,
                    Some(resume),
                    Some(0.7),
                    Some("mission"),
                )
                .await;
        }
    }

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
pub(crate) async fn lancer_iteration_mission(state: Arc<AppState>, mission: missions::Mission) -> u32 {
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
        // Held across the iteration AND the LaReine review below, so the indicator does
        // not blink off between the two halves of the same piece of work.
        let _garde = ouvrir_travail(&run_state, "recherche", &slug, &cfg, channel.clone());
        let result = boucle_react_memoire(
            &prompt,
            &mut session,
            &run_state.essaim_registry,
            &cfg,
            &tx,
            run_state.memoire.clone(),
        )
        .await;
        // LaReine Tier 1 (only when enabled): review the iteration's output and re-do the work
        // if it falls short, using the mission's own config, then deliver the approved version.
        let result = match result {
            Ok(bilan) => {
                Ok(crate::reine_api::revue_mission(&run_state, &mut session, &cfg, &prompt, &bilan, &tx).await)
            }
            err => err,
        };
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
/// Delivers a message on a channel. Kept under its old name because six call sites use
/// it, but it no longer knows only Telegram: it forwards to the shared sender, which
/// handles Discord and Slack as well. It also no longer requires an explicit chat id,
/// falling back to the first entry of the allow list like the rest of the delivery path.
pub(crate) async fn livrer_telegram(channel: &str, text: &str) {
    crate::background::livrer_message(channel, text).await;
}

/// GET /api/butinage/carnets - lists UNFINISHED butinage notebooks (resumable).
pub(crate) async fn api_carnets_list() -> Json<serde_json::Value> {
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
pub(crate) async fn api_carnet_resume(
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
pub(crate) async fn api_run_mission(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    axum::extract::Path(slug): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    if !auth_user::require_admin(&state, &headers).await {
        return Json(serde_json::json!({"error": "unauthorized (admin required)"}));
    }
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
pub(crate) async fn api_mission_dossier(
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
pub(crate) async fn api_update_mission(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    axum::extract::Path(slug): axum::extract::Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    if !auth_user::require_admin(&state, &headers).await {
        return Json(serde_json::json!({"error": "unauthorized (admin required)"}));
    }
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
pub(crate) async fn api_delete_mission(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    axum::extract::Path(slug): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    if !auth_user::require_admin(&state, &headers).await {
        return Json(serde_json::json!({"error": "unauthorized (admin required)"}));
    }
    let ok = state.missions.write().await.remove(&slug);
    Json(serde_json::json!({"status": if ok {"ok"} else {"not_found"}, "slug": slug}))
}

/// Level-2 orbit - DECOMPOSES a mission into parallel kanban tasks (one per open
/// question, otherwise an angle to cover). The kanban dispatcher executes them (research), each
/// task writing its findings into the mission's subtree. Skills are forged
/// automatically (background_review) each turn. Reuses everything that exists.
pub(crate) async fn decomposer_mission(
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
pub(crate) async fn api_decompose_mission(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    axum::extract::Path(slug): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    if !auth_user::require_admin(&state, &headers).await {
        return Json(serde_json::json!({"error": "unauthorized (admin required)"}));
    }
    let Some(mission) = state.missions.read().await.get(&slug) else {
        return Json(serde_json::json!({"error": "mission not found"}));
    };
    let n = decomposer_mission(&state, &mission, 4).await;
    Json(serde_json::json!({"status": "ok", "slug": slug, "tasks_created": n}))
}

pub(crate) async fn api_run_cron(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    if !auth_user::require_admin(&state, &headers).await {
        return Json(serde_json::json!({"error": "unauthorized (admin required)"}));
    }
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
        // A cron fired by hand from the UI: same actor as the scheduled one.
        let _garde = ouvrir_travail(&run_state, "cron", &task.name, &cfg, task.channel.clone());
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
pub(crate) async fn api_update_cron(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    if !auth_user::require_admin(&state, &headers).await {
        return Json(serde_json::json!({"error": "unauthorized (admin required)"}));
    }
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
    // Creation accepts a provider profile; without it here, editing a task could not keep
    // or change the profile it runs on, only the raw provider/model pair.
    if body.get("profile_id").is_some() {
        task.profile_id = body["profile_id"]
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
