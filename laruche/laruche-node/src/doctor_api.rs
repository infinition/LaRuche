//! System diagnostics endpoint (health check and configuration validation) - split out of main.rs.

use crate::*;
use axum::extract::State;
use axum::response::Json;
use std::sync::Arc;

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
    let ollama_ok = reqwest::Client::new()
        .get(format!("{}/api/tags", ec.ollama_url))
        .timeout(std::time::Duration::from_millis(1500))
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false);
    checks.push(serde_json::json!({
        "name": "Ollama",
        "status": if ollama_ok { "ok" } else { "error" },
        "detail": if ollama_ok { format!("Connected to {}", ec.ollama_url) }
                  else { format!("Cannot reach {}", ec.ollama_url) },
    }));

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
                            .map(|e| e.path().extension().map_or(false, |ext| ext == "json"))
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
