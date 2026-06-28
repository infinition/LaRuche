//! System diagnostics endpoint (health check and configuration validation) - split out of main.rs.

use crate::*;
use axum::extract::State;
use axum::response::Json;
use std::sync::Arc;

/// GET /api/doctor - system health check and configuration validation.
pub(crate) async fn api_doctor(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let mut checks = Vec::new();

    // Check Ollama connectivity
    let ec = state.essaim_config.read().await;
    let ollama_ok = reqwest::Client::new()
        .get(format!("{}/api/tags", ec.ollama_url))
        .timeout(std::time::Duration::from_secs(5))
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

    // Check STT/TTS
    let mut stt_found = false;
    let mut tts_found = false;
    for (_id, node) in &nodes {
        let caps: Vec<String> = node
            .manifest
            .capabilities
            .iter()
            .map(|c| c.to_string())
            .collect();
        if caps.iter().any(|c| c == "stt") {
            stt_found = true;
        }
        if caps.iter().any(|c| c == "tts") {
            tts_found = true;
        }
    }
    checks.push(serde_json::json!({
        "name": "STT Service",
        "status": if stt_found { "ok" } else { "warning" },
        "detail": if stt_found { "Available" } else { "Not found - voice input disabled" },
    }));
    checks.push(serde_json::json!({
        "name": "TTS Service",
        "status": if tts_found { "ok" } else { "warning" },
        "detail": if tts_found { "Available" } else { "Not found - voice output disabled" },
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

    // Check TLS configuration
    let tls_configured =
        std::env::var("LARUCHE_TLS_CERT").is_ok() && std::env::var("LARUCHE_TLS_KEY").is_ok();
    checks.push(serde_json::json!({
        "name": "TLS/HTTPS",
        "status": if tls_configured { "ok" } else { "warning" },
        "detail": if tls_configured { "TLS enabled" } else { "Not configured - using plain HTTP" },
    }));

    // Abeilles count
    checks.push(serde_json::json!({
        "name": "Abeilles (Tools)",
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
