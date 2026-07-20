//! The **recolte**: execution of a pass's tools.
//!
//! Claude Code-style partitioning: consecutive **read-only** calls are harvested
//! **in parallel** (bounded), while mutating/approval calls stay **sequential**
//! and isolated. The original ordering of observations is preserved (including
//! vigie blocks, interleaved at their call position).
//! The [`Vigie`] is consulted before each call (block) and after (signal).
//!
//! Engine-level guarantees, whatever the tools do:
//! - every call is **bounded in time** ([`Reglages::timeout_outil_secs`], overridable
//!   per tool via [`Outils::timeout_secs`]);
//! - every observation is **bounded in size** ([`Reglages::max_chars_observation`]);
//! - a tool_result is **always** reinjected, even on failure/timeout/block (models
//!   derail when the expected tool turn is missing from the context);
//! - the cancellation flag is honored between calls.

use crate::carnet::Carnet;
use crate::cap::vigie::{Signal, Vigie};
use crate::evenement::{Emetteur, Evenement};
use crate::issue::{Appel, Bilan, FinDeVol};
use crate::messagerie::Message;
use crate::outils::{Outils, ResultatOutil};
use crate::reglages::Reglages;
use futures_util::future::join_all;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

/// Ceiling on calls launched simultaneously within a parallel batch.
const MAX_PARALLELE: usize = 6;

/// Outcome of one pass's harvest.
pub struct Moisson {
    /// `Some(bilan)`: the loop must land (sterile loop / interruption).
    pub arret: Option<Bilan>,
    /// Number of calls that actually EXECUTED (blocked calls do not count).
    /// The loop uses it to decide whether real progress happened this pass.
    pub executes: usize,
}

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

/// Executes a call bounded by the effective timeout (per-tool override, otherwise the
/// reglages default; `0` = unbounded). A timeout becomes an observable failure.
async fn executer_borne(outils: &dyn Outils, appel: &Appel, defaut_secs: u64) -> ResultatOutil {
    let secs = outils.timeout_secs(&appel.nom).unwrap_or(defaut_secs);
    if secs == 0 {
        return outils.executer(appel).await;
    }
    match tokio::time::timeout(std::time::Duration::from_secs(secs), outils.executer(appel)).await
    {
        Ok(r) => r,
        Err(_) => ResultatOutil::echec(format!(
            "Tool `{}` timed out after {secs}s. The operation was aborted; try a narrower \
             scope, different arguments, or another tool.",
            appel.nom
        )),
    }
}

fn annule(flag: Option<&AtomicBool>) -> bool {
    flag.is_some_and(|f| f.load(Ordering::Relaxed))
}

fn bilan_interrompu(carnet: &mut Carnet) -> Bilan {
    carnet.itineraire.finaliser();
    Bilan::nouveau(
        "Interrupted by the user.",
        FinDeVol::Interrompue,
        carnet.passe + 1,
    )
}

/// Executes all calls of a pass. `moisson.arret` is `Some` if the vigie forces a
/// stop (sterile loop) or the run was cancelled.
pub async fn recolter(
    appels: &[Appel],
    carnet: &mut Carnet,
    reglages: &Reglages,
    outils: &dyn Outils,
    vigie: &mut Vigie,
    emet: &dyn Emetteur,
    annulation: Option<&AtomicBool>,
) -> Moisson {
    let parallele = reglages.profil.parallelisme();
    let mut executes = 0usize;

    // Intra-pass dedup: local models routinely emit the SAME call twice in one message.
    // Only the first occurrence executes; duplicates get a synthetic observation (a
    // tool_result is still ALWAYS reinjected). Also spares double-firing mutations.
    let mut doublons: std::collections::HashSet<usize> = std::collections::HashSet::new();
    {
        let mut vues: std::collections::HashSet<u64> = std::collections::HashSet::new();
        for (i, a) in appels.iter().enumerate() {
            if !vues.insert(a.signature()) {
                doublons.insert(i);
            }
        }
    }
    const MSG_DOUBLON: &str = "duplicate call: same tool with identical arguments as an \
        earlier call in this same message. Not re-executed - use that call's result.";

    for (sur, idxs) in partitionner(appels, outils) {
        if annule(annulation) {
            return Moisson { arret: Some(bilan_interrompu(carnet)), executes };
        }
        if sur && parallele && idxs.len() > 1 {
            // ── Parallel batch (read-only) ──
            // Pre-filter via the vigie (avant_appel is non-mutating). Blocked calls keep
            // their index so the observation order matches the call order.
            let mut bloques: HashMap<usize, String> = HashMap::new();
            let mut a_lancer: Vec<usize> = Vec::new();
            for &i in &idxs {
                if doublons.contains(&i) {
                    bloques.insert(i, MSG_DOUBLON.to_string());
                } else if let Signal::Bloquer(msg) = vigie.avant_appel(appels[i].signature()) {
                    bloques.insert(i, msg);
                } else {
                    a_lancer.push(i);
                }
            }
            if a_lancer.len() > 1 {
                emet.emettre(Evenement::Statut(format!(
                    "Parallel récolte of {} tools...",
                    a_lancer.len()
                )));
            }
            let mut resultats: HashMap<usize, (ResultatOutil, u64)> = HashMap::new();
            for groupe in a_lancer.chunks(MAX_PARALLELE) {
                if annule(annulation) {
                    return Moisson { arret: Some(bilan_interrompu(carnet)), executes };
                }
                for &i in groupe {
                    emet.emettre(Evenement::AppelOutil { nom: appels[i].nom.clone() });
                }
                let futs = groupe.iter().map(|&i| {
                    let appel = &appels[i];
                    async move {
                        let t0 = Instant::now();
                        let res = executer_borne(outils, appel, reglages.timeout_outil_secs).await;
                        (i, res, t0.elapsed().as_millis() as u64)
                    }
                });
                for (i, res, ms) in join_all(futs).await {
                    resultats.insert(i, (res, ms));
                }
            }
            // Apply in ORIGINAL order, blocks interleaved at their position.
            for &i in &idxs {
                if let Some(msg) = bloques.get(&i) {
                    pousser_blocage(carnet, &appels[i], msg, emet);
                } else if let Some((res, ms)) = resultats.remove(&i) {
                    executes += 1;
                    if let Some(bilan) =
                        appliquer(&appels[i], res, ms, carnet, reglages, outils, vigie, emet)
                    {
                        return Moisson { arret: Some(bilan), executes };
                    }
                }
            }
        } else {
            // ── Sequential (mutating, approval, or non-parallel profile) ──
            for &i in &idxs {
                if annule(annulation) {
                    return Moisson { arret: Some(bilan_interrompu(carnet)), executes };
                }
                let appel = &appels[i];
                if doublons.contains(&i) {
                    pousser_blocage(carnet, appel, MSG_DOUBLON, emet);
                    continue;
                }
                if let Signal::Bloquer(msg) = vigie.avant_appel(appel.signature()) {
                    pousser_blocage(carnet, appel, &msg, emet);
                    continue;
                }
                emet.emettre(Evenement::AppelOutil { nom: appel.nom.clone() });
                let t0 = Instant::now();
                let res = executer_borne(outils, appel, reglages.timeout_outil_secs).await;
                let ms = t0.elapsed().as_millis() as u64;
                executes += 1;
                if let Some(bilan) =
                    appliquer(appel, res, ms, carnet, reglages, outils, vigie, emet)
                {
                    return Moisson { arret: Some(bilan), executes };
                }
            }
        }
    }
    Moisson { arret: None, executes }
}

/// Caps an observation to `max` characters, keeping head + tail (the head carries the
/// payload, the tail often carries totals/errors). `0` = no cap.
pub fn plafonner_observation(s: &str, max: usize) -> String {
    if max == 0 {
        return s.to_string();
    }
    let n = s.chars().count();
    if n <= max {
        return s.to_string();
    }
    let tete = max * 3 / 4;
    let queue = max / 4;
    let debut: String = s.chars().take(tete).collect();
    let fin: String = s
        .chars()
        .skip(n.saturating_sub(queue))
        .collect();
    format!(
        "{debut}\n\n[... observation truncated: {} of {n} characters shown. Narrow the \
         query (pagination, filters, offsets) if you need the elided middle. ...]\n\n{fin}",
        tete + queue
    )
}

/// Applies the result of a call: web counter, vigie, event, observation.
/// Returns `Some(bilan)` if the vigie forces a stop.
#[allow(clippy::too_many_arguments)]
fn appliquer(
    appel: &Appel,
    res: ResultatOutil,
    ms: u64,
    carnet: &mut Carnet,
    reglages: &Reglages,
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

    let mut observation = plafonner_observation(&res.sortie, reglages.max_chars_observation);
    if let Signal::Avertir(m) | Signal::Poser(m) = &signal {
        observation.push_str(&format!("\n\n[vigie: {m}]"));
    }
    carnet
        .historique
        .push(Message::observation_liee(&appel.nom, &appel.id, observation));

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

fn pousser_blocage(carnet: &mut Carnet, appel: &Appel, msg: &str, emet: &dyn Emetteur) {
    carnet
        .historique
        .push(Message::observation_liee(&appel.nom, &appel.id, format!("Blocked: {msg}")));
    emet.emettre(Evenement::ResultatOutil { nom: appel.nom.clone(), ok: false, ms: 0 });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::carnet::ModeMission;
    use crate::cap::vigie::SeuilsVigie;
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

    /// Tool that hangs forever: only the timeout brings it back.
    struct OutilsLent;
    #[async_trait]
    impl Outils for OutilsLent {
        async fn executer(&self, _appel: &Appel) -> ResultatOutil {
            tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
            ResultatOutil::ok("jamais")
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
        let moisson =
            recolter(&appels, &mut carnet, &reglages, &OutilsMock, &mut vigie, &Silencieux, None)
                .await;
        assert!(moisson.arret.is_none());
        assert_eq!(moisson.executes, 3);
        assert_eq!(carnet.recolte_web, 3);
        // observations in original order despite the parallelism
        let obs: Vec<&str> = carnet
            .historique
            .iter()
            .filter_map(|m| m.outil.as_deref())
            .collect();
        assert_eq!(obs, vec!["web_a", "web_b", "web_c"]);
        // observations carry the id of the call that produced them
        assert!(carnet.historique.iter().all(|m| m.appel_id.is_some()));
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
        recolter(&appels, &mut carnet, &reglages, &OutilsMock, &mut vigie, &Silencieux, None)
            .await;
        assert_eq!(carnet.recolte_web, 2);
    }

    #[tokio::test]
    async fn outil_pendu_est_borne_par_le_timeout() {
        let appels = vec![Appel::nouveau("lent", json!({}))];
        let mut carnet = Carnet::ouvrir("m", ModeMission::Standard, t0());
        let reglages = Reglages { timeout_outil_secs: 1, ..Reglages::default() };
        let mut vigie = Vigie::nouvelle(SeuilsVigie::default());
        let moisson =
            recolter(&appels, &mut carnet, &reglages, &OutilsLent, &mut vigie, &Silencieux, None)
                .await;
        assert!(moisson.arret.is_none());
        assert_eq!(moisson.executes, 1);
        // a tool_result is still reinjected (never omitted)
        let obs = carnet.historique.last().unwrap();
        assert!(obs.contenu.contains("timed out"));
    }

    #[tokio::test]
    async fn appels_bloques_ne_comptent_pas_comme_executes() {
        let appel = Appel::nouveau("web_x", json!({"q": "y"}));
        let mut vigie = Vigie::nouvelle(SeuilsVigie::default());
        // Pre-seed 5 exact failures: avant_appel now blocks this signature.
        for _ in 0..5 {
            vigie.apres_appel("web_x", appel.signature(), false, false, 0);
        }
        let mut carnet = Carnet::ouvrir("m", ModeMission::Standard, t0());
        let reglages = Reglages::default();
        let moisson = recolter(
            &[appel],
            &mut carnet,
            &reglages,
            &OutilsMock,
            &mut vigie,
            &Silencieux,
            None,
        )
        .await;
        assert!(moisson.arret.is_none());
        assert_eq!(moisson.executes, 0, "a blocked call is not an execution");
        assert!(carnet.historique.last().unwrap().contenu.starts_with("Blocked:"));
    }

    #[tokio::test]
    async fn doublon_intra_passe_n_execute_qu_une_fois() {
        // The same call emitted twice in one message: one execution, but TWO
        // observations (a tool_result is always reinjected, even for the duplicate).
        let appels = vec![
            Appel::nouveau("web_a", json!({"q": "x"})),
            Appel::nouveau("web_a", json!({"q": "x"})),
            Appel::nouveau("web_b", json!({})),
        ];
        let mut carnet = Carnet::ouvrir("m", ModeMission::Standard, t0());
        let reglages = Reglages { profil: ProfilModele::Robuste, ..Reglages::default() };
        let mut vigie = Vigie::nouvelle(ProfilModele::Robuste.seuils_vigie());
        let moisson =
            recolter(&appels, &mut carnet, &reglages, &OutilsMock, &mut vigie, &Silencieux, None)
                .await;
        assert_eq!(moisson.executes, 2, "the duplicate must not execute");
        let obs: Vec<&Message> = carnet.historique.iter().filter(|m| m.outil.is_some()).collect();
        assert_eq!(obs.len(), 3, "every call still gets an observation");
        assert!(obs[1].contenu.contains("duplicate call"));
        // sequential path too (Fragile profile)
        let mut carnet2 = Carnet::ouvrir("m", ModeMission::Standard, t0());
        let reglages2 = Reglages { profil: ProfilModele::Fragile, ..Reglages::default() };
        let mut vigie2 = Vigie::nouvelle(ProfilModele::Fragile.seuils_vigie());
        let m2 = recolter(&appels, &mut carnet2, &reglages2, &OutilsMock, &mut vigie2, &Silencieux, None)
            .await;
        assert_eq!(m2.executes, 2);
    }

    #[tokio::test]
    async fn annulation_interrompt_proprement() {
        let appels = vec![Appel::nouveau("web_a", json!({}))];
        let mut carnet = Carnet::ouvrir("m", ModeMission::Standard, t0());
        let reglages = Reglages::default();
        let mut vigie = Vigie::nouvelle(SeuilsVigie::default());
        let flag = AtomicBool::new(true);
        let moisson = recolter(
            &appels,
            &mut carnet,
            &reglages,
            &OutilsMock,
            &mut vigie,
            &Silencieux,
            Some(&flag),
        )
        .await;
        let bilan = moisson.arret.expect("must land");
        assert_eq!(bilan.fin, FinDeVol::Interrompue);
        assert_eq!(moisson.executes, 0);
    }

    #[test]
    fn plafonner_garde_tete_et_queue() {
        let s = "a".repeat(500) + &"z".repeat(500);
        let p = plafonner_observation(&s, 100);
        assert!(p.starts_with("aaa"));
        assert!(p.ends_with("zzz"));
        assert!(p.contains("truncated"));
        // untouched below the cap
        assert_eq!(plafonner_observation("court", 100), "court");
        assert_eq!(plafonner_observation(&s, 0), s); // 0 = disabled
    }
}
