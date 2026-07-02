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
    /// Result delivery channel (e.g. `telegram:123`). `None` -> home channel.
    #[serde(default)]
    pub channel: Option<String>,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub last_run: Option<DateTime<Utc>>,
    pub run_count: u32,
    pub last_state: Option<String>,
    /// Poll interval in seconds. `None` = type default (file/log 10s, url 60s).
    #[serde(default)]
    pub interval_secs: Option<u64>,
    /// Minimum seconds between two FIRES. `None` = type default (url 900s, others 0).
    #[serde(default)]
    pub cooldown_secs: Option<u64>,
    /// Sustained mode: instead of firing only on a state TRANSITION, re-fires every
    /// cooldown while the observed situation lasts ("remind me every 20 min while
    /// the site is down"). Requires a semantic `condition` (the LLM gate decides
    /// each time with the current state and datetime in hand).
    #[serde(default)]
    pub sustained: bool,
}

impl Watcher {
    /// Effective poll interval (floored at 5s so a typo cannot hammer a target).
    pub fn interval_effectif(&self) -> u64 {
        self.interval_secs
            .unwrap_or(match self.watcher_type {
                WatcherType::Url => 60,
                _ => 10,
            })
            .max(5)
    }

    /// Effective fire cooldown. URLs default to 15 min (a flapping page must not
    /// spam runs and notifications); file/log transitions fire freely by default.
    pub fn cooldown_effectif(&self) -> u64 {
        self.cooldown_secs.unwrap_or(match self.watcher_type {
            WatcherType::Url => 900,
            _ => 0,
        })
    }
}

/// A watcher event ready for dispatch. When `semantique` is true the dispatcher
/// must run the LLM condition gate on `condition` before launching `prompt`.
#[derive(Debug, Clone)]
pub struct Declenchement {
    pub id: Uuid,
    pub name: String,
    pub prompt: String,
    pub contexte: String,
    pub condition: String,
    pub semantique: bool,
}

pub struct WatchersRegistry {
    watchers: HashMap<Uuid, Watcher>,
    file_path: PathBuf,
    /// Last poll instant per watcher (in-memory only): implements the per-watcher
    /// interval on top of the dispatcher's fixed tick.
    derniers_polls: HashMap<Uuid, DateTime<Utc>>,
}

impl WatchersRegistry {
    pub fn new(file_path: &Path) -> Self {
        let mut registry = Self {
            watchers: HashMap::new(),
            file_path: file_path.to_path_buf(),
            derniers_polls: HashMap::new(),
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

    /// Updates the EDITABLE fields of a watcher (id/run_count/created_at/last_state
    /// preserved). A `None` argument means the field is unchanged; for model/profile_id,
    /// `Some(None)` clears the value.
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
        channel: Option<Option<String>>,
        interval_secs: Option<Option<u64>>,
        cooldown_secs: Option<Option<u64>>,
        sustained: Option<bool>,
    ) -> bool {
        if let Some(w) = self.watchers.get_mut(id) {
            if let Some(v) = channel {
                w.channel = v;
            }
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
            if let Some(v) = interval_secs {
                w.interval_secs = v;
            }
            if let Some(v) = cooldown_secs {
                w.cooldown_secs = v;
            }
            if let Some(v) = sustained {
                w.sustained = v;
            }
            let _ = self.save();
            true
        } else {
            false
        }
    }

    pub async fn check_triggered_watchers(&mut self) -> Vec<Declenchement> {
        let now = Utc::now();
        let mut triggered = Vec::new();
        let mut needs_save = false;
        let mut updates = Vec::new();
        let mut polled: Vec<Uuid> = Vec::new();

        for watcher in self.watchers.values() {
            if !watcher.active {
                continue;
            }
            // Per-watcher interval on top of the dispatcher tick.
            let du = self
                .derniers_polls
                .get(&watcher.id)
                .map(|t| (now - *t).num_seconds())
                .unwrap_or(i64::MAX);
            if du < watcher.interval_effectif() as i64 {
                continue;
            }
            polled.push(watcher.id);

            match evaluate_watcher(watcher, now).await {
                Ok((transition, new_state, desc)) => {
                    // Fire cooldown, anchored on the last actual fire.
                    let pret = watcher
                        .last_run
                        .map(|lr| (now - lr).num_seconds() >= watcher.cooldown_effectif() as i64)
                        .unwrap_or(true);
                    // Sustained: while a situation lasts, keep offering it to the
                    // semantic gate every cooldown. Requires a condition (otherwise
                    // it would degenerate into a cron) and an established baseline.
                    let soutenu = watcher.sustained
                        && watcher.last_state.is_some()
                        && !watcher.condition.trim().is_empty()
                        && !desc.is_empty();
                    if pret && (transition || soutenu) {
                        triggered.push(Declenchement {
                            id: watcher.id,
                            name: watcher.name.clone(),
                            prompt: watcher.prompt.clone(),
                            contexte: format!(
                                "Watcher '{}' ({:?}) on '{}': {}",
                                watcher.name, watcher.watcher_type, watcher.target, desc
                            ),
                            condition: watcher.condition.clone(),
                            // Log conditions are already enforced mechanically
                            // (substring on the new content); file/url conditions
                            // are semantic and go through the LLM gate.
                            semantique: !watcher.condition.trim().is_empty()
                                && watcher.watcher_type != WatcherType::Log,
                        });
                    }
                    if new_state != watcher.last_state {
                        updates.push((watcher.id, new_state));
                    }
                }
                Err(e) => {
                    tracing::error!("Error evaluating watcher {}: {}", watcher.name, e);
                }
            }
        }

        for d in &triggered {
            if let Some(w) = self.watchers.get_mut(&d.id) {
                w.last_run = Some(now);
                w.run_count += 1;
                needs_save = true;
            }
        }
        for (id, new_state) in updates {
            if let Some(w) = self.watchers.get_mut(&id) {
                w.last_state = new_state;
                needs_save = true;
            }
        }
        for id in polled {
            self.derniers_polls.insert(id, now);
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

/// Pure FILE transition: `(mtime observed or None when absent, previous state)`
/// -> `(transition_fired, new_state, situation description)`. States: `absent` |
/// `present:<mtime>` (a bare legacy mtime is read as present). Appearance,
/// deletion and modification all fire; the first observation only sets the
/// baseline.
pub fn transition_fichier(
    mtime: Option<String>,
    last: Option<&str>,
) -> (bool, Option<String>, String) {
    let etat = match &mtime {
        Some(m) => format!("present:{m}"),
        None => "absent".to_string(),
    };
    let Some(last) = last else {
        let desc = match &mtime {
            Some(m) => format!("file present (baseline set, modified {m})"),
            None => "file absent (baseline set)".to_string(),
        };
        return (false, Some(etat), desc);
    };
    let last_present = last != "absent";
    let last_mtime = last.strip_prefix("present:").unwrap_or(last);
    match (&mtime, last_present) {
        (Some(m), false) => (true, Some(etat), format!("file APPEARED (modified {m})")),
        (None, true) => (true, Some(etat), "file was DELETED".to_string()),
        (Some(m), true) if m != last_mtime => {
            (true, Some(etat), format!("file MODIFIED (now {m})"))
        }
        (Some(m), true) => (false, Some(etat), format!("file present, unchanged since {m}")),
        (None, false) => (false, Some(etat), "file still absent".to_string()),
    }
}

/// Pure URL transition: `(hash of the page's extracted text, or None when the
/// site is unreachable/5xx, previous state, now)` -> `(transition_fired,
/// new_state, situation description)`. States: `up:<hash>` | `down:<since>` (a
/// bare legacy hash is read as up). Going down, coming back up, and content
/// changes all fire; while down the state keeps its original `since` so the
/// description carries the outage duration.
pub fn transition_url(
    hash_texte: Option<u64>,
    last: Option<&str>,
    now: DateTime<Utc>,
) -> (bool, Option<String>, String) {
    let (last_up_hash, last_down_since): (Option<&str>, Option<&str>) = match last {
        None => (None, None),
        Some(l) if l.starts_with("down:") => (None, Some(&l[5..])),
        Some(l) => (Some(l.strip_prefix("up:").unwrap_or(l)), None),
    };
    let minutes_down = |since: &str| {
        chrono::DateTime::parse_from_rfc3339(since)
            .map(|d| (now - d.with_timezone(&Utc)).num_minutes())
            .unwrap_or(0)
    };
    match hash_texte {
        Some(h) => {
            let etat = format!("up:{h}");
            if last.is_none() {
                return (false, Some(etat), "site UP (baseline set)".to_string());
            }
            if let Some(since) = last_down_since {
                let m = minutes_down(since);
                return (true, Some(etat), format!("site BACK UP after {m} min down"));
            }
            if last_up_hash != Some(format!("{h}").as_str()) {
                (true, Some(etat), "page CONTENT CHANGED".to_string())
            } else {
                (false, Some(etat), "site up, content unchanged".to_string())
            }
        }
        None => {
            if let Some(since) = last_down_since {
                let m = minutes_down(since);
                // Keep the original `since`: the outage duration keeps growing.
                (
                    false,
                    Some(format!("down:{since}")),
                    format!("site still DOWN since {since} ({m} min)"),
                )
            } else if last.is_none() {
                (
                    false,
                    Some(format!("down:{}", now.to_rfc3339())),
                    "site DOWN (baseline set)".to_string(),
                )
            } else {
                (
                    true,
                    Some(format!("down:{}", now.to_rfc3339())),
                    "site went DOWN".to_string(),
                )
            }
        }
    }
}

/// Strips scripts, styles and tags from an HTML page and collapses whitespace:
/// the hash is computed on STABLE text, so rotating tokens/timestamps buried in
/// markup no longer make every poll look like a change.
pub fn extraire_texte(html: &str) -> String {
    let mut out = String::with_capacity(html.len() / 4);
    let bytes = html.as_bytes();
    // ASCII lowercase copy for case-insensitive tag matching: same byte length
    // as the original by construction (full Unicode lowercasing is not).
    let lower: Vec<u8> = bytes.iter().map(|b| b.to_ascii_lowercase()).collect();
    let cherche = |depuis: usize, motif: &[u8]| -> Option<usize> {
        lower[depuis..]
            .windows(motif.len())
            .position(|w| w == motif)
            .map(|p| depuis + p)
    };
    let mut i = 0;
    let mut dans_tag = false;
    while i < bytes.len() {
        if !dans_tag && bytes[i] == b'<' {
            // Skip <script>/<style> blocks entirely (their content is noise).
            let avant = i;
            for (ouvre, ferme) in [
                (b"<script".as_slice(), b"</script>".as_slice()),
                (b"<style".as_slice(), b"</style>".as_slice()),
            ] {
                if lower[i..].starts_with(ouvre) {
                    match cherche(i, ferme) {
                        Some(fin) => i = fin + ferme.len(),
                        None => i = bytes.len(),
                    }
                    break;
                }
            }
            if i != avant {
                continue; // a block was skipped: re-examine from the new position
            }
            dans_tag = true;
            i += 1;
            continue;
        }
        if dans_tag {
            if bytes[i] == b'>' {
                dans_tag = false;
                out.push(' ');
            }
            i += 1;
            continue;
        }
        // Copy one full UTF-8 char.
        let ch_len = match bytes[i] {
            b if b < 0x80 => 1,
            b if b >= 0xF0 => 4,
            b if b >= 0xE0 => 3,
            _ => 2,
        };
        out.push_str(&html[i..(i + ch_len).min(html.len())]);
        i += ch_len;
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

async fn evaluate_watcher(
    watcher: &Watcher,
    now: DateTime<Utc>,
) -> Result<(bool, Option<String>, String)> {
    match watcher.watcher_type {
        WatcherType::File => {
            let mtime = std::fs::metadata(&watcher.target)
                .and_then(|m| m.modified())
                .ok()
                .map(|m| {
                    let t: chrono::DateTime<chrono::Utc> = m.into();
                    t.to_rfc3339()
                });
            Ok(transition_fichier(mtime, watcher.last_state.as_deref()))
        }
        WatcherType::Url => {
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()?;
            // Unreachable, timeout or 5xx = DOWN; the hash is computed on the
            // extracted text so dynamic markup does not flap the state.
            let hash = match client.get(&watcher.target).send().await {
                Ok(resp) if resp.status().is_server_error() => None,
                Ok(resp) => match resp.text().await {
                    Ok(html) => {
                        let mut hasher = DefaultHasher::new();
                        extraire_texte(&html).hash(&mut hasher);
                        Some(hasher.finish())
                    }
                    Err(_) => None,
                },
                Err(_) => None,
            };
            Ok(transition_url(hash, watcher.last_state.as_deref(), now))
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
                return Ok((
                    false,
                    Some(current_len.to_string()),
                    "log unchanged".to_string(),
                ));
            }

            file.seek(SeekFrom::Start(last_offset))?;
            let mut content = String::new();
            file.read_to_string(&mut content)?;

            let triggered = content.contains(&watcher.condition);
            let desc = if triggered {
                let extrait: String = content
                    .lines()
                    .filter(|l| l.contains(&watcher.condition))
                    .take(3)
                    .collect::<Vec<_>>()
                    .join(" | ");
                format!(
                    "new log content matches '{}': {}",
                    watcher.condition,
                    &extrait.chars().take(300).collect::<String>()
                )
            } else {
                "new log content (no match)".to_string()
            };
            Ok((triggered, Some(current_len.to_string()), desc))
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
            channel: None,
            active: true,
            created_at: Utc::now(),
            last_run: None,
            run_count: 0,
            last_state: None,
            model: None,
            profile_id: None,
            interval_secs: None,
            cooldown_secs: None,
            sustained: false,
        };

        // First run initializes state without triggering
        let (triggered, state, _) = evaluate_watcher(&watcher, Utc::now()).await.unwrap();
        assert!(!triggered);
        assert!(state.is_some());

        // Second run with same file should not trigger
        let mut watcher2 = watcher.clone();
        watcher2.last_state = state.clone();
        let (triggered2, state2, _) = evaluate_watcher(&watcher2, Utc::now()).await.unwrap();
        assert!(!triggered2);
        assert_eq!(state, state2);

        // Wait a little to ensure modification time is different
        // In actual tests, modifying time might have low resolution on some file systems.
        // We'll trust standard fs tests for now, but sleep could be used if it flakes.
    }

    #[test]
    fn fichier_apparition_suppression_modification() {
        // Baseline on an absent file, then it appears.
        let (t0, s0, _) = transition_fichier(None, None);
        assert!(!t0);
        assert_eq!(s0.as_deref(), Some("absent"));
        let (t1, s1, d1) = transition_fichier(Some("2026-07-02T10:00:00Z".into()), s0.as_deref());
        assert!(t1, "appearance must fire");
        assert!(d1.contains("APPEARED"));
        // Modification fires; unchanged does not.
        let (t2, s2, _) = transition_fichier(Some("2026-07-02T11:00:00Z".into()), s1.as_deref());
        assert!(t2);
        let (t3, s3, _) = transition_fichier(Some("2026-07-02T11:00:00Z".into()), s2.as_deref());
        assert!(!t3);
        // Deletion fires.
        let (t4, _, d4) = transition_fichier(None, s3.as_deref());
        assert!(t4);
        assert!(d4.contains("DELETED"));
        // Legacy state (bare mtime, pre-upgrade watchers.json) reads as present.
        let (t5, _, _) = transition_fichier(None, Some("2026-07-01T09:00:00+00:00"));
        assert!(t5, "legacy present state -> deletion fires");
    }

    #[test]
    fn url_down_puis_retour_et_contenu() {
        let now = Utc::now();
        // Baseline up, then goes down: fires once, then stays silent while down
        // (the sustained mode re-offers it to the semantic gate instead).
        let (_, s0, _) = transition_url(Some(42), None, now);
        let (t1, s1, d1) = transition_url(None, s0.as_deref(), now);
        assert!(t1);
        assert!(d1.contains("DOWN"));
        let (t2, s2, d2) = transition_url(None, s1.as_deref(), now + chrono::Duration::minutes(12));
        assert!(!t2, "still down = no new transition");
        assert_eq!(s1, s2, "outage keeps its original since");
        assert!(d2.contains("12 min"), "{d2}");
        // Back up fires with the outage duration.
        let (t3, s3, d3) = transition_url(Some(42), s2.as_deref(), now + chrono::Duration::minutes(15));
        assert!(t3);
        assert!(d3.contains("BACK UP"));
        // Content change fires; same content does not.
        let (t4, s4, _) = transition_url(Some(43), s3.as_deref(), now);
        assert!(t4);
        let (t5, _, _) = transition_url(Some(43), s4.as_deref(), now);
        assert!(!t5);
        // Legacy state (bare hash) reads as up with that hash.
        let (t6, _, _) = transition_url(Some(7), Some("7"), now);
        assert!(!t6);
    }

    #[test]
    fn extraction_texte_stabilise_le_hash() {
        let page_a = "<html><head><script>var t=123456;</script><style>.x{}</style></head><body><h1>Statut</h1><p>Tout va bien</p></body></html>";
        let page_b = "<html><head><script>var t=999999;</script><style>.y{}</style></head><body><h1>Statut</h1><p>Tout   va bien</p></body></html>";
        assert_eq!(extraire_texte(page_a), extraire_texte(page_b));
        assert!(extraire_texte(page_a).contains("Tout va bien"));
        let page_c = "<body><p>Panne en cours</p></body>";
        assert_ne!(extraire_texte(page_a), extraire_texte(page_c));
    }

    #[test]
    fn intervalles_et_cooldowns_par_defaut() {
        let mut w = Watcher {
            id: Uuid::new_v4(),
            name: "t".into(),
            watcher_type: WatcherType::Url,
            target: String::new(),
            condition: String::new(),
            prompt: String::new(),
            channel: None,
            active: true,
            created_at: Utc::now(),
            last_run: None,
            run_count: 0,
            last_state: None,
            model: None,
            profile_id: None,
            interval_secs: None,
            cooldown_secs: None,
            sustained: false,
        };
        assert_eq!(w.interval_effectif(), 60);
        assert_eq!(w.cooldown_effectif(), 900);
        w.watcher_type = WatcherType::File;
        assert_eq!(w.interval_effectif(), 10);
        assert_eq!(w.cooldown_effectif(), 0);
        w.interval_secs = Some(1); // floored: no hammering
        assert_eq!(w.interval_effectif(), 5);
        w.cooldown_secs = Some(1200);
        assert_eq!(w.cooldown_effectif(), 1200);
    }
}
