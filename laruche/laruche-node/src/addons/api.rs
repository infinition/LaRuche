use super::installer::{self, InstallError};
use super::permissions::{self, GrantError};
use super::{AddonDiagnostic, AddonSnapshot, RegistryMutationError};
use crate::{auth_user, log_activite, AppState};
use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Json;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AddonListResponse {
    addons: Vec<AddonSnapshot>,
    diagnostics: Vec<AddonDiagnostic>,
}

type ApiError = (StatusCode, Json<serde_json::Value>);

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct EnableRequest {
    #[serde(default)]
    granted_permissions: Vec<String>,
}

pub(crate) async fn permission_catalog() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "version": permissions::CATALOG_VERSION,
        "permissions": permissions::catalog()
    }))
}

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
    body: Option<Json<EnableRequest>>,
) -> Result<Json<AddonSnapshot>, ApiError> {
    set_enabled(
        state,
        headers,
        id,
        true,
        body.map(|Json(body)| body.granted_permissions),
    )
    .await
}

pub(crate) async fn disable(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<AddonSnapshot>, ApiError> {
    set_enabled(state, headers, id, false, None).await
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

pub(crate) async fn install(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<AddonSnapshot>, ApiError> {
    require_admin_or_fresh(&state, &headers).await?;
    let root = state.addons.read().await.root().to_path_buf();
    let installed = tokio::task::spawn_blocking(move || installer::install(&root, body.to_vec()))
        .await
        .map_err(|error| {
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "addon_install_task_failed",
                &error.to_string(),
            )
        })?
        .map_err(map_install_error)?;
    let id = installed.manifest.id.clone();
    let version = installed.manifest.version.clone();
    let path = installed.path.clone();
    let snapshot = match state
        .addons
        .write()
        .await
        .adopt_installed(installed.manifest)
    {
        Ok(snapshot) => snapshot,
        Err(error) => {
            tokio::task::spawn_blocking(move || installer::rollback(&path))
                .await
                .ok();
            return Err(map_mutation_error(error));
        }
    };
    log_activite(
        &state,
        "info",
        "addons",
        format!("Addon {id} version {version} installed (disabled)"),
        auth_user::extract_user_from_headers(&headers, &state.cookie_secret),
    )
    .await;
    Ok(Json(snapshot))
}

async fn set_enabled(
    state: Arc<AppState>,
    headers: HeaderMap,
    id: String,
    enabled: bool,
    requested_permissions: Option<Vec<String>>,
) -> Result<Json<AddonSnapshot>, ApiError> {
    require_admin_or_fresh(&state, &headers).await?;
    let snapshot = {
        let mut registry = state.addons.write().await;
        let granted_permissions = if enabled {
            let current = registry.get(&id).ok_or_else(|| {
                api_error(StatusCode::NOT_FOUND, "addon_not_found", "Addon not found")
            })?;
            let manifest = current.manifest.as_ref().ok_or_else(|| {
                api_error(
                    StatusCode::CONFLICT,
                    "addon_broken",
                    "Addon package is invalid",
                )
            })?;
            let requested =
                requested_permissions.unwrap_or_else(|| manifest.permissions.required.clone());
            permissions::validate_grants(manifest, requested).map_err(map_grant_error)?
        } else {
            Vec::new()
        };
        registry
            .set_enabled(&id, enabled, granted_permissions)
            .map_err(map_mutation_error)?
    };
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

fn map_grant_error(error: GrantError) -> ApiError {
    match error {
        GrantError::RequiredUnavailable(permission) => api_error(
            StatusCode::CONFLICT,
            "required_permission_unavailable",
            &format!("Required permission is not available: {permission}"),
        ),
        GrantError::MissingRequired(permission) => api_error(
            StatusCode::BAD_REQUEST,
            "required_permission_missing",
            &format!("Required permission was not granted: {permission}"),
        ),
        GrantError::Undeclared(permission) => api_error(
            StatusCode::BAD_REQUEST,
            "permission_not_declared",
            &format!("Permission is not declared by the addon: {permission}"),
        ),
        GrantError::Unavailable(permission) => api_error(
            StatusCode::CONFLICT,
            "permission_unavailable",
            &format!("Permission is not available: {permission}"),
        ),
    }
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

fn map_install_error(error: InstallError) -> ApiError {
    match error {
        InstallError::Invalid(message) => {
            api_error(StatusCode::BAD_REQUEST, "addon_package_invalid", &message)
        }
        InstallError::TooLarge(message) => api_error(
            StatusCode::PAYLOAD_TOO_LARGE,
            "addon_package_too_large",
            &message,
        ),
        InstallError::AlreadyInstalled => api_error(
            StatusCode::CONFLICT,
            "addon_version_exists",
            "This addon version is already installed",
        ),
        InstallError::Io(message) => api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "addon_install_io_failed",
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
