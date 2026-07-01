use crate::abeille::{Abeille, ContextExecution, NiveauDanger, ResultatAbeille};
use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::SystemTime;

static FILE_READ_STATES: OnceLock<Mutex<HashMap<PathBuf, SystemTime>>> = OnceLock::new();

fn read_states() -> &'static Mutex<HashMap<PathBuf, SystemTime>> {
    FILE_READ_STATES.get_or_init(|| Mutex::new(HashMap::new()))
}

fn stable_path(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn modified_at(path: &Path) -> std::io::Result<SystemTime> {
    std::fs::metadata(path)?.modified()
}

fn remember_read(path: &Path) {
    if let Ok(modified) = modified_at(path) {
        if let Ok(mut states) = read_states().lock() {
            states.insert(stable_path(path), modified);
        }
    }
}

fn check_timestamp_lock(path: &Path) -> Result<Option<String>> {
    let current = modified_at(path)?;
    let key = stable_path(path);
    let Some(previous) = read_states()
        .lock()
        .ok()
        .and_then(|states| states.get(&key).copied())
    else {
        return Ok(Some(
            "No prior file_read recorded for this file; read-before-write not enforced."
                .to_string(),
        ));
    };

    if current != previous {
        anyhow::bail!(
            "Edit refused: file has changed since the last file_read in this session."
        );
    }
    Ok(None)
}

fn quote_normalized(c: char) -> char {
    match c {
        '\'' | '"' | '`' | '‘' | '’' | '“' | '”' | '«' | '»' => '"',
        _ => c,
    }
}

fn normalized_with_spans(input: &str) -> (String, Vec<(usize, usize)>) {
    let mut normalized = String::new();
    let mut spans = Vec::new();
    let mut pending_space: Option<usize> = None;
    let mut last_space_end = 0usize;

    for (idx, c) in input.char_indices() {
        let end = idx + c.len_utf8();
        if c.is_whitespace() {
            if pending_space.is_none() {
                pending_space = Some(idx);
            }
            last_space_end = end;
            continue;
        }
        if let Some(start) = pending_space.take() {
            normalized.push(' ');
            spans.push((start, last_space_end));
        }
        normalized.push(quote_normalized(c).to_ascii_lowercase());
        spans.push((idx, end));
    }
    if let Some(start) = pending_space.take() {
        normalized.push(' ');
        spans.push((start, last_space_end));
    }
    (normalized, spans)
}

fn fuzzy_occurrences(source: &str, target: &str) -> Vec<(usize, usize)> {
    let (source_norm, spans) = normalized_with_spans(source);
    let (target_norm, _) = normalized_with_spans(target);
    let target_norm = target_norm.trim();
    if target_norm.is_empty() {
        return Vec::new();
    }

    let mut out = Vec::new();
    let mut search_from = 0usize;
    while let Some(rel) = source_norm[search_from..].find(target_norm) {
        let start_norm = search_from + rel;
        let end_norm = start_norm + target_norm.len();
        let start_chars = source_norm[..start_norm].chars().count();
        let end_chars = source_norm[..end_norm].chars().count();
        if start_chars < spans.len() && end_chars > start_chars && end_chars <= spans.len() {
            out.push((spans[start_chars].0, spans[end_chars - 1].1));
        }
        search_from = end_norm;
    }
    out
}

fn fuzzy_replace(
    source: &str,
    old: &str,
    new: &str,
    replace_all: bool,
) -> std::result::Result<(String, usize, bool), String> {
    let exact_count = source.matches(old).count();
    if exact_count > 0 {
        if exact_count > 1 && !replace_all {
            return Err(format!(
                "old_string found {exact_count}x - add more context to make it unique, or set replace_all=true"
            ));
        }
        let updated = if replace_all {
            source.replace(old, new)
        } else {
            source.replacen(old, new, 1)
        };
        return Ok((updated, if replace_all { exact_count } else { 1 }, false));
    }

    let occurrences = fuzzy_occurrences(source, old);
    if occurrences.is_empty() {
        return Err(
            "old_string not found, even after whitespace/quote normalization".to_string(),
        );
    }
    if occurrences.len() > 1 && !replace_all {
        return Err(format!(
            "old_string found {}x (fuzzy) - add more context to make it unique, or set replace_all=true",
            occurrences.len()
        ));
    }

    let mut updated = String::new();
    let mut last = 0usize;
    let selected = if replace_all {
        occurrences.as_slice()
    } else {
        &occurrences[..1]
    };
    for (start, end) in selected {
        updated.push_str(&source[last..*start]);
        updated.push_str(new);
        last = *end;
    }
    updated.push_str(&source[last..]);
    Ok((updated, selected.len(), true))
}

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
                        "Failed to create directories: {}",
                        e
                    )));
                }
            }
        }

        // Atomic write (tmp + rename): a crash mid-write must never leave a
        // half-written file behind.
        let tmp = path.with_extension(format!(
            "{}.tmp",
            path.extension().and_then(|e| e.to_str()).unwrap_or("laruche")
        ));
        let ecriture = std::fs::write(&tmp, content)
            .and_then(|()| std::fs::rename(&tmp, path));
        match ecriture {
            Ok(()) => Ok(ResultatAbeille::ok(format!(
                "File written successfully: {} ({} bytes)",
                path_str,
                content.len()
            ))),
            Err(e) => {
                let _ = std::fs::remove_file(&tmp);
                Ok(ResultatAbeille::err(format!("Failed to write file: {}", e)))
            }
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
        "Read a file and return its content WITH line numbers. For large files, use `offset` \
         (1-based start line) and `limit` (line count) to read a specific range."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "The file path to read" },
                "offset": { "type": "integer", "description": "Start line (1-based), optional" },
                "limit": { "type": "integer", "description": "Number of lines to read, optional" }
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
            return Ok(ResultatAbeille::err(format!("Not a file: {}", path_str)));
        }

        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => return Ok(ResultatAbeille::err(format!("Failed to read file: {}", e))),
        };
        remember_read(path);

        let offset = args["offset"].as_u64().map(|o| o.max(1) as usize);
        let limit = args["limit"].as_u64().map(|l| l as usize);
        let lines: Vec<&str> = content.lines().collect();
        let total = lines.len();

        // Explicit range, otherwise auto-range for large files (>1500 lines) to avoid blowing up the context.
        let (start, count) = match (offset, limit) {
            (Some(o), Some(l)) => (o - 1, l),
            (Some(o), None) => (o - 1, 2000),
            (None, _) if total > 1500 => (0, 1500),
            (None, _) => (0, total),
        };
        if total > 0 && start >= total {
            return Ok(ResultatAbeille::err(format!(
                "offset {} is past the end of the file ({} lines)",
                start + 1,
                total
            )));
        }
        let end = (start + count).min(total);
        let mut out = String::new();
        for (i, line) in lines[start..end].iter().enumerate() {
            // A single minified/generated line can weigh 500k chars and blow up the
            // context: cap per line (the model can re-read via a narrower range).
            if line.chars().count() > 2000 {
                let tronquee: String = line.chars().take(2000).collect();
                out.push_str(&format!(
                    "{:>6}\t{tronquee} …[line truncated: {} chars]\n",
                    start + i + 1,
                    line.chars().count()
                ));
            } else {
                out.push_str(&format!("{:>6}\t{}\n", start + i + 1, line));
            }
        }
        if end < total {
            out.push_str(&format!(
                "\n... ({} lines remaining - use offset={} to read more)",
                total - end,
                end + 1
            ));
        }
        Ok(ResultatAbeille::ok(out))
    }
}

/// Targeted file edit via EXACT string replacement (patch).
pub struct FileEdit;

#[async_trait]
impl Abeille for FileEdit {
    fn nom(&self) -> &str {
        "file_edit"
    }

    fn description(&self) -> &str {
        "Edit a file by replacing an EXACT string with another (targeted patch, no full rewrite). \
         `old_string` must be unique in the file (fails otherwise) unless `replace_all`=true. \
         Prefer this over file_write for precise code edits."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "File to edit" },
                "old_string": { "type": "string", "description": "Exact text to replace (must be unique)" },
                "new_string": { "type": "string", "description": "Replacement text" },
                "replace_all": { "type": "boolean", "description": "Replace all occurrences (default false)" }
            },
            "required": ["path", "old_string", "new_string"]
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
            .ok_or_else(|| anyhow::anyhow!("Missing 'path'"))?;
        let old = args["old_string"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'old_string'"))?;
        let new = args["new_string"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'new_string'"))?;
        let replace_all = args["replace_all"].as_bool().unwrap_or(false);

        if old.is_empty() {
            return Ok(ResultatAbeille::err("old_string is empty"));
        }
        let path = Path::new(path_str);
        let timestamp_warning = match check_timestamp_lock(path) {
            Ok(warning) => warning,
            Err(e) => return Ok(ResultatAbeille::err(e.to_string())),
        };
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => return Ok(ResultatAbeille::err(format!("Cannot read file: {e}"))),
        };
        let (updated, count, fuzzy) = match fuzzy_replace(&content, old, new, replace_all) {
            Ok(result) => result,
            Err(e) => return Ok(ResultatAbeille::err(e)),
        };
        if count == 0 {
            return Ok(ResultatAbeille::err(
                "old_string not found - copy the exact text (indentation included)",
            ));
        }
        if count > 1 && !replace_all {
            return Ok(ResultatAbeille::err(format!(
                "old_string found {count}x - add more context to make it unique, or set replace_all=true"
            )));
        }
        let _unused_updated = if replace_all {
            content.replace(old, new)
        } else {
            content.replacen(old, new, 1)
        };
        let _ = fuzzy;
        let _ = timestamp_warning;
        let write_result = std::fs::write(path, &updated);
        remember_read(path);
        match write_result {
            Ok(()) => Ok(ResultatAbeille::ok(format!(
                "Edited: {path_str} ({} replacement(s))",
                if replace_all { count } else { 1 }
            ))),
            Err(e) => Ok(ResultatAbeille::err(format!("Cannot write file: {e}"))),
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
        "List files and directories at the given path (sorted: directories first, with \
         file sizes). Set `recursive` to true for a tree view (build/VCS dirs skipped)."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "The directory path to list"
                },
                "recursive": {
                    "type": "boolean",
                    "description": "Tree view of subdirectories (default false)"
                },
                "max_depth": {
                    "type": "integer",
                    "description": "Recursion depth when recursive=true (default 3)"
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

        let recursive = args["recursive"].as_bool().unwrap_or(false);
        let max_depth = if recursive {
            args["max_depth"].as_u64().unwrap_or(3) as usize
        } else {
            0
        };
        let mut entries = Vec::new();
        lister_trie(path, 0, max_depth, &mut entries);
        let tronque = entries.len() > 300;
        entries.truncate(300);
        if tronque {
            entries.push("... (truncated at 300 entries - narrow the path or depth)".into());
        }

        if entries.is_empty() {
            Ok(ResultatAbeille::ok("(empty directory)"))
        } else {
            Ok(ResultatAbeille::ok(entries.join("\n")))
        }
    }
}

/// Noise directories skipped during recursive listings/searches (build artifacts,
/// VCS, dependency caches): pure pollution for an agent reading a project.
pub(crate) const DOSSIERS_IGNORES: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    "__pycache__",
    ".venv",
    "venv",
    "dist",
    "build",
    ".next",
    ".cache",
];

fn taille_humaine(octets: u64) -> String {
    if octets >= 1_048_576 {
        format!("{:.1} MB", octets as f64 / 1_048_576.0)
    } else if octets >= 1024 {
        format!("{:.1} KB", octets as f64 / 1024.0)
    } else {
        format!("{octets} B")
    }
}

/// Sorted listing (directories first, alphabetical), sizes on files, bounded
/// recursion with noise dirs skipped.
fn lister_trie(dir: &Path, depth: usize, max_depth: usize, out: &mut Vec<String>) {
    if out.len() >= 320 {
        return;
    }
    let Ok(reader) = std::fs::read_dir(dir) else {
        return;
    };
    let mut dossiers: Vec<(String, PathBuf)> = Vec::new();
    let mut fichiers: Vec<(String, u64)> = Vec::new();
    for entry in reader.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        let p = entry.path();
        if p.is_dir() {
            dossiers.push((name, p));
        } else {
            let taille = entry.metadata().map(|m| m.len()).unwrap_or(0);
            fichiers.push((name, taille));
        }
    }
    dossiers.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
    fichiers.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
    let indent = "  ".repeat(depth);
    for (name, p) in &dossiers {
        let ignore = DOSSIERS_IGNORES.contains(&name.as_str());
        out.push(format!(
            "{indent}[DIR] {name}/{}",
            if ignore && depth < max_depth { " (skipped)" } else { "" }
        ));
        if depth < max_depth && !ignore && !name.starts_with('.') {
            lister_trie(p, depth + 1, max_depth, out);
        }
    }
    for (name, taille) in &fichiers {
        out.push(format!("{indent}[FILE] {name} ({})", taille_humaine(*taille)));
    }
}
