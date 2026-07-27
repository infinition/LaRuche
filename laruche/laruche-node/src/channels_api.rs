//! Channel bot management (start/stop/status) and Telegram bot runtime, plus shared channel query helpers - split out of main.rs.

use crate::*;
use axum::extract::State;
use axum::response::Json;
use std::sync::Arc;

/// POST /api/channels/start: start a channel bot.
/// Body: {"channel": "telegram"}
pub(crate) async fn api_start_channel(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    if !auth_user::require_admin(&state, &headers).await {
        return Json(serde_json::json!({"status": "error", "message": "unauthorized (admin required)"}));
    }
    let channel = body["channel"].as_str().unwrap_or("");

    // Check if already running
    {
        let handles = state.channel_handles.read().await;
        if handles.contains_key(channel) {
            return Json(serde_json::json!({"status": "already_running", "channel": channel}));
        }
    }

    // Load config
    let config_path = std::path::Path::new("channels-config.json");
    let config: serde_json::Value = if config_path.exists() {
        std::fs::read_to_string(config_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        return Json(
            serde_json::json!({"status": "error", "message": "No channels-config.json found. Configure in Settings > Channels."}),
        );
    };

    match channel {
        "telegram" => {
            let token = config["telegram"]["bot_token"]
                .as_str()
                .unwrap_or("")
                .to_string();
            let allowed = config["telegram"]["allowed_chats"]
                .as_str()
                .unwrap_or("")
                .to_string();
            if token.is_empty() {
                return Json(
                    serde_json::json!({"status": "error", "message": "No Telegram bot token configured"}),
                );
            }
            let state_clone = state.clone();
            let handle = tokio::spawn(async move {
                run_telegram_bot(&token, &allowed, &state_clone).await;
            });
            state
                .channel_handles
                .write()
                .await
                .insert("telegram".into(), handle);
            info!("Telegram bot started");
            Json(serde_json::json!({"status": "started", "channel": "telegram"}))
        }
        _ => Json(
            serde_json::json!({"status": "error", "message": format!("Unknown channel: {}", channel)}),
        ),
    }
}

/// POST /api/channels/stop: stop a channel bot.
pub(crate) async fn api_stop_channel(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    if !auth_user::require_admin(&state, &headers).await {
        return Json(serde_json::json!({"status": "error", "message": "unauthorized (admin required)"}));
    }
    let channel = body["channel"].as_str().unwrap_or("");
    let mut handles = state.channel_handles.write().await;
    if let Some(handle) = handles.remove(channel) {
        handle.abort();
        info!(channel = channel, "Channel bot stopped");
        Json(serde_json::json!({"status": "stopped", "channel": channel}))
    } else {
        Json(serde_json::json!({"status": "not_running", "channel": channel}))
    }
}

/// GET /api/channels/status: check which channels are running.
pub(crate) async fn api_channels_status(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let handles = state.channel_handles.read().await;
    let running: Vec<&String> = handles.keys().collect();
    Json(serde_json::json!({"running": running}))
}

//// Live approval brokers, one per Telegram chat with a run in flight: routes a
/// button press back to the tool call that is waiting. Registered for the
/// duration of a run only.
type CourtierAppro = tokio::sync::mpsc::Sender<laruche_essaim::brain::ApprovalResponse>;

static COURTIERS_TG: std::sync::OnceLock<
    std::sync::Mutex<std::collections::HashMap<i64, CourtierAppro>>,
> = std::sync::OnceLock::new();

fn courtiers_tg() -> &'static std::sync::Mutex<std::collections::HashMap<i64, CourtierAppro>> {
    COURTIERS_TG.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
}

fn enregistrer_courtier(chat_id: i64, tx: CourtierAppro) {
    courtiers_tg().lock().unwrap().insert(chat_id, tx);
}

fn oublier_courtier(chat_id: i64) {
    courtiers_tg().lock().unwrap().remove(&chat_id);
}

/// Inline keyboard of the `/menu` quick actions. Buttons carry OUR `data`
/// payloads (never free user text), so a press can never smuggle a command.
pub(crate) fn clavier_menu() -> serde_json::Value {
    serde_json::json!({
        "inline_keyboard": [
            [
                {"text": "🧹 Reset", "callback_data": "cmd:reset"},
                {"text": "📊 Status", "callback_data": "cmd:status"},
            ],
            [
                {"text": "🏠 Set home", "callback_data": "cmd:sethome"},
                {"text": "🔊 Voice", "callback_data": "cmd:voice"},
            ],
            [
                {"text": "⏱️ Crons", "callback_data": "cmd:crons"},
                {"text": "🚫 Deny rules", "callback_data": "cmd:deny"},
            ],
        ]
    })
}

/// Inline keyboard offered with an approval request: approve once, approve the
/// whole class for the session, or refuse.
pub(crate) fn clavier_approbation(tool_call_id: &str) -> serde_json::Value {
    serde_json::json!({
        "inline_keyboard": [[
            {"text": "✅ Allow", "callback_data": format!("ok:{tool_call_id}")},
            {"text": "♾️ Always", "callback_data": format!("always:{tool_call_id}")},
            {"text": "⛔ Refuse", "callback_data": format!("no:{tool_call_id}")},
        ]]
    })
}

/// Handles an inline-button press. `callback_data` is one of OUR payloads
/// (`cmd:*`, `ok:*`, `always:*`, `no:*`), never user-authored text.
async fn traiter_callback_telegram(
    client: &reqwest::Client,
    api: &str,
    cb: &serde_json::Value,
    state: &Arc<AppState>,
) {
    let data = cb["data"].as_str().unwrap_or("");
    let chat_id = cb["message"]["chat"]["id"].as_i64().unwrap_or(0);
    let cb_id = cb["id"].as_str().unwrap_or("");
    // Always answer the callback, otherwise Telegram spins on the button.
    let accuser = |texte: &str| {
        let body = serde_json::json!({ "callback_query_id": cb_id, "text": texte });
        client
            .post(format!("{api}/answerCallbackQuery"))
            .json(&body)
            .send()
    };

    let repondre = |texte: String| {
        let body = serde_json::json!({
            "chat_id": chat_id, "text": texte, "parse_mode": "Markdown"
        });
        client.post(format!("{api}/sendMessage")).json(&body).send()
    };

    match data.split_once(':') {
        // Approval decisions: routed to the tool call waiting in this chat's run.
        Some(("ok", tcid)) | Some(("always", tcid)) | Some(("no", tcid)) => {
            let approuve = !data.starts_with("no:");
            let courtier = courtiers_tg().lock().unwrap().get(&chat_id).cloned();
            match courtier {
                Some(tx) => {
                    let _ = tx
                        .send(laruche_essaim::brain::ApprovalResponse {
                            tool_call_id: tcid.to_string(),
                            approved: approuve,
                        })
                        .await;
                    // "Always": the engine already session-approves the class on an OK;
                    // this makes it permanent so it survives restarts too.
                    let _ = accuser(if !approuve {
                        "Refused"
                    } else if data.starts_with("always:") {
                        "Approved (this kind, always)"
                    } else {
                        "Approved"
                    })
                    .await;
                }
                None => {
                    let _ = accuser("This request has expired").await;
                }
            }
        }
        Some(("cmd", "sethome")) => {
            {
                let mut ec = state.essaim_config.write().await;
                ec.home_channel = Some(format!("telegram:{chat_id}"));
            }
            save_persistent_state(state).await;
            let _ = accuser("Home channel set").await;
            let _ = repondre("🏠 This chat is now your *home channel*.".into()).await;
        }
        Some(("cmd", "deny")) => {
            let regles = laruche_essaim::approbation::globales().regles_refus();
            let liste = if regles.is_empty() {
                "No deny rule set.".to_string()
            } else {
                regles
                    .iter()
                    .map(|r| format!("• `{}` {}", r.pattern, r.motif))
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            let _ = accuser("Deny rules").await;
            let _ = repondre(format!("*Deny rules*\n{liste}")).await;
        }
        Some(("cmd", autre)) => {
            // Remaining quick actions map onto the existing text commands: we ask
            // the user to send them (keeps ONE implementation of each command).
            let _ = accuser("Send the command").await;
            let _ = repondre(format!("Send `/{autre}` to run it.")).await;
        }
        _ => {
            let _ = accuser("").await;
        }
    }
}

// Telegram bot: runs as a background task within the server.
pub(crate) async fn run_telegram_bot(token: &str, allowed_chats: &str, state: &Arc<AppState>) {
    let api = format!("https://api.telegram.org/bot{}", token);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .unwrap();
    let allowed: Vec<String> = allowed_chats
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let mut offset: i64 = 0;
    let mut processed_ids: std::collections::HashSet<i64> = std::collections::HashSet::new();
    let mut tg_sessions: std::collections::HashMap<i64, Uuid> = std::collections::HashMap::new();
    // Chats that opted into voice replies (/voice), restored from the persisted config.
    let tg_voice: Arc<tokio::sync::RwLock<std::collections::HashSet<i64>>> = Arc::new(
        tokio::sync::RwLock::new(
            crate::voice_config::charger()
                .telegram_voice_chats
                .into_iter()
                .collect(),
        ),
    );
    let active_steers: Arc<
        tokio::sync::RwLock<std::collections::HashMap<i64, tokio::sync::mpsc::Sender<String>>>,
    > = Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new()));
    info!("Telegram bot polling started");

    // Delivery registry: an answer produced but never delivered (crash/restart
    // between "the answer exists" and "Telegram accepted it") is re-sent now.
    crate::outbox::rejouer(Some(token)).await;

    loop {
        let url = format!("{}/getUpdates?offset={}&timeout=30", api, offset);
        match client.get(&url).send().await {
            Ok(resp) => {
                if let Ok(data) = resp.json::<serde_json::Value>().await {
                    if let Some(updates) = data["result"].as_array() {
                        // Advance offset immediately to prevent duplicate processing
                        if let Some(last) = updates.last() {
                            offset = last["update_id"].as_i64().unwrap_or(0) + 1;
                            // Confirm offset with Telegram (quick call, no wait)
                            let _ = client
                                .get(format!("{}/getUpdates?offset={}&timeout=0", api, offset))
                                .send()
                                .await;
                        }

                        for update in updates {
                            let update_id = update["update_id"].as_i64().unwrap_or(0);
                            if processed_ids.contains(&update_id) {
                                continue;
                            }
                            processed_ids.insert(update_id);
                            // Keep set small: only remember last 100
                            if processed_ids.len() > 100 {
                                let min = *processed_ids.iter().min().unwrap_or(&0);
                                processed_ids.remove(&min);
                            }

                            // ── Inline BUTTON press (callback_query) ──
                            // Native buttons instead of typing commands. The payload is
                            // our own `data` string, never free user text.
                            if let Some(cb) = update.get("callback_query") {
                                traiter_callback_telegram(&client, &api, cb, state).await;
                                continue;
                            }

                            let chat_id = update["message"]["chat"]["id"].as_i64().unwrap_or(0);
                            let user = update["message"]["from"]["first_name"]
                                .as_str()
                                .unwrap_or("?");

                            // The reply format is governed only by the /voice toggle:
                            // ON -> voice note only; OFF -> text only (whatever the input was).
                            let voice_on = tg_voice.read().await.contains(&chat_id);

                            // Text message, or a voice/audio message. We prefer the local STT
                            // service when it answers (clean text, any model); otherwise we hand
                            // the audio straight to the model, which transcribes it itself if it
                            // is audio-capable (e.g. Gemma) - no separate STT service needed.
                            let mut text_owned =
                                update["message"]["text"].as_str().unwrap_or("").to_string();
                            let mut tg_attachment: Vec<laruche_essaim::session::Attachment> = Vec::new();
                            if text_owned.is_empty() && chat_id != 0 {
                                let file_id = update["message"]["voice"]["file_id"]
                                    .as_str()
                                    .or_else(|| update["message"]["audio"]["file_id"].as_str())
                                    .or_else(|| update["message"]["video_note"]["file_id"].as_str());
                                if let Some(fid) = file_id {
                                    // Default: let the model transcribe (native STT). The Settings
                                    // toggle forces the external STT service instead.
                                    let use_external_stt = crate::voice_config::charger().stt_external;
                                    match download_telegram_file(&client, &token, fid).await {
                                        Some(bytes) => {
                                            let stt_text = if use_external_stt {
                                                stt_transcribe_bytes(&bytes).await
                                            } else {
                                                None
                                            };
                                            if let Some(t) = stt_text {
                                                if !voice_on {
                                                    let _ = client.post(format!("{}/sendMessage", api))
                                                        .json(&serde_json::json!({"chat_id": chat_id, "text": format!("🎤 \"{}\"", t)}))
                                                        .send().await;
                                                }
                                                text_owned = t;
                                            } else if let Some(wav) = audio_to_wav(bytes).await {
                                                use base64::Engine;
                                                let b64 = base64::engine::general_purpose::STANDARD.encode(&wav);
                                                tg_attachment.push(laruche_essaim::session::Attachment {
                                                    kind: "audio".to_string(),
                                                    mime_type: "audio/wav".to_string(),
                                                    data: b64,
                                                    filename: None,
                                                });
                                                text_owned = "[The user sent a voice message. Listen to the attached audio and reply to it.]".to_string();
                                            } else {
                                                let _ = client.post(format!("{}/sendMessage", api))
                                                    .json(&serde_json::json!({"chat_id": chat_id, "text": "I could not handle that audio. Either run the STT service (:8421), or use an audio-capable model (with ffmpeg installed for conversion)."}))
                                                    .send().await;
                                                continue;
                                            }
                                        }
                                        None => {
                                            let _ = client.post(format!("{}/sendMessage", api))
                                                .json(&serde_json::json!({"chat_id": chat_id, "text": "Could not download that audio from Telegram."}))
                                                .send().await;
                                            continue;
                                        }
                                    }
                                }
                            }
                            let text = text_owned.as_str();

                            if text.is_empty() || chat_id == 0 {
                                continue;
                            }

                            // Check allowlist
                            if !allowed.is_empty() && !allowed.contains(&chat_id.to_string()) {
                                let _ = client.post(format!("{}/sendMessage", api))
                                    .json(&serde_json::json!({"chat_id": chat_id, "text": "Access denied."}))
                                    .send().await;
                                continue;
                            }

                            if text == "/start" || text == "/clear" || text == "/reset" {
                                tg_sessions.insert(chat_id, Uuid::new_v4());
                                let _ = client
                                    .post(format!("{}/sendMessage", api))
                                    .json(&serde_json::json!({
                                        "chat_id": chat_id,
                                        "text": "New conversation started. The session history has been reset.",
                                        "parse_mode": "Markdown",
                                    }))
                                    .send()
                                    .await;
                                continue;
                            }

                            // ── /deny: forbid a command class for good, WITH a reason ──
                            // `/deny <glob> | <reason>`. The rule fires before every bypass
                            // (auto mode included) and the reason travels back to the model
                            // so it corrects its behaviour instead of rephrasing.
                            if text.starts_with("/deny") {
                                let reste = text.trim_start_matches("/deny").trim();
                                if reste.is_empty() {
                                    let regles = laruche_essaim::approbation::globales().regles_refus();
                                    let liste = if regles.is_empty() {
                                        "No deny rule set.".to_string()
                                    } else {
                                        regles
                                            .iter()
                                            .map(|r| {
                                                if r.motif.is_empty() {
                                                    format!("• `{}`", r.pattern)
                                                } else {
                                                    format!("• `{}` - {}", r.pattern, r.motif)
                                                }
                                            })
                                            .collect::<Vec<_>>()
                                            .join("\n")
                                    };
                                    let aide = format!(
                                        "*Deny rules*\n{liste}\n\n\
                                         `/deny <pattern> | <reason>` to add one \
                                         (e.g. `/deny *rm -rf* | never delete recursively`).\n\
                                         `/undeny <pattern>` to lift one."
                                    );
                                    let _ = client.post(format!("{}/sendMessage", api))
                                        .json(&serde_json::json!({"chat_id": chat_id, "text": aide, "parse_mode": "Markdown"}))
                                        .send().await;
                                    continue;
                                }
                                let (pattern, motif) = match reste.split_once('|') {
                                    Some((p, m)) => (p.trim(), m.trim()),
                                    None => (reste, ""),
                                };
                                laruche_essaim::approbation::globales().refuser(pattern, motif);
                                // The reason is also a first-class user preference: the
                                // curateur treats corrections as capability signals.
                                if !motif.is_empty() {
                                    let mem = state.memoire.clone();
                                    let (p, m) = (pattern.to_string(), motif.to_string());
                                    tokio::spawn(async move {
                                        let _ = mem
                                            .write(laruche_memoire::MemoryItem::new(
                                                "preferences.refus",
                                                format!("The user forbids `{p}`: {m}"),
                                            ))
                                            .await;
                                    });
                                }
                                let _ = client.post(format!("{}/sendMessage", api))
                                    .json(&serde_json::json!({
                                        "chat_id": chat_id,
                                        "text": format!("🚫 Deny rule added: `{pattern}`. It cannot be bypassed, not even in auto mode."),
                                        "parse_mode": "Markdown",
                                    }))
                                    .send().await;
                                continue;
                            }
                            if text.starts_with("/undeny") {
                                let pattern = text.trim_start_matches("/undeny").trim();
                                let retire =
                                    laruche_essaim::approbation::globales().oublier_refus(pattern);
                                let msg = if retire {
                                    format!("✅ Deny rule lifted: `{pattern}`.")
                                } else {
                                    format!("No rule matches `{pattern}`.")
                                };
                                let _ = client.post(format!("{}/sendMessage", api))
                                    .json(&serde_json::json!({"chat_id": chat_id, "text": msg, "parse_mode": "Markdown"}))
                                    .send().await;
                                continue;
                            }

                            // ── /menu: native buttons instead of typing commands ──
                            if text == "/menu" {
                                let _ = client.post(format!("{}/sendMessage", api))
                                    .json(&serde_json::json!({
                                        "chat_id": chat_id,
                                        "text": "*LaRuche* - quick actions",
                                        "parse_mode": "Markdown",
                                        "reply_markup": clavier_menu(),
                                    }))
                                    .send().await;
                                continue;
                            }

                            // /sethome: sets THIS chat as the "home channel": default destination
                            // for proactive messages (cron, missions) without an explicit channel.
                            if text == "/sethome" {
                                let home = format!("telegram:{}", chat_id);
                                {
                                    let mut ec = state.essaim_config.write().await;
                                    ec.home_channel = Some(home.clone());
                                }
                                save_persistent_state(state).await;
                                let _ = client
                                    .post(format!("{}/sendMessage", api))
                                    .json(&serde_json::json!({
                                        "chat_id": chat_id,
                                        "text": "🏠 This chat is now your *home channel*. Scheduled tasks and missions without an explicit destination will reply here.",
                                        "parse_mode": "Markdown",
                                    }))
                                    .send()
                                    .await;
                                continue;
                            }

                            // /help: command list.
                            if text == "/help" || text == "/commands" {
                                let aide = "*LaRuche - commands*\n\
                                    *Status & info*\n\
                                    /help: this help\n\
                                    /status: model, home channel, crons\n\
                                    /model: current model + active profile\n\
                                    /reine: LaReine supervisor settings\n\
                                    /tools: registered tools\n\
                                    /skills: enabled skills\n\
                                    /missions: long-running missions\n\
                                    /tasks: kanban tasks\n\
                                    /crons: scheduled tasks\n\
                                    /memory <query>: search the cognitive memory\n\
                                    /whoami: this chat's identity\n\n\
                                    *Actions*\n\
                                    /clear (or /reset, /start): reset THIS chat's history\n\
                                    /sethome: set THIS chat as the task destination\n\
                                    /voice: toggle voice-note replies (TTS) for this chat\n\
                                    /delcron <name|all>: delete a cron (or all)\n\n\
                                    _Tip: send a message while a task runs to steer it. The full UI is at the web dashboard._";
                                let _ = client.post(format!("{}/sendMessage", api))
                                    .json(&serde_json::json!({"chat_id": chat_id, "text": aide, "parse_mode": "Markdown"}))
                                    .send().await;
                                continue;
                            }

                            // /status: current state.
                            if text == "/status" {
                                let modele = get_llm_default(state).await;
                                let home = state.essaim_config.read().await.home_channel.clone()
                                    .unwrap_or_else(|| "(not set)".into());
                                let n_crons = state.essaim_cron.read().await.list().len();
                                let msg = format!(
                                    "*LaRuche status*\nModel: `{modele}`\nHome: `{home}`\nCrons: {n_crons}"
                                );
                                let _ = client.post(format!("{}/sendMessage", api))
                                    .json(&serde_json::json!({"chat_id": chat_id, "text": msg, "parse_mode": "Markdown"}))
                                    .send().await;
                                continue;
                            }

                            // /crons: list the scheduled tasks.
                            if text == "/crons" {
                                let lignes: Vec<String> = state.essaim_cron.read().await.list()
                                    .iter()
                                    .map(|t| format!("• *{}* - `{}` (runs: {})", t.name, t.cron_expr.clone().unwrap_or_else(|| "one-off".into()), t.run_count))
                                    .collect();
                                let msg = if lignes.is_empty() {
                                    "No scheduled task.".to_string()
                                } else {
                                    format!("*Scheduled tasks*\n{}\n\n_Delete: /delcron <name> or /delcron all_", lignes.join("\n"))
                                };
                                let _ = client.post(format!("{}/sendMessage", api))
                                    .json(&serde_json::json!({"chat_id": chat_id, "text": msg, "parse_mode": "Markdown"}))
                                    .send().await;
                                continue;
                            }

                            // /delcron <name|all>: deletes a cron (or all). Stops the spam from Telegram.
                            if let Some(arg) = text.strip_prefix("/delcron").map(|s| s.trim()) {
                                let arg = arg.to_string();
                                let msg = {
                                    let mut sched = state.essaim_cron.write().await;
                                    if arg.is_empty() {
                                        "Usage: /delcron <name> or /delcron all".to_string()
                                    } else if arg.eq_ignore_ascii_case("all") {
                                        let ids: Vec<Uuid> = sched.list().iter().map(|t| t.id).collect();
                                        let n = ids.len();
                                        for id in ids { sched.remove(&id); }
                                        format!("🗑️ {n} cron(s) deleted.")
                                    } else {
                                        let id = sched.list().iter()
                                            .find(|t| t.name.eq_ignore_ascii_case(&arg))
                                            .map(|t| t.id);
                                        match id {
                                            Some(id) => { sched.remove(&id); format!("🗑️ Cron \"{arg}\" deleted.") }
                                            None => format!("No cron named \"{arg}\". See /crons."),
                                        }
                                    }
                                };
                                let _ = client.post(format!("{}/sendMessage", api))
                                    .json(&serde_json::json!({"chat_id": chat_id, "text": msg, "parse_mode": "Markdown"}))
                                    .send().await;
                                continue;
                            }

                            // Helper to push a Markdown reply to this chat.
                            macro_rules! repondre {
                                ($txt:expr) => {{
                                    let _ = client.post(format!("{}/sendMessage", api))
                                        .json(&serde_json::json!({"chat_id": chat_id, "text": $txt, "parse_mode": "Markdown"}))
                                        .send().await;
                                    continue;
                                }};
                            }

                            // /model: current model + active profile.
                            if text == "/model" || text == "/models" {
                                let modele = get_llm_default(state).await;
                                let (prov_model, prof) = {
                                    let p = state.profiles.read().await;
                                    (p.active_model.model.clone(), p.active_model.profile_id.clone())
                                };
                                repondre!(format!(
                                    "*Model*\nActive: `{modele}`\nProfile: `{prof}` (`{prov_model}`)\n\n_Switch models in the web UI > Settings > Providers._"
                                ));
                            }

                            // /reine: LaReine supervisor settings.
                            if text == "/reine" {
                                let rs = crate::reine_api::charger_reine_settings();
                                let reworks = if rs.max_revues == 255 { "unlimited".to_string() } else { rs.max_revues.to_string() };
                                repondre!(format!(
                                    "*LaReine* 👑\nMode: `{}`\nMax reworks: `{}`\nContext turns: `{}`\nProposals gate: `{}`\nSupervision: `{}`",
                                    rs.mode, reworks, rs.contexte_messages, rs.queue_gate, rs.tier_supervision
                                ));
                            }

                            // /tools: registered tools.
                            if text == "/tools" {
                                let mut noms = state.essaim_registry.noms();
                                noms.sort();
                                let n = noms.len();
                                let apercu = noms.iter().take(40).map(|s| format!("`{s}`")).collect::<Vec<_>>().join(", ");
                                let suite = if n > 40 { ", ..." } else { "" };
                                repondre!(format!("*Tools* ({n})\n{apercu}{suite}"));
                            }

                            // /skills: enabled skills on disk.
                            if text == "/skills" {
                                let mut slugs: Vec<String> = crate::mesh_api::lister_skills_locaux()
                                    .into_iter().map(|(slug, _, _)| slug).collect();
                                slugs.sort();
                                let msg = if slugs.is_empty() {
                                    "No skill.".to_string()
                                } else {
                                    format!("*Skills* ({})\n{}", slugs.len(),
                                        slugs.iter().map(|s| format!("• `{s}`")).collect::<Vec<_>>().join("\n"))
                                };
                                repondre!(msg);
                            }

                            // /missions: long-running missions.
                            if text == "/missions" {
                                let lignes: Vec<String> = state.missions.read().await.list().iter()
                                    .map(|m| {
                                        let obj: String = m.objective.chars().take(60).collect();
                                        format!("• *{}* - {} (_{}_)", m.slug, obj, m.status)
                                    })
                                    .collect();
                                let msg = if lignes.is_empty() { "No mission.".to_string() }
                                    else { format!("*Missions*\n{}", lignes.join("\n")) };
                                repondre!(msg);
                            }

                            // /tasks: kanban tasks.
                            if text == "/tasks" {
                                let lignes: Vec<String> = state.kanban_board.read().await.list().iter()
                                    .map(|t| format!("• {} - _{:?}_", t.title, t.status))
                                    .collect();
                                let msg = if lignes.is_empty() { "No task.".to_string() }
                                    else { format!("*Kanban tasks* ({})\n{}", lignes.len(), lignes.join("\n")) };
                                repondre!(msg);
                            }

                            // /voice: toggle voice-note replies (TTS) for THIS chat.
                            if text == "/voice" {
                                let on = {
                                    let mut v = tg_voice.write().await;
                                    if v.contains(&chat_id) {
                                        v.remove(&chat_id);
                                        false
                                    } else {
                                        v.insert(chat_id);
                                        true
                                    }
                                };
                                crate::voice_config::set_telegram_voice(chat_id, on); // persist
                                repondre!(if on {
                                    "🔊 Voice replies ON: I will also send my answers as a voice note. (/voice to turn off)"
                                } else {
                                    "🔇 Voice replies OFF."
                                });
                            }

                            // /whoami: this chat's identity + session.
                            if text == "/whoami" {
                                let home = state.essaim_config.read().await.home_channel.clone()
                                    .unwrap_or_else(|| "(not set)".into());
                                let is_home = home == format!("telegram:{}", chat_id);
                                repondre!(format!(
                                    "*This chat*\nChannel: `telegram:{chat_id}`\nName: {user}\nIs home channel: `{is_home}`"
                                ));
                            }

                            // /memory <query>: search the cognitive memory.
                            if let Some(q) = text.strip_prefix("/memory").map(|s| s.trim().to_string()) {
                                if q.is_empty() {
                                    repondre!("Usage: /memory <query>");
                                }
                                let res = state.memoire.grep(&q, Some(8)).await.ok();
                                let body = match res {
                                    Some(v) => {
                                        let s = serde_json::to_string(&v).unwrap_or_default();
                                        if s.is_empty() || s == "null" { "(no match)".to_string() }
                                        else { s.chars().take(600).collect::<String>() }
                                    }
                                    None => "(search failed)".to_string(),
                                };
                                repondre!(format!("*Memory* `{q}`\n```\n{body}\n```"));
                            }

                            // Check for active steering
                            let mut steers_lock = active_steers.write().await;
                            if let Some(steer_tx) = steers_lock.get(&chat_id) {
                                match steer_tx.try_send(text.to_string()) {
                                    Ok(()) => {
                                        let _ = client
                                            .post(format!("{}/sendMessage", api))
                                            .json(&serde_json::json!({
                                                "chat_id": chat_id,
                                                "text": "Steering received: applied at the next step.",
                                            }))
                                            .send()
                                            .await;
                                    }
                                    Err(_) => {
                                        let _ = client
                                            .post(format!("{}/sendMessage", api))
                                            .json(&serde_json::json!({
                                                "chat_id": chat_id,
                                                "text": "The task just finished: send this message as a new request.",
                                            }))
                                            .send()
                                            .await;
                                    }
                                }
                                continue;
                            }

                            // Setup steering for new task
                            let (steer_tx, steer_rx) = tokio::sync::mpsc::channel::<String>(100);
                            steers_lock.insert(chat_id, steer_tx);
                            drop(steers_lock);

                            info!(
                                user = user,
                                chat_id = chat_id,
                                text = %text.chars().take(50).collect::<String>(),
                                "Telegram message"
                            );

                            // Get or create LaRuche user for this Telegram chat_id
                            let tg_user_id = {
                                let tg_name = format!("telegram:{}", chat_id);
                                let users = state.users.read().await;
                                if let Some(u) = auth_user::find_user_by_name(&users, &tg_name) {
                                    u.id
                                } else {
                                    drop(users);
                                    let new_user = auth_user::create_user(
                                        &tg_name,
                                        auth_user::UserRole::User,
                                        None,
                                    );
                                    let uid = new_user.id;
                                    let _ = auth_user::save_user(
                                        &new_user,
                                        std::path::Path::new("users"),
                                    );
                                    state.users.write().await.insert(uid, new_user);
                                    info!(chat_id = chat_id, user_id = %uid, "Auto-created Telegram user");
                                    uid
                                }
                            };

                            // Telegram clears the "typing..." indicator after a few seconds.
                            // Keeping it up for the whole turn avoids the impression that the bot
                            // abandoned the request during a tool call or a long response.
                            let (typing_stop, mut typing_stopped) =
                                tokio::sync::watch::channel(false);
                            let typing_client = client.clone();
                            let typing_api = api.clone();
                            // Show "recording voice" when a voice note is coming, else "typing".
                            let typing_action = if voice_on { "record_voice" } else { "typing" };
                            let typing_task = tokio::spawn(async move {
                                let mut ticker =
                                    tokio::time::interval(std::time::Duration::from_secs(4));
                                loop {
                                    tokio::select! {
                                        _ = ticker.tick() => {
                                            if let Err(error) = typing_client
                                                .post(format!("{}/sendChatAction", typing_api))
                                                .json(&serde_json::json!({"chat_id": chat_id, "action": typing_action}))
                                                .send()
                                                .await
                                            {
                                                tracing::debug!(error = %error, chat_id, "Telegram typing update failed");
                                            }
                                        }
                                        changed = typing_stopped.changed() => {
                                            if changed.is_err() || *typing_stopped.borrow() {
                                                break;
                                            }
                                        }
                                    }
                                }
                            });

                            // Query agent with current default model
                            let current_model = get_llm_default(state).await;
                            let sessions_dir = std::path::Path::new("sessions");

                            let session_id =
                                // Deterministic id (channel:chat_id) → the history survives server
                                // restarts/rebuilds. /clear sets a temporary random id
                                // (reset until the next restart).
                                *tg_sessions.entry(chat_id).or_insert_with(|| {
                                    session_id_channel("telegram", &chat_id.to_string())
                                });
                            let mut session = if let Ok(mut loaded) =
                                Session::charger(&sessions_dir.join(format!("{}.json", session_id)))
                            {
                                loaded.model = current_model.clone();
                                loaded
                            } else {
                                Session::new_with_id(session_id, &current_model, sessions_dir)
                            };
                            session.user_id = Some(tg_user_id);
                            let (tx, _rx) = broadcast::channel::<ChatEvent>(64);

                            let mut config = state.essaim_config.read().await.clone();
                            config.model = current_model;
                            // Origin channel → cron_create will send the recurring task here, and the
                            // conversational memory is already tied to this Telegram session.
                            config.origin_channel = Some(format!("telegram:{}", chat_id));
                            // Per-channel model override (Settings > Channels): lets Telegram run a
                            // tool-reliable model. No override -> keeps the global active model.
                            apply_channel_model(state, "telegram", &mut config).await;

                            let state_clone = state.clone();
                            let client_clone = client.clone();
                            let api_clone = api.clone();
                            let text_clone = text.to_string();
                            let user_clone = user.to_string();
                            let active_steers_clone = active_steers.clone();

                            // Approvals over Telegram: the run gets a real approval channel,
                            // and a listener turns each ApprovalRequest into a message with
                            // native buttons. Without this the chat ran fully autonomously
                            // (no way to refuse a sensitive call from the phone).
                            let (approval_tx, approval_rx) = tokio::sync::mpsc::channel::<
                                laruche_essaim::brain::ApprovalResponse,
                            >(4);
                            enregistrer_courtier(chat_id, approval_tx);
                            let mut rx_appro = tx.subscribe();
                            let (appro_client, appro_api) = (client.clone(), api.clone());
                            let appro_task = tokio::spawn(async move {
                                while let Ok(ev) = rx_appro.recv().await {
                                    if let ChatEvent::ApprovalRequest { tool_call_id, name, args } = ev {
                                        let apercu: String =
                                            args.to_string().chars().take(400).collect();
                                        let _ = appro_client
                                            .post(format!("{appro_api}/sendMessage"))
                                            .json(&serde_json::json!({
                                                "chat_id": chat_id,
                                                "text": format!(
                                                    "🛡️ *Approval needed*\n`{name}`\n```\n{apercu}\n```"
                                                ),
                                                "parse_mode": "Markdown",
                                                "reply_markup": clavier_approbation(&tool_call_id),
                                            }))
                                            .send()
                                            .await;
                                    }
                                }
                            });

                            tokio::spawn(async move {
                                let result = boucle_react_memoire_multimodal(
                                    &text_clone,
                                    &mut session,
                                    &state_clone.essaim_registry,
                                    &config,
                                    &tx,
                                    state_clone.memoire.clone(),
                                    tg_attachment,
                                    Some(approval_rx),
                                    Some(steer_rx),
                                )
                                .await;
                                appro_task.abort();
                                oublier_courtier(chat_id);
                                let _ = typing_stop.send(true);
                                let _ = typing_task.await;

                                let mut response = match result {
                                    Ok(r) => {
                                        let mut clean = r;
                                        while let Some(s) = clean.find("<tool_call>") {
                                            if let Some(e) = clean.find("</tool_call>") {
                                                clean = format!(
                                                    "{}{}",
                                                    &clean[..s],
                                                    &clean[e + "</tool_call>".len()..]
                                                );
                                            } else {
                                                clean.truncate(s);
                                                break;
                                            }
                                        }
                                        while let Some(s) = clean.find("<plan>") {
                                            if let Some(e) = clean.find("</plan>") {
                                                clean = format!(
                                                    "{}{}",
                                                    &clean[..s],
                                                    &clean[e + "</plan>".len()..]
                                                );
                                            } else {
                                                clean.truncate(s);
                                                break;
                                            }
                                        }
                                        clean.trim().to_string()
                                    }
                                    Err(e) => format!("Error: {}", e),
                                };
                                if response.trim().is_empty() {
                                    response =
                                        "✅ Done. No additional text response."
                                            .to_string();
                                }

                                let chunks: Vec<String> = response
                                    .chars()
                                    .collect::<Vec<_>>()
                                    .chunks(4000)
                                    .map(|c| c.iter().collect())
                                    .collect();
                                // /voice ON: send the answer as a voice note ONLY (no text).
                                // If synthesis fails, fall back to text so nothing is lost.
                                let voice_sent = if voice_on {
                                    match send_telegram_voice(&client_clone, &api_clone, chat_id, &response).await {
                                        Ok(()) => true,
                                        Err(e) => {
                                            tracing::warn!(error = %e, chat_id, "Telegram voice reply failed; falling back to text");
                                            false
                                        }
                                    }
                                } else {
                                    false
                                };
                                if !voice_sent {
                                    for chunk in &chunks {
                                        // Delivery registry: the answer is recorded BEFORE the
                                        // send and cleared only once Telegram accepted it. A
                                        // crash in between re-sends it at the next boot instead
                                        // of losing the whole turn's work.
                                        let billet = crate::outbox::enregistrer(
                                            "telegram",
                                            &chat_id.to_string(),
                                            chunk,
                                        );
                                        match send_telegram_text(
                                            &client_clone,
                                            &api_clone,
                                            chat_id,
                                            chunk,
                                        )
                                        .await
                                        {
                                            Ok(()) => crate::outbox::confirmer(&billet),
                                            Err(error) => {
                                                tracing::error!(error = %error, chat_id, "Telegram final response failed to send (kept in the outbox)");
                                            }
                                        }
                                    }
                                }

                                let _ = session.sauvegarder();
                                state_clone
                                    .essaim_sessions
                                    .write()
                                    .await
                                    .insert(session.id, session.clone());

                                tracing::info!(
                                    user = user_clone,
                                    response_len = response.len(),
                                    "Telegram replied"
                                );

                                active_steers_clone.write().await.remove(&chat_id);
                            });
                        }
                    }
                }
            }
            Err(e) => {
                warn!(error = %e, "Telegram polling error");
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            }
        }
    }
}

/// Remove emoji / pictographs / variation selectors so a TTS does not pronounce them.
fn strip_emoji_for_speech(text: &str) -> String {
    text.chars()
        .filter(|c| {
            let u = *c as u32;
            !((0x1F000..=0x1FFFF).contains(&u)
                || (0x2600..=0x27BF).contains(&u)
                || (0x2B00..=0x2BFF).contains(&u)
                || (0x2190..=0x21FF).contains(&u)
                || (0x2300..=0x23FF).contains(&u)
                || (0xFE00..=0xFE0F).contains(&u)
                || u == 0x200D
                || u == 0x20E3)
        })
        .collect()
}

/// Download a Telegram file (voice/audio) to bytes.
async fn download_telegram_file(
    client: &reqwest::Client,
    token: &str,
    file_id: &str,
) -> Option<Vec<u8>> {
    let api = format!("https://api.telegram.org/bot{token}");
    let gf = client
        .get(format!("{api}/getFile"))
        .query(&[("file_id", file_id)])
        .send()
        .await
        .ok()?;
    let v: serde_json::Value = gf.json().await.ok()?;
    let file_path = v["result"]["file_path"].as_str()?;
    let url = format!("https://api.telegram.org/file/bot{token}/{file_path}");
    let bytes = client.get(&url).send().await.ok()?.bytes().await.ok()?;
    Some(bytes.to_vec())
}

/// Transcribe audio bytes via the local STT service (handles OGG/Opus via ffmpeg).
/// None if no STT service answers.
async fn stt_transcribe_bytes(bytes: &[u8]) -> Option<String> {
    let stt = reqwest::Client::new();
    let part = reqwest::multipart::Part::bytes(bytes.to_vec())
        .file_name("voice.oga")
        .mime_str("audio/ogg")
        .ok()?;
    let form = reqwest::multipart::Form::new().part("file", part);
    let resp = stt
        .post("http://127.0.0.1:8421/transcribe")
        .multipart(form)
        .send()
        .await
        .ok()?;
    let r: serde_json::Value = resp.json().await.ok()?;
    r.get("text")
        .and_then(|t| t.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Transcode audio bytes (OGG/Opus etc.) to 16kHz mono WAV via ffmpeg, so they can be
/// attached as `input_audio` to an audio-capable model. None if ffmpeg is unavailable.
async fn audio_to_wav(bytes: Vec<u8>) -> Option<Vec<u8>> {
    tokio::task::spawn_blocking(move || {
        use std::io::Write;
        if which::which("ffmpeg").is_err() {
            return None;
        }
        let dir = std::env::temp_dir();
        let stamp = std::process::id();
        let in_path = dir.join(format!("laruche_tg_in_{stamp}.bin"));
        let out_path = dir.join(format!("laruche_tg_out_{stamp}.wav"));
        std::fs::File::create(&in_path).ok()?.write_all(&bytes).ok()?;
        let ok = std::process::Command::new("ffmpeg")
            .args(["-y", "-i"])
            .arg(&in_path)
            .args(["-ar", "16000", "-ac", "1"])
            .arg(&out_path)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        let wav = if ok { std::fs::read(&out_path).ok() } else { None };
        let _ = std::fs::remove_file(&in_path);
        let _ = std::fs::remove_file(&out_path);
        wav
    })
    .await
    .ok()
    .flatten()
}

/// Synthesize `text` via the local TTS service (as OGG/opus) and send it as a Telegram
/// voice note. Best-effort: returns an error string the caller just logs.
/// Split text into speakable chunks of at most `max` chars, preferring sentence breaks,
/// so a long answer becomes several voice notes instead of one truncated one.
fn split_for_voice(text: &str, max: usize) -> Vec<String> {
    let text = text.trim();
    if text.is_empty() {
        return vec![];
    }
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= max {
        return vec![text.to_string()];
    }
    let mut out = Vec::new();
    let mut start = 0;
    while start < chars.len() {
        let end = (start + max).min(chars.len());
        let mut brk = end;
        if end < chars.len() {
            // Search backward (within the second half) for a sentence boundary.
            let mut i = end;
            while i > start + max / 2 {
                i -= 1;
                let c = chars[i];
                if c == '.' || c == '!' || c == '?' || c == '\n' {
                    brk = i + 1;
                    break;
                }
            }
        }
        let piece: String = chars[start..brk].iter().collect();
        let piece = piece.trim().to_string();
        if !piece.is_empty() {
            out.push(piece);
        }
        start = brk;
    }
    out
}

/// Synthesize one chunk and send it as a Telegram voice note.
async fn synth_and_send_voice(
    client: &reqwest::Client,
    tts: &reqwest::Client,
    api: &str,
    chat_id: i64,
    spoken: &str,
    speed: f32,
    voice: &str,
    backend: &str,
) -> std::result::Result<(), String> {
    let mut payload = serde_json::json!({ "text": spoken, "format": "ogg", "speed": speed });
    if !voice.is_empty() {
        payload["voice"] = serde_json::Value::String(voice.to_string());
    }
    if !backend.is_empty() {
        payload["backend"] = serde_json::Value::String(backend.to_string());
    }
    let resp = tts
        .post("http://127.0.0.1:8422/synthesize")
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("tts request: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("tts status {}", resp.status()));
    }
    let is_audio = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|c| c.contains("audio"))
        .unwrap_or(false);
    if !is_audio {
        return Err("tts did not return audio (check the TTS service)".into());
    }
    let bytes = resp.bytes().await.map_err(|e| format!("tts read: {e}"))?;
    let part = reqwest::multipart::Part::bytes(bytes.to_vec())
        .file_name("voice.ogg")
        .mime_str("audio/ogg")
        .map_err(|e| format!("mime: {e}"))?;
    let form = reqwest::multipart::Form::new()
        .text("chat_id", chat_id.to_string())
        .part("voice", part);
    let r = client
        .post(format!("{api}/sendVoice"))
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("sendVoice: {e}"))?;
    if !r.status().is_success() {
        return Err(format!("sendVoice status {}", r.status()));
    }
    Ok(())
}

pub(crate) async fn send_telegram_voice(
    client: &reqwest::Client,
    api: &str,
    chat_id: i64,
    text: &str,
) -> std::result::Result<(), String> {
    // Strip emoji (do not pronounce them) and split long answers into several notes.
    // Belt-and-suspenders: the TTS service also strips emoji.
    let cfg = crate::voice_config::charger();
    let stripped = strip_emoji_for_speech(text);
    let mut chunks = split_for_voice(&stripped, 1000);
    // Cap the number of notes so a very long answer cannot flood the chat.
    const MAX_NOTES: usize = 8;
    if chunks.len() > MAX_NOTES {
        tracing::warn!(chat_id, total = chunks.len(), "Telegram voice answer truncated to {MAX_NOTES} notes");
        chunks.truncate(MAX_NOTES);
    }
    if chunks.is_empty() {
        return Ok(());
    }
    let tts = reqwest::Client::new();
    let mut sent = 0usize;
    let mut last_err = None;
    for chunk in chunks {
        match synth_and_send_voice(
            client,
            &tts,
            api,
            chat_id,
            &chunk,
            cfg.tts_speed,
            &cfg.tts_voice,
            &cfg.tts_backend,
        )
        .await
        {
            Ok(()) => sent += 1,
            Err(e) => {
                last_err = Some(e);
                break;
            }
        }
    }
    if sent == 0 {
        return Err(last_err.unwrap_or_else(|| "no audio sent".into()));
    }
    Ok(())
}

/// Sends a plain Telegram message and treats API rejections as real errors.
///
/// Agent output is intentionally not parsed as Telegram Markdown: ordinary code snippets,
/// paths and tool output frequently contain unbalanced Markdown markers.
pub(crate) async fn send_telegram_text(
    client: &reqwest::Client,
    api: &str,
    chat_id: i64,
    text: &str,
) -> Result<()> {
    let response = client
        .post(format!("{api}/sendMessage"))
        .json(&serde_json::json!({
            "chat_id": chat_id,
            "text": text,
        }))
        .send()
        .await?;
    if response.status().is_success() {
        return Ok(());
    }
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    Err(anyhow::anyhow!(
        "Telegram sendMessage rejected ({status}): {body}"
    ))
}

/// Helper: run agent query and return cleaned response text.
/// DETERMINISTIC session id for a (channel, user): survives restarts, unlike
/// a random UUID. Same (channel, key) → same session → conversational memory.
/// Example key: `telegram:12345`, `discord:bob`, `slack:C07...`.
fn session_id_channel(channel: &str, user_key: &str) -> Uuid {
    Uuid::new_v5(
        &Uuid::NAMESPACE_OID,
        format!("{channel}:{user_key}").as_bytes(),
    )
}

/// Runs an agent query for a CHANNEL (Discord, Slack, ...) with a **persistent session**
/// per (channel, user) → conversational memory between messages, like Telegram.
/// Any new channel that calls this function gets the memory for free.
pub(crate) async fn run_agent_query(
    state: &Arc<AppState>,
    channel: &str,
    user_key: &str,
    text: &str,
) -> String {
    let current_model = get_llm_default(state).await;
    let sessions_dir = std::path::Path::new("sessions");
    let session_id = session_id_channel(channel, user_key);
    let mut session = match Session::charger(&sessions_dir.join(format!("{}.json", session_id))) {
        Ok(mut loaded) => {
            loaded.model = current_model.clone();
            loaded
        }
        Err(_) => Session::new_with_id(session_id, &current_model, sessions_dir),
    };
    let (tx, _rx) = broadcast::channel::<ChatEvent>(64);

    let mut config = state.essaim_config.read().await.clone();
    config.model = current_model;
    // Origin channel → cron_create will send the recurring task here; also serves as the home key.
    config.origin_channel = Some(format!("{channel}:{user_key}"));
    // Per-channel model override (Settings > Channels).
    apply_channel_model(state, channel, &mut config).await;

    let result = boucle_react_memoire(
        text,
        &mut session,
        &state.essaim_registry,
        &config,
        &tx,
        state.memoire.clone(),
    )
    .await;

    // Persist the session (the agent already added the current turn + its responses) → the
    // next message from the same (channel, user) reloads it with the full history.
    let _ = session.sauvegarder();
    state
        .essaim_sessions
        .write()
        .await
        .insert(session.id, session);

    match result {
        Ok(r) => {
            let mut clean = r;
            while let Some(s) = clean.find("<tool_call>") {
                if let Some(e) = clean.find("</tool_call>") {
                    clean = format!("{}{}", &clean[..s], &clean[e + "</tool_call>".len()..]);
                } else {
                    clean.truncate(s);
                    break;
                }
            }
            while let Some(s) = clean.find("<plan>") {
                if let Some(e) = clean.find("</plan>") {
                    clean = format!("{}{}", &clean[..s], &clean[e + "</plan>".len()..]);
                } else {
                    clean.truncate(s);
                    break;
                }
            }
            clean.trim().to_string()
        }
        Err(e) => format!("Error: {}", e),
    }
}
