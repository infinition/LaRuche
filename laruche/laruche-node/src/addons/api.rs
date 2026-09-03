use super::{AddonDiagnostic, AddonSnapshot, RegistryMutationError};
use crate::{auth_user, log_activite, AppState};
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Json;
use serde::Serialize;
use std::sync::Arc;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AddonListResponse {
    addons: Vec<AddonSnapshot>,
    diagnostics: Vec<AddonDiagnostic>,
}

type ApiError = (StatusCode, Json<serde_json::Value>);

pub(crate) async fn list(State(state): State<Arc<AppState>>) -> Json<AddonListResponse> {
    let registry = state.addons.read().await;
    Json(AddonListResponse {
        addons: registry.list(),
        diagnostics: registry.diagnostics().to_vec(),
    })
}

pub(crate) async fn get_one(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<AddonSnapshot>, ApiError> {
    state
        .addons
        .read()
        .await
        .get(&id)
        .map(Json)
        .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "addon_not_found", "Addon not found"))
}

pub(crate) async fn enable(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<AddonSnapshot>, ApiError> {
    set_enabled(state, headers, id, true).await
}

pub(crate) async fn disable(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<AddonSnapshot>, ApiError> {
    set_enabled(state, headers, id, false).await
}

pub(crate) async fn rescan(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<AddonListResponse>, ApiError> {
    require_admin_or_fresh(&state, &headers).await?;
    let response = {
        let mut registry = state.addons.write().await;
        registry.rescan().map_err(|error| {
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "registry_persistence_failed",
                &error,
            )
        })?;
        AddonListResponse {
            addons: registry.list(),
            diagnostics: registry.diagnostics().to_vec(),
        }
    };
    log_activite(
        &state,
        "info",
        "addons",
        "Addon packages rescanned".into(),
        auth_user::extract_user_from_headers(&headers, &state.cookie_secret),
    )
    .await;
    Ok(Json(response))
}

async fn set_enabled(
    state: Arc<AppState>,
    headers: HeaderMap,
    id: String,
    enabled: bool,
) -> Result<Json<AddonSnapshot>, ApiError> {
    require_admin_or_fresh(&state, &headers).await?;
    let snapshot = state
        .addons
        .write()
        .await
        .set_enabled(&id, enabled)
        .map_err(map_mutation_error)?;
    let actor = auth_user::extract_user_from_headers(&headers, &state.cookie_secret);
    log_activite(
        &state,
        "info",
        "addons",
        format!(
            "Addon {id} {}",
            if enabled { "enabled" } else { "disabled" }
        ),
        actor,
    )
    .await;
    Ok(Json(snapshot))
}

async fn require_admin_or_fresh(state: &AppState, headers: &HeaderMap) -> Result<(), ApiError> {
    let users = state.users.read().await;
    if users.is_empty() || auth_user::check_admin(headers, &state.cookie_secret, &users).1 {
        return Ok(());
    }
    Err(api_error(
        StatusCode::FORBIDDEN,
        "admin_required",
        "Administrator access required",
    ))
}

fn map_mutation_error(error: RegistryMutationError) -> ApiError {
    match error {
        RegistryMutationError::NotFound => {
            api_error(StatusCode::NOT_FOUND, "addon_not_found", "Addon not found")
        }
        RegistryMutationError::Broken(message) => {
            api_error(StatusCode::CONFLICT, "addon_broken", &message)
        }
        RegistryMutationError::Persistence(message) => api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "registry_persistence_failed",
            &message,
        ),
    }
}

fn api_error(status: StatusCode, code: &str, message: &str) -> ApiError {
    (
        status,
        Json(serde_json::json!({
            "error": {
                "code": code,
                "message": message
            }
        })),
    )
}
