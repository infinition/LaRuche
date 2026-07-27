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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum WatcherType {
    File,
    Url,
    Log,
    /// A COMMAND whose output is the observation.
    ///
    /// The other three cover what a file, a page or a log says. Everything else,
    /// a lamp, a service, a container, free disk space, is only reachable through a
    /// CLI, and without this the agent falls back to a cron: a cron wakes a whole
    /// model turn at every tick, where a rule tree costs nothing.
    Commande,
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
                // Running a process is heavier than a stat: poll it like a URL.
                WatcherType::Url | WatcherType::Commande => 60,
                _ => 10,
            })
            .max(5)
    }

    /// Effective fire cooldown. URLs default to 15 min (a flapping page must not
    /// spam runs and notifications); file/log transitions fire freely by default.
    pub fn cooldown_effectif(&self) -> u64 {
        self.cooldown_secs.unwrap_or(match self.watcher_type {
            // A state that stays true (a lamp left on) must not notify every minute.
            WatcherType::Url | WatcherType::Commande => 900,
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
    /// Exit code of the command (command watchers).
    pub code_retour: Option<i32>,
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
    /// Exit code of a watched command. `0` means it succeeded, anything else is a
    /// failure, which is how you watch a service rather than its output text.
    CodeRetour { codes: Vec<i32> },
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
            Regle::CodeRetour { codes } => bool_verdict(
                obs.code_retour.map(|c| codes.contains(&c)).unwrap_or(false),
            ),
            Regle::LlmCheck { question } => BesoinLlm(question.clone()),
        }
    }

    /// Reject a tree that would deserialize cleanly yet never behave as written.
    ///
    /// Every leaf here degrades to a silent false rather than to an error at
    /// evaluation time, which is the right call at runtime (fail closed) and the
    /// wrong one at creation time: the user is told the watcher is armed while
    /// it can never fire. Real case: `heure_entre {de:"8", a:"23"}` was accepted,
    /// failed to parse at every poll, and turned an ERROR alert into a no-op for
    /// a whole night. Validation belongs where a human can still react.
    pub fn valider(&self) -> Result<(), String> {
        match self {
            Regle::Et { regles } | Regle::Ou { regles } => {
                let nom = if matches!(self, Regle::Et { .. }) { "et" } else { "ou" };
                if regles.is_empty() {
                    return Err(format!("`{nom}` needs at least one sub-rule in `regles`"));
                }
                for r in regles {
                    r.valider()?;
                }
                Ok(())
            }
            Regle::Non { regle } => regle.valider(),
            Regle::JourSemaine { jours } => {
                if jours.is_empty() {
                    return Err("`jour_semaine` needs at least one day in `jours`".into());
                }
                let inconnus: Vec<&str> = jours
                    .iter()
                    .filter(|j| normaliser_jour(j) == "??")
                    .map(|j| j.as_str())
                    .collect();
                if !inconnus.is_empty() {
                    return Err(format!(
                        "unknown day(s) {:?} in `jour_semaine`: use mon..sun or lun..dim (full names work too)",
                        inconnus
                    ));
                }
                Ok(())
            }
            Regle::HeureEntre { de, a } => {
                let bad: Vec<&str> = [de.as_str(), a.as_str()]
                    .into_iter()
                    .filter(|v| parse_hhmm(v).is_none())
                    .collect();
                if !bad.is_empty() {
                    return Err(format!(
                        "`heure_entre` cannot read {:?}: expected \"HH:MM\" (\"22:00\"), a bare hour (\"22\") or \"8h30\". \
                         An unreadable window makes the whole rule false at EVERY hour",
                        bad
                    ));
                }
                if parse_hhmm(de) == parse_hhmm(a) {
                    return Err(format!(
                        "`heure_entre` window {de}-{a} is empty (start equals end), so the rule is never true. \
                         The end bound is EXCLUSIVE: to cover up to 23:55 write a=\"23:56\", and for a whole day drop this rule"
                    ));
                }
                Ok(())
            }
            Regle::PlageDate { du, au } => {
                let ok = |d: &str| chrono::NaiveDate::parse_from_str(d.trim(), "%Y-%m-%d").is_ok();
                if !ok(du) || !ok(au) {
                    return Err(format!(
                        "`plage_date` expects \"YYYY-MM-DD\", got du={du:?} au={au:?}"
                    ));
                }
                if du.trim() > au.trim() {
                    return Err(format!("`plage_date` starts after it ends ({du} > {au})"));
                }
                Ok(())
            }
            Regle::Contient { motif } => {
                if motif.trim().is_empty() {
                    return Err("`contient` needs a non-empty `motif`".into());
                }
                Ok(())
            }
            Regle::DownDepuisMin { minutes } => {
                if *minutes == 0 {
                    return Err("`down_depuis_min` needs `minutes` greater than 0".into());
                }
                Ok(())
            }
            Regle::TailleDepasseMo { mo } => {
                if !(*mo > 0.0) {
                    return Err("`taille_depasse_mo` needs `mo` greater than 0".into());
                }
                Ok(())
            }
            Regle::StatusHttp { codes } => {
                if codes.is_empty() {
                    return Err("`status_http` needs at least one code in `codes`".into());
                }
                Ok(())
            }
            Regle::CodeRetour { codes } => {
                if codes.is_empty() {
                    return Err("`code_retour` needs at least one code in `codes`".into());
                }
                Ok(())
            }
            Regle::LlmCheck { question } => {
                if question.trim().is_empty() {
                    return Err("`llm_check` needs a non-empty `question`".into());
                }
                Ok(())
            }
            Regle::Apparu
            | Regle::Supprime
            | Regle::Modifie
            | Regle::ContenuChange
            | Regle::EstDown
            | Regle::RetourEnLigne => Ok(()),
        }
    }

    /// Reject leaves that the chosen watcher type can never satisfy.
    ///
    /// `valider` alone is not enough, because a rule can be perfectly well formed
    /// and still be dead on this target. A `file` watcher builds its observation
    /// with an EMPTY `nouveau_contenu`, so `contient` is false at every poll: the
    /// real case was "watch release.log and ping me if a line contains ERROR",
    /// created as `file`, which needed `log` to ever see a line.
    pub fn valider_pour(&self, wtype: WatcherType) -> Result<(), String> {
        let exige = |ok: bool, quoi: &str, veut: &str| -> Result<(), String> {
            if ok {
                Ok(())
            } else {
                Err(format!(
                    "`{quoi}` cannot be true on a {wtype:?} watcher: it needs a {veut} one. \
                     Set watcher_type accordingly, or drop that leaf"
                ))
            }
        };
        match self {
            Regle::Et { regles } | Regle::Ou { regles } => {
                for r in regles {
                    r.valider_pour(wtype)?;
                }
                Ok(())
            }
            Regle::Non { regle } => regle.valider_pour(wtype),

            // Need fresh text: a log, a page, or a command's output.
            Regle::Contient { .. } => exige(
                matches!(
                    wtype,
                    WatcherType::Log | WatcherType::Url | WatcherType::Commande
                ),
                "contient",
                "log, url or command",
            ),
            Regle::ContenuChange => exige(
                matches!(
                    wtype,
                    WatcherType::Log | WatcherType::Url | WatcherType::Commande
                ),
                "contenu_change",
                "log, url or command",
            ),
            // The exit code exists only where a process ran.
            Regle::CodeRetour { .. } => exige(
                matches!(wtype, WatcherType::Commande),
                "code_retour",
                "command",
            ),

            // File lifecycle and size: only the file watcher reports them.
            Regle::Apparu => exige(matches!(wtype, WatcherType::File), "apparu", "file"),
            Regle::Supprime => exige(matches!(wtype, WatcherType::File), "supprime", "file"),
            Regle::Modifie => exige(matches!(wtype, WatcherType::File), "modifie", "file"),
            Regle::TailleDepasseMo { .. } => exige(
                matches!(wtype, WatcherType::File),
                "taille_depasse_mo",
                "file",
            ),

            // Reachability: only the url watcher probes it.
            Regle::EstDown => exige(matches!(wtype, WatcherType::Url), "est_down", "url"),
            Regle::DownDepuisMin { .. } => {
                exige(matches!(wtype, WatcherType::Url), "down_depuis_min", "url")
            }
            Regle::RetourEnLigne => {
                exige(matches!(wtype, WatcherType::Url), "retour_en_ligne", "url")
            }
            Regle::StatusHttp { .. } => {
                exige(matches!(wtype, WatcherType::Url), "status_http", "url")
            }

            // Time, dates and the semantic leaf apply to every type.
            Regle::JourSemaine { .. }
            | Regle::HeureEntre { .. }
            | Regle::PlageDate { .. }
            | Regle::LlmCheck { .. } => Ok(()),
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

            Regle::CodeRetour { codes } => format!(
                "exit∈[{}]",
                codes.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(",")
            ),            Regle::LlmCheck { question } => format!("🧠« {question} »"),
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

/// Parse a local time of day into minutes since midnight.
///
/// Tolerant on input because the writer is usually a model: "22:00" is the
/// canonical form, but "22" (bare hour), "8h" and "8h30" all parse too. That
/// tolerance is not cosmetic. A window that fails to parse makes `HeureEntre`
/// evaluate false at every hour, which silently turns the whole watcher off:
/// a rule tree written as `{de:"8", a:"23"}` used to never fire, at any time,
/// while reporting itself as active.
fn parse_hhmm(s: &str) -> Option<u32> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let (h, m) = match s.split_once([':', 'h', 'H']) {
        Some((h, m)) => (h.trim(), m.trim()),
        None => (s, ""),
    };
    let h: u32 = h.parse().ok()?;
    let m: u32 = if m.is_empty() { 0 } else { m.parse().ok()? };
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
        WatcherType::Commande => {
            // A command is the only way to observe a lamp, a service, a container or
            // free disk space. The shape mirrors the log branch: `last_state` holds a
            // fingerprint of the previous output, so `contenu_change` works, and the
            // output itself feeds `contient`.
            if let Some(motif) = commande_refusee(&watcher.target) {
                return Err(anyhow::anyhow!(
                    "command refused for safety (forbidden pattern '{motif}')"
                ));
            }

            let sortie = tokio::time::timeout(
                std::time::Duration::from_secs(TIMEOUT_COMMANDE_SECS),
                executer_commande(&watcher.target),
            )
            .await;

            // A timeout is an observation, not an error: a command that stopped
            // answering is exactly the kind of change someone wants to be told about.
            let (texte, code) = match sortie {
                Ok(Ok(v)) => v,
                Ok(Err(e)) => (format!("error: {e}"), None),
                Err(_) => (
                    format!("error: command timed out after {TIMEOUT_COMMANDE_SECS}s"),
                    None,
                ),
            };

            let mut texte = normaliser_sortie(&texte);
            if texte.len() > 20_000 {
                let mut debut = texte.len() - 20_000;
                while !texte.is_char_boundary(debut) {
                    debut += 1;
                }
                texte = texte[debut..].to_string();
            }

            let empreinte = empreinte_sortie(&texte, code);
            let change = watcher.last_state.as_deref() != Some(empreinte.as_str());
            // First poll establishes the baseline. Firing on it would notify about a
            // lamp that was already on before anyone asked to be told.
            let premier_passage = watcher.last_state.is_none();

            // Legacy `condition` semantics, kept for watchers with no rule tree.
            let triggered = if watcher.condition.trim().is_empty() {
                change && !premier_passage
            } else {
                texte.contains(&watcher.condition)
            };
            let desc = if watcher.condition.trim().is_empty() {
                format!(
                    "command output changed (exit {}): {}",
                    code.map(|c| c.to_string()).unwrap_or_else(|| "?".into()),
                    texte.chars().take(300).collect::<String>()
                )
            } else {
                format!(
                    "command output contains '{}': {}",
                    watcher.condition,
                    texte.chars().take(300).collect::<String>()
                )
            };

            Ok((
                triggered,
                Some(empreinte),
                desc,
                Observation {
                    evenement: if change && !premier_passage {
                        Evenement::ContenuChange
                    } else {
                        Evenement::Rien
                    },
                    nouveau_contenu: texte,
                    code_retour: code,
                    ..Default::default()
                },
            ))
        }
    }
}

/// A watched command must answer fast. Past this it is not a state check any more,
/// and the poll loop would pile up processes.
const TIMEOUT_COMMANDE_SECS: u64 = 20;

/// Patterns refused in a watched command.
///
/// Mirrors the `shell_exec` blocklist rather than importing it: this crate does not
/// depend on the tool registry, and a watcher is worse than a one-off call anyway. It
/// runs unattended, every minute, forever, so what is merely risky by hand becomes a
/// standing hazard here. Returns the offending pattern, for an error the user can act on.
fn commande_refusee(commande: &str) -> Option<&'static str> {
    const INTERDITS: &[&str] = &[
        "rm -rf /",
        "rm -rf ~",
        "rm -rf .",
        "mkfs",
        "dd if=",
        ":(){",
        "shutdown",
        "reboot",
        "format ",
        "del /s /q c:\\",
        "rd /s /q c:\\",
        // A watcher must observe, not write. These are the ones that turn a poll loop
        // into a repeated mutation.
        "remove-item -recurse",
        "> /dev/sda",
    ];
    let c = commande.to_lowercase();
    INTERDITS.iter().find(|p| c.contains(**p)).copied()
}

/// Run the command through the platform shell and return `(output, exit code)`.
///
/// stderr is merged into stdout: a watcher on a service wants the error text as much as
/// the normal output, and splitting them would only make the rules harder to write.
async fn executer_commande(commande: &str) -> Result<(String, Option<i32>)> {
    let sortie = if cfg!(windows) {
        tokio::process::Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", commande])
            .output()
            .await?
    } else {
        tokio::process::Command::new("sh")
            .args(["-c", commande])
            .output()
            .await?
    };
    let mut texte = String::from_utf8_lossy(&sortie.stdout).to_string();
    let err = String::from_utf8_lossy(&sortie.stderr);
    if !err.trim().is_empty() {
        if !texte.is_empty() {
            texte.push('\n');
        }
        texte.push_str(&err);
    }
    Ok((texte, sortie.status.code()))
}

/// Fingerprint of an output, stored in `last_state` so `contenu_change` can compare.
///
/// A hash, not the text: `last_state` is persisted on every poll and a command can
/// print thousands of lines. The exit code is part of it, so a command that starts
/// failing while printing the same thing still counts as a change.
fn empreinte_sortie(texte: &str, code: Option<i32>) -> String {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    texte.hash(&mut h);
    code.hash(&mut h);
    format!("{:x}", h.finish())
}

/// Collapse whitespace so a command that pads its output differently between two runs
/// does not read as a change.
fn normaliser_sortie(texte: &str) -> String {
    texte
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
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

    /// The exact tree an agent produced for "ping me on Telegram if a line
    /// contains ERROR, but not at night". It parsed, it was stored, it was shown
    /// as active, and it could never fire: "8" has no colon, the window failed to
    /// parse, and an unreadable window is false at every hour.
    #[test]
    fn heure_entre_heure_nue_ne_tue_plus_la_regle() {
        let regle = Regle::Et {
            regles: vec![
                Regle::Apparu,
                Regle::Contient { motif: "ERROR".into() },
                Regle::HeureEntre { de: "8".into(), a: "23".into() },
            ],
        };
        assert!(regle.valider().is_ok(), "a bare hour must be accepted");

        let mut o = obs(Evenement::Apparu);
        o.nouveau_contenu = "boom ERROR boom".into();
        assert_eq!(regle.evaluer(&o, &a_local("tue", "10:00")), Verdict::Vrai);
        assert_eq!(regle.evaluer(&o, &a_local("tue", "23:30")), Verdict::Faux);
        assert_eq!(regle.evaluer(&o, &a_local("tue", "03:00")), Verdict::Faux);
    }

    /// Second half of the same real failure: even with a readable window, that
    /// tree was created as a `file` watcher, whose observation carries no text,
    /// so `contient` could never be true either.
    #[test]
    fn contient_sur_un_watcher_fichier_est_refuse() {
        let regle = Regle::Et {
            regles: vec![
                Regle::Apparu,
                Regle::Contient { motif: "ERROR".into() },
                Regle::HeureEntre { de: "08:00".into(), a: "23:56".into() },
            ],
        };
        let err = regle
            .valider_pour(WatcherType::File)
            .expect_err("contient on a file watcher must be rejected");
        assert!(err.contains("contient"), "the message must name the guilty leaf: {err}");
        assert!(err.contains("log"), "and point at the right type: {err}");

        // The same intent, written correctly, passes.
        let bonne = Regle::Et {
            regles: vec![
                Regle::Contient { motif: "ERROR".into() },
                Regle::HeureEntre { de: "08:00".into(), a: "23:56".into() },
            ],
        };
        assert!(bonne.valider_pour(WatcherType::Log).is_ok());
        // And a file lifecycle leaf has no meaning on a log watcher.
        assert!(Regle::Apparu.valider_pour(WatcherType::Log).is_err());
        assert!(Regle::EstDown.valider_pour(WatcherType::File).is_err());
        assert!(Regle::StatusHttp { codes: vec![500] }
            .valider_pour(WatcherType::Url)
            .is_ok());
    }

    #[test]
    fn parse_hhmm_tolere_les_formes_humaines() {
        assert_eq!(parse_hhmm("22:00"), Some(22 * 60));
        assert_eq!(parse_hhmm("8"), Some(8 * 60));
        assert_eq!(parse_hhmm(" 8h "), Some(8 * 60));
        assert_eq!(parse_hhmm("8h30"), Some(8 * 60 + 30));
        assert_eq!(parse_hhmm("23:56"), Some(23 * 60 + 56));
        assert_eq!(parse_hhmm(""), None);
        assert_eq!(parse_hhmm("24:00"), None);
        assert_eq!(parse_hhmm("8:60"), None);
        assert_eq!(parse_hhmm("midi"), None);
    }

    #[test]
    fn valider_refuse_les_arbres_qui_ne_declenchent_jamais() {
        // Unreadable window.
        assert!(Regle::HeureEntre { de: "midi".into(), a: "18:00".into() }
            .valider()
            .is_err());
        // Empty window: start equals end.
        assert!(Regle::HeureEntre { de: "08:00".into(), a: "8".into() }
            .valider()
            .is_err());
        // Empty combinator.
        assert!(Regle::Et { regles: vec![] }.valider().is_err());
        // Typo in a day name, which used to match nothing in silence.
        assert!(Regle::JourSemaine { jours: vec!["mardi".into(), "jeudy".into()] }
            .valider()
            .is_err());
        assert!(Regle::JourSemaine { jours: vec!["mardi".into(), "thu".into()] }
            .valider()
            .is_ok());
        // Empty pattern, zero threshold, empty question.
        assert!(Regle::Contient { motif: "  ".into() }.valider().is_err());
        assert!(Regle::DownDepuisMin { minutes: 0 }.valider().is_err());
        assert!(Regle::TailleDepasseMo { mo: 0.0 }.valider().is_err());
        assert!(Regle::StatusHttp { codes: vec![] }.valider().is_err());
        assert!(Regle::LlmCheck { question: "".into() }.valider().is_err());
        // A bad leaf deep in the tree must surface.
        assert!(Regle::Et {
            regles: vec![Regle::Apparu, Regle::Non {
                regle: Box::new(Regle::Contient { motif: "".into() })
            }],
        }
        .valider()
        .is_err());
    }

    /// The end bound is exclusive, which is exactly what the agent got wrong when
    /// it claimed a="23" would cover 23:55.
    #[test]
    fn heure_entre_borne_haute_exclusive() {
        let r = Regle::HeureEntre { de: "08:00".into(), a: "23:00".into() };
        assert_eq!(r.evaluer(&obs(Evenement::Rien), &a_local("tue", "22:59")), Verdict::Vrai);
        assert_eq!(r.evaluer(&obs(Evenement::Rien), &a_local("tue", "23:00")), Verdict::Faux);
        let large = Regle::HeureEntre { de: "08:00".into(), a: "23:56".into() };
        assert_eq!(large.evaluer(&obs(Evenement::Rien), &a_local("tue", "23:55")), Verdict::Vrai);
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

#[cfg(test)]
mod tests_commande {
    use super::*;

    #[test]
    fn une_commande_destructrice_est_refusee() {
        // A watcher runs unattended, every minute, forever. What is merely risky by
        // hand is a standing hazard here, so the guard is stricter than a one-off call.
        assert!(commande_refusee("rm -rf / --no-preserve-root").is_some());
        assert!(commande_refusee("Remove-Item -Recurse C:\\").is_some());
        assert!(commande_refusee("shutdown /s /t 0").is_some());
        // What people actually watch must go through untouched.
        assert!(commande_refusee("openhue get light \"Hue Play Bureau fab\"").is_none());
        assert!(commande_refusee("docker ps --filter status=running").is_none());
        assert!(commande_refusee("git status --porcelain").is_none());
    }

    #[test]
    fn lempreinte_change_avec_la_sortie_et_avec_le_code() {
        let a = empreinte_sortie("light on", Some(0));
        assert_eq!(a, empreinte_sortie("light on", Some(0)));
        assert_ne!(a, empreinte_sortie("light off", Some(0)));
        // Same text, different exit code: a command that starts failing while printing
        // the same thing is still a change worth firing on.
        assert_ne!(a, empreinte_sortie("light on", Some(1)));
    }

    #[test]
    fn la_sortie_est_normalisee_pour_ne_pas_faussement_changer() {
        assert_eq!(normaliser_sortie("  a  \n b   \n"), "a\n b");
        assert_eq!(normaliser_sortie("x   "), normaliser_sortie("x"));
    }

    #[test]
    fn les_regles_texte_sont_permises_sur_une_commande_et_code_retour_reserve() {
        // The point of the type: `contient` on a command's output.
        assert!(Regle::Contient { motif: "[on]".into() }
            .valider_pour(WatcherType::Commande)
            .is_ok());
        assert!(Regle::ContenuChange.valider_pour(WatcherType::Commande).is_ok());
        assert!(Regle::CodeRetour { codes: vec![0] }
            .valider_pour(WatcherType::Commande)
            .is_ok());
        // An exit code exists nowhere else, and a file carries no text.
        assert!(Regle::CodeRetour { codes: vec![0] }
            .valider_pour(WatcherType::Url)
            .is_err());
        assert!(Regle::Apparu.valider_pour(WatcherType::Commande).is_err());
        // An empty list can never match: refuse it at creation, like status_http.
        assert!(Regle::CodeRetour { codes: vec![] }.valider().is_err());
    }
}
