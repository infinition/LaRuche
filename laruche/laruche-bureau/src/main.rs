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

/// Le noeud sert-il vraiment ?
///
/// Une connexion TCP ne suffit pas: le port accepte des le `bind`, avant que le
/// serveur ne reponde. La fenetre s'ouvrait alors sur une SPA qui interrogeait des
/// routes pas encore pretes, peignait des panneaux vides et ne reessayait jamais -
/// d'ou le F5 necessaire sur certaines pages. On exige donc une vraie reponse HTTP.
fn noeud_repond(url: &str) -> bool {
    use std::io::{Read, Write};
    let Some(adr) = adresse(url) else { return false };
    let Ok(mut flux) = TcpStream::connect_timeout(&adr, Duration::from_millis(500)) else {
        return false;
    };
    let _ = flux.set_read_timeout(Some(Duration::from_millis(1500)));
    let _ = flux.set_write_timeout(Some(Duration::from_millis(500)));
    // /manifest.json plutot que / : quelques centaines d'octets, aucune
    // authentification, et servi par le meme routeur que le reste.
    let requete = format!(
        "GET /manifest.json HTTP/1.1\r\nHost: {adr}\r\nConnection: close\r\nUser-Agent: laruche-bureau\r\n\r\n"
    );
    if flux.write_all(requete.as_bytes()).is_err() {
        return false;
    }
    let mut tete = [0u8; 16];
    let mut lus = 0;
    // Boucle: la premiere lecture peut rendre moins que la ligne de statut.
    while lus < tete.len() {
        match flux.read(&mut tete[lus..]) {
            Ok(0) => break,
            Ok(n) => lus += n,
            Err(_) => break,
        }
    }
    String::from_utf8_lossy(&tete[..lus]).starts_with("HTTP/1.1 200")
}

/// Cherche l'executable du noeud. A cote de nous une fois installe; dans
/// `target/<profil>/` pendant le developpement.
///
/// `LARUCHE_SANS_NOEUD=1` rend None sans rien chercher: c'est le mode client pur,
/// qui va droit a la decouverte reseau. Une variable explicite plutot qu'une ruse
/// de repertoire, car le chemin `target/<profil>/` est COMPILE dans le binaire -
/// lancer la coque depuis un dossier vide ne l'empeche donc pas de retrouver le
/// noeud du depot, et le « mode client » demarrait un serveur local malgre tout.
fn chemin_noeud() -> Option<PathBuf> {
    if std::env::var("LARUCHE_SANS_NOEUD").is_ok_and(|v| v != "0" && !v.is_empty()) {
        return None;
    }
    let nom = if cfg!(windows) {
        "laruche-node.exe"
    } else {
        "laruche-node"
    };
    let a_cote = std::env::current_exe().ok()?.parent()?.to_path_buf();
    // `bin/` (ou l'installeur depose le noeud) et le dossier de l'exe (extraction
    // manuelle d'une archive). On prend le PLUS RECENT des deux, et non le premier
    // trouve: en developpement, `bin/` garde la copie du dernier empaquetage, qui
    // gagnait en silence sur le binaire fraichement compile a cote. On testait alors
    // une version d'il y a plusieurs heures sans le savoir - une route ajoutee dans la
    // minute repondait 404 sans aucune trace pour l'expliquer.
    let mut candidats: Vec<(std::time::SystemTime, PathBuf)> = [a_cote.join("bin").join(nom), a_cote.join(nom)]
        .into_iter()
        .filter_map(|p| {
            let t = std::fs::metadata(&p).ok()?.modified().ok()?;
            Some((t, p))
        })
        .collect();
    if !candidats.is_empty() {
        candidats.sort_by_key(|(t, _)| *t);
        return candidats.pop().map(|(_, p)| p);
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
        // Pas de tableau de bord terminal: la fenetre console qui s'ouvrait a cote
        // de l'application n'etait pas un artefact, c'etait le TUI complet du noeud
        // (journaux, jauges, onglets). Il a tout son sens quand on lance
        // laruche-node.exe soi-meme; lance PAR l'application, personne ne le
        // regarde. Le noeud bascule alors sur serveur + icone de barre systeme.
        .arg("--no-tui")
        .current_dir(&dossier);
    // Et aucune console, pas meme un clignotement. `--no-tui` seul ne suffit pas:
    // laruche-node est un binaire console, donc Windows lui en attribue une des
    // qu'il demarre, vide et par-dessus tout le reste.
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        commande.creation_flags(CREATE_NO_WINDOW);
    }
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

use tauri::Manager;

/// Le noeud que NOUS avons demarre, ou rien.
///
/// Il naissait dans `main` et mourait avec elle. Depuis que la resolution part
/// dans un fil, pour que la fenetre s'ouvre sans l'attendre, il faut un point de
/// rendez-vous entre ce fil et la boucle d'evenements qui, seule, sait quand
/// fermer. Sans lui, le noeud lance par l'application survivrait a sa fenetre.
static ENFANT: std::sync::Mutex<Option<Child>> = std::sync::Mutex::new(None);

/// Ecrit la page de choix a cote des donnees de l'application et rend son URL.
fn page_choix_url(html: &str) -> tauri::Url {
    let chemin = std::env::temp_dir().join("laruche-choix.html");
    let _ = std::fs::write(&chemin, html);
    tauri::Url::from_file_path(&chemin).expect("chemin temporaire absolu")
}

/// Ou pointer la fenetre, et faut-il arreter un noeud en partant.
/// Vrai quand un noeud repondait deja: la coque s'y rattache au lieu d'en lancer
/// un. Un booleen partage plutot qu'une valeur de retour de plus, parce que le
/// canal qui relie `resoudre` a la fenetre porte deja une paire et que le
/// troisieme element ne servirait qu'a une phrase.
static DEJA_LA: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

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

    // 2. Un noeud local repond deja: on s'y rattache, on n'en lance pas un second.
    //
    // C'etait deja le comportement, mais rien ne le DISAIT: l'ecran annoncait
    // « Demarrage de LaRuche » dans tous les cas, et on pouvait croire qu'un
    // second noeud allait se lancer par-dessus le premier. Ce sont deux
    // situations differentes, elles meritent deux phrases.
    if noeud_repond(URL_LOCALE) {
        DEJA_LA.store(true, std::sync::atomic::Ordering::Relaxed);
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

/// La palette de l'ecran d'attente, lue dans le foyer avant d'ouvrir la fenetre.
///
/// Cet ecran s'affiche AVANT que le noeud existe: il ne peut donc rien lui
/// demander, et il restait sombre et ambre quel que soit le theme choisi. Il est
/// pourtant la premiere chose que l'on voit, et la seule surface de LaRuche a ne
/// pas suivre le reglage de l'utilisateur.
///
/// Le theme actif est un simple fichier, `themes/actif.txt`. Un theme a soi porte
/// ses valeurs dans son propre JSON, qu'on lit exactement. Les themes livres
/// vivent dans la feuille de style de l'application, hors d'atteinte d'ici: leurs
/// quatre couleurs sont donc reprises ci-dessous. Quatre sur trente-trois, pour
/// un ecran qui affiche un logo, un titre et une ligne de detail: la duplication
/// est bornee, et elle est le prix d'un demarrage qui ne clignote pas.
fn palette_attente(foyer: &std::path::Path) -> (String, String, String, String) {
    const DEFAUT: (&str, &str, &str, &str) = ("#0f0f10", "#e8e8ea", "#f5a623", "#8b8b92");
    const INTEGRES: &[(&str, (&str, &str, &str, &str))] = &[
        ("defaut", DEFAUT),
        ("ardoise", ("#0b0d10", "#f1f5f9", "#7dd3fc", "#94a3b8")),
        ("foret", ("#0a0f0d", "#ecfdf5", "#6ee7b7", "#9ca3af")),
        ("nuit", ("#000000", "#fafafa", "#fbbf24", "#a1a1aa")),
        ("papier", ("#faf7f2", "#1c1917", "#b45309", "#57534e")),
        ("nature", ("#0a0f0d", "#ecfdf5", "#39f3a9", "#86a99b")),
    ];
    let actif = std::fs::read_to_string(foyer.join("themes").join("actif.txt"))
        .unwrap_or_default()
        .trim()
        .to_string();
    if let Some(id) = actif.strip_prefix("perso:") {
        // Un identifiant vient d'un fichier: il sert de nom de fichier, donc on
        // n'en garde que ce qui ne peut pas remonter l'arborescence.
        let sur: String = id
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
            .collect();
        if !sur.is_empty() {
            if let Ok(t) = std::fs::read_to_string(foyer.join("themes").join(format!("{sur}.json")))
            {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&t) {
                    let j = &v["jetons"];
                    let lire = |cle: &str, repli: &str| {
                        j[cle].as_str().unwrap_or(repli).to_string()
                    };
                    return (
                        lire("--bg", DEFAUT.0),
                        lire("--text", DEFAUT.1),
                        lire("--amber", DEFAUT.2),
                        lire("--text-dim", DEFAUT.3),
                    );
                }
            }
        }
    }
    let p = INTEGRES
        .iter()
        .find(|(id, _)| *id == actif)
        .map(|(_, p)| *p)
        .unwrap_or(DEFAUT);
    (p.0.into(), p.1.into(), p.2.into(), p.3.into())
}

/// Le foyer de la ruche, vu depuis l'application de bureau.
///
/// Le noeud choisit son foyer puis s'y PLACE, une fois, au tout debut. L'ecran
/// d'attente lisait donc ses deux fichiers, le theme actif et la langue, dans le
/// repertoire courant, en supposant que c'etait le meme endroit. Ce n'est vrai
/// que si l'on lance l'application depuis le foyer: une copie de `laruche.exe`
/// posee sur le bureau cherchait `themes/actif.txt` et `langue.txt` sur le
/// bureau, ne trouvait rien, et retombait sur le theme sombre et le francais.
/// L'application se lance justement presque toujours d'ailleurs que du foyer.
///
/// Meme regle que le noeud, et dans le meme ordre, sans quoi les deux ne
/// parleraient pas de la meme ruche.
fn foyer() -> std::path::PathBuf {
    if let Ok(d) = std::env::var("LARUCHE_DATA_DIR") {
        if !d.is_empty() {
            return std::path::PathBuf::from(d);
        }
    }
    let ici = std::env::current_dir().unwrap_or_default();
    for marqueur in [
        "memoire.db",
        "config.json",
        "laruche.toml",
        "secrets.enc",
        "missions.json",
        "cron-tasks.json",
    ] {
        if ici.join(marqueur).exists() {
            return ici;
        }
    }
    let nom = if cfg!(target_os = "linux") { "laruche" } else { "LaRuche" };
    dirs::data_dir().map(|d| d.join(nom)).unwrap_or(ici)
}

/// Les phrases de l'ecran d'attente, dans la langue reglee dans l'interface.
///
/// Cet ecran est une fenetre a part, servie depuis l'application et non depuis le
/// noeud: il n'a acces ni au cookie ni au `localStorage` ou vivait le choix de
/// langue, et il annoncait donc son demarrage en francais meme quand toute
/// l'interface etait en anglais. Le noeud ecrit desormais ce choix dans
/// `langue.txt`, au meme titre que `themes/actif.txt` pour le theme, et un
/// fichier se lit d'ici sans rien demander a personne.
///
/// Sept phrases, ecrites la plutot que tirees de `strings.json`: ce fichier est
/// compile dans le noeud, pas dans l'application de bureau, et aller le chercher
/// pour sept lignes couterait une dependance entre deux binaires qui n'en ont
/// aucune autre.
fn textes_attente(foyer: &std::path::Path) -> serde_json::Value {
    let en = std::fs::read_to_string(foyer.join("langue.txt"))
        .map(|c| c.trim() == "en")
        .unwrap_or(false);
    if en {
        serde_json::json!({
            "titre": "Starting LaRuche...",
            "detail": "The hive is opening its memory and probing the network. A few seconds.",
            "lent": "This is taking longer than usual. The memory may be indexing for the first time.",
            "dejaTitre": "LaRuche is already running",
            "dejaDetail": "A node answers on this machine: attaching to it, rather than starting a second one.",
            "echecTitre": "LaRuche did not start",
            "echecDetail": "Run laruche-node.exe by hand to see the error."
        })
    } else {
        serde_json::json!({
            "titre": "Demarrage de LaRuche...",
            "detail": "La ruche ouvre sa memoire et sonde le reseau. Quelques secondes.",
            "lent": "C'est plus long que d'habitude. La memoire s'indexe peut-etre pour la premiere fois.",
            "dejaTitre": "LaRuche tourne deja",
            "dejaDetail": "Un noeud repond sur ce poste: on s'y rattache, sans en lancer un second.",
            "echecTitre": "LaRuche n'a pas demarre",
            "echecDetail": "Lance laruche-node.exe a la main pour voir l'erreur."
        })
    }
}

/// Le fond de fenetre, en composantes, pour que Tauri peigne deja la bonne
/// couleur avant le premier octet de HTML.
fn composantes(hex: &str) -> (u8, u8, u8) {
    // Un jeton de theme n'est pas toujours un hexa: des qu'on touche a son
    // opacite, l'interface l'ecrit `rgba(250,247,242,0)`. Cette fonction ne
    // savait lire que la premiere forme et retombait sur son gris sombre, ce qui
    // donnait une fenetre sombre pour un theme clair.
    let t = hex.trim();
    if let Some(dedans) = t
        .strip_prefix("rgba(")
        .or_else(|| t.strip_prefix("rgb("))
        .and_then(|x| x.strip_suffix(')'))
    {
        let n: Vec<u8> = dedans
            .split(',')
            .take(3)
            .filter_map(|c| c.trim().parse::<f32>().ok())
            .map(|v| v.clamp(0.0, 255.0) as u8)
            .collect();
        if n.len() == 3 {
            return (n[0], n[1], n[2]);
        }
    }
    let h = t.trim_start_matches('#');
    if h.len() == 6 {
        if let (Ok(r), Ok(v), Ok(b)) = (
            u8::from_str_radix(&h[0..2], 16),
            u8::from_str_radix(&h[2..4], 16),
            u8::from_str_radix(&h[4..6], 16),
        ) {
            return (r, v, b);
        }
    }
    (0x0f, 0x0f, 0x10)
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

    // La fenetre s'ouvre TOUT DE SUITE, sur l'ecran d'attente, et la resolution
    // part dans un fil.
    //
    // `resoudre()` demarre le noeud et sonde jusqu'a 45 secondes: tant qu'elle
    // tournait ici, rien n'etait construit et l'utilisateur avait double-clique
    // dans le vide. C'etait supportable tant que la console du noeud servait de
    // preuve de vie; maintenant qu'elle est silencieuse, ce ne l'est plus.
    let (envoi, reception) = std::sync::mpsc::channel::<(tauri::Url, Option<Child>)>();
    std::thread::spawn(move || {
        let _ = envoi.send(resoudre());
    });

    // La palette est lue UNE fois, avant la fenetre: l'ecran d'attente s'ouvre
    // deja dans le theme de l'utilisateur, sans transition ni clignotement.
    let (att_fond, att_texte, att_accent, att_attenue) =
        palette_attente(&foyer());
    let (fond_r, fond_v, fond_b) = composantes(&att_fond);
    let textes = textes_attente(&foyer());
    let script_init = format!(
        "window.__LARUCHE_BUREAU__ = true; window.__LARUCHE_PALETTE__ = {}; window.__LARUCHE_TEXTES__ = {};",
        serde_json::json!({
            "fond": att_fond, "texte": att_texte,
            "accent": att_accent, "attenue": att_attenue
        }),
        textes
    );

    tauri::Builder::default()
        .setup(move |app| {
            tauri::WebviewWindowBuilder::new(app, "principale", tauri::WebviewUrl::App("index.html".into()))
                .title("LaRuche")
                .inner_size(1400.0, 900.0)
                // 380 px de large: la SPA est deja responsive et bascule en
                // presentation telephone sous ~640 px. Un minimum a 900 empechait
                // simplement d'y arriver, et donc de voir cette mise en page.
                .min_inner_size(380.0, 500.0)
                // Meme fond que la page: sans cela la fenetre clignote en blanc le
                // temps que la SPA peigne.
                .background_color(tauri::window::Color(fond_r, fond_v, fond_b, 0xff))
                // Un marqueur, pose avant que la page ne s'execute.
                //
                // La SPA est la meme dans un navigateur et ici, mais un lien
                // `target="_blank"` ne fait RIEN dans cette fenetre: la webview
                // n'ouvre pas d'onglet, et il n'y a pas de navigateur autour
                // pour en ouvrir un. Tous les liens du logiciel etaient donc
                // morts dans l'application de bureau, en silence.
                //
                // La page a besoin de savoir ou elle tourne pour choisir: son
                // comportement normal dans un navigateur, un passage par le
                // noeud ici, qui sait ouvrir le navigateur du systeme.
                .initialization_script(&script_init)
                .build()?;

            // Des que la cible est connue, la fenetre y va. Un fil de plus plutot
            // qu'une attente dans `setup`: Tauri veut la main pour peindre, et
            // bloquer ici reviendrait exactement a ce qu'on vient de defaire.
            let fenetre = app.get_webview_window("principale").expect("fenetre creee");
            let sortie = app.handle().clone();
            std::thread::spawn(move || match reception.recv() {
                Ok((cible, ne)) => {
                    // L'enfant voyage jusqu'a la boucle d'evenements, qui seule sait
                    // quand l'arreter: sans cela le noeud que NOUS avons lance
                    // survivrait a la fermeture de la fenetre.
                    if let Some(c) = ne {
                        ENFANT.lock().expect("verrou enfant").replace(c);
                    }
                    // Le dire AVANT de naviguer: la phrase n'a que le temps du
                    // chargement pour etre lue, mais elle explique pourquoi
                    // l'application s'ouvre instantanement au lieu de mettre
                    // quinze secondes comme au premier lancement.
                    if DEJA_LA.load(std::sync::atomic::Ordering::Relaxed) {
                        let _ = fenetre.eval(
                            "var x=window.__LARUCHE_TEXTES__||{};var t=document.getElementById('titre');if(t) t.textContent=x.dejaTitre||t.textContent;var d=document.getElementById('detail');if(d) d.textContent=x.dejaDetail||d.textContent;",
                        );
                    }
                    let _ = fenetre.navigate(cible);
                }
                // Le fil est mort sans repondre: on ne laisse pas la fenetre sur un
                // ecran d'attente perpetuel, elle doit dire ce qui s'est passe.
                Err(_) => {
                    let _ = fenetre.eval(
                        "var x=window.__LARUCHE_TEXTES__||{};document.getElementById('titre').textContent=x.echecTitre||'LaRuche';document.getElementById('detail').textContent=x.echecDetail||'';document.querySelector('.barre').style.display='none';",
                    );
                    let _ = sortie;
                }
            });
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("construction de l'application")
        .run(move |_app, evenement| {
            // On n'arrete que ce qu'on a demarre. Un noeud qui tournait avant nous
            // continue apres nous - fermer la fenetre ne doit pas couper un service
            // que quelqu'un d'autre utilise.
            if let tauri::RunEvent::Exit = evenement {
                if let Some(c) = ENFANT.lock().expect("verrou enfant").as_mut() {
                    let _ = c.kill();
                }
            }
        });
}
