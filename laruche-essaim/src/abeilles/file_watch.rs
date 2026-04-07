use crate::abeille::{Abeille, ContextExecution, NiveauDanger, ResultatAbeille};
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// Abeille that checks if a file has been modified since a given timestamp.
pub struct FileWatch;

#[async_trait]
impl Abeille for FileWatch {
    fn nom(&self) -> &str {
        "file_watch"
    }

    fn description(&self) -> &str {
        "Check if a file has been modified since a given timestamp. \
         Returns whether the file was modified and its last modification time."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Absolute or relative path to the file to watch"
                },
                "since": {
                    "type": "string",
                    "description": "ISO 8601 timestamp to compare against (e.g., '2026-04-05T12:00:00Z')"
                }
            },
            "required": ["path", "since"]
        })
    }

    fn niveau_danger(&self) -> NiveauDanger {
        NiveauDanger::Safe
    }

    async fn executer(
        &self,
        args: serde_json::Value,
        _ctx: &ContextExecution,
    ) -> Result<ResultatAbeille> {
        let path = args["path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'path' argument"))?;

        let since_str = args["since"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'since' argument"))?;

        let since: DateTime<Utc> = match since_str.parse() {
            Ok(dt) => dt,
            Err(e) => {
                return Ok(ResultatAbeille::err(format!(
                    "Invalid ISO 8601 timestamp '{}': {}",
                    since_str, e
                )));
            }
        };

        let metadata = match std::fs::metadata(path) {
            Ok(m) => m,
            Err(e) => {
                return Ok(ResultatAbeille::err(format!(
                    "Cannot access file '{}': {}",
                    path, e
                )));
            }
        };

        let modified_system = match metadata.modified() {
            Ok(t) => t,
            Err(e) => {
                return Ok(ResultatAbeille::err(format!(
                    "Cannot read modification time for '{}': {}",
                    path, e
                )));
            }
        };

        let last_modified: DateTime<Utc> = modified_system.into();
        let was_modified = last_modified > since;

        Ok(ResultatAbeille::ok(
            serde_json::json!({
                "modified": was_modified,
                "last_modified": last_modified.to_rfc3339(),
            })
            .to_string(),
        ))
    }
}
