//! What a **passe** (an iteration) produces and what ends a **butinage**.
//!
//! `Issue` normalizes the model output into *facts* usable by the policy
//! ([`crate::cap::boussole`]), so decisions never rely on string matching.

use serde::{Deserialize, Serialize};

/// **Native** stop reason from the model, normalized across providers.
/// (Fixes the hard-coded `Some("stop")` of some backends that broke
/// truncation detection.)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    /// The model finished its turn naturally.
    FinTour,
    /// Cut off by the output token limit (resume possible).
    Longueur,
    /// The model wants to call tools.
    Outils,
    /// Other/unknown (network, filter...).
    Autre,
}

/// A tool call requested by the model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Appel {
    pub id: String,
    pub nom: String,
    pub args: serde_json::Value,
}

impl Appel {
    pub fn nouveau(nom: impl Into<String>, args: serde_json::Value) -> Self {
        // Short id (9 hex chars): some providers (e.g. Mistral) truncate tool-call ids
        // to 9 characters internally; a full UUID would silently desynchronize.
        let mut id = uuid::Uuid::new_v4().simple().to_string();
        id.truncate(9);
        Self {
            id,
            nom: nom.into(),
            args,
        }
    }

    /// Stable canonical signature (name + sorted args) for the [`crate::cap::vigie::Vigie`].
    pub fn signature(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.nom.hash(&mut h);
        // serde_json serializes objects in insertion order; we re-sort the keys
        // for an order-independent signature.
        canonicaliser(&self.args).hash(&mut h);
        h.finish()
    }
}

/// Compact JSON with sorted keys (recursive): basis for a stable signature.
pub fn canonicaliser(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::Object(m) => {
            let mut clefs: Vec<&String> = m.keys().collect();
            clefs.sort();
            let corps: Vec<String> = clefs
                .into_iter()
                .map(|k| format!("{}:{}", k, canonicaliser(&m[k])))
                .collect();
            format!("{{{}}}", corps.join(","))
        }
        serde_json::Value::Array(a) => {
            let corps: Vec<String> = a.iter().map(canonicaliser).collect();
            format!("[{}]", corps.join(","))
        }
        autre => autre.to_string(),
    }
}

/// The issue of a passe: what the loop observes AFTER the model call.
#[derive(Debug, Clone)]
pub enum Issue {
    /// The model called the explicit `mission_accomplie` tool.
    MissionAccomplie { resume: String, confiance: f32 },
    /// The model called `clarify`: hand control back to the user.
    Clarification(String),
    /// The model wants to execute tools.
    Outils(Vec<Appel>),
    /// The model (only) set/updated its plan, with no other tool. Setting a plan
    /// is a productive act (like a tool call): the loop must continue so it
    /// moves to action rather than concluding. Bounded by `relance_max` (anti plan-loop).
    PlanEnregistre,
    /// Text-only response (no tool). We attach the facts that guide the decision.
    TexteSeul(TexteSeul),
}

/// Facts extracted from a text-only response, consumed by the boussole.
#[derive(Debug, Clone)]
pub struct TexteSeul {
    pub texte: String,
    /// Native stop reason from the model (if the provider supplies it).
    pub fin_native: Option<StopReason>,
    /// True if the itineraire still has open steps.
    pub plan_inacheve: bool,
    /// True if the text looks like a broken tool_call (recovery rail).
    pub malforme: bool,
    /// True if the output was truncated (stop_reason=length or unclosed tool block).
    pub tronquee: bool,
}

/// Terminal reason of a butinage.
#[derive(Debug, Clone, PartialEq)]
pub enum FinDeVol {
    /// Mission accomplished (final answer ready).
    Accomplie,
    /// Passe ceiling reached: possibly incomplete.
    Plafond,
    /// Fatal error (provider/auth) after exhausting recovery options.
    Erreur(String),
    /// Interrupted by the user (cancellation flag).
    Interrompue,
    /// The tool requests a clarification (hands control back).
    Clarification(String),
    /// Clean stop by the vigie (sterile loop detected).
    BoucleSterile(String),
    /// Tier 3 supervisor escalated to a human (stalled past the nudge budget).
    Escalade(String),
    /// Cumulative token budget exhausted (`Reglages::budget_tokens`).
    Budget,
}

/// Final result of a butinage, reported to the caller (node/dashboard).
#[derive(Debug, Clone)]
pub struct Bilan {
    pub texte: String,
    pub fin: FinDeVol,
    pub passes: usize,
}

impl Bilan {
    pub fn nouveau(texte: impl Into<String>, fin: FinDeVol, passes: usize) -> Self {
        Self {
            texte: texte.into(),
            fin,
            passes,
        }
    }

    /// Did the butinage yield a real answer (vs error/ceiling)?
    pub fn est_succes(&self) -> bool {
        matches!(
            self.fin,
            FinDeVol::Accomplie | FinDeVol::Clarification(_)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn signature_independante_de_l_ordre_des_clefs() {
        let a = Appel::nouveau("web", json!({"q": "rust", "n": 3}));
        let b = Appel::nouveau("web", json!({"n": 3, "q": "rust"}));
        assert_eq!(a.signature(), b.signature());
    }

    #[test]
    fn signature_distingue_les_args() {
        let a = Appel::nouveau("web", json!({"q": "rust"}));
        let b = Appel::nouveau("web", json!({"q": "go"}));
        assert_ne!(a.signature(), b.signature());
    }

    #[test]
    fn signature_distingue_le_nom() {
        let a = Appel::nouveau("web", json!({"q": "x"}));
        let b = Appel::nouveau("fetch", json!({"q": "x"}));
        assert_ne!(a.signature(), b.signature());
    }

    #[test]
    fn canonicaliser_recursif() {
        let v = json!({"b": [3, {"y": 1, "x": 2}], "a": 1});
        let c = canonicaliser(&v);
        assert_eq!(c, "{a:1,b:[3,{x:2,y:1}]}");
    }

    #[test]
    fn bilan_succes() {
        let b = Bilan::nouveau("ok", FinDeVol::Accomplie, 3);
        assert!(b.est_succes());
        let e = Bilan::nouveau("", FinDeVol::Plafond, 100);
        assert!(!e.est_succes());
    }
}
