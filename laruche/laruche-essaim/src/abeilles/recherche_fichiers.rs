//! file_search: find files by NAME (substring or `*` glob) and/or by CONTENT
//! (grep-like, line numbers) in a directory tree.
//!
//! War-machine spec: build/VCS noise dirs are skipped (`target/`, `node_modules/`,
//! `.git/`...), `*.rs`-style globs work, and `content` turns it into a grep that
//! returns `path:line: text` matches — the single most useful primitive for an
//! agent exploring a codebase.

use crate::abeille::{Abeille, ContextExecution, NiveauDanger, ResultatAbeille};
use crate::abeilles::fichiers::DOSSIERS_IGNORES;
use anyhow::Result;
use async_trait::async_trait;
use std::path::Path;

/// Max file size for content search (bigger = generated/binary, pure noise).
const CONTENU_MAX_OCTETS: u64 = 2_000_000;

/// Search for files by name pattern and/or content in a directory tree.
pub struct FileSearch;

#[async_trait]
impl Abeille for FileSearch {
    fn nom(&self) -> &str {
        "file_search"
    }
    fn description(&self) -> &str {
        "Search files in a directory tree. `pattern` matches file NAMES (substring, or \
         glob with `*`, e.g. `*.rs`, `carnet*`). Add `content` to grep INSIDE the matching \
         files: returns `path:line:` matches with the line text. Build/VCS dirs are skipped."
    }
    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Root directory to search in" },
                "pattern": { "type": "string", "description": "Filename filter: substring or glob with `*` (e.g. `*.rs`). Use `*` to match all files (with `content`)." },
                "content": { "type": "string", "description": "Text to find INSIDE files (case-insensitive grep, returns path:line: text)" },
                "max_depth": { "type": "integer", "description": "Maximum directory depth (default: 8)" }
            },
            "required": ["path", "pattern"]
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
        let root = args["path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'path'"))?;
        let pattern = args["pattern"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'pattern'"))?;
        let contenu = args["content"].as_str().filter(|s| !s.is_empty());
        let max_depth = args["max_depth"].as_u64().unwrap_or(8) as usize;

        let root_path = Path::new(root);
        if !root_path.exists() {
            return Ok(ResultatAbeille::err(format!(
                "Directory not found: {}",
                root
            )));
        }

        let pattern_lower = pattern.to_lowercase();
        let mut fichiers = Vec::new();
        collecter(root_path, &pattern_lower, 0, max_depth, &mut fichiers);

        // ── Content grep inside the matching files ──
        if let Some(cible) = contenu {
            let cible_lower = cible.to_lowercase();
            let mut hits: Vec<String> = Vec::new();
            let mut fichiers_touches = 0usize;
            for chemin in &fichiers {
                if hits.len() >= 100 {
                    break;
                }
                let p = Path::new(chemin);
                if p.metadata().map(|m| m.len()).unwrap_or(u64::MAX) > CONTENU_MAX_OCTETS {
                    continue;
                }
                let Ok(octets) = std::fs::read(p) else { continue };
                // Skip binaries (NUL byte heuristic).
                if octets.iter().take(4096).any(|&b| b == 0) {
                    continue;
                }
                let texte = String::from_utf8_lossy(&octets);
                let mut touche = false;
                for (i, ligne) in texte.lines().enumerate() {
                    if ligne.to_lowercase().contains(&cible_lower) {
                        touche = true;
                        let apercu: String = ligne.trim().chars().take(200).collect();
                        hits.push(format!("{chemin}:{}: {apercu}", i + 1));
                        if hits.len() >= 100 {
                            break;
                        }
                    }
                }
                if touche {
                    fichiers_touches += 1;
                }
            }
            return Ok(if hits.is_empty() {
                ResultatAbeille::ok(format!(
                    "No content match for '{cible}' in {} file(s) matching '{pattern}' under {root}",
                    fichiers.len()
                ))
            } else {
                ResultatAbeille::ok(format!(
                    "{} match(es) in {} file(s):\n{}{}",
                    hits.len(),
                    fichiers_touches,
                    hits.join("\n"),
                    if hits.len() >= 100 { "\n... (capped at 100 matches - narrow the search)" } else { "" }
                ))
            });
        }

        // ── Name search only ──
        if fichiers.is_empty() {
            Ok(ResultatAbeille::ok(format!(
                "No files matching '{}' found in {}",
                pattern, root
            )))
        } else {
            let count = fichiers.len();
            let display: Vec<String> = fichiers.into_iter().take(50).collect();
            let mut output = display.join("\n");
            if count > 50 {
                output.push_str(&format!("\n... and {} more", count - 50));
            }
            Ok(ResultatAbeille::ok(format!(
                "Found {} file(s):\n{}",
                count, output
            )))
        }
    }
}

/// Filename match: substring by default, glob when the pattern contains `*`
/// (segments must appear in order; anchored at start/end unless `*`-edged).
pub(crate) fn nom_correspond(nom: &str, pattern: &str) -> bool {
    if !pattern.contains('*') {
        return nom.contains(pattern);
    }
    let segments: Vec<&str> = pattern.split('*').collect();
    let mut pos = 0usize;
    for (i, seg) in segments.iter().enumerate() {
        if seg.is_empty() {
            continue;
        }
        match nom[pos..].find(seg) {
            Some(rel) => {
                // First segment anchored at the start unless the pattern starts with `*`.
                if i == 0 && rel != 0 {
                    return false;
                }
                pos += rel + seg.len();
            }
            None => return false,
        }
    }
    // Last segment anchored at the end unless the pattern ends with `*`.
    if let Some(dernier) = segments.last() {
        if !dernier.is_empty() && !nom.ends_with(dernier) {
            return false;
        }
    }
    true
}

fn collecter(
    dir: &Path,
    pattern: &str,
    depth: usize,
    max_depth: usize,
    results: &mut Vec<String>,
) {
    if depth > max_depth || results.len() >= 500 {
        return;
    }

    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_lowercase();

        if path.is_dir() {
            if !name.starts_with('.') && !DOSSIERS_IGNORES.contains(&name.as_str()) {
                collecter(&path, pattern, depth + 1, max_depth, results);
            }
        } else if pattern == "*" || nom_correspond(&name, pattern) {
            results.push(path.display().to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glob_et_substring() {
        // substring (no `*`)
        assert!(nom_correspond("carnet.rs", "carnet"));
        assert!(!nom_correspond("cycle.rs", "carnet"));
        // suffix glob
        assert!(nom_correspond("cycle.rs", "*.rs"));
        assert!(!nom_correspond("cycle.rss", "*.rs"));
        assert!(!nom_correspond("cycle.py", "*.rs"));
        // prefix glob
        assert!(nom_correspond("carnet.json.tmp", "carnet*"));
        // middle glob, anchored both ends
        assert!(nom_correspond("run-20260702.jsonl", "run-*.jsonl"));
        assert!(!nom_correspond("xrun-20260702.jsonl", "run-*.jsonl"));
        // multiple segments
        assert!(nom_correspond("web_deep_search.rs", "web*search*"));
    }

    #[tokio::test]
    async fn grep_contenu_retourne_chemin_ligne() {
        let dir = std::env::temp_dir().join(format!("fsearch-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("target")).unwrap();
        std::fs::write(dir.join("a.rs"), "fn main() {\n    let ruche = 42;\n}\n").unwrap();
        std::fs::write(dir.join("b.txt"), "rien ici\n").unwrap();
        // noise dir must be skipped even if it matches
        std::fs::write(dir.join("target").join("c.rs"), "let ruche = 0;").unwrap();

        let t = FileSearch;
        let r = t
            .executer(
                serde_json::json!({"path": dir.display().to_string(), "pattern": "*.rs", "content": "ruche"}),
                &ContextExecution::default(),
            )
            .await
            .unwrap();
        assert!(r.success);
        assert!(r.output.contains("a.rs:2:"), "{}", r.output);
        assert!(!r.output.contains("c.rs"), "target/ must be skipped: {}", r.output);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
