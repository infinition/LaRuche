//! The **settings** of a butinage: bounds, system prompt, model profile.
//!
//! A single engine, data-driven behavior: the [`ProfilModele`] adjusts the
//! rails (vigie, parallelism, trust in stop_reason) based on the target, without
//! duplicating code.

use crate::cap::vigie::SeuilsVigie;
use std::path::PathBuf;

/// Inference target profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProfilModele {
    /// Weak local models (gemma e4b/12b, small qwen): strict rails, 1 tool/turn,
    /// text fallback for tool_calls active, fast diversion.
    Fragile,
    /// Capable models (gemma 27/35b, qwen 32b, DeepSeek): parallelism, lenient vigie.
    #[default]
    Robuste,
    /// Robust native tools (Claude API, Codex): we trust the native stop_reason,
    /// heuristics nearly disabled, full parallelism.
    NatifOutils,
}

impl ProfilModele {
    /// Does the profile tolerate multiple tool calls per turn?
    pub fn parallelisme(self) -> bool {
        !matches!(self, ProfilModele::Fragile)
    }
    /// Vigie thresholds suited to the profile.
    pub fn seuils_vigie(self) -> SeuilsVigie {
        match self {
            ProfilModele::Fragile => SeuilsVigie::strict(),
            ProfilModele::Robuste => SeuilsVigie::default(),
            ProfilModele::NatifOutils => SeuilsVigie::souple(),
        }
    }
}

/// Settings of a butinage.
#[derive(Debug, Clone)]
pub struct Reglages {
    /// Hard ceiling on passes (absolute anti-runaway).
    pub plafond_passes: usize,
    /// Hard bound on sterile relances (weak-model rails: truncation, malformed tool,
    /// exploration). Small (~3): text alone = end of turn, we NEVER force continuation.
    pub relance_max: usize,
    /// In exploration mode: minimal web calls before accepting an end.
    pub min_web_exploration: usize,
    /// Max waits on rate-limit before giving up/diverting.
    pub max_rate_limit: usize,
    /// Max retries on transient failure.
    pub max_transitoire: usize,
    /// Model context window (tokens): drives the jauge/escale.
    pub context_max_tokens: usize,
    /// Number of recent turns kept intact during a compaction.
    pub garder_recents: usize,
    /// System prompt (stable tier). In English (best practice).
    pub systeme: String,
    /// Override of the memory consolidation prompt (escale). `None` = code default.
    /// Lets the user edit it via `system.prompt_extraction` (memory mirror).
    pub prompt_extraction: Option<String>,
    /// Target profile.
    pub profil: ProfilModele,
    /// Carnet persistence path (checkpoint). `None` = no disk resume.
    pub chemin_carnet: Option<PathBuf>,
    /// Tier 3 supervision (LaReine watching a long task). `None` = off (default).
    /// When set and `actif`, the loop detects plan stagnation and nudges/escalates.
    pub supervision: Option<crate::cap::reine::ConfigSupervision>,
    /// Default per-tool timeout (seconds). A hung tool (network, subprocess) must never
    /// freeze the whole butinage. `0` = no timeout. Overridable per tool via
    /// [`crate::outils::Outils::timeout_secs`].
    pub timeout_outil_secs: u64,
    /// Timeout (seconds) on a single MODEL call — mirror of the per-tool bound. A
    /// stalled stream (backend swap, half-dead connection) must never freeze the
    /// butinage; expiry is classed transient (backoff, then give up). `0` = unbounded.
    /// Generous default: a local 12B on CPU legitimately takes minutes.
    pub timeout_modele_secs: u64,
    /// Hard cap (characters) on a single tool observation reinjected into the history
    /// (head+tail kept, middle elided). Protects the context from a 500 KB web page.
    /// `0` = no cap.
    pub max_chars_observation: usize,
    /// Cumulative token budget (input + output) for the whole butinage. `0` = unlimited.
    /// When exhausted, the loop lands with [`crate::issue::FinDeVol::Budget`].
    pub budget_tokens: u64,
    /// LLM-summarized compaction (dense, preserves discoveries/decisions/dead-ends).
    /// Falls back to the extractive compaction when the auxiliary call fails.
    pub compaction_llm: bool,
    /// Is `delegate` available in this context? False for sub-agents (anti-recursion):
    /// steering nudges then stop suggesting an impossible fan-out.
    pub delegation_disponible: bool,
    /// Just-in-time memory recall at mission start (`Source::rappeler(mission)`
    /// injected as internal context). ON for sub-agents (scouts must know past
    /// findings and dead ends) and resumed runs; OFF for the main chat mission,
    /// whose caller already injects a budgeted working set into the system prompt.
    pub rappel_initial: bool,
}

impl Default for Reglages {
    fn default() -> Self {
        Self {
            plafond_passes: 100,
            relance_max: 3,
            min_web_exploration: 12,
            max_rate_limit: 3,
            max_transitoire: 3,
            context_max_tokens: 128_000,
            garder_recents: 12,
            systeme: String::new(),
            prompt_extraction: None,
            profil: ProfilModele::default(),
            chemin_carnet: None,
            supervision: None,
            timeout_outil_secs: 300,
            timeout_modele_secs: 600,
            max_chars_observation: 30_000,
            budget_tokens: 0,
            compaction_llm: true,
            delegation_disponible: true,
            rappel_initial: false,
        }
    }
}

impl Reglages {
    pub fn avec_systeme(mut self, s: impl Into<String>) -> Self {
        self.systeme = s.into();
        self
    }
    pub fn avec_profil(mut self, p: ProfilModele) -> Self {
        self.profil = p;
        self
    }
}
