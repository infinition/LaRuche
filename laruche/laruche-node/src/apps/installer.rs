use super::registry::{load_manifest, validate_referenced_files};
use super::AppManifest;
use std::collections::HashSet;
use std::fs::{self, OpenOptions};
use std::io::{Cursor, Read, Write};
use std::path::{Component, Path, PathBuf};
use uuid::Uuid;
use zip::ZipArchive;

const MAX_ARCHIVE_BYTES: usize = 32 * 1024 * 1024;
const MAX_FILES: usize = 4_096;
const MAX_FILE_BYTES: u64 = 128 * 1024 * 1024;
const MAX_EXPANDED_BYTES: u64 = 256 * 1024 * 1024;

#[derive(Debug)]
pub(crate) enum InstallError {
    Invalid(String),
    TooLarge(String),
    AlreadyInstalled,
    Io(String),
}

pub(crate) struct InstalledPackage {
    pub(crate) manifest: AppManifest,
    pub(crate) path: PathBuf,
}

pub(crate) fn install(root: &Path, archive: Vec<u8>) -> Result<InstalledPackage, InstallError> {
    if archive.is_empty() {
        return Err(InstallError::Invalid("the package is empty".into()));
    }
    if archive.len() > MAX_ARCHIVE_BYTES {
        return Err(InstallError::TooLarge(
            "compressed package exceeds 32 MiB".into(),
        ));
    }

    let staging_root = root.join(".staging");
    fs::create_dir_all(&staging_root).map_err(io_error)?;
    let staging = staging_root.join(Uuid::new_v4().to_string());
    fs::create_dir(&staging).map_err(io_error)?;
    let mut guard = StagingGuard::new(staging.clone());
    extract_archive(&archive, &staging)?;

    let canonical_staging_root = fs::canonicalize(&staging_root).map_err(io_error)?;
    let canonical_manifest = staging.join("app.json");
    let legacy_manifest = staging.join("addon.json");
    let manifest_path = if canonical_manifest.exists() {
        canonical_manifest
    } else {
        legacy_manifest
    };
    let manifest = load_manifest(&manifest_path, &canonical_staging_root)
        .map_err(InstallError::Invalid)?;
    validate_referenced_files(&manifest, &staging, &canonical_staging_root)
        .map_err(InstallError::Invalid)?;

    let target_parent = root.join("packages").join(&manifest.id);
    fs::create_dir_all(&target_parent).map_err(io_error)?;
    let target = target_parent.join(&manifest.version);
    if target.exists() {
        return Err(InstallError::AlreadyInstalled);
    }
    fs::rename(&staging, &target).map_err(io_error)?;
    guard.disarm();
    Ok(InstalledPackage {
        manifest,
        path: target,
    })
}

pub(crate) fn rollback(path: &Path) {
    let _ = fs::remove_dir_all(path);
}

fn extract_archive(bytes: &[u8], destination: &Path) -> Result<(), InstallError> {
    let mut archive = ZipArchive::new(Cursor::new(bytes))
        .map_err(|error| InstallError::Invalid(format!("invalid ZIP package: {error}")))?;
    if archive.len() == 0 || archive.len() > MAX_FILES {
        return Err(InstallError::TooLarge(format!(
            "package must contain between 1 and {MAX_FILES} entries"
        )));
    }

    let mut expanded = 0u64;
    let mut paths = HashSet::new();
    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|error| InstallError::Invalid(format!("invalid ZIP entry: {error}")))?;
        let raw = entry.name().to_string();
        validate_entry_path(&raw)?;
        let relative = entry
            .enclosed_name()
            .ok_or_else(|| InstallError::Invalid(format!("unsafe ZIP path: {raw}")))?
            .to_path_buf();
        let identity = raw.trim_end_matches('/').to_ascii_lowercase();
        if !paths.insert(identity) {
            return Err(InstallError::Invalid(format!("duplicate ZIP path: {raw}")));
        }
        if entry
            .unix_mode()
            .is_some_and(|mode| mode & 0o170000 == 0o120000)
        {
            return Err(InstallError::Invalid(format!(
                "symbolic links are forbidden: {raw}"
            )));
        }
        if entry.is_dir() {
            fs::create_dir_all(destination.join(relative)).map_err(io_error)?;
            continue;
        }
        if entry.size() > MAX_FILE_BYTES {
            return Err(InstallError::TooLarge(format!(
                "file exceeds 128 MiB: {raw}"
            )));
        }
        if expanded
            .checked_add(entry.size())
            .is_none_or(|declared| declared > MAX_EXPANDED_BYTES)
        {
            return Err(InstallError::TooLarge(
                "expanded package exceeds 256 MiB".into(),
            ));
        }

        let output = destination.join(relative);
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent).map_err(io_error)?;
        }
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&output)
            .map_err(io_error)?;
        let copied = std::io::copy(&mut entry.by_ref().take(MAX_FILE_BYTES + 1), &mut file)
            .map_err(io_error)?;
        if copied > MAX_FILE_BYTES {
            return Err(InstallError::TooLarge(format!(
                "file exceeds 128 MiB while expanding: {raw}"
            )));
        }
        expanded = expanded
            .checked_add(copied)
            .ok_or_else(|| InstallError::TooLarge("expanded package is too large".into()))?;
        if expanded > MAX_EXPANDED_BYTES {
            return Err(InstallError::TooLarge(
                "expanded package exceeds 256 MiB".into(),
            ));
        }
        file.flush().map_err(io_error)?;
    }
    Ok(())
}

fn validate_entry_path(raw: &str) -> Result<(), InstallError> {
    if raw.is_empty()
        || raw.len() > 500
        || raw.contains('\\')
        || raw.contains(':')
        || raw.chars().any(char::is_control)
    {
        return Err(InstallError::Invalid(format!("unsafe ZIP path: {raw}")));
    }
    let trimmed = raw.trim_end_matches('/');
    if trimmed.is_empty()
        || Path::new(trimmed).is_absolute()
        || Path::new(trimmed)
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err(InstallError::Invalid(format!("unsafe ZIP path: {raw}")));
    }
    Ok(())
}

fn io_error(error: std::io::Error) -> InstallError {
    InstallError::Io(error.to_string())
}

struct StagingGuard {
    path: PathBuf,
    armed: bool,
}

impl StagingGuard {
    fn new(path: PathBuf) -> Self {
        Self { path, armed: true }
    }

    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for StagingGuard {
    fn drop(&mut self) {
        if self.armed {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;

    fn root() -> PathBuf {
        std::env::temp_dir().join(format!("laruche-app-install-{}", Uuid::new_v4()))
    }

    fn package(entries: &[(&str, &str)]) -> Vec<u8> {
        let cursor = Cursor::new(Vec::new());
        let mut writer = ZipWriter::new(cursor);
        for (name, body) in entries {
            writer
                .start_file(*name, SimpleFileOptions::default())
                .unwrap();
            writer.write_all(body.as_bytes()).unwrap();
        }
        writer.finish().unwrap().into_inner()
    }

    fn manifest(version: &str) -> String {
        format!(
            r#"{{"apiVersion":1,"id":"dev.laruche.test","name":"Test","version":"{version}","description":"Test app","publisher":{{"name":"LaRuche"}},"ui":{{"views":[{{"id":"main","title":"Main","entry":"ui/index.html"}}]}}}}"#
        )
    }

    #[test]
    fn installs_a_valid_package_at_its_identity_path() {
        let root = root();
        let bytes = package(&[
            ("app.json", &manifest("1.2.3")),
            ("ui/index.html", "<!doctype html>"),
        ]);
        let installed = install(&root, bytes).unwrap();
        assert_eq!(installed.manifest.id, "dev.laruche.test");
        assert!(installed.path.ends_with("dev.laruche.test/1.2.3"));
        assert!(installed.path.join("ui/index.html").is_file());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn accepts_a_legacy_addon_manifest() {
        let root = root();
        let bytes = package(&[
            ("addon.json", &manifest("1.2.3")),
            ("ui/index.html", "<!doctype html>"),
        ]);
        let installed = install(&root, bytes).unwrap();
        assert_eq!(installed.manifest.id, "dev.laruche.test");
        assert!(installed.path.join("addon.json").is_file());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_traversal_and_cleans_staging() {
        let root = root();
        let bytes = package(&[("../escape.txt", "no")]);
        assert!(matches!(
            install(&root, bytes),
            Err(InstallError::Invalid(_))
        ));
        assert!(!root.join("escape.txt").exists());
        let staging = root.join(".staging");
        assert_eq!(fs::read_dir(staging).unwrap().count(), 0);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn refuses_overwriting_an_installed_version() {
        let root = root();
        let entries = [
            ("app.json", manifest("1.0.0")),
            ("ui/index.html", "ok".into()),
        ];
        let refs: Vec<(&str, &str)> = entries
            .iter()
            .map(|(name, body)| (*name, body.as_str()))
            .collect();
        install(&root, package(&refs)).unwrap();
        assert!(matches!(
            install(&root, package(&refs)),
            Err(InstallError::AlreadyInstalled)
        ));
        let _ = fs::remove_dir_all(root);
    }
}
