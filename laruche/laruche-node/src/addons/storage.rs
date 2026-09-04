use super::AddonSnapshot;
use crate::{auth_user, log_activite, AppState};
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Json;
use serde::Deserialize;
use serde_json::{json, Map, Value};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path as FsPath, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use uuid::Uuid;

const CAPABILITY: &str = "storage.private";
const MAX_KEYS: usize = 256;
const MAX_KEY_BYTES: usize = 128;
const MAX_VALUE_BYTES: usize = 64 * 1024;
const MAX_STORAGE_BYTES: usize = 1024 * 1024;
const MAX_JSON_DEPTH: usize = 20;

type ApiError = (StatusCode, Json<Value>);

#[derive(Debug, Deserialize)]
#[serde(tag = "op", rename_all = "camelCase", deny_unknown_fields)]
pub(crate) enum StorageRequest {
    Get {
        key: String,
    },
    Set {
        key: String,
        value: Value,
    },
    Delete {
        key: String,
    },
    List {
        #[serde(default)]
        prefix: String,
    },
}

#[derive(Debug)]
enum StorageError {
    Invalid(String),
    Quota(String),
    Io(String),
}

pub(crate) async fn handle(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(request): Json<StorageRequest>,
) -> Result<Json<Value>, ApiError> {
    let user_id = authenticated_user(&state, &headers).await?;
    let root = {
        let registry = state.addons.read().await;
        let addon = registry.get(&id).ok_or_else(|| {
            api_error(StatusCode::NOT_FOUND, "addon_not_found", "Addon not found")
        })?;
        authorize(&addon)?;
        registry.root().to_path_buf()
    };
    let operation = operation_name(&request);
    let addon_id = id.clone();
    let response = tokio::task::spawn_blocking(move || execute(&root, user_id, &addon_id, request))
        .await
        .map_err(|error| {
            tracing::warn!(error = %error, "addon private storage task failed");
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "storage_task_failed",
                "Private storage task failed",
            )
        })?
        .map_err(map_storage_error)?;

    if operation != "get" && operation != "list" {
        log_activite(
            &state,
            "info",
            "addons",
            format!("Addon {id} private storage {operation}"),
            Some(user_id),
        )
        .await;
    }
    Ok(Json(response))
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

fn authorize(addon: &AddonSnapshot) -> Result<(), ApiError> {
    if !addon.enabled {
        return Err(api_error(
            StatusCode::CONFLICT,
            "addon_disabled",
            "Addon is disabled",
        ));
    }
    let granted = addon.manifest.as_ref().is_some_and(|manifest| {
        manifest
            .permissions
            .required
            .iter()
            .any(|item| item == CAPABILITY)
    });
    if !granted {
        return Err(api_error(
            StatusCode::FORBIDDEN,
            "permission_denied",
            "Capability not granted",
        ));
    }
    Ok(())
}

fn operation_name(request: &StorageRequest) -> &'static str {
    match request {
        StorageRequest::Get { .. } => "get",
        StorageRequest::Set { .. } => "set",
        StorageRequest::Delete { .. } => "delete",
        StorageRequest::List { .. } => "list",
    }
}

fn execute(
    root: &FsPath,
    user_id: Uuid,
    addon_id: &str,
    request: StorageRequest,
) -> Result<Value, StorageError> {
    validate_request(&request)?;
    let _guard = storage_lock()
        .lock()
        .map_err(|_| StorageError::Io("private storage lock is poisoned".into()))?;
    let directory = storage_directory(root, user_id, addon_id)?;
    let target = directory.join("storage.json");
    let mut values = read_values(&target)?;
    match request {
        StorageRequest::Get { key } => {
            Ok(json!({"value": values.get(&key).cloned().unwrap_or(Value::Null)}))
        }
        StorageRequest::List { prefix } => {
            let keys: Vec<&String> = values
                .keys()
                .filter(|key| key.starts_with(&prefix))
                .collect();
            Ok(json!({"keys": keys}))
        }
        StorageRequest::Delete { key } => {
            values.remove(&key);
            persist_values(&target, &values)?;
            Ok(json!({}))
        }
        StorageRequest::Set { key, value } => {
            if !values.contains_key(&key) && values.len() >= MAX_KEYS {
                return Err(StorageError::Quota(format!(
                    "private storage cannot exceed {MAX_KEYS} keys"
                )));
            }
            values.insert(key, value);
            persist_values(&target, &values)?;
            Ok(json!({}))
        }
    }
}

fn validate_request(request: &StorageRequest) -> Result<(), StorageError> {
    match request {
        StorageRequest::Get { key } | StorageRequest::Delete { key } => validate_key(key),
        StorageRequest::Set { key, value } => {
            validate_key(key)?;
            validate_value(value)
        }
        StorageRequest::List { prefix } => validate_prefix(prefix),
    }
}

fn storage_directory(
    root: &FsPath,
    user_id: Uuid,
    addon_id: &str,
) -> Result<PathBuf, StorageError> {
    let data_root = root.join("data");
    fs::create_dir_all(&data_root).map_err(io_error)?;
    let canonical_root = fs::canonicalize(&data_root).map_err(io_error)?;
    let directory = data_root.join(user_id.to_string()).join(addon_id);
    fs::create_dir_all(&directory).map_err(io_error)?;
    let canonical_directory = fs::canonicalize(&directory).map_err(io_error)?;
    if !canonical_directory.starts_with(&canonical_root) {
        return Err(StorageError::Invalid(
            "private storage path is unsafe".into(),
        ));
    }
    Ok(canonical_directory)
}

fn read_values(path: &FsPath) -> Result<Map<String, Value>, StorageError> {
    recover_backup(path)?;
    if !path.exists() {
        return Ok(Map::new());
    }
    let metadata = fs::symlink_metadata(path).map_err(io_error)?;
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.len() > MAX_STORAGE_BYTES as u64
    {
        return Err(StorageError::Invalid(
            "private storage file is unsafe".into(),
        ));
    }
    let bytes = fs::read(path).map_err(io_error)?;
    let value: Value = serde_json::from_slice(&bytes)
        .map_err(|_| StorageError::Invalid("private storage file is invalid".into()))?;
    let values = value
        .as_object()
        .cloned()
        .ok_or_else(|| StorageError::Invalid("private storage root must be an object".into()))?;
    if values.len() > MAX_KEYS
        || values
            .iter()
            .any(|(key, value)| validate_key(key).is_err() || validate_value(value).is_err())
    {
        return Err(StorageError::Invalid(
            "private storage file violates its limits".into(),
        ));
    }
    Ok(values)
}

fn recover_backup(path: &FsPath) -> Result<(), StorageError> {
    let backup = path.with_extension("json.bak");
    if !path.exists() && backup.exists() {
        fs::rename(backup, path).map_err(io_error)?;
    }
    Ok(())
}

fn validate_key(key: &str) -> Result<(), StorageError> {
    if key.is_empty()
        || key.len() > MAX_KEY_BYTES
        || key.split('/').any(|segment| segment == "..")
        || !key
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._:/-".contains(&byte))
    {
        return Err(StorageError::Invalid(
            "private storage key is invalid".into(),
        ));
    }
    Ok(())
}

fn validate_prefix(prefix: &str) -> Result<(), StorageError> {
    if prefix.len() > MAX_KEY_BYTES
        || prefix.split('/').any(|segment| segment == "..")
        || !prefix
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._:/-".contains(&byte))
    {
        return Err(StorageError::Invalid(
            "private storage prefix is invalid".into(),
        ));
    }
    Ok(())
}

fn validate_value(value: &Value) -> Result<(), StorageError> {
    if json_depth(value, 0) > MAX_JSON_DEPTH {
        return Err(StorageError::Invalid(
            "private storage value is too deep".into(),
        ));
    }
    let bytes =
        serde_json::to_vec(value).map_err(|error| StorageError::Invalid(error.to_string()))?;
    if bytes.len() > MAX_VALUE_BYTES {
        return Err(StorageError::Quota(
            "private storage value exceeds 64 KiB".into(),
        ));
    }
    Ok(())
}

fn json_depth(value: &Value, depth: usize) -> usize {
    match value {
        Value::Array(values) => values
            .iter()
            .map(|value| json_depth(value, depth + 1))
            .max()
            .unwrap_or(depth),
        Value::Object(values) => values
            .values()
            .map(|value| json_depth(value, depth + 1))
            .max()
            .unwrap_or(depth),
        _ => depth,
    }
}

fn persist_values(path: &FsPath, values: &Map<String, Value>) -> Result<(), StorageError> {
    let bytes =
        serde_json::to_vec(values).map_err(|error| StorageError::Invalid(error.to_string()))?;
    if bytes.len() > MAX_STORAGE_BYTES {
        return Err(StorageError::Quota("private storage exceeds 1 MiB".into()));
    }
    let temporary = path.with_extension(format!("tmp-{}", Uuid::new_v4()));
    let backup = path.with_extension("json.bak");
    let write_result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(io_error)?;
        file.write_all(&bytes).map_err(io_error)?;
        file.sync_all().map_err(io_error)
    })();
    if let Err(error) = write_result {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }

    if backup.exists() {
        fs::remove_file(&backup).map_err(io_error)?;
    }
    if path.exists() {
        fs::rename(path, &backup).map_err(io_error)?;
    }
    if let Err(error) = fs::rename(&temporary, path) {
        if backup.exists() && !path.exists() {
            let _ = fs::rename(&backup, path);
        }
        let _ = fs::remove_file(&temporary);
        return Err(io_error(error));
    }
    if backup.exists() {
        if let Err(error) = fs::remove_file(backup) {
            tracing::warn!(error = %error, "addon private storage backup cleanup failed");
        }
    }
    Ok(())
}

fn storage_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn io_error(error: std::io::Error) -> StorageError {
    StorageError::Io(error.to_string())
}

fn map_storage_error(error: StorageError) -> ApiError {
    match error {
        StorageError::Invalid(message) => {
            api_error(StatusCode::BAD_REQUEST, "validation_failed", &message)
        }
        StorageError::Quota(message) => {
            api_error(StatusCode::PAYLOAD_TOO_LARGE, "quota_exceeded", &message)
        }
        StorageError::Io(message) => {
            tracing::warn!(error = %message, "addon private storage failed");
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "storage_unavailable",
                "Private storage is unavailable",
            )
        }
    }
}

fn api_error(status: StatusCode, code: &str, message: &str) -> ApiError {
    (
        status,
        Json(json!({"error": {"code": code, "message": message}})),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> PathBuf {
        std::env::temp_dir().join(format!("laruche-addon-storage-{}", Uuid::new_v4()))
    }

    fn request(root: &FsPath, user: Uuid, op: StorageRequest) -> Value {
        execute(root, user, "dev.laruche.test", op).unwrap()
    }

    #[test]
    fn values_persist_and_are_isolated_by_user() {
        let root = root();
        let alice = Uuid::new_v4();
        let bob = Uuid::new_v4();
        request(
            &root,
            alice,
            StorageRequest::Set {
                key: "game.board".into(),
                value: json!([2, 4, 8]),
            },
        );
        assert_eq!(
            request(
                &root,
                alice,
                StorageRequest::Get {
                    key: "game.board".into()
                }
            )["value"],
            json!([2, 4, 8])
        );
        assert_eq!(
            request(
                &root,
                bob,
                StorageRequest::Get {
                    key: "game.board".into()
                }
            )["value"],
            Value::Null
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn values_are_isolated_by_addon() {
        let root = root();
        let user = Uuid::new_v4();
        execute(
            &root,
            user,
            "dev.laruche.first",
            StorageRequest::Set {
                key: "save".into(),
                value: json!("first"),
            },
        )
        .unwrap();
        assert_eq!(
            execute(
                &root,
                user,
                "dev.laruche.second",
                StorageRequest::Get { key: "save".into() }
            )
            .unwrap()["value"],
            Value::Null
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn lists_by_prefix_and_deletes_values() {
        let root = root();
        let user = Uuid::new_v4();
        for key in ["game.board", "game.score", "preferences.theme"] {
            request(
                &root,
                user,
                StorageRequest::Set {
                    key: key.into(),
                    value: json!(key),
                },
            );
        }
        assert_eq!(
            request(
                &root,
                user,
                StorageRequest::List {
                    prefix: "game.".into()
                }
            )["keys"],
            json!(["game.board", "game.score"])
        );
        request(
            &root,
            user,
            StorageRequest::Delete {
                key: "game.board".into(),
            },
        );
        assert_eq!(
            request(
                &root,
                user,
                StorageRequest::Get {
                    key: "game.board".into()
                }
            )["value"],
            Value::Null
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn recovers_the_previous_file_after_an_interrupted_replace() {
        let root = root();
        let user = Uuid::new_v4();
        request(
            &root,
            user,
            StorageRequest::Set {
                key: "save".into(),
                value: json!(42),
            },
        );
        let directory = storage_directory(&root, user, "dev.laruche.test").unwrap();
        let target = directory.join("storage.json");
        fs::rename(&target, target.with_extension("json.bak")).unwrap();
        assert_eq!(
            request(&root, user, StorageRequest::Get { key: "save".into() })["value"],
            json!(42)
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_oversized_values_without_replacing_the_previous_one() {
        let root = root();
        let user = Uuid::new_v4();
        request(
            &root,
            user,
            StorageRequest::Set {
                key: "save".into(),
                value: json!("small"),
            },
        );
        let result = execute(
            &root,
            user,
            "dev.laruche.test",
            StorageRequest::Set {
                key: "save".into(),
                value: json!("x".repeat(MAX_VALUE_BYTES)),
            },
        );
        assert!(matches!(result, Err(StorageError::Quota(_))));
        assert_eq!(
            request(&root, user, StorageRequest::Get { key: "save".into() })["value"],
            json!("small")
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_unsafe_keys() {
        let root = root();
        let result = execute(
            &root,
            Uuid::new_v4(),
            "dev.laruche.test",
            StorageRequest::Get {
                key: "../secret".into(),
            },
        );
        assert!(matches!(result, Err(StorageError::Invalid(_))));
        let _ = fs::remove_dir_all(root);
    }
}
