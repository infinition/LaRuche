use crate::abeille::{Abeille, ContextExecution, NiveauDanger, ResultatAbeille};
use anyhow::Result;
use async_trait::async_trait;
use std::path::Path;

/// Write content to a file.
pub struct FileWrite;

#[async_trait]
impl Abeille for FileWrite {
    fn nom(&self) -> &str {
        "file_write"
    }

    fn description(&self) -> &str {
        "Write text content to a file at the given path. Creates the file if it doesn't exist, \
         overwrites if it does. Use with caution."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "The file path to write to"
                },
                "content": {
                    "type": "string",
                    "description": "The text content to write"
                }
            },
            "required": ["path", "content"]
        })
    }

    fn niveau_danger(&self) -> NiveauDanger {
        NiveauDanger::NeedsApproval
    }

    async fn executer(
        &self,
        args: serde_json::Value,
        _ctx: &ContextExecution,
    ) -> Result<ResultatAbeille> {
        let path_str = args["path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'path' argument"))?;
        let content = args["content"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'content' argument"))?;

        let path = Path::new(path_str);

        // Create parent directories if needed
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    return Ok(ResultatAbeille::err(format!(
                        "Failed to create directories: {}", e
                    )));
                }
            }
        }

        match std::fs::write(path, content) {
            Ok(()) => Ok(ResultatAbeille::ok(format!(
                "File written successfully: {} ({} bytes)",
                path_str,
                content.len()
            ))),
            Err(e) => Ok(ResultatAbeille::err(format!(
                "Failed to write file: {}", e
            ))),
        }
    }
}


/// Read the contents of a file.
pub struct FileRead;

#[async_trait]
impl Abeille for FileRead {
    fn nom(&self) -> &str {
        "file_read"
    }

    fn description(&self) -> &str {
        "Read the contents of a file at the given path. Returns the file text content."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "The file path to read"
                }
            },
            "required": ["path"]
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
        let path_str = args["path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'path' argument"))?;

        let path = Path::new(path_str);

        if !path.exists() {
            return Ok(ResultatAbeille::err(format!(
                "File not found: {}",
                path_str
            )));
        }

        if !path.is_file() {
            return Ok(ResultatAbeille::err(format!(
                "Not a file: {}",
                path_str
            )));
        }

        // Limit file size to 100KB to avoid context explosion
        let metadata = std::fs::metadata(path)?;
        if metadata.len() > 100_000 {
            return Ok(ResultatAbeille::err(format!(
                "File too large ({} bytes). Maximum is 100,000 bytes.",
                metadata.len()
            )));
        }

        match std::fs::read_to_string(path) {
            Ok(content) => Ok(ResultatAbeille::ok(content)),
            Err(e) => Ok(ResultatAbeille::err(format!(
                "Failed to read file: {}",
                e
            ))),
        }
    }
}

/// List files in a directory.
pub struct FileList;

#[async_trait]
impl Abeille for FileList {
    fn nom(&self) -> &str {
        "file_list"
    }

    fn description(&self) -> &str {
        "List files and directories at the given path. Returns names with [DIR] or [FILE] prefix."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "The directory path to list"
                }
            },
            "required": ["path"]
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
        let path_str = args["path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'path' argument"))?;

        let path = Path::new(path_str);

        if !path.exists() {
            return Ok(ResultatAbeille::err(format!(
                "Directory not found: {}",
                path_str
            )));
        }

        if !path.is_dir() {
            return Ok(ResultatAbeille::err(format!(
                "Not a directory: {}",
                path_str
            )));
        }

        let mut entries = Vec::new();
        let mut count = 0;

        match std::fs::read_dir(path) {
            Ok(reader) => {
                for entry in reader {
                    if count >= 100 {
                        entries.push("... (truncated, more than 100 entries)".to_string());
                        break;
                    }
                    if let Ok(entry) = entry {
                        let name = entry.file_name().to_string_lossy().to_string();
                        let prefix = if entry.path().is_dir() {
                            "[DIR]"
                        } else {
                            "[FILE]"
                        };
                        entries.push(format!("{} {}", prefix, name));
                        count += 1;
                    }
                }
            }
            Err(e) => {
                return Ok(ResultatAbeille::err(format!(
                    "Failed to read directory: {}",
                    e
                )));
            }
        }

        if entries.is_empty() {
            Ok(ResultatAbeille::ok("(empty directory)"))
        } else {
            Ok(ResultatAbeille::ok(entries.join("\n")))
        }
    }
}
