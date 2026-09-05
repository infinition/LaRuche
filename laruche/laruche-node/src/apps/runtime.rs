//! User-bound App commands, live permissions and reusable agent definitions.
use super::AppManifest;
use serde::{Deserialize, Serialize};
#[cfg(test)]
use serde_json::json;
use serde_json::Value;
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::sync::oneshot;
use uuid::Uuid;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Rule {
    pub discover: Option<bool>,
    pub open: Option<bool>,
    #[serde(default)]
    pub actions: BTreeMap<String, bool>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct AppPolicy {
    #[serde(default)]
    pub principals: BTreeMap<String, Rule>,
    #[serde(default)]
    pub invoke_agents: Vec<String>,
}
impl AppPolicy {
    pub fn allows(&self, principal: &str, operation: &str) -> bool {
        let get = |rule: &Rule| match operation {
            "discover" => rule.discover,
            "open" => rule.open,
            name => rule.actions.get(name).copied(),
        };
        self.principals
            .get(principal)
            .and_then(get)
            .or_else(|| self.principals.get("laruche").and_then(get))
            .unwrap_or(operation == "discover")
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Agent {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub avatar: String,
    #[serde(default)]
    pub personality: String,
    #[serde(default)]
    pub instructions: String,
    pub profile_id: String,
    pub model: String,
    #[serde(default = "tokens")]
    pub max_tokens: u32,
    #[serde(default = "temperature")]
    pub temperature: f32,
}
fn tokens() -> u32 {
    1024
}
fn temperature() -> f32 {
    0.4
}
impl Agent {
    pub fn validate(&self) -> Result<(), String> {
        if Uuid::parse_str(&self.id).is_err()
            || self.name.trim().is_empty()
            || self.name.len() > 80
            || self.avatar.len() > 256
            || self.avatar.contains('<')
            || self.avatar.contains('>')
            || self.personality.len() > 2000
            || self.instructions.len() > 16_384
            || self.profile_id.is_empty()
            || self.model.is_empty()
            || self.profile_id.len() > 128
            || self.model.len() > 200
            || !(64..=8192).contains(&self.max_tokens)
            || !self.temperature.is_finite()
            || !(0.0..=2.0).contains(&self.temperature)
        {
            return Err("Invalid agent definition".into());
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UserConfig {
    pub agents: Vec<Agent>,
    pub policies: BTreeMap<String, AppPolicy>,
}
#[derive(Clone, Default, Serialize, Deserialize)]
struct Store {
    #[serde(default)]
    users: BTreeMap<Uuid, UserConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Instance {
    pub instance_id: String,
    pub app_id: String,
    pub view_id: String,
    pub version: String,
    #[serde(default)]
    pub ready: bool,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub progress: Option<u8>,
}
pub(crate) struct Host {
    pub touched: Instant,
    pub instances: Vec<Instance>,
}
pub(crate) struct Pending {
    pub user: Uuid,
    pub host: String,
    pub app: String,
    pub principal: String,
    pub operation: String,
    pub payload: Value,
    pub sent: bool,
    pub expires: Instant,
    pub reply: oneshot::Sender<Result<Value, String>>,
}
pub(crate) struct Conversation {
    pub fingerprint: String,
    pub messages: Vec<Value>,
}
/// One conversation per (account, App, view, instance).
///
/// The key is long on purpose: two open games of the same App must not share a
/// context, and the account has to be part of the key so a second user never
/// lands in the first one's history.
pub(crate) type Sessions =
    Mutex<BTreeMap<(Uuid, String, String, String), Arc<tokio::sync::Mutex<Conversation>>>>;

pub(crate) struct Runtime {
    path: PathBuf,
    store: Mutex<Store>,
    pub hosts: Mutex<BTreeMap<(Uuid, String), Host>>,
    pub pending: Mutex<BTreeMap<Uuid, Pending>>,
    pub sessions: Sessions,
    pub model_slots: tokio::sync::Semaphore,
    pub rates: Mutex<BTreeMap<Uuid, Vec<Instant>>>,
}
/// What it takes to route one call to an open App view.
///
/// Six of these were already positional strings; adding the instance made
/// eight in a row, and eight bare arguments of the same type swap places at
/// the call site without the compiler ever noticing.
pub(crate) struct Call<'a> {
    pub user: Uuid,
    pub app: &'a str,
    pub principal: &'a str,
    pub operation: &'a str,
    pub view: &'a str,
    pub instance: Option<&'a str>,
}

impl Runtime {
    pub fn load(path: PathBuf) -> anyhow::Result<Self> {
        let backup = path.with_extension("json.bak");
        let store = if path.exists() {
            match serde_json::from_slice(&std::fs::read(&path)?) {
                Ok(s) => s,
                Err(e) => {
                    if backup.exists() {
                        serde_json::from_slice(&std::fs::read(&backup)?)?
                    } else {
                        return Err(e.into());
                    }
                }
            }
        } else if backup.exists() {
            serde_json::from_slice(&std::fs::read(&backup)?)?
        } else {
            Store::default()
        };
        Ok(Self {
            path,
            store: Mutex::new(store),
            hosts: Mutex::new(BTreeMap::new()),
            pending: Mutex::new(BTreeMap::new()),
            sessions: Mutex::new(BTreeMap::new()),
            model_slots: tokio::sync::Semaphore::new(4),
            rates: Mutex::new(BTreeMap::new()),
        })
    }
    pub fn config(&self, user: Uuid) -> UserConfig {
        self.store
            .lock()
            .unwrap()
            .users
            .get(&user)
            .cloned()
            .unwrap_or_default()
    }
    pub fn update(
        &self,
        user: Uuid,
        edit: impl FnOnce(&mut UserConfig) -> Result<(), String>,
    ) -> Result<(), String> {
        let mut guard = self.store.lock().unwrap();
        let mut next = guard.clone();
        edit(next.users.entry(user).or_default())?;
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let tmp = self.path.with_extension(format!("{}.tmp", Uuid::new_v4()));
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)
            .map_err(|e| e.to_string())?;
        file.write_all(&serde_json::to_vec_pretty(&next).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        drop(file);
        let backup = self.path.with_extension("json.bak");
        if self.path.exists() {
            std::fs::copy(&self.path, &backup).map_err(|e| e.to_string())?;
            std::fs::remove_file(&self.path).map_err(|e| e.to_string())?;
        }
        if let Err(error) = std::fs::rename(&tmp, &self.path) {
            if backup.exists() {
                let _ = std::fs::copy(&backup, &self.path);
            }
            return Err(error.to_string());
        }
        *guard = next;
        Ok(())
    }
    pub fn allows(&self, user: Uuid, app: &str, principal: &str, operation: &str) -> bool {
        self.config(user)
            .policies
            .get(app)
            .cloned()
            .unwrap_or_default()
            .allows(principal, operation)
    }
    pub async fn enqueue(&self, call: Call<'_>, payload: Value) -> Result<Value, String> {
        let Call {
            user,
            app,
            principal,
            operation,
            view,
            instance,
        } = call;
        if !self.allows(user, app, principal, operation) {
            return Err(
                "Permission denied. Open Apps > Permissions to grant this operation.".into(),
            );
        }
        let host = {
            let mut hosts = self.hosts.lock().unwrap();
            hosts.retain(|_, h| h.touched.elapsed() < Duration::from_secs(15));
            let matching: Vec<_> = hosts
                .iter()
                .filter(|((owner, _), h)| {
                    *owner == user
                        && (operation == "open"
                            || h.instances.iter().any(|i| {
                                i.app_id == app
                                    && i.view_id == view
                                    && instance.map(|s| s == i.instance_id).unwrap_or(true)
                            }))
                })
                .collect();
            if operation != "open" && instance.is_none() {
                let count: usize = matching
                    .iter()
                    .map(|(_, h)| {
                        h.instances
                            .iter()
                            .filter(|i| i.app_id == app && i.view_id == view)
                            .count()
                    })
                    .sum();
                if count > 1 {
                    return Err(
                        "Several App views are open. Specify instanceId from app_list.".into(),
                    );
                }
            }
            if operation != "open"
                && matching.iter().any(|(_, h)| {
                    h.instances.iter().any(|i| {
                        i.app_id == app
                            && i.view_id == view
                            && instance.map(|s| s == i.instance_id).unwrap_or(true)
                            && !i.ready
                    })
                })
            {
                return Err("App is loading. Call app_wait before sending actions.".into());
            }
            matching.into_iter().max_by_key(|(_,h)|h.touched).map(|((_,id),_)|id.clone()).ok_or("App view is not connected. Open LaRuche in your browser and use app_open first.")?
        };
        let id = Uuid::new_v4();
        let (tx, rx) = oneshot::channel();
        {
            let mut pending = self.pending.lock().unwrap();
            pending.retain(|_, p| p.expires > Instant::now());
            if pending.values().filter(|p| p.user == user).count() >= 8 {
                return Err("Too many App operations in progress".into());
            }
            pending.insert(
                id,
                Pending {
                    user,
                    host,
                    app: app.into(),
                    principal: principal.into(),
                    operation: operation.into(),
                    payload,
                    sent: false,
                    expires: Instant::now() + Duration::from_secs(25),
                    reply: tx,
                },
            );
        }
        let result = tokio::time::timeout(Duration::from_secs(25), rx).await;
        self.pending.lock().unwrap().remove(&id);
        result
            .map_err(|_| "App timed out; the command is not retried automatically".to_string())?
            .map_err(|_| "App command cancelled".to_string())?
    }
}

// Deliberately documented JSON Schema subset. Unsupported keywords are rejected
// when installing, rather than silently ignored during permissioned execution.
pub(crate) fn validate_schema(schema: &Value) -> Result<(), String> {
    validate_schema_depth(schema, 0)
}
fn validate_schema_depth(s: &Value, depth: usize) -> Result<(), String> {
    if depth > 8 || !s.is_object() {
        return Err("Invalid action schema".into());
    }
    let kind = s["type"].as_str().ok_or("Schema type required")?;
    if ![
        "object", "array", "string", "number", "integer", "boolean", "null",
    ]
    .contains(&kind)
    {
        return Err("Unsupported schema type".into());
    }
    for key in s.as_object().unwrap().keys() {
        if ![
            "type",
            "description",
            "title",
            "properties",
            "required",
            "additionalProperties",
            "items",
            "enum",
            "minimum",
            "maximum",
            "minLength",
            "maxLength",
            "maxItems",
            "minItems",
        ]
        .contains(&key.as_str())
        {
            return Err(format!("Unsupported schema keyword: {key}"));
        }
    }
    if let Some(p) = s.get("properties") {
        for child in p.as_object().ok_or("properties must be object")?.values() {
            validate_schema_depth(child, depth + 1)?;
        }
    }
    if let Some(items) = s.get("items") {
        validate_schema_depth(items, depth + 1)?;
    }
    if let Some(required) = s.get("required") {
        if !required
            .as_array()
            .map(|a| a.iter().all(Value::is_string))
            .unwrap_or(false)
        {
            return Err("required must be string array".into());
        }
    }
    if let Some(additional) = s.get("additionalProperties") {
        if !additional.is_boolean() {
            return Err("additionalProperties must be boolean".into());
        }
    }
    if let Some(values) = s.get("enum") {
        if !values.is_array() {
            return Err("enum must be an array".into());
        }
    }
    for key in ["minimum", "maximum"] {
        if let Some(v) = s.get(key) {
            if v.as_f64().filter(|n| n.is_finite()).is_none() {
                return Err(format!("{key} must be a finite number"));
            }
        }
    }
    for key in ["minLength", "maxLength", "minItems", "maxItems"] {
        if let Some(v) = s.get(key) {
            if v.as_u64().is_none() {
                return Err(format!("{key} must be a nonnegative integer"));
            }
        }
    }
    for (lo, hi) in [
        ("minimum", "maximum"),
        ("minLength", "maxLength"),
        ("minItems", "maxItems"),
    ] {
        if let (Some(a), Some(b)) = (s[lo].as_f64(), s[hi].as_f64()) {
            if a > b {
                return Err(format!("{lo} exceeds {hi}"));
            }
        }
    }
    Ok(())
}
pub(crate) fn check_input(s: &Value, v: &Value) -> bool {
    if let Some(values) = s["enum"].as_array() {
        if !values.contains(v) {
            return false;
        }
    }
    let range = |n: f64, lo: &str, hi: &str| {
        s[lo].as_f64().map(|a| n >= a).unwrap_or(true)
            && s[hi].as_f64().map(|a| n <= a).unwrap_or(true)
    };
    match s["type"].as_str().unwrap_or("") {
        "object" => v
            .as_object()
            .map(|o| {
                let props = s["properties"].as_object();
                s["required"]
                    .as_array()
                    .map(|r| r.iter().all(|k| o.contains_key(k.as_str().unwrap_or(""))))
                    .unwrap_or(true)
                    && o.iter().all(|(k, v)| {
                        props
                            .and_then(|p| p.get(k))
                            .map(|p| check_input(p, v))
                            .unwrap_or(s["additionalProperties"] != false)
                    })
            })
            .unwrap_or(false),
        "array" => v
            .as_array()
            .map(|a| {
                range(a.len() as f64, "minItems", "maxItems")
                    && a.iter()
                        .all(|v| s.get("items").map(|p| check_input(p, v)).unwrap_or(true))
            })
            .unwrap_or(false),
        "string" => v
            .as_str()
            .map(|t| range(t.chars().count() as f64, "minLength", "maxLength"))
            .unwrap_or(false),
        "number" | "integer" => v
            .as_f64()
            .map(|n| {
                n.is_finite()
                    && range(n, "minimum", "maximum")
                    && (s["type"] != "integer" || n.fract() == 0.0)
            })
            .unwrap_or(false),
        "boolean" => v.is_boolean(),
        "null" => v.is_null(),
        _ => false,
    }
}
pub(crate) fn action<'a>(
    manifest: &'a AppManifest,
    name: &str,
    args: &Value,
) -> Result<&'a super::AppAction, String> {
    let action = manifest
        .actions
        .iter()
        .find(|a| a.name == name)
        .ok_or("Unknown App action")?;
    if !check_input(&action.input_schema, args) {
        return Err("Arguments do not match the action inputSchema".into());
    }
    Ok(action)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn deny_and_inherit() {
        let mut p = AppPolicy::default();
        assert!(p.allows("a", "discover"));
        assert!(!p.allows("a", "open"));
        p.principals.insert(
            "laruche".into(),
            Rule {
                open: Some(true),
                ..Default::default()
            },
        );
        assert!(p.allows("a", "open"));
        p.principals.insert(
            "a".into(),
            Rule {
                open: Some(false),
                ..Default::default()
            },
        );
        assert!(!p.allows("a", "open"));
    }
    #[test]
    fn action_arguments() {
        let s = json!({"type":"object","required":["direction"],"additionalProperties":false,"properties":{"direction":{"type":"string","enum":["left","right"]}}});
        validate_schema(&s).unwrap();
        assert!(check_input(&s, &json!({"direction":"left"})));
        assert!(!check_input(&s, &json!({"direction":"up"})));
        assert!(!check_input(
            &s,
            &json!({"direction":"left","userId":"other"})
        ));
        assert!(validate_schema(&json!({"type":"object","$ref":"https://example.com"})).is_err());
    }
    #[test]
    fn grants_persist_and_are_user_bound() {
        let dir = std::env::temp_dir().join(format!("laruche-app-access-{}", Uuid::new_v4()));
        let path = dir.join("access.json");
        let first = Uuid::new_v4();
        let second = Uuid::new_v4();
        let runtime = Runtime::load(path.clone()).unwrap();
        runtime
            .update(first, |c| {
                let mut p = AppPolicy::default();
                p.principals.insert(
                    "laruche".into(),
                    Rule {
                        open: Some(true),
                        ..Default::default()
                    },
                );
                c.policies.insert("dev.test.app".into(), p);
                Ok(())
            })
            .unwrap();
        assert!(runtime.allows(first, "dev.test.app", "laruche", "open"));
        assert!(!runtime.allows(second, "dev.test.app", "laruche", "open"));
        drop(runtime);
        let runtime = Runtime::load(path).unwrap();
        assert!(runtime.allows(first, "dev.test.app", "laruche", "open"));
        drop(runtime);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
