//! Provider Profiles: multi-provider LLM configuration.
//!
//! Stores multiple LLM provider profiles (Ollama, OpenAI-compatible, Anthropic)
//! in `provider-profiles.json`. Each profile has its own base_url, api_key, and
//! model list. One model is "active" at any time.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tracing::{info, warn};

/// Provider visibility on the Miel mesh.
/// - `Prive`: usable only by this node; its API key is never advertised.
/// - `PublicProxy`: this node becomes a gateway, the mesh can use this provider
///   via it. The key stays local, the node relays and runs the calls (the raw
///   key is NEVER exposed on the network).
/// - `Restricted`: gateway, but limited to the hives listed in `allowed_peers` (node_ids).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum Visibilite {
    #[default]
    Prive,
    PublicProxy,
    Restricted,
}

/// A single provider profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderProfile {
    /// Provider type: "ollama", "openai", "anthropic", "codex", "miel" (remote node)
    pub provider: String,
    /// Human-readable display name
    pub name: String,
    /// Base URL for the provider API
    pub base_url: String,
    /// API key (empty for Ollama)
    #[serde(default)]
    pub api_key: String,
    /// Known models for this profile
    #[serde(default)]
    pub models: Vec<String>,
    /// Mesh visibility (private by default). JSON: `"visibility"`.
    #[serde(default, rename = "visibility")]
    pub visibilite: Visibilite,
    /// node_ids of the hives allowed when `visibilite == Restricted`.
    #[serde(default)]
    pub allowed_peers: Vec<String>,
    /// Maximum context window in tokens for this provider's models.
    /// Used to display the context gauge ratio in the UI.
    /// Sensible defaults: ollama=32768, openai=128000, anthropic=200000, codex=128000.
    #[serde(default = "default_max_context_length")]
    pub max_context_length: u32,
}

fn default_max_context_length() -> u32 {
    128000
}

/// Which model is currently active.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveModel {
    pub profile_id: String,
    pub model: String,
}

/// Root structure for provider-profiles.json.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfilesConfig {
    #[serde(default = "default_version")]
    pub version: u32,
    pub profiles: HashMap<String, ProviderProfile>,
    pub active_model: ActiveModel,
}

fn default_version() -> u32 {
    1
}

impl Default for ProfilesConfig {
    fn default() -> Self {
        let mut profiles = HashMap::new();
        profiles.insert(
            "ollama-local".to_string(),
            ProviderProfile {
                provider: "ollama".to_string(),
                name: "Ollama Local".to_string(),
                base_url: "http://127.0.0.1:11434".to_string(),
                api_key: String::new(),
                models: vec![],
                visibilite: Visibilite::Prive, allowed_peers: Vec::new(),
                max_context_length: 32768,
            },
        );
        Self {
            version: 1,
            profiles,
            active_model: ActiveModel {
                profile_id: "ollama-local".to_string(),
                model: "gemma4:e4b".to_string(),
            },
        }
    }
}

/// A unified model entry for the dropdown.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedModel {
    /// Composite ID: "profile_id/model_name"
    pub id: String,
    /// Model name
    pub name: String,
    /// Profile ID it belongs to
    pub profile: String,
    /// Profile display name
    pub profile_name: String,
    /// Provider type
    pub provider: String,
}

/// Load profiles from disk, creating defaults if missing.
pub fn load_profiles(path: &Path) -> ProfilesConfig {
    if path.exists() {
        match std::fs::read_to_string(path) {
            Ok(raw) => match serde_json::from_str::<ProfilesConfig>(&raw) {
                Ok(cfg) => {
                    info!(path = %path.display(), profiles = cfg.profiles.len(), "Loaded provider profiles");
                    return cfg;
                }
                Err(e) => {
                    warn!(error = %e, "Failed to parse provider-profiles.json, using defaults");
                }
            },
            Err(e) => {
                warn!(error = %e, "Failed to read provider-profiles.json, using defaults");
            }
        }
    }
    let cfg = ProfilesConfig::default();
    let _ = save_profiles(path, &cfg);
    info!("Created default provider-profiles.json with ollama-local profile");
    cfg
}

/// Save profiles to disk atomically.
pub fn save_profiles(path: &Path, config: &ProfilesConfig) -> Result<()> {
    let json = serde_json::to_string_pretty(config)?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, &json)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

/// Get the active profile and model name. Returns (profile, model_name).
#[allow(dead_code)]
pub fn get_active_model(config: &ProfilesConfig) -> Option<(&ProviderProfile, &str)> {
    let active = &config.active_model;
    config
        .profiles
        .get(&active.profile_id)
        .map(|p| (p, active.model.as_str()))
}

/// Build a unified list of all models across all profiles.
pub fn build_unified_models(config: &ProfilesConfig) -> Vec<UnifiedModel> {
    let mut result = Vec::new();
    // Sort profiles by name for consistent ordering
    let mut profile_ids: Vec<&String> = config.profiles.keys().collect();
    profile_ids.sort();

    for profile_id in profile_ids {
        let profile = &config.profiles[profile_id];
        for model in &profile.models {
            result.push(UnifiedModel {
                id: format!("{}/{}", profile_id, model),
                name: model.clone(),
                profile: profile_id.clone(),
                profile_name: profile.name.clone(),
                provider: profile.provider.clone(),
            });
        }
    }
    result
}

/// Auto-discover Ollama models by calling {base_url}/api/tags.
pub async fn discover_ollama_models(base_url: &str) -> Vec<String> {
    let url = format!("{}/api/tags", base_url.trim_end_matches('/'));
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap_or_default();

    match client.get(&url).send().await {
        Ok(resp) if resp.status().is_success() => match resp.json::<serde_json::Value>().await {
            Ok(body) => body["models"]
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .filter_map(|m| m["name"].as_str().map(|s| s.to_string()))
                .collect(),
            Err(_) => vec![],
        },
        _ => vec![],
    }
}

/// Auto-discover llama.cpp / OpenAI-compatible models by calling {base_url}/v1/models.
pub async fn discover_llamacpp_models(base_url: &str) -> Vec<String> {
    let url = format!("{}/v1/models", base_url.trim_end_matches('/'));
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap_or_default();

    match client.get(&url).send().await {
        Ok(resp) if resp.status().is_success() => match resp.json::<serde_json::Value>().await {
            Ok(body) => {
                // OpenAI/llama.cpp: {"data":[{"id":...}]}; llama.cpp also exposes {"models":[{"name":...}]}
                let from_data: Vec<String> = body["data"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .filter_map(|m| m["id"].as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default();
                if !from_data.is_empty() {
                    return from_data;
                }
                body["models"]
                    .as_array()
                    .unwrap_or(&vec![])
                    .iter()
                    .filter_map(|m| {
                        m["name"]
                            .as_str()
                            .or_else(|| m["id"].as_str())
                            .map(|s| s.to_string())
                    })
                    .collect()
            }
            Err(_) => vec![],
        },
        _ => vec![],
    }
}

/// Refresh Ollama models for all ollama-type profiles in the config.
pub async fn refresh_ollama_profiles(config: &mut ProfilesConfig) {
    let ollama_ids: Vec<(String, String)> = config
        .profiles
        .iter()
        .filter(|(_, p)| p.provider == "ollama")
        .map(|(id, p)| (id.clone(), p.base_url.clone()))
        .collect();

    for (id, base_url) in ollama_ids {
        let models = discover_ollama_models(&base_url).await;
        if let Some(profile) = config.profiles.get_mut(&id) {
            if !models.is_empty() {
                profile.models = models;
            } else {
                // Ollama is local: unreachable (server down) means EMPTY list (reflects the
                // real state); it refills as soon as it responds again.
                profile.models.clear();
            }
        }
    }

    // Local OpenAI-compatible profiles (llama.cpp, LM Studio, ...): /v1/models.
    let openai_ids: Vec<(String, String)> = config
        .profiles
        .iter()
        .filter(|(_, p)| p.provider == "openai")
        .map(|(id, p)| (id.clone(), p.base_url.clone()))
        .collect();
    for (id, base_url) in openai_ids {
        let models = discover_llamacpp_models(&base_url).await;
        // Probe the REAL context window (n_ctx) of a local llama.cpp server: its openai
        // default (128000) is wrong for local (often 32768/8192), otherwise the context
        // overflows (HTTP 400 "context size exceeded"). We align max_context_length to the server.
        let nctx = if est_endpoint_local(&base_url) {
            discover_llamacpp_nctx(&base_url).await
        } else {
            None
        };
        if let Some(profile) = config.profiles.get_mut(&id) {
            if !models.is_empty() {
                profile.models = models;
            } else if est_endpoint_local(&base_url) {
                // Local provider (llama.cpp/LM Studio) down means EMPTY list. We do NOT touch
                // cloud providers (remote OpenAI) whose /v1/models may fail transiently.
                profile.models.clear();
            }
            if let Some(n) = nctx {
                profile.max_context_length = n;
            }
        }
    }
}

/// Probe the REAL context window (`n_ctx`) of a llama.cpp server via `/props`.
/// `None` if unreachable or if the field is absent.
pub async fn discover_llamacpp_nctx(base_url: &str) -> Option<u32> {
    let url = format!("{}/props", base_url.trim_end_matches('/'));
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .ok()?;
    let resp = client.get(&url).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let body: serde_json::Value = resp.json().await.ok()?;
    body.get("default_generation_settings")
        .and_then(|g| g.get("n_ctx"))
        .and_then(|v| v.as_u64())
        .or_else(|| body.get("n_ctx").and_then(|v| v.as_u64()))
        .map(|n| n as u32)
}

/// Is an endpoint LOCAL (server on the machine)? Used to clear the model list
/// when a local provider is unreachable, without affecting cloud providers.
fn est_endpoint_local(url: &str) -> bool {
    let u = url.to_lowercase();
    u.contains("127.0.0.1") || u.contains("localhost") || u.contains("0.0.0.0") || u.contains("[::1]")
}

/// Add a local llama.cpp profile when an OpenAI-compatible server is listening on :8001.
pub async fn ensure_llamacpp_8001_profile(config: &mut ProfilesConfig) {
    let id = "llamacpp-8001".to_string();
    let base_url = "http://127.0.0.1:8001".to_string();
    let models = discover_llamacpp_models(&base_url).await;
    if models.is_empty() {
        return;
    }

    config.profiles.insert(
        id.clone(),
        ProviderProfile {
            provider: "openai".to_string(),
            name: "llama.cpp Local (:8001)".to_string(),
            base_url,
            api_key: String::new(),
            models: models.clone(),
            visibilite: Visibilite::Prive, allowed_peers: Vec::new(),
            max_context_length: 32768,
        },
    );

    let active_is_unusable_default = config.active_model.profile_id == "ollama-local"
        && config
            .profiles
            .get("ollama-local")
            .map(|p| p.models.is_empty())
            .unwrap_or(true);
    if active_is_unusable_default {
        config.active_model = ActiveModel {
            profile_id: id,
            model: models[0].clone(),
        };
    }

    info!(
        models = models.len(),
        "Detected local llama.cpp OpenAI-compatible server on :8001"
    );
}

/// Convert the active profile into EssaimConfig-compatible fields.
/// Returns (provider, model, api_key, api_base, ollama_url, max_context_length).
pub fn active_to_essaim_fields(
    config: &ProfilesConfig,
) -> (String, String, String, Option<String>, String, u32) {
    let active = &config.active_model;
    if let Some(profile) = config.profiles.get(&active.profile_id) {
        let ollama_url = if profile.provider == "ollama" {
            profile.base_url.clone()
        } else {
            // Find the first ollama profile's base_url as fallback
            config
                .profiles
                .values()
                .find(|p| p.provider == "ollama")
                .map(|p| p.base_url.clone())
                .unwrap_or_else(|| "http://127.0.0.1:11434".to_string())
        };

        let api_base = if profile.provider != "ollama" {
            Some(profile.base_url.clone())
        } else {
            None
        };

        (
            profile.provider.clone(),
            active.model.clone(),
            profile.api_key.clone(),
            api_base,
            ollama_url,
            profile.max_context_length,
        )
    } else {
        // Fallback
        (
            "ollama".to_string(),
            active.model.clone(),
            String::new(),
            None,
            "http://127.0.0.1:11434".to_string(),
            32768,
        )
    }
}
