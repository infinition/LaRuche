//! A real file tree an App may read and write, confined to one directory.
//!
//! `storage.rs` next door keeps a JSON map, which is the right shape for a
//! setting or a saved board and the wrong one for a notebook: the Obsidian
//! plugin this App reproduces reads and writes actual files, and a data science
//! session produces CSVs, JSON exports and notes that outlive the App.
//!
//! Everything lives under `apps/files/<user>/<app>/`. That root is created here,
//! canonicalized, and every resolved path is checked to still sit beneath it
//! after resolution: a relative path is not trusted for being written without a
//! `..`, it is trusted for landing where it should. Symlinks are refused rather
//! than followed, since a link planted in the tree is the one way a confined
//! path reaches outside it.
use super::AppSnapshot;
use crate::{auth_user, log_activite, AppState};
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use std::fs;
use std::path::{Component, Path as FsPath, PathBuf};
use std::sync::Arc;
use uuid::Uuid;

const CAPABILITY: &str = "laruche.files";
/// One file. Large enough for a year of daily rows, small enough that a runaway
/// loop fills the quota instead of the disk.
const MAX_FILE_BYTES: usize = 2 * 1024 * 1024;
/// The whole tree, per user and per App.
const MAX_TREE_BYTES: u64 = 32 * 1024 * 1024;
const MAX_ENTRIES: usize = 2_000;
const MAX_PATH_BYTES: usize = 512;
const MAX_DEPTH: usize = 12;

type ApiError = (StatusCode, Json<Value>);

#[derive(Debug, Deserialize)]
#[serde(tag = "op", rename_all = "camelCase", deny_unknown_fields)]
pub(crate) enum FileRequest {
    Read {
        path: String,
    },
    Write {
        path: String,
        content: String,
        #[serde(default)]
        append: bool,
    },
    Delete {
        path: String,
    },
    Exists {
        path: String,
    },
    List {
        #[serde(default)]
        path: String,
    },
    Mkdir {
        path: String,
    },
}

#[derive(Debug)]
enum FileError {
    Invalid(String),
    NotFound,
    TooLarge(String),
    Io(String),
}

pub(crate) async fn handle(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(request): Json<FileRequest>,
) -> Result<Json<Value>, ApiError> {
    let user_id = authenticated_user(&state, &headers).await?;
    let root = {
        let registry = state.apps.read().await;
        let app = registry
            .get(&id)
            .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "app_not_found", "App not found"))?;
        authorize(&app)?;
        registry.root().to_path_buf()
    };
    let operation = operation_name(&request);
    let app_id = id.clone();
    let response = tokio::task::spawn_blocking(move || execute(&root, user_id, &app_id, request))
        .await
        .map_err(|error| {
            tracing::warn!(error = %error, "app file task failed");
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "file_task_failed",
                "File task failed",
            )
        })?
        .map_err(map_error)?;

    if !matches!(operation, "read" | "list" | "exists") {
        log_activite(
            &state,
            "info",
            "apps",
            format!("App {id} files {operation}"),
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

fn operation_name(request: &FileRequest) -> &'static str {
    match request {
        FileRequest::Read { .. } => "read",
        FileRequest::Write { .. } => "write",
        FileRequest::Delete { .. } => "delete",
        FileRequest::Exists { .. } => "exists",
        FileRequest::List { .. } => "list",
        FileRequest::Mkdir { .. } => "mkdir",
    }
}

/// The App's own directory, created and canonicalized.
fn tree_root(root: &FsPath, user_id: Uuid, app_id: &str) -> Result<PathBuf, FileError> {
    let base = root.join("files");
    fs::create_dir_all(&base).map_err(io_error)?;
    let canonical_base = fs::canonicalize(&base).map_err(io_error)?;
    let directory = base.join(user_id.to_string()).join(app_id);
    fs::create_dir_all(&directory).map_err(io_error)?;
    let canonical = fs::canonicalize(&directory).map_err(io_error)?;
    if !canonical.starts_with(&canonical_base) {
        return Err(FileError::Invalid("file root is unsafe".into()));
    }
    Ok(canonical)
}

/// A relative path with no surprises, checked before it touches the disk.
///
/// The syntactic pass rejects what should never be written at all: an absolute
/// path, a drive letter, a parent hop, a control character. It is not enough on
/// its own, which is why every caller also verifies the resolved path is still
/// under the root, but it is what keeps a hostile name from reaching the OS.
fn safe_relative(path: &str) -> Result<PathBuf, FileError> {
    let trimmed = path.trim().replace('\\', "/");
    if trimmed.is_empty() || trimmed.len() > MAX_PATH_BYTES {
        return Err(FileError::Invalid("path is empty or too long".into()));
    }
    if trimmed.chars().any(|c| c.is_control()) {
        return Err(FileError::Invalid("path has a control character".into()));
    }
    let candidate = PathBuf::from(&trimmed);
    let mut depth = 0usize;
    let mut clean = PathBuf::new();
    for component in candidate.components() {
        match component {
            Component::Normal(part) => {
                let name = part.to_str().ok_or_else(|| {
                    FileError::Invalid("path is not valid text".into())
                })?;
                if name == "." || name.is_empty() || name.len() > 128 {
                    return Err(FileError::Invalid("path segment is invalid".into()));
                }
                depth += 1;
                if depth > MAX_DEPTH {
                    return Err(FileError::Invalid("path is too deep".into()));
                }
                clean.push(name);
            }
            _ => {
                return Err(FileError::Invalid(
                    "path must be relative, with no parent or root component".into(),
                ))
            }
        }
    }
    if clean.as_os_str().is_empty() {
        return Err(FileError::Invalid("path resolves to nothing".into()));
    }
    Ok(clean)
}

/// Resolve inside the tree, refusing links and anything that escapes.
///
/// `must_exist` separates reading from writing: a write resolves the parent,
/// because the file itself is allowed not to be there yet.
fn resolve(root: &FsPath, relative: &FsPath, must_exist: bool) -> Result<PathBuf, FileError> {
    let target = root.join(relative);
    if must_exist {
        let metadata = fs::symlink_metadata(&target).map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                FileError::NotFound
            } else {
                io_error(error)
            }
        })?;
        if metadata.file_type().is_symlink() {
            return Err(FileError::Invalid("symlinks are not followed".into()));
        }
        let canonical = fs::canonicalize(&target).map_err(io_error)?;
        if !canonical.starts_with(root) {
            return Err(FileError::Invalid("path escapes the app directory".into()));
        }
        return Ok(canonical);
    }
    let parent = target
        .parent()
        .ok_or_else(|| FileError::Invalid("path has no parent".into()))?;
    fs::create_dir_all(parent).map_err(io_error)?;
    let canonical_parent = fs::canonicalize(parent).map_err(io_error)?;
    if !canonical_parent.starts_with(root) {
        return Err(FileError::Invalid("path escapes the app directory".into()));
    }
    let name = target
        .file_name()
        .ok_or_else(|| FileError::Invalid("path has no file name".into()))?;
    // An existing entry at the destination must not be a link either: writing
    // through one would put the bytes wherever it points.
    if let Ok(metadata) = fs::symlink_metadata(canonical_parent.join(name)) {
        if metadata.file_type().is_symlink() {
            return Err(FileError::Invalid("symlinks are not followed".into()));
        }
    }
    Ok(canonical_parent.join(name))
}

fn tree_size(root: &FsPath) -> u64 {
    fn walk(directory: &FsPath, total: &mut u64, depth: usize) {
        if depth > MAX_DEPTH {
            return;
        }
        let Ok(entries) = fs::read_dir(directory) else {
            return;
        };
        for entry in entries.flatten() {
            let Ok(metadata) = entry.metadata() else {
                continue;
            };
            if metadata.is_dir() {
                walk(&entry.path(), total, depth + 1);
            } else {
                *total += metadata.len();
            }
        }
    }
    let mut total = 0;
    walk(root, &mut total, 0);
    total
}

fn execute(
    root: &FsPath,
    user_id: Uuid,
    app_id: &str,
    request: FileRequest,
) -> Result<Value, FileError> {
    let tree = tree_root(root, user_id, app_id)?;
    match request {
        FileRequest::Read { path } => {
            let relative = safe_relative(&path)?;
            let target = resolve(&tree, &relative, true)?;
            let metadata = fs::symlink_metadata(&target).map_err(io_error)?;
            if !metadata.is_file() {
                return Err(FileError::Invalid("not a file".into()));
            }
            if metadata.len() > MAX_FILE_BYTES as u64 {
                return Err(FileError::TooLarge(format!(
                    "{path} is {} bytes, over the {MAX_FILE_BYTES} limit",
                    metadata.len()
                )));
            }
            let bytes = fs::read(&target).map_err(io_error)?;
            let content = String::from_utf8(bytes).map_err(|_| {
                FileError::Invalid("file is not UTF-8 text".into())
            })?;
            Ok(json!({ "path": path, "content": content, "bytes": metadata.len() }))
        }
        FileRequest::Write {
            path,
            content,
            append,
        } => {
            if content.len() > MAX_FILE_BYTES {
                return Err(FileError::TooLarge(format!(
                    "content is {} bytes, over the {MAX_FILE_BYTES} limit",
                    content.len()
                )));
            }
            let relative = safe_relative(&path)?;
            let target = resolve(&tree, &relative, false)?;
            let existing = fs::symlink_metadata(&target).map(|m| m.len()).unwrap_or(0);
            let ajout = content.len() as u64;
            let total = tree_size(&tree).saturating_sub(if append { 0 } else { existing }) + ajout;
            if total > MAX_TREE_BYTES {
                return Err(FileError::TooLarge(format!(
                    "the app file tree would reach {total} bytes, over the {MAX_TREE_BYTES} limit"
                )));
            }
            if append {
                use std::io::Write as _;
                let mut file = fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&target)
                    .map_err(io_error)?;
                file.write_all(content.as_bytes()).map_err(io_error)?;
            } else {
                fs::write(&target, content.as_bytes()).map_err(io_error)?;
            }
            let bytes = fs::symlink_metadata(&target).map(|m| m.len()).unwrap_or(0);
            Ok(json!({ "path": path, "bytes": bytes, "written": true }))
        }
        FileRequest::Delete { path } => {
            let relative = safe_relative(&path)?;
            let target = match resolve(&tree, &relative, true) {
                Ok(target) => target,
                Err(FileError::NotFound) => {
                    return Ok(json!({ "path": path, "deleted": false }))
                }
                Err(error) => return Err(error),
            };
            let metadata = fs::symlink_metadata(&target).map_err(io_error)?;
            if metadata.is_dir() {
                // Only an empty directory. Removing a tree on one call is how a
                // wrong path becomes an unrecoverable afternoon.
                fs::remove_dir(&target).map_err(io_error)?;
            } else {
                fs::remove_file(&target).map_err(io_error)?;
            }
            Ok(json!({ "path": path, "deleted": true }))
        }
        FileRequest::Exists { path } => {
            let relative = safe_relative(&path)?;
            match resolve(&tree, &relative, true) {
                Ok(target) => {
                    let metadata = fs::symlink_metadata(&target).map_err(io_error)?;
                    Ok(json!({
                        "path": path,
                        "exists": true,
                        "directory": metadata.is_dir(),
                        "bytes": metadata.len()
                    }))
                }
                Err(FileError::NotFound) => Ok(json!({ "path": path, "exists": false })),
                Err(error) => Err(error),
            }
        }
        FileRequest::List { path } => {
            let directory = if path.trim().is_empty() {
                tree.clone()
            } else {
                let relative = safe_relative(&path)?;
                resolve(&tree, &relative, true)?
            };
            let metadata = fs::symlink_metadata(&directory).map_err(io_error)?;
            if !metadata.is_dir() {
                return Err(FileError::Invalid("not a directory".into()));
            }
            let mut entries = Vec::new();
            for entry in fs::read_dir(&directory).map_err(io_error)?.flatten() {
                if entries.len() >= MAX_ENTRIES {
                    break;
                }
                let Ok(meta) = entry.metadata() else { continue };
                let name = entry.file_name().to_string_lossy().to_string();
                entries.push(json!({
                    "name": name,
                    "directory": meta.is_dir(),
                    "bytes": if meta.is_dir() { 0 } else { meta.len() }
                }));
            }
            entries.sort_by(|a, b| a["name"].as_str().cmp(&b["name"].as_str()));
            Ok(json!({
                "path": path,
                "entries": entries,
                "treeBytes": tree_size(&tree),
                "treeLimit": MAX_TREE_BYTES
            }))
        }
        FileRequest::Mkdir { path } => {
            let relative = safe_relative(&path)?;
            let target = tree.join(&relative);
            fs::create_dir_all(&target).map_err(io_error)?;
            let canonical = fs::canonicalize(&target).map_err(io_error)?;
            if !canonical.starts_with(&tree) {
                return Err(FileError::Invalid("path escapes the app directory".into()));
            }
            Ok(json!({ "path": path, "created": true }))
        }
    }
}

fn io_error(error: std::io::Error) -> FileError {
    FileError::Io(error.to_string())
}

fn map_error(error: FileError) -> ApiError {
    match error {
        FileError::Invalid(message) => {
            api_error(StatusCode::BAD_REQUEST, "validation_failed", &message)
        }
        FileError::NotFound => api_error(StatusCode::NOT_FOUND, "not_found", "File not found"),
        FileError::TooLarge(message) => {
            api_error(StatusCode::PAYLOAD_TOO_LARGE, "quota_exceeded", &message)
        }
        FileError::Io(message) => {
            tracing::warn!(error = %message, "app file operation failed");
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "file_unavailable",
                "File operation failed",
            )
        }
    }
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

    fn tmp() -> PathBuf {
        let base = std::env::temp_dir().join(format!("laruche-files-{}", Uuid::new_v4()));
        fs::create_dir_all(&base).unwrap();
        base
    }

    #[test]
    fn un_chemin_qui_sort_de_l_arbre_est_refuse() {
        for mauvais in [
            "../escape.txt",
            "a/../../escape.txt",
            "/etc/passwd",
            "C:/Windows/system32/x.txt",
            "",
            "   ",
        ] {
            assert!(
                safe_relative(mauvais).is_err(),
                "{mauvais} aurait du etre refuse"
            );
        }
    }

    #[test]
    fn un_chemin_ordinaire_est_accepte_et_normalise() {
        assert_eq!(
            safe_relative("DS_Studio\\datasets\\sales.csv").unwrap(),
            PathBuf::from("DS_Studio/datasets/sales.csv")
        );
        assert_eq!(
            safe_relative("notes.md").unwrap(),
            PathBuf::from("notes.md")
        );
    }

    #[test]
    fn ecrire_puis_relire_conserve_le_contenu() {
        let root = tmp();
        let user = Uuid::new_v4();
        let ecrire = FileRequest::Write {
            path: "DS_Studio/notes.md".into(),
            content: "# Titre\nligne".into(),
            append: false,
        };
        execute(&root, user, "dev.laruche.test", ecrire).unwrap();
        let lu = execute(
            &root,
            user,
            "dev.laruche.test",
            FileRequest::Read {
                path: "DS_Studio/notes.md".into(),
            },
        )
        .unwrap();
        assert_eq!(lu["content"], "# Titre\nligne");
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn deux_apps_ne_se_voient_pas() {
        let root = tmp();
        let user = Uuid::new_v4();
        execute(
            &root,
            user,
            "dev.laruche.a",
            FileRequest::Write {
                path: "secret.txt".into(),
                content: "a".into(),
                append: false,
            },
        )
        .unwrap();
        let vue = execute(
            &root,
            user,
            "dev.laruche.b",
            FileRequest::Exists {
                path: "secret.txt".into(),
            },
        )
        .unwrap();
        assert_eq!(vue["exists"], false);
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn deux_comptes_ne_se_voient_pas() {
        let root = tmp();
        execute(
            &root,
            Uuid::new_v4(),
            "dev.laruche.test",
            FileRequest::Write {
                path: "prive.txt".into(),
                content: "x".into(),
                append: false,
            },
        )
        .unwrap();
        let vue = execute(
            &root,
            Uuid::new_v4(),
            "dev.laruche.test",
            FileRequest::Exists {
                path: "prive.txt".into(),
            },
        )
        .unwrap();
        assert_eq!(vue["exists"], false);
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn un_fichier_trop_gros_est_refuse_avant_d_etre_ecrit() {
        let root = tmp();
        let user = Uuid::new_v4();
        let erreur = execute(
            &root,
            user,
            "dev.laruche.test",
            FileRequest::Write {
                path: "gros.csv".into(),
                content: "x".repeat(MAX_FILE_BYTES + 1),
                append: false,
            },
        );
        assert!(matches!(erreur, Err(FileError::TooLarge(_))));
        let vu = execute(
            &root,
            user,
            "dev.laruche.test",
            FileRequest::Exists {
                path: "gros.csv".into(),
            },
        )
        .unwrap();
        assert_eq!(vu["exists"], false, "rien ne doit avoir ete ecrit");
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn supprimer_ce_qui_n_existe_pas_ne_lance_pas_d_erreur() {
        let root = tmp();
        let vue = execute(
            &root,
            Uuid::new_v4(),
            "dev.laruche.test",
            FileRequest::Delete {
                path: "absent.txt".into(),
            },
        )
        .unwrap();
        assert_eq!(vue["deleted"], false);
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn lister_rend_les_entrees_et_la_taille_de_l_arbre() {
        let root = tmp();
        let user = Uuid::new_v4();
        for nom in ["a.csv", "b.csv"] {
            execute(
                &root,
                user,
                "dev.laruche.test",
                FileRequest::Write {
                    path: nom.into(),
                    content: "12345".into(),
                    append: false,
                },
            )
            .unwrap();
        }
        let vue = execute(
            &root,
            user,
            "dev.laruche.test",
            FileRequest::List { path: String::new() },
        )
        .unwrap();
        assert_eq!(vue["entries"].as_array().unwrap().len(), 2);
        assert_eq!(vue["treeBytes"], 10);
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn ajouter_ecrit_a_la_suite() {
        let root = tmp();
        let user = Uuid::new_v4();
        for morceau in ["une", " deux"] {
            execute(
                &root,
                user,
                "dev.laruche.test",
                FileRequest::Write {
                    path: "journal.txt".into(),
                    content: morceau.into(),
                    append: true,
                },
            )
            .unwrap();
        }
        let lu = execute(
            &root,
            user,
            "dev.laruche.test",
            FileRequest::Read {
                path: "journal.txt".into(),
            },
        )
        .unwrap();
        assert_eq!(lu["content"], "une deux");
        fs::remove_dir_all(&root).ok();
    }
}
