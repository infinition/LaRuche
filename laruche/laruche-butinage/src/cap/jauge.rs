//! La **jauge** — budget de contexte de l'abeille.
//!
//! Pilote la longévité : quand le contexte se remplit, l'[`crate::escale`] décide de
//! compacter ou consolider. La jauge se base sur les **tokens réels** du provider
//! quand ils sont disponibles ([`maj_usage`](Jauge::maj_usage)), sinon sur une
//! estimation `caractères / 4` ([`estimer`](Jauge::estimer)).

use crate::messagerie::Message;

/// Action recommandée par la jauge selon le remplissage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Besoin {
    /// Le contexte respire, rien à faire.
    Rien,
    /// Compacter (résumer les vieux tours, garder les récents).
    Compacter,
    /// Consolider (compaction agressive — le contexte est presque plein).
    Consolider,
}

/// Le budget de contexte.
#[derive(Debug, Clone)]
pub struct Jauge {
    /// Fenêtre de contexte du modèle (tokens).
    pub max_tokens: usize,
    /// Tokens actuellement utilisés (réels ou estimés).
    pub utilise: usize,
    seuil_compaction: f32,
    seuil_consolidation: f32,
}

impl Jauge {
    pub fn nouvelle(max_tokens: usize, seuil_compaction: f32, seuil_consolidation: f32) -> Self {
        Self {
            max_tokens: max_tokens.max(1),
            utilise: 0,
            seuil_compaction,
            seuil_consolidation,
        }
    }

    /// Estimation `caractères / 4` (fallback, toujours disponible).
    pub fn estimer(&mut self, systeme: &str, historique: &[Message]) {
        let chars: usize = systeme.len() + historique.iter().map(|m| m.contenu.len()).sum::<usize>();
        self.utilise = chars / 4;
    }

    /// Recale sur les tokens d'entrée réels renvoyés par le provider (plus précis).
    pub fn maj_usage(&mut self, tokens_entree: usize) {
        if tokens_entree > 0 {
            self.utilise = tokens_entree;
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
}
