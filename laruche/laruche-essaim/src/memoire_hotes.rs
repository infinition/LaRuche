//! What LaRuche has learned about fetching a given host.
//!
//! DOCTRINE, the same as [`crate::stats_outils`]: a learned signal **reorders
//! the attempts, it never decides and never skips**. The anti-blocking chain
//! still contains every route it always did; experience only changes which one
//! is tried FIRST. A host that changes its mind, a proxy that comes back up, a
//! block that gets lifted, all stay reachable, because a memory that could
//! remove a route would turn one bad day into a permanent false belief.
//!
//! What this buys: the second fetch of a walled host skips the two failed
//! attempts the first one paid for. Over a research run against one domain,
//! that is most of the latency and all of the pointless load on the target.
//!
//! What it deliberately does NOT record: page content, URLs, or anything
//! per-path. The key is the HOST, the value is which route worked. A store of
//! visited URLs would be a browsing history, which is not this tool's business.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

/// How a page was successfully obtained. Ordered as the chain tries them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Route {
    /// Plain HTTP request. The cheapest, and what we always try first by default.
    Directe,
    /// Structured data read straight from the source (JSON-LD): no renderer.
    Structuree,
    /// The r.jina.ai reader proxy.
    Jina,
    /// A Wayback snapshot: the live site did not answer.
    Archive,
    /// Headless browser render, the most expensive route.
    Rendu,
}

impl Route {
    pub fn etiquette(self) -> &'static str {
        match self {
            Route::Directe => "direct",
            Route::Structuree => "structured",
            Route::Jina => "jina",
            Route::Archive => "wayback",
            Route::Rendu => "render",
        }
    }
}

/// Tally for one host.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FicheHote {
    /// Successes per route.
    pub succes: HashMap<String, u32>,
    /// Direct attempts that came back blocked (403/429) or empty.
    pub murs: u32,
    /// Total recorded attempts, all routes.
    pub essais: u32,
}

impl FicheHote {
    /// Route with the most successes, if the evidence is worth acting on.
    pub fn route_preferee(&self) -> Option<Route> {
        if self.essais < MIN_ESSAIS_SIGNAL {
            return None;
        }
        let (nom, n) = self.succes.iter().max_by_key(|(_, n)| **n)?;
        // A single success is an anecdote. Two is a habit.
        if *n < 2 {
            return None;
        }
        route_depuis(nom)
    }

    /// Does the direct route keep hitting a wall on this host?
    pub fn mure(&self) -> bool {
        self.essais >= MIN_ESSAIS_SIGNAL
            && self.murs * 2 > self.essais
            && self
                .succes
                .get(Route::Directe.etiquette())
                .copied()
                .unwrap_or(0)
                == 0
    }
}

fn route_depuis(nom: &str) -> Option<Route> {
    [
        Route::Directe,
        Route::Structuree,
        Route::Jina,
        Route::Archive,
        Route::Rendu,
    ]
    .into_iter()
    .find(|r| r.etiquette() == nom)
}

/// Below this many attempts nothing is inferred: one unlucky fetch must not
/// pin a host to an expensive route for good.
const MIN_ESSAIS_SIGNAL: u32 = 3;
/// Persist every N records. These are statistics, not state: losing the tail
/// of them on a crash costs nothing.
const PERSISTER_TOUS_LES: u32 = 10;

#[derive(Default, Serialize, Deserialize)]
struct Table {
    par_hote: HashMap<String, FicheHote>,
}

/// The store. One global instance ([`globales`]), JSON-persisted.
pub struct MemoireHotes {
    etat: Mutex<(Table, u32)>,
    chemin: PathBuf,
}

static GLOBALES: OnceLock<MemoireHotes> = OnceLock::new();

/// Global store (lazy). Path: `LARUCHE_MEMOIRE_HOTES`, or `memoire-hotes.json`
/// in the node's working directory, next to `stats-outils.json`.
pub fn globales() -> &'static MemoireHotes {
    GLOBALES.get_or_init(|| {
        let chemin = std::env::var("LARUCHE_MEMOIRE_HOTES")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("memoire-hotes.json"));
        MemoireHotes::charger(chemin)
    })
}

/// Host of a URL, lowercased, without `www.`. The learning key.
pub fn hote_de(url: &str) -> Option<String> {
    let sans_scheme = url.split_once("://")?.1;
    let hote = sans_scheme
        .split(['/', '?', '#'])
        .next()
        .unwrap_or(sans_scheme);
    let hote = hote.split('@').next_back().unwrap_or(hote);
    let hote = hote.split(':').next().unwrap_or(hote);
    let hote = hote.trim_start_matches("www.");
    (!hote.is_empty()).then(|| hote.to_lowercase())
}

impl MemoireHotes {
    fn charger(chemin: PathBuf) -> Self {
        let table = std::fs::read_to_string(&chemin)
            .ok()
            .and_then(|j| serde_json::from_str(&j).ok())
            .unwrap_or_default();
        Self {
            etat: Mutex::new((table, 0)),
            chemin,
        }
    }

    /// Records that `route` got the page for `url`.
    pub fn succes(&self, url: &str, route: Route) {
        let Some(hote) = hote_de(url) else { return };
        let mut g = self.etat.lock().unwrap();
        let fiche = g.0.par_hote.entry(hote).or_default();
        *fiche
            .succes
            .entry(route.etiquette().to_string())
            .or_default() += 1;
        fiche.essais += 1;
        self.peut_etre_persister(g);
    }

    /// Records that the direct route was walled (403/429) or came back empty.
    pub fn mur(&self, url: &str) {
        let Some(hote) = hote_de(url) else { return };
        let mut g = self.etat.lock().unwrap();
        let fiche = g.0.par_hote.entry(hote).or_default();
        fiche.murs += 1;
        fiche.essais += 1;
        self.peut_etre_persister(g);
    }

    /// What we know about this host, if anything.
    pub fn fiche(&self, url: &str) -> Option<FicheHote> {
        let hote = hote_de(url)?;
        let g = self.etat.lock().unwrap();
        g.0.par_hote.get(&hote).cloned()
    }

    /// Route to try FIRST for this host. `None` means the usual order.
    ///
    /// Never returns a route to SKIP: the caller keeps its whole chain, it just
    /// starts somewhere better.
    pub fn route_preferee(&self, url: &str) -> Option<Route> {
        self.fiche(url)?.route_preferee()
    }

    /// One line for the model when a learned route is used, so a reordering is
    /// never invisible: an agent that cannot see why a page came from a proxy
    /// cannot reason about it either.
    pub fn note(&self, url: &str) -> Option<String> {
        let fiche = self.fiche(url)?;
        let route = fiche.route_preferee()?;
        Some(format!(
            "[learned: {} answers best via {} ({} successes over {} attempts)]",
            hote_de(url)?,
            route.etiquette(),
            fiche.succes.get(route.etiquette()).copied().unwrap_or(0),
            fiche.essais
        ))
    }

    fn peut_etre_persister(&self, mut g: std::sync::MutexGuard<'_, (Table, u32)>) {
        g.1 += 1;
        if g.1 < PERSISTER_TOUS_LES {
            return;
        }
        g.1 = 0;
        let json = serde_json::to_string_pretty(&g.0).unwrap_or_default();
        let chemin = self.chemin.clone();
        drop(g); // never hold the lock across I/O
        if let Some(parent) = chemin.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let provisoire = chemin.with_extension("json.tmp");
        if std::fs::write(&provisoire, &json).is_ok() {
            let _ = std::fs::rename(&provisoire, &chemin);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn memoire_jetable(nom: &str) -> MemoireHotes {
        let chemin = std::env::temp_dir().join(format!("laruche-test-{nom}.json"));
        let _ = std::fs::remove_file(&chemin);
        MemoireHotes::charger(chemin)
    }

    #[test]
    fn lhote_est_la_cle_pas_lurl() {
        assert_eq!(
            hote_de("https://www.Example.com/a/b?x=1").unwrap(),
            "example.com"
        );
        assert_eq!(
            hote_de("http://ds.lordtry.com:80/file/").unwrap(),
            "ds.lordtry.com"
        );
        assert!(hote_de("pas-une-url").is_none());
    }

    /// The anti-noise rule: one success proves nothing.
    #[test]
    fn un_seul_succes_ne_cree_pas_de_preference() {
        let m = memoire_jetable("un-succes");
        m.succes("https://exemple.test/a", Route::Jina);
        assert!(m.route_preferee("https://exemple.test/a").is_none());
    }

    #[test]
    fn une_habitude_repetee_devient_une_preference() {
        let m = memoire_jetable("habitude");
        for _ in 0..3 {
            m.succes("https://mure.test/a", Route::Jina);
        }
        assert_eq!(
            m.route_preferee("https://mure.test/x").unwrap(),
            Route::Jina
        );
        // The note must be legible: a silent reordering is not auditable.
        let note = m.note("https://mure.test/x").unwrap();
        assert!(note.contains("jina") && note.contains("mure.test"));
    }

    #[test]
    fn un_hote_qui_mure_est_reconnu_comme_tel() {
        let m = memoire_jetable("mur");
        for _ in 0..4 {
            m.mur("https://cloudwall.test/p");
        }
        assert!(m.fiche("https://cloudwall.test/p").unwrap().mure());
    }

    /// A host that starts answering directly again must stop being treated as
    /// walled: the memory reorders, it must never trap.
    #[test]
    fn un_mur_qui_tombe_cesse_de_compter() {
        let m = memoire_jetable("mur-tombe");
        for _ in 0..3 {
            m.mur("https://revenu.test/p");
        }
        assert!(m.fiche("https://revenu.test/p").unwrap().mure());
        for _ in 0..4 {
            m.succes("https://revenu.test/p", Route::Directe);
        }
        assert!(
            !m.fiche("https://revenu.test/p").unwrap().mure(),
            "a lifted block must not stay recorded as a wall"
        );
    }

    #[test]
    fn un_hote_inconnu_ne_dit_rien() {
        let m = memoire_jetable("inconnu");
        assert!(m.route_preferee("https://jamais-vu.test/").is_none());
        assert!(m.note("https://jamais-vu.test/").is_none());
    }
}
