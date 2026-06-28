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
