//! Tool-usage statistics per **(model, tool)** — phase 1 of the validated design.
//!
//! DOCTRINE (2026-07-02, same as the memory Hebbian bonus): usage signals
//! **re-rank availability, never decide, never exhort**. Raw call frequency is
//! NEVER a signal (positive feedback loop: monoculture, "calling ≠ calling well",
//! broken prefix cache). Only *success rate* and *latency* are recorded, keyed by
//! model — a tool gemma-12b keeps fumbling may be flawless for Claude.
//!
//! Phase 1 consumers:
//! 1. dynamic tool selection ([`crate::contexte::schema_outils_pour_prompt`]):
//!    reliability is a TIEBREAK between equally relevant tools;
//! 2. ε-greedy cold start: a never-tried forged tool (origin `custom`)
//!    occasionally gets a seat so it can ever earn a track record.
//!
//! Later phases (validated, not built): the curateur amending tool descriptions
//! with learned pitfalls; LaReine judging methodology with stats in hand; learned
//! Fragile/Robuste profiles per (model, tool).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

/// Counters for one (model, tool) pair. Cumulative, persisted.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StatOutil {
    pub appels: u64,
    pub succes: u64,
    pub latence_totale_ms: u64,
}

impl StatOutil {
    pub fn taux_succes(&self) -> f32 {
        if self.appels == 0 {
            0.0
        } else {
            self.succes as f32 / self.appels as f32
        }
    }
    pub fn latence_moyenne_ms(&self) -> u64 {
        if self.appels == 0 {
            0
        } else {
            self.latence_totale_ms / self.appels
        }
    }
}

/// Below this many attempts, no reliability signal is derived (anti-noise: one
/// lucky or unlucky call must not move rankings).
const MIN_ESSAIS_SIGNAL: u64 = 3;
/// Persist every N recorded calls (plus at load): cheap enough, crash-tolerant
/// (losing a few counters is harmless — these are statistics, not state).
const PERSISTER_TOUS_LES: u32 = 20;

#[derive(Default, Serialize, Deserialize)]
struct Table {
    /// modele -> outil -> stats
    par_modele: HashMap<String, HashMap<String, StatOutil>>,
}

/// The store. One global instance ([`globales`]), JSON-persisted.
pub struct StatsOutils {
    etat: Mutex<(Table, u32)>, // (table, writes since last persist)
    chemin: PathBuf,
}

static GLOBALES: OnceLock<StatsOutils> = OnceLock::new();

/// Global store (lazy). Path: `LARUCHE_STATS_OUTILS` or `stats-outils.json` in the
/// node's working directory (next to kanban.json & co).
pub fn globales() -> &'static StatsOutils {
    GLOBALES.get_or_init(|| {
        let chemin = std::env::var("LARUCHE_STATS_OUTILS")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("stats-outils.json"));
        StatsOutils::charger(chemin)
    })
}

impl StatsOutils {
    fn charger(chemin: PathBuf) -> Self {
        let table = std::fs::read_to_string(&chemin)
            .ok()
            .and_then(|j| serde_json::from_str(&j).ok())
            .unwrap_or_default();
        Self { etat: Mutex::new((table, 0)), chemin }
    }

    /// Records one executed call. Vigie blocks and permission denials never reach
    /// the executor, so they are not counted — a block is not the tool's fault.
    pub fn enregistrer(&self, modele: &str, outil: &str, ok: bool, ms: u64) {
        let mut g = self.etat.lock().unwrap();
        let s = g
            .0
            .par_modele
            .entry(modele.to_string())
            .or_default()
            .entry(outil.to_string())
            .or_default();
        s.appels += 1;
        if ok {
            s.succes += 1;
        }
        s.latence_totale_ms += ms;
        g.1 += 1;
        if g.1 >= PERSISTER_TOUS_LES {
            g.1 = 0;
            let json = serde_json::to_string_pretty(&g.0).unwrap_or_default();
            let chemin = self.chemin.clone();
            drop(g); // never hold the lock across I/O
            Self::ecrire_atomique(&chemin, &json);
        }
    }

    /// Attempts recorded for this pair (cold-start detection).
    pub fn essais(&self, modele: &str, outil: &str) -> u64 {
        let g = self.etat.lock().unwrap();
        g.0.par_modele
            .get(modele)
            .and_then(|m| m.get(outil))
            .map(|s| s.appels)
            .unwrap_or(0)
    }

    /// Reliability signal in [0,1], or `None` below [`MIN_ESSAIS_SIGNAL`] attempts
    /// (unknown ≠ bad: the consumer treats `None` as neutral).
    pub fn fiabilite(&self, modele: &str, outil: &str) -> Option<f32> {
        let g = self.etat.lock().unwrap();
        let s = g.0.par_modele.get(modele)?.get(outil)?;
        (s.appels >= MIN_ESSAIS_SIGNAL).then(|| s.taux_succes())
    }

    /// Compact digest of the tools that STRUGGLE with this model — success rate
    /// below 80% over at least 5 attempts, worst first, `n` max. `None` when
    /// nothing is noteworthy (the common case: the curateur then sees nothing).
    /// Consumed by the curateur (phase 2): stats re-rank its ATTENTION; the
    /// transcript remains the only admissible evidence of a cause.
    pub fn digest_problemes(&self, modele: &str, n: usize) -> Option<String> {
        const MIN_ESSAIS: u64 = 5;
        const SEUIL_OK: f32 = 0.8;
        let g = self.etat.lock().unwrap();
        let m = g.0.par_modele.get(modele)?;
        let mut mauvais: Vec<(&String, &StatOutil)> = m
            .iter()
            .filter(|(_, s)| s.appels >= MIN_ESSAIS && s.taux_succes() < SEUIL_OK)
            .collect();
        if mauvais.is_empty() {
            return None;
        }
        mauvais.sort_by(|a, b| {
            a.1.taux_succes()
                .partial_cmp(&b.1.taux_succes())
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.0.cmp(b.0))
        });
        let lignes: Vec<String> = mauvais
            .into_iter()
            .take(n)
            .map(|(o, s)| {
                format!(
                    "- {o}: {:.0}% success over {} calls (avg {} ms)",
                    s.taux_succes() * 100.0,
                    s.appels,
                    s.latence_moyenne_ms()
                )
            })
            .collect();
        Some(lignes.join("\n"))
    }

    /// Full snapshot (dashboard/API/debug).
    pub fn snapshot(&self) -> serde_json::Value {
        let g = self.etat.lock().unwrap();
        serde_json::to_value(&g.0).unwrap_or_default()
    }

    /// Force-persist now (shutdown hook, tests).
    pub fn persister(&self) {
        let g = self.etat.lock().unwrap();
        let json = serde_json::to_string_pretty(&g.0).unwrap_or_default();
        let chemin = self.chemin.clone();
        drop(g);
        Self::ecrire_atomique(&chemin, &json);
    }

    fn ecrire_atomique(chemin: &PathBuf, json: &str) {
        let tmp = chemin.with_extension("json.tmp");
        if std::fs::write(&tmp, json).is_ok() {
            let _ = std::fs::rename(&tmp, chemin);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> StatsOutils {
        let dir = std::env::temp_dir().join(format!("stats-test-{}", uuid::Uuid::new_v4()));
        let _ = std::fs::create_dir_all(&dir);
        StatsOutils::charger(dir.join("stats.json"))
    }

    #[test]
    fn fiabilite_neutre_sous_le_seuil() {
        let s = store();
        s.enregistrer("gemma", "web_fetch", true, 100);
        s.enregistrer("gemma", "web_fetch", false, 100);
        assert_eq!(s.fiabilite("gemma", "web_fetch"), None, "2 essais < seuil");
        s.enregistrer("gemma", "web_fetch", true, 100);
        assert_eq!(s.fiabilite("gemma", "web_fetch"), Some(2.0 / 3.0));
        // Keyed by model: another model has no signal.
        assert_eq!(s.fiabilite("claude", "web_fetch"), None);
        assert_eq!(s.essais("gemma", "web_fetch"), 3);
        assert_eq!(s.essais("gemma", "jamais_vu"), 0);
    }

    #[test]
    fn digest_ne_liste_que_les_outils_en_difficulte() {
        let s = store();
        // Healthy tool: 10/10 — must not appear.
        for _ in 0..10 {
            s.enregistrer("gemma", "web_fetch", true, 50);
        }
        // Struggling tool: 2/6 — must appear.
        for i in 0..6 {
            s.enregistrer("gemma", "convert_pdf", i < 2, 200);
        }
        // Too few attempts: 0/2 — must NOT appear (no signal yet).
        s.enregistrer("gemma", "rare_tool", false, 10);
        s.enregistrer("gemma", "rare_tool", false, 10);
        let d = s.digest_problemes("gemma", 8).expect("one struggling tool");
        assert!(d.contains("convert_pdf") && d.contains("33%"), "{d}");
        assert!(!d.contains("web_fetch") && !d.contains("rare_tool"), "{d}");
        // Other model: clean slate, no digest.
        assert!(s.digest_problemes("claude", 8).is_none());
    }

    #[test]
    fn persiste_et_recharge() {
        let s = store();
        s.enregistrer("m", "t", true, 42);
        s.persister();
        let relu = StatsOutils::charger(s.chemin.clone());
        assert_eq!(relu.essais("m", "t"), 1);
        let _ = std::fs::remove_file(&s.chemin);
    }
}
