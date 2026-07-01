//! The **Vigie**: pure monitoring of sterile loops (inspired by the best of
//! third-party: a side-effect-free controller).
//!
//! The Vigie *observes* tool calls and returns a [`Signal`]. It touches nothing:
//! the loop decides what to do with the signal (inject advice, stop cleanly).
//! It detects three pathologies:
//! 1. **repeated exact failure**: same (name+args) fails N times -> warn then block;
//! 2. **same tool fails**: a tool fails N times (varying args) -> warn then stop;
//! 3. **idempotent without progress**: a read tool returns the same result N times.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The Vigie's decision. Side-effect-free: the loop applies it.
#[derive(Debug, Clone, PartialEq)]
pub enum Signal {
    /// Nothing to report, carry on.
    Laisser,
    /// Suspicious loop: inject this advice into the observation (the tool still runs).
    Avertir(String),
    /// Too many repetitions: refuse this precise call (synthetic error result).
    Bloquer(String),
    /// Confirmed sterile loop: stop the butinage cleanly with this reason.
    Poser(String),
}

impl Signal {
    /// Can the call still execute despite the signal?
    pub fn autorise_execution(&self) -> bool {
        matches!(self, Signal::Laisser | Signal::Avertir(_))
    }
    /// Does the signal request stopping the butinage?
    pub fn demande_arret(&self) -> bool {
        matches!(self, Signal::Poser(_))
    }
}

/// Detection thresholds. Warnings always active; hard stops opt-in
/// (`arret_dur`) so as not to constrain an interactive agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeuilsVigie {
    pub avertir_echec_exact: u32,
    pub bloquer_echec_exact: u32,
    pub avertir_meme_outil: u32,
    pub poser_meme_outil: u32,
    pub avertir_sans_progres: u32,
    pub bloquer_sans_progres: u32,
    /// Enables hard blocks/stops (otherwise: warnings only).
    pub arret_dur: bool,
}

impl Default for SeuilsVigie {
    fn default() -> Self {
        Self {
            avertir_echec_exact: 2,
            bloquer_echec_exact: 5,
            avertir_meme_outil: 3,
            poser_meme_outil: 8,
            avertir_sans_progres: 2,
            bloquer_sans_progres: 5,
            arret_dur: true,
        }
    }
}

impl SeuilsVigie {
    /// Lenient profile for strong models (Claude/Codex/DeepSeek): trust them,
    /// warnings only.
    pub fn souple() -> Self {
        Self {
            arret_dur: false,
            ..Self::default()
        }
    }

    /// Strict profile for weak local models (small gemma/qwen): earlier hard
    /// stops to avoid runaway.
    pub fn strict() -> Self {
        Self {
            avertir_echec_exact: 2,
            bloquer_echec_exact: 4,
            avertir_meme_outil: 2,
            poser_meme_outil: 6,
            avertir_sans_progres: 2,
            bloquer_sans_progres: 4,
            arret_dur: true,
        }
    }
}

/// The monitor. Keeps per-signature/tool counters for the duration of a butinage.
/// Serializable: persisted in the [`crate::carnet::Carnet`] so a crash-resume does
/// not grant a stuck model a free round of re-looping.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Vigie {
    seuils: SeuilsVigie,
    echecs_exacts: HashMap<u64, u32>,
    echecs_par_outil: HashMap<String, u32>,
    /// signature -> (hash of the last result, number of identical repetitions).
    sans_progres: HashMap<u64, (u64, u32)>,
}

impl Vigie {
    pub fn nouvelle(seuils: SeuilsVigie) -> Self {
        Self {
            seuils,
            ..Default::default()
        }
    }

    /// Overrides the thresholds (on resume: the profile may have changed since the checkpoint).
    pub fn set_seuils(&mut self, seuils: SeuilsVigie) {
        self.seuils = seuils;
    }

    /// Before executing: do we block this call because it has already failed/stagnated too much?
    pub fn avant_appel(&self, signature: u64) -> Signal {
        if !self.seuils.arret_dur {
            return Signal::Laisser;
        }
        if let Some(&n) = self.echecs_exacts.get(&signature) {
            if n >= self.seuils.bloquer_echec_exact {
                return Signal::Bloquer(format!(
                    "This exact tool call has failed {n} times with identical arguments. \
                     Stop retrying it unchanged - change strategy or report the blocker."
                ));
            }
        }
        if let Some(&(_, r)) = self.sans_progres.get(&signature) {
            if r >= self.seuils.bloquer_sans_progres {
                return Signal::Bloquer(format!(
                    "This read-only call returned the same result {r} times. \
                     Use the result you already have or change the query."
                ));
            }
        }
        Signal::Laisser
    }

    /// After executing: updates the counters and returns an optional signal.
    ///
    /// - `ok`: did the tool succeed?
    /// - `idempotent`: is this a read tool (same args => same expected effect)?
    /// - `resultat_hash`: fingerprint of the result (to detect lack of progress).
    pub fn apres_appel(
        &mut self,
        nom: &str,
        signature: u64,
        ok: bool,
        idempotent: bool,
        resultat_hash: u64,
    ) -> Signal {
        if !ok {
            let exact = self.echecs_exacts.entry(signature).or_insert(0);
            *exact += 1;
            let exact = *exact;
            self.sans_progres.remove(&signature);

            let par_outil = self.echecs_par_outil.entry(nom.to_string()).or_insert(0);
            *par_outil += 1;
            let par_outil = *par_outil;

            if self.seuils.arret_dur && par_outil >= self.seuils.poser_meme_outil {
                return Signal::Poser(format!(
                    "Tool `{nom}` failed {par_outil} times this run. Stopping to avoid a sterile loop."
                ));
            }
            if exact >= self.seuils.avertir_echec_exact {
                return Signal::Avertir(format!(
                    "`{nom}` has failed {exact} times with identical arguments. This looks like a loop; \
                     inspect the error and change approach instead of retrying unchanged."
                ));
            }
            if par_outil >= self.seuils.avertir_meme_outil {
                return Signal::Avertir(format!(
                    "`{nom}` has failed {par_outil} times this run. Diagnose before retrying: try \
                     different arguments, a narrower scope, or another tool."
                ));
            }
            return Signal::Laisser;
        }

        // Success: forget the failures of this signature/tool.
        self.echecs_exacts.remove(&signature);
        self.echecs_par_outil.remove(nom);

        if !idempotent {
            self.sans_progres.remove(&signature);
            return Signal::Laisser;
        }

        // Idempotent: detect stagnation (same result repeated).
        let repet = match self.sans_progres.get(&signature) {
            Some(&(h, r)) if h == resultat_hash => r + 1,
            _ => 1,
        };
        self.sans_progres.insert(signature, (resultat_hash, repet));

        if repet >= self.seuils.avertir_sans_progres {
            return Signal::Avertir(format!(
                "`{nom}` returned the same result {repet} times. Use it or change the query \
                 instead of repeating the call."
            ));
        }
        Signal::Laisser
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rien_a_signaler_au_premier_appel() {
        let mut v = Vigie::nouvelle(SeuilsVigie::default());
        assert_eq!(v.avant_appel(1), Signal::Laisser);
        assert_eq!(v.apres_appel("web", 1, true, true, 42), Signal::Laisser);
    }

    #[test]
    fn echec_exact_repete_avertit_puis_bloque() {
        let mut v = Vigie::nouvelle(SeuilsVigie::default());
        // 1st failure: nothing
        assert_eq!(v.apres_appel("web", 7, false, false, 0), Signal::Laisser);
        // 2nd exact failure: warning (threshold avertir_echec_exact=2)
        assert!(matches!(v.apres_appel("web", 7, false, false, 0), Signal::Avertir(_)));
        // after 5 failures, avant_appel blocks
        v.apres_appel("web", 7, false, false, 0);
        v.apres_appel("web", 7, false, false, 0);
        v.apres_appel("web", 7, false, false, 0); // total 5
        assert!(matches!(v.avant_appel(7), Signal::Bloquer(_)));
    }

    #[test]
    fn meme_outil_echoue_finit_par_poser() {
        let mut v = Vigie::nouvelle(SeuilsVigie::default());
        // 8 failures with different signatures (varying args) -> Poser
        let mut dernier = Signal::Laisser;
        for i in 0..8 {
            dernier = v.apres_appel("shell", 1000 + i, false, false, 0);
        }
        assert!(matches!(dernier, Signal::Poser(_)));
    }

    #[test]
    fn idempotent_sans_progres_avertit() {
        let mut v = Vigie::nouvelle(SeuilsVigie::default());
        assert_eq!(v.apres_appel("web", 3, true, true, 999), Signal::Laisser);
        // same result (999) a 2nd time -> warning (threshold 2)
        assert!(matches!(v.apres_appel("web", 3, true, true, 999), Signal::Avertir(_)));
    }

    #[test]
    fn succes_efface_les_echecs() {
        let mut v = Vigie::nouvelle(SeuilsVigie::default());
        v.apres_appel("web", 5, false, false, 0);
        v.apres_appel("web", 5, false, false, 0); // 2 failures
        v.apres_appel("web", 5, true, false, 0); // success -> reset
        assert_eq!(v.avant_appel(5), Signal::Laisser);
    }

    #[test]
    fn profil_souple_n_arrete_jamais_dur() {
        let mut v = Vigie::nouvelle(SeuilsVigie::souple());
        for _ in 0..20 {
            let s = v.apres_appel("shell", 1, false, false, 0);
            assert!(!s.demande_arret(), "lenient must never Poser");
        }
        assert_eq!(v.avant_appel(1), Signal::Laisser); // never Bloquer
    }
}
