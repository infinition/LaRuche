//! Detection of **OpenAI-compatible** local inference backends (llama.cpp, vLLM,
//! LM Studio, etc.) to expose/announce them on the mesh as we already do for Ollama.
//!
//! Ollama is probed elsewhere (`fetch_local_models`, `:11434/api/tags`). Here we cover
//! any endpoint exposing `GET /v1/models` (OpenAI protocol), the de facto standard
//! of llama.cpp/vLLM/LM Studio/TGI.

use serde::Serialize;
use std::time::Duration;

/// An OpenAI-compatible backend to probe: readable label + root base_url (without `/v1`).
#[derive(Debug, Clone)]
pub struct BackendLocal {
    pub label: String,
    pub base_url: String,
}

/// A model detected on a local backend.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ModeleLocalDetecte {
    pub backend: String,
    pub base_url: String,
    pub name: String,
}

/// Local backends probed by default. Overridable via `LARUCHE_OPENAI_ENDPOINTS`
/// (comma-separated `label=url` list, e.g.
/// `llama.cpp=http://127.0.0.1:8001,vllm=http://127.0.0.1:8000`).
pub fn backends_openai_compat_par_defaut() -> Vec<BackendLocal> {
    if let Ok(spec) = std::env::var("LARUCHE_OPENAI_ENDPOINTS") {
        let parsed: Vec<BackendLocal> = spec
            .split(',')
            .filter_map(|e| {
                let (label, url) = e.trim().split_once('=')?;
                if label.trim().is_empty() || url.trim().is_empty() {
                    return None;
                }
                Some(BackendLocal {
                    label: label.trim().to_string(),
                    base_url: url.trim().trim_end_matches('/').to_string(),
                })
            })
            .collect();
        if !parsed.is_empty() {
            return parsed;
        }
    }
    vec![
        BackendLocal {
            label: "llama.cpp".into(),
            base_url: "http://127.0.0.1:8001".into(),
        },
        BackendLocal {
            label: "lmstudio".into(),
            base_url: "http://127.0.0.1:1234".into(),
        },
        BackendLocal {
            label: "vllm".into(),
            base_url: "http://127.0.0.1:8000".into(),
        },
        BackendLocal {
            label: "chatgpt-bridge".into(),
            base_url: "http://127.0.0.1:8787".into(),
        },
    ]
}

/// Parses the `GET /v1/models` (OpenAI) body into model ids. Pure, testable.
pub fn parser_modeles_openai(body: &serde_json::Value) -> Vec<String> {
    body["data"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|m| m["id"].as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

/// Probes each backend (`GET {base_url}/v1/models`, short timeout). Unreachable
/// backends are silently ignored (closed port = fast failure).
pub async fn detecter_modeles_openai_compat(backends: &[BackendLocal]) -> Vec<ModeleLocalDetecte> {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_millis(500))
        .build()
    {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let mut out = Vec::new();
    for b in backends {
        let url = format!("{}/v1/models", b.base_url.trim_end_matches('/'));
        let Ok(resp) = client.get(&url).send().await else {
            continue;
        };
        if !resp.status().is_success() {
            continue;
        }
        let Ok(body) = resp.json::<serde_json::Value>().await else {
            continue;
        };
        for name in parser_modeles_openai(&body) {
            out.push(ModeleLocalDetecte {
                backend: b.label.clone(),
                base_url: b.base_url.clone(),
                name,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_v1_models() {
        let body = serde_json::json!({
            "object": "list",
            "data": [
                {"id": "qwen3.6-35b-a3b", "object": "model"},
                {"id": "llama-3.3-70b", "object": "model"}
            ]
        });
        assert_eq!(
            parser_modeles_openai(&body),
            vec!["qwen3.6-35b-a3b", "llama-3.3-70b"]
        );
        assert!(parser_modeles_openai(&serde_json::json!({})).is_empty());
    }

    #[test]
    fn env_override_parse() {
        std::env::set_var(
            "LARUCHE_OPENAI_ENDPOINTS",
            "llama.cpp=http://127.0.0.1:8001/, foo=http://x:9",
        );
        let b = backends_openai_compat_par_defaut();
        assert_eq!(b.len(), 2);
        assert_eq!(b[0].label, "llama.cpp");
        assert_eq!(b[0].base_url, "http://127.0.0.1:8001");
        std::env::remove_var("LARUCHE_OPENAI_ENDPOINTS");
    }
}
