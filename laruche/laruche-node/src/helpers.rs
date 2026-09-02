//! Shared helpers (capability normalization, peer/model fetch, misc text utils) - split out of main.rs.

use crate::*;
use axum::http::StatusCode;

pub(crate) const PEER_FETCH_TIMEOUT_MS: u64 = 4000;
// Peer staleness window. MUST be > the mDNS re-announce interval (30s below),
// otherwise a peer "flickers": it goes stale between two announcements. 90s tolerates 2 missed announcements.
pub(crate) const PEER_STALE_SECS: i64 = 90;

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct OllamaModelInfo {
    pub(crate) name: String,
    pub(crate) size_gb: f64,
    pub(crate) digest: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ModelsResponse {
    pub(crate) models: Vec<OllamaModelInfo>,
    pub(crate) default_model: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct PeerStatusResponse {
    pub(crate) node_name: String,
    pub(crate) capabilities: Vec<String>,
    pub(crate) tokens_per_sec: f32,
    pub(crate) queue_depth: usize,
    pub(crate) memory_used_mb: u64,
    pub(crate) memory_total_mb: u64,
    pub(crate) memory_usage_pct: f32,
    pub(crate) cpu_usage_pct: f32,
    pub(crate) vram_total_mb: Option<u64>,
}

/// Infer a Miel capability from a model name using heuristics.
/// Falls back to "llm" if no specific pattern is matched.
pub(crate) fn infer_capability_from_model_name(name: &str) -> String {
    let lower = name.to_lowercase();
    if lower.contains("coder")
        || lower.contains("codestral")
        || lower.contains("deepseek-coder")
        || lower.contains("starcoder")
        || lower.contains("code")
    {
        return "code".into();
    }
    if lower.contains("llava")
        || lower.contains("bakllava")
        || lower.contains("moondream")
        || lower.contains("minicpm-v")
        || lower.contains("vision")
    {
        return "vlm".into();
    }
    if lower.contains("whisper") || lower.contains("audio") {
        return "audio".into();
    }
    if lower.contains("nomic-embed")
        || lower.contains("mxbai-embed")
        || lower.contains("all-minilm")
        || lower.contains("embed")
    {
        return "embed".into();
    }
    if lower.contains("stable-diffusion") || lower.contains("sdxl") || lower.contains("dall") {
        return "image".into();
    }
    "llm".into()
}

/// Resolve capability for a model: first check CapabilityConfig mappings, then heuristic.
pub(crate) fn resolve_model_capability(model_name: &str, capabilities: &[CapabilityConfig]) -> String {
    // Check if any capability config explicitly maps this model
    for cap in capabilities {
        let cap_model = cap.model_name.to_lowercase();
        let check = model_name.to_lowercase();
        if check == cap_model
            || check.starts_with(&format!("{}:", cap_model))
            || cap_model.starts_with(&check)
        {
            return normalize_capability_label(&cap.capability);
        }
    }
    infer_capability_from_model_name(model_name)
}

/// Read the "llm" default model from the per-capability map, falling back to config.
pub(crate) async fn get_llm_default(state: &AppState) -> String {
    // First check profiles (new system)
    let profiles = state.profiles.read().await;
    if !profiles.active_model.model.is_empty() {
        return profiles.active_model.model.clone();
    }
    drop(profiles);
    // Fallback to old default_models
    let dm = state.default_models.read().await;
    dm.get("llm")
        .cloned()
        .unwrap_or_else(|| state.config.default_model.clone())
}

/// Override a config's provider/model with the per-channel choice (Settings >
/// Channels). Channels without an override keep the global active model, so this
/// is always safe to call. Lets e.g. Telegram run a tool-reliable model while the
/// web chat uses a faster one.
pub(crate) async fn apply_channel_model(state: &AppState, channel: &str, config: &mut EssaimConfig) {
    let profiles = state.profiles.read().await;
    if let Some((profile, model)) = profiles::model_for_channel(&profiles, channel) {
        config.provider = profile.provider.clone();
        config.api_key = profile.api_key.clone();
        config.api_base = if profile.base_url.is_empty() {
            None
        } else {
            Some(profile.base_url.clone())
        };
        config.model = model.to_string();
    }
}

/// Declare a job as running and keep it visible until the returned guard is dropped.
/// The provider and model come from the config the job actually runs on, not the global
/// one, so a cron on a cheap local model does not claim to be using the chat's model.
pub(crate) fn ouvrir_travail(
    state: &AppState,
    acteur: &str,
    sujet: &str,
    config: &EssaimConfig,
    canal: Option<String>,
) -> GardeTravail {
    GardeTravail::nouveau(
        &state.travaux,
        Travail {
            acteur: acteur.to_string(),
            sujet: sujet.to_string(),
            fournisseur: if config.provider.is_empty() {
                "ollama".to_string()
            } else {
                config.provider.clone()
            },
            modele: config.model.clone(),
            canal,
            depuis: chrono::Utc::now().to_rfc3339(),
        },
    )
}

pub(crate) fn preview_text(input: &str, max_chars: usize) -> String {
    let flat = input.replace(['\n', '\r'], " ");
    let truncated: String = flat.chars().take(max_chars).collect();
    if flat.chars().count() > max_chars {
        format!("{truncated}...")
    } else {
        truncated
    }
}

/// Le texte COMPLET d'une reponse, borne mais non aplati.
///
/// Jumeau de `preview_text`, et la difference est tout le sujet: un apercu tient
/// sur une ligne, une reponse conservee non. Les quatre endroits qui archivent une
/// reponse d'agent passaient par `preview_text`, qui remplace les retours a la
/// ligne par des espaces: le markdown etait donc detruit a l'ecriture, bien avant
/// que le flux ne tente de le rendre. Le gras survivait, etant en ligne, mais les
/// listes et les titres arrivaient colles en un seul pave.
pub(crate) fn texte_complet(input: &str, max_chars: usize) -> String {
    if input.chars().count() <= max_chars {
        return input.to_string();
    }
    let tronque: String = input.chars().take(max_chars).collect();
    format!("{tronque}...")
}

pub(crate) fn inject_no_think(prompt: &str, no_think: bool) -> String {
    if no_think && !prompt.trim_start().starts_with("/no_think") {
        format!("/no_think\n{prompt}")
    } else {
        prompt.to_string()
    }
}

pub(crate) fn normalize_capability_label(raw: &str) -> String {
    raw.strip_prefix("capability:")
        .unwrap_or(raw)
        .trim()
        .to_lowercase()
}

pub(crate) fn normalize_capabilities(caps: Vec<String>) -> Vec<String> {
    let mut normalized: Vec<String> = caps
        .into_iter()
        .map(|c| normalize_capability_label(&c))
        .filter(|c| !c.is_empty())
        .collect();
    normalized.sort();
    normalized.dedup();
    normalized
}

pub(crate) fn merge_capabilities(primary: Vec<String>, fallback: Vec<String>) -> Vec<String> {
    let mut merged = normalize_capabilities(primary);
    for cap in normalize_capabilities(fallback) {
        if !merged.contains(&cap) {
            merged.push(cap);
        }
    }
    merged.sort();
    merged.dedup();
    merged
}

pub(crate) fn format_host_for_url(host: &str) -> String {
    if host.contains(':') && !host.starts_with('[') && !host.ends_with(']') {
        format!("[{host}]")
    } else {
        host.to_string()
    }
}

pub(crate) fn endpoint_url(host: &str, port: u16, path: &str) -> String {
    let safe_host = format_host_for_url(host);
    format!("http://{safe_host}:{port}{path}")
}

pub(crate) fn is_stale(last_seen: chrono::DateTime<chrono::Utc>) -> bool {
    (chrono::Utc::now() - last_seen).num_seconds() > PEER_STALE_SECS
}

pub(crate) async fn fetch_peer_status(
    client: &reqwest::Client,
    host: &str,
    port: u16,
) -> Option<PeerStatusResponse> {
    let url = endpoint_url(host, port, "/");
    match client.get(url).send().await {
        Ok(resp) if resp.status().is_success() => resp.json::<PeerStatusResponse>().await.ok(),
        _ => None,
    }
}

pub(crate) async fn fetch_models_from_node(
    client: &reqwest::Client,
    host: &str,
    port: u16,
) -> Option<ModelsResponse> {
    let url = endpoint_url(host, port, "/models");
    match client.get(url).send().await {
        Ok(resp) if resp.status().is_success() => resp.json::<ModelsResponse>().await.ok(),
        _ => None,
    }
}

pub(crate) async fn fetch_local_models(
    ollama_url: &str,
    default_model: &str,
) -> Result<ModelsResponse, StatusCode> {
    let client = reqwest::Client::new();
    let url = format!("{ollama_url}/api/tags");

    match client.get(&url).send().await {
        Ok(resp) => match resp.json::<serde_json::Value>().await {
            Ok(body) => {
                let models: Vec<OllamaModelInfo> = body["models"]
                    .as_array()
                    .unwrap_or(&vec![])
                    .iter()
                    .map(|m| OllamaModelInfo {
                        name: m["name"].as_str().unwrap_or("unknown").to_string(),
                        size_gb: m["size"].as_f64().unwrap_or(0.0) / 1_073_741_824.0,
                        digest: m["digest"]
                            .as_str()
                            .unwrap_or("")
                            .chars()
                            .take(12)
                            .collect(),
                    })
                    .collect();

                Ok(ModelsResponse {
                    models,
                    default_model: default_model.to_string(),
                })
            }
            Err(_) => Err(StatusCode::BAD_GATEWAY),
        },
        Err(_) => Err(StatusCode::SERVICE_UNAVAILABLE),
    }
}
