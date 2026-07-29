//! Trouver les ruches du reseau local, sans en heberger une.
//!
//! `MielListener` ecoute les annonces mDNS `_ai-inference._tcp.local.` sans rien
//! annoncer lui-meme. Une coque depourvue de noeud peut donc dresser la liste des
//! ruches joignables et laisser choisir - c'est ce qui rend un client pur possible,
//! et c'est exactement le chemin qu'empruntera une application mobile.

use std::time::Duration;

/// Une ruche reperee sur le reseau.
pub struct Ruche {
    pub nom: String,
    pub url: String,
    /// Modele annonce, affiche pour distinguer deux ruches du meme nom.
    pub modele: Option<String>,
    /// Accepte-t-elle reellement une connexion ?
    ///
    /// Une ruche s'annonce avec son adresse LAN mais n'ecoute par defaut que sur
    /// `127.0.0.1`: elle est donc visible sans etre joignable, tant qu'elle n'a pas
    /// ete demarree avec `LARUCHE_BIND_LAN=1`. Sans cette verification, cliquer
    /// dessus ouvrait une fenetre vide sans la moindre explication.
    pub joignable: bool,
}

/// Une connexion TCP aboutit-elle sur cette URL ?
fn joignable(url: &str) -> bool {
    let sans_schema = url.split("://").nth(1).unwrap_or(url);
    let hote = match sans_schema.split('/').next() {
        Some(h) => h,
        None => return false,
    };
    match std::net::ToSocketAddrs::to_socket_addrs(&hote).ok().and_then(|mut a| a.next()) {
        Some(a) => std::net::TcpStream::connect_timeout(&a, Duration::from_millis(600)).is_ok(),
        None => false,
    }
}

/// Ecoute pendant `duree` et rend les ruches vues, triees par nom.
///
/// L'attente est fixe et non interruptible: mDNS n'a pas de notion de « liste
/// complete », on ne peut qu'accorder un delai. Trois secondes suffisent sur un
/// reseau domestique et restent supportables au demarrage.
pub fn chercher(duree: Duration) -> Vec<Ruche> {
    // MielListener essaime des taches: il lui faut un runtime. On en cree un le temps
    // de la recherche, rendu avant que Tauri ne demarre le sien. Multi-thread et non
    // current-thread, sinon les taches de l'ecouteur n'avanceraient pas pendant
    // qu'on attend.
    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(_) => return Vec::new(),
    };
    rt.block_on(chercher_async(duree))
}

async fn chercher_async(duree: Duration) -> Vec<Ruche> {
    let mut ecouteur = match miel_protocol::MielListener::new() {
        Ok(e) => e,
        // Pas de mDNS (pare-feu, interface absente): on rend une liste vide plutot
        // que d'empecher l'application de demarrer.
        Err(_) => return Vec::new(),
    };
    let noeuds = match ecouteur.start() {
        Ok(n) => n,
        Err(_) => return Vec::new(),
    };

    tokio::time::sleep(duree).await;
    let vues = noeuds.read().await;

    let mut ruches: Vec<Ruche> = vues
        .values()
        .filter_map(|n| {
            let m = &n.manifest;
            // Le port d'API est celui qui sert la SPA - malgre son nom, `dashboard_port`
            // designe un autre service, souvent eteint. Preferer ce dernier menait la
            // fenetre sur un port ou personne n'ecoutait.
            let port = m.port.or(m.dashboard_port)?;
            let hote = miel_protocol::format_host_for_url(&m.host);
            let url = format!("http://{hote}:{port}");
            Some(Ruche {
                nom: m.node_name.clone().unwrap_or_else(|| hote.clone()),
                joignable: joignable(&url),
                url,
                modele: m.model.clone(),
            })
        })
        .collect();
    // Les joignables d'abord: ce sont les seules sur lesquelles on peut cliquer.
    ruches.sort_by(|a, b| b.joignable.cmp(&a.joignable).then(a.nom.cmp(&b.nom)));
    ruches
}

fn echapper(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Page de choix, ecrite dans un fichier temporaire et chargee dans la fenetre.
///
/// De simples liens: cliquer navigue la webview vers la ruche. Aucun IPC Tauri
/// n'est necessaire, ce qui evite d'introduire une dependance entre la coque et la
/// page - la meme raison qui fait qu'on ne touche pas au front habituel.
pub fn page_choix(ruches: &[Ruche], noeud_absent: bool) -> String {
    let cartes = if ruches.is_empty() {
        let raison = if noeud_absent {
            "Cette application ne contient pas de noeud: elle se connecte a une ruche existante."
        } else {
            "Aucun noeud n'a pu etre demarre localement."
        };
        format!(
            "<p class=\"vide\">Aucune ruche trouvee sur le reseau local.</p>\
             <p class=\"aide\">{}</p>\
             <p class=\"aide\">Si la ruche est ailleurs, indique son adresse :<br>\
             <code>LARUCHE_URL=http://192.168.1.20:8419</code></p>",
            echapper(raison)
        )
    } else {
        ruches
            .iter()
            .map(|r| {
                let meta = format!(
                    "{}{}",
                    echapper(&r.url),
                    r.modele
                        .as_deref()
                        .map(|m| format!(" · {}", echapper(m)))
                        .unwrap_or_default()
                );
                let nom = echapper(&r.nom);
                if r.joignable {
                    format!(
                        "<a class=\"ruche\" href=\"{url}\">\
                           <span class=\"nom\">{nom}</span>\
                           <span class=\"meta\">{meta}</span>\
                         </a>",
                        url = echapper(&r.url)
                    )
                } else {
                    // Visible mais sourde: on le dit, et on donne le remede exact.
                    format!(
                        "<div class=\"ruche muette\">\
                           <span class=\"nom\">{nom} <span class=\"etiq\">injoignable</span></span>\
                           <span class=\"meta\">{meta}</span>\
                           <span class=\"pourquoi\">Elle s'annonce mais n'ecoute que sur elle-meme. \
                             Sur cette machine, demarrer la ruche avec \
                             <code>LARUCHE_BIND_LAN=1</code>.</span>\
                         </div>"
                    )
                }
            })
            .collect::<Vec<_>>()
            .join("")
    };

    format!(
        r#"<!doctype html>
<html lang="fr"><head><meta charset="utf-8"><title>LaRuche</title><style>
  :root {{ color-scheme: dark; }}
  body {{ margin:0; background:#0f0f10; color:#e7e7ea; display:flex; min-height:100vh;
         align-items:center; justify-content:center;
         font:14px/1.5 system-ui,-apple-system,Segoe UI,sans-serif; }}
  main {{ width:min(560px,92vw); padding:28px 0; }}
  h1 {{ font-size:17px; font-weight:600; color:#f59e0b; margin:0 0 4px; }}
  .sous {{ color:#8a8a92; font-size:12px; margin:0 0 20px; }}
  .ruche {{ display:flex; flex-direction:column; gap:2px; padding:12px 14px; margin-bottom:8px;
           border:1px solid #2a2a30; border-radius:10px; text-decoration:none; color:inherit;
           transition:border-color .15s, background .15s; }}
  .ruche:hover {{ border-color:#f59e0b; background:rgba(245,158,11,.06); }}
  .nom {{ font-weight:600; }}
  .meta {{ font-size:11px; color:#8a8a92; font-family:ui-monospace,Consolas,monospace; }}
  .vide {{ color:#e7e7ea; }}
  .aide {{ color:#8a8a92; font-size:12px; }}
  .muette {{ opacity:.72; cursor:default; }}
  .muette:hover {{ border-color:#2a2a30; background:none; }}
  .etiq {{ font-size:10px; font-weight:400; color:#0f0f10; background:#8a8a92;
          padding:1px 6px; border-radius:8px; vertical-align:middle; margin-left:6px; }}
  .pourquoi {{ font-size:11px; color:#8a8a92; margin-top:6px; }}
  code {{ background:#1a1a1f; padding:2px 6px; border-radius:4px; font-size:11px; }}
</style></head><body><main>
  <h1>Choisis une ruche</h1>
  <p class="sous">Reperees sur le reseau local.</p>
  {cartes}
</main></body></html>"#
    )
}
