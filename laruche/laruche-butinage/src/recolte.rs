//! The **recolte**: execution of a pass's tools.
//!
//! Claude Code-style partitioning: consecutive **read-only** calls are harvested
//! **in parallel** (bounded), while mutating/approval calls stay **sequential**
//! and isolated. The original ordering of observations is preserved.
//! The [`Vigie`] is consulted before each call (block) and after (signal).

use crate::carnet::Carnet;
use crate::cap::vigie::{Signal, Vigie};
use crate::evenement::{Emetteur, Evenement};
use crate::issue::{Appel, Bilan, FinDeVol};
use crate::messagerie::Message;
use crate::outils::{Outils, ResultatOutil};
use crate::reglages::Reglages;
use futures_util::future::join_all;
use std::time::Instant;

/// Ceiling on calls launched simultaneously within a parallel batch.
const MAX_PARALLELE: usize = 6;

/// Splits calls into batches: `(read_only, indices)`. Consecutive safe calls are
/// grouped together; a mutating call breaks the batch. Order preserved.
pub fn partitionner(appels: &[Appel], outils: &dyn Outils) -> Vec<(bool, Vec<usize>)> {
    let mut lots: Vec<(bool, Vec<usize>)> = Vec::new();
    for (i, a) in appels.iter().enumerate() {
        let sur = outils.concurrence_sure(a);
        match lots.last_mut() {
            Some((lot_sur, idxs)) if *lot_sur && sur => idxs.push(i),
            _ => lots.push((sur, vec![i])),
        }
    }
    lots
}

/// Executes all calls of a pass. Returns `Some(bilan)` if the vigie forces a
/// stop (sterile loop), otherwise `None`.
pub async fn recolter(
    appels: &[Appel],
    carnet: &mut Carnet,
    reglages: &Reglages,
    outils: &dyn Outils,
    vigie: &mut Vigie,
    emet: &dyn Emetteur,
) -> Option<Bilan> {
    let parallele = reglages.profil.parallelisme();

    for (sur, idxs) in partitionner(appels, outils) {
        if sur && parallele && idxs.len() > 1 {
            // ── Parallel batch (read-only) ──
            // Pre-filter via the vigie (avant_appel is non-mutating).
            let mut a_lancer: Vec<usize> = Vec::new();
            for &i in &idxs {
                if let Signal::Bloquer(msg) = vigie.avant_appel(appels[i].signature()) {
                    pousser_blocage(carnet, &appels[i].nom, &msg, emet);
                } else {
                    a_lancer.push(i);
                }
            }
            if a_lancer.len() > 1 {
                emet.emettre(Evenement::Statut(format!(
                    "Parallel harvest of {} tools...",
                    a_lancer.len()
                )));
            }
            for groupe in a_lancer.chunks(MAX_PARALLELE) {
                for &i in groupe {
                    emet.emettre(Evenement::AppelOutil { nom: appels[i].nom.clone() });
                }
                let futs = groupe.iter().map(|&i| {
                    let appel = &appels[i];
                    async move {
                        let t0 = Instant::now();
                        let res = outils.executer(appel).await;
                        (i, res, t0.elapsed().as_millis() as u64)
                    }
                });
                let mut resultats = join_all(futs).await;
                resultats.sort_by_key(|(i, _, _)| *i); // observations in original order
                for (i, res, ms) in resultats {
                    if let Some(bilan) = appliquer(&appels[i], res, ms, carnet, outils, vigie, emet) {
                        return Some(bilan);
                    }
                }
            }
        } else {
            // ── Sequential (mutating, approval, or non-parallel profile) ──
            for &i in &idxs {
                let appel = &appels[i];
                if let Signal::Bloquer(msg) = vigie.avant_appel(appel.signature()) {
                    pousser_blocage(carnet, &appel.nom, &msg, emet);
                    continue;
                }
                emet.emettre(Evenement::AppelOutil { nom: appel.nom.clone() });
                let t0 = Instant::now();
                let res = outils.executer(appel).await;
                let ms = t0.elapsed().as_millis() as u64;
                if let Some(bilan) = appliquer(appel, res, ms, carnet, outils, vigie, emet) {
                    return Some(bilan);
                }
            }
        }
    }
    None
}

/// Applies the result of a call: web counter, vigie, event, observation.
/// Returns `Some(bilan)` if the vigie forces a stop.
fn appliquer(
    appel: &Appel,
    res: ResultatOutil,
    ms: u64,
    carnet: &mut Carnet,
    outils: &dyn Outils,
    vigie: &mut Vigie,
    emet: &dyn Emetteur,
) -> Option<Bilan> {
    if outils.est_web(appel) {
        carnet.recolte_web += 1;
    }
    let signal = vigie.apres_appel(
        &appel.nom,
        appel.signature(),
        res.ok,
        outils.idempotent(&appel.nom),
        res.empreinte(),
    );
    emet.emettre(Evenement::ResultatOutil { nom: appel.nom.clone(), ok: res.ok, ms });

    let mut observation = res.sortie.clone();
    if let Signal::Avertir(m) | Signal::Poser(m) = &signal {
        observation.push_str(&format!("\n\n[vigie: {m}]"));
    }
    carnet.historique.push(Message::observation(&appel.nom, observation));

    if let Signal::Poser(motif) = signal {
        carnet.itineraire.finaliser();
        return Some(Bilan::nouveau(
            "Stopped: sterile loop detected by the vigie.",
            FinDeVol::BoucleSterile(motif),
            carnet.passe + 1,
        ));
    }
    None
}

fn pousser_blocage(carnet: &mut Carnet, nom: &str, msg: &str, emet: &dyn Emetteur) {
    carnet
        .historique
        .push(Message::observation(nom, format!("Blocked: {msg}")));
    emet.emettre(Evenement::ResultatOutil { nom: nom.to_string(), ok: false, ms: 0 });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::carnet::ModeMission;
    use crate::evenement::Silencieux;
    use crate::reglages::ProfilModele;
    use async_trait::async_trait;
    use serde_json::json;

    struct OutilsMock;
    #[async_trait]
    impl Outils for OutilsMock {
        async fn executer(&self, appel: &Appel) -> ResultatOutil {
            ResultatOutil::ok(format!("res:{}", appel.nom))
        }
        fn idempotent(&self, nom: &str) -> bool {
            nom.starts_with("web_") || nom.starts_with("lire_")
        }
    }

    fn t0() -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::from_timestamp(1_700_000_000, 0).unwrap()
    }

    #[test]
    fn partition_groupe_les_lectures_consecutives() {
        let appels = vec![
            Appel::nouveau("web_a", json!({})),
            Appel::nouveau("web_b", json!({})),
            Appel::nouveau("file_write", json!({})), // mutating -> breaks the batch
            Appel::nouveau("lire_x", json!({})),
        ];
        let lots = partitionner(&appels, &OutilsMock);
        assert_eq!(lots.len(), 3);
        assert_eq!(lots[0], (true, vec![0, 1])); // web_a + web_b in parallel
        assert_eq!(lots[1], (false, vec![2])); // write alone
        assert_eq!(lots[2], (true, vec![3])); // lire_x alone (batch of one)
    }

    #[tokio::test]
    async fn recolte_parallele_preserve_l_ordre_et_compte_le_web() {
        let appels = vec![
            Appel::nouveau("web_a", json!({})),
            Appel::nouveau("web_b", json!({})),
            Appel::nouveau("web_c", json!({})),
        ];
        let mut carnet = Carnet::ouvrir("m", ModeMission::Standard, t0());
        let reglages = Reglages { profil: ProfilModele::Robuste, ..Reglages::default() };
        let mut vigie = Vigie::nouvelle(ProfilModele::Robuste.seuils_vigie());
        let arret = recolter(&appels, &mut carnet, &reglages, &OutilsMock, &mut vigie, &Silencieux).await;
        assert!(arret.is_none());
        assert_eq!(carnet.recolte_web, 3);
        // observations in original order despite the parallelism
        let obs: Vec<&str> = carnet
            .historique
            .iter()
            .filter_map(|m| m.outil.as_deref())
            .collect();
        assert_eq!(obs, vec!["web_a", "web_b", "web_c"]);
    }

    #[tokio::test]
    async fn profil_fragile_reste_sequentiel() {
        // Same input, Fragile profile -> no parallelism, but identical result.
        let appels = vec![
            Appel::nouveau("web_a", json!({})),
            Appel::nouveau("web_b", json!({})),
        ];
        let mut carnet = Carnet::ouvrir("m", ModeMission::Standard, t0());
        let reglages = Reglages { profil: ProfilModele::Fragile, ..Reglages::default() };
        let mut vigie = Vigie::nouvelle(ProfilModele::Fragile.seuils_vigie());
        recolter(&appels, &mut carnet, &reglages, &OutilsMock, &mut vigie, &Silencieux).await;
        assert_eq!(carnet.recolte_web, 2);
    }
}
