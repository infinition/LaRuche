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
            "Aucun file_read anterieur enregistre pour ce fichier; garde read-before-write best-effort."
                .to_string(),
        ));
    };

    if current != previous {
        anyhow::bail!(
            "Refus d'edition: le fichier a change depuis le dernier file_read de la session."
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
                "old_string trouve {exact_count}x - ajoute du contexte pour le rendre unique, ou replace_all=true"
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
            "old_string introuvable, meme apres normalisation espaces/guillemets".to_string(),
        );
    }
    if occurrences.len() > 1 && !replace_all {
        return Err(format!(
            "old_string trouve {}x en fuzzy - ajoute du contexte pour le rendre unique, ou replace_all=true",
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

        match std::fs::write(path, content) {
            Ok(()) => Ok(ResultatAbeille::ok(format!(
                "File written successfully: {} ({} bytes)",
                path_str,
                content.len()
            ))),
            Err(e) => Ok(ResultatAbeille::err(format!("Failed to write file: {}", e))),
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
        "Lit un fichier et renvoie son contenu AVEC numéros de ligne. Pour un gros fichier, \
         utilise `offset` (ligne de départ, 1-based) et `limit` (nombre de lignes) pour lire une plage."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "The file path to read" },
                "offset": { "type": "integer", "description": "Ligne de départ (1-based), optionnel" },
                "limit": { "type": "integer", "description": "Nombre de lignes à lire, optionnel" }
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

        // Plage explicite, sinon auto-plage si gros fichier (>1500 lignes) pour ne pas exploser le contexte.
        let (start, count) = match (offset, limit) {
            (Some(o), Some(l)) => (o - 1, l),
            (Some(o), None) => (o - 1, 2000),
            (None, _) if total > 1500 => (0, 1500),
            (None, _) => (0, total),
        };
        if total > 0 && start >= total {
            return Ok(ResultatAbeille::err(format!(
                "offset {} dépasse le fichier ({} lignes)",
                start + 1,
                total
            )));
        }
        let end = (start + count).min(total);
        let mut out = String::new();
        for (i, line) in lines[start..end].iter().enumerate() {
            out.push_str(&format!("{:>6}\t{}\n", start + i + 1, line));
        }
        if end < total {
            out.push_str(&format!(
                "\n... ({} lignes restantes — utilise offset={} pour lire la suite)",
                total - end,
                end + 1
            ));
        }
        Ok(ResultatAbeille::ok(out))
    }
}

/// Édition ciblée d'un fichier par remplacement de chaîne EXACTE (patch).
pub struct FileEdit;

#[async_trait]
impl Abeille for FileEdit {
    fn nom(&self) -> &str {
        "file_edit"
    }

    fn description(&self) -> &str {
        "Édite un fichier en remplaçant une chaîne EXACTE par une autre (patch ciblé, sans \
         réécrire tout le fichier). `old_string` doit être unique dans le fichier (sinon échec), \
         sauf si `replace_all`=true. Idéal pour modifier du code précisément."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Fichier à éditer" },
                "old_string": { "type": "string", "description": "Texte exact à remplacer (doit être unique)" },
                "new_string": { "type": "string", "description": "Texte de remplacement" },
                "replace_all": { "type": "boolean", "description": "Remplacer toutes les occurrences (défaut false)" }
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
            return Ok(ResultatAbeille::err("old_string vide"));
        }
        let path = Path::new(path_str);
        let timestamp_warning = match check_timestamp_lock(path) {
            Ok(warning) => warning,
            Err(e) => return Ok(ResultatAbeille::err(e.to_string())),
        };
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => return Ok(ResultatAbeille::err(format!("Lecture impossible: {e}"))),
        };
        let (updated, count, fuzzy) = match fuzzy_replace(&content, old, new, replace_all) {
            Ok(result) => result,
            Err(e) => return Ok(ResultatAbeille::err(e)),
        };
        if count == 0 {
            return Ok(ResultatAbeille::err(
                "old_string introuvable — copie le texte exact (indentation comprise)",
            ));
        }
        if count > 1 && !replace_all {
            return Ok(ResultatAbeille::err(format!(
                "old_string trouvé {count}× — ajoute du contexte pour le rendre unique, ou replace_all=true"
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
                "Édité: {path_str} ({} remplacement(s))",
                if replace_all { count } else { 1 }
            ))),
            Err(e) => Ok(ResultatAbeille::err(format!("Écriture impossible: {e}"))),
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
