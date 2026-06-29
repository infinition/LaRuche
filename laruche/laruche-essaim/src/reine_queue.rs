//! Persistence and write-gating for the proposals queue (`laruche-reine-queue.json`).
//!
//! When the queue gate is on, curateur memory writes land here as [`Proposition`]s
//! instead of being applied; a human approves or rejects them from the Memory UI.
//! The backlog is a first-class store: disabling the gate never touches it, it only
//! stops gating new writes (see [`crate::reine_file::transition_desactivation`]).

use crate::reine_file::{classifier_risque, disposition, Disposition, Proposition, Statut, TypeProposition};
use laruche_butinage::cap::reine::ModeReine;
use laruche_memoire::{MemoireCognitive, MemoryItem};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

const QUEUE_FILE: &str = "laruche-reine-queue.json";

fn maintenant_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn id_unique(prefixe: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{prefixe}-{nanos}")
}

/// Load the queue (empty when absent or invalid).
pub fn charger() -> Vec<Proposition> {
    std::fs::read_to_string(QUEUE_FILE)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn sauver(props: &[Proposition]) {
    if let Ok(j) = serde_json::to_string_pretty(props) {
        let _ = std::fs::write(QUEUE_FILE, j);
    }
}

/// Append a proposal to the backlog.
pub fn enfiler(p: Proposition) {
    let mut props = charger();
    props.push(p);
    sauver(&props);
}

/// Route a curateur memory write through the gate. When the gate is off it applies
/// directly. When on, the disposition policy decides: in Off/Humaine mode every
/// write is queued for human review; in Auto/Hybride the Reine auto-applies safe
/// writes (new, non-colliding facts) and queues only the risky ones. Returns true
/// when the write was queued (left for a human).
pub async fn proposer_memoire(
    memoire: &Arc<dyn MemoireCognitive>,
    item: MemoryItem,
    gate: bool,
    mode: &str,
    provenance: &str,
) -> bool {
    if !gate {
        let _ = memoire.write(item).await;
        return false;
    }
    let cible = item.node_id.clone();
    // Does the target already hold items? Update vs add (for display + risk class).
    let existe = memoire
        .read_node(&cible)
        .await
        .ok()
        .and_then(|n| {
            n.get("items")
                .and_then(|i| i.as_array())
                .map(|a| !a.is_empty())
        })
        .unwrap_or(false);
    let type_ = if existe {
        TypeProposition::MemoireMaj
    } else {
        TypeProposition::MemoireAjout
    };
    let risque = classifier_risque(type_, false);
    let confiance = item
        .confidence
        .map(|c| (c * 100.0).clamp(0.0, 100.0) as u8)
        .unwrap_or(80);

    // Gate on but Reine off -> a pure human gate: queue everything. Otherwise the
    // disposition policy decides (Humaine queues all, Auto/Hybride auto-applies safe).
    let m = ModeReine::depuis_str(mode);
    let disp = if m == ModeReine::Off {
        Disposition::MettreEnFile
    } else {
        disposition(m, risque, confiance, 60)
    };
    if disp == Disposition::AutoApprouver {
        let _ = memoire.write(item).await;
        return false;
    }

    let apercu = item.content.clone();
    let p = Proposition {
        id: id_unique(&cible),
        type_,
        cible: Some(cible),
        base_version: None,
        contenu: serde_json::to_string(&item).unwrap_or_default(),
        provenance: provenance.to_string(),
        raison: apercu,
        ecrase_existant: false,
        statut: Statut::EnAttente,
        cree_a: maintenant_secs(),
    };
    enfiler(p);
    true
}

/// Apply a proposal's change to the cognitive memory.
async fn appliquer(memoire: &Arc<dyn MemoireCognitive>, p: &Proposition) -> bool {
    match p.type_ {
        TypeProposition::MemoireAjout | TypeProposition::MemoireMaj => {
            match serde_json::from_str::<MemoryItem>(&p.contenu) {
                Ok(item) => memoire.write(item).await.is_ok(),
                Err(_) => false,
            }
        }
        TypeProposition::MemoireSuppr => match &p.cible {
            Some(node) => memoire.delete_node(node).await.is_ok(),
            None => false,
        },
        // Skill/tool/mission proposals carry no memory write.
        _ => false,
    }
}

/// Approve a proposal: apply it and mark it approved. Returns true on success.
pub async fn approuver(memoire: &Arc<dyn MemoireCognitive>, id: &str) -> bool {
    let mut props = charger();
    let Some(idx) = props.iter().position(|p| p.id == id) else {
        return false;
    };
    if !props[idx].statut.actionnable() {
        return false;
    }
    let ok = appliquer(memoire, &props[idx]).await;
    if ok {
        props[idx].statut = Statut::Approuve;
        sauver(&props);
    }
    ok
}

/// Reject a proposal (kept for audit, not applied).
pub fn rejeter(id: &str) -> bool {
    let mut props = charger();
    if let Some(p) = props.iter_mut().find(|p| p.id == id) {
        if p.statut.actionnable() {
            p.statut = Statut::Rejete;
            sauver(&props);
            return true;
        }
    }
    false
}

/// Mark pending proposals older than `ttl_secondes` as expired (anti-rot policy,
/// independent of the Reine toggle). Returns how many were aged out.
pub fn purger_perimes(ttl_secondes: i64) -> usize {
    let now = maintenant_secs();
    let mut props = charger();
    let mut n = 0;
    for p in props.iter_mut() {
        if p.perime(now, ttl_secondes) {
            p.statut = Statut::Perime;
            n += 1;
        }
    }
    if n > 0 {
        sauver(&props);
    }
    n
}

/// Approve every pending **safe** proposal (additions of new, non-colliding info)
/// in one go. Returns how many were applied.
pub async fn approuver_surs(memoire: &Arc<dyn MemoireCognitive>) -> usize {
    let ids: Vec<String> = charger()
        .into_iter()
        .filter(|p| p.statut == Statut::EnAttente && p.risque() == crate::reine_file::Risque::Sur)
        .map(|p| p.id)
        .collect();
    let mut n = 0;
    for id in ids {
        if approuver(memoire, &id).await {
            n += 1;
        }
    }
    n
}
