use semver::Version;
use serde::{Deserialize, Serialize};
use std::path::{Component, Path};

pub(crate) const APP_API_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct AppManifest {
    pub(crate) api_version: u32,
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) version: String,
    pub(crate) description: String,
    pub(crate) publisher: Publisher,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) icon: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) homepage: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) license: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) compatibility: Option<Compatibility>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) ui: Option<AppUi>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) backend: Option<AppBackend>,
    #[serde(default)]
    pub(crate) permissions: AppPermissions,
    #[serde(default)]
    pub(crate) contributes: Contributions,
    #[serde(default)]
    pub(crate) guide: String,
    #[serde(default)]
    pub(crate) actions: Vec<AppAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct AppAction {
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) view_id: String,
    pub(crate) input_schema: serde_json::Value,
    #[serde(default)]
    pub(crate) read_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Publisher {
    pub(crate) name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) key_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Compatibility {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) laruche: Option<String>,
    #[serde(default)]
    pub(crate) platforms: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct AppUi {
    pub(crate) views: Vec<AppView>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct AppView {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) entry: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) icon: Option<String>,
    #[serde(default = "default_true")]
    pub(crate) navigation: bool,
    #[serde(default = "default_true")]
    pub(crate) detachable: bool,
    #[serde(default)]
    pub(crate) multi_instance: bool,
    #[serde(default)]
    pub(crate) wait_for_ready: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) min_size: Option<ViewSize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ViewSize {
    pub(crate) width: u32,
    pub(crate) height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum BackendType {
    McpStdio,
    Wasi,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct AppBackend {
    #[serde(rename = "type")]
    pub(crate) kind: BackendType,
    pub(crate) command: String,
    #[serde(default)]
    pub(crate) args: Vec<String>,
    #[serde(default = "default_health_timeout")]
    pub(crate) health_timeout_ms: u64,
    #[serde(default = "default_shutdown_timeout")]
    pub(crate) shutdown_timeout_ms: u64,
    #[serde(default)]
    pub(crate) restart: RestartPolicy,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum RestartPolicy {
    Never,
    #[default]
    OnFailure,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct AppPermissions {
    #[serde(default)]
    pub(crate) required: Vec<String>,
    #[serde(default)]
    pub(crate) optional: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Contributions {
    #[serde(default)]
    pub(crate) tools: Vec<String>,
    #[serde(default)]
    pub(crate) events: Vec<String>,
    #[serde(default)]
    pub(crate) jobs: Vec<String>,
}

fn default_true() -> bool {
    true
}

fn default_health_timeout() -> u64 {
    10_000
}

fn default_shutdown_timeout() -> u64 {
    5_000
}

impl AppManifest {
    pub(crate) fn parse_and_validate(input: &str) -> Result<Self, String> {
        const MAX_MANIFEST_BYTES: usize = 256 * 1024;
        if input.len() > MAX_MANIFEST_BYTES {
            return Err("manifest exceeds 256 KiB".into());
        }
        let manifest: Self =
            serde_json::from_str(input).map_err(|error| format!("invalid app.json: {error}"))?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub(crate) fn validate(&self) -> Result<(), String> {
        if self.api_version != APP_API_VERSION {
            return Err(format!(
                "unsupported apiVersion {} (expected {APP_API_VERSION})",
                self.api_version
            ));
        }
        validate_app_id(&self.id)?;
        Version::parse(&self.version).map_err(|error| format!("invalid version: {error}"))?;
        validate_text("name", &self.name, 1, 80)?;
        validate_text("description", &self.description, 1, 500)?;
        validate_text("publisher.name", &self.publisher.name, 1, 100)?;
        if self.ui.is_none() && self.backend.is_none() {
            return Err("an app needs at least a UI or a backend".into());
        }
        if let Some(icon) = &self.icon {
            validate_package_path("icon", icon)?;
        }
        if let Some(ui) = &self.ui {
            if ui.views.is_empty() || ui.views.len() > 20 {
                return Err("ui.views must contain between 1 and 20 views".into());
            }
            let mut ids = std::collections::HashSet::new();
            for view in &ui.views {
                validate_local_id("view id", &view.id)?;
                if !ids.insert(&view.id) {
                    return Err(format!("duplicate view id: {}", view.id));
                }
                validate_text("view title", &view.title, 1, 80)?;
                validate_package_path("view entry", &view.entry)?;
                if !view.entry.starts_with("ui/") {
                    return Err(format!("view entry must live under ui/: {}", view.entry));
                }
                if let Some(icon) = &view.icon {
                    validate_package_path("view icon", icon)?;
                }
                if let Some(size) = &view.min_size {
                    if !(240..=4096).contains(&size.width) || !(240..=4096).contains(&size.height) {
                        return Err("view minSize must stay between 240 and 4096".into());
                    }
                }
            }
        }
        if let Some(backend) = &self.backend {
            validate_text("backend.command", &backend.command, 1, 500)?;
            if backend.args.len() > 100 || backend.args.iter().any(|arg| arg.len() > 1_000) {
                return Err("backend args exceed their limit".into());
            }
            if !(1_000..=120_000).contains(&backend.health_timeout_ms) {
                return Err("healthTimeoutMs must stay between 1000 and 120000".into());
            }
            if !(100..=30_000).contains(&backend.shutdown_timeout_ms) {
                return Err("shutdownTimeoutMs must stay between 100 and 30000".into());
            }
            if matches!(backend.kind, BackendType::Wasi) {
                validate_package_path("WASI module", &backend.command)?;
            }
        }
        validate_permissions(&self.permissions)?;
        validate_contributions(&self.contributes)?;
        if self.guide.len() > 16_384 || self.actions.len() > 64 {
            return Err("guide or action catalogue too large".into());
        }
        let mut names = std::collections::HashSet::new();
        for action in &self.actions {
            if ["open", "discover"].contains(&action.name.as_str()) {
                return Err("Reserved action name".into());
            }
            if action.name.is_empty()
                || action.name.len() > 80
                || !action
                    .name
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
                || !names.insert(&action.name)
            {
                return Err("invalid or duplicate action name".into());
            }
            validate_text("action description", &action.description, 1, 1000)?;
            if !self
                .ui
                .as_ref()
                .map(|ui| ui.views.iter().any(|v| v.id == action.view_id))
                .unwrap_or(false)
            {
                return Err("action view does not exist".into());
            }
            super::runtime::validate_schema(&action.input_schema)?;
        }
        Ok(())
    }
}

fn validate_text(field: &str, value: &str, min: usize, max: usize) -> Result<(), String> {
    let trimmed = value.trim();
    if trimmed.len() < min || trimmed.len() > max || trimmed.chars().any(char::is_control) {
        return Err(format!(
            "{field} has an invalid length or control character"
        ));
    }
    Ok(())
}

fn validate_app_id(id: &str) -> Result<(), String> {
    if id.len() < 3
        || id.len() > 128
        || !id.contains('.')
        || id.starts_with(['.', '-'])
        || id.ends_with(['.', '-'])
        || !id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'.' || b == b'-')
    {
        return Err("id must be a lowercase reverse-DNS identifier".into());
    }
    if id.split(['.', '-']).any(str::is_empty) {
        return Err("id cannot contain consecutive separators".into());
    }
    Ok(())
}

fn validate_local_id(field: &str, id: &str) -> Result<(), String> {
    if id.is_empty()
        || id.len() > 80
        || !id.as_bytes()[0].is_ascii_lowercase()
        || !id.bytes().all(|b| {
            b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'.' | b'-' | b'_')
        })
    {
        return Err(format!("{field} is not a valid local identifier: {id}"));
    }
    Ok(())
}

fn validate_package_path(field: &str, value: &str) -> Result<(), String> {
    if value.is_empty()
        || value.len() > 500
        || value.contains('\\')
        || value.contains(':')
        || value.chars().any(char::is_control)
    {
        return Err(format!("{field} is not a safe package path"));
    }
    let path = Path::new(value);
    if path.is_absolute()
        || path
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err(format!("{field} is not a safe package path"));
    }
    Ok(())
}

fn validate_permissions(permissions: &AppPermissions) -> Result<(), String> {
    let all = permissions.required.iter().chain(&permissions.optional);
    let mut unique = std::collections::HashSet::new();
    for permission in all {
        if permission.is_empty()
            || permission.len() > 300
            || !permission.as_bytes()[0].is_ascii_lowercase()
            || permission.chars().any(char::is_whitespace)
            || !permission
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || ".:_-*".contains(c))
        {
            return Err(format!("invalid permission: {permission}"));
        }
        if !unique.insert(permission) {
            return Err(format!("duplicate permission: {permission}"));
        }
    }
    Ok(())
}

fn validate_contributions(contributions: &Contributions) -> Result<(), String> {
    for (kind, values, max) in [
        ("tool", &contributions.tools, 100usize),
        ("event", &contributions.events, 100usize),
        ("job", &contributions.jobs, 50usize),
    ] {
        if values.len() > max {
            return Err(format!("too many {kind} contributions"));
        }
        let mut unique = std::collections::HashSet::new();
        for value in values {
            validate_local_id(kind, value)?;
            if !unique.insert(value) {
                return Err(format!("duplicate {kind}: {value}"));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(entry: &str) -> String {
        format!(
            r#"{{
              "apiVersion": 1,
              "id": "dev.laruche.test",
              "name": "Test",
              "version": "1.2.3",
              "description": "A test app",
              "publisher": {{"name": "LaRuche"}},
              "ui": {{"views": [{{"id": "main", "title": "Main", "entry": "{entry}"}}]}},
              "permissions": {{"required": ["storage.private"], "optional": []}}
            }}"#
        )
    }

    #[test]
    fn parses_a_valid_manifest() {
        let parsed = AppManifest::parse_and_validate(&manifest("ui/index.html")).unwrap();
        assert_eq!(parsed.id, "dev.laruche.test");
        assert!(parsed.ui.unwrap().views[0].detachable);
    }

    #[test]
    fn rejects_path_traversal() {
        let error =
            AppManifest::parse_and_validate(&manifest("ui/../../secrets.json")).unwrap_err();
        assert!(error.contains("safe package path"), "{error}");
    }

    #[test]
    fn rejects_unknown_fields() {
        let input = manifest("ui/index.html").replace(
            "\"name\": \"Test\"",
            "\"name\": \"Test\", \"surprise\": true",
        );
        assert!(AppManifest::parse_and_validate(&input).is_err());
    }

    #[test]
    fn rejects_duplicate_permissions() {
        let input = manifest("ui/index.html")
            .replace("\"optional\": []", "\"optional\": [\"storage.private\"]");
        assert!(AppManifest::parse_and_validate(&input)
            .unwrap_err()
            .contains("duplicate permission"));
    }
}
