use super::{AppManifest, BackendType};
use chrono::Utc;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use uuid::Uuid;

const REGISTRY_SCHEMA_VERSION: u32 = 1;
const REGISTRY_FILE: &str = "registry.json";
const REGISTRY_BACKUP: &str = "registry.json.bak";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AppLifecycleState {
    Enabled,
    InstalledDisabled,
    Broken,
    Missing,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AppSnapshot {
    pub(crate) id: String,
    pub(crate) active_version: String,
    pub(crate) enabled: bool,
    pub(crate) state: AppLifecycleState,
    pub(crate) installed_at: String,
    pub(crate) granted_permissions: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) manifest: Option<AppManifest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AppDiagnostic {
    pub(crate) path: String,
    pub(crate) error: String,
}

#[derive(Debug, Clone)]
struct AppRecord {
    active_version: String,
    enabled: bool,
    installed_at: String,
    granted_permissions: Vec<String>,
    manifest: Option<AppManifest>,
    error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StoredApp {
    active_version: String,
    enabled: bool,
    installed_at: String,
    #[serde(default)]
    granted_permissions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StoredRegistry {
    schema_version: u32,
    #[serde(default, alias = "addons")]
    apps: BTreeMap<String, StoredApp>,
}

impl Default for StoredRegistry {
    fn default() -> Self {
        Self {
            schema_version: REGISTRY_SCHEMA_VERSION,
            apps: BTreeMap::new(),
        }
    }
}

#[derive(Debug)]
pub(crate) struct AppRegistry {
    root: PathBuf,
    records: BTreeMap<String, AppRecord>,
    diagnostics: Vec<AppDiagnostic>,
}

impl AppRegistry {
    pub(crate) fn empty(root: PathBuf) -> Self {
        Self {
            root,
            records: BTreeMap::new(),
            diagnostics: Vec::new(),
        }
    }

    pub(crate) fn load(root: PathBuf) -> Result<Self, String> {
        let packages = root.join("packages");
        fs::create_dir_all(&packages)
            .map_err(|error| format!("cannot create {}: {error}", packages.display()))?;

        let mut diagnostics = Vec::new();
        let stored = load_stored_registry(&root, &mut diagnostics);
        if stored.schema_version != REGISTRY_SCHEMA_VERSION {
            return Err(format!(
                "unsupported app registry schema {}",
                stored.schema_version
            ));
        }

        let discovered = discover_packages(&packages, &mut diagnostics)?;
        let mut records = BTreeMap::new();

        for (id, versions) in &discovered {
            let selected = stored
                .apps
                .get(id)
                .and_then(|saved| {
                    versions
                        .iter()
                        .find(|(_, manifest)| manifest.version == saved.active_version)
                })
                .or_else(|| versions.last());

            if let Some((_, manifest)) = selected {
                let saved = stored.apps.get(id);
                let granted_permissions = saved
                    .map(|item| {
                        if item.enabled && item.granted_permissions.is_empty() {
                            manifest.permissions.required.clone()
                        } else {
                            item.granted_permissions.clone()
                        }
                    })
                    .unwrap_or_default();
                records.insert(
                    id.clone(),
                    AppRecord {
                        active_version: manifest.version.clone(),
                        enabled: saved.map(|item| item.enabled).unwrap_or(false),
                        installed_at: saved
                            .map(|item| item.installed_at.clone())
                            .unwrap_or_else(|| Utc::now().to_rfc3339()),
                        granted_permissions,
                        manifest: Some(manifest.clone()),
                        error: None,
                    },
                );
            }
        }

        for (id, saved) in &stored.apps {
            if records.contains_key(id) {
                continue;
            }
            records.insert(
                id.clone(),
                AppRecord {
                    active_version: saved.active_version.clone(),
                    enabled: false,
                    installed_at: saved.installed_at.clone(),
                    granted_permissions: Vec::new(),
                    manifest: None,
                    error: Some("the active package is missing or invalid".into()),
                },
            );
        }

        let registry = Self {
            root,
            records,
            diagnostics,
        };
        registry.persist()?;
        Ok(registry)
    }

    pub(crate) fn list(&self) -> Vec<AppSnapshot> {
        self.records
            .iter()
            .map(|(id, record)| snapshot(id, record))
            .collect()
    }

    pub(crate) fn diagnostics(&self) -> &[AppDiagnostic] {
        &self.diagnostics
    }

    pub(crate) fn get(&self, id: &str) -> Option<AppSnapshot> {
        self.records.get(id).map(|record| snapshot(id, record))
    }

    pub(crate) fn root(&self) -> &Path {
        &self.root
    }

    /// Select a freshly installed version, always disabled. An update may ask
    /// for broader permissions than the previous release, so installation must
    /// never silently keep executable authority.
    pub(crate) fn adopt_installed(
        &mut self,
        manifest: AppManifest,
    ) -> Result<AppSnapshot, RegistryMutationError> {
        let id = manifest.id.clone();
        let previous = self.records.get(&id).cloned();
        self.records.insert(
            id.clone(),
            AppRecord {
                active_version: manifest.version.clone(),
                enabled: false,
                installed_at: Utc::now().to_rfc3339(),
                granted_permissions: Vec::new(),
                manifest: Some(manifest),
                error: None,
            },
        );
        if let Err(error) = self.persist() {
            match previous {
                Some(record) => {
                    self.records.insert(id.clone(), record);
                }
                None => {
                    self.records.remove(&id);
                }
            }
            return Err(RegistryMutationError::Persistence(error));
        }
        Ok(self.get(&id).expect("installed app is present"))
    }

    pub(crate) fn set_enabled(
        &mut self,
        id: &str,
        enabled: bool,
        granted_permissions: Vec<String>,
    ) -> Result<AppSnapshot, RegistryMutationError> {
        let record = self
            .records
            .get_mut(id)
            .ok_or(RegistryMutationError::NotFound)?;
        if record.manifest.is_none() || record.error.is_some() {
            return Err(RegistryMutationError::Broken(
                record
                    .error
                    .clone()
                    .unwrap_or_else(|| "app package is invalid".into()),
            ));
        }
        let old = record.enabled;
        let old_permissions = record.granted_permissions.clone();
        record.enabled = enabled;
        record.granted_permissions = if enabled {
            granted_permissions
        } else {
            Vec::new()
        };
        if let Err(error) = self.persist() {
            if let Some(record) = self.records.get_mut(id) {
                record.enabled = old;
                record.granted_permissions = old_permissions;
            }
            return Err(RegistryMutationError::Persistence(error));
        }
        Ok(self.get(id).expect("record exists after mutation"))
    }

    pub(crate) fn rescan(&mut self) -> Result<(), String> {
        let replacement = Self::load(self.root.clone())?;
        *self = replacement;
        Ok(())
    }

    fn persist(&self) -> Result<(), String> {
        fs::create_dir_all(&self.root)
            .map_err(|error| format!("cannot create {}: {error}", self.root.display()))?;
        let stored = StoredRegistry {
            schema_version: REGISTRY_SCHEMA_VERSION,
            apps: self
                .records
                .iter()
                .map(|(id, record)| {
                    (
                        id.clone(),
                        StoredApp {
                            active_version: record.active_version.clone(),
                            enabled: record.enabled && record.manifest.is_some(),
                            installed_at: record.installed_at.clone(),
                            granted_permissions: record.granted_permissions.clone(),
                        },
                    )
                })
                .collect(),
        };
        let bytes = serde_json::to_vec_pretty(&stored)
            .map_err(|error| format!("cannot serialize app registry: {error}"))?;
        replace_recoverably(&self.root, &bytes)
    }
}

#[derive(Debug)]
pub(crate) enum RegistryMutationError {
    NotFound,
    Broken(String),
    Persistence(String),
}

#[derive(Debug)]
pub(crate) enum AssetLookupError {
    NotFound,
    Disabled,
    InvalidPath,
    TooLarge,
    Io(String),
}

fn snapshot(id: &str, record: &AppRecord) -> AppSnapshot {
    let state = if record.error.is_some() {
        if record.manifest.is_some() {
            AppLifecycleState::Broken
        } else {
            AppLifecycleState::Missing
        }
    } else if record.enabled {
        AppLifecycleState::Enabled
    } else {
        AppLifecycleState::InstalledDisabled
    };
    AppSnapshot {
        id: id.to_string(),
        active_version: record.active_version.clone(),
        enabled: record.enabled && record.error.is_none(),
        state,
        installed_at: record.installed_at.clone(),
        granted_permissions: record.granted_permissions.clone(),
        manifest: record.manifest.clone(),
        error: record.error.clone(),
    }
}

impl AppRegistry {
    /// Resolve a UI asset from an enabled, exact package version.
    ///
    /// The returned path has been canonicalized and checked beneath the package's
    /// `ui` directory. Callers must still open the path without following a swapped
    /// link; packages are immutable after installation, and the asset handler also
    /// checks metadata immediately before reading.
    /// Hosts this App's frame may reach, once the user has granted them.
    ///
    /// Empty unless the package declares them AND `network.fetch` is granted AND
    /// the App is enabled on its active version. Anything short of all three
    /// leaves the sandbox exactly as tight as it is by default, which is what a
    /// user who never approved the exception is entitled to.
    pub(crate) fn network_hosts(&self, id: &str, version: &str) -> Vec<String> {
        let Some(record) = self.records.get(id) else {
            return Vec::new();
        };
        if !record.enabled || record.error.is_some() || record.active_version != version {
            return Vec::new();
        }
        if !record
            .granted_permissions
            .iter()
            .any(|p| p == super::model::NETWORK_PERMISSION)
        {
            return Vec::new();
        }
        record
            .manifest
            .as_ref()
            .and_then(|m| m.network.as_ref())
            .map(|n| n.hosts.clone())
            .unwrap_or_default()
    }

    pub(crate) fn resolve_ui_asset(
        &self,
        id: &str,
        version: &str,
        relative: &str,
    ) -> Result<PathBuf, AssetLookupError> {
        let record = self.records.get(id).ok_or(AssetLookupError::NotFound)?;
        if !record.enabled || record.error.is_some() {
            return Err(AssetLookupError::Disabled);
        }
        if record.active_version != version {
            return Err(AssetLookupError::NotFound);
        }
        validate_asset_request_path(relative)?;

        let ui_root = self.root.join("packages").join(id).join(version).join("ui");
        let canonical_root =
            fs::canonicalize(&ui_root).map_err(|error| AssetLookupError::Io(error.to_string()))?;
        let candidate = ui_root.join(relative);
        let metadata = fs::symlink_metadata(&candidate).map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                AssetLookupError::NotFound
            } else {
                AssetLookupError::Io(error.to_string())
            }
        })?;
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            return Err(AssetLookupError::InvalidPath);
        }
        const MAX_UI_ASSET_BYTES: u64 = 32 * 1024 * 1024;
        if metadata.len() > MAX_UI_ASSET_BYTES {
            return Err(AssetLookupError::TooLarge);
        }
        let canonical = fs::canonicalize(&candidate)
            .map_err(|error| AssetLookupError::Io(error.to_string()))?;
        if !canonical.starts_with(&canonical_root) {
            return Err(AssetLookupError::InvalidPath);
        }
        Ok(canonical)
    }
}

fn validate_asset_request_path(relative: &str) -> Result<(), AssetLookupError> {
    if relative.is_empty()
        || relative.len() > 500
        || relative.contains('\\')
        || relative.contains(':')
        || relative.chars().any(char::is_control)
    {
        return Err(AssetLookupError::InvalidPath);
    }
    let path = Path::new(relative);
    if path.is_absolute()
        || path
            .components()
            .any(|part| !matches!(part, std::path::Component::Normal(_)))
    {
        return Err(AssetLookupError::InvalidPath);
    }
    Ok(())
}

fn load_stored_registry(root: &Path, diagnostics: &mut Vec<AppDiagnostic>) -> StoredRegistry {
    let primary = root.join(REGISTRY_FILE);
    let backup = root.join(REGISTRY_BACKUP);
    for path in [&primary, &backup] {
        if !path.exists() {
            continue;
        }
        match fs::read_to_string(path)
            .map_err(|error| error.to_string())
            .and_then(|input| {
                serde_json::from_str::<StoredRegistry>(&input).map_err(|e| e.to_string())
            }) {
            Ok(registry) => return registry,
            Err(error) => diagnostics.push(AppDiagnostic {
                path: path.display().to_string(),
                error: format!("registry file ignored: {error}"),
            }),
        }
    }
    StoredRegistry::default()
}

fn discover_packages(
    packages: &Path,
    diagnostics: &mut Vec<AppDiagnostic>,
) -> Result<BTreeMap<String, Vec<(Version, AppManifest)>>, String> {
    let canonical_root = fs::canonicalize(packages)
        .map_err(|error| format!("cannot resolve {}: {error}", packages.display()))?;
    let mut found: HashMap<String, Vec<(Version, AppManifest)>> = HashMap::new();

    let ids = fs::read_dir(packages)
        .map_err(|error| format!("cannot read {}: {error}", packages.display()))?;
    for id_entry in ids.flatten() {
        if !safe_directory(&id_entry, &canonical_root) {
            diagnostics.push(AppDiagnostic {
                path: id_entry.path().display().to_string(),
                error: "ignored: package id entry is not a safe directory".into(),
            });
            continue;
        }
        let versions = match fs::read_dir(id_entry.path()) {
            Ok(entries) => entries,
            Err(error) => {
                diagnostics.push(AppDiagnostic {
                    path: id_entry.path().display().to_string(),
                    error: format!("cannot read versions: {error}"),
                });
                continue;
            }
        };
        for version_entry in versions.flatten() {
            if !safe_directory(&version_entry, &canonical_root) {
                diagnostics.push(AppDiagnostic {
                    path: version_entry.path().display().to_string(),
                    error: "ignored: version entry is not a safe directory".into(),
                });
                continue;
            }
            let canonical_manifest = version_entry.path().join("app.json");
            let legacy_manifest = version_entry.path().join("addon.json");
            let manifest_path = if canonical_manifest.exists() {
                canonical_manifest
            } else {
                legacy_manifest
            };
            let result = load_manifest(&manifest_path, &canonical_root).and_then(|manifest| {
                validate_referenced_files(&manifest, &version_entry.path(), &canonical_root)?;
                let folder_id = id_entry.file_name().to_string_lossy().to_string();
                let folder_version = version_entry.file_name().to_string_lossy().to_string();
                if manifest.id != folder_id {
                    return Err(format!(
                        "manifest id {} does not match folder {folder_id}",
                        manifest.id
                    ));
                }
                if manifest.version != folder_version {
                    return Err(format!(
                        "manifest version {} does not match folder {folder_version}",
                        manifest.version
                    ));
                }
                let parsed = Version::parse(&manifest.version)
                    .map_err(|error| format!("invalid version: {error}"))?;
                Ok((parsed, manifest))
            });
            match result {
                Ok((version, manifest)) => {
                    found
                        .entry(manifest.id.clone())
                        .or_default()
                        .push((version, manifest));
                }
                Err(error) => diagnostics.push(AppDiagnostic {
                    path: manifest_path.display().to_string(),
                    error,
                }),
            }
        }
    }

    let mut sorted = BTreeMap::new();
    for (id, mut versions) in found {
        versions.sort_by(|a, b| a.0.cmp(&b.0));
        sorted.insert(id, versions);
    }
    Ok(sorted)
}

fn safe_directory(entry: &fs::DirEntry, root: &Path) -> bool {
    let file_type = match entry.file_type() {
        Ok(value) => value,
        Err(_) => return false,
    };
    if !file_type.is_dir() || file_type.is_symlink() {
        return false;
    }
    fs::canonicalize(entry.path())
        .map(|path| path.starts_with(root))
        .unwrap_or(false)
}

pub(super) fn load_manifest(path: &Path, root: &Path) -> Result<AppManifest, String> {
    let manifest_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("app.json");
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("cannot inspect {manifest_name}: {error}"))?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > 256 * 1024 {
        return Err(format!("{manifest_name} is not a regular bounded file"));
    }
    let canonical = fs::canonicalize(path)
        .map_err(|error| format!("cannot resolve {manifest_name}: {error}"))?;
    if !canonical.starts_with(root) {
        return Err(format!("{manifest_name} escapes the package root"));
    }
    let input = fs::read_to_string(&canonical)
        .map_err(|error| format!("cannot read {manifest_name}: {error}"))?;
    AppManifest::parse_and_validate(&input)
}

pub(super) fn validate_referenced_files(
    manifest: &AppManifest,
    package: &Path,
    packages_root: &Path,
) -> Result<(), String> {
    let canonical_package = fs::canonicalize(package)
        .map_err(|error| format!("cannot resolve package directory: {error}"))?;
    if !canonical_package.starts_with(packages_root) {
        return Err("package directory escapes the packages root".into());
    }

    if let Some(icon) = &manifest.icon {
        validate_package_file(&canonical_package, icon, 1024 * 1024, "app icon")?;
    }
    if let Some(ui) = &manifest.ui {
        for view in &ui.views {
            validate_package_file(
                &canonical_package,
                &view.entry,
                4 * 1024 * 1024,
                "view entry",
            )?;
            if let Some(icon) = &view.icon {
                validate_package_file(&canonical_package, icon, 1024 * 1024, "view icon")?;
            }
        }
    }
    if let Some(backend) = &manifest.backend {
        if matches!(backend.kind, BackendType::Wasi) {
            validate_package_file(
                &canonical_package,
                &backend.command,
                256 * 1024 * 1024,
                "WASI module",
            )?;
        }
    }
    Ok(())
}

fn validate_package_file(
    package: &Path,
    relative: &str,
    max_bytes: u64,
    label: &str,
) -> Result<(), String> {
    let candidate = package.join(relative);
    let metadata = fs::symlink_metadata(&candidate)
        .map_err(|error| format!("{label} {relative} cannot be inspected: {error}"))?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(format!("{label} {relative} is not a regular file"));
    }
    if metadata.len() > max_bytes {
        return Err(format!("{label} {relative} exceeds its size limit"));
    }
    let canonical = fs::canonicalize(&candidate)
        .map_err(|error| format!("{label} {relative} cannot be resolved: {error}"))?;
    if !canonical.starts_with(package) {
        return Err(format!("{label} {relative} escapes the package"));
    }
    Ok(())
}

fn replace_recoverably(root: &Path, bytes: &[u8]) -> Result<(), String> {
    let target = root.join(REGISTRY_FILE);
    let backup = root.join(REGISTRY_BACKUP);
    let temporary = root.join(format!("registry.json.new-{}", Uuid::new_v4()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|error| format!("cannot create {}: {error}", temporary.display()))?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|error| format!("cannot persist {}: {error}", temporary.display()))?;
    drop(file);

    if backup.exists() {
        fs::remove_file(&backup)
            .map_err(|error| format!("cannot remove old registry backup: {error}"))?;
    }
    if target.exists() {
        fs::rename(&target, &backup)
            .map_err(|error| format!("cannot rotate app registry: {error}"))?;
    }
    if let Err(error) = fs::rename(&temporary, &target) {
        if backup.exists() && !target.exists() {
            let _ = fs::rename(&backup, &target);
        }
        let _ = fs::remove_file(&temporary);
        return Err(format!("cannot activate new app registry: {error}"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temporary_root() -> PathBuf {
        std::env::temp_dir().join(format!("laruche-apps-test-{}", Uuid::new_v4()))
    }

    fn write_package(root: &Path, id: &str, version: &str, entry: &str) {
        let package = root.join("packages").join(id).join(version);
        fs::create_dir_all(package.join("ui")).unwrap();
        fs::write(package.join("ui/index.html"), "<!doctype html>").unwrap();
        fs::write(
            package.join("app.json"),
            format!(
                r#"{{
                  "apiVersion": 1,
                  "id": "{id}",
                  "name": "Test App",
                  "version": "{version}",
                  "description": "Registry test",
                  "publisher": {{"name": "LaRuche"}},
                  "ui": {{"views": [{{"id": "main", "title": "Main", "entry": "{entry}"}}]}},
                  "permissions": {{"required": [], "optional": []}}
                }}"#
            ),
        )
        .unwrap();
    }

    #[test]
    fn discovers_and_persists_enable_state() {
        let root = temporary_root();
        write_package(&root, "dev.laruche.test", "1.0.0", "ui/index.html");
        let mut registry = AppRegistry::load(root.clone()).unwrap();
        assert!(!registry.get("dev.laruche.test").unwrap().enabled);
        registry
            .set_enabled("dev.laruche.test", true, Vec::new())
            .unwrap();
        let reloaded = AppRegistry::load(root.clone()).unwrap();
        assert!(reloaded.get("dev.laruche.test").unwrap().enabled);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn persists_grants_and_revokes_them_when_disabled() {
        let root = temporary_root();
        write_package(&root, "dev.laruche.test", "1.0.0", "ui/index.html");
        let manifest_path = root.join("packages/dev.laruche.test/1.0.0/app.json");
        let manifest = fs::read_to_string(&manifest_path).unwrap().replace(
            r#""required": [], "optional": []"#,
            r#""required": ["storage.private"], "optional": []"#,
        );
        fs::write(manifest_path, manifest).unwrap();

        let mut registry = AppRegistry::load(root.clone()).unwrap();
        registry
            .set_enabled("dev.laruche.test", true, vec!["storage.private".into()])
            .unwrap();
        let mut reloaded = AppRegistry::load(root.clone()).unwrap();
        assert_eq!(
            reloaded
                .get("dev.laruche.test")
                .unwrap()
                .granted_permissions,
            vec!["storage.private"]
        );
        reloaded
            .set_enabled("dev.laruche.test", false, Vec::new())
            .unwrap();
        assert!(AppRegistry::load(root.clone())
            .unwrap()
            .get("dev.laruche.test")
            .unwrap()
            .granted_permissions
            .is_empty());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn migrates_implicit_required_grants_from_the_old_registry() {
        let root = temporary_root();
        write_package(&root, "dev.laruche.test", "1.0.0", "ui/index.html");
        let manifest_path = root.join("packages/dev.laruche.test/1.0.0/app.json");
        let manifest = fs::read_to_string(&manifest_path).unwrap().replace(
            r#""required": [], "optional": []"#,
            r#""required": ["storage.private"], "optional": []"#,
        );
        fs::write(manifest_path, manifest).unwrap();
        fs::write(
            root.join(REGISTRY_FILE),
            r#"{
              "schemaVersion": 1,
              "addons": {
                "dev.laruche.test": {
                  "activeVersion": "1.0.0",
                  "enabled": true,
                  "installedAt": "2026-01-01T00:00:00Z"
                }
              }
            }"#,
        )
        .unwrap();

        let app = AppRegistry::load(root.clone())
            .unwrap()
            .get("dev.laruche.test")
            .unwrap();
        assert_eq!(app.granted_permissions, vec!["storage.private"]);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn discovers_a_legacy_addon_manifest() {
        let root = temporary_root();
        write_package(&root, "dev.laruche.test", "1.0.0", "ui/index.html");
        let package = root.join("packages/dev.laruche.test/1.0.0");
        fs::rename(package.join("app.json"), package.join("addon.json")).unwrap();

        let app = AppRegistry::load(root.clone())
            .unwrap()
            .get("dev.laruche.test")
            .unwrap();
        assert_eq!(app.active_version, "1.0.0");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn adopting_an_update_selects_it_disabled_and_persists_the_choice() {
        let root = temporary_root();
        write_package(&root, "dev.laruche.test", "1.0.0", "ui/index.html");
        let mut registry = AppRegistry::load(root.clone()).unwrap();
        registry
            .set_enabled("dev.laruche.test", true, Vec::new())
            .unwrap();

        write_package(&root, "dev.laruche.test", "2.0.0", "ui/index.html");
        let packages_root = fs::canonicalize(root.join("packages")).unwrap();
        let manifest = load_manifest(
            &root.join("packages/dev.laruche.test/2.0.0/app.json"),
            &packages_root,
        )
        .unwrap();
        let installed = registry.adopt_installed(manifest).unwrap();
        assert_eq!(installed.active_version, "2.0.0");
        assert!(!installed.enabled);

        let reloaded = AppRegistry::load(root.clone()).unwrap();
        let persisted = reloaded.get("dev.laruche.test").unwrap();
        assert_eq!(persisted.active_version, "2.0.0");
        assert!(!persisted.enabled);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn selects_the_newest_version_until_one_is_pinned() {
        let root = temporary_root();
        write_package(&root, "dev.laruche.test", "1.2.0", "ui/index.html");
        write_package(&root, "dev.laruche.test", "1.10.0", "ui/index.html");
        let registry = AppRegistry::load(root.clone()).unwrap();
        assert_eq!(
            registry.get("dev.laruche.test").unwrap().active_version,
            "1.10.0"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn reports_an_invalid_package_without_registering_it() {
        let root = temporary_root();
        write_package(&root, "dev.laruche.test", "1.0.0", "../outside.html");
        let registry = AppRegistry::load(root.clone()).unwrap();
        assert!(registry.list().is_empty());
        assert_eq!(registry.diagnostics().len(), 1);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn reports_a_missing_view_entry_without_registering_it() {
        let root = temporary_root();
        write_package(&root, "dev.laruche.test", "1.0.0", "ui/index.html");
        fs::remove_file(root.join("packages/dev.laruche.test/1.0.0/ui/index.html")).unwrap();
        let registry = AppRegistry::load(root.clone()).unwrap();
        assert!(registry.list().is_empty());
        assert!(registry.diagnostics()[0]
            .error
            .contains("cannot be inspected"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn a_missing_active_package_is_visible_but_disabled() {
        let root = temporary_root();
        write_package(&root, "dev.laruche.test", "1.0.0", "ui/index.html");
        let mut registry = AppRegistry::load(root.clone()).unwrap();
        registry
            .set_enabled("dev.laruche.test", true, Vec::new())
            .unwrap();
        fs::remove_dir_all(root.join("packages/dev.laruche.test/1.0.0")).unwrap();
        let reloaded = AppRegistry::load(root.clone()).unwrap();
        let app = reloaded.get("dev.laruche.test").unwrap();
        assert!(!app.enabled);
        assert!(matches!(app.state, AppLifecycleState::Missing));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn assets_require_an_enabled_exact_version() {
        let root = temporary_root();
        write_package(&root, "dev.laruche.test", "1.0.0", "ui/index.html");
        let mut registry = AppRegistry::load(root.clone()).unwrap();
        assert!(matches!(
            registry.resolve_ui_asset("dev.laruche.test", "1.0.0", "index.html"),
            Err(AssetLookupError::Disabled)
        ));
        registry
            .set_enabled("dev.laruche.test", true, Vec::new())
            .unwrap();
        assert!(registry
            .resolve_ui_asset("dev.laruche.test", "1.0.0", "index.html")
            .unwrap()
            .ends_with("index.html"));
        assert!(matches!(
            registry.resolve_ui_asset("dev.laruche.test", "2.0.0", "index.html"),
            Err(AssetLookupError::NotFound)
        ));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn asset_paths_cannot_escape_the_ui_directory() {
        let root = temporary_root();
        write_package(&root, "dev.laruche.test", "1.0.0", "ui/index.html");
        let mut registry = AppRegistry::load(root.clone()).unwrap();
        registry
            .set_enabled("dev.laruche.test", true, Vec::new())
            .unwrap();
        for path in [
            "../app.json",
            "assets\\secret",
            "C:/boot.ini",
            "./index.html",
        ] {
            assert!(matches!(
                registry.resolve_ui_asset("dev.laruche.test", "1.0.0", path),
                Err(AssetLookupError::InvalidPath)
            ));
        }
        let _ = fs::remove_dir_all(root);
    }
}
