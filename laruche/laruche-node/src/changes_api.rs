//! Memory change-sync (disk-to-SQL skill sync, OKF change import/export, mesh pull) and state version endpoint - split out of main.rs.

use crate::*;
use axum::extract::State;
use axum::response::Json;
use std::sync::Arc;

/// True when this node came FROM the disk (every item carries `source: skill-file`)
/// and its folder is gone.
///
/// Deleting or renaming a `skills/<name>/` folder used to leave its copy in SQL
/// forever: the cut of 45 skills and six renames left 51 ghosts in the catalog, each
/// still costing its description in the prompt on every single turn, and several
/// shadowing the skill that replaced them.
///
/// Provenance is what makes this safe. A skill the agent forged with `skill_create`
/// exists only in SQL and carries another source, so it is never touched. A backend
/// that does not expose `source` at all reports `false` here and sweeps nothing,
/// which is the right way to fail.
fn supprimee_du_disque(
    node: &serde_json::Value,
    id: &str,
    sur_disque: &std::collections::HashSet<String>,
) -> bool {
    if sur_disque.is_empty() || sur_disque.contains(id) {
        return false;
    }
    node["items"]
        .as_array()
        .filter(|items| !items.is_empty())
        .is_some_and(|items| {
            items.iter().all(|it| it["source"].as_str() == Some("skill-file"))
        })
}

/// Sweep the `capacities.skills.*` catalog of what is no longer a real skill.
///
/// TWO rules, both conservative:
///
/// 1. A node holding NO skill document is an empty shell. The writer and the reader
///    disagreed on `-` versus `_` for a long time, so the same skill was created
///    twice and one copy never received a body: 88 children for 73 folders. Listing
///    a name that `skill_view` cannot open is worse than not listing it.
/// 2. A node that CAME from the disk whose folder is gone, see
///    [`supprimee_du_disque`]. Without this, deleting or renaming a skill folder
///    left its copy in the catalog forever.
///
/// A skill living only in memory, forged by the agent or the curator, matches
/// neither rule and is never touched.
///
/// `sur_disque` = node ids for the `skills/*/SKILL.md` folders seen in this scan.
/// Empty means "no disk scan happened", and then nothing is swept on that basis.
async fn reconcilier_skills_orphelines(
    memoire: &Arc<dyn laruche_memoire::MemoireCognitive>,
    sur_disque: &std::collections::HashSet<String>,
) {
    let Ok(racine) = memoire.read_node("capacities.skills").await else {
        return;
    };
    let Some(enfants) = racine["children"].as_array() else {
        return;
    };
    let ids: Vec<String> = enfants
        .iter()
        .filter_map(|e| e.get("id").or_else(|| e.get("node_id")))
        .filter_map(|v| v.as_str())
        .map(str::to_string)
        .collect();

    // Node ids that hold a PROPOSED item, which `read_node` never returns: it filters
    // on `status='active'`.
    //
    // Without this the sweep destroyed exactly the skills waiting for the user. With
    // `queue_gate` on, every skill the agent forges lands as a proposal, so it has no
    // active item, so the shell rule below called it an empty node and deleted it, and
    // `delete_node` reparented the proposal into `orphans.<slug>_<ts>`. Found 87 orphan
    // nodes and a review queue full of skill markdown that way.
    let en_attente: std::collections::HashSet<String> = memoire
        .list_proposed(Some(200))
        .await
        .ok()
        // ATTENTION: list_proposed rend un OBJET {count, items}, pas un tableau. Un
        // `as_array()` dessus echoue toujours, la garde etait donc toujours vide et
        // le balayage supprimait exactement ce qu'elle devait epargner. Le
        // commentaire ci-dessus decrivait une protection qui n'a jamais fonctionne.
        .and_then(|v| v.get("items").and_then(|i| i.as_array()).cloned())
        .map(|items| {
            items
                .iter()
                .filter_map(|it| it.get("node_id").and_then(|v| v.as_str()))
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();

    let mut supprimes = 0usize;
    for id in ids {
        // A node whose document is merely awaiting review is not a shell.
        if en_attente.contains(&id) {
            continue;
        }
        let Ok(node) = memoire.read_node(&id).await else {
            continue;
        };
        let a_document = node["items"]
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .any(|it| it["content"].as_str().is_some_and(|c| c.contains("type: skill")))
            })
            .unwrap_or(false);
        let du_disque_disparu = supprimee_du_disque(&node, &id, sur_disque);
        if a_document && !du_disque_disparu {
            continue;
        }
        // Dire QUELLE regle supprime, et sur quel skill.
        //
        // Le compte seul ("swept count=37") ne permet pas de distinguer les deux
        // regles, et elles n'ont pas du tout la meme gravite: une coquille sans
        // document est un dechet, un skill present sur le disque qui se fait balayer
        // est une perte. Trente-neuf skills synchronises puis trente-sept balayes au
        // meme demarrage, ce n'est pas du menage, et la corbeille le montre: le meme
        // `arxiv` supprime et recree a chaque lancement, une copie horodatee par
        // passage.
        tracing::warn!(
            skill = %id,
            sans_document = !a_document,
            absent_du_disque = du_disque_disparu,
            skills_vus_sur_disque = sur_disque.len(),
            "skill balaye du catalogue"
        );
        // delete_node reparents to `orphans.*`, so remove that residue too.
        //
        // L'adresse du residu se LIT dans la reponse, elle ne se devine pas. Elle
        // etait reconstruite en `orphans.<nom>`, alors que `delete_node` y ajoute un
        // horodatage pour eviter les collisions: la cible n'existait donc jamais, le
        // residu n'etait jamais nettoye, et chaque demarrage en deposait une fournee
        // de plus. Trente-sept par demarrage, indefiniment: 328 noeuds dans la
        // corbeille en une matinee, tous des copies horodatees des memes skills.
        if let Ok(res) = memoire.delete_node(&id).await {
            if let Some(dest) = res.get("relocated_to").and_then(|v| v.as_str()) {
                // `orphans.*` prend la branche de suppression definitive.
                let _ = memoire.delete_node(dest).await;
            }
            supprimes += 1;
        }
    }
    if supprimes > 0 {
        tracing::info!(count = supprimes, "empty skill nodes swept from the catalog");
    }
}

/// Rattrape les propositions memoire restees en rade, une fois pour toutes.
///
/// `propose_write` posait un item en `status='proposed'` et s'arretait la: aucune
/// route ne sait approuver un item memoire, `/api/memory/proposed` est en lecture
/// seule. Ils s'accumulaient donc, comptes par le bandeau ("9 en attente") mais
/// absents du panneau, qui lit la file de LaReine. Un skill ainsi propose avait de
/// surcroit l'air vide, une lecture de noeud ne rendant que les items actifs.
///
/// Les ecrivains sont corriges, mais ceux qui sont deja poses ne se deplaceront pas
/// tout seuls. On les verse dans la file de LaReine, puis on retire la ligne
/// d'origine: sans cela, approuver ecrirait une copie active a cote de la
/// proposition, qui continuerait de compter dans le bandeau.
async fn rattraper_propositions_orphelines(memoire: &Arc<dyn laruche_memoire::MemoireCognitive>) {
    let Ok(v) = memoire.list_proposed(Some(200)).await else {
        return;
    };
    let items: Vec<serde_json::Value> = v
        .get("items")
        .and_then(|i| i.as_array())
        .cloned()
        .unwrap_or_default();
    if items.is_empty() {
        return;
    }
    let mut verses = 0usize;
    for it in &items {
        let (Some(id), Some(node), Some(contenu)) = (
            it.get("id").and_then(|x| x.as_str()),
            it.get("node_id").and_then(|x| x.as_str()),
            it.get("content").and_then(|x| x.as_str()),
        ) else {
            continue;
        };
        let source = it.get("source").and_then(|x| x.as_str()).unwrap_or("rattrapage");
        // Un skill garde son chemin d'application a lui: approuver un `SkillNouveau`
        // ecrit le noeud ET le fichier SKILL.md, ce qu'un simple ajout memoire ne
        // ferait pas, et le skill resterait invisible sur le disque.
        if node.starts_with("capacities.skills.") {
            laruche_essaim::reine_queue::proposer_skill(node, contenu, source);
        } else {
            laruche_essaim::reine_queue::proposer_memoire(
                memoire,
                laruche_memoire::MemoryItem::new(node, contenu).with_source(source),
                true,
                "hybride",
                source,
            )
            .await;
        }
        let _ = memoire.delete_item(id, Some("rattrapage-file-unique")).await;
        verses += 1;
    }
    if verses > 0 {
        tracing::info!(
            count = verses,
            "propositions memoire orphelines versees dans la file de LaReine"
        );
    }
}

/// Phase 1 - DISK -> SQL sync: scans `skills/*/SKILL.md` and upserts each skill into
/// `capacities.skills.<slug>` (single item), using the SAME id function as the reader.
/// Additive for content (an SQL-only skill is never dropped); it does sweep nodes left
/// without any document, which are shells rather than skills.
pub(crate) async fn sync_skills_disk_to_sql(memoire: &Arc<dyn laruche_memoire::MemoireCognitive>) {
    let dir = std::path::Path::new("skills");
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    let mut n = 0usize;
    let mut sur_disque: std::collections::HashSet<String> = std::collections::HashSet::new();
    for e in rd.flatten() {
        let p = e.path();
        if !p.is_dir() {
            continue;
        }
        let Ok(content) = std::fs::read_to_string(p.join("SKILL.md")) else {
            continue;
        };
        let content = content.replace("\r\n", "\n"); // normalize (SQL in LF)
        if !content.contains("type: skill") {
            continue; // only real OKF skills
        }
        let Some(slug) = p.file_name().and_then(|x| x.to_str()).filter(|s| !s.is_empty()) else {
            continue;
        };
        // Same function as the READER (`skill_view`). Formatting this id by hand is
        // exactly how the two drifted: the folder name went in verbatim while the
        // reader normalised it, and 40 of the 73 shipped skills became unreachable.
        let node_id = laruche_skills::skill_node_id(slug);
        // Recorded BEFORE the incremental skip below: an unchanged skill is still on
        // disk, and the sweep needs the full picture, not just what we rewrote.
        sur_disque.insert(node_id.clone());
        let existing = memoire.read_node(&node_id).await.ok();
        // INCREMENTAL: skip when the SQL copy already matches the disk file. The
        // unconditional delete+rewrite re-embedded every skill at every boot and
        // could consult the write arbiter (aux LLM) per skill; with the LLM busy,
        // startup crawled for minutes. Unchanged file = untouched row.
        let identique = existing
            .as_ref()
            .and_then(|node| node.get("items").and_then(|i| i.as_array()))
            .map(|items| {
                items.len() == 1
                    && items[0].get("content").and_then(|c| c.as_str())
                        == Some(content.as_str())
            })
            .unwrap_or(false);
        if identique {
            continue;
        }
        // Update IN PLACE when the node already holds exactly one item, which is the
        // shape a skill always has.
        //
        // This used to delete then write. A delete here is a SOFT delete that nothing
        // ever purges, so every edited skill left a tombstone at every boot: 11 792 of
        // them, 94% of all the rows in the base, and 364 MB on disk. Updating keeps one
        // row per skill forever.
        let items_existants = existing
            .as_ref()
            .and_then(|n| n.get("items").and_then(|i| i.as_array()).cloned())
            .unwrap_or_default();
        let id_unique = (items_existants.len() == 1)
            .then(|| items_existants[0].get("id").and_then(|x| x.as_str()))
            .flatten()
            .map(str::to_string);

        match id_unique {
            Some(id) => {
                let _ = memoire.update_item(&id, &content).await;
            }
            None => {
                // Zero items, or several: fall back to the old shape, which is also the
                // only way to converge a node that somehow accumulated copies.
                for it in &items_existants {
                    if let Some(id) = it.get("id").and_then(|x| x.as_str()) {
                        let _ = memoire.delete_item(id, Some("skill-file-sync")).await;
                    }
                }
                let _ = memoire
                    .write(
                        laruche_memoire::MemoryItem::new(node_id, content)
                            .with_source("skill-file"),
                    )
                    .await;
            }
        }
        n += 1;
    }
    if n > 0 {
        tracing::info!(count = n, "skills synchronized from disk (SKILL.md -> SQL)");
    }
    // Clear what the old delete-and-rewrite sync left behind. Runs at every boot, and
    // costs nothing once the base is clean: the first pass is the one that matters.
    match memoire.purger_tombes_skills().await {
        Ok(p) if p > 0 => {
            tracing::info!(count = p, "skill-file tombstones purged, database compacted")
        }
        Err(e) => tracing::warn!(error = %e, "skill tombstone purge failed"),
        _ => {}
    }
    reconcilier_skills_orphelines(memoire, &sur_disque).await;
    rattraper_propositions_orphelines(memoire).await;
    // Targeted purge of META-SKILLS from other agent frameworks (CLI docs of third-party agents),
    // wrongly imported: they describe ANOTHER agent, not LaRuche. Explicit DENYLIST: definitely
    // NOT a disk diff "delete everything not on disk" (that would destroy skills
    // created by the agent, like arxiv_search). Hard-delete:
    // delete_node reparents to `orphans.*`, so we also delete the resulting orphan.
    //
    // `web_research` is on the list for a different reason: it used to be seeded in
    // code at boot, and the disk skill `web-research` now covers it. Two entries with
    // overlapping descriptions is precisely what makes a model hesitate and pick the
    // wrong one, so the superseded copy goes. The seeding block is gone from main.rs;
    // this line clears what earlier boots already wrote.
    const META_SKILLS_A_PURGER: &[&str] = &[
        "claude-code",
        "codex",
        "opencode",
        "web_research",
    ];
    let mut purges = 0usize;
    for slug in META_SKILLS_A_PURGER {
        let node_id = laruche_skills::skill_node_id(slug);
        if memoire.read_node(&node_id).await.is_err() {
            continue; // absent -> nothing to do
        }
        if let Ok(r) = memoire.delete_node(&node_id).await {
            purges += 1;
            // delete_node moved it to orphans.<base>_<ts> -> hard-delete this orphan.
            if let Some(orphan) = r.get("relocated_to").and_then(|v| v.as_str()) {
                let _ = memoire.delete_node(orphan).await;
            }
        }
    }
    if purges > 0 {
        tracing::info!(count = purges, "meta-skills from other frameworks purged (denylist)");
    }
}

/// Imports a list of facts `{node_id, content}` into memory (exact dedup). (imported, skipped).
pub(crate) async fn importer_changes(
    state: &Arc<AppState>,
    items: &[serde_json::Value],
    src: &str,
) -> (usize, usize) {
    let (mut imported, mut skipped) = (0usize, 0usize);
    for it in items {
        let node = it["node_id"].as_str().unwrap_or("").trim();
        let content = it["content"].as_str().unwrap_or("");
        if node.is_empty() || content.trim().is_empty() {
            continue;
        }
        // Exact dedup: if an identical item already exists in this node, skip.
        let exists = state
            .memoire
            .grep(content, Some(8))
            .await
            .ok()
            .and_then(|g| {
                g["items"].as_array().map(|a| {
                    a.iter().any(|x| {
                        x["node_id"].as_str() == Some(node) && x["content"].as_str() == Some(content)
                    })
                })
            })
            .unwrap_or(false);
        if exists {
            skipped += 1;
            continue;
        }
        let _ = state
            .memoire
            .write(
                laruche_memoire::MemoryItem::new(node.to_string(), content.to_string())
                    .with_source(src),
            )
            .await;
        imported += 1;
    }
    (imported, skipped)
}

/// GET /api/memory/export_changes?since=<ts> - facts (op=write) written since `since`, for
/// mesh federation (Lever 3, first slice). Excludes system/capacities projections.
pub(crate) async fn api_memory_export_changes(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(q): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let since = q.get("since").and_then(|s| s.parse::<i64>().ok()).unwrap_or(0);
    let muts = state
        .memoire
        .mutations(Some(250))
        .await
        .unwrap_or_else(|_| serde_json::json!({}));
    let items: Vec<serde_json::Value> = muts["mutations"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter(|m| {
                    m["op"].as_str() == Some("write")
                        && m["ts"].as_i64().unwrap_or(0) > since
                        && {
                            let n = m["node_id"].as_str().unwrap_or("");
                            !n.starts_with("capacities") && !n.starts_with("system")
                        }
                })
                .map(|m| serde_json::json!({ "node_id": m["node_id"], "content": m["content"], "ts": m["ts"] }))
                .collect()
        })
        .unwrap_or_default();
    let count = items.len();
    Json(serde_json::json!({ "items": items, "count": count }))
}

/// POST /api/memory/import_changes {items:[{node_id,content}], source?} - applies facts (dedup).
pub(crate) async fn api_memory_import_changes(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let src = body["source"].as_str().unwrap_or("mesh").to_string();
    let empty: Vec<serde_json::Value> = vec![];
    let items = body["items"].as_array().unwrap_or(&empty);
    let (imported, skipped) = importer_changes(&state, items, &src).await;
    Json(serde_json::json!({ "imported": imported, "skipped": skipped }))
}

/// POST /api/memory/mesh_pull {peer, since?} - pulls facts from a PEER node (Miel) and imports them
/// locally. First building block of the mesh's COLLECTIVE memory (Lever 3).
pub(crate) async fn api_memory_mesh_pull(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let peer = body["peer"]
        .as_str()
        .unwrap_or("")
        .trim()
        .trim_end_matches('/')
        .to_string();
    if peer.is_empty() {
        return Json(serde_json::json!({ "error": "missing peer (e.g. http://192.168.1.20:8419)" }));
    }
    let since = body["since"].as_i64().unwrap_or(0);
    let url = format!("{peer}/api/memory/export_changes?since={since}");
    let data: serde_json::Value = match reqwest::get(&url).await {
        Ok(r) => match r.json().await {
            Ok(v) => v,
            Err(e) => return Json(serde_json::json!({ "error": format!("peer json: {e}") })),
        },
        Err(e) => return Json(serde_json::json!({ "error": format!("peer contact: {e}") })),
    };
    let empty: Vec<serde_json::Value> = vec![];
    let items = data["items"].as_array().unwrap_or(&empty);
    let src = format!("mesh:{peer}");
    let (imported, skipped) = importer_changes(&state, items, &src).await;
    Json(serde_json::json!({ "pulled_from": peer, "imported": imported, "skipped": skipped }))
}

/// GET /api/state/version - ts of the last memory mutation (P7 lite: the UI polls to know
/// whether to refresh, without a push channel).
pub(crate) async fn api_state_version(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let v = state
        .memoire
        .mutations(Some(1))
        .await
        .ok()
        .and_then(|m| {
            m["mutations"]
                .as_array()
                .and_then(|a| a.first())
                .and_then(|x| x["ts"].as_i64())
        })
        .unwrap_or(0);
    Json(serde_json::json!({ "version": v }))
}

/// GET /api/version: la version de CE binaire.
///
/// `/api/state/version` porte deja un nom proche mais rend l'horodatage de la
/// derniere mutation de la memoire, ce qui n'a rien a voir. Ici c'est la version
/// du logiciel, celle qu'on compare a la derniere release publiee. Elle vient de
/// `CARGO_PKG_VERSION` et pas d'une constante ecrite a la main: une version
/// codee en dur ment des la publication suivante, et c'est precisement la chose
/// qu'une verification de mise a jour ne doit pas faire.
pub(crate) async fn api_version() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "version": env!("CARGO_PKG_VERSION") }))
}

/// POST /api/ouvrir {url} - ouvre une adresse dans le navigateur du systeme.
///
/// Sert a l'application de bureau, ou un lien `target="_blank"` ne fait rien:
/// la webview n'ouvre pas d'onglet et n'a pas de navigateur autour d'elle. La
/// page lui renvoie donc l'adresse, et le noeud la confie au systeme.
///
/// Le schema est verifie, et c'est la seule chose qui compte ici: cet appel
/// lance un programme choisi par le systeme d'exploitation. Un `file://`
/// ouvrirait un fichier local, un schema exotique un logiciel qu'on n'a pas
/// choisi. Seuls http et https passent.
pub(crate) async fn api_ouvrir(
    axum::extract::Json(body): axum::extract::Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let url = body["url"].as_str().unwrap_or("").trim().to_string();
    let sur = url.starts_with("http://") || url.starts_with("https://");
    if !sur {
        return Json(serde_json::json!({
            "status": "error",
            "error": "only http and https URLs can be opened"
        }));
    }
    match open::that_detached(&url) {
        Ok(()) => Json(serde_json::json!({ "status": "ok" })),
        Err(e) => Json(serde_json::json!({ "status": "error", "error": e.to_string() })),
    }
}

/// GET /api/maj - compare cette version a la derniere release publiee.
///
/// Cote serveur, et pas depuis la page: un appel a api.github.com depuis la
/// webview de l'application se heurte a la politique de securite du contenu et
/// aux regles d'origine croisee, et echouait donc silencieusement la ou il
/// marchait dans un navigateur. Le noeud, lui, n'a ni l'une ni les autres.
pub(crate) async fn api_maj() -> Json<serde_json::Value> {
    let installee = env!("CARGO_PKG_VERSION");
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return Json(
                serde_json::json!({ "installee": installee, "error": e.to_string() }),
            )
        }
    };
    // L'API de GitHub REFUSE une requete sans en-tete `User-Agent`, avec un 403
    // qui ne dit pas pourquoi.
    let rep = client
        .get("https://api.github.com/repos/infinition/LaRuche/releases/latest")
        .header("User-Agent", format!("LaRuche/{installee}"))
        .header("Accept", "application/vnd.github+json")
        .send()
        .await;
    match rep {
        Ok(r) if r.status().is_success() => match r.json::<serde_json::Value>().await {
            Ok(v) => Json(serde_json::json!({
                "installee": installee,
                "derniere": v["tag_name"].as_str().unwrap_or("").trim_start_matches('v'),
                "url": v["html_url"].as_str().unwrap_or(""),
                "publiee": v["published_at"].as_str().unwrap_or(""),
            })),
            Err(e) => Json(serde_json::json!({ "installee": installee, "error": e.to_string() })),
        },
        Ok(r) => Json(serde_json::json!({
            "installee": installee,
            "error": format!("GitHub a repondu {}", r.status())
        })),
        Err(e) => Json(serde_json::json!({ "installee": installee, "error": e.to_string() })),
    }
}

#[cfg(test)]
mod tests {
    use super::supprimee_du_disque;
    use serde_json::json;
    use std::collections::HashSet;

    fn disque(ids: &[&str]) -> HashSet<String> {
        ids.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn balaye_une_skill_retiree_du_disque_et_epargne_celles_forgees() {
        let venue_du_disque =
            json!({"items": [{"content": "type: skill", "source": "skill-file"}]});
        let forgee_par_lagent =
            json!({"items": [{"content": "type: skill", "source": "skill_create"}]});
        let present = disque(&["capacities.skills.openhue"]);

        // Its folder is gone and it came from disk: it is a ghost, sweep it.
        assert!(supprimee_du_disque(
            &venue_du_disque,
            "capacities.skills.airtable",
            &present
        ));
        // Still on disk: untouched.
        assert!(!supprimee_du_disque(
            &venue_du_disque,
            "capacities.skills.openhue",
            &present
        ));
        // Forged by the agent, lives only in SQL: never swept, whatever the disk says.
        assert!(!supprimee_du_disque(
            &forgee_par_lagent,
            "capacities.skills.airtable",
            &present
        ));
    }

    #[test]
    fn sans_scan_disque_ou_sans_provenance_on_ne_supprime_rien() {
        let venue_du_disque =
            json!({"items": [{"content": "type: skill", "source": "skill-file"}]});
        // No disk scan happened: sweeping on that basis would erase the whole catalog.
        assert!(!supprimee_du_disque(&venue_du_disque, "capacities.skills.x", &disque(&[])));

        // A backend that does not expose `source` must sweep nothing, not guess.
        let sans_source = json!({"items": [{"content": "type: skill"}]});
        assert!(!supprimee_du_disque(
            &sans_source,
            "capacities.skills.x",
            &disque(&["capacities.skills.y"])
        ));
        // Neither must an empty node reach the disk rule (the shell sweep owns it).
        let vide = json!({"items": []});
        assert!(!supprimee_du_disque(
            &vide,
            "capacities.skills.x",
            &disque(&["capacities.skills.y"])
        ));
    }
}
