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
    /// Compiled condition (deterministic predicate tree, see [`Regle`]). When set
    /// it REPLACES the transition+condition logic: the tree is evaluated at every
    /// poll for free, fires on Vrai (cooldown-gated), and only `llm_check` leaves
    /// reach the LLM gate. A state rule (down_depuis_min...) naturally re-fires
    /// every cooldown while true, so `sustained` is built in.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub regles: Option<Regle>,
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

/// A watcher event ready for dispatch. `question_llm` (compiled-rules path) or
/// `semantique` (legacy text-condition path) tell the dispatcher whether an LLM
/// gate call must approve the event before launching `prompt`.
#[derive(Debug, Clone)]
pub struct Declenchement {
    pub id: Uuid,
    pub name: String,
    pub prompt: String,
    pub contexte: String,
    pub condition: String,
    pub semantique: bool,
    /// The residual question from a compiled-rules evaluation whose deterministic
    /// prefix passed but which carries `llm_check` leaves. None = fire directly.
    pub question_llm: Option<String>,
}

// ═══════════════════ Compiled rules: deterministic predicate DSL ═══════════════════
//
// "Compile, don't interpret": the agent turns a natural-language wish into this
// expression ONCE at creation (guided by the watcher-architecte skill); every
// poll then evaluates it mechanically for free. `llm_check` is the only leaf
// that costs an LLM call, and only when the deterministic prefix already passed
// (short-circuit), so "Tuesday AND down for 10 min" never touches a model.

/// What the poll actually observed, handed to the rules evaluator.
#[derive(Debug, Clone, Default)]
pub struct Observation {
    pub evenement: Evenement,
    /// Minutes since the target went down (url watchers, while down).
    pub minutes_down: Option<u64>,
    /// Fresh content for `contient`/LLM context: extracted page text (url),
    /// new log lines (log). Empty for file watchers.
    pub nouveau_contenu: String,
    /// Current file size in bytes (file watchers).
    pub taille_octets: Option<u64>,
    /// Last observed HTTP status (url watchers).
    pub status_http: Option<u16>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Evenement {
    #[default]
    Rien,
    Apparu,
    Supprime,
    Modifie,
    ContenuChange,
    Down,
    ToujoursDown,
    RetourEnLigne,
    NouveauContenuLog,
}

/// Outcome of a rules evaluation. `BesoinLlm` means the deterministic prefix
/// passed and the joined question must be approved by the LLM gate.
#[derive(Debug, Clone, PartialEq)]
pub enum Verdict {
    Vrai,
    Faux,
    BesoinLlm(String),
}

/// The compiled condition tree. Serialized as JSON with an `op` tag, e.g.:
/// `{"op":"et","regles":[{"op":"jour_semaine","jours":["mar","jeu"]},
///                        {"op":"down_depuis_min","minutes":10}]}`
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Regle {
    Et { regles: Vec<Regle> },
    Ou { regles: Vec<Regle> },
    Non { regle: Box<Regle> },
    /// Days of week, en or fr 3-letter forms: mon/tue/... or lun/mar/mer/jeu/ven/sam/dim.
    JourSemaine { jours: Vec<String> },
    /// Local time window "HH:MM"-"HH:MM"; overnight windows (22:00-06:00) supported.
    HeureEntre { de: String, a: String },
    /// Inclusive local date range "YYYY-MM-DD".
    PlageDate { du: String, au: String },
    Apparu,
    Supprime,
    Modifie,
    ContenuChange,
    EstDown,
    DownDepuisMin { minutes: u64 },
    RetourEnLigne,
    /// Case-insensitive match on the fresh content (page text / new log lines).
    Contient { motif: String },
    TailleDepasseMo { mo: f64 },
    StatusHttp { codes: Vec<u16> },
    /// Semantic leaf: the only rule that costs an LLM call, asked AFTER the
    /// deterministic prefix passed.
    LlmCheck { question: String },
}

impl Regle {
    pub fn evaluer(&self, obs: &Observation, maintenant: &chrono::DateTime<chrono::Local>) -> Verdict {
        use Verdict::*;
        match self {
            Regle::Et { regles } => {
                let mut questions: Vec<String> = Vec::new();
                for r in regles {
                    match r.evaluer(obs, maintenant) {
                        Faux => return Faux,
                        Vrai => {}
                        BesoinLlm(q) => questions.push(q),
                    }
                }
                if questions.is_empty() {
                    Vrai
                } else {
                    BesoinLlm(questions.join(" AND ALSO "))
                }
            }
            Regle::Ou { regles } => {
                let mut questions: Vec<String> = Vec::new();
                for r in regles {
                    match r.evaluer(obs, maintenant) {
                        Vrai => return Vrai,
                        Faux => {}
                        BesoinLlm(q) => questions.push(q),
                    }
                }
                if questions.is_empty() {
                    Faux
                } else {
                    BesoinLlm(questions.join(" OR "))
                }
            }
            Regle::Non { regle } => match regle.evaluer(obs, maintenant) {
                Vrai => Faux,
                Faux => Vrai,
                BesoinLlm(q) => BesoinLlm(format!("NOT ({q})")),
            },
            Regle::JourSemaine { jours } => {
                let jour = jour_court(maintenant);
                if jours.iter().any(|j| normaliser_jour(j) == jour) {
                    Vrai
                } else {
                    Faux
                }
            }
            Regle::HeureEntre { de, a } => {
                let (Some(d), Some(f)) = (parse_hhmm(de), parse_hhmm(a)) else {
                    return Faux; // unparseable window: never true, visible in tests
                };
                let now = maintenant.format("%H:%M").to_string();
                let n = parse_hhmm(&now).unwrap_or(0);
                let dans = if d <= f { n >= d && n < f } else { n >= d || n < f };
                if dans { Vrai } else { Faux }
            }
            Regle::PlageDate { du, au } => {
                let d = maintenant.format("%Y-%m-%d").to_string();
                if d.as_str() >= du.as_str() && d.as_str() <= au.as_str() {
                    Vrai
                } else {
                    Faux
                }
            }
            Regle::Apparu => bool_verdict(obs.evenement == Evenement::Apparu),
            Regle::Supprime => bool_verdict(obs.evenement == Evenement::Supprime),
            Regle::Modifie => bool_verdict(obs.evenement == Evenement::Modifie),
            Regle::ContenuChange => bool_verdict(matches!(
                obs.evenement,
                Evenement::ContenuChange | Evenement::NouveauContenuLog
            )),
            Regle::EstDown => bool_verdict(matches!(
                obs.evenement,
                Evenement::Down | Evenement::ToujoursDown
            )),
            Regle::DownDepuisMin { minutes } => bool_verdict(
                matches!(obs.evenement, Evenement::Down | Evenement::ToujoursDown)
                    && obs.minutes_down.unwrap_or(0) >= *minutes,
            ),
            Regle::RetourEnLigne => bool_verdict(obs.evenement == Evenement::RetourEnLigne),
            Regle::Contient { motif } => bool_verdict(
                !obs.nouveau_contenu.is_empty()
                    && obs
                        .nouveau_contenu
                        .to_lowercase()
                        .contains(&motif.to_lowercase()),
            ),
            Regle::TailleDepasseMo { mo } => bool_verdict(
                obs.taille_octets
                    .map(|t| t as f64 >= mo * 1_048_576.0)
                    .unwrap_or(false),
            ),
            Regle::StatusHttp { codes } => bool_verdict(
                obs.status_http.map(|s| codes.contains(&s)).unwrap_or(false),
            ),
            Regle::LlmCheck { question } => BesoinLlm(question.clone()),
        }
    }

    /// Compact human summary for the UI bubble ("ET(jour∈[mar,jeu], down≥10min)").
    pub fn resume(&self) -> String {
        match self {
            Regle::Et { regles } => format!(
                "ET({})",
                regles.iter().map(|r| r.resume()).collect::<Vec<_>>().join(", ")
            ),
            Regle::Ou { regles } => format!(
                "OU({})",
                regles.iter().map(|r| r.resume()).collect::<Vec<_>>().join(", ")
            ),
            Regle::Non { regle } => format!("NON({})", regle.resume()),
            Regle::JourSemaine { jours } => format!("jour∈[{}]", jours.join(",")),
            Regle::HeureEntre { de, a } => format!("{de}-{a}"),
            Regle::PlageDate { du, au } => format!("{du}..{au}"),
            Regle::Apparu => "apparu".into(),
            Regle::Supprime => "supprimé".into(),
            Regle::Modifie => "modifié".into(),
            Regle::ContenuChange => "contenu≠".into(),
            Regle::EstDown => "down".into(),
            Regle::DownDepuisMin { minutes } => format!("down≥{minutes}min"),
            Regle::RetourEnLigne => "retour en ligne".into(),
            Regle::Contient { motif } => format!("contient « {motif} »"),
            Regle::TailleDepasseMo { mo } => format!("taille≥{mo}Mo"),
            Regle::StatusHttp { codes } => format!(
                "http∈[{}]",
                codes.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(",")
            ),
            Regle::LlmCheck { question } => format!("🧠« {question} »"),
        }
    }
}

fn bool_verdict(b: bool) -> Verdict {
    if b {
        Verdict::Vrai
    } else {
        Verdict::Faux
    }
}

fn jour_court(dt: &chrono::DateTime<chrono::Local>) -> &'static str {
    use chrono::Datelike;
    match dt.weekday() {
        chrono::Weekday::Mon => "mon",
        chrono::Weekday::Tue => "tue",
        chrono::Weekday::Wed => "wed",
        chrono::Weekday::Thu => "thu",
        chrono::Weekday::Fri => "fri",
        chrono::Weekday::Sat => "sat",
        chrono::Weekday::Sun => "sun",
    }
}

fn normaliser_jour(j: &str) -> &'static str {
    match j.trim().to_lowercase().as_str() {
        "mon" | "lun" | "monday" | "lundi" => "mon",
        "tue" | "mar" | "tuesday" | "mardi" => "tue",
        "wed" | "mer" | "wednesday" | "mercredi" => "wed",
        "thu" | "jeu" | "thursday" | "jeudi" => "thu",
        "fri" | "ven" | "friday" | "vendredi" => "fri",
        "sat" | "sam" | "saturday" | "samedi" => "sat",
        "sun" | "dim" | "sunday" | "dimanche" => "sun",
        _ => "??",
    }
}

fn parse_hhmm(s: &str) -> Option<u32> {
    let (h, m) = s.trim().split_once(':')?;
    let h: u32 = h.parse().ok()?;
    let m: u32 = m.parse().ok()?;
    if h > 23 || m > 59 {
        return None;
    }
    Some(h * 60 + m)
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
        regles: Option<Option<Regle>>,
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
            if let Some(v) = regles {
                w.regles = v;
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
                Ok((transition, new_state, desc, obs)) => {
                    // Fire cooldown, anchored on the last actual fire.
                    let pret = watcher
                        .last_run
                        .map(|lr| (now - lr).num_seconds() >= watcher.cooldown_effectif() as i64)
                        .unwrap_or(true);
                    let baseline = watcher.last_state.is_some();
                    let mut feu = false;
                    let mut question_llm: Option<String> = None;
                    if let Some(regles) = &watcher.regles {
                        // Compiled rules: evaluated mechanically at every poll,
                        // free. A state rule (down_depuis_min...) stays true while
                        // the situation lasts, so the cooldown makes it re-fire
                        // naturally (built-in sustained). Only llm_check leaves
                        // that survive the deterministic prefix cost an LLM call.
                        if pret && baseline {
                            match regles.evaluer(&obs, &chrono::Local::now()) {
                                Verdict::Vrai => feu = true,
                                Verdict::BesoinLlm(q) => {
                                    feu = true;
                                    question_llm = Some(q);
                                }
                                Verdict::Faux => {}
                            }
                        }
                    } else {
                        // Legacy path: transitions + optional text condition
                        // (semantic gate) + explicit sustained mode.
                        let soutenu = watcher.sustained
                            && baseline
                            && !watcher.condition.trim().is_empty()
                            && !desc.is_empty();
                        feu = pret && (transition || soutenu);
                    }
                    if feu {
                        let contexte_regles = watcher
                            .regles
                            .as_ref()
                            .map(|r| format!(" [rules: {}]", r.resume()))
                            .unwrap_or_default();
                        triggered.push(Declenchement {
                            id: watcher.id,
                            name: watcher.name.clone(),
                            prompt: watcher.prompt.clone(),
                            contexte: format!(
                                "Watcher '{}' ({:?}) on '{}': {}{}",
                                watcher.name, watcher.watcher_type, watcher.target, desc,
                                contexte_regles
                            ),
                            condition: watcher.condition.clone(),
                            // Legacy text conditions on file/url are semantic and
                            // go through the LLM gate; log substrings are already
                            // enforced mechanically. Rules carry their own
                            // question via question_llm.
                            semantique: watcher.regles.is_none()
                                && !watcher.condition.trim().is_empty()
                                && watcher.watcher_type != WatcherType::Log,
                            question_llm,
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
) -> (bool, Option<String>, String, Evenement) {
    let etat = match &mtime {
        Some(m) => format!("present:{m}"),
        None => "absent".to_string(),
    };
    let Some(last) = last else {
        let desc = match &mtime {
            Some(m) => format!("file present (baseline set, modified {m})"),
            None => "file absent (baseline set)".to_string(),
        };
        return (false, Some(etat), desc, Evenement::Rien);
    };
    let last_present = last != "absent";
    let last_mtime = last.strip_prefix("present:").unwrap_or(last);
    match (&mtime, last_present) {
        (Some(m), false) => (
            true,
            Some(etat),
            format!("file APPEARED (modified {m})"),
            Evenement::Apparu,
        ),
        (None, true) => (
            true,
            Some(etat),
            "file was DELETED".to_string(),
            Evenement::Supprime,
        ),
        (Some(m), true) if m != last_mtime => (
            true,
            Some(etat),
            format!("file MODIFIED (now {m})"),
            Evenement::Modifie,
        ),
        (Some(m), true) => (
            false,
            Some(etat),
            format!("file present, unchanged since {m}"),
            Evenement::Rien,
        ),
        (None, false) => (
            false,
            Some(etat),
            "file still absent".to_string(),
            Evenement::Rien,
        ),
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
) -> (bool, Option<String>, String, Evenement, Option<u64>) {
    let (last_up_hash, last_down_since): (Option<&str>, Option<&str>) = match last {
        None => (None, None),
        Some(l) if l.starts_with("down:") => (None, Some(&l[5..])),
        Some(l) => (Some(l.strip_prefix("up:").unwrap_or(l)), None),
    };
    let minutes_down = |since: &str| {
        chrono::DateTime::parse_from_rfc3339(since)
            .map(|d| (now - d.with_timezone(&Utc)).num_minutes().max(0) as u64)
            .unwrap_or(0)
    };
    match hash_texte {
        Some(h) => {
            let etat = format!("up:{h}");
            if last.is_none() {
                return (
                    false,
                    Some(etat),
                    "site UP (baseline set)".to_string(),
                    Evenement::Rien,
                    None,
                );
            }
            if let Some(since) = last_down_since {
                let m = minutes_down(since);
                return (
                    true,
                    Some(etat),
                    format!("site BACK UP after {m} min down"),
                    Evenement::RetourEnLigne,
                    None,
                );
            }
            if last_up_hash != Some(format!("{h}").as_str()) {
                (
                    true,
                    Some(etat),
                    "page CONTENT CHANGED".to_string(),
                    Evenement::ContenuChange,
                    None,
                )
            } else {
                (
                    false,
                    Some(etat),
                    "site up, content unchanged".to_string(),
                    Evenement::Rien,
                    None,
                )
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
                    Evenement::ToujoursDown,
                    Some(m),
                )
            } else if last.is_none() {
                (
                    false,
                    Some(format!("down:{}", now.to_rfc3339())),
                    "site DOWN (baseline set)".to_string(),
                    Evenement::Rien,
                    Some(0),
                )
            } else {
                (
                    true,
                    Some(format!("down:{}", now.to_rfc3339())),
                    "site went DOWN".to_string(),
                    Evenement::Down,
                    Some(0),
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
) -> Result<(bool, Option<String>, String, Observation)> {
    match watcher.watcher_type {
        WatcherType::File => {
            let meta = std::fs::metadata(&watcher.target).ok();
            let taille = meta.as_ref().map(|m| m.len());
            let mtime = meta.and_then(|m| m.modified().ok()).map(|m| {
                let t: chrono::DateTime<chrono::Utc> = m.into();
                t.to_rfc3339()
            });
            let (t, s, d, ev) = transition_fichier(mtime, watcher.last_state.as_deref());
            Ok((
                t,
                s,
                d,
                Observation {
                    evenement: ev,
                    taille_octets: taille,
                    ..Default::default()
                },
            ))
        }
        WatcherType::Url => {
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()?;
            // Unreachable, timeout or 5xx = DOWN; the hash is computed on the
            // extracted text so dynamic markup does not flap the state.
            let mut status: Option<u16> = None;
            let mut texte = String::new();
            let hash = match client.get(&watcher.target).send().await {
                Ok(resp) => {
                    status = Some(resp.status().as_u16());
                    if resp.status().is_server_error() {
                        None
                    } else {
                        match resp.text().await {
                            Ok(html) => {
                                texte = extraire_texte(&html);
                                texte.truncate(20_000);
                                let mut hasher = DefaultHasher::new();
                                texte.hash(&mut hasher);
                                Some(hasher.finish())
                            }
                            Err(_) => None,
                        }
                    }
                }
                Err(_) => None,
            };
            let (t, s, d, ev, minutes) =
                transition_url(hash, watcher.last_state.as_deref(), now);
            Ok((
                t,
                s,
                d,
                Observation {
                    evenement: ev,
                    minutes_down: minutes,
                    nouveau_contenu: texte,
                    status_http: status,
                    ..Default::default()
                },
            ))
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
                    Observation::default(),
                ));
            }

            file.seek(SeekFrom::Start(last_offset))?;
            let mut content = String::new();
            file.read_to_string(&mut content)?;

            // Legacy semantics: empty condition = any new content fires.
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
                "new log content".to_string()
            };
            let mut extrait_obs = content;
            if extrait_obs.len() > 20_000 {
                let coupe = extrait_obs.len() - 20_000;
                // Keep the TAIL (most recent lines), on a char boundary.
                let mut debut = coupe;
                while !extrait_obs.is_char_boundary(debut) {
                    debut += 1;
                }
                extrait_obs = extrait_obs[debut..].to_string();
            }
            Ok((
                triggered,
                Some(current_len.to_string()),
                desc,
                Observation {
                    evenement: Evenement::NouveauContenuLog,
                    nouveau_contenu: extrait_obs,
                    ..Default::default()
                },
            ))
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
            regles: None,
        };

        // First run initializes state without triggering
        let (triggered, state, _, _) = evaluate_watcher(&watcher, Utc::now()).await.unwrap();
        assert!(!triggered);
        assert!(state.is_some());

        // Second run with same file should not trigger
        let mut watcher2 = watcher.clone();
        watcher2.last_state = state.clone();
        let (triggered2, state2, _, _) = evaluate_watcher(&watcher2, Utc::now()).await.unwrap();
        assert!(!triggered2);
        assert_eq!(state, state2);

        // Wait a little to ensure modification time is different
        // In actual tests, modifying time might have low resolution on some file systems.
        // We'll trust standard fs tests for now, but sleep could be used if it flakes.
    }

    #[test]
    fn fichier_apparition_suppression_modification() {
        // Baseline on an absent file, then it appears.
        let (t0, s0, _, _) = transition_fichier(None, None);
        assert!(!t0);
        assert_eq!(s0.as_deref(), Some("absent"));
        let (t1, s1, d1, e1) = transition_fichier(Some("2026-07-02T10:00:00Z".into()), s0.as_deref());
        assert!(t1, "appearance must fire");
        assert!(d1.contains("APPEARED"));
        assert_eq!(e1, Evenement::Apparu);
        // Modification fires; unchanged does not.
        let (t2, s2, _, _) = transition_fichier(Some("2026-07-02T11:00:00Z".into()), s1.as_deref());
        assert!(t2);
        let (t3, s3, _, _) = transition_fichier(Some("2026-07-02T11:00:00Z".into()), s2.as_deref());
        assert!(!t3);
        // Deletion fires.
        let (t4, _, d4, e4) = transition_fichier(None, s3.as_deref());
        assert!(t4);
        assert!(d4.contains("DELETED"));
        assert_eq!(e4, Evenement::Supprime);
        // Legacy state (bare mtime, pre-upgrade watchers.json) reads as present.
        let (t5, _, _, _) = transition_fichier(None, Some("2026-07-01T09:00:00+00:00"));
        assert!(t5, "legacy present state -> deletion fires");
    }

    #[test]
    fn url_down_puis_retour_et_contenu() {
        let now = Utc::now();
        // Baseline up, then goes down: fires once, then stays silent while down
        // (the sustained mode re-offers it to the semantic gate instead).
        let (_, s0, _, _, _) = transition_url(Some(42), None, now);
        let (t1, s1, d1, e1, _) = transition_url(None, s0.as_deref(), now);
        assert!(t1);
        assert!(d1.contains("DOWN"));
        let (t2, s2, d2, e2, m2) = transition_url(None, s1.as_deref(), now + chrono::Duration::minutes(12));
        assert!(!t2, "still down = no new transition");
        assert_eq!(s1, s2, "outage keeps its original since");
        assert!(d2.contains("12 min"), "{d2}");
        assert_eq!(e2, Evenement::ToujoursDown);
        assert_eq!(m2, Some(12));
        assert_eq!(e1, Evenement::Down);
        // Back up fires with the outage duration.
        let (t3, s3, d3, _, _) = transition_url(Some(42), s2.as_deref(), now + chrono::Duration::minutes(15));
        assert!(t3);
        assert!(d3.contains("BACK UP"));
        // Content change fires; same content does not.
        let (t4, s4, _, _, _) = transition_url(Some(43), s3.as_deref(), now);
        assert!(t4);
        let (t5, _, _, _, _) = transition_url(Some(43), s4.as_deref(), now);
        assert!(!t5);
        // Legacy state (bare hash) reads as up with that hash.
        let (t6, _, _, _, _) = transition_url(Some(7), Some("7"), now);
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

    fn obs(ev: Evenement) -> Observation {
        Observation { evenement: ev, ..Default::default() }
    }

    fn a_local(jour: &str, hhmm: &str) -> chrono::DateTime<chrono::Local> {
        // Known anchors: 2026-07-02 is a Thursday.
        use chrono::TimeZone;
        let base = match jour {
            "thu" => "2026-07-02",
            "tue" => "2026-06-30",
            "sat" => "2026-07-04",
            _ => "2026-07-01", // wed
        };
        chrono::Local
            .datetime_from_str(&format!("{base} {hhmm}:00"), "%Y-%m-%d %H:%M:%S")
            .unwrap()
    }

    #[test]
    fn regles_scenario_mardi_jeudi_down_10min() {
        // The exact JSON the agent compiles for: "if Tuesday or Thursday and the
        // site has been down for at least 10 minutes".
        let r: Regle = serde_json::from_str(
            r#"{"op":"et","regles":[
                {"op":"jour_semaine","jours":["mar","jeu"]},
                {"op":"down_depuis_min","minutes":10}
            ]}"#,
        )
        .unwrap();
        let mut o = obs(Evenement::ToujoursDown);
        o.minutes_down = Some(12);
        // Thursday, down 12 min -> fires, ZERO LLM involved.
        assert_eq!(r.evaluer(&o, &a_local("thu", "09:00")), Verdict::Vrai);
        // Saturday -> deterministic Faux.
        assert_eq!(r.evaluer(&o, &a_local("sat", "09:00")), Verdict::Faux);
        // Thursday but only 5 min down -> Faux.
        o.minutes_down = Some(5);
        assert_eq!(r.evaluer(&o, &a_local("thu", "09:00")), Verdict::Faux);
        // Summary is auditable.
        assert!(r.resume().contains("down≥10min"), "{}", r.resume());
    }

    #[test]
    fn regles_llm_check_en_court_circuit() {
        let r: Regle = serde_json::from_str(
            r#"{"op":"et","regles":[
                {"op":"jour_semaine","jours":["tue"]},
                {"op":"contenu_change"},
                {"op":"llm_check","question":"the changelog mentions a security fix"}
            ]}"#,
        )
        .unwrap();
        // Saturday: the deterministic prefix short-circuits, the LLM is never consulted.
        assert_eq!(
            r.evaluer(&obs(Evenement::ContenuChange), &a_local("sat", "10:00")),
            Verdict::Faux
        );
        // Tuesday + content changed: only now the residual LLM question surfaces.
        match r.evaluer(&obs(Evenement::ContenuChange), &a_local("tue", "10:00")) {
            Verdict::BesoinLlm(q) => assert!(q.contains("security fix")),
            v => panic!("expected BesoinLlm, got {v:?}"),
        }
        // Tuesday but nothing changed: Faux without LLM.
        assert_eq!(
            r.evaluer(&obs(Evenement::Rien), &a_local("tue", "10:00")),
            Verdict::Faux
        );
    }

    #[test]
    fn regles_combinaisons_ou_non_heures() {
        // Overnight window 22:00-06:00.
        let nuit = Regle::HeureEntre { de: "22:00".into(), a: "06:00".into() };
        assert_eq!(nuit.evaluer(&obs(Evenement::Rien), &a_local("thu", "23:30")), Verdict::Vrai);
        assert_eq!(nuit.evaluer(&obs(Evenement::Rien), &a_local("thu", "05:59")), Verdict::Vrai);
        assert_eq!(nuit.evaluer(&obs(Evenement::Rien), &a_local("thu", "12:00")), Verdict::Faux);
        // NOT + OR + LlmCheck propagation.
        let r = Regle::Ou {
            regles: vec![
                Regle::Apparu,
                Regle::Non { regle: Box::new(Regle::LlmCheck { question: "is it noise".into() }) },
            ],
        };
        assert_eq!(r.evaluer(&obs(Evenement::Apparu), &a_local("thu", "12:00")), Verdict::Vrai);
        match r.evaluer(&obs(Evenement::Rien), &a_local("thu", "12:00")) {
            Verdict::BesoinLlm(q) => assert!(q.contains("NOT (")),
            v => panic!("expected BesoinLlm, got {v:?}"),
        }
    }

    #[test]
    fn regles_contenu_taille_status() {
        let mut o = obs(Evenement::NouveauContenuLog);
        o.nouveau_contenu = "ERROR: Disk Full on /dev/sda".into();
        o.taille_octets = Some(5 * 1_048_576);
        o.status_http = Some(503);
        assert_eq!(
            Regle::Contient { motif: "disk full".into() }.evaluer(&o, &a_local("thu", "12:00")),
            Verdict::Vrai
        );
        assert_eq!(
            Regle::TailleDepasseMo { mo: 4.0 }.evaluer(&o, &a_local("thu", "12:00")),
            Verdict::Vrai
        );
        assert_eq!(
            Regle::TailleDepasseMo { mo: 6.0 }.evaluer(&o, &a_local("thu", "12:00")),
            Verdict::Faux
        );
        assert_eq!(
            Regle::StatusHttp { codes: vec![500, 503] }.evaluer(&o, &a_local("thu", "12:00")),
            Verdict::Vrai
        );
        // French day names normalize too.
        assert_eq!(
            Regle::JourSemaine { jours: vec!["jeudi".into()] }
                .evaluer(&o, &a_local("thu", "12:00")),
            Verdict::Vrai
        );
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
            regles: None,
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
