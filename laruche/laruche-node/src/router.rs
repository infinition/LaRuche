//! HTTP router assembly (all routes, strict-localhost CORS and the global auth guard) - split out of main.rs.

use crate::*;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use std::sync::Arc;

/// Global auth guard (defense in depth, on top of loopback bind + strict CORS).
///
/// Enforces the session cookie on STATE-CHANGING requests (POST/PUT/DELETE/PATCH)
/// to `/api/*`, but ONLY once an account with a password exists - so a fresh install
/// and the onboarding flow stay open. GET/HEAD reads pass (low risk on loopback and
/// needed to render the UI). The auth flow and the mesh sync endpoints (own ed25519
/// signature) are allowlisted.
async fn auth_guard(
    State(state): State<Arc<AppState>>,
    req: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let method = req.method().clone();
    let mutating = matches!(
        method,
        axum::http::Method::POST
            | axum::http::Method::PUT
            | axum::http::Method::DELETE
            | axum::http::Method::PATCH
    );
    let path = req.uri().path().to_string();
    // The MCP surfaces are exempt from the SESSION-COOKIE guard because they have their
    // own, stricter door: `mcp_pare_feu::controler` (opt-in switch, IP allowlist, token or
    // loopback, ban on repeated refusals, and an audit line per call). Guarding them here
    // too made the feature unusable rather than safer: an MCP client authenticates with
    // `x-laruche-mcp-token`, never with a browser cookie, so every call was rejected before
    // its own door was ever consulted. Both handlers call `controler` first thing; that
    // invariant is what this exemption rests on.
    let mcp = path == "/mcp" || path == "/api/mcp";
    let exempte = !mutating
        || mcp
        || !path.starts_with("/api/")
        || path.starts_with("/api/auth/")
        || path.starts_with("/api/internal/sync");
    if exempte {
        return next.run(req).await;
    }
    // Only enforce once the user has actually configured an account with a password.
    let (auth_configuree, cookie_ok) = {
        let users = state.users.read().await;
        let configuree = users.values().any(|u| u.password_hash.is_some());
        let ok = auth_user::extract_user_from_headers(req.headers(), &state.cookie_secret)
            .map(|id| users.contains_key(&id))
            .unwrap_or(false);
        (configuree, ok)
    };
    if !auth_configuree || cookie_ok {
        return next.run(req).await;
    }
    warn!(path = %path, method = %method, "auth guard: rejected unauthenticated mutation");
    (
        StatusCode::UNAUTHORIZED,
        Json(serde_json::json!({ "error": "authentication required" })),
    )
        .into_response()
}

/// Builds the full HTTP router: every route plus the CORS and auth-guard layers.
/// Moved verbatim from main.rs: the route set, the guard and its allowlists are
/// security sensitive and must stay strictly identical.
pub(crate) fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(web::spa_page))
        .route("/app.css", get(web::app_css))
        .route("/app.js", get(web::app_js))
        .route("/vendor/:name", get(web::vendor_js))
        .route("/manifest.json", get(web::manifest))
        .route("/icon.svg", get(web::icon_svg))
        .route("/icones/icon-192.png", get(web::icon_png_192))
        .route("/icones/icon-512.png", get(web::icon_png_512))
        .route("/sw.js", get(web::service_worker))
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
        .route("/ws/navigateur", get(ws_navigateur::ws_navigateur_handler))
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
        .route("/api/memory/node/restore", post(memory_crud_api::api_memory_node_restore))
        .route("/api/memory/node/delete", post(memory_crud_api::api_memory_node_delete))
        .route("/api/memory/episodes", get(episodes_api::api_etat_episodes))
        .route("/api/memory/episodes/purge", post(episodes_api::api_purger_episodes))
        .route("/api/memory/move", post(memory_crud_api::api_memory_move))
        .route("/api/memory/review", post(memory_crud_api::api_memory_review))
        .route("/api/memory/dream", post(memory_crud_api::api_memory_dream))
        .route("/api/memory/consolidate", post(memory_crud_api::api_memory_consolidate))
        .route("/api/feed", get(feed_api::api_feed))
        .route("/api/feed/ask", post(feed_api::api_feed_ask))
        .route("/api/deliberation/pool", get(deliberation_api::api_pool).post(deliberation_api::api_pool_set))
        .route("/api/deliberation/constitution", get(deliberation_api::api_constitution))
        .route("/api/deliberation/run", post(deliberation_api::api_run))
        .route("/api/deliberation/tours", get(deliberation_api::api_tours))
        .route("/api/deliberation/tour/:id", get(deliberation_api::api_tour).delete(deliberation_api::api_tour_supprimer))
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
        .route("/api/version", get(changes_api::api_version))
        .route("/api/maj", get(changes_api::api_maj))
        .route("/api/ouvrir", post(changes_api::api_ouvrir))
        .route(
            "/api/kanban/interval",
            get(kanban_api::api_kanban_interval_get).post(kanban_api::api_kanban_interval_set),
        )
        .route("/api/memory/tree", get(memory_api::api_memory_tree))
        .route("/api/vision", get(feed_api::api_vision))
        .route("/api/vision/reset", post(feed_api::api_vision_reset))
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
        .route("/api/sessions/:id/reaction", post(sessions_api::api_set_reaction))
        .route("/api/sessions/:id/reactions", get(sessions_api::api_get_reactions))
        .route("/api/reactions/palette", get(sessions_api::api_reactions_palette))
        .route("/api/voice/status", get(status_api::api_voice_status))
        .route("/api/voice/tts", post(status_api::api_tts_proxy))
        .route("/api/webhook", post(local_api::api_webhook))
        .route("/api/preload", post(local_api::api_preload))
        .route("/api/rpc", post(local_api::api_rpc))
        .route("/api/files/suggest", get(local_api::api_files_suggest))
        .route("/api/onboarding", get(local_api::api_onboarding))
        .route("/api/cwd", get(local_api::api_get_cwd).post(local_api::api_set_cwd))
        .route("/api/fs/dirs", get(local_api::api_fs_dirs))
        // Un theme porte son image de fond, encodee dans le JSON: la limite de
        // corps par defaut d'axum, deux mebioctets, la refusait en silence. Le
        // navigateur recevait un 413 sans corps JSON, la promesse echouait, et le
        // panneau restait sur "enregistrement..." pour toujours. Le plafond reel
        // du contenu est verifie dans `themes_api`, celui-ci lui laisse la place.
        .route("/api/skills/livres", get(livres_api::api_livres_etat))
        .route("/api/skills/livres/contenu", get(livres_api::api_livres_contenu))
        .route("/api/skills/livres/appliquer", post(livres_api::api_livres_appliquer))
        .route("/api/skills/livres/ignorer", post(livres_api::api_livres_ignorer))
        .route(
            "/api/themes",
            get(themes_api::api_themes_list).post(
                post(themes_api::api_themes_save)
                    .layer(axum::extract::DefaultBodyLimit::max(12 * 1024 * 1024)),
            ),
        )
        .route("/api/themes/actif", get(themes_api::api_theme_actif_get).post(themes_api::api_theme_actif_set))
        .route("/api/themes/:id", axum::routing::delete(themes_api::api_themes_delete))
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
        .route("/api/reseau/qr", get(auth_api::api_reseau_qr))
        .route("/api/reseau/bind-lan", get(auth_api::api_bind_lan_get).post(auth_api::api_bind_lan_set))
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
        .route(
            "/api/config/voice",
            get(voice_config::api_get_voice).post(voice_config::api_set_voice),
        )
        .route("/api/reine/proposals", get(reine_api::api_list_proposals))
        .route("/api/reine/scorecards", get(reine_api::api_reine_scorecards))
        .route("/api/reine/dataset", get(reine_api::api_reine_dataset))
        .route("/api/reine/appel", post(reine_api::api_reine_appel))
        .route("/api/reine/renvoyer", post(reine_api::api_reine_renvoyer))
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
        .route("/api/travaux", get(doctor_api::api_travaux))
        .route(
            "/api/mcp/bans",
            get(doctor_api::api_mcp_bans).post(doctor_api::api_mcp_unban),
        )
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
        .route("/api/skills/resync", post(skills_api::api_resync_skills))
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
        .route(
            "/api/kanban/todo_sweep",
            get(kanban_api::api_kanban_todo_get).post(kanban_api::api_kanban_todo_set),
        )
        .route(
            "/api/kanban/todo_sweep/now",
            post(kanban_api::api_kanban_todo_maintenant),
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
        .route("/api/admin/users", get(auth_api::api_admin_list_users))
        .route(
            "/api/admin/users/:id",
            axum::routing::delete(auth_api::api_admin_delete_user),
        )
        .route("/api/admin/users/:id/role", post(auth_api::api_admin_set_role))
        .route(
            "/api/admin/users/:id/password",
            post(auth_api::api_admin_set_password),
        )
        .route(
            "/api/admin/users/:id/avatar",
            post(auth_api::api_admin_set_avatar),
        )
        .route("/api/auth/password", post(auth_api::api_auth_set_password))
        .route("/api/auth/account", post(auth_api::api_auth_update_account))
        .route("/api/auth/totp/setup", post(auth_api::api_totp_setup))
        .route("/api/auth/totp/enable", post(auth_api::api_totp_enable))
        .route("/api/auth/totp/disable", post(auth_api::api_totp_disable))
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
            // SECURITY: only same-machine origins may make cross-origin calls. The UI
            // is served same-origin (no CORS needed); a wildcard used to let ANY visited
            // website script requests to LaRuche. We reflect only localhost/127.0.0.1
            // origins (any port, http/https).
            tower_http::cors::CorsLayer::new()
                .allow_origin(tower_http::cors::AllowOrigin::predicate(
                    |origin: &axum::http::HeaderValue, _req: &_| {
                        origin
                            .to_str()
                            .map(|o| {
                                o.starts_with("http://localhost")
                                    || o.starts_with("https://localhost")
                                    || o.starts_with("http://127.0.0.1")
                                    || o.starts_with("https://127.0.0.1")
                                    || o.starts_with("http://[::1]")
                            })
                            .unwrap_or(false)
                    },
                ))
                .allow_methods(tower_http::cors::Any)
                .allow_headers(tower_http::cors::Any),
        )
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            auth_guard,
        ))
        .with_state(state)
}
