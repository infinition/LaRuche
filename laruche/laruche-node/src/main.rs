//! LaRuche Node Daemon
//!
//! The main process that runs on each LaRuche box. It:
//! 1. Broadcasts its Cognitive Manifest via Miel (mDNS)
//! 2. Listens for peer nodes (swarm)
//! 3. Exposes an inference API (proxying to Ollama)
//! 4. Manages authentication via Proof of Proximity
//! 5. Runs the web dashboard
//! 6. Exposes /models to list available Ollama models
//! 7. Reports real system metrics (CPU, RAM) via sysinfo
//! 8. Exposes MCP server for external AI clients
//! 9. Discord & Slack channel integrations

/// Delivery channel that writes the result into the cognitive memory instead of sending it
/// to an external service. The only one that needs no token and no configuration, which is
/// why it is always offered in the pickers.
pub(crate) const CANAL_MEMOIRE: &str = "memory";

mod abeilles_local;
mod arbitre_memoire;
mod auth_user;
mod themes_api;
mod local_inference;
mod mcp;
mod missions;
mod outbox;
mod profiles;
mod secrets_vault;
mod sync;
mod systray;
mod tui;
mod config_api;
mod plugins_api;
mod voice_api;
mod profiles_api;
mod knowledge_api;
mod web;
mod slack_api;
mod local_api;
mod ws_chat;
mod episodes_api;
mod ws_navigateur;
mod discord_api;
mod channels_api;
mod auth_api;
mod events_api;
mod credentials_api;
mod settings_api;
mod deliberation_api;
mod doctor_api;
mod kanban_api;
mod watchers_api;
mod skills_api;
mod missions_api;
mod sessions_api;
mod memory_api;
mod feed_api;
mod mesh_api;
mod changes_api;
mod memory_crud_api;
mod tools_api;
mod openai_api;
mod swarm_api;
mod mcp_api;
mod mcp_pare_feu;
mod status_api;
mod blueprints_api;
mod reine_api;
mod voice_config;
mod totp;
mod state;
mod helpers;
mod router;
mod background;
mod okf_git;

pub(crate) use state::*;
pub(crate) use helpers::*;

use anyhow::Result;
use axum::{
    extract::{ws, WebSocketUpgrade},
    http::StatusCode,
};
use miel_protocol::{
    auth::ProximityAuth,
    capabilities::{Capability, CapabilityInfo},
    discovery::{MielBroadcaster, MielListener},
    manifest::{CognitiveManifest, HardwareTier},
    qos::{QosPolicy, RequestQueue},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap, collections::HashSet, fs, net::SocketAddr, path::PathBuf, sync::Arc,
    time::Duration,
};
use sysinfo::System;
use tokio::sync::{broadcast, RwLock};
use tracing::{error, info, warn};
use uuid::Uuid;

use laruche_essaim::{
    abeilles::{charger_plugins, enregistrer_abeilles_builtin, enregistrer_delegation},
    brain::{boucle_react_memoire, boucle_react_memoire_multimodal},
    cron::{CronScheduler, ScheduledTask},
    mcp_client::charger_mcp_servers,
    AbeilleRegistry, ChatEvent, EssaimConfig, Session,
};

use std::collections::VecDeque;

// Web asset serving (SPA shell, CSS, concatenated JS) and i18n language-file
// injection live in `web.rs` (handlers: web::spa_page / app_css / app_js / lang_file).


/// What a fresh install sees first.
///
/// The node used to start into a console and say nothing usable: the web interface
/// existed, on a port nobody had been told about, and there was no way onto a phone at
/// all. Three things fix that, in the order someone actually needs them: a link they can
/// click, a browser that opens by itself, and a QR code for the device that is not this
/// machine.
///
/// Best-effort throughout. A headless server has no browser to open and no terminal to
/// draw in; none of that is a reason to fail a boot.
fn accueil_demarrage(scheme: &str, port: u16) {
    let local = format!("{scheme}://localhost:{port}");
    println!();
    println!("  LaRuche est prete.");
    println!();
    println!("    Sur cette machine : {local}");

    // The LAN address is what a phone needs; localhost means nothing to it.
    let lan = detect_local_ip().map(|ip| format!("{scheme}://{ip}:{port}"));
    if let Some(url) = &lan {
        println!("    Depuis le reseau  : {url}");
    }

    // QR of the LAN address when there is one: scanning a `localhost` code from a phone
    // would open the phone's own web server, which is a confusing kind of nothing.
    if let Some(qr) = lan.as_deref().and_then(auth_user::qr_terminal) {
        println!();
        println!("    Scanner pour ouvrir sur un telephone :");
        for ligne in qr.lines() {
            println!("    {ligne}");
        }
    }
    println!();

    // Opt-out rather than opt-in: someone running a daemon knows to set it, someone
    // installing for the first time should not have to.
    if std::env::var("LARUCHE_NO_BROWSER").is_ok() {
        return;
    }
    if let Err(e) = open::that_detached(&local) {
        tracing::debug!(error = %e, "could not open a browser (headless?)");
        println!("    (Ouvre le lien ci-dessus dans ton navigateur.)");
        println!();
    }
}

/// Best-effort local LAN IP (for the cert SAN list and the phone QR), via a UDP connect
/// trick. No packet is actually sent; the OS just picks the outbound interface address.
pub(crate) fn detect_local_ip() -> Option<String> {
    let sock = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    sock.connect("8.8.8.8:80").ok()?;
    sock.local_addr().ok().map(|a| a.ip().to_string())
}

/// Ensure a self-signed cert + key exist on disk (generated once), returning their
/// paths. The SANs cover localhost, 127.0.0.1 and the detected LAN IP so HTTPS works
/// from other devices. Browsers warn on a self-signed cert: accept it once.
fn ensure_self_signed_cert() -> Option<(String, String)> {
    let cert_path = "laruche-cert.pem";
    let key_path = "laruche-key.pem";
    if std::path::Path::new(cert_path).exists() && std::path::Path::new(key_path).exists() {
        return Some((cert_path.to_string(), key_path.to_string()));
    }
    let mut sans = vec!["localhost".to_string(), "127.0.0.1".to_string()];
    if let Some(ip) = detect_local_ip() {
        if !sans.contains(&ip) {
            sans.push(ip);
        }
    }
    let certified = rcgen::generate_simple_self_signed(sans).ok()?;
    std::fs::write(cert_path, certified.cert.pem()).ok()?;
    std::fs::write(key_path, certified.key_pair.serialize_pem()).ok()?;
    info!(cert = cert_path, "generated self-signed TLS certificate for HTTPS");
    Some((cert_path.to_string(), key_path.to_string()))
}

// ======================== Main ========================

/// Serve `app` on `addr`, with optional TLS. A bad/unreadable cert pair no longer panics
/// the server task: it logs and falls back to plain HTTP so the node stays reachable.
async fn serve_with_optional_tls(app: axum::Router, addr: String, tls: Option<(String, String)>) {
    let make = app.into_make_service_with_connect_info::<SocketAddr>();
    if let Some((cert, key)) = tls {
        info!(cert = %cert, key = %key, "TLS enabled: starting HTTPS server");
        match axum_server::tls_rustls::RustlsConfig::from_pem_file(&cert, &key).await {
            Ok(cfg) => match addr.parse::<SocketAddr>() {
                Ok(bind_addr) => {
                    let _ = axum_server::bind_rustls(bind_addr, cfg).serve(make).await;
                }
                Err(e) => error!(error = %e, addr = %addr, "Invalid bind address for HTTPS"),
            },
            Err(e) => {
                error!(error = %e, "Failed to load TLS cert/key; falling back to HTTP");
                match tokio::net::TcpListener::bind(&addr).await {
                    Ok(l) => { let _ = axum::serve(l, make).await; }
                    Err(e) => error!(error = %e, addr = %addr, "Failed to bind HTTP fallback"),
                }
            }
        }
    } else {
        match tokio::net::TcpListener::bind(&addr).await {
            Ok(l) => { let _ = axum::serve(l, make).await; }
            Err(e) => error!(error = %e, addr = %addr, "Failed to bind HTTP listener"),
        }
    }
}

/// Skills et plugins livres avec LaRuche, embarques DANS le binaire.
///
/// Sans cela, un `laruche-node.exe` telecharge depuis les releases - ou installe
/// par l'application de bureau - demarre une ruche a zero capacite, alors que le
/// depot en contient trente-huit. Elles etaient jusqu'ici simplement « les fichiers
/// qui se trouvaient a cote », ce qui ne marchait que depuis une copie du depot.
///
/// ~580 Ko de markdown dans un binaire de 33 Mo: le prix est negligeable, et il n'y
/// a plus aucun fichier a livrer a cote de l'executable.
static SKILLS_LIVRES: include_dir::Dir<'_> =
    include_dir::include_dir!("$CARGO_MANIFEST_DIR/../skills");
static PLUGINS_LIVRES: include_dir::Dir<'_> =
    include_dir::include_dir!("$CARGO_MANIFEST_DIR/../plugins");
static MCP_LIVRES: include_dir::Dir<'_> =
    include_dir::include_dir!("$CARGO_MANIFEST_DIR/../mcp");

/// Depose le contenu livre dans le foyer, si et seulement si le dossier n'existe pas.
///
/// La condition porte sur le DOSSIER, pas sur chaque fichier: quelqu'un qui supprime
/// une capacite ne doit pas la voir revenir au redemarrage suivant. En contrepartie,
/// les capacites ajoutees par une mise a jour n'apparaissent pas toutes seules dans
/// un foyer deja etabli - c'est le compromis, et il penche du cote de « on ne
/// ressuscite pas ce que l'utilisateur a efface ».
fn amorcer(livre: &include_dir::Dir<'_>, cible: &str) {
    let racine = std::path::Path::new(cible);
    if racine.exists() {
        return;
    }
    // Creer le dossier de base AVANT d'extraire: `extract` ne cree que les
    // sous-dossiers, si bien qu'un fichier pose a la racine du contenu livre
    // (skills/AUTHORING.md) n'avait nulle part ou atterrir. plugins/ s'en sortait
    // par accident, n'ayant que des sous-dossiers.
    if let Err(e) = std::fs::create_dir_all(racine) {
        error!(dossier = cible, error = %e, "amorcage impossible: dossier non creable");
        return;
    }
    // L'erreur est journalisee, jamais avalee: un amorcage qui echoue en silence
    // donne une ruche sans capacites et aucune trace pour comprendre pourquoi.
    match livre.extract(racine) {
        Ok(()) => {
            let n = std::fs::read_dir(racine).map(|d| d.count()).unwrap_or(0);
            info!(dossier = cible, entrees = n, "foyer neuf: contenu livre depose");
        }
        Err(e) => {
            error!(dossier = cible, error = %e, "amorcage impossible: la ruche demarrera sans");
        }
    }
}

/// Verrouille le foyer pour ce processus, ou explique pourquoi c'est impossible.
///
/// Deux noeuds peuvent tres bien ouvrir le MEME foyer: il suffit de les lancer sur des
/// ports differents. Ils partagent alors `identity.json`, donc ils s'annoncent sur le
/// reseau avec le MEME identifiant a deux adresses - l'essaim n'y comprend rien - et
/// ils ecrivent tous les deux dans `memoire.db`, `laruche-state.json` et `skills/`.
///
/// Le verrou porte le PID et le nom du processus. Un PID mort, ou reutilise par un
/// autre programme, rend le verrou perime: un noeud tue brutalement ne se condamne
/// donc pas lui-meme au redemarrage suivant.
fn verrouiller_foyer() -> Result<(), String> {
    let chemin = std::path::Path::new("laruche.lock");
    let moi = std::process::id();
    let mon_nom = std::env::current_exe()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
        .unwrap_or_else(|| "laruche-node".to_string());

    if let Ok(contenu) = std::fs::read_to_string(chemin) {
        let mut lignes = contenu.lines();
        let pid = lignes.next().and_then(|l| l.trim().parse::<u32>().ok());
        let nom = lignes.next().unwrap_or("").trim().to_string();
        if let Some(pid) = pid {
            if pid != moi {
                let mut sys = System::new();
                let cible = sysinfo::Pid::from_u32(pid);
                sys.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[cible]), true);
                // Le nom doit correspondre AUSSI: un PID est reutilisable, et refuser de
                // demarrer parce qu'un editeur de texte a herite du numero serait absurde.
                let vivant = sys
                    .process(cible)
                    .map(|p| p.name().to_string_lossy().eq_ignore_ascii_case(&nom))
                    .unwrap_or(false);
                if vivant {
                    return Err(format!(
                        "une autre LaRuche (PID {pid}) utilise deja ce foyer. Deux noeuds sur le \
                         meme dossier partagent identity.json et memoire.db: ils s'annoncent avec \
                         le meme identifiant et s'ecrasent mutuellement. Fermer l'autre, ou lancer \
                         celui-ci avec LARUCHE_DATA_DIR sur un dossier a lui."
                    ));
                }
            }
        }
    }
    std::fs::write(chemin, format!("{moi}\n{mon_nom}\n"))
        .map_err(|e| format!("verrou du foyer non ecrit: {e}"))
}

/// Le foyer de cette ruche: ou vivent `memoire.db`, `sessions/`, `skills/`,
/// `plugins/`, les secrets et la configuration.
///
/// Tout le code lit ces chemins relativement au repertoire courant. On choisit donc
/// le foyer UNE fois, au tout debut, et on s'y place - plutot que de reecrire des
/// dizaines de chemins et d'en oublier un.
///
/// L'ordre compte:
///
///   1. `LARUCHE_DATA_DIR`, quand on veut decider soi-meme;
///   2. le repertoire courant s'il EST deja une ruche - c'est ce qui fait que
///      lancer_butinage.bat depuis le depot continue d'ouvrir la meme memoire
///      qu'avant, sans que rien n'ait a bouger;
///   3. sinon le dossier standard de l'utilisateur, pour que le double-clic sur
///      l'executable, le service et l'application de bureau tombent tous les trois
///      sur la MEME ruche, au lieu d'en fabriquer une chacun a cote de leur binaire.
fn foyer() -> std::path::PathBuf {
    if let Ok(d) = std::env::var("LARUCHE_DATA_DIR") {
        if !d.is_empty() {
            return std::path::PathBuf::from(d);
        }
    }
    let ici = std::env::current_dir().unwrap_or_default();
    // Marqueurs volontairement etroits: des fichiers qui n'existent que dans une
    // ruche deja etablie. `skills/` en ferait partie a tort - il est livre avec
    // l'installation, y compris dans un dossier ou l'on n'a pas le droit d'ecrire.
    // Plusieurs marqueurs, et non le seul `memoire.db`: un foyer a moitie deplace -
    // la base partie, les missions, les watchers et les secrets restes - serait
    // sinon abandonne d'un coup, et la ruche repartirait de zero en laissant
    // derriere elle des fichiers que plus personne ne lit.
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
    // Windows : %APPDATA%\LaRuche
    // macOS   : ~/Library/Application Support/LaRuche
    // Linux   : ~/.local/share/laruche  (XDG_DATA_HOME), en minuscules par usage
    let nom = if cfg!(target_os = "linux") { "laruche" } else { "LaRuche" };
    dirs::data_dir().map(|d| d.join(nom)).unwrap_or(ici)
}

#[tokio::main]
async fn main() -> Result<()> {
    // Avant TOUTE ouverture de fichier: la configuration, la memoire et les sessions
    // se resolvent depuis le repertoire courant.
    let foyer = foyer();
    if let Err(e) = std::fs::create_dir_all(&foyer) {
        eprintln!("impossible de creer {} : {e}", foyer.display());
    }
    if let Err(e) = std::env::set_current_dir(&foyer) {
        eprintln!("impossible de se placer dans {} : {e}", foyer.display());
    }
    // Avant d'ouvrir quoi que ce soit: deux noeuds dans le meme foyer se marchent sur
    // les pieds en silence, ce qui est bien pire qu'un refus de demarrer explicite.
    if let Err(raison) = verrouiller_foyer() {
        eprintln!("\n  LaRuche ne demarre pas: {raison}\n");
        std::process::exit(1);
    }

    let use_tui = !std::env::args().any(|a| a == "--no-tui");

    let tui_log_rx = if use_tui {
        // Layered subscriber: TUI captures logs + optional stderr fallback
        use tracing_subscriber::layer::SubscriberExt;
        use tracing_subscriber::util::SubscriberInitExt;
        let (tui_buf, rx) = tui::TuiLogBuffer::new();
        let tui_layer = tui::TuiTracingLayer::new(tui_buf.sender());
        let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "laruche_node=info,miel_protocol=info,laruche_essaim=info".into());
        tracing_subscriber::registry()
            .with(env_filter)
            .with(tui_layer)
            .init();
        Some(rx)
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "laruche_node=info,miel_protocol=info".into()),
            )
            .init();
        None
    };

    let config = load_config()?;

    // Un foyer neuf doit arriver equipe: sans cela la ruche demarre sans aucune
    // capacite et l'utilisateur n'a aucun moyen de deviner ce qui manque. Pose ICI
    // et non au tout debut de main(): avant l'initialisation des traces, un echec
    // d'amorcage partait dans le vide et ne laissait aucune ligne de journal.
    amorcer(&SKILLS_LIVRES, "skills");
    amorcer(&PLUGINS_LIVRES, "plugins");
    amorcer(&MCP_LIVRES, "mcp");


    info!(name = %config.node_name, tier = ?config.tier, "Starting LaRuche node");

    let local_ip = miel_protocol::get_local_ip();
    info!(ip = %local_ip, "Detected local IP");

    let mut manifest = CognitiveManifest::new(config.node_name.clone(), config.tier);
    // PERSISTENT IDENTITY (identity.json). Without it, node_id = Uuid::new_v4() at EVERY startup:
    // the ruche appears as a NEW node to peers at every reboot (the old one expires) → this is
    // a direct cause of flapping. We load the saved ID, or persist the generated one.
    {
        let id_path = std::path::Path::new("identity.json");
        let saved = std::fs::read_to_string(id_path)
            .ok()
            .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
            .and_then(|v| v.get("node_id").and_then(|x| x.as_str()).map(String::from))
            .and_then(|s| Uuid::parse_str(&s).ok());
        match saved {
            Some(id) => {
                manifest.node_id = id;
                info!(node_id = %id, "Identity loaded (identity.json)");
            }
            None => {
                let _ = std::fs::write(
                    id_path,
                    serde_json::json!({ "node_id": manifest.node_id.to_string() }).to_string(),
                );
                info!(node_id = %manifest.node_id, "New identity persisted (identity.json)");
            }
        }
    }
    manifest.api_endpoint.host = local_ip;
    manifest.api_endpoint.port = config.api_port;
    manifest.api_endpoint.dashboard_port = config.dashboard_port;

    for cap_config in &config.capabilities {
        if let Some(cap) = Capability::from_flag(&cap_config.capability) {
            manifest.capabilities.add(CapabilityInfo {
                capability: cap,
                model_name: cap_config.model_name.clone(),
                model_size: cap_config.model_size.clone(),
                quantization: cap_config.quantization.clone(),
                max_context_length: Some(8192),
            });
            info!(capability = %cap, model = %cap_config.model_name, "Registered capability");
        }
    }

    // This node is also an agent (Essaim)
    manifest.capabilities.add(CapabilityInfo {
        capability: Capability::Agent,
        model_name: config.default_model.clone(),
        model_size: None,
        quantization: None,
        max_context_length: Some(8192),
    });
    info!(capability = "agent", "Registered Essaim agent capability");

    // PRIVACY NOTE: we NO LONGER announce locally detected backends at startup. The mesh
    // should only expose explicitly public providers (`public_proxy`): it's the re-announce
    // loop (below) that rebuilds the capabilities from the public set only.

    // Feed journal (persistent): loads the history of system events at startup.
    laruche_essaim::feed_journal::init(std::path::PathBuf::from("feed-journal.ndjson"), 500);

    // Secrets vault: decrypts the at-rest file → in-memory view (never re-serialized).
    // Tools/providers substitute `${NAME}` with the real value without showing it to the LLM.
    laruche_essaim::secrets::init(secrets_vault::charger());

    // Gap D: USER HOOKS: loads `hooks.json` (pre/post-tool) if it exists.
    {
        let hooks = std::fs::read_to_string("hooks.json")
            .ok()
            .and_then(|s| serde_json::from_str::<Vec<laruche_essaim::hooks::Hook>>(&s).ok())
            .unwrap_or_default();
        if !hooks.is_empty() {
            eprintln!("🪝 {} user hook(s) loaded from hooks.json", hooks.len());
        }
        laruche_essaim::hooks::init(hooks);
    }

    let mut broadcaster = MielBroadcaster::new()?;
    broadcaster.register(&manifest)?;
    let broadcaster = Arc::new(broadcaster);

    let mut listener = MielListener::new()?;
    let _discovered_nodes = listener.start()?;

    let mut sys = System::new_all();
    sys.refresh_all();

    // Load persistent state (activity log, default model) from previous session
    let state_file_path = resolve_state_file_path();
    let persistent = load_persistent_state(&state_file_path);

    // Build initial per-capability default models map:
    // 1) Start from config capabilities
    // 2) Overlay with persisted runtime choices from last session
    let mut initial_defaults: HashMap<String, String> = HashMap::new();
    for cap in &config.capabilities {
        let cap_name = normalize_capability_label(&cap.capability);
        initial_defaults
            .entry(cap_name)
            .or_insert_with(|| cap.model_name.clone());
    }
    // Ensure "llm" is always present
    initial_defaults
        .entry("llm".into())
        .or_insert_with(|| config.default_model.clone());
    // Overlay persisted state (takes priority: user's runtime choices)
    if let Some(persisted_map) = persistent.default_models {
        for (k, v) in persisted_map {
            if !v.is_empty() {
                initial_defaults.insert(k, v);
            }
        }
    } else if let Some(dm) = persistent.default_model.filter(|m| !m.is_empty()) {
        // Legacy migration: single default_model → "llm" entry
        initial_defaults.insert("llm".into(), dm);
    }

    // Pre-populate activity log from persistent state
    let mut initial_log = VecDeque::with_capacity(ACTIVITY_LOG_LIMIT);
    for entry in persistent
        .activity_log
        .into_iter()
        .rev()
        .take(ACTIVITY_LOG_LIMIT)
    {
        initial_log.push_front(entry);
    }

    // Load provider profiles (multi-provider support)
    let profiles_path = PathBuf::from("provider-profiles.json");
    let mut profiles_cfg = profiles::load_profiles(&profiles_path);

    // Migrate old single-provider config into profiles if no profiles exist beyond default
    if profiles_cfg.profiles.len() <= 1
        && !config.provider.is_empty()
        && config.provider != "ollama"
    {
        let migrated_id = format!("{}-migrated", config.provider);
        profiles_cfg.profiles.insert(
            migrated_id.clone(),
            profiles::ProviderProfile {
                provider: config.provider.clone(),
                name: config.provider.clone(),
                base_url: config.api_base.clone().unwrap_or_else(|| {
                    match config.provider.as_str() {
                        "openai" => "https://api.openai.com".to_string(),
                        "anthropic" => "https://api.anthropic.com".to_string(),
                        _ => String::new(),
                    }
                }),
                api_key: config.api_key.clone(),
                models: vec![config.default_model.clone()],
                visibilite: Default::default(), allowed_peers: Vec::new(),
                max_context_length: match config.provider.as_str() {
                    "anthropic" => 200000,
                    "openai" => 128000,
                    _ => 32768,
                },
            },
        );
        profiles_cfg.active_model = profiles::ActiveModel {
            profile_id: migrated_id,
            model: config.default_model.clone(),
        };
        let _ = profiles::save_profiles(&profiles_path, &profiles_cfg);
        info!("Migrated legacy provider config into profiles");
    }

    // Auto-discover local models at startup.
    profiles::refresh_ollama_profiles(&mut profiles_cfg).await;
    profiles::ensure_llamacpp_8001_profile(&mut profiles_cfg).await;

    // Cleanup of duplicate profiles (historical bug in /api/models/use that created
    // duplicate "local-<host>" + OpenAI profiles with an empty base_url).
    {
        // 1) Remove OpenAI profiles with an empty base_url (broken, e.g. bogus "local-codex").
        profiles_cfg
            .profiles
            .retain(|_, p| !(p.provider == "openai" && p.base_url.trim().is_empty()));
        // 2) Merge profiles with identical (provider, base_url): keep the 1st (sorted order
        //    → "llamacpp-8001" before "local-llama.cpp"), recover its models, remove.
        let mut ids: Vec<String> = profiles_cfg.profiles.keys().cloned().collect();
        ids.sort();
        let mut seen: std::collections::HashMap<(String, String), String> =
            std::collections::HashMap::new();
        let mut to_remove: Vec<String> = Vec::new();
        for id in ids {
            let (prov, url) = {
                let p = &profiles_cfg.profiles[&id];
                (p.provider.clone(), p.base_url.clone())
            };
            if url.trim().is_empty() {
                continue;
            }
            if let Some(keep) = seen.get(&(prov.clone(), url.clone())).cloned() {
                let models = profiles_cfg.profiles[&id].models.clone();
                if let Some(kp) = profiles_cfg.profiles.get_mut(&keep) {
                    for m in models {
                        if !kp.models.contains(&m) {
                            kp.models.push(m);
                        }
                    }
                }
                to_remove.push(id);
            } else {
                seen.insert((prov, url), id);
            }
        }
        for id in &to_remove {
            profiles_cfg.profiles.remove(id);
        }
        // 3) Repair active_model if its profile was removed.
        if !profiles_cfg
            .profiles
            .contains_key(&profiles_cfg.active_model.profile_id)
        {
            let m = profiles_cfg.active_model.model.clone();
            let found = profiles_cfg
                .profiles
                .iter()
                .find(|(_, p)| p.models.contains(&m))
                .map(|(id, _)| id.clone());
            if let Some(id) = found {
                profiles_cfg.active_model.profile_id = id;
            } else {
                let fallback = profiles_cfg
                    .profiles
                    .iter()
                    .find(|(_, p)| !p.models.is_empty())
                    .map(|(id, p)| (id.clone(), p.models[0].clone()));
                if let Some((id, model)) = fallback {
                    profiles_cfg.active_model = profiles::ActiveModel {
                        profile_id: id,
                        model,
                    };
                }
            }
        }
        if !to_remove.is_empty() {
            tracing::info!(
                removed = to_remove.len(),
                "Duplicate profiles cleaned up at startup"
            );
        }
    }

    let _ = profiles::save_profiles(&profiles_path, &profiles_cfg);

    // Derive EssaimConfig from active profile
    let (prof_provider, prof_model, prof_api_key, prof_api_base, prof_ollama_url, prof_max_context_len) =
        profiles::active_to_essaim_fields(&profiles_cfg);

    let cron_arc = Arc::new(RwLock::new(CronScheduler::new(std::path::Path::new(
        "cron-tasks.json",
    ))));
    let watchers_arc = Arc::new(RwLock::new(laruche_watchers::WatchersRegistry::new(
        std::path::Path::new("watchers.json"),
    )));
    let kanban_arc = Arc::new(RwLock::new(laruche_kanban::KanbanBoard::new(
        std::path::Path::new("kanban.json"),
    )));
    // Initialize Essaim (agent engine)
    let essaim_registry = Arc::new(AbeilleRegistry::new());
    enregistrer_abeilles_builtin(&essaim_registry);
    // Wire the mesh signer: the inference path (laruche-essaim) signs its calls to a LAN peer
    // with this node's ed25519 identity → the peer can apply `restricted`.
    laruche_essaim::providers::set_mesh_signer(std::sync::Arc::new(|path: &str| {
        sync::sign_headers(path)
    }));
    // Mission store, Arc-shared with the mission_* abeilles and AppState.
    let missions_arc = Arc::new(RwLock::new(missions::MissionStore::new(
        std::path::Path::new("missions.json"),
    )));
    essaim_registry.enregistrer(Box::new(abeilles_local::AbeilleCronCreate {
        cron_store: cron_arc.clone(),
    }));
    essaim_registry.enregistrer(Box::new(abeilles_local::AbeilleCronList {
        cron_store: cron_arc.clone(),
        missions: missions_arc.clone(),
    }));
    essaim_registry.enregistrer(Box::new(abeilles_local::AbeilleCronDelete {
        cron_store: cron_arc.clone(),
    }));
    essaim_registry.enregistrer(Box::new(abeilles_local::AbeilleMissionList {
        missions: missions_arc.clone(),
    }));
    essaim_registry.enregistrer(Box::new(abeilles_local::AbeilleMissionCreate {
        missions: missions_arc.clone(),
    }));
    essaim_registry.enregistrer(Box::new(abeilles_local::AbeilleForcerLancement));
    essaim_registry.enregistrer(Box::new(abeilles_local::AbeilleMissionDelete {
        missions: missions_arc.clone(),
    }));
    essaim_registry.enregistrer(Box::new(abeilles_local::AbeilleWatcherCreate {
        watcher_store: watchers_arc.clone(),
    }));
    essaim_registry.enregistrer(Box::new(abeilles_local::AbeilleWatcherList {
        watcher_store: watchers_arc.clone(),
    }));
    essaim_registry.enregistrer(Box::new(abeilles_local::AbeilleWatcherDelete {
        watcher_store: watchers_arc.clone(),
    }));
    essaim_registry.enregistrer(Box::new(abeilles_local::AbeilleKanbanCreate {
        kanban_board: kanban_arc.clone(),
    }));
    essaim_registry.enregistrer(Box::new(abeilles_local::AbeilleKanbanList {
        kanban_board: kanban_arc.clone(),
    }));
    let mut essaim_config = EssaimConfig {
        ollama_url: prof_ollama_url,
        model: prof_model,
        provider: prof_provider,
        api_key: prof_api_key,
        api_base: prof_api_base,
        context_max_tokens: prof_max_context_len,
        disabled_tools: persistent.disabled_tools.clone(),
        disabled_skills: persistent.disabled_skills.clone(),
        ..EssaimConfig::default()
    };
    if let Some(max) = persistent.context_max_messages {
        essaim_config.context_max_messages = max;
    }
    if let Some(tok) = persistent.context_max_tokens {
        essaim_config.context_max_tokens = tok;
    }
    if let Some(th) = persistent.compaction_threshold {
        essaim_config.compaction_threshold = th;
    }
    if let Some(j) = persistent.episodes_retention_jours {
        essaim_config.episodes_retention_jours = j;
    }
    if let Some(h) = persistent.halo_actif {
        essaim_config.halo_actif = h;
    }
    // L'interrupteur vivant que les outils consultent a chaque geste.
    laruche_essaim::config::definir_halo(essaim_config.halo_actif);
    // Le budget de lecture suit la fenetre du modele, par la meme regle que le
    // plafond des observations: un file_read plus genereux serait rabote en aval
    // si les deux divergeaient, et personne ne comprendrait pourquoi.
    laruche_essaim::config::definir_budget_lecture(laruche_essaim::config::plafond_observation(
        (essaim_config.context_max_tokens as usize).max(8_000),
    ));
    // Le bureau de l'agent, cree et annonce avant le premier outil.
    //
    // Sans lui, le repli d'un outil est `current_dir()`, c'est-a-dire le FOYER:
    // scripts, tests et dossiers d'eclaireuse atterrissaient a cote de `memoire.db`,
    // de `sessions/` et de `skills/`. Le foyer garde ses dossiers structures, chacun
    // alimente par un outil dedie (`skill_create`, `plugin_create`, `mcp_add`);
    // `travail/` est la piece qui manquait, celle du brouillon.
    let bureau = local_api::dossier_brouillon();
    if let Err(e) = std::fs::create_dir_all(&bureau) {
        tracing::warn!(error = %e, chemin = %bureau.display(), "bureau de l'agent non cree");
    }
    // On NE pose PAS ce dossier comme repertoire des outils: le repli reste le
    // foyer, sans quoi les skills ne trouveraient plus leurs scripts. Le brouillon
    // sert de destination annoncee, pas de racine d execution.
    laruche_essaim::config::definir_dossier_brouillon(bureau);
    if let Some(c) = persistent.curateur_actif {
        essaim_config.curateur_actif = c;
    }
    // Written on every save but never read back, so the MCP switch and its firewall reset
    // to off at each restart and the user's choice vanished without a word. Failing closed
    // is the right default; forgetting a setting is not the same thing.
    if let Some(v) = persistent.mcp_server_actif {
        essaim_config.mcp_server_actif = v;
    }
    if let Some(v) = persistent.mcp_pare_feu_actif {
        essaim_config.mcp_pare_feu_actif = v;
    }
    if let Some(v) = persistent.mcp_ip_autorisees.clone() {
        essaim_config.mcp_ip_autorisees = v;
    }
    if let Some(v) = persistent.reactions_agent {
        essaim_config.reactions_agent = v;
    }
    if let Some(d) = persistent.dynamic_tool_selection {
        essaim_config.dynamic_tool_selection = d;
    }
    if let Some(v) = persistent.max_iterations {
        essaim_config.max_iterations = v;
    }
    if let Some(v) = persistent.temperature {
        essaim_config.temperature = v;
    }
    if let Some(v) = persistent.max_tokens {
        essaim_config.max_tokens = v;
    }
    if let Some(v) = persistent.tool_selection_limit {
        essaim_config.tool_selection_limit = v;
    }
    if let Some(v) = persistent.dynamic_context_threshold {
        essaim_config.dynamic_context_threshold = v;
    }
    if let Some(ref models) = persistent.fallback_models {
        essaim_config.fallback_models = models.clone();
    }
    if persistent.review_model.is_some() {
        essaim_config.review_model = persistent.review_model.clone();
    }
    if persistent.home_channel.is_some() {
        essaim_config.home_channel = persistent.home_channel.clone();
    }
    if let Some(ref m) = persistent.permission_mode {
        if let Some(mode) = settings_api::permission_mode_from_str(m) {
            essaim_config.permission_mode = mode;
        }
    }

    // Scout toolset: the reduced registry handed to delegated sub-agents (builtins
    // only, no delegate = no recursive fan-out). tool_call / tool_search / run_script
    // get the LIVE main registry instead, so they can reach and discover every tool
    // registered later (crons, watchers, memory, plugins, background-loaded MCP).
    let sub_registry = Arc::new({
        let r = AbeilleRegistry::new();
        enregistrer_abeilles_builtin(&r);
        r
    });
    enregistrer_delegation(
        &essaim_registry,
        essaim_registry.clone(),
        sub_registry,
        essaim_config.clone(),
    );

    // Cognitive memory (laruche-memoire): env-selectable backend.
    //   LARUCHE_MEMOIRE_BACKEND=sidecar         → real paradigm on http://127.0.0.1:8765
    //   LARUCHE_MEMOIRE_BACKEND=memory|native   → in-memory, volatile (tests, demos)
    //   (default)                                → SQLite, persistent
    //
    // Le defaut etait la memoire vive. Conséquence: tout lancement qui ne posait pas
    // LARUCHE_MEMOIRE_BACKEND=sqlite - double-clic sur l'exe, service, application de
    // bureau - donnait une LaRuche qui confirme « c'est memorise » puis oublie tout en
    // s'arretant, sans le moindre avertissement. Seul lancer_butinage.bat posait la
    // variable, ce qui faisait dependre la persistance du chemin emprunte pour demarrer.
    // SqliteBackend est compile dans le binaire de toute facon: le defaut ne coute rien
    // et correspond a ce que tout le monde attend. Le mode volatile reste accessible,
    // mais il faut desormais le demander.
    let memoire: Arc<dyn laruche_memoire::MemoireCognitive> =
        match std::env::var("LARUCHE_MEMOIRE_BACKEND").as_deref() {
            Ok("sidecar") => Arc::new(laruche_memoire::SidecarBackend::loopback()),
            Ok("memory") | Ok("native") | Ok("inmemory") => {
                warn!("memory backend: IN-MEMORY (volatile) - nothing will be persisted");
                Arc::new(laruche_memoire::NativeBackend::new())
            }
            _ => {
                // Embedder ALWAYS wired (semantic recall by default): LARUCHE_EMBED_URL
                // (Ollama `/api/embed` OR llama.cpp/OpenAI-compat `/v1/embeddings` -
                // format auto-detected), falling back to the local Ollama default.
                // HttpEmbedder opens a circuit breaker when the server is down, so a
                // missing embedder costs ~nothing and recall degrades to FTS5.
                let url = std::env::var("LARUCHE_EMBED_URL")
                    .ok()
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| "http://127.0.0.1:11434".to_string());
                let model = std::env::var("LARUCHE_EMBED_MODEL")
                    .unwrap_or_else(|_| "nomic-embed-text".to_string());
                info!(url = %url, model = %model, "memory embedder: semantic recall active (auto-detected format)");
                Arc::new(
                    laruche_memoire::SqliteBackend::open_with_embedder(
                        "memoire.db",
                        Arc::new(laruche_memoire::HttpEmbedder::new(url, model)),
                    )
                    .expect("opening memoire.db (SQLite+FTS5+embeddings)"),
                )
            }
        };
    // Backfill: items written while the embedder was down get their embeddings
    // (semantic recall would otherwise never see them). Deferred a little so the
    // local embed server has time to come up; the breaker makes failures cheap.
    {
        let mem_bf = memoire.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(20)).await;
            if let Ok(n) = mem_bf.backfill_embeddings(500).await {
                if n > 0 {
                    info!(items = n, "memory: missing embeddings backfilled");
                }
            }
        });
    }
    // Write-time contradiction arbiter (aux LLM): resolves near-miss updates cosine
    // cannot (e.g. "4070 Ti" -> "5080" ~0.71). No-op on backends without arbiter support;
    // any LLM failure keeps both facts (never destructive). Opt-out via LARUCHE_MEMOIRE_ARBITRE=0.
    if std::env::var("LARUCHE_MEMOIRE_ARBITRE").as_deref() != Ok("0") {
        memoire.definir_arbitre(std::sync::Arc::new(
            arbitre_memoire::ArbitreLLM::depuis_config(&essaim_config),
        ));
    }
    laruche_essaim::abeilles::enregistrer_memoire(&essaim_registry, memoire.clone());
    // LLM consolidation (item merging): requires memory + config (aux model).
    essaim_registry.enregistrer(Box::new(
        laruche_essaim::abeilles::memoire::MemoireConsolidate {
            mem: memoire.clone(),
            config: essaim_config.clone(),
        },
    ));

    // Load dynamic plugins from plugins/ directory
    charger_plugins(std::path::Path::new("plugins"), &essaim_registry);
    essaim_registry.enregistrer(Box::new(
        laruche_essaim::abeilles::reload_plugins::ReloadPluginsTool {
            registry: essaim_registry.clone(),
        },
    ));
    // SELF-IMPROVEMENT tools (forge): skill_file_*, plugin_*, mcp_*. The main registry
    // is passed so plugin_create/delete reload in the right place.
    laruche_essaim::abeilles::enregistrer_forge(&essaim_registry, essaim_registry.clone());
    essaim_registry.enregistrer(Box::new(abeilles_local::AbeilleMeshSend));

    // Boot phase chronometer: each heavy startup step logs its cumulative time,
    // so a slow boot points at its culprit instead of a silent multi-second gap.
    let boot_t0 = std::time::Instant::now();

    // Migration `tools.* → capacities.*` (idempotent, run at every boot but no-op afterwards).
    // The forged skills (real data) are PRESERVED; tools.abeilles (a mere projection)
    // is purged then recreated by the indexer under capacities.tools/plugins/mcp.
    match memoire
        .renommer_sous_arbre("tools.skills", "capacities.skills")
        .await
    {
        Ok(n) if n > 0 => tracing::info!(noeuds = n, "migration skills -> capacities.skills"),
        Ok(_) => {}
        Err(e) => tracing::warn!(error = %e, "skills migration skipped (backend without support)"),
    }
    let _ = memoire.supprimer_sous_arbre("tools").await; // purge the remaining legacy projection

    // These nodes are CONTAINERS: their children carry the content, they hold none
    // themselves. Two writers had been parking items on them anyway. The curator wrote
    // straight to the store, around the memory_write guard, and left facts about
    // cron_create on the system root; the indexer wrote a "capabilities index: N tools"
    // line on capacities.tools at every single startup. Nothing reads either, the
    // interface locks those nodes so they could not even be removed by hand, and a
    // memory search can hand them back as if they were souvenirs.
    //
    // Both sources are closed now, `node_id_valide` for the curator and a log line for
    // the indexer, so this sweep is here for the databases that already carry them.
    for contenant in [
        "system",
        "capacities",
        "capacities.tools",
        "capacities.plugins",
        "capacities.mcp",
        "capacities.skills",
    ] {
        let Ok(node) = memoire.read_node(contenant).await else {
            continue;
        };
        let ids: Vec<String> = node["items"]
            .as_array()
            .map(|a| a.as_slice())
            .unwrap_or(&[])
            .iter()
            .filter_map(|it| it["id"].as_str().map(str::to_string))
            .collect();
        for id in &ids {
            let _ = memoire
                .delete_item(id, Some("container node holds no items"))
                .await;
        }
        if !ids.is_empty() {
            info!(node = contenant, items = ids.len(), "boot: swept items parked on a container node");
        }
    }
    info!(t_ms = boot_t0.elapsed().as_millis() as u64, "boot: legacy tools migration done");

    // Map nodes (virtual .md files). Created empty if absent (idempotent).
    // `capacities.*` = tool ecosystem (protected); `system.*` = editable prompt/SOUL base.
    for (id, label, desc) in [
        (
            "capacities",
            "Capacities",
            "Ecosystem: tools, plugins, MCP, skills",
        ),
        ("capacities.tools", "Tools", "Native tools (builtin)"),
        (
            "capacities.plugins",
            "Plugins",
            "Custom tools (JSON plugins)",
        ),
        (
            "capacities.mcp",
            "MCP",
            "Tools served by MCP servers",
        ),
        ("capacities.skills", "Skills", "Learned OKF procedures"),
        (
            "system",
            "System",
            "Editable sections of the system prompt (hot-reload, no restart)",
        ),
        (
            "system.prompt",
            "Identity",
            "Editable identity / persona (empty = code default)",
        ),
        (
            // Seeded like its siblings. It used to be created lazily, on the first save
            // from the profile form, so the tree showed every system node EXCEPT the one
            // about the person using it: nothing hinted that a profile could be written,
            // and the node only appeared once you had already found the form.
            "system.user",
            "User",
            "Who you are: what LaRuche should know about you (written from Profile)",
        ),
        (
            "system.behavior",
            "Behavior",
            "Editable behavior rules (empty = code default)",
        ),
        (
            "system.soul",
            "SOUL",
            "Injectable personalization layer (frontmatter enabled)",
        ),
        (
            "system.prompt_curateur",
            "Curateur Prompt",
            "Self-improvement curateur prompt (empty = code default, hot-reload)",
        ),
        (
            "system.prompt_extraction",
            "Consolidation Prompt",
            "Memory / escale consolidation prompt (empty = code default, hot-reload)",
        ),
        (
            "system.prompt_planning",
            "Planning Prompt",
            "Planning section of the system prompt (empty = code default, hot-reload)",
        ),
        (
            "system.prompt_reine",
            "LaReine Prompt",
            "LaReine supervisor rubric (empty = code default, hot-reload)",
        ),
        (
            "system.constitution",
            "Constitution",
            "Shared rules for every Table Ronde specialist (empty = code baseline, hot-reload)",
        ),
    ] {
        let _ = memoire
            .create_node(id, label, Some(desc), Some(1.0), None)
            .await;
    }

    // No web-research skill is seeded here any more. It lives on disk as
    // `skills/web-research/`, synced by `sync_skills_disk_to_sql`. Seeding a second
    // copy under `web_research` put TWO overlapping entries in the prompt catalog,
    // which is exactly what makes a model hesitate and pick the wrong one.

    info!(t_ms = boot_t0.elapsed().as_millis() as u64, "boot: map nodes + seed done");

    // Index the tool registry into the map (capacities.*) RIGHT FROM startup, incrementally,
    // so any new tool is visible in memory and semantically retrievable.
    // (MCP tools, loaded below, are indexed on the 1st chat turn via the same call.)
    if let Err(e) =
        laruche_essaim::brain::indexer_abeilles_memoire(&essaim_registry, &memoire).await
    {
        tracing::warn!(error = %e, "tool indexing at startup skipped");
    }
    info!(t_ms = boot_t0.elapsed().as_millis() as u64, "boot: tool indexing done");

    // Phase 1: flat-file layer: disk → SQL sync of skills (skills/<slug>/SKILL.md),
    // in the BACKGROUND. Unchanged files are no-ops (incremental), but a real
    // resync re-embeds and can consult the write arbiter (aux LLM): with the LLM
    // busy that took minutes and must never delay the HTTP bind.
    {
        let mem_sync = memoire.clone();
        let t0 = boot_t0;
        tokio::spawn(async move {
            changes_api::sync_skills_disk_to_sql(&mem_sync).await;
            info!(t_ms = t0.elapsed().as_millis() as u64, "boot: skills disk->SQL sync done (background)");
        });
    }

    // Load MCP servers in the BACKGROUND: a slow server must not delay the HTTP
    // bind (the computer-use python server takes 8s+ of imports before answering
    // the handshake; a mute one costs the full 60s request timeout). The registry
    // has interior mutability, so MCP tools and the resource abeilles register
    // themselves as soon as each server is ready.
    {
        let registry_mcp = essaim_registry.clone();
        let memoire_mcp = memoire.clone();
        tokio::spawn(async move {
            let (count, mcp_clients) =
                charger_mcp_servers(std::path::Path::new("mcp_servers.json"), &registry_mcp)
                    .await;
            let mcp_clients = Arc::new(mcp_clients);
            registry_mcp.enregistrer(Box::new(
                laruche_essaim::abeilles::mcp_resources::McpListResources {
                    clients: mcp_clients.clone(),
                },
            ));
            registry_mcp.enregistrer(Box::new(
                laruche_essaim::abeilles::mcp_resources::McpReadResource {
                    clients: mcp_clients.clone(),
                },
            ));
            if count > 0 {
                info!(tools = count, "MCP servers ready (background load)");
            } else {
                info!("MCP load finished with no tool");
            }
            // Index them NOW. The startup pass above ran before this load, so the
            // mcp family was empty, and the only other trigger was the first chat
            // turn: no conversation after a boot meant capacities.mcp stayed empty
            // while the server was running and its tools callable.
            //
            // Et on passe AUSSI quand le compte est nul. C'etait la condition de
            // cet appel, donc un serveur devenu injoignable laissait ses outils
            // dans l'arbre de la memoire pour toujours: personne ne venait plus
            // constater leur absence.
            if let Err(e) = laruche_essaim::brain::indexer_abeilles_memoire_ex(
                &registry_mcp,
                &memoire_mcp,
                true,
            )
            .await
            {
                tracing::warn!(error = %e, "MCP tool indexing skipped");
            }
        });
    }

    // Initialize RAG knowledge base
    let kb = Arc::new(tokio::sync::RwLock::new(
        laruche_essaim::rag::KnowledgeBase::new(
            std::path::Path::new("knowledge-base.json"),
            &config.ollama_url,
            "nomic-embed-text", // Default embedding model: user should pull it
        ),
    ));
    // Fix A: knowledge_add/knowledge_search REMOVED: it was a 2nd memory system
    // (flat KnowledgeBase/RAG) DUPLICATING the cognitive map. Everything now goes through
    // memory_write / memory_search (the cognitive memory = LaRuche's differentiator).
    let _ = &kb; // kb kept for rag.rs (legacy RAG), but no longer exposed as an agent tool.

    // Load existing sessions from disk
    let mut loaded_sessions: HashMap<Uuid, Session> = HashMap::new();
    let sessions_dir = std::path::Path::new("sessions");
    if sessions_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(sessions_dir) {
            for entry in entries.flatten() {
                if entry.path().extension().is_some_and(|e| e == "json") {
                    match Session::charger(&entry.path()) {
                        Ok(session) => {
                            tracing::debug!(session_id = %session.id, title = ?session.title, "Loaded session");
                            loaded_sessions.insert(session.id, session);
                        }
                        Err(e) => {
                            warn!(path = %entry.path().display(), error = %e, "Failed to load session");
                        }
                    }
                }
            }
        }
    }
    info!(count = loaded_sessions.len(), "Sessions loaded from disk");

    let sessions_arc = Arc::new(RwLock::new(loaded_sessions));
    essaim_registry.enregistrer(Box::new(abeilles_local::AbeilleSessionSearch {
        sessions_store: sessions_arc.clone(),
    }));

    // Load users from disk
    let users_dir = std::path::Path::new("users");
    let mut loaded_users = auth_user::load_all_users(users_dir);
    let deduped = auth_user::dedupe_users(&mut loaded_users, users_dir);
    if deduped > 0 {
        info!(removed = deduped, "Deduplicated legacy duplicate accounts (old enroll bug)");
    }
    if !loaded_users.is_empty() {
        info!(count = loaded_users.len(), "Users loaded from disk");
    }

    // Load or generate cookie secret (persisted in laruche-state.json).
    // The fingerprint log is the auth debugging anchor: if it CHANGES between two
    // boots, every session cookie is invalidated (that is the "re-login every
    // launch" symptom) and the state file is not persisting/loading correctly.
    let cookie_secret = if let Some(ref hex) = persistent.cookie_secret {
        match auth_user::cookie_secret_from_base64(hex) {
            Some(s) => {
                info!(
                    fingerprint = %&auth_user::cookie_secret_to_base64(&s)[..8],
                    "Cookie secret loaded from laruche-state.json (sessions survive restarts)"
                );
                s
            }
            None => {
                let s = auth_user::generate_cookie_secret();
                warn!(
                    stored_len = hex.len(),
                    fingerprint = %&auth_user::cookie_secret_to_base64(&s)[..8],
                    "Stored cookie secret INVALID -> regenerated: every session cookie is now invalid (re-login required)"
                );
                s
            }
        }
    } else {
        let s = auth_user::generate_cookie_secret();
        warn!(
            fingerprint = %&auth_user::cookie_secret_to_base64(&s)[..8],
            "No cookie secret in laruche-state.json -> generated: previous sessions are invalid"
        );
        s
    };

    // Load or create CredentialPool
    let credentials_path = std::path::PathBuf::from("credentials.json");
    let pool_data = if credentials_path.exists() {
        std::fs::read_to_string(&credentials_path)
            .ok()
            .and_then(|data| {
                serde_json::from_str::<laruche_essaim::credential_pool::CredentialPool>(&data).ok()
            })
            .unwrap_or_else(laruche_essaim::credential_pool::CredentialPool::default)
    } else {
        laruche_essaim::credential_pool::CredentialPool::default()
    };
    let credential_pool = Arc::new(RwLock::new(pool_data));

    let state = Arc::new(AppState {
        manifest: RwLock::new(manifest),
        auth: RwLock::new(ProximityAuth::new()),
        queue: RwLock::new(RequestQueue::new(QosPolicy::default())),
        listener: RwLock::new(listener),
        default_models: RwLock::new(initial_defaults),
        custom_services: RwLock::new(HashMap::new()),
        capability_selection: RwLock::new(
            persistent.capability_selection.clone().unwrap_or_default(),
        ),
        missions: missions_arc.clone(),
        config: config.clone(),
        sys: RwLock::new(sys),
        activity_log: RwLock::new(initial_log),
        state_file_path,
        metrics_history: RwLock::new(VecDeque::with_capacity(METRICS_HISTORY_LIMIT)),
        node_events: RwLock::new(VecDeque::with_capacity(NODE_EVENTS_LIMIT)),
        known_node_ids: RwLock::new(HashSet::new()),
        essaim_registry: essaim_registry.clone(),
        essaim_config: RwLock::new({
            essaim_config.credential_pool = Some(credential_pool.clone());
            essaim_config
        }),
        memoire,
        essaim_sessions: sessions_arc.clone(),
        active_context_stats: Arc::new(RwLock::new(HashMap::new())),
        runs_actifs: Arc::new(RwLock::new(HashMap::new())),
        dossier_travail: Arc::new(RwLock::new(local_api::dossier_travail_defaut())),
        theme_actif: Arc::new(RwLock::new(themes_api::theme_actif_au_demarrage())),
        bind_lan: Arc::new(RwLock::new(persistent.bind_lan.unwrap_or(false))),
        essaim_cron: cron_arc.clone(),
        watchers: watchers_arc.clone(),
        kanban_board: kanban_arc.clone(),
        essaim_kb: kb.clone(),
        events: Arc::new(RwLock::new(laruche_events::EventBus::new())),
        channel_handles: RwLock::new(HashMap::new()),
        profiles: RwLock::new(profiles_cfg),
        profiles_path,
        users: RwLock::new(loaded_users),
        auth_challenges: RwLock::new(HashMap::new()),
        cookie_secret,
        credential_pool: credential_pool.clone(),
        credentials_path,
        last_activity: RwLock::new(std::time::Instant::now()),
        travaux: Arc::new(std::sync::RwLock::new(HashMap::new())),
        mcp_verrou: Arc::new(std::sync::Mutex::new(Default::default())),
    });

    // Published once, right after construction. The tool registry is built long before
    // AppState exists (line ~467), so a tool that needs the whole node - `run_now` has to
    // reach the cron store, the mission store and the agent loop - cannot hold it as a
    // field. One Arc for the process lifetime; nothing to reclaim on a daemon that exits
    // with the process.
    let _ = crate::abeilles_local::ETAT_NOEUD.set(state.clone());

    // Persist the state RIGHT AWAY: the shutdown save only runs on a clean exit
    // (Ctrl+C / tray Quit). Closing the console window kills the process without
    // saving - a cookie secret generated this boot would then never be written,
    // and the NEXT boot would regenerate it, invalidating every session cookie
    // ("re-login on every launch"). Saving here makes the secret durable no
    // matter how the process dies.
    save_persistent_state(&state).await;

    // Mirror the saved LaReine gate into the process-global at boot, so self-created
    // skills are held for approval even before the first chat turn (cron/curateur).
    laruche_essaim::reine_queue::definir_gate(reine_api::charger_reine_settings().queue_gate);

    let app = router::build_router(state.clone());

    // Background jobs (metrics, schedulers, dispatchers): see background.rs.
    background::spawn_metrics_refresh(&state, &broadcaster);

    background::spawn_auth_challenge_cleanup(&state);

    // Boot resume: purge stale butinage notebooks (crashed/abandoned missions)
    // and log the still-recent ones (potentially resumable). Successful missions already
    // deleted their notebook (see butinage_pont::executer).
    purger_carnets_au_boot();

    background::spawn_periodic_dream(&state);
    episodes_api::spawn_balayage_episodes(&state);
    background::spawn_purge_corbeille(&state);

    background::spawn_ollama_heartbeat(&state);

    background::spawn_cron_checker(&state);

    background::spawn_watchers_checker(&state);

    background::spawn_mdns_reannounce(&state, &broadcaster);

    background::spawn_missions_tick(&state);

    background::spawn_kanban_dispatcher(&state);
    background::spawn_kanban_todo_sweeper(&state);

    background::spawn_idle_dream(&state);

    okf_git::spawn_okf_git(&state);

    // Graceful shutdown: save state on Ctrl+C
    let shutdown_state = state.clone();
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            info!("Shutting down: saving persistent state...");
            save_persistent_state(&shutdown_state).await;
            std::process::exit(0);
        }
    });

    // Bind to loopback ONLY by default (single-user local app). Serving on the whole
    // network is an explicit opt-in (LARUCHE_BIND_LAN=1) since not every route requires
    // the session cookie; the choice is loudly logged so it is never accidental.
    // La variable d'environnement d'abord, le reglage ensuite. L'ordre compte: un
    // lancement en ligne de commande, ou par un .bat, doit pouvoir imposer son choix
    // sans que l'interface le contredise en silence. Mais quand la variable est
    // absente, ce que l'utilisateur a coche fait foi, plutot que de l'obliger a
    // editer un fichier pour ouvrir sa ruche a son telephone.
    let bind_lan = match std::env::var("LARUCHE_BIND_LAN").as_deref() {
        Ok("1") => true,
        Ok("0") => false,
        _ => persistent.bind_lan.unwrap_or(false),
    };
    let bind_ip = if bind_lan { "0.0.0.0" } else { "127.0.0.1" };
    let addr = format!("{bind_ip}:{}", config.api_port);
    let scheme = if std::env::var("LARUCHE_HTTPS").as_deref() == Ok("1")
        || std::env::var("LARUCHE_TLS_CERT").map(|s| !s.is_empty()).unwrap_or(false)
    {
        "https"
    } else {
        "http"
    };
    if bind_lan {
        warn!(
            "LARUCHE_BIND_LAN=1: API exposed on the whole LAN ({bind_ip}:{}). Ensure auth \
             is configured; anyone on the network can reach it.",
            config.api_port
        );
    } else {
        // Le noeud s'annonce en mDNS avec son adresse LAN, mais n'ecoute que sur la
        // boucle locale: il est donc VISIBLE sans etre JOIGNABLE. Le mDNS est du
        // multicast et traverse quand meme, alors que le message de test, la liste des
        // pairs et l'appel d'un modele partage sont du HTTP vers l'adresse annoncee.
        //
        // C'etait silencieux, et c'est la premiere cause de « l'autre ruche me voit
        // mais rien ne repond ». On le dit fort, une fois, au demarrage.
        warn!(
            "Ruche visible sur le reseau mais INJOIGNABLE: l'API n'ecoute que sur \
             127.0.0.1:{}. Les autres ruches vous verront en mDNS et n'obtiendront \
             aucune reponse. Pour un essaim, demarrer avec LARUCHE_BIND_LAN=1.",
            config.api_port
        );
    }
    info!("LaRuche ready → {scheme}://localhost:{}", config.api_port);
    accueil_demarrage(scheme, config.api_port);

    // Sync essaim config from active profile at startup
    profiles_api::sync_essaim_from_profiles(&state).await;

    background::spawn_mesh_memory_sync(&state);

    background::spawn_skill_file_watcher(&state);

    background::spawn_event_notifier(&state);

    info!("Starting MCP servers if configured...");
    // Auto-start channels if configured
    background::autostart_channels(&state).await;

    // rustls 0.23 requires a process-wide crypto provider before any TLS server starts;
    // with both ring and aws-lc-rs in the tree it cannot auto-pick, so install ring here.
    // Without this, bind_rustls panics inside its spawned task and HTTPS silently never
    // starts (the app keeps serving plain HTTP).
    let _ = rustls::crypto::ring::default_provider().install_default();

    // TLS: explicit LARUCHE_TLS_CERT/KEY win. Otherwise LARUCHE_HTTPS=1 auto-generates a
    // self-signed cert (localhost + 127.0.0.1 + LAN IP), so the browser microphone works
    // from other devices on the network (a secure context), not just localhost.
    let (tls_cert, tls_key) = {
        let cert = std::env::var("LARUCHE_TLS_CERT").ok().filter(|s| !s.is_empty());
        let key = std::env::var("LARUCHE_TLS_KEY").ok().filter(|s| !s.is_empty());
        if cert.is_some() && key.is_some() {
            (cert, key)
        } else if std::env::var("LARUCHE_HTTPS").as_deref() == Ok("1") {
            match ensure_self_signed_cert() {
                Some((c, k)) => (Some(c), Some(k)),
                None => {
                    error!("LARUCHE_HTTPS=1 but self-signed cert generation failed; serving HTTP");
                    (None, None)
                }
            }
        } else {
            (None, None)
        }
    };

    if use_tui {
        // Spawn server in background, run TUI in foreground
        let tui_state = state.clone();
        tokio::spawn(async move {
            serve_with_optional_tls(app, addr, tls_cert.zip(tls_key)).await;
        });

        // Run TUI (blocks until user presses 'q')
        if let Some(rx) = tui_log_rx {
            tui::run_tui(tui_state.clone(), rx).await?;
        }

        // TUI exited: save state and shutdown
        save_persistent_state(&tui_state).await;
    } else {
        // --no-tui mode: spawn server + system tray (Windows)
        let (tray_shutdown_tx, tray_shutdown_rx) = tokio::sync::oneshot::channel::<()>();

        // Spawn systray on a dedicated OS thread (requires win32 message pump)
        let tray_port = config.api_port;
        std::thread::spawn(move || {
            systray::run_systray(tray_port, tray_shutdown_tx);
        });

        // Spawn HTTP server
        tokio::spawn(async move {
            serve_with_optional_tls(app, addr, tls_cert.zip(tls_key)).await;
        });

        // Wait for either Ctrl+C or tray "Quit"
        let save_state = state.clone();
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                info!("Ctrl+C received: shutting down...");
            }
            _ = tray_shutdown_rx => {
                info!("Quit from system tray: shutting down...");
            }
        }
        save_persistent_state(&save_state).await;
    }

    Ok(())
}

fn parse_tier(value: &str) -> Option<HardwareTier> {
    match value.to_ascii_lowercase().as_str() {
        "nano" => Some(HardwareTier::Nano),
        "core" => Some(HardwareTier::Core),
        "pro" => Some(HardwareTier::Pro),
        "max" => Some(HardwareTier::Max),
        _ => None,
    }
}

fn parse_env_capabilities(default_model: &str) -> Option<Vec<CapabilityConfig>> {
    let cap1 = std::env::var("LARUCHE_CAP").ok()?;
    let model1 = std::env::var("LARUCHE_MODEL").unwrap_or_else(|_| default_model.to_string());

    let mut caps = vec![CapabilityConfig {
        capability: cap1,
        model_name: model1,
        model_size: None,
        quantization: None,
    }];

    if let Ok(cap2) = std::env::var("LARUCHE_CAP2") {
        let model2 = std::env::var("LARUCHE_MODEL2").unwrap_or_else(|_| default_model.to_string());
        caps.push(CapabilityConfig {
            capability: cap2,
            model_name: model2,
            model_size: None,
            quantization: None,
        });
    }

    Some(caps)
}

/// At startup: purges stale butinage notebooks (checkpoints of crashed/abandoned
/// missions, > 3 days) and logs the still-recent ones (potentially resumable).
/// Successful missions already delete their notebook at the end.
fn purger_carnets_au_boot() {
    let dir = std::path::Path::new("sessions").join("butinage");
    let entries = match std::fs::read_dir(&dir) {
        Ok(e) => e,
        Err(_) => return, // no folder = nothing to do
    };
    let max_age = std::time::Duration::from_secs(3 * 24 * 3600); // 3 days
    let now = std::time::SystemTime::now();
    let (mut purges, mut repris) = (0u32, 0u32);
    for e in entries.flatten() {
        let p = e.path();
        if p.extension().and_then(|x| x.to_str()) != Some("json") {
            continue;
        }
        let age = e
            .metadata()
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| now.duration_since(t).ok())
            .unwrap_or_default();
        if age > max_age {
            if std::fs::remove_file(&p).is_ok() {
                purges += 1;
            }
        } else {
            repris += 1;
        }
    }
    if purges > 0 || repris > 0 {
        info!(purges, repris, "Butinage notebooks: cleanup at startup");
    }
}

fn load_config() -> Result<NodeConfig> {
    let config_path = std::env::var("LARUCHE_CONFIG").unwrap_or_else(|_| "laruche.toml".into());
    let mut config = NodeConfig::default();

    if std::path::Path::new(&config_path).exists() {
        let raw = fs::read_to_string(&config_path)?;
        let file_cfg: NodeConfigFile = toml::from_str(&raw)?;

        if let Some(v) = file_cfg.node_name {
            config.node_name = v;
        }
        if let Some(v) = file_cfg.tier {
            config.tier = v;
        }
        if let Some(v) = file_cfg.ollama_url {
            config.ollama_url = v;
        }
        if let Some(v) = file_cfg.default_model {
            config.default_model = v;
        }
        if let Some(v) = file_cfg.api_port {
            config.api_port = v;
        }
        if let Some(v) = file_cfg.dashboard_port {
            config.dashboard_port = v;
        }
        if let Some(v) = file_cfg.capabilities {
            config.capabilities = v;
        }
        if let Some(v) = file_cfg.provider {
            config.provider = v;
        }
        if let Some(v) = file_cfg.api_key {
            config.api_key = v;
        }
        if let Some(v) = file_cfg.api_base {
            config.api_base = Some(v);
        }

        info!(path = %config_path, "Loaded config file");
    }

    // Environment variables override config file values (with warnings)
    if let Ok(v) = std::env::var("LARUCHE_NAME") {
        info!(env = "LARUCHE_NAME", value = %v, "Env override: node_name");
        config.node_name = v;
    }
    if let Ok(v) = std::env::var("LARUCHE_TIER") {
        if let Some(tier) = parse_tier(&v) {
            info!(env = "LARUCHE_TIER", value = %v, "Env override: tier");
            config.tier = tier;
        }
    }
    if let Ok(v) = std::env::var("OLLAMA_URL") {
        info!(env = "OLLAMA_URL", value = %v, "Env override: ollama_url");
        config.ollama_url = v;
    }
    if let Ok(v) = std::env::var("LARUCHE_MODEL") {
        info!(env = "LARUCHE_MODEL", value = %v, "Env override: default_model");
        config.default_model = v;
    }
    if let Ok(v) = std::env::var("LARUCHE_PORT") {
        if let Ok(port) = v.parse::<u16>() {
            info!(env = "LARUCHE_PORT", value = %v, "Env override: api_port");
            config.api_port = port;
        }
    }
    if let Ok(v) = std::env::var("LARUCHE_DASH_PORT") {
        if let Ok(port) = v.parse::<u16>() {
            info!(env = "LARUCHE_DASH_PORT", value = %v, "Env override: dashboard_port");
            config.dashboard_port = port;
        }
    }

    if let Ok(v) = std::env::var("LARUCHE_PROVIDER") {
        info!(env = "LARUCHE_PROVIDER", value = %v, "Env override: provider");
        config.provider = v;
    }
    if let Ok(v) = std::env::var("LARUCHE_API_KEY") {
        info!(env = "LARUCHE_API_KEY", "Env override: api_key (redacted)");
        config.api_key = v;
    }
    if let Ok(v) = std::env::var("LARUCHE_API_BASE") {
        info!(env = "LARUCHE_API_BASE", value = %v, "Env override: api_base");
        config.api_base = Some(v);
    }

    if let Some(caps) = parse_env_capabilities(&config.default_model) {
        info!("Env override: capabilities from LARUCHE_CAP/LARUCHE_MODEL");
        config.capabilities = caps;
    }

    if config.capabilities.is_empty() {
        config.capabilities = vec![CapabilityConfig {
            capability: "llm".into(),
            model_name: config.default_model.clone(),
            model_size: Some("7B".into()),
            quantization: Some("Q4_K_M".into()),
        }];
    }

    for cap in &mut config.capabilities {
        cap.capability = normalize_capability_label(&cap.capability);
    }

    Ok(config)
}
