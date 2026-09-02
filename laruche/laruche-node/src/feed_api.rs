//! Feed endpoints (feed poll, ask LaRuche from the feed, profile get/save, system prompt defaults) - split out of main.rs.

use crate::*;
use axum::extract::State;
use axum::response::Json;
use std::sync::Arc;

/// Actor of a memory mutation based on its `src` (source/reason). UI -> User, otherwise LaRuche.
pub(crate) fn feed_actor(src: &str) -> &'static str {
    let s = src.trim().to_lowercase();
    // No hardcoded first name here. One used to be, a special case for a single
    // person shipped in a public binary, and it never did anything the checks around
    // it did not already cover.
    if s.starts_with("ui") || s == "user" || s == "admin" {
        "User"
    } else {
        "LaRuche"
    }
}

/// Cleans an agent response for the Feed: removes protocol blocks (`<plan>`, `<tool_call>`,
/// `<think>`) - complete or truncated - and normalizes whitespace. Otherwise the Feed shows JSON/XML
/// unreadable to a human.
pub(crate) fn nettoyer_reponse_feed(s: &str) -> String {
    let mut out = s.to_string();
    for (open, close) in [
        ("<plan>", "</plan>"),
        ("<tool_call>", "</tool_call>"),
        ("<think>", "</think>"),
    ] {
        while let Some(i) = out.find(open) {
            match out[i..].find(close) {
                Some(j_rel) => {
                    let j = i + j_rel + close.len();
                    out.replace_range(i..j, " ");
                }
                None => out.truncate(i), // opening tag without closing -> cut the tail
            }
        }
    }
    // Les retours a la ligne SURVIVENT.
    //
    // Ils etaient ecrases par un `split_whitespace().join(" ")`, et la reponse de
    // LaRuche arrivait dans le flux en un seul pave. Le flux rend pourtant le
    // markdown: le gras passait, parce qu'il est en ligne, mais ni les listes ni
    // les titres, qui ont besoin d'un debut de ligne. On lisait donc "voies
    // s'ouvrent : Voie A - Monter un point d'acces - Rediriger le trafic - Le Uni
    // fait du TLS" d'un trait, la ou trois puces etaient ecrites.
    //
    // L'apercu, lui, reste sur une ligne: `preview_text` l'aplatit de son cote, et
    // c'est le champ `full` qui porte le texte deplie.
    //
    // Le nettoyage des balises laisse derriere lui des trous: on ramene les
    // espaces horizontaux en trop et on borne les lignes vides a une seule, sans
    // jamais toucher a la structure.
    let mut propre = String::with_capacity(out.len());
    let mut vides = 0usize;
    for ligne in out.lines() {
        let l = ligne.trim_end();
        if l.trim().is_empty() {
            vides += 1;
            if vides > 1 {
                continue;
            }
        } else {
            vides = 0;
        }
        propre.push_str(l);
        propre.push('\n');
    }
    propre.trim().to_string()
}

/// POST /api/feed/ask {text} - talks to LaRuche FROM the Feed. Runs on a dedicated "feed"
/// session (rolling context ~10 exchanges, isolated from the main chat), in the background; the response
/// appears in the Feed via activity_log on the next poll. Full agent capabilities (crons...).
pub(crate) async fn api_feed_ask(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let text = match body["text"].as_str().map(str::trim).filter(|s| !s.is_empty()) {
        Some(t) => t.to_string(),
        None => return Json(serde_json::json!({ "status": "error", "error": "empty text" })),
    };
    let st = state.clone();
    tokio::spawn(async move {
        // Dedicated feed session (deterministic id) -> rolling context, separate from the main chat.
        let feed_id = Uuid::from_u128(0xFEED_0000_0000_0000_0000_0000_0000_0001);
        let sessions_dir = std::path::Path::new("sessions");
        // The Feed is its own usage, like a channel: it can run a different model from the
        // web chat without either of them noticing.
        let mut cfg = st.essaim_config.read().await.clone();
        apply_channel_model(&st, "feed", &mut cfg).await;
        let model = cfg.model.clone();
        let mut session = {
            let mut sessions = st.essaim_sessions.write().await;
            sessions
                .remove(&feed_id)
                .unwrap_or_else(|| Session::new_with_id(feed_id, &model, sessions_dir))
        };
        let (tx, _rx) = tokio::sync::broadcast::channel::<laruche_essaim::ChatEvent>(256);
        let _garde = ouvrir_travail(&st, "feed", "ask", &cfg, Some("feed".to_string()));
        let result = boucle_react_memoire(
            &text,
            &mut session,
            &st.essaim_registry,
            &cfg,
            &tx,
            st.memoire.clone(),
        )
        .await;
        // Short rolling context (~10 exchanges = 20 messages): truncate the oldest.
        if session.messages.len() > 20 {
            let drop_n = session.messages.len() - 20;
            session.messages.drain(0..drop_n);
        }
        {
            let now = chrono::Utc::now().to_rfc3339();
            let mut activity = st.activity_log.write().await;
            if activity.len() >= ACTIVITY_LOG_LIMIT {
                activity.pop_front();
            }
            activity.push_back(ActivityLogEntry {
                timestamp: now,
                level: if result.is_ok() { "info" } else { "error" }.into(),
                tag: "agent".into(),
                message: format!("Feed: {}", preview_text(&text, 60)),
                full_prompt: Some(text.clone()),
                full_response: result.as_ref().ok().map(|r| texte_complet(r, 4000)),
                model_used: Some(cfg.model.clone()),
                tokens_generated: None,
                latency_ms: None,
                user_id: None,
            });
        }
        let _ = session.sauvegarder();
        st.essaim_sessions.write().await.insert(feed_id, session);
    });
    Json(serde_json::json!({ "status": "ok" }))
}

/// GET /api/profile - user profile (node `system.user`, injected into LaRuche's context).
pub(crate) async fn api_profile_get(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let fiche = state
        .memoire
        .read_node("system.user")
        .await
        .ok()
        .and_then(|n| {
            n.get("items").and_then(|i| i.as_array()).and_then(|a| {
                a.iter().rev().find_map(|it| {
                    it.get("content").and_then(|c| c.as_str()).map(str::to_string)
                })
            })
        })
        .unwrap_or_default();
    Json(serde_json::json!({ "fiche": fiche }))
}

/// POST /api/profile {fiche} - replaces the user profile (single item). Source `ui-profile`
/// (User actor in the Feed). Only the user edits; the agent is forbidden (memory_write).
pub(crate) async fn api_profile_save(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let fiche = body["fiche"].as_str().unwrap_or("").trim().to_string();
    if let Ok(node) = state.memoire.read_node("system.user").await {
        if let Some(items) = node.get("items").and_then(|i| i.as_array()) {
            for it in items {
                if let Some(id) = it.get("id").and_then(|x| x.as_str()) {
                    let _ = state.memoire.delete_item(id, Some("ui-profile")).await;
                }
            }
        }
    }
    if !fiche.is_empty() {
        let _ = state
            .memoire
            .write(
                laruche_memoire::MemoryItem::new("system.user", fiche).with_source("ui-profile"),
            )
            .await;
    }
    Json(serde_json::json!({ "status": "ok" }))
}

/// GET /api/feed?limit=N - UNIFIED activity stream for the global Feed pane: memory mutations
/// (with User/LaRuche actor + clickable ref) + agent inferences (activity_log), sorted recent->old.
pub(crate) async fn api_feed(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(q): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let limit = q
        .get("limit")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(200);
    let mut events: Vec<serde_json::Value> = Vec::new();

    // 1) Memory mutations (who added/deleted/modified what).
    if let Ok(muts) = state.memoire.mutations_activite(Some(400)).await {
        if let Some(arr) = muts.get("mutations").and_then(|m| m.as_array()) {
            for m in arr {
                let op = m.get("op").and_then(|v| v.as_str()).unwrap_or("");
                let node = m.get("node_id").and_then(|v| v.as_str()).unwrap_or("");
                let ts = m.get("ts").and_then(|v| v.as_i64()).unwrap_or(0);
                let src = m.get("src").and_then(|v| v.as_str()).unwrap_or("");
                // System noise (non-activity): tool indexing + node (re)seed at boot
                // + disk<->SQL skill sync (delete+write per skill on each startup/watch ->
                // flooded the Feed with dozens of capacities.skills.* lines).
                if matches!(
                    src,
                    "tool-registry" | "seed" | "skill-file" | "skill-file-sync" | "skill-file-watch"
                ) {
                    continue;
                }
                if (op == "create_node" || op == "update_node")
                    && (node.starts_with("system") || node.starts_with("capacities"))
                {
                    continue;
                }
                let action = match op {
                    "write" if src == "consolidation" => "consolidated",
                    "write" => "added an item to",
                    "propose" => "proposed an item in",
                    "update" => "modified an item of",
                    "delete" => "deleted an item from",
                    "move" => "moved an item to",
                    "create_node" => "created the node",
                    "update_node" => "updated the node",
                    "rename_subtree" => "moved the subtree",
                    _ => "modified",
                };
                events.push(serde_json::json!({
                    "ts": ts, "actor": feed_actor(src), "kind": "memory",
                    "action": action, "object": node, "ref": node
                }));
            }
        }
    }

    // 2) Agent exchanges: the user's message (full_prompt) THEN LaRuche's response
    //    (full_response cleaned of protocol tags). Lets you see your own messages in the
    //    Feed, attributed to User, and a readable response (no raw <plan>/<tool_call>).
    {
        let logs = state.activity_log.read().await;
        for e in logs.iter() {
            // MILLISECONDS (rfc3339 has sub-second). Reverse-chronological Feed (recent on
            // TOP) -> within a turn, the RESPONSE (more recent) is placed 1 ms ABOVE the
            // question. You read: response, then its question below; next turn lower down.
            let ms = chrono::DateTime::parse_from_rfc3339(&e.timestamp)
                .map(|d| d.timestamp_millis())
                .unwrap_or(0);
            // An exchange is an exchange whatever carried it. The tag used to have to be
            // exactly "agent", so a whole Telegram or Discord conversation lost its user
            // half and kept only a bare reply. The kind follows the tag, so the Feed can
            // filter a channel apart from the web chat.
            let est_echange = matches!(
                e.tag.as_str(),
                "agent" | "telegram" | "discord" | "slack" | "whatsapp" | "voice"
            );
            let genre = if e.tag == "agent" { "agent" } else { e.tag.as_str() };
            // a) User message (only for conversational exchanges).
            if est_echange {
                if let Some(prompt) = e.full_prompt.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
                    let clean = prompt.split("\n\n[SYSTEM]").next().unwrap_or(prompt).trim();
                    if !clean.is_empty() {
                        events.push(serde_json::json!({
                            "ts": ms, "actor": "User", "kind": genre,
                            "action": "asked", "object": preview_text(clean, 160),
                            "full": clean, "ref": serde_json::Value::Null, "tag": e.tag
                        }));
                    }
                }
            }
            // b) LaRuche's response, cleaned (otherwise unreadable JSON/XML). Empty after cleaning
            //    (pure tool turn) -> we don't add a hollow "replied" event.
            let brut = e.full_response.as_deref().filter(|s| !s.is_empty()).unwrap_or(&e.message);
            let resp = nettoyer_reponse_feed(brut);
            if !resp.is_empty() {
                events.push(serde_json::json!({
                    "ts": ms + 1, "actor": "LaRuche", "kind": if est_echange { genre } else { "agent" },
                    "action": "replied", "object": preview_text(&resp, 160),
                    "full": resp, "ref": serde_json::Value::Null, "tag": e.tag
                }));
            }
        }
    }

    // 3) Executed crons (last run).
    {
        let cron = state.essaim_cron.read().await;
        for t in cron.list() {
            if let Some(lr) = t.last_run {
                events.push(serde_json::json!({
                    "ts": lr.timestamp(), "actor": "LaRuche", "kind": "cron",
                    "action": "ran the cron", "object": t.name, "ref": serde_json::Value::Null
                }));
            }
        }
    }
    // 4) Missions (last iteration).
    {
        let missions = state.missions.read().await;
        for m in missions.list() {
            if let Some(lr) = m.last_run.as_deref() {
                let ts = chrono::DateTime::parse_from_rfc3339(lr)
                    .map(|d| d.timestamp())
                    .unwrap_or(0);
                events.push(serde_json::json!({
                    "ts": ts, "actor": "LaRuche", "kind": "mission",
                    "action": "advanced the mission", "object": m.slug, "ref": serde_json::Value::Null
                }));
            }
        }
    }
    // 5) Triggered watchers (last detection).
    {
        let watchers = state.watchers.read().await;
        for w in watchers.list() {
            if let Some(lr) = w.last_run {
                events.push(serde_json::json!({
                    "ts": lr.timestamp(), "actor": "LaRuche", "kind": "watcher",
                    "action": "triggered the watcher", "object": w.name, "ref": serde_json::Value::Null
                }));
            }
        }
    }

    // 5b) Kanban tasks that have run. They were the one scheduled family missing from the
    //     Feed entirely: a task could be picked up, run and completed without a trace.
    {
        let board = state.kanban_board.read().await;
        for t in board.list() {
            if let Some(fin) = t.completed_at {
                let ts = fin.timestamp();
                events.push(serde_json::json!({
                    "ts": ts, "actor": "LaRuche", "kind": "kanban",
                    "action": "finished the kanban task", "object": t.title,
                    "ref": serde_json::Value::Null
                }));
            }
        }
    }

    // 6) Direct messages (DM) from the mesh -> first building block of the global feed. Actor = the PEER (purple
    //    ruche) for received ones; Me for sent ones.
    for m in mesh_api::read_inbox() {
        let (actor, action, akind) = if m.dir == "out" {
            ("User".to_string(), format!("wrote to {}", m.peer_name), "user")
        } else {
            (m.peer_name.clone(), "wrote to you".to_string(), "peer")
        };
        events.push(serde_json::json!({
            "ts": m.ts, "actor": actor, "kind": "dm", "action": action,
            "object": preview_text(&m.text, 160), "full": m.text,
            "ref": serde_json::Value::Null, "actor_kind": akind
        }));
    }

    // Unify the unit: the mutations/cron/mission/watcher sections are in SECONDS, the agent
    // section in MILLISECONDS. Convert everything to ms (a ts < 1e12 = seconds -> x1000) for a
    // consistent sort (otherwise the agent events, 1000x larger, would crush everything).
    for e in events.iter_mut() {
        let t = e["ts"].as_i64().unwrap_or(0);
        if t > 0 && t < 1_000_000_000_000 {
            e["ts"] = serde_json::Value::from(t * 1000);
        }
    }
    // Normalize all `ts` to MILLISECONDS (some sources are in seconds: memory,
    // missions, watchers, crons). Without this, agent events (already in ms) ALWAYS floated
    // above the others, regardless of real time. Heuristic: ts < 1e12 -> seconds.
    // PERSISTENT system journal: creations (cron/watcher/mission/kanban) + curateur runs.
    // Survives restart (before: only executions via last_run appeared).
    for ev in laruche_essaim::feed_journal::recent(limit) {
        events.push(serde_json::json!({
            "ts": ev.ts, "actor": ev.actor, "kind": ev.kind,
            "action": ev.action, "object": ev.object, "ref": serde_json::Value::Null
        }));
    }

    for e in events.iter_mut() {
        if let Some(ts) = e.get("ts").and_then(|v| v.as_i64()) {
            if ts != 0 && ts < 1_000_000_000_000 {
                e["ts"] = serde_json::Value::from(ts * 1000);
            }
        }
    }
    events.sort_by(|a, b| {
        b["ts"].as_i64().unwrap_or(0).cmp(&a["ts"].as_i64().unwrap_or(0))
    });
    events.truncate(limit);
    Json(serde_json::json!({ "events": events }))
}

/// GET /api/vision - quels modeles sont ecartes, et pour combien de temps.
pub(crate) async fn api_vision() -> Json<serde_json::Value> {
    let ecartes: Vec<serde_json::Value> = laruche_essaim::vision::ecartes()
        .into_iter()
        .map(|(modele, restant)| serde_json::json!({ "model": modele, "restant_s": restant }))
        .collect();
    Json(serde_json::json!({
        "ecartes": ecartes,
        "repit_s": laruche_essaim::vision::repit_secs(),
    }))
}

/// POST /api/vision/reset {model?} - rend sa vue a un modele, tout de suite.
///
/// Sans corps, c'est le modele actif. Attendre dix minutes ou redemarrer le
/// noeud etaient les deux seules issues, et aucune des deux ne se devine.
pub(crate) async fn api_vision_reset(
    State(state): State<Arc<AppState>>,
    axum::extract::Json(body): axum::extract::Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let modele = match body["model"].as_str().map(str::trim).filter(|s| !s.is_empty()) {
        Some(m) => m.to_string(),
        None => state.essaim_config.read().await.model.clone(),
    };
    let rendu = laruche_essaim::vision::reessayer(&modele);
    Json(serde_json::json!({ "status": "ok", "model": modele, "reactive": rendu }))
}

/// GET /api/system/prompt-defaults - default (hardcoded) texts of the editable sections,
/// to pre-fill the editor: the user sees and edits the full prompt (empty in DB =
/// this default is used). The `node_*` override REPLACES the corresponding section.
pub(crate) async fn api_system_prompt_defaults() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "identity": laruche_essaim::prompt::section_identite_stable(),
        "behavior": laruche_essaim::prompt::section_comportement(),
        "prompt_curateur": laruche_essaim::butinage_pont::prompt_curateur_defaut(),
        "prompt_extraction": laruche_essaim::butinage_pont::prompt_extraction_defaut(),
        "prompt_planning": laruche_essaim::prompt::section_planification(),
        "prompt_reine": laruche_essaim::reine_live::prompt_reine_defaut(),
        // Le socle de la table ronde. Sans lui, l'editeur de `system.constitution`
        // tombait sur la valeur par defaut de la chaine, l'identite, et proposait
        // donc d'enregistrer le mauvais texte a la place des regles communes.
        "constitution": laruche_essaim::deliberation::CONSTITUTION,
    }))
}

#[cfg(test)]
mod tests_nettoyage_feed {
    use super::nettoyer_reponse_feed;

    /// La structure markdown doit arriver intacte dans le flux, qui la rend.
    ///
    /// Elle etait aplatie par un split_whitespace: le gras survivait, etant en
    /// ligne, mais les puces se retrouvaient collees les unes aux autres sur une
    /// seule ligne, donc rendues comme du texte courant.
    #[test]
    fn les_retours_a_la_ligne_survivent() {
        let reponse = "Voici les voies possibles :\n\n\
                       **Voie A : interception reseau**\n\
                       - Monter un point d'acces controle\n\
                       - Rediriger le trafic vers un proxy\n\n\
                       ### Voie B\n\
                       Capturer la mise a jour.";
        let out = nettoyer_reponse_feed(reponse);
        assert!(out.contains("\n- Monter un point"), "les puces gardent leur ligne");
        assert!(out.contains("\n### Voie B"), "les titres gardent la leur");
        assert_eq!(out.lines().count(), 8);
    }

    /// Le nettoyage des balises reste entier, et ne laisse pas de trou beant.
    #[test]
    fn les_balises_partent_sans_laisser_de_vide() {
        let out = nettoyer_reponse_feed(
            "Debut.\n<think>raisonnement interne</think>\n\n\n\nFin.",
        );
        assert!(!out.contains("think"), "la balise et son contenu partent");
        assert!(out.starts_with("Debut."));
        assert!(out.ends_with("Fin."));
        assert!(!out.contains("\n\n\n"), "pas plus d'une ligne vide d'affilee");
    }

    /// Une balise ouverte sans fermeture coupe la queue: comportement conserve.
    #[test]
    fn une_balise_non_fermee_coupe_la_suite() {
        let out = nettoyer_reponse_feed("Reponse utile.\n<tool_call>{\"name\":");
        assert_eq!(out, "Reponse utile.");
    }
}
