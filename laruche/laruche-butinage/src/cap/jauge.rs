//! The **jauge**: tool context budget.
//!
//! Drives longevity: when the context fills up, [`crate::escale`] decides whether to
//! compact or consolidate. The jauge relies on the provider's **real tokens** when
//! available ([`maj_usage`](Jauge::maj_usage)), otherwise on a
//! `characters / 4` estimate ([`estimer`](Jauge::estimer)).

use crate::messagerie::Message;

/// Action recommended by the jauge based on fill level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Besoin {
    /// The context has room, nothing to do.
    Rien,
    /// Compact (summarize old turns, keep recent ones).
    Compacter,
    /// Consolidate (aggressive compaction, the context is nearly full).
    Consolider,
}

/// The context budget.
#[derive(Debug, Clone)]
pub struct Jauge {
    /// Model context window (tokens).
    pub max_tokens: usize,
    /// Tokens currently used (real or estimated).
    pub utilise: usize,
    seuil_compaction: f32,
    seuil_consolidation: f32,
    /// Last raw estimate (`characters / 4`): calibration base for the factor.
    dernier_brut: usize,
    /// Tokenizer calibration factor learned from real usage (`real / estimated`). Also
    /// captures the overhead of multimodal parts (images), absent from the character count.
    facteur: f32,
}

impl Jauge {
    pub fn nouvelle(max_tokens: usize, seuil_compaction: f32, seuil_consolidation: f32) -> Self {
        Self {
            max_tokens: max_tokens.max(1),
            utilise: 0,
            seuil_compaction,
            seuil_consolidation,
            dernier_brut: 0,
            facteur: 1.0,
        }
    }

    /// `characters / 4` estimate, recalibrated by the factor learned from real usage.
    pub fn estimer(&mut self, systeme: &str, historique: &[Message]) {
        let chars: usize = systeme.len() + historique.iter().map(|m| m.contenu.len()).sum::<usize>();
        let brut = chars / 4;
        self.dernier_brut = brut;
        self.utilise = (brut as f32 * self.facteur).round() as usize;
    }

    /// Resyncs on the real input tokens returned by the provider (more precise) and learns
    /// the calibration factor to refine subsequent estimates.
    pub fn maj_usage(&mut self, tokens_entree: usize) {
        if tokens_entree > 0 {
            self.utilise = tokens_entree;
            if self.dernier_brut > 0 {
                let f = tokens_entree as f32 / self.dernier_brut as f32;
                // Bound: prevents an aberrant count from making estimates diverge.
                self.facteur = f.clamp(0.2, 5.0);
            }
        }
    }

    pub fn ratio(&self) -> f32 {
        self.utilise as f32 / self.max_tokens as f32
    }

    pub fn besoin(&self) -> Besoin {
        let r = self.ratio();
        if r >= self.seuil_consolidation {
            Besoin::Consolider
        } else if r >= self.seuil_compaction {
            Besoin::Compacter
        } else {
            Besoin::Rien
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn besoin_selon_le_ratio() {
        let mut j = Jauge::nouvelle(1000, 0.70, 0.85);
        j.utilise = 500;
        assert_eq!(j.besoin(), Besoin::Rien);
        j.utilise = 720;
        assert_eq!(j.besoin(), Besoin::Compacter);
        j.utilise = 900;
        assert_eq!(j.besoin(), Besoin::Consolider);
    }

    #[test]
    fn estimer_compte_systeme_et_historique() {
        let mut j = Jauge::nouvelle(1000, 0.7, 0.85);
        let h = vec![Message::utilisateur("a".repeat(40)), Message::assistant("b".repeat(40))];
        j.estimer(&"s".repeat(20), &h);
        assert_eq!(j.utilise, (20 + 80) / 4);
    }

    #[test]
    fn usage_reel_surclasse_l_estimation() {
        let mut j = Jauge::nouvelle(1000, 0.7, 0.85);
        j.estimer("x", &[]);
        j.maj_usage(640);
        assert_eq!(j.utilise, 640);
        assert!((j.ratio() - 0.64).abs() < 1e-6);
    }

    #[test]
    fn maj_usage_apprend_un_facteur_pour_calibrer_les_estimations() {
        let mut j = Jauge::nouvelle(100_000, 0.7, 0.85);
        // 4000 characters -> raw = 1000 estimated tokens.
        let h = vec![Message::utilisateur("a".repeat(4000))];
        j.estimer("", &h);
        assert_eq!(j.utilise, 1000);
        // The provider reveals 1500 real tokens -> factor 1.5 learned.
        j.maj_usage(1500);
        assert_eq!(j.utilise, 1500);
        // The next estimate (same content) is now recalibrated to 1500.
        j.estimer("", &h);
        assert_eq!(j.utilise, 1500);
    }
}
