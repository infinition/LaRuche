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
    /// Counts the FULL request payload: system + per-message cost (text + serialized
    /// tool calls + multimodal pieces, via [`Message::cout_chars`]) + `extra_chars`
    /// (tool schemas sent with every call — dozens of tools are far from free).
    pub fn estimer(&mut self, systeme: &str, historique: &[Message], extra_chars: usize) {
        let chars: usize =
            systeme.len() + extra_chars + historique.iter().map(|m| m.cout_chars()).sum::<usize>();
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

    /// Calibrated characters-per-token ratio (base 4 chars/token corrected by the
    /// factor learned from real usage). Single source of truth for every consumer
    /// that reasons in characters (e.g. the hard truncation guardrail).
    pub fn chars_par_token(&self) -> f32 {
        (4.0 / self.facteur).clamp(1.0, 8.0)
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
        j.estimer(&"s".repeat(20), &h, 0);
        assert_eq!(j.utilise, (20 + 80) / 4);
    }

    #[test]
    fn estimer_compte_les_schemas_les_appels_et_les_pieces() {
        let mut j = Jauge::nouvelle(1_000_000, 0.7, 0.85);
        // Schemas (extra_chars) count.
        j.estimer("", &[], 4000);
        assert_eq!(j.utilise, 1000);
        // An assistant message carrying tool calls costs more than its bare text.
        let appel = crate::issue::Appel::nouveau("web_search", serde_json::json!({"q": "x".repeat(400)}));
        let h = vec![Message::assistant_avec_appels("", vec![appel])];
        j.estimer("", &h, 0);
        assert!(j.utilise > 100, "serialized tool call counted, got {}", j.utilise);
        // A multimodal piece is charged a bounded vision cost, not its byte count.
        let piece = crate::messagerie::Piece {
            kind: "image".into(),
            mime: "image/png".into(),
            data: "A".repeat(2_000_000), // 2 MB base64
        };
        let h = vec![Message::utilisateur_multimodal("voici", vec![piece])];
        j.estimer("", &h, 0);
        let borne_haute = 16_384 / 4 + 10;
        assert!(j.utilise > 500 && j.utilise < borne_haute, "clamped vision cost, got {}", j.utilise);
    }

    #[test]
    fn usage_reel_surclasse_l_estimation() {
        let mut j = Jauge::nouvelle(1000, 0.7, 0.85);
        j.estimer("x", &[], 0);
        j.maj_usage(640);
        assert_eq!(j.utilise, 640);
        assert!((j.ratio() - 0.64).abs() < 1e-6);
    }

    #[test]
    fn maj_usage_apprend_un_facteur_pour_calibrer_les_estimations() {
        let mut j = Jauge::nouvelle(100_000, 0.7, 0.85);
        // 4000 characters -> raw = 1000 estimated tokens.
        let h = vec![Message::utilisateur("a".repeat(4000))];
        j.estimer("", &h, 0);
        assert_eq!(j.utilise, 1000);
        // The provider reveals 1500 real tokens -> factor 1.5 learned.
        j.maj_usage(1500);
        assert_eq!(j.utilise, 1500);
        // The next estimate (same content) is now recalibrated to 1500.
        j.estimer("", &h, 0);
        assert_eq!(j.utilise, 1500);
    }
}
