//! System diagnostics endpoint (health check and configuration validation) - split out of main.rs.

use crate::*;
use axum::extract::State;
use axum::response::Json;
use std::sync::Arc;

/// GET /api/travaux - what LaRuche is doing right now, one entry per running job. Polled
/// by the status-bar indicator, so it stays cheap: a read lock and a serialization, no
/// network call and no disk.
pub(crate) async fn api_travaux(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let travaux: Vec<Travail> = state
        .travaux
        .read()
        .map(|m| m.values().cloned().collect())
        .unwrap_or_default();
    Json(serde_json::json!({ "travaux": travaux }))
}

/// GET /api/mcp/bans - addresses currently serving a ban on the MCP surface.
/// POST with {"ip":"..."} lifts one, for when the banned client is your own machine.
pub(crate) async fn api_mcp_bans(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let maintenant = std::time::Instant::now();
    let bans: Vec<serde_json::Value> = state
        .mcp_verrou
        .lock()
        .map(|v| {
            v.bannies(maintenant)
                .into_iter()
                .map(|(ip, reste)| serde_json::json!({ "ip": ip.to_string(), "reste_s": reste }))
                .collect()
        })
        .unwrap_or_default();
    Json(serde_json::json!({ "bans": bans }))
}

/// POST /api/mcp/bans {ip} - lift a ban by hand. Admin only: unbanning is a security
/// decision, not a convenience.
pub(crate) async fn api_mcp_unban(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    if !auth_user::require_admin(&state, &headers).await {
        return Json(serde_json::json!({"ok": false, "error": "admin required"}));
    }
    let Some(ip) = body["ip"].as_str().and_then(|s| s.parse().ok()) else {
        return Json(serde_json::json!({"ok": false, "error": "ip required"}));
    };
    let leve = state
        .mcp_verrou
        .lock()
        .map(|mut v| v.liberer(ip))
        .unwrap_or(false);
    log_activite(
        &state,
        "warn",
        "mcp",
        format!("MCP ban lifted by hand for {ip}"),
        None,
    )
    .await;
    Json(serde_json::json!({ "ok": leve }))
}

/// GET /api/doctor - system health check and configuration validation.
pub(crate) async fn api_doctor(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let mut checks = Vec::new();

    // Check Ollama connectivity.
    //
    // 1.5s, not 5. This endpoint is called by EVERY Settings section, so with Ollama
    // absent (a DeepSeek or llama.cpp setup) the whole panel waited five seconds per
    // tab. A reachable Ollama answers /api/tags in milliseconds on loopback and well
    // under a second on a LAN; past that it is down for our purposes.
    let ec = state.essaim_config.read().await;

    // Probe the provider ACTUALLY in use, not Ollama on principle. A perfectly healthy
    // llama.cpp, LM Studio or DeepSeek setup was reported as an error because Ollama was
    // absent, which made the panel say something false about a working install. Ollama is
    // one option among several, not the standard.
    let (nom_fournisseur, url_sonde) = match ec.provider.as_str() {
        "ollama" | "" => ("Ollama", Some(format!("{}/api/tags", ec.ollama_url))),
        "llamacpp" | "llama.cpp" | "llama-server" => (
            "llama.cpp",
            Some(format!(
                "{}/v1/models",
                ec.api_base.as_deref().unwrap_or("http://127.0.0.1:8001")
            )),
        ),
        "lmstudio" | "lm-studio" => (
            "LM Studio",
            Some(format!(
                "{}/v1/models",
                ec.api_base.as_deref().unwrap_or("http://127.0.0.1:1234")
            )),
        ),
        "vllm" => (
            "vLLM",
            Some(format!(
                "{}/v1/models",
                ec.api_base.as_deref().unwrap_or("http://127.0.0.1:8000")
            )),
        ),
        // Le pont ChatGPT expose /health, qui dit aussi si le navigateur est au bout.
        "chatgpt-bridge" | "chatgpt-web" => (
            "ChatGPT Bridge",
            Some(format!(
                "{}/health",
                ec.api_base.as_deref().unwrap_or("http://127.0.0.1:8787")
            )),
        ),
        // A hosted provider: reaching it costs a billable request and proves little more
        // than the key being present, so it is reported as configured without a probe.
        "anthropic" => ("Anthropic", None),
        "codex" => ("ChatGPT Codex", None),
        autre if autre.starts_with("peer:") => ("Swarm node", None),
        _ => ("OpenAI-compatible", ec.api_base.clone().map(|b| format!("{b}/v1/models"))),
    };

    match url_sonde {
        Some(url) => {
            let joignable = reqwest::Client::new()
                .get(&url)
                .timeout(std::time::Duration::from_millis(1500))
                .send()
                .await
                .map(|r| r.status().is_success())
                .unwrap_or(false);
            checks.push(serde_json::json!({
                "name": nom_fournisseur,
                "status": if joignable { "ok" } else { "error" },
                "detail": if joignable { format!("Connected to {url}") }
                          else { format!("Cannot reach {url}") },
            }));
        }
        None => checks.push(serde_json::json!({
            "name": nom_fournisseur,
            "status": "ok",
            "detail": "Remote provider, configured",
        })),
    }

    // Le pont ChatGPT depend d'un navigateur qui reste ouvert. Son serveur repond
    // meme quand l'onglet a ete ferme, si bien que la sonde generique ci-dessus
    // afficherait un vert trompeur. On relit /health pour dire ce qui manque
    // vraiment: le serveur, ou le navigateur au bout du fil.
    if matches!(ec.provider.as_str(), "chatgpt-bridge" | "chatgpt-web") {
        let base = ec
            .api_base
            .as_deref()
            .unwrap_or("http://127.0.0.1:8787")
            .trim_end_matches('/')
            .to_string();
        let reponse = reqwest::Client::new()
            .get(format!("{base}/health"))
            .timeout(std::time::Duration::from_millis(1500))
            .send()
            .await;
        let agent = match reponse {
            Ok(r) => r
                .json::<serde_json::Value>()
                .await
                .ok()
                .and_then(|v| v["agent"].as_bool()),
            Err(_) => None,
        };
        checks.push(serde_json::json!({
            "name": "ChatGPT Bridge - browser",
            "status": if agent == Some(true) { "ok" } else { "error" },
            "detail": match agent {
                Some(true) => "Chrome extension connected".to_string(),
                Some(false) => format!("Bridge running at {base} but no browser attached: open the ChatGPT tab and check the extension popup"),
                None => format!("No /health at {base}: the bridge server is not running"),
            },
        }));
    }

    // Check model availability
    checks.push(serde_json::json!({
        "name": "Model",
        "status": "ok",
        "detail": format!("Default model: {}", ec.model),
    }));
    let _ = ec;

    // Check Miel network
    let listener = state.listener.read().await;
    let nodes = listener.get_nodes().await;
    checks.push(serde_json::json!({
        "name": "Miel Network",
        "status": "ok",
        "detail": format!("{} peer(s) discovered", nodes.len()),
    }));

    // Check STT/TTS: real HTTP probe on the same URLs the runtime resolves
    // (local defaults, mesh discovery on top), not just mesh capability flags.
    drop(listener);
    let (stt_url, tts_url) = crate::voice_api::resolve_voice_urls(&state).await;
    let (stt_up, tts_up) = tokio::join!(
        crate::voice_api::voice_service_up(&stt_url),
        crate::voice_api::voice_service_up(&tts_url),
    );
    checks.push(serde_json::json!({
        "name": "STT Service",
        "status": if stt_up { "ok" } else { "warning" },
        "detail": if stt_up { format!("Responding at {stt_url}") }
                  else { format!("No /health at {stt_url} - voice input disabled") },
    }));
    checks.push(serde_json::json!({
        "name": "TTS Service",
        "status": if tts_up { "ok" } else { "warning" },
        "detail": if tts_up { format!("Responding at {tts_url}") }
                  else { format!("No /health at {tts_url} - voice output disabled") },
    }));

    // Check sessions directory
    let sessions_ok = std::path::Path::new("sessions").exists();
    checks.push(serde_json::json!({
        "name": "Sessions Storage",
        "status": if sessions_ok { "ok" } else { "warning" },
        "detail": if sessions_ok { "sessions/ directory exists" } else { "Will be created on first chat" },
    }));

    // Check plugins directory
    let plugins_dir = std::path::Path::new("plugins");
    let plugin_count = if plugins_dir.exists() {
        std::fs::read_dir(plugins_dir)
            .map(|entries| {
                entries
                    .filter(|e| {
                        e.as_ref()
                            .map(|e| e.path().extension().is_some_and(|ext| ext == "json"))
                            .unwrap_or(false)
                    })
                    .count()
            })
            .unwrap_or(0)
    } else {
        0
    };
    checks.push(serde_json::json!({
        "name": "Plugins",
        "status": "ok",
        "detail": format!("{} plugin(s) loaded", plugin_count),
    }));

    // Check Chrome for browser tools
    let chrome_found = if cfg!(windows) {
        std::path::Path::new(r"C:\Program Files\Google\Chrome\Application\chrome.exe").exists()
            || std::path::Path::new(r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe")
                .exists()
    } else {
        which::which("google-chrome").is_ok() || which::which("chromium-browser").is_ok()
    };
    checks.push(serde_json::json!({
        "name": "Browser (Chrome/Edge)",
        "status": if chrome_found { "ok" } else { "warning" },
        "detail": if chrome_found { "Available for browser_navigate/screenshot" } else { "Not found - browser tools disabled" },
    }));

    // Check TLS configuration: mirror the startup resolution (explicit cert/key
    // win, else LARUCHE_HTTPS=1 self-signs) and verify the files are actually
    // readable, since an unreadable cert makes the server fall back to plain HTTP.
    let tls_cert = std::env::var("LARUCHE_TLS_CERT").ok().filter(|s| !s.is_empty());
    let tls_key = std::env::var("LARUCHE_TLS_KEY").ok().filter(|s| !s.is_empty());
    let (tls_status, tls_detail) = match (tls_cert, tls_key) {
        (Some(cert), Some(key)) => {
            let readable = std::fs::metadata(&cert).is_ok() && std::fs::metadata(&key).is_ok();
            if readable {
                ("ok", "TLS enabled (cert/key readable)".to_string())
            } else {
                ("error", format!("LARUCHE_TLS_CERT/KEY set but unreadable ({cert}) - server fell back to plain HTTP"))
            }
        }
        _ if std::env::var("LARUCHE_HTTPS").as_deref() == Ok("1") => {
            ("ok", "Self-signed TLS (LARUCHE_HTTPS=1, generated at startup)".to_string())
        }
        _ => ("warning", "Not configured - using plain HTTP".to_string()),
    };
    checks.push(serde_json::json!({
        "name": "TLS/HTTPS",
        "status": tls_status,
        "detail": tls_detail,
    }));

    // Abeilles count
    checks.push(serde_json::json!({
        "name": "Tools",
        "status": "ok",
        "detail": format!("{} tools registered", state.essaim_registry.noms().len()),
    }));

    let all_ok = checks.iter().all(|c| c["status"].as_str() != Some("error"));

    Json(serde_json::json!({
        "status": if all_ok { "healthy" } else { "unhealthy" },
        "checks": checks,
        "version": "0.2.0",
        "protocol": "Miel",
    }))
}
