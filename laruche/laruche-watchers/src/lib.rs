use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum WatcherType {
    File,
    Url,
    Log,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Watcher {
    pub id: Uuid,
    pub name: String,
    #[serde(default)]
    pub profile_id: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    pub watcher_type: WatcherType,
    pub target: String,
    pub condition: String,
    pub prompt: String,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub last_run: Option<DateTime<Utc>>,
    pub run_count: u32,
    pub last_state: Option<String>,
}

pub struct WatchersRegistry {
    watchers: HashMap<Uuid, Watcher>,
    file_path: PathBuf,
}

impl WatchersRegistry {
    pub fn new(file_path: &Path) -> Self {
        let mut registry = Self {
            watchers: HashMap::new(),
            file_path: file_path.to_path_buf(),
        };
        if file_path.exists() {
            if let Ok(content) = std::fs::read_to_string(file_path) {
                if let Ok(watchers) = serde_json::from_str::<Vec<Watcher>>(&content) {
                    for w in watchers {
                        registry.watchers.insert(w.id, w);
                    }
                    tracing::info!(count = registry.watchers.len(), "Loaded watchers");
                }
            }
        }
        registry
    }

    pub fn add(&mut self, watcher: Watcher) -> Uuid {
        let id = watcher.id;
        tracing::info!(id = %id, name = %watcher.name, "Watcher added");
        self.watchers.insert(id, watcher);
        let _ = self.save();
        id
    }

    pub fn remove(&mut self, id: &Uuid) -> bool {
        let removed = self.watchers.remove(id).is_some();
        if removed {
            let _ = self.save();
        }
        removed
    }

    pub fn list(&self) -> Vec<&Watcher> {
        self.watchers.values().collect()
    }

    pub fn set_active(&mut self, id: &Uuid, active: bool) -> bool {
        if let Some(w) = self.watchers.get_mut(id) {
            w.active = active;
            let _ = self.save();
            true
        } else {
            false
        }
    }

    /// Met à jour les champs ÉDITABLES d'un watcher (id/run_count/created_at/last_state
    /// préservés). Un argument `None` = champ inchangé ; pour model/profile_id, `Some(None)`
    /// efface la valeur.
    #[allow(clippy::too_many_arguments)]
    pub fn update(
        &mut self,
        id: &Uuid,
        name: Option<String>,
        watcher_type: Option<WatcherType>,
        target: Option<String>,
        condition: Option<String>,
        prompt: Option<String>,
        active: Option<bool>,
        model: Option<Option<String>>,
        profile_id: Option<Option<String>>,
    ) -> bool {
        if let Some(w) = self.watchers.get_mut(id) {
            if let Some(v) = name {
                w.name = v;
            }
            if let Some(v) = watcher_type {
                w.watcher_type = v;
            }
            if let Some(v) = target {
                w.target = v;
            }
            if let Some(v) = condition {
                w.condition = v;
            }
            if let Some(v) = prompt {
                w.prompt = v;
            }
            if let Some(v) = active {
                w.active = v;
            }
            if let Some(v) = model {
                w.model = v;
            }
            if let Some(v) = profile_id {
                w.profile_id = v;
            }
            let _ = self.save();
            true
        } else {
            false
        }
    }

    pub async fn check_triggered_watchers(&mut self) -> Vec<(Uuid, String, String)> {
        let mut triggered = Vec::new();
        let mut needs_save = false;
        let mut updates = Vec::new();

        for watcher in self.watchers.values() {
            if !watcher.active {
                continue;
            }

            match evaluate_watcher(watcher).await {
                Ok((is_triggered, new_state)) => {
                    if is_triggered {
                        let ctx = format!(
                            "Watcher '{}' ({:?}) triggered on target '{}'",
                            watcher.name, watcher.watcher_type, watcher.target
                        );
                        triggered.push((watcher.id, watcher.prompt.clone(), ctx));
                    }
                    if new_state != watcher.last_state {
                        updates.push((watcher.id, new_state, is_triggered));
                    }
                }
                Err(e) => {
                    tracing::error!("Error evaluating watcher {}: {}", watcher.name, e);
                }
            }
        }

        for (id, new_state, is_triggered) in updates {
            if let Some(w) = self.watchers.get_mut(&id) {
                w.last_state = new_state;
                if is_triggered {
                    w.last_run = Some(Utc::now());
                    w.run_count += 1;
                }
                needs_save = true;
            }
        }

        if needs_save {
            let _ = self.save();
        }

        triggered
    }

    fn save(&self) -> Result<()> {
        if let Some(parent) = self.file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let watchers: Vec<&Watcher> = self.watchers.values().collect();
        let json = serde_json::to_string_pretty(&watchers)?;
        std::fs::write(&self.file_path, json)?;
        Ok(())
    }
}

async fn evaluate_watcher(watcher: &Watcher) -> Result<(bool, Option<String>)> {
    match watcher.watcher_type {
        WatcherType::File => {
            let meta = std::fs::metadata(&watcher.target)?;
            let modified = meta.modified()?;
            let mod_time: chrono::DateTime<chrono::Utc> = modified.into();
            let new_state = mod_time.to_rfc3339();

            if let Some(ref last) = watcher.last_state {
                if new_state != *last {
                    return Ok((true, Some(new_state)));
                }
                return Ok((false, Some(new_state)));
            } else {
                return Ok((false, Some(new_state)));
            }
        }
        WatcherType::Url => {
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()?;
            let text = client.get(&watcher.target).send().await?.text().await?;

            let mut hasher = DefaultHasher::new();
            text.hash(&mut hasher);
            let new_state = hasher.finish().to_string();

            if let Some(ref last) = watcher.last_state {
                if new_state != *last {
                    return Ok((true, Some(new_state)));
                }
                return Ok((false, Some(new_state)));
            } else {
                return Ok((false, Some(new_state)));
            }
        }
        WatcherType::Log => {
            let mut file = File::open(&watcher.target)?;
            let mut last_offset: u64 = 0;
            if let Some(ref last) = watcher.last_state {
                last_offset = last.parse().unwrap_or(0);
            }

            let current_len = file.metadata()?.len();
            if current_len < last_offset {
                last_offset = 0;
            }

            if current_len == last_offset {
                return Ok((false, Some(current_len.to_string())));
            }

            file.seek(SeekFrom::Start(last_offset))?;
            let mut content = String::new();
            file.read_to_string(&mut content)?;

            let triggered = content.contains(&watcher.condition);
            Ok((triggered, Some(current_len.to_string())))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_file_watcher() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "hello").unwrap();

        let target = file.path().to_str().unwrap().to_string();

        let watcher = Watcher {
            id: Uuid::new_v4(),
            name: "test".into(),
            watcher_type: WatcherType::File,
            target: target.clone(),
            condition: "".into(),
            prompt: "Do something".into(),
            active: true,
            created_at: Utc::now(),
            last_run: None,
            run_count: 0,
            last_state: None,
            model: None,
            profile_id: None,
        };

        // First run initializes state without triggering
        let (triggered, state) = evaluate_watcher(&watcher).await.unwrap();
        assert!(!triggered);
        assert!(state.is_some());

        // Second run with same file should not trigger
        let mut watcher2 = watcher.clone();
        watcher2.last_state = state.clone();
        let (triggered2, state2) = evaluate_watcher(&watcher2).await.unwrap();
        assert!(!triggered2);
        assert_eq!(state, state2);

        // Wait a little to ensure modification time is different
        // In actual tests, modifying time might have low resolution on some file systems.
        // We'll trust standard fs tests for now, but sleep could be used if it flakes.
    }
}
