//! OpenAI-compatible chat completions endpoint with signed peer verification - split out of main.rs.

use crate::*;
use axum::extract::State;
use std::sync::Arc;

// --- P3: OpenAI-compatible /v1/chat/completions ---
#[derive(Deserialize)]
pub struct OpenAiChatReq {
    pub model: String,
    pub messages: Vec<serde_json::Value>,
    #[serde(default)]
    pub stream: bool,
    #[serde(default)]
    pub temperature: Option<f32>,
    #[serde(default)]
    pub max_tokens: Option<u32>,
}

/// Fetches (with caching) a peer's public key via its /api/mesh/identity. Verifies that the
/// node_id announced by the IP matches the one declared (X-Miel-From).
pub(crate) async fn peer_pubkey(node_id: &str, ip: &str) -> Option<String> {
    use std::sync::{Mutex, OnceLock};
    static CACHE: OnceLock<Mutex<std::collections::HashMap<String, String>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(std::collections::HashMap::new()));
    if let Some(pk) = cache.lock().unwrap().get(node_id) {
        return Some(pk.clone());
    }
    let url = format!("http://{ip}:8419/api/mesh/identity");
    let resp: serde_json::Value = reqwest::Client::new()
        .get(&url)
        .timeout(std::time::Duration::from_secs(3))
        .send()
        .await
        .ok()?
        .json()
        .await
        .ok()?;
    let nid = resp.get("node_id").and_then(|v| v.as_str())?;
    let pk = resp.get("pubkey").and_then(|v| v.as_str())?.to_string();
    if nid != node_id {
        return None; // the IP does not match the declared node_id -> suspicious
    }
    cache.lock().unwrap().insert(node_id.to_string(), pk.clone());
    Some(pk)
}

/// VERIFIED node_id (ed25519 signature) of the caller of an inference request, or None.
pub(crate) async fn verified_inference_caller(
    headers: &axum::http::HeaderMap,
    addr: &std::net::SocketAddr,
) -> Option<String> {
    let from = headers.get("X-Miel-From").and_then(|v| v.to_str().ok())?.to_string();
    let pubkey = peer_pubkey(&from, &addr.ip().to_string()).await?;
    sync::verified_caller(headers, "/v1/chat/completions", &pubkey)
}

pub(crate) async fn api_v1_chat_completions(
    State(state): State<Arc<AppState>>,
    axum::extract::ConnectInfo(addr): axum::extract::ConnectInfo<std::net::SocketAddr>,
    headers: axum::http::HeaderMap,
    axum::extract::Json(req): axum::extract::Json<OpenAiChatReq>,
) -> axum::response::Response {
    use axum::response::IntoResponse;
    use futures_util::StreamExt;

    // 1. Resolve provider based on req.model or active_model
    let mut provider_id = "ollama".to_string();
    let mut api_key = "".to_string();
    let mut api_base = None;
    let ollama_url = state.config.ollama_url.clone();
    let mut vis = profiles::Visibilite::Prive;
    let mut allowed: Vec<String> = Vec::new();

    {
        let profiles = state.profiles.read().await;
        let mut found = false;
        for profile in profiles.profiles.values() {
            if profile.models.contains(&req.model) {
                provider_id = profile.provider.clone();
                api_key = profile.api_key.clone();
                api_base = Some(profile.base_url.clone());
                vis = profile.visibilite;
                allowed = profile.allowed_peers.clone();
                found = true;
                break;
            }
        }
        if !found {
            // fallback to active model provider if it matches
            let active = &profiles.active_model;
            if let Some(p) = profiles.profiles.get(&active.profile_id) {
                provider_id = p.provider.clone();
                api_key = p.api_key.clone();
                api_base = Some(p.base_url.clone());
                vis = p.visibilite;
                allowed = p.allowed_peers.clone();
            }
        }
    }

    // Mesh ENFORCEMENT: a REMOTE caller (non-loopback) may only use this model according to
    // its visibility. The local node (loopback) is never blocked.
    if !addr.ip().is_loopback() {
        let refus = |msg: &str| {
            (
                axum::http::StatusCode::FORBIDDEN,
                axum::Json(serde_json::json!({ "error": { "message": msg, "type": "forbidden" } })),
            )
                .into_response()
        };
        match vis {
            profiles::Visibilite::Prive => {
                return refus("Private model: not shared on the mesh.");
            }
            profiles::Visibilite::Restricted => {
                match verified_inference_caller(&headers, &addr).await {
                    Some(nid) if allowed.iter().any(|a| a == &nid) => {} // allowed
                    Some(_) => return refus("Ruche not authorized for this model (restricted)."),
                    None => return refus("Mesh identity required/invalid for a restricted model."),
                }
            }
            // « Public » veut dire public POUR LA RUCHE, pas pour le reseau. Sans cette
            // verification, toute machine du LAN pouvait depenser une cle d'API en
            // pointant sur ce noeud: le garde de visibilite fermait le prive et le
            // restreint, mais laissait le public totalement ouvert.
            //
            // mesh_auth_ok rend None quand AUCUN code n'est configure: on reste alors
            // ouvert, faute de tout moyen d'authentifier. Des qu'un code existe, il est
            // exige - et les appels sortants sont deja signes par MESH_SIGNER, donc un
            // pair legitime passe sans rien changer chez lui.
            profiles::Visibilite::PublicProxy => {
                if let Some(false) = sync::mesh_auth_ok(&headers, "/v1/chat/completions") {
                    return refus(
                        "Mesh code required: this ruche shares its models with its own swarm, \
                         not with the whole network. Configure the same code in Settings > Network.",
                    );
                }
            }
        }
    }

    let temp = req.temperature.unwrap_or(0.7);
    let max_t = req.max_tokens.unwrap_or(2048);

    match laruche_essaim::providers::provider_chat_stream(
        &provider_id,
        &req.model,
        &req.messages,
        temp,
        max_t,
        &api_key,
        api_base.as_deref(),
        &ollama_url,
            None,
        ).await
    {
        Ok(mut stream) => {
            if req.stream {
                let model_name = req.model.clone();
                let stream_res = stream.map(move |chunk| {
                    let json_str = serde_json::json!({
                        "id": "chatcmpl-mesh",
                        "object": "chat.completion.chunk",
                        "created": 0,
                        "model": model_name.clone(),
                        "choices": [{
                            "index": 0,
                            "delta": {
                                "content": chunk.text
                            },
                            "finish_reason": if chunk.done { Some("stop") } else { None::<&str> }
                        }]
                    })
                    .to_string();

                    let mut out = format!("data: {}\n\n", json_str);
                    if chunk.done {
                        out.push_str("data: [DONE]\n\n");
                    }
                    Ok::<_, std::convert::Infallible>(axum::body::Bytes::from(out))
                });
                axum::response::Response::builder()
                    .header("Content-Type", "text/event-stream")
                    .header("Cache-Control", "no-cache")
                    .body(axum::body::Body::from_stream(stream_res))
                    .unwrap()
            } else {
                let mut full_text = String::new();
                while let Some(chunk) = stream.next().await {
                    full_text.push_str(&chunk.text);
                }
                let res = serde_json::json!({
                    "id": "chatcmpl-mesh",
                    "object": "chat.completion",
                    "created": 0,
                    "model": req.model,
                    "choices": [{
                        "index": 0,
                        "message": {
                            "role": "assistant",
                            "content": full_text
                        },
                        "finish_reason": "stop"
                    }]
                });
                axum::Json(res).into_response()
            }
        }
        Err(e) => {
            let res = serde_json::json!({
                "error": { "message": e.to_string(), "type": "server_error" }
            });
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(res),
            )
                .into_response()
        }
    }
}
