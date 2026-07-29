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
//! Au demarrage:
//!   1. si un noeud repond deja, on s'y raccroche - lancer l'app ne doit pas doubler
//!      un service deja en marche, ni se battre pour le port;
//!   2. sinon on demarre `laruche-node` a cote de nous, et on l'arrete en partant.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;
use std::process::{Child, Command};
use std::time::{Duration, Instant};

/// Adresse servie par le noeud. `LARUCHE_URL` permet de viser un autre port ou une
/// autre machine (une ruche du reseau, par exemple).
fn url_noeud() -> String {
    std::env::var("LARUCHE_URL").unwrap_or_else(|_| "http://127.0.0.1:8419".to_string())
}

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
    let voisin = std::env::current_exe().ok()?.parent()?.join(nom);
    if voisin.exists() {
        return Some(voisin);
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
    let enfant = Command::new(&exe)
        // Sans cela le noeud ouvrirait le navigateur par-dessus notre fenetre - on
        // aurait l'interface en double.
        .env("LARUCHE_NO_BROWSER", "1")
        .current_dir(&dossier)
        .spawn()
        .ok()?;

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

fn main() {
    let url = url_noeud();

    // Si quelque chose ecoute deja, on ne lance rien: le service Windows, un `.bat`
    // ou un autre onglet restent maitres du port.
    let mut enfant = if noeud_repond(&url) {
        None
    } else {
        demarrer_noeud(&url)
    };

    let cible = url.parse().expect("LARUCHE_URL n'est pas une URL valide");

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
