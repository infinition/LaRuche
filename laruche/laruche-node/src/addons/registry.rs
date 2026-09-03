use super::{AddonManifest, BackendType};
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
pub(crate) enum AddonLifecycleState {
    Enabled,
    InstalledDisabled,
    Broken,
    Missing,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AddonSnapshot {
    pub(crate) id: String,
    pub(crate) active_version: String,
    pub(crate) enabled: bool,
    pub(crate) state: AddonLifecycleState,
    pub(crate) installed_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) manifest: Option<AddonManifest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AddonDiagnostic {
    pub(crate) path: String,
    pub(crate) error: String,
}

#[derive(Debug, Clone)]
struct AddonRecord {
    active_version: String,
    enabled: bool,
    installed_at: String,
    manifest: Option<AddonManifest>,
    error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StoredAddon {
    active_version: String,
    enabled: bool,
    installed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StoredRegistry {
    schema_version: u32,
    #[serde(default)]
    addons: BTreeMap<String, StoredAddon>,
}

impl Default for StoredRegistry {
    fn default() -> Self {
        Self {
            schema_version: REGISTRY_SCHEMA_VERSION,
            addons: BTreeMap::new(),
        }
    }
}

#[derive(Debug)]
pub(crate) struct AddonRegistry {
    root: PathBuf,
    records: BTreeMap<String, AddonRecord>,
    diagnostics: Vec<AddonDiagnostic>,
}

impl AddonRegistry {
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
                "unsupported addon registry schema {}",
                stored.schema_version
            ));
        }

        let discovered = discover_packages(&packages, &mut diagnostics)?;
        let mut records = BTreeMap::new();

        for (id, versions) in &discovered {
            let selected = stored
                .addons
                .get(id)
                .and_then(|saved| {
                    versions
                        .iter()
                        .find(|(_, manifest)| manifest.version == saved.active_version)
                })
                .or_else(|| versions.last());

            if let Some((_, manifest)) = selected {
                let saved = stored.addons.get(id);
                records.insert(
                    id.clone(),
                    AddonRecord {
                        active_version: manifest.version.clone(),
                        enabled: saved.map(|item| item.enabled).unwrap_or(false),
                        installed_at: saved
                            .map(|item| item.installed_at.clone())
                            .unwrap_or_else(|| Utc::now().to_rfc3339()),
                        manifest: Some(manifest.clone()),
                        error: None,
                    },
                );
            }
        }

        for (id, saved) in &stored.addons {
            if records.contains_key(id) {
                continue;
            }
            records.insert(
                id.clone(),
                AddonRecord {
                    active_version: saved.active_version.clone(),
                    enabled: false,
                    installed_at: saved.installed_at.clone(),
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

    pub(crate) fn list(&self) -> Vec<AddonSnapshot> {
        self.records
            .iter()
            .map(|(id, record)| snapshot(id, record))
            .collect()
    }

    pub(crate) fn diagnostics(&self) -> &[AddonDiagnostic] {
        &self.diagnostics
    }

    pub(crate) fn get(&self, id: &str) -> Option<AddonSnapshot> {
        self.records.get(id).map(|record| snapshot(id, record))
    }

    pub(crate) fn set_enabled(
        &mut self,
        id: &str,
        enabled: bool,
    ) -> Result<AddonSnapshot, RegistryMutationError> {
        let record = self
            .records
            .get_mut(id)
            .ok_or(RegistryMutationError::NotFound)?;
        if record.manifest.is_none() || record.error.is_some() {
            return Err(RegistryMutationError::Broken(
                record
                    .error
                    .clone()
                    .unwrap_or_else(|| "addon package is invalid".into()),
            ));
        }
        let old = record.enabled;
        record.enabled = enabled;
        if let Err(error) = self.persist() {
            if let Some(record) = self.records.get_mut(id) {
                record.enabled = old;
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
            addons: self
                .records
                .iter()
                .map(|(id, record)| {
                    (
                        id.clone(),
                        StoredAddon {
                            active_version: record.active_version.clone(),
                            enabled: record.enabled && record.manifest.is_some(),
                            installed_at: record.installed_at.clone(),
                        },
                    )
                })
                .collect(),
        };
        let bytes = serde_json::to_vec_pretty(&stored)
            .map_err(|error| format!("cannot serialize addon registry: {error}"))?;
        replace_recoverably(&self.root, &bytes)
    }
}

#[derive(Debug)]
pub(crate) enum RegistryMutationError {
    NotFound,
    Broken(String),
    Persistence(String),
}

fn snapshot(id: &str, record: &AddonRecord) -> AddonSnapshot {
    let state = if record.error.is_some() {
        if record.manifest.is_some() {
            AddonLifecycleState::Broken
        } else {
            AddonLifecycleState::Missing
        }
    } else if record.enabled {
        AddonLifecycleState::Enabled
    } else {
        AddonLifecycleState::InstalledDisabled
    };
    AddonSnapshot {
        id: id.to_string(),
        active_version: record.active_version.clone(),
        enabled: record.enabled && record.error.is_none(),
        state,
        installed_at: record.installed_at.clone(),
        manifest: record.manifest.clone(),
        error: record.error.clone(),
    }
}

fn load_stored_registry(root: &Path, diagnostics: &mut Vec<AddonDiagnostic>) -> StoredRegistry {
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
            Err(error) => diagnostics.push(AddonDiagnostic {
                path: path.display().to_string(),
                error: format!("registry file ignored: {error}"),
            }),
        }
    }
    StoredRegistry::default()
}

fn discover_packages(
    packages: &Path,
    diagnostics: &mut Vec<AddonDiagnostic>,
) -> Result<BTreeMap<String, Vec<(Version, AddonManifest)>>, String> {
    let canonical_root = fs::canonicalize(packages)
        .map_err(|error| format!("cannot resolve {}: {error}", packages.display()))?;
    let mut found: HashMap<String, Vec<(Version, AddonManifest)>> = HashMap::new();

    let ids = fs::read_dir(packages)
        .map_err(|error| format!("cannot read {}: {error}", packages.display()))?;
    for id_entry in ids.flatten() {
        if !safe_directory(&id_entry, &canonical_root) {
            diagnostics.push(AddonDiagnostic {
                path: id_entry.path().display().to_string(),
                error: "ignored: package id entry is not a safe directory".into(),
            });
            continue;
        }
        let versions = match fs::read_dir(id_entry.path()) {
            Ok(entries) => entries,
            Err(error) => {
                diagnostics.push(AddonDiagnostic {
                    path: id_entry.path().display().to_string(),
                    error: format!("cannot read versions: {error}"),
                });
                continue;
            }
        };
        for version_entry in versions.flatten() {
            if !safe_directory(&version_entry, &canonical_root) {
                diagnostics.push(AddonDiagnostic {
                    path: version_entry.path().display().to_string(),
                    error: "ignored: version entry is not a safe directory".into(),
                });
                continue;
            }
            let manifest_path = version_entry.path().join("addon.json");
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
                Err(error) => diagnostics.push(AddonDiagnostic {
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

fn load_manifest(path: &Path, root: &Path) -> Result<AddonManifest, String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("cannot inspect addon.json: {error}"))?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > 256 * 1024 {
        return Err("addon.json is not a regular bounded file".into());
    }
    let canonical =
        fs::canonicalize(path).map_err(|error| format!("cannot resolve addon.json: {error}"))?;
    if !canonical.starts_with(root) {
        return Err("addon.json escapes the package root".into());
    }
    let input = fs::read_to_string(&canonical)
        .map_err(|error| format!("cannot read addon.json: {error}"))?;
    AddonManifest::parse_and_validate(&input)
}

fn validate_referenced_files(
    manifest: &AddonManifest,
    package: &Path,
    packages_root: &Path,
) -> Result<(), String> {
    let canonical_package = fs::canonicalize(package)
        .map_err(|error| format!("cannot resolve package directory: {error}"))?;
    if !canonical_package.starts_with(packages_root) {
        return Err("package directory escapes the packages root".into());
    }

    if let Some(icon) = &manifest.icon {
        validate_package_file(&canonical_package, icon, 1024 * 1024, "addon icon")?;
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
            .map_err(|error| format!("cannot rotate addon registry: {error}"))?;
    }
    if let Err(error) = fs::rename(&temporary, &target) {
        if backup.exists() && !target.exists() {
            let _ = fs::rename(&backup, &target);
        }
        let _ = fs::remove_file(&temporary);
        return Err(format!("cannot activate new addon registry: {error}"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temporary_root() -> PathBuf {
        std::env::temp_dir().join(format!("laruche-addons-test-{}", Uuid::new_v4()))
    }

    fn write_package(root: &Path, id: &str, version: &str, entry: &str) {
        let package = root.join("packages").join(id).join(version);
        fs::create_dir_all(package.join("ui")).unwrap();
        fs::write(package.join("ui/index.html"), "<!doctype html>").unwrap();
        fs::write(
            package.join("addon.json"),
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
        let mut registry = AddonRegistry::load(root.clone()).unwrap();
        assert!(!registry.get("dev.laruche.test").unwrap().enabled);
        registry.set_enabled("dev.laruche.test", true).unwrap();
        let reloaded = AddonRegistry::load(root.clone()).unwrap();
        assert!(reloaded.get("dev.laruche.test").unwrap().enabled);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn selects_the_newest_version_until_one_is_pinned() {
        let root = temporary_root();
        write_package(&root, "dev.laruche.test", "1.2.0", "ui/index.html");
        write_package(&root, "dev.laruche.test", "1.10.0", "ui/index.html");
        let registry = AddonRegistry::load(root.clone()).unwrap();
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
        let registry = AddonRegistry::load(root.clone()).unwrap();
        assert!(registry.list().is_empty());
        assert_eq!(registry.diagnostics().len(), 1);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn reports_a_missing_view_entry_without_registering_it() {
        let root = temporary_root();
        write_package(&root, "dev.laruche.test", "1.0.0", "ui/index.html");
        fs::remove_file(root.join("packages/dev.laruche.test/1.0.0/ui/index.html")).unwrap();
        let registry = AddonRegistry::load(root.clone()).unwrap();
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
        let mut registry = AddonRegistry::load(root.clone()).unwrap();
        registry.set_enabled("dev.laruche.test", true).unwrap();
        fs::remove_dir_all(root.join("packages/dev.laruche.test/1.0.0")).unwrap();
        let reloaded = AddonRegistry::load(root.clone()).unwrap();
        let addon = reloaded.get("dev.laruche.test").unwrap();
        assert!(!addon.enabled);
        assert!(matches!(addon.state, AddonLifecycleState::Missing));
        let _ = fs::remove_dir_all(root);
    }
}
