//! An App's window onto LaRuche's cognitive memory.
//!
//! Unlike `files.rs` and `storage.rs`, nothing here is confined to the App: the
//! memory is LaRuche's, shared by every agent and every session, and an item
//! written from a notebook is read months later by something that has no idea a
//! notebook existed. That is the whole value, and the whole risk.
//!
//! So writing defaults to LaReine's review queue rather than the store. An App
//! that wants a fact remembered proposes it and a human decides, which is the
//! same road the agents already take. Writing straight through stays possible,
//! because the capability was granted for a reason, but it has to be asked for
//! explicitly rather than being what happens when nobody thought about it.
use super::AppSnapshot;
use crate::{auth_user, log_activite, AppState};
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;

const CAPABILITY: &str = "laruche.memory";
const MAX_CONTENT_BYTES: usize = 32 * 1024;
const MAX_NODE_BYTES: usize = 200;
const MAX_QUERY_BYTES: usize = 500;
const MAX_TAGS: usize = 12;

type ApiError = (StatusCode, Json<Value>);

#[derive(Debug, Deserialize)]
#[serde(tag = "op", rename_all = "camelCase", deny_unknown_fields)]
pub(crate) enum MemoryRequest {
    Search {
        query: String,
        #[serde(default)]
        limit: Option<u8>,
    },
    Read {
        // `rename_all` sur l'enumeration ne renomme que les VARIANTES, pas les
        // champs. Sans ce rename explicite, le pont enverrait `nodeId` comme le
        // reste de l'API des Apps et le noeud refuserait un champ inconnu.
        #[serde(rename = "nodeId")]
        node_id: String,
    },
    List,
    Write {
        #[serde(rename = "nodeId")]
        node_id: String,
        content: String,
        #[serde(default)]
        tags: Vec<String>,
        /// Absent means proposed. See the module header.
        #[serde(default)]
        direct: bool,
    },
}

pub(crate) async fn handle(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(request): Json<MemoryRequest>,
) -> Result<Json<Value>, ApiError> {
    let user_id = authenticated_user(&state, &headers).await?;
    {
        let registry = state.apps.read().await;
        let app = registry
            .get(&id)
            .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "app_not_found", "App not found"))?;
        authorize(&app)?;
    }
    validate(&request)?;

    let response = match request {
        MemoryRequest::Search { query, limit } => {
            let opts = laruche_memoire::SearchOpts {
                limit: Some(limit.unwrap_or(8).clamp(1, 30)),
                ..Default::default()
            };
            // `ContextPack` ne porte qu'un `raw`: on rend ce JSON tel quel plutot
            // que de le reemballer, pour que l'App voie ce que voit l'agent.
            state
                .memoire
                .search(&query, opts)
                .await
                .map_err(memory_error)?
                .raw
        }
        MemoryRequest::Read { node_id } => state
            .memoire
            .read_node(&node_id)
            .await
            .map_err(memory_error)?,
        MemoryRequest::List => state.memoire.list_nodes().await.map_err(memory_error)?,
        MemoryRequest::Write {
            node_id,
            content,
            tags,
            direct,
        } => {
            let mut item = laruche_memoire::MemoryItem::new(&node_id, &content);
            if !tags.is_empty() {
                item = item.with_tags(tags);
            }
            item.source = Some(format!("app:{id}"));
            if direct {
                let written = state.memoire.write(item).await.map_err(memory_error)?;
                log_activite(
                    &state,
                    "info",
                    "apps",
                    format!("App {id} wrote memory node {node_id}"),
                    Some(user_id),
                )
                .await;
                json!({ "written": true, "queued": false, "result": written })
            } else {
                laruche_essaim::reine_queue::proposer_memoire(
                    &state.memoire,
                    item,
                    true,
                    "humaine",
                    "app",
                )
                .await;
                log_activite(
                    &state,
                    "info",
                    "apps",
                    format!("App {id} proposed memory node {node_id}"),
                    Some(user_id),
                )
                .await;
                json!({ "written": false, "queued": true })
            }
        }
    };
    Ok(Json(response))
}

fn validate(request: &MemoryRequest) -> Result<(), ApiError> {
    let invalid = |message: &str| api_error(StatusCode::BAD_REQUEST, "validation_failed", message);
    match request {
        MemoryRequest::Search { query, .. } => {
            if query.trim().is_empty() || query.len() > MAX_QUERY_BYTES {
                return Err(invalid("query is empty or too long"));
            }
        }
        MemoryRequest::Read { node_id } => validate_node(node_id).map_err(|m| invalid(&m))?,
        MemoryRequest::List => {}
        MemoryRequest::Write {
            node_id,
            content,
            tags,
            ..
        } => {
            validate_node(node_id).map_err(|m| invalid(&m))?;
            if content.trim().is_empty() || content.len() > MAX_CONTENT_BYTES {
                return Err(invalid("content is empty or over 32 KiB"));
            }
            if tags.len() > MAX_TAGS || tags.iter().any(|t| t.len() > 60 || t.trim().is_empty()) {
                return Err(invalid("tags are too many or malformed"));
            }
        }
    }
    Ok(())
}

/// A dotted node identifier and nothing else.
///
/// The memory addresses its nodes by a dotted path, and a slash or a parent hop
/// there does not mean anything: rejecting them keeps a malformed id from
/// reaching a store that would have to guess what was meant.
fn validate_node(node_id: &str) -> Result<(), String> {
    let trimmed = node_id.trim();
    if trimmed.is_empty()
        || trimmed.len() > MAX_NODE_BYTES
        || trimmed.starts_with('.')
        || trimmed.ends_with('.')
        || trimmed.contains("..")
        || !trimmed
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
    {
        return Err("nodeId must be a dotted identifier, for example projects.analyse".into());
    }
    Ok(())
}

async fn authenticated_user(state: &AppState, headers: &HeaderMap) -> Result<Uuid, ApiError> {
    let user_id =
        auth_user::extract_user_from_headers(headers, &state.cookie_secret).ok_or_else(|| {
            api_error(
                StatusCode::UNAUTHORIZED,
                "authentication_required",
                "Authentication required",
            )
        })?;
    if state.users.read().await.contains_key(&user_id) {
        Ok(user_id)
    } else {
        Err(api_error(
            StatusCode::UNAUTHORIZED,
            "authentication_required",
            "Authentication required",
        ))
    }
}

fn authorize(app: &AppSnapshot) -> Result<(), ApiError> {
    if !app.enabled {
        return Err(api_error(
            StatusCode::CONFLICT,
            "app_disabled",
            "App is disabled",
        ));
    }
    if !app.granted_permissions.iter().any(|item| item == CAPABILITY) {
        return Err(api_error(
            StatusCode::FORBIDDEN,
            "permission_denied",
            "Capability not granted",
        ));
    }
    Ok(())
}

fn memory_error(error: anyhow::Error) -> ApiError {
    tracing::warn!(error = %error, "app memory operation failed");
    api_error(
        StatusCode::INTERNAL_SERVER_ERROR,
        "memory_unavailable",
        "Memory operation failed",
    )
}

fn api_error(status: StatusCode, code: &str, message: &str) -> ApiError {
    (
        status,
        Json(json!({ "error": { "code": code, "message": message } })),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_identifiant_de_noeud_doit_etre_pointe() {
        for bon in ["projects.analyse", "people.fabien", "a", "a.b.c_d-e"] {
            assert!(validate_node(bon).is_ok(), "{bon} devrait passer");
        }
        for mauvais in [
            "",
            "   ",
            ".projects",
            "projects.",
            "projects..analyse",
            "projects/analyse",
            "../etc",
            "projects analyse",
        ] {
            assert!(validate_node(mauvais).is_err(), "{mauvais} devrait etre refuse");
        }
    }

    #[test]
    fn une_ecriture_vide_ou_enorme_est_refusee() {
        let vide = MemoryRequest::Write {
            node_id: "projects.x".into(),
            content: "   ".into(),
            tags: vec![],
            direct: false,
        };
        assert!(validate(&vide).is_err());
        let enorme = MemoryRequest::Write {
            node_id: "projects.x".into(),
            content: "x".repeat(MAX_CONTENT_BYTES + 1),
            tags: vec![],
            direct: false,
        };
        assert!(validate(&enorme).is_err());
    }

    /// Absent `direct` means proposed. The default is what happens when nobody
    /// thought about it, so it must be the reversible one.
    #[test]
    fn une_ecriture_passe_par_la_file_sauf_demande_explicite() {
        let requete: MemoryRequest = serde_json::from_str(
            r#"{"op":"write","nodeId":"projects.x","content":"un fait"}"#,
        )
        .unwrap();
        match requete {
            MemoryRequest::Write { direct, .. } => assert!(!direct),
            _ => panic!("mauvaise variante"),
        }
    }

    #[test]
    fn une_recherche_vide_est_refusee() {
        assert!(validate(&MemoryRequest::Search {
            query: "  ".into(),
            limit: None
        })
        .is_err());
        assert!(validate(&MemoryRequest::Search {
            query: "climat".into(),
            limit: Some(5)
        })
        .is_ok());
    }
}
