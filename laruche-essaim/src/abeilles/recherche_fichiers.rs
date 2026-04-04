use crate::abeille::{Abeille, ContextExecution, NiveauDanger, ResultatAbeille};
use anyhow::Result;
use async_trait::async_trait;
use std::path::Path;

/// Search for files by name pattern in a directory tree.
pub struct FileSearch;

#[async_trait]
impl Abeille for FileSearch {
    fn nom(&self) -> &str { "file_search" }
    fn description(&self) -> &str {
        "Search for files matching a pattern in a directory tree. \
         Returns matching file paths. Useful for finding files by name or extension."
    }
    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Root directory to search in" },
                "pattern": { "type": "string", "description": "Search pattern (case-insensitive substring match on filename)" },
                "max_depth": { "type": "integer", "description": "Maximum directory depth (default: 5)" }
            },
            "required": ["path", "pattern"]
        })
    }
    fn niveau_danger(&self) -> NiveauDanger { NiveauDanger::Safe }

    async fn executer(&self, args: serde_json::Value, _ctx: &ContextExecution) -> Result<ResultatAbeille> {
        let root = args["path"].as_str().ok_or_else(|| anyhow::anyhow!("Missing 'path'"))?;
        let pattern = args["pattern"].as_str().ok_or_else(|| anyhow::anyhow!("Missing 'pattern'"))?;
        let max_depth = args["max_depth"].as_u64().unwrap_or(5) as usize;

        let root_path = Path::new(root);
        if !root_path.exists() {
            return Ok(ResultatAbeille::err(format!("Directory not found: {}", root)));
        }

        let pattern_lower = pattern.to_lowercase();
        let mut results = Vec::new();
        search_recursive(root_path, &pattern_lower, 0, max_depth, &mut results);

        if results.is_empty() {
            Ok(ResultatAbeille::ok(format!("No files matching '{}' found in {}", pattern, root)))
        } else {
            let count = results.len();
            let display: Vec<String> = results.into_iter().take(50).collect();
            let mut output = display.join("\n");
            if count > 50 {
                output.push_str(&format!("\n... and {} more", count - 50));
            }
            Ok(ResultatAbeille::ok(format!("Found {} file(s):\n{}", count, output)))
        }
    }
}

fn search_recursive(
    dir: &Path,
    pattern: &str,
    depth: usize,
    max_depth: usize,
    results: &mut Vec<String>,
) {
    if depth > max_depth || results.len() >= 200 {
        return;
    }

    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_lowercase();

        if name.contains(pattern) {
            results.push(path.display().to_string());
        }

        if path.is_dir() && !name.starts_with('.') {
            search_recursive(&path, pattern, depth + 1, max_depth, results);
        }
    }
}
