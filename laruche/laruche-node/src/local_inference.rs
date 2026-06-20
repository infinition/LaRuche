//! Détection des backends d'inférence locaux **OpenAI-compatibles** (llama.cpp, vLLM,
//! LM Studio…) pour les exposer/annoncer sur le mesh comme on le fait déjà pour Ollama.
//!
//! Ollama est sondé ailleurs (`fetch_local_models`, `:11434/api/tags`). Ici on couvre
//! tout endpoint exposant `GET /v1/models` (protocole OpenAI), qui est le standard de
//! facto de llama.cpp/vLLM/LM Studio/TGI.

use serde::Serialize;
use std::time::Duration;

/// Un backend OpenAI-compatible à sonder : label lisible + base_url racine (sans `/v1`).
#[derive(Debug, Clone)]
pub struct BackendLocal {
    pub label: String,
    pub base_url: String,
}

/// Un modèle détecté sur un backend local.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ModeleLocalDetecte {
    pub backend: String,
    pub base_url: String,
    pub name: String,
}

/// Backends locaux sondés par défaut. Surchargeable via `LARUCHE_OPENAI_ENDPOINTS`
/// (liste `label=url` séparée par des virgules, ex.
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
    ]
}

/// Parse le corps de `GET /v1/models` (OpenAI) → ids de modèles. Pur, testable.
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

/// Sonde chaque backend (`GET {base_url}/v1/models`, timeout court). Les backends
/// injoignables sont silencieusement ignorés (port fermé = échec rapide).
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
