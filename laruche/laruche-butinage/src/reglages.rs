//! Les **réglages** d'un butinage : bornes, prompt système, profil modèle.
//!
//! Un seul moteur, comportement piloté par données : le [`ProfilModele`] ajuste les
//! rails (vigie, parallélisme, confiance au stop_reason) selon la cible, sans dupliquer
//! de code.

use crate::cap::vigie::SeuilsVigie;
use std::path::PathBuf;

/// Profil de la cible d'inférence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProfilModele {
    /// Modèles locaux faibles (gemma e4b/12b, qwen petit) : rails stricts, 1 outil/tour,
    /// fallback texte des tool_calls actif, déroutement rapide.
    Fragile,
    /// Modèles capables (gemma 27/35b, qwen 32b, DeepSeek) : parallélisme, vigie souple.
    #[default]
    Robuste,
    /// Outils natifs robustes (Claude API, Codex) : on fait confiance au stop_reason natif,
    /// heuristiques quasi désactivées, parallélisme plein.
    NatifOutils,
}

impl ProfilModele {
    /// Le profil tolère-t-il plusieurs appels d'outils par tour ?
    pub fn parallelisme(self) -> bool {
        !matches!(self, ProfilModele::Fragile)
    }
    /// Seuils de vigie adaptés au profil.
    pub fn seuils_vigie(self) -> SeuilsVigie {
        match self {
            ProfilModele::Fragile => SeuilsVigie::strict(),
            ProfilModele::Robuste => SeuilsVigie::default(),
            ProfilModele::NatifOutils => SeuilsVigie::souple(),
        }
    }
}

/// Réglages d'un butinage.
#[derive(Debug, Clone)]
pub struct Reglages {
    /// Plafond dur de passes (anti-runaway absolu).
    pub plafond_passes: usize,
    /// Plafond d'auto-continuations entre deux récoltes d'outils.
    pub auto_continue_max: usize,
    /// En mode exploration : appels web minimaux avant d'accepter une fin.
    pub min_web_exploration: usize,
    /// Attentes max sur rate-limit avant d'abandonner/dérouter.
    pub max_rate_limit: usize,
    /// Retries max sur panne passagère.
    pub max_transitoire: usize,
    /// Prompt système (tier stable). En anglais (best practice).
    pub systeme: String,
    /// Profil de la cible.
    pub profil: ProfilModele,
    /// Chemin de persistance du carnet (checkpoint). `None` = pas de reprise disque.
    pub chemin_carnet: Option<PathBuf>,
}

impl Default for Reglages {
    fn default() -> Self {
        Self {
            plafond_passes: 100,
            auto_continue_max: 20,
            min_web_exploration: 12,
            max_rate_limit: 3,
            max_transitoire: 3,
            systeme: String::new(),
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
