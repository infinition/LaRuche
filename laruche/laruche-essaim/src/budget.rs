use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BudgetTracker {
    pub max_context: usize,
    pub used: usize,
}

impl BudgetTracker {
    pub fn new(max_context: usize) -> Self {
        Self {
            max_context,
            used: 0,
        }
    }

    pub fn with_used(max_context: usize, used: usize) -> Self {
        Self { max_context, used }
    }

    pub fn restant(&self) -> usize {
        self.max_context.saturating_sub(self.used)
    }

    pub fn ratio_utilise(&self) -> f32 {
        if self.max_context == 0 {
            return 0.0;
        }
        self.used as f32 / self.max_context as f32
    }

    pub fn status(&self) -> BudgetStatus {
        BudgetStatus::from_tracker(self)
    }
}

pub fn estimer_tokens(texte: &str) -> usize {
    let chars = texte.chars().count();
    if chars == 0 {
        0
    } else {
        chars.div_ceil(4)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BudgetStatus {
    pub used: usize,
    pub max: usize,
    pub ratio: f32,
    pub warn: bool,
    pub critical: bool,
}

impl BudgetStatus {
    pub fn new(used: usize, max: usize) -> Self {
        let ratio = if max == 0 {
            0.0
        } else {
            used as f32 / max as f32
        };
        Self {
            used,
            max,
            ratio,
            warn: ratio >= 0.75,
            critical: ratio >= 0.9,
        }
    }

    pub fn from_tracker(tracker: &BudgetTracker) -> Self {
        Self::new(tracker.used, tracker.max_context)
    }
}

impl Default for BudgetStatus {
    fn default() -> Self {
        Self::new(0, 0)
    }
}

impl Default for BudgetTracker {
    fn default() -> Self {
        Self::new(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estime_les_tokens_par_quart_de_chars() {
        assert_eq!(estimer_tokens(""), 0);
        assert_eq!(estimer_tokens("abcd"), 1);
        assert_eq!(estimer_tokens("abcde"), 2);
    }

    #[test]
    fn tracker_calcule_restant_et_ratio() {
        let tracker = BudgetTracker::with_used(100, 25);

        assert_eq!(tracker.restant(), 75);
        assert_eq!(tracker.ratio_utilise(), 0.25);
    }

    #[test]
    fn status_declenche_warn_et_critical() {
        let warn = BudgetStatus::new(75, 100);
        let critical = BudgetStatus::new(90, 100);

        assert!(warn.warn);
        assert!(!warn.critical);
        assert!(critical.warn);
        assert!(critical.critical);
    }
}
