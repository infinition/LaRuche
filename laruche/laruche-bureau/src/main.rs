//! LaRuche en application de bureau.
//!
//! La coque n'embarque aucune interface: elle ouvre une fenetre sur le noeud local,
//! qui sert deja la SPA. C'est le point important de cette approche - il n'y a pas de
//! seconde version du front a maintenir, et rien a reecrire cote page. Ce que tu vois
//! dans la fenetre est, octet pour octet, ce que sert `http://127.0.0.1:8419`.
//!
//! Aucun IPC Tauri n'est utilise: la page ne connait pas `window.__TAURI__` et n'en a
//! pas besoin. Tout ce qu'elle fait (fichiers, mDNS, memoire) passe deja par le noeud.
//!
//! Au demarrage, dans l'ordre:
//!   1. `LARUCHE_URL` est prioritaire: on vise cette ruche, ou qu'elle soit;
//!   2. sinon, si un noeud repond deja en local, on s'y raccroche - lancer l'app ne
//!      doit pas doubler un service en marche, ni se battre pour le port;
//!   3. sinon on demarre `laruche-node` s'il voyage avec nous, et on l'arrete en
//!      partant - mais seulement si c'est nous qui l'avons lance;
//!   4. sinon on cherche les ruches du reseau en mDNS et on laisse choisir.
//!
//! L'etape 4 est ce qui rend possible une coque SANS noeud: 1,8 Mo au lieu de 9,7,
//! qui se connecte a la ruche de la maison. C'est aussi, exactement, le chemin
//! qu'empruntera une application mobile.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod decouverte;

use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;
use std::process::{Child, Command};
use std::time::{Duration, Instant};

/// Adresse imposee par l'utilisateur, s'il en a indique une.
fn url_imposee() -> Option<String> {
    std::env::var("LARUCHE_URL").ok().filter(|s| !s.is_empty())
}

/// Adresse locale par defaut.
const URL_LOCALE: &str = "http://127.0.0.1:8419";

/// Extrait `host:port` de l'URL pour la sonde TCP.
fn adresse(url: &str) -> Option<SocketAddr> {
    let sans_schema = url.split("://").nth(1).unwrap_or(url);
    let hote = sans_schema.split('/').next()?;
    // Une resolution DNS plutot qu'un parse: `localhost:8419` doit marcher aussi.
    std::net::ToSocketAddrs::to_socket_addrs(&hote).ok()?.next()
}

/// Le noeud ecoute-t-il ? Une connexion TCP suffit: axum se lie au port puis sert,
/// donc un connect qui aboutit veut dire que la page va repondre.
fn noeud_repond(url: &str) -> bool {
    match adresse(url) {
        Some(a) => TcpStream::connect_timeout(&a, Duration::from_millis(400)).is_ok(),
        None => false,
    }
}

/// Cherche l'executable du noeud. A cote de nous une fois installe; dans
/// `target/<profil>/` pendant le developpement.
fn chemin_noeud() -> Option<PathBuf> {
    let nom = if cfg!(windows) {
        "laruche-node.exe"
    } else {
        "laruche-node"
    };
    let a_cote = std::env::current_exe().ok()?.parent()?.to_path_buf();
    // `bin/` d'abord: c'est la ou l'installeur depose le noeud (bundle.resources).
    // Le dossier de l'exe ensuite, pour une extraction manuelle de l'archive.
    for candidat in [a_cote.join("bin").join(nom), a_cote.join(nom)] {
        if candidat.exists() {
            return Some(candidat);
        }
    }
    // En dev, `cargo run -p laruche-bureau` produit un debug alors que le noeud est
    // souvent compile en release: on regarde les deux.
    let racine = PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent()?.join("target");
    for profil in ["release", "debug"] {
        let p = racine.join(profil).join(nom);
        if p.exists() {
            return Some(p);
        }
    }
    None
}

/// Demarre le noeud et attend qu'il ecoute. `None` si on n'a rien lance (il tournait
/// deja, ou l'executable est introuvable).
fn demarrer_noeud(url: &str) -> Option<Child> {
    let exe = chemin_noeud()?;
    let dossier = exe.parent()?.to_path_buf();
    let mut commande = Command::new(&exe);
    commande
        // Sans cela le noeud ouvrirait le navigateur par-dessus notre fenetre - on
        // aurait l'interface en double.
        .env("LARUCHE_NO_BROWSER", "1")
        .current_dir(&dossier);
    // Le port que nous allons afficher doit etre celui sur lequel il ecoute. Sans
    // cela, un LARUCHE_URL personnalise lancait un noeud sur son port par defaut et
    // la fenetre attendait sur un port ou personne ne repondrait jamais.
    if let Some(p) = adresse(url).map(|a| a.port()) {
        commande.env("LARUCHE_PORT", p.to_string());
    }
    let enfant = commande.spawn().ok()?;

    // Le noeud ouvre sa base, amorce la memoire et sonde le reseau avant d'ecouter:
    // quelques secondes sur un demarrage a froid.
    let limite = Instant::now() + Duration::from_secs(45);
    while Instant::now() < limite {
        if noeud_repond(url) {
            break;
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    Some(enfant)
}

/// Ecrit la page de choix a cote des donnees de l'application et rend son URL.
fn page_choix_url(html: &str) -> tauri::Url {
    let chemin = std::env::temp_dir().join("laruche-choix.html");
    let _ = std::fs::write(&chemin, html);
    tauri::Url::from_file_path(&chemin).expect("chemin temporaire absolu")
}

/// Ou pointer la fenetre, et faut-il arreter un noeud en partant.
fn resoudre() -> (tauri::Url, Option<Child>) {
    // 1. Choix explicite: on n'essaie rien d'autre, meme si la ruche ne repond pas
    //    encore - c'est peut-etre une machine en train de demarrer.
    if let Some(url) = url_imposee() {
        let enfant = if noeud_repond(&url) {
            None
        } else {
            demarrer_noeud(&url)
        };
        return (url.parse().expect("LARUCHE_URL n'est pas une URL valide"), enfant);
    }

    // 2. Un noeud local repond deja.
    if noeud_repond(URL_LOCALE) {
        return (URL_LOCALE.parse().expect("URL locale valide"), None);
    }

    // 3. Un noeud voyage avec nous: on le lance.
    if let Some(enfant) = demarrer_noeud(URL_LOCALE) {
        if noeud_repond(URL_LOCALE) {
            return (URL_LOCALE.parse().expect("URL locale valide"), Some(enfant));
        }
        // Lance mais muet: on ne le laisse pas trainer avant de passer a la suite.
        let mut enfant = enfant;
        let _ = enfant.kill();
    }

    // 4. Client pur: on demande au reseau qui est la.
    let ruches = decouverte::chercher(Duration::from_secs(3));
    // Une seule ruche et aucune ambiguite: on y va directement plutot que d'imposer
    // un ecran de choix a un seul bouton.
    if ruches.len() == 1 {
        if let Ok(u) = ruches[0].url.parse() {
            return (u, None);
        }
    }
    let html = decouverte::page_choix(&ruches, chemin_noeud().is_none());
    (page_choix_url(&html), None)
}

fn main() {
    // Diagnostic: « qu'est-ce que ce client voit sur le reseau ? ». Repond a la
    // question sans ouvrir de fenetre ni demarrer quoi que ce soit - utile quand un
    // pare-feu mange le mDNS et qu'on ne sait pas d'ou vient le silence.
    if std::env::var("LARUCHE_DECOUVRIR").is_ok() {
        let ruches = decouverte::chercher(Duration::from_secs(3));
        println!("ruches vues sur le reseau local: {}", ruches.len());
        for r in &ruches {
            println!(
                "  {}  {}  {}{}",
                r.nom,
                r.url,
                if r.joignable {
                    "joignable"
                } else {
                    "INJOIGNABLE (demarrer la ruche avec LARUCHE_BIND_LAN=1)"
                },
                r.modele.as_deref().map(|m| format!("  [{m}]")).unwrap_or_default()
            );
        }
        return;
    }

    let (cible, mut enfant) = resoudre();

    tauri::Builder::default()
        .setup(move |app| {
            tauri::WebviewWindowBuilder::new(app, "principale", tauri::WebviewUrl::External(cible))
                .title("LaRuche")
                .inner_size(1400.0, 900.0)
                .min_inner_size(900.0, 600.0)
                // Meme fond que la page: sans cela la fenetre clignote en blanc le
                // temps que la SPA peigne.
                .background_color(tauri::window::Color(0x0f, 0x0f, 0x10, 0xff))
                .build()?;
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("construction de l'application")
        .run(move |_app, evenement| {
            // On n'arrete que ce qu'on a demarre. Un noeud qui tournait avant nous
            // continue apres nous - fermer la fenetre ne doit pas couper un service
            // que quelqu'un d'autre utilise.
            if let tauri::RunEvent::Exit = evenement {
                if let Some(c) = enfant.as_mut() {
                    let _ = c.kill();
                }
            }
        });
}
