//! `computer`: la souris, le clavier et l'ecran, en natif.
//!
//! Remplace le serveur MCP Python (`mcp/computer_use.py`, pyautogui). Ce n'est
//! pas qu'une question d'elegance:
//!
//!   - pyautogui clique en coordonnees LOGIQUES. Sur un ecran a 150%, ou sur un
//!     montage 4K plus 1080p, les clics partent a cote. C'est un bug qu'on ne
//!     diagnostique jamais depuis le modele, qui croit avoir mal vise. Ici la
//!     conversion est explicite et calibree (voir [`facteur_enigo`]).
//!   - il coute 8s d'imports Python au demarrage du noeud, mesures.
//!   - il ne sait rien du multi-ecran.
//!
//! # Le contrat de coordonnees
//!
//! Le modele parle TOUJOURS en pixels de la derniere capture qu'il a recue.
//! Jamais en pixels systeme. L'outil garde le cadre de cette capture
//! ([`Cadre`]) et fait seul les deux conversions: pixels image vers pixels
//! physiques du bureau virtuel, puis pixels physiques vers l'espace de
//! coordonnees d'enigo, qui n'est pas le meme selon l'OS et selon que le
//! processus est conscient du DPI ou non.
//!
//! C'est la transposition directe des `ref_N` du navigateur: le modele ne
//! manipule jamais un systeme de coordonnees qu'il ne voit pas.
//!
//! # Les deux gardes
//!
//! Cet outil contourne toutes les barrieres construites ailleurs, parce qu'un
//! clic peut approuver une popup. Deux gardes lui sont donc propres:
//!
//!   1. il refuse d'agir sur une fenetre de LaRuche elle-meme, comparee par PID
//!      et non par titre, sinon l'agent peut cliquer "Approuver" sur sa propre
//!      demande d'approbation et toute la chaine de `butinage_pont` devient
//!      decorative;
//!   2. il rend la main des que l'humain bouge physiquement la souris. Le
//!      script Python actuel met `FAILSAFE = False`, il n'y a donc aujourd'hui
//!      aucune sortie de secours.

use crate::abeille::{Abeille, ContextExecution, NiveauDanger, ResultatAbeille};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use enigo::{Axis, Button, Coordinate, Direction, Enigo, Keyboard, Mouse, Settings};
use xcap::{Monitor, Window};

#[cfg(windows)]
use super::ordinateur_abandon as abandon;
#[cfg(windows)]
use super::ordinateur_arbre as arbre;

/// Le decor, quand il existe. Sur les plateformes sans halo, ces quatre
/// fonctions ne font rien, ce qui evite de parsemer le reste du fichier de
/// `#[cfg]`: le code appelant est ecrit une fois, il decrit ce qu'il fait, et
/// c'est ici que se decide si quelqu'un l'affiche.
mod decor {
    #[cfg(windows)]
    // `eteindre` n'est pas appele aujourd'hui: le decor s'efface tout seul au bout
    // de douze secondes sans action, exactement comme celui du navigateur, et
    // l'outil n'a aucun moyen de savoir quand le tour du modele se termine.
    #[allow(unused_imports)]
    pub use super::super::ordinateur_halo::{curseur, ecran, eteindre, flash, ligne};

    #[cfg(not(windows))]
    pub fn ligne(_: &str) {}
    #[cfg(not(windows))]
    pub fn curseur(_: i32, _: i32, _: bool) {}
    #[cfg(not(windows))]
    pub fn ecran(_: i32, _: i32, _: i32, _: i32) {}
    #[cfg(not(windows))]
    pub fn flash() {}
    #[cfg(not(windows))]
    #[allow(dead_code)]
    pub fn eteindre() {}
}

/// Amene le curseur a destination en glissant, pour qu'un humain qui regarde
/// puisse suivre ce que fait l'agent.
///
/// Le saut instantane est plus rapide et parfois necessaire, mais il rend le
/// pilotage illisible: la fenetre change et personne ne sait pourquoi. Meme
/// raisonnement que pour l'animation du navigateur, et meme reglage: `animate`
/// l'eteint, `speed` la ralentit.
fn glisser_vers(enigo: &mut Enigo, x: f64, y: f64, facteur: f64, duree_ms: u64) {
    let depart = enigo.location().unwrap_or((0, 0));
    let (dx, dy) = (depart.0 as f64, depart.1 as f64);
    let (ax, ay) = (x * facteur, y * facteur);
    let distance = ((ax - dx).powi(2) + (ay - dy).powi(2)).sqrt();
    // Sous quelques pixels, animer ne montre rien et coute une frame.
    if duree_ms == 0 || distance < 6.0 {
        let _ = enigo.move_mouse(ax.round() as i32, ay.round() as i32, Coordinate::Abs);
        decor::curseur((ax / facteur) as i32, (ay / facteur) as i32, false);
        return;
    }
    let pas = (duree_ms / 16).clamp(6, 40) as f64;
    for i in 1..=(pas as u64) {
        let t = i as f64 / pas;
        // Meme courbe que le curseur du navigateur: depart franc, arrivee douce.
        let e = 1.0 - (1.0 - t).powi(3);
        let (cx, cy) = (dx + (ax - dx) * e, dy + (ay - dy) * e);
        let _ = enigo.move_mouse(cx.round() as i32, cy.round() as i32, Coordinate::Abs);
        decor::curseur((cx / facteur) as i32, (cy / facteur) as i32, false);
        std::thread::sleep(Duration::from_millis(duree_ms / pas as u64));
    }
}

/// Les noms d'action que les modeles produisent naturellement.
///
/// `click` est le nom qu'utilise l'outil `browser`, et le meme modele passe de
/// l'un a l'autre dans la meme conversation. Refuser sur un synonyme evident ne
/// protege de rien, ca coute juste un tour et une chance de se tromper ailleurs.
fn alias_action(action: &str) -> String {
    match action {
        "click" | "click_left" => "left_click",
        "move" | "move_mouse" | "hover" => "mouse_move",
        "screen_capture" | "capture" => "screenshot",
        "list_windows" => "windows",
        "monitors" | "displays" => "screens",
        "press" | "key_press" => "key",
        "write" | "type_text" => "type",
        "drag" => "left_click_drag",
        autre => autre,
    }
    .to_string()
}

/// Le point vise, quelle que soit la facon dont le modele l'a ecrit.
///
/// `coordinate: [x, y]` est la convention des outils de controle d'ecran les
/// plus repandus, donc c'est ce qu'un modele ecrit spontanement. Ignorer la clef
/// inconnue et cliquer la ou se trouvait deja le curseur est le pire des
/// comportements: l'action reussit, au mauvais endroit, sans un mot. Observe en
/// production, avec toute la suite de la demo qui part de travers.
fn point_demande(args: &Value, prefixe: &str) -> Option<(f64, f64)> {
    let cle = |suffixe: &str| {
        if prefixe.is_empty() {
            suffixe.to_string()
        } else {
            format!("{prefixe}_{suffixe}")
        }
    };
    if let (Some(x), Some(y)) = (args[cle("x")].as_f64(), args[cle("y")].as_f64()) {
        return Some((x, y));
    }
    for nom in [cle("coordinate"), cle("coordinates"), cle("position")] {
        if let Some(paire) = args.get(&nom).and_then(Value::as_array) {
            if let (Some(x), Some(y)) = (
                paire.first().and_then(Value::as_f64),
                paire.get(1).and_then(Value::as_f64),
            ) {
                return Some((x, y));
            }
        }
    }
    None
}

/// La fenetre qui a le focus, et donc celle qui recevra la frappe.
///
/// Le dire dans le resultat change tout pour le modele: sans cela, `type` repond
/// "tape dans ce qui a le focus", ce qui est exact et parfaitement inutile. Un
/// modele a ainsi ecrit trois phrases dans le navigateur en croyant remplir le
/// Bloc-notes, et il lui a fallu quatre commandes PowerShell pour s'en rendre
/// compte.
fn fenetre_active() -> Option<(String, u32)> {
    let fenetres = Window::all().ok()?;
    let f = fenetres
        .into_iter()
        .find(|f| f.is_focused().unwrap_or(false))?;
    let titre = f.title().unwrap_or_default();
    let app = f.app_name().unwrap_or_default();
    let nom = if titre.trim().is_empty() { app } else { titre };
    Some((nom, f.pid().unwrap_or(0)))
}

/// Ou part la frappe, en une phrase, pour la coller au resultat.
fn ou_part_la_frappe() -> String {
    match fenetre_active() {
        Some((nom, _)) => format!(" Focus: \"{nom}\"."),
        None => String::new(),
    }
}

/// La ligne affichee dans le panneau, ecrite AVANT le geste.
///
/// Pendant les quelques centaines de millisecondes ou le curseur glisse vers sa
/// cible, l'humain doit deja savoir ce qui est vise. Une ligne posee apres coup
/// arriverait toujours quand elle ne sert plus a rien.
fn resume(action: &str, args: &Value) -> String {
    let court = |s: &str| -> String {
        let s = s.replace(['\n', '\r'], " ");
        if s.chars().count() > 30 {
            format!("{}...", s.chars().take(30).collect::<String>())
        } else {
            s
        }
    };
    let detail = match action {
        "read" | "focus_window" => court(args["window"].as_str().unwrap_or("front window")),
        "fill" => format!(
            "ref_{} = {}",
            args["ref"].as_u64().unwrap_or(0),
            court(args["text"].as_str().unwrap_or(""))
        ),
        "type" => court(args["text"].as_str().unwrap_or("")),
        "key" | "key_down" | "key_up" => {
            court(args["key"].as_str().or(args["text"].as_str()).unwrap_or(""))
        }
        "scroll" => args["direction"].as_str().unwrap_or("down").to_string(),
        "screenshot" => match args.get("screen") {
            Some(v) if !v.is_null() => format!("screen {v}"),
            _ => String::new(),
        },
        _ => match args["ref"].as_u64() {
            Some(r) => format!("ref_{r}"),
            None => match (args["x"].as_f64(), args["y"].as_f64()) {
                (Some(x), Some(y)) => format!("{x:.0},{y:.0}"),
                _ => String::new(),
            },
        },
    };
    format!("{action} {detail}").trim_end().to_string()
}

/// Largeur par defaut de la capture rendue au modele.
///
/// Une capture 4K brute coute une fortune en tokens ET brouille la visee: le
/// modele lit mal des coordonnees dans une image qu'il ne voit qu'a travers un
/// redimensionnement dont il ignore le facteur. On reduit donc ici, une fois,
/// et on garde le facteur pour reconvertir nous-memes.
const LARGEUR_CAPTURE: u32 = 1280;

/// Au-dela de ce deplacement non explique, on considere que l'humain a repris
/// la souris. Assez large pour ne pas se declencher sur l'arrondi d'un pixel,
/// assez etroit pour attraper un vrai geste.
const SEUIL_REPRISE_PX: f64 = 30.0;

/// Duree maximale d'un maintien de touche, pour qu'une erreur de l'agent ne
/// laisse pas une touche enfoncee une minute.
const MAINTIEN_MAX_MS: u64 = 10_000;

/// Le cadre de la derniere capture, qui definit le systeme de coordonnees dans
/// lequel le modele s'exprime jusqu'a la capture suivante.
#[derive(Clone, Copy, Debug)]
struct Cadre {
    ecran: u32,
    /// Origine physique du moniteur dans le bureau virtuel.
    origine_x: i32,
    origine_y: i32,
    physique_l: u32,
    physique_h: u32,
    image_l: u32,
    image_h: u32,
}

impl Cadre {
    /// Pixels de l'image rendue au modele vers pixels physiques du bureau.
    fn vers_physique(&self, x: f64, y: f64) -> (f64, f64) {
        let fx = self.physique_l as f64 / self.image_l.max(1) as f64;
        let fy = self.physique_h as f64 / self.image_h.max(1) as f64;
        (
            self.origine_x as f64 + x * fx,
            self.origine_y as f64 + y * fy,
        )
    }

    /// Le retour, pour dire au modele ou se trouve le curseur dans SON image.
    fn vers_image(&self, px: f64, py: f64) -> (f64, f64) {
        let fx = self.image_l as f64 / self.physique_l.max(1) as f64;
        let fy = self.image_h as f64 / self.physique_h.max(1) as f64;
        (
            (px - self.origine_x as f64) * fx,
            (py - self.origine_y as f64) * fy,
        )
    }
}

/// Etat de session, volontairement minimal: le cadre courant et la derniere
/// position que NOUS avons demandee, qui sert a detecter la reprise humaine.
#[derive(Default)]
struct Etat {
    cadre: Option<Cadre>,
    /// Position physique posee par la derniere action, et quand.
    derniere_position: Option<(f64, f64, Instant)>,
    /// Vrai quand on a deja signale une reprise humaine et qu'on attend que
    /// l'agent reformule: le refus ne doit pas devenir un blocage definitif.
    reprise_signalee: bool,
    /// Ce qui est reste enfonce: touches, puis boutons de souris.
    ///
    /// Une touche ou un bouton laisse enfonce ne casse pas seulement la suite de
    /// la mission, il casse la machine de l'utilisateur, et personne ne fera le
    /// lien entre un Ctrl fantome et un agent qui a rendu la main il y a dix
    /// minutes. Le skill le demandait par ecrit ("if you press a key down,
    /// release it"), ce qui revenait a confier au modele un invariant que le
    /// code peut tenir seul.
    touches_enfoncees: Vec<(String, Instant)>,
    boutons_enfonces: Vec<(String, Instant)>,
}

/// Au-dela de ce delai, un maintien n'est plus une intention, c'est un oubli.
///
/// Assez long pour qu'une sequence normale, tenir Shift puis cliquer trois
/// fois, ne soit jamais interrompue. Assez court pour qu'un abandon en plein
/// milieu ne laisse pas la machine inutilisable jusqu'au redemarrage.
const MAINTIEN_MAX: Duration = Duration::from_secs(60);

fn etat() -> &'static Mutex<Etat> {
    static ETAT: std::sync::OnceLock<Mutex<Etat>> = std::sync::OnceLock::new();
    ETAT.get_or_init(|| Mutex::new(Etat::default()))
}

/// Relache tout ce qui est enfonce et rend la liste de ce qui l'etait.
///
/// `perimes_seulement` ne libere que ce qui est tenu depuis plus de
/// [`MAINTIEN_MAX`], ce qui laisse tranquille une sequence en cours.
fn relacher_tout(enigo: &mut Enigo, perimes_seulement: bool) -> Vec<String> {
    let (touches, boutons) = {
        let mut e = etat().lock().unwrap();
        let garder = |v: &mut Vec<(String, Instant)>| -> Vec<String> {
            let (a_liberer, a_garder): (Vec<_>, Vec<_>) = std::mem::take(v)
                .into_iter()
                .partition(|(_, t)| !perimes_seulement || t.elapsed() > MAINTIEN_MAX);
            *v = a_garder;
            a_liberer.into_iter().map(|(n, _)| n).collect()
        };
        let t = garder(&mut e.touches_enfoncees);
        let b = garder(&mut e.boutons_enfonces);
        (t, b)
    };

    let mut liberes = Vec::new();
    for nom in touches {
        if let Some((mods, touche)) = parse_touche(&nom) {
            let _ = enigo.key(touche, Direction::Release);
            for m in mods {
                let _ = enigo.key(m, Direction::Release);
            }
        }
        liberes.push(nom);
    }
    for nom in boutons {
        let bouton = match nom.as_str() {
            "right" => Button::Right,
            "middle" => Button::Middle,
            _ => Button::Left,
        };
        let _ = enigo.button(bouton, Direction::Release);
        liberes.push(format!("{nom} button"));
    }
    liberes
}

/// Le rapport entre l'espace de coordonnees d'enigo et les pixels physiques.
///
/// Ce n'est pas le meme selon la plateforme, et c'est exactement la ou pyautogui
/// se trompe. Windows sans conscience du DPI rapporte un bureau reduit; macOS
/// travaille en points logiques, donc moitie moins sur un Retina; X11 rapporte
/// des pixels physiques. Plutot que de coder ces trois cas en dur, on mesure:
/// enigo et xcap decrivent tous les deux l'ecran principal, le rapport de leurs
/// largeurs EST le facteur, quelle que soit la raison de l'ecart.
fn facteur_enigo(enigo: &Enigo, principal: &Monitor) -> f64 {
    let physique = principal.width().unwrap_or(0);
    if physique == 0 {
        return 1.0;
    }
    match enigo.main_display() {
        Ok((l, _)) if l > 0 => l as f64 / physique as f64,
        _ => 1.0,
    }
}

fn moniteurs() -> Result<Vec<Monitor>> {
    Monitor::all().map_err(|e| anyhow!("cannot enumerate monitors: {e}"))
}

fn moniteur_principal(liste: &[Monitor]) -> Option<&Monitor> {
    liste
        .iter()
        .find(|m| m.is_primary().unwrap_or(false))
        .or_else(|| liste.first())
}

/// Choisit l'ecran demande: par identifiant, par index (1 = premier), ou
/// l'ecran principal par defaut.
fn choisir_ecran<'a>(liste: &'a [Monitor], demande: Option<&Value>) -> Option<&'a Monitor> {
    match demande {
        None | Some(Value::Null) => moniteur_principal(liste),
        Some(v) => {
            if let Some(n) = v.as_u64() {
                // Un identifiant d'abord, un rang ensuite: sur beaucoup de
                // systemes l'identifiant est un grand nombre, donc les deux
                // lectures ne se marchent pas dessus en pratique.
                liste
                    .iter()
                    .find(|m| m.id().ok().map(u64::from) == Some(n))
                    .or_else(|| liste.get(n.saturating_sub(1) as usize))
            } else if let Some(s) = v.as_str() {
                // `"1"` est un rang ecrit en chaine, pas un nom d'ecran. Un
                // modele produit l'un ou l'autre selon son humeur, et refuser
                // sur ce detail lui coute un tour pour rien.
                if let Ok(n) = s.trim().parse::<u64>() {
                    return liste
                        .iter()
                        .find(|m| m.id().ok().map(u64::from) == Some(n))
                        .or_else(|| liste.get(n.saturating_sub(1) as usize));
                }
                liste.iter().find(|m| {
                    m.name().map(|n| n.eq_ignore_ascii_case(s)).unwrap_or(false)
                        || m.friendly_name()
                            .map(|n| n.to_lowercase().contains(&s.to_lowercase()))
                            .unwrap_or(false)
                })
            } else {
                None
            }
        }
    }
}

/// Sur quel ecran tombe un rectangle de fenetre, en rang lisible (1, 2, ...).
///
/// Par recouvrement et non par coin haut gauche: une fenetre a cheval sur deux
/// moniteurs appartient a celui qui en montre le plus, ce qui est ce que
/// l'humain voit. Le coin haut gauche designerait l'ecran de gauche pour une
/// fenetre dont 90% est a droite.
/// Prend des rectangles nus plutot que des `Monitor`: la regle du recouvrement
/// est la seule chose qui puisse se tromper ici, et on ne construit pas un
/// moniteur dans un test.
fn ecran_du_rect(ecrans: &[(i32, i32, i32, i32)], x: i32, y: i32, l: i32, h: i32) -> Option<usize> {
    let (x2, y2) = (x + l, y + h);
    let mut meilleur: Option<(i64, usize)> = None;
    for (i, (mx, my, ml, mh)) in ecrans.iter().enumerate() {
        let (mx2, my2) = (mx + ml, my + mh);
        let cl = i64::from((x2.min(mx2) - x.max(*mx)).max(0));
        let ch = i64::from((y2.min(my2) - y.max(*my)).max(0));
        let aire = cl * ch;
        if aire > 0 && meilleur.as_ref().map(|(a, _)| aire > *a).unwrap_or(true) {
            meilleur = Some((aire, i + 1));
        }
    }
    meilleur.map(|(_, rang)| rang)
}

fn ecran_de_la_fenetre(liste: &[Monitor], x: i32, y: i32, l: i32, h: i32) -> Option<usize> {
    let rects: Vec<(i32, i32, i32, i32)> = liste
        .iter()
        .map(|m| {
            (
                m.x().unwrap_or(0),
                m.y().unwrap_or(0),
                m.width().unwrap_or(0) as i32,
                m.height().unwrap_or(0) as i32,
            )
        })
        .collect();
    ecran_du_rect(&rects, x, y, l, h)
}

fn decrire_ecrans(liste: &[Monitor]) -> String {
    liste
        .iter()
        .enumerate()
        .map(|(i, m)| {
            format!(
                "screen {} (id {}) {}x{} at ({},{}) scale {:.2}{}  {}",
                i + 1,
                m.id().unwrap_or(0),
                m.width().unwrap_or(0),
                m.height().unwrap_or(0),
                m.x().unwrap_or(0),
                m.y().unwrap_or(0),
                m.scale_factor().unwrap_or(1.0),
                if m.is_primary().unwrap_or(false) {
                    " primary"
                } else {
                    ""
                },
                m.friendly_name().or_else(|_| m.name()).unwrap_or_default()
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// La fenetre au-dessus du point donne appartient-elle a LaRuche?
///
/// Compare le PID et non le titre: un titre se change, se traduit et se
/// falsifie, un PID non. On prend la fenetre de z le plus eleve parmi celles
/// qui contiennent le point; a defaut d'ordre fiable, la comparaison reste
/// conservatrice, elle refuse plutot deux fois qu'une.
fn fenetre_laruche_au_point(px: f64, py: f64) -> Option<String> {
    let moi = std::process::id();
    let fenetres = Window::all().ok()?;
    let mut candidate: Option<(i32, u32, String)> = None;
    for f in fenetres {
        if f.is_minimized().unwrap_or(false) {
            continue;
        }
        let (x, y) = (f.x().ok()? as f64, f.y().ok()? as f64);
        let (l, h) = (f.width().unwrap_or(0) as f64, f.height().unwrap_or(0) as f64);
        if px < x || py < y || px > x + l || py > y + h {
            continue;
        }
        let z = f.z().unwrap_or(0);
        let pid = f.pid().unwrap_or(0);
        let titre = f.title().unwrap_or_default();
        if candidate.as_ref().map(|(zc, _, _)| z >= *zc).unwrap_or(true) {
            candidate = Some((z, pid, titre));
        }
    }
    let (_, pid, titre) = candidate?;
    (pid == moi).then_some(titre)
}

/// Traduit un nom de touche vers enigo, accords compris.
///
/// Les noms sont reconnus sans tenir compte de la casse: un modele ecrit
/// `"enter"` aussi souvent que `"Enter"`, et etre strict la-dessus ne produit
/// qu'un geste silencieusement absent.
fn parse_touche(spec: &str) -> Option<(Vec<enigo::Key>, enigo::Key)> {
    use enigo::Key;
    let spec = spec.trim();
    if spec.is_empty() {
        return None;
    }
    let mut parts: Vec<&str> = spec.split('+').collect();
    if spec.ends_with('+') && parts.len() > 1 {
        parts.pop();
        parts.push("+");
    }
    let nom = parts.pop()?;
    let mut mods = Vec::new();
    for m in parts {
        mods.push(match m.trim().to_ascii_lowercase().as_str() {
            "ctrl" | "control" => Key::Control,
            "shift" => Key::Shift,
            "alt" | "option" => Key::Alt,
            "meta" | "cmd" | "command" | "win" | "super" => Key::Meta,
            // Un modificateur inconnu est une faute de frappe, et deviner
            // reviendrait a presser autre chose que ce qui est demande.
            _ => return None,
        });
    }
    Some((mods, touche_nommee(nom)?))
}

fn touche_nommee(nom: &str) -> Option<enigo::Key> {
    use enigo::Key;
    let n = nom.trim();
    Some(match n.to_ascii_lowercase().as_str() {
        "enter" | "return" => Key::Return,
        "tab" => Key::Tab,
        "escape" | "esc" => Key::Escape,
        "backspace" => Key::Backspace,
        "delete" | "del" => Key::Delete,
        "space" => Key::Space,
        "up" | "arrowup" => Key::UpArrow,
        "down" | "arrowdown" => Key::DownArrow,
        "left" | "arrowleft" => Key::LeftArrow,
        "right" | "arrowright" => Key::RightArrow,
        "home" => Key::Home,
        "end" => Key::End,
        "pageup" => Key::PageUp,
        "pagedown" => Key::PageDown,
        "control" | "ctrl" => Key::Control,
        "shift" => Key::Shift,
        "alt" => Key::Alt,
        "meta" | "cmd" | "win" | "super" => Key::Meta,
        "f1" => Key::F1,
        "f2" => Key::F2,
        "f3" => Key::F3,
        "f4" => Key::F4,
        "f5" => Key::F5,
        "f6" => Key::F6,
        "f7" => Key::F7,
        "f8" => Key::F8,
        "f9" => Key::F9,
        "f10" => Key::F10,
        "f11" => Key::F11,
        "f12" => Key::F12,
        _ => {
            let mut chars = n.chars();
            let c = chars.next()?;
            if chars.next().is_some() {
                return None;
            }
            Key::Unicode(c)
        }
    })
}

/// Le tour de vis: tout ce qui bouge la souris ou frappe le clavier passe par
/// ici, donc les deux gardes sont impossibles a contourner par une action qui
/// aurait oublie de les appeler.
/// L'humain a-t-il repris la souris? Sans toucher a la position memorisee.
///
/// Extrait de [`autoriser_geste`] pour que les gestes SANS coordonnees puissent
/// le consulter aussi. Une frappe ne vise aucun point, mais elle doit s'arreter
/// quand quelqu'un se sert de la machine, et c'est meme le cas le plus utile:
/// une longue frappe est precisement ce qu'on veut pouvoir interrompre.
fn verifier_reprise(enigo: &Enigo, facteur: f64) -> Result<()> {
    let mut etat = etat().lock().unwrap();
    let Some((ax, ay, quand)) = etat.derniere_position else {
        return Ok(());
    };
    if quand.elapsed() >= Duration::from_secs(45) || etat.reprise_signalee {
        return Ok(());
    }
    let Ok((cx, cy)) = enigo.location() else {
        return Ok(());
    };
    let (cx, cy) = (cx as f64 / facteur, cy as f64 / facteur);
    let ecart = ((cx - ax).powi(2) + (cy - ay).powi(2)).sqrt();
    if ecart <= SEUIL_REPRISE_PX {
        return Ok(());
    }
    etat.reprise_signalee = true;
    etat.derniere_position = None;
    Err(anyhow!(
        "Yielded: the mouse moved {ecart:.0}px away from where this tool left it, so the \
         human is using the machine right now. Nothing was typed. Say so and wait, or repeat \
         the same call to take control back deliberately."
    ))
}

fn autoriser_geste(px: f64, py: f64, enigo: &Enigo, facteur: f64) -> Result<()> {
    if let Some(titre) = fenetre_laruche_au_point(px, py) {
        return Err(anyhow!(
            "Refused: ({px:.0},{py:.0}) is inside LaRuche's own window (\"{titre}\"). \
             Clicking there would let you answer your own approval prompts, so this is \
             never allowed. Act on the application you were asked about instead."
        ));
    }

    let mut etat = etat().lock().unwrap();
    if let Some((ax, ay, quand)) = etat.derniere_position {
        // Passe un certain temps, l'utilisateur a forcement bouge sa souris pour
        // ses propres raisons, et le comparer n'a plus de sens.
        if quand.elapsed() < Duration::from_secs(45) && !etat.reprise_signalee {
            if let Ok((cx, cy)) = enigo.location() {
                let (cx, cy) = (cx as f64 / facteur, cy as f64 / facteur);
                let ecart = ((cx - ax).powi(2) + (cy - ay).powi(2)).sqrt();
                if ecart > SEUIL_REPRISE_PX {
                    etat.reprise_signalee = true;
                    etat.derniere_position = None;
                    return Err(anyhow!(
                        "Yielded: the mouse moved {ecart:.0}px away from where this tool \
                         left it, so the human is using the machine right now. Nothing was \
                         done. Say so and wait, or repeat the same call to take control \
                         back deliberately."
                    ));
                }
            }
        }
    }
    etat.reprise_signalee = false;
    etat.derniere_position = Some((px, py, Instant::now()));
    Ok(())
}

/// Resout les coordonnees fournies par le modele, qui sont celles de sa
/// derniere capture, vers des pixels physiques.
fn resoudre(x: f64, y: f64) -> Result<(f64, f64)> {
    let cadre = etat().lock().unwrap().cadre;
    match cadre {
        Some(c) => Ok(c.vers_physique(x, y)),
        // Sans capture prealable il n'existe aucun systeme de coordonnees
        // partage. Refuser est la seule reponse honnete: interpreter les
        // nombres comme des pixels physiques marcherait sur un montage simple
        // et raterait silencieusement partout ailleurs.
        None => Err(anyhow!(
            "No screenshot yet, so there is no coordinate system to point into. Call \
             screenshot first: coordinates are always pixels of the image it returns."
        )),
    }
}

pub struct Ordinateur;

impl Ordinateur {
    /// Tout le travail bloquant. enigo et xcap sont synchrones et parlent a
    /// l'OS: les laisser sur l'executeur async bloquerait le noeud entier
    /// pendant une capture d'ecran, qui prend des dizaines de millisecondes.
    fn executer_bloquant(args: Value) -> Result<ResultatAbeille> {
        let action = alias_action(args["action"].as_str().unwrap_or_default());
        let action = action.as_str();
        // Le decor suit les memes reglages que celui du navigateur, pour qu'un
        // utilisateur n'ait pas deux vocabulaires a retenir.
        let glow = args["glow"].as_bool().unwrap_or(true);
        let animate = glow && args["animate"].as_bool().unwrap_or(true);
        let vitesse = args["speed"].as_f64().filter(|v| *v > 0.0).unwrap_or(1.0);
        let glisse_ms = if animate {
            (450.0 * vitesse) as u64
        } else {
            0
        };
        if glow && action != "screens" && action != "cursor_position" {
            decor::ligne(&resume(action, &args));
        }

        let liste = moniteurs()?;
        if liste.is_empty() {
            return Err(anyhow!(
                "No monitor detected. On a headless server there is nothing to drive."
            ));
        }

        // Les deux actions purement descriptives n'ont besoin ni d'enigo ni de
        // garde: elles ne touchent a rien.
        if action == "screens" {
            return Ok(ResultatAbeille::ok(format!(
                "{} screen(s):\n{}\n\nPass `screen` to screenshot to choose one. \
                 Coordinates are always pixels of the screenshot you got last.",
                liste.len(),
                decrire_ecrans(&liste)
            )));
        }

        let mut enigo = Enigo::new(&Settings::default())
            .map_err(|e| anyhow!("cannot reach the input layer: {e}"))?;
        let facteur = facteur_enigo(&enigo, moniteur_principal(&liste).expect("liste non vide"));

        // Le coupe-circuit. Arme au premier appel et jamais desarme: son interet
        // est d'exister avant qu'on en ait besoin.
        #[cfg(windows)]
        {
            abandon::armer();
            if abandon::demande() {
                let liberes = relacher_tout(&mut enigo, false);
                return Err(anyhow!(
                    "Aborted: the user pressed Ctrl+Alt+Shift+H. Nothing was done.{} They took \
                     the machine back deliberately, so stop and say so; do not retry the \
                     gesture and do not work around it.",
                    if liberes.is_empty() {
                        String::new()
                    } else {
                        format!(" Released what was held: {}.", liberes.join(", "))
                    }
                ));
            }
        }

        // Le filet de securite des maintiens: ce qui traine depuis plus d'une
        // minute est relache avant toute autre chose. Une sequence normale ne le
        // voit jamais passer; une mission abandonnee en plein glisser ne laisse
        // plus la machine de l'utilisateur avec un bouton enfonce.
        let oublies = relacher_tout(&mut enigo, true);
        if !oublies.is_empty() {
            tracing::warn!(?oublies, "maintiens perimes relaches d'office");
            decor::ligne(&format!("released {}", oublies.join(", ")));
        }

        match action {
            "screenshot" => {
                let ecran = choisir_ecran(&liste, args.get("screen")).ok_or_else(|| {
                    anyhow!(
                        "No screen matches that. Call screens to list them.\n{}",
                        decrire_ecrans(&liste)
                    )
                })?;
                let image = ecran
                    .capture_image()
                    .map_err(|e| anyhow!("capture failed: {e}"))?;
                let (pl, ph) = (image.width(), image.height());
                let cible = args["max_width"]
                    .as_u64()
                    .unwrap_or(LARGEUR_CAPTURE as u64)
                    .clamp(320, 3840) as u32;
                let (il, ih) = if pl > cible {
                    (cible, (ph as f64 * cible as f64 / pl as f64).round() as u32)
                } else {
                    (pl, ph)
                };
                let reduite = if (il, ih) == (pl, ph) {
                    xcap::image::DynamicImage::ImageRgba8(image)
                } else {
                    xcap::image::DynamicImage::ImageRgba8(image).resize_exact(
                        il,
                        ih,
                        xcap::image::imageops::FilterType::Triangle,
                    )
                };
                let mut png = std::io::Cursor::new(Vec::new());
                reduite
                    .write_to(&mut png, xcap::image::ImageFormat::Png)
                    .map_err(|e| anyhow!("cannot encode the capture: {e}"))?;

                let cadre = Cadre {
                    ecran: ecran.id().unwrap_or(0),
                    origine_x: ecran.x().unwrap_or(0),
                    origine_y: ecran.y().unwrap_or(0),
                    physique_l: pl,
                    physique_h: ph,
                    image_l: il,
                    image_h: ih,
                };
                etat().lock().unwrap().cadre = Some(cadre);
                if glow {
                    // Le cadre du halo suit l'ecran regarde, pas l'ecran
                    // principal: sur deux moniteurs c'est la seule facon de
                    // montrer OU l'agent travaille.
                    decor::ecran(
                        cadre.origine_x,
                        cadre.origine_y,
                        cadre.origine_x + pl as i32,
                        cadre.origine_y + ph as i32,
                    );
                    // Apres la capture, jamais avant: le flash signale la prise
                    // sans apparaitre dedans.
                    decor::flash();
                }

                let mut out = ResultatAbeille::ok(format!(
                    "Screen {} captured, shown to you at {}x{} (physical {}x{}). \
                     Point at things with coordinates read off THIS image; the tool \
                     converts them itself.\n\n{} screen(s) available:\n{}",
                    cadre.ecran,
                    il,
                    ih,
                    pl,
                    ph,
                    liste.len(),
                    decrire_ecrans(&liste)
                ));
                out.images = vec![base64_png(png.into_inner())];
                Ok(out)
            }

            "cursor_position" => {
                let (ex, ey) = enigo
                    .location()
                    .map_err(|e| anyhow!("cannot read the cursor position: {e}"))?;
                let (px, py) = (ex as f64 / facteur, ey as f64 / facteur);
                let cadre = etat().lock().unwrap().cadre;
                Ok(ResultatAbeille::ok(match cadre {
                    Some(c) => {
                        let (ix, iy) = c.vers_image(px, py);
                        format!(
                            "Cursor at ({ix:.0},{iy:.0}) in the last screenshot, \
                             ({px:.0},{py:.0}) on the desktop."
                        )
                    }
                    None => format!(
                        "Cursor at ({px:.0},{py:.0}) on the desktop. Take a screenshot to \
                         get a coordinate system you can point into."
                    ),
                }))
            }

            "windows" => Self::lister_fenetres(),

            "read" | "find" | "focus_window" | "focus" => Self::geste_arbre(action, &args),

            "move_window" | "minimize_window" | "maximize_window" | "restore_window"
            | "close_window" => Self::geste_fenetre(action, &args),

            "mouse_move" | "left_click" | "right_click" | "middle_click" | "double_click"
            | "triple_click" | "left_click_drag" | "mouse_down" | "mouse_up" | "scroll"
            | "fill" => Self::geste_souris(action, &args, &mut enigo, facteur, glisse_ms),

            // Le presse-papiers est la voie la plus courte pour faire entrer et
            // sortir du texte d'une application native. Sans lui, l'agent
            // pouvait envoyer Control+c et n'avait aucun moyen de LIRE ce qu'il
            // venait de copier; et le refus de `type` au-dela de cinq mille
            // caracteres conseillait de coller depuis un fichier, un conseil que
            // l'outil ne pouvait pas suivre.
            "read_clipboard" => {
                let contenu = arboard::Clipboard::new()
                    .and_then(|mut c| c.get_text())
                    .map_err(|e| anyhow!("cannot read the clipboard: {e}"))?;
                Ok(ResultatAbeille::ok(if contenu.is_empty() {
                    "The clipboard holds no text.".to_string()
                } else {
                    format!("Clipboard ({} chars):\n{contenu}", contenu.chars().count())
                }))
            }

            "write_clipboard" => {
                let texte = args["text"]
                    .as_str()
                    .ok_or_else(|| anyhow!("write_clipboard needs 'text'."))?;
                arboard::Clipboard::new()
                    .and_then(|mut c| c.set_text(texte.to_string()))
                    .map_err(|e| anyhow!("cannot write the clipboard: {e}"))?;
                Ok(ResultatAbeille::ok(format!(
                    "Put {} character(s) on the clipboard. Paste it with key Control+v, \
                     which has no length limit, unlike type.",
                    texte.chars().count()
                )))
            }

            "release_all" => {
                let liberes = relacher_tout(&mut enigo, false);
                Ok(ResultatAbeille::ok(if liberes.is_empty() {
                    "Nothing was held down.".to_string()
                } else {
                    format!("Released: {}.", liberes.join(", "))
                }))
            }

            "wait" => {
                let ms = args["ms"]
                    .as_u64()
                    .or_else(|| args["seconds"].as_u64().map(|s| s * 1000))
                    .unwrap_or(1000)
                    .clamp(50, 30_000);
                std::thread::sleep(Duration::from_millis(ms));
                Ok(ResultatAbeille::ok(format!(
                    "Waited {ms}ms. Read or take a screenshot to see where things got to."
                )))
            }

            "type" => {
                let texte = args["text"]
                    .as_str()
                    .ok_or_else(|| anyhow!("type needs 'text'."))?;
                if texte.chars().count() > 5000 {
                    return Err(anyhow!(
                        "Text too long (max 5000 chars). Paste through a file instead."
                    ));
                }
                // La frappe va au focus courant, pas a un point de l'ecran, donc
                // la garde de POSITION ne s'applique pas. Mais rendre la main
                // quand l'humain reprend la souris, si: sans ca, un agent en
                // train de taper ne pouvait pas etre arrete en touchant la
                // souris, ce qui vidait le coupe-circuit de son sens exactement
                // dans le cas ou il sert le plus, une longue frappe.
                verifier_reprise(&enigo, facteur)?;
                // Ou va la frappe, LU AVANT de taper: apres, une fenetre a pu
                // s'ouvrir, et on rapporterait la mauvaise.
                let ou = ou_part_la_frappe();
                let mut avertissement = String::new();
                if let Some((nom, pid)) = fenetre_active() {
                    if pid == std::process::id() {
                        return Err(anyhow!(
                            "Refused: the focus is on LaRuche's own window (\"{nom}\"). \
                             Typing there would put text into your own interface. Bring the \
                             application you were asked about to the front first, with \
                             focus_window."
                        ));
                    }
                    // Une fenetre elevee avale la frappe sans rien dire. Le
                    // rapport dirait "10 caracteres tapes" et l'ecran serait
                    // inchange; le modele conclurait a une erreur de focus et
                    // recommencerait.
                    #[cfg(windows)]
                    if let Some(p) = super::ordinateur_fenetres::hors_datteinte(pid) {
                        avertissement = format!(" Warning: {p}");
                    }
                }
                if glisse_ms > 0 && texte.chars().count() <= 200 {
                    // Frappe progressive, comme dans le navigateur: on voit le
                    // texte apparaitre au lieu de le voir surgir. Au-dela de
                    // deux cents caracteres ce serait une punition, pas une
                    // demonstration.
                    let par_char = if texte.chars().count() > 60 { 18 } else { 32 };
                    for (ecrits, c) in texte.chars().enumerate() {
                        // Le coupe-circuit se consulte a CHAQUE caractere. Une
                        // longue frappe est precisement ce qu'on veut pouvoir
                        // arreter, et ne la verifier qu'entre deux appels
                        // laisserait partir le texte entier.
                        #[cfg(windows)]
                        if abandon::en_cours() {
                            return Err(anyhow!(
                                "Aborted after {ecrits} of {} characters: the user pressed                                  Ctrl+Alt+Shift+H mid-typing. What was already typed is still                                  in the window, so read it before doing anything else.",
                                texte.chars().count()
                            ));
                        }
                        enigo
                            .text(&c.to_string())
                            .map_err(|e| anyhow!("typing failed: {e}"))?;
                        std::thread::sleep(Duration::from_millis(par_char));
                    }
                } else {
                    enigo
                        .text(texte)
                        .map_err(|e| anyhow!("typing failed: {e}"))?;
                }
                Ok(ResultatAbeille::ok(format!(
                    "Typed {} character(s).{ou}{avertissement} If that is not the window you \
                     meant, nothing you typed went where you thought: bring the right one to \
                     the front with focus_window and type again.",
                    texte.chars().count()
                )))
            }

            "key" | "key_down" | "key_up" => Self::geste_clavier(action, &args, &mut enigo),

            autre => Err(anyhow!(
                "Unknown action '{autre}'. The reliable path first: windows (list them), \
                 focus_window, read (numbered map of the controls), find (the same, filtered), \
                 then click, fill, scroll or hover with a `ref`, plus focus. By pixels: \
                 screens, screenshot, cursor_position, mouse_move, left_click, right_click, \
                 middle_click, double_click, triple_click, left_click_drag, mouse_down, \
                 mouse_up, scroll. Keyboard: type, key, key_down, key_up, release_all. \
                 Also: wait, read_clipboard, write_clipboard."
            )),
        }
    }

    /// Les fenetres ouvertes. Traverse les plateformes, contrairement a l'arbre
    /// d'accessibilite: xcap sait enumerer sur les trois systemes.
    fn lister_fenetres() -> Result<ResultatAbeille> {
        let ecrans = moniteurs().unwrap_or_default();
        // L'ecran de la derniere capture, pour dire tout de suite si le modele
        // regarde le bon. Sans ca il devait croiser des coordonnees absolues
        // avec la liste des moniteurs, calcul qu'il fait de tete et rate en
        // silence: il visait une fenetre parfaitement reelle, sur l'autre ecran,
        // avec des coordonnees lues sur celui-ci.
        let regarde = etat().lock().unwrap().cadre.map(|c| c.ecran);
        let rang_regarde = regarde.and_then(|id| {
            ecrans
                .iter()
                .position(|m| m.id().unwrap_or(0) == id)
                .map(|i| i + 1)
        });

        let mut lignes: Vec<String> = Window::all()
            .map_err(|e| anyhow!("cannot enumerate windows: {e}"))?
            .into_iter()
            .filter(|f| !f.is_minimized().unwrap_or(false))
            .filter_map(|f| {
                let titre = f.title().unwrap_or_default();
                let app = f.app_name().unwrap_or_default();
                // Une fenetre sans titre ni application est un artefact du
                // compositeur, pas quelque chose que l'agent peut viser.
                if titre.trim().is_empty() && app.trim().is_empty() {
                    return None;
                }
                let (x, y) = (f.x().unwrap_or(0), f.y().unwrap_or(0));
                let (l, h) = (f.width().unwrap_or(0), f.height().unwrap_or(0));
                let rang = ecran_de_la_fenetre(&ecrans, x, y, l as i32, h as i32);
                let ou = match rang {
                    Some(r) if Some(r) == rang_regarde => format!("screen {r} <- captured"),
                    Some(r) => format!("screen {r}"),
                    None => "off-screen".to_string(),
                };
                Some(format!(
                    "{}{}x{} at ({},{})  {}  [{}]  {}",
                    if f.is_focused().unwrap_or(false) {
                        "* "
                    } else {
                        "  "
                    },
                    l,
                    h,
                    x,
                    y,
                    ou,
                    app,
                    titre
                ))
            })
            .collect();
        lignes.sort();

        // Les reduites, a part et nommees. Elles etaient simplement absentes, ce
        // qui faisait croire que l'application n'existait pas: l'agent cherchait
        // un Bloc-notes ouvert et concluait qu'il fallait le lancer, alors qu'il
        // etait la, dans la barre des taches, avec le travail de l'utilisateur
        // dedans.
        #[cfg(windows)]
        let reduites: Vec<String> = super::ordinateur_fenetres::lister()
            .unwrap_or_default()
            .into_iter()
            .filter(|f| f.reduite)
            .map(|f| format!("  {}", f.titre))
            .collect();
        #[cfg(not(windows))]
        let reduites: Vec<String> = Vec::new();

        Ok(ResultatAbeille::ok(format!(
            "{} window(s) on screen, * marks the focused one:\n{}{}\n\n{}Use focus_window with \
             a piece of the title to bring one to the front, then read it. move_window, \
             minimize_window, maximize_window, restore_window and close_window take the same \
             piece of title.",
            lignes.len(),
            lignes.join("\n"),
            if reduites.is_empty() {
                String::new()
            } else {
                format!(
                    "\n\n{} minimised, not on screen but still open (focus_window or \
                     restore_window brings one back):\n{}",
                    reduites.len(),
                    reduites.join("\n")
                )
            },
            // Le rappel n'a de sens que sur plusieurs moniteurs, et il compte
            // surtout quand une capture a deja fige un systeme de coordonnees.
            if ecrans.len() < 2 {
                String::new()
            } else {
                match rang_regarde {
                    Some(r) => format!(
                        "Your last screenshot was of screen {r}, so any x,y you point at \
                         lands there. A window marked screen {} is on another monitor: \
                         capture that one first, or act on it by ref, which needs no \
                         coordinates at all.\n\n",
                        if r == 1 { 2 } else { 1 }
                    ),
                    None => format!(
                        "{} screens. You have not captured one yet, so there is no \
                         coordinate system: work by ref, or screenshot the screen holding \
                         the window you want.\n\n",
                        ecrans.len()
                    ),
                }
            }
        )))
    }

    /// Deplacer, redimensionner, reduire, agrandir, restaurer, fermer.
    #[cfg(windows)]
    fn geste_fenetre(action: &str, args: &Value) -> Result<ResultatAbeille> {
        use super::ordinateur_fenetres as fen;

        let filtre = args["window"].as_str().unwrap_or("").trim();
        let f = fen::trouver(filtre)?;

        // Refuser d'agir sur une fenetre de LaRuche vaut ici aussi: fermer sa
        // propre interface, ou la reduire au moment ou l'utilisateur y repond,
        // contourne les memes garde-fous qu'un clic.
        if f.pid == std::process::id() {
            return Err(anyhow!(
                "Refused: \"{}\" is one of LaRuche's own windows. Moving, hiding or closing \
                 the interface the user answers your approval prompts in is never allowed.",
                f.titre
            ));
        }

        // L'elevation se dit AVANT le geste. Une fenetre elevee ignore aussi
        // SetWindowPos et WM_CLOSE venus d'un processus ordinaire.
        if let Some(pourquoi) = fen::hors_datteinte(f.pid) {
            return Err(anyhow!("Cannot act on \"{}\": {pourquoi}", f.titre));
        }

        decor::ligne(&format!("{action} {}", f.titre));

        match action {
            "move_window" => {
                let lire = |cle: &str| args[cle].as_i64().map(|v| v as i32);
                fen::poser(
                    &f,
                    lire("x"),
                    lire("y"),
                    lire("width"),
                    lire("height"),
                )?;
                let apres = fen::trouver(filtre)?;
                Ok(ResultatAbeille::ok(format!(
                    "\"{}\" is now {}x{} at ({},{}).",
                    apres.titre,
                    apres.rect.2 - apres.rect.0,
                    apres.rect.3 - apres.rect.1,
                    apres.rect.0,
                    apres.rect.1
                )))
            }
            "minimize_window" | "maximize_window" | "restore_window" => {
                let quoi = action.trim_end_matches("_window");
                let fait = fen::etat(&f, quoi)?;
                Ok(ResultatAbeille::ok(format!("\"{}\" {fait}.", f.titre)))
            }
            "close_window" => {
                fen::fermer(&f)?;
                Ok(ResultatAbeille::ok(format!(
                    "Asked \"{}\" to close. This is the same as clicking its cross: the \
                     application may put up a save prompt, and may refuse. Call windows again \
                     to see whether it actually went, and read the screen if a dialog \
                     appeared.",
                    f.titre
                )))
            }
            autre => Err(anyhow!("unknown window action '{autre}'.")),
        }
    }

    #[cfg(not(windows))]
    fn geste_fenetre(_action: &str, _args: &Value) -> Result<ResultatAbeille> {
        Err(anyhow!(
            "Moving, resizing and closing windows is only wired for Windows today. The \
             `windows` action still lists what is open everywhere."
        ))
    }

    /// Tout ce qui passe par l'arbre d'accessibilite plutot que par les pixels.
    #[cfg(windows)]
    fn geste_arbre(action: &str, args: &Value) -> Result<ResultatAbeille> {
        match action {
            "find" => {
                let quoi = args["text"]
                    .as_str()
                    .filter(|s| !s.trim().is_empty())
                    .ok_or_else(|| {
                        anyhow!("find needs 'text', the wording to look for in the controls.")
                    })?;
                let filtre = args["window"].as_str().filter(|s| !s.trim().is_empty());
                let (titre, trouves, tronque) = arbre::chercher(filtre, quoi)?;
                if trouves.is_empty() {
                    return Ok(ResultatAbeille::ok(format!(
                        "Nothing matching \"{quoi}\" in window \"{titre}\"{}. The wording may \
                         differ, or the control may be scrolled out of view, which hides it \
                         from the tree entirely: scroll a nearby ref into view and search \
                         again, or read the whole window.",
                        if tronque {
                            ", and the window was too big to list whole"
                        } else {
                            ""
                        }
                    )));
                }
                Ok(ResultatAbeille::ok(format!(
                    "Window: {titre}\n\n{} match(es) for \"{quoi}\":\n{}\n\nThese numbers come \
                     from this search, which read the window: use them now, and read or \
                     search again after acting.{}",
                    trouves.len(),
                    trouves.join("\n"),
                    if tronque {
                        " The window was too big to list whole, so a match may be missing."
                    } else {
                        ""
                    }
                )))
            }
            "read" => {
                let filtre = args["window"].as_str().filter(|s| !s.trim().is_empty());
                let l = arbre::lire(filtre)?;
                if l.lignes.is_empty() && l.textes.is_empty() {
                    return Ok(ResultatAbeille::ok(format!(
                        "Window \"{}\" exposes no accessible control. It is probably drawn \
                         by hand (a game, a canvas, a poorly tagged Electron app). Fall back \
                         to screenshot plus coordinates for this one.",
                        l.titre
                    )));
                }
                let mut sortie = format!(
                    "Window: {}\n\nActionable elements ({}{}):\n{}",
                    l.titre,
                    l.lignes.len(),
                    if l.tronque { ", truncated" } else { "" },
                    l.lignes.join("\n")
                );
                if !l.textes.is_empty() {
                    sortie.push_str(&format!("\n\nText on screen:\n{}", l.textes.join("\n")));
                }
                sortie.push_str(
                    "\n\nAct with click, double_click, right_click or fill and a `ref` \
                     number. Refs are renumbered by every read and are lost when the window \
                     changes, so read again after acting.",
                );
                Ok(ResultatAbeille::ok(sortie))
            }
            "focus_window" => {
                let filtre = args["window"].as_str().filter(|s| !s.trim().is_empty());
                let Some(filtre) = filtre else {
                    return Err(anyhow!(
                        "focus_window needs 'window', a piece of the window title. List them \
                         with the windows action."
                    ));
                };
                // Une fenetre reduite n'est pas dans l'arbre d'accessibilite: la
                // restaurer d'abord est la seule facon de la ramener, et c'est
                // exactement ce qu'on veut dire par "mets-la devant".
                let restauree = {
                    let f = super::ordinateur_fenetres::trouver(filtre).ok();
                    match f {
                        Some(f) if f.reduite => {
                            super::ordinateur_fenetres::activer(&f)?;
                            // Laisser l'animation de restauration finir, sinon
                            // l'arbre est lu sur une fenetre encore en vol.
                            std::thread::sleep(Duration::from_millis(250));
                            Some(f.titre)
                        }
                        _ => None,
                    }
                };
                let titre = match restauree {
                    Some(t) => t,
                    None => arbre::activer_fenetre(filtre)?,
                };
                // Prevenir MAINTENANT plutot que de laisser le premier clic
                // disparaitre dans le vide.
                let eleve = super::ordinateur_fenetres::trouver(filtre)
                    .ok()
                    .and_then(|f| super::ordinateur_fenetres::hors_datteinte(f.pid));
                Ok(ResultatAbeille::ok(format!(
                    "\"{titre}\" is now in front. Refs were dropped with the previous \
                     window: read it before acting.{}",
                    match eleve {
                        Some(p) => format!(" Warning: {p}"),
                        None => String::new(),
                    }
                )))
            }
            "focus" => {
                let Some(n) = args["ref"].as_u64() else {
                    return Err(anyhow!("focus needs a 'ref' number from read."));
                };
                let c = arbre::focaliser(n as usize)?;
                Ok(ResultatAbeille::ok(format!(
                    "ref_{n} <{}> {} has focus. Type into it with the type action.",
                    c.genre, c.nom
                )))
            }
            _ => unreachable!("action filtree en amont"),
        }
    }

    #[cfg(not(windows))]
    fn geste_arbre(_action: &str, _args: &Value) -> Result<ResultatAbeille> {
        Err(anyhow!(
            "The accessibility tree is only wired for Windows today (UI Automation). On this \
             system, use screenshot plus coordinates. The `windows` action still lists what \
             is open."
        ))
    }

    /// Agir sur un element numerote plutot que sur un point.
    ///
    /// On tente d'abord le motif d'automatisation, qui appelle le controle
    /// directement: c'est deterministe, ca ne depend pas de la position du
    /// curseur et ca marche meme si la fenetre n'est pas au premier plan. Le
    /// clic physique n'est que le repli, pour les interfaces qui n'exposent
    /// rien (jeux, canvas, Electron mal balise).
    #[cfg(windows)]
    fn geste_ref(
        action: &str,
        args: &Value,
        numero: usize,
        enigo: &mut Enigo,
        facteur: f64,
        glisse_ms: u64,
    ) -> Result<ResultatAbeille> {
        use crate::abeilles::ordinateur_arbre::Effet;

        // Ce qu'un `ref` veut dire depend de l'action, et les confondre coutait
        // cher: tout ce qui n'etait ni fill, ni clic droit, ni double clic
        // partait dans `actionner`, donc etait INVOQUE. `hover` sur un bouton le
        // cliquait, `middle_click` faisait un clic gauche, `scroll` et
        // `left_click_drag` actionnaient l'element au lieu de le faire defiler
        // ou de le glisser. Le tout rapporte comme un succes, ce qui est le pire
        // mode de defaillance possible: le modele n'avait aucun moyen de savoir
        // que son geste n'avait pas eu lieu.
        //
        // Trois familles: celles qui ont un equivalent dans les motifs
        // d'automatisation, celles qui n'en ont pas et visent le rectangle, et
        // le defilement qui a son propre motif.
        let geste_physique = matches!(
            action,
            "right_click" | "double_click" | "middle_click" | "mouse_move" | "left_click_drag"
        );
        let par_motif = match action {
            "fill" => {
                let texte = args["text"]
                    .as_str()
                    .ok_or_else(|| anyhow!("fill needs 'text'."))?;
                arbre::remplir(numero, texte)?
            }
            // Amener l'element dans la vue a son propre motif, et c'est le seul
            // chemin vers un element hors ecran qui ne demande pas de capture.
            "scroll" => arbre::defiler_vers(numero)?,
            // Aucun equivalent en motif: on vise le rectangle memorise. Survoler
            // n'est pas cliquer, et un clic milieu n'est pas une invocation.
            _ if geste_physique => (
                arbre::cible(numero)
                    .ok_or_else(|| anyhow!("No element ref_{numero}. Run read again."))?,
                Effet::ClicRequis,
            ),
            _ => arbre::actionner(numero)?,
        };
        let (cible, effet) = par_motif;

        // Un controle qui porte une plage ne se clique pas, il se regle. On le
        // dit avec ses bornes, pour que la correction tienne en un seul appel.
        if let Effet::PlageRequise(min, max, actuel) = effet {
            return Err(anyhow!(
                "ref_{numero} <{}> {} holds a value on a range ({min} to {max}, currently \
                 {actuel}), so clicking it means nothing: the click would land at the centre \
                 of its rectangle, which is halfway along its travel. Set it instead, with \
                 fill on ref_{numero} and the number you want.",
                cible.genre,
                cible.nom
            ));
        }

        if let Effet::Motif(nom) = effet {
            return Ok(ResultatAbeille::ok(format!(
                "ref_{numero} <{}> {} acted on via its {nom} pattern, without touching the \
                 mouse. Read again to see what changed.",
                cible.genre, cible.nom
            )));
        }

        // Repli physique: on vise le centre du rectangle memorise, en passant
        // par les memes gardes que n'importe quel clic.
        let (px, py) = cible.centre();
        autoriser_geste(px, py, enigo, facteur)?;
        glisser_vers(enigo, px, py, facteur, glisse_ms);

        // Survoler s'arrete ici. C'est tout l'interet du geste: ouvrir un menu
        // ou une infobulle sans rien declencher.
        if action == "mouse_move" {
            decor::curseur(px as i32, py as i32, false);
            return Ok(ResultatAbeille::ok(format!(
                "Hovering ref_{numero} <{}> {} at ({px:.0},{py:.0}). Nothing was clicked. \
                 Read again to see what the hover opened.",
                cible.genre, cible.nom
            )));
        }

        // Le defilement n'a pas trouve son motif: on tourne la molette la ou est
        // l'element, ce qui defile son conteneur.
        if action == "scroll" {
            let direction = args["direction"].as_str().unwrap_or("down");
            let crans = args["amount"].as_i64().unwrap_or(3).clamp(1, 50) as i32;
            let (axe, signe) = match direction {
                "down" => (Axis::Vertical, 1),
                "up" => (Axis::Vertical, -1),
                "right" => (Axis::Horizontal, 1),
                "left" => (Axis::Horizontal, -1),
                _ => return Err(anyhow!("scroll direction must be up, down, left or right.")),
            };
            enigo
                .scroll(crans * signe, axe)
                .map_err(|e| anyhow!("scroll failed: {e}"))?;
            return Ok(ResultatAbeille::ok(format!(
                "ref_{numero} <{}> {} exposes no scroll pattern, so the wheel was turned \
                 {direction} {crans} notch(es) over it instead.",
                cible.genre, cible.nom
            )));
        }

        // Glisser DEPUIS l'element: le ref donne le point de prise, to_x et to_y
        // le point de depose.
        if action == "left_click_drag" {
            let (tx, ty) = match point_demande(args, "to") {
                Some((x, y)) => resoudre(x, y)?,
                None => {
                    return Err(anyhow!(
                        "left_click_drag from ref_{numero} needs 'to_x' and 'to_y', the point \
                         to drop on. Those are pixels of the last screenshot, so take one \
                         first if you have not."
                    ))
                }
            };
            autoriser_geste(tx, ty, enigo, facteur)?;
            decor::curseur(px as i32, py as i32, true);
            let bouton = Self::bouton_demande(args)?;
            let pose = args["hold_ms"].as_u64().unwrap_or(120).min(5000);
            Self::glisser_bouton(enigo, bouton, tx, ty, facteur, pose)?;
            return Ok(ResultatAbeille::ok(format!(
                "Dragged ref_{numero} <{}> {} to ({tx:.0},{ty:.0}) on the desktop.",
                cible.genre, cible.nom
            )));
        }

        decor::curseur(px as i32, py as i32, true);
        let bouton = match action {
            "right_click" => Button::Right,
            "middle_click" => Button::Middle,
            _ => Button::Left,
        };
        let fois = if action == "double_click" { 2 } else { 1 };
        for _ in 0..fois {
            enigo
                .button(bouton, Direction::Click)
                .map_err(|e| anyhow!("click failed: {e}"))?;
        }
        if action == "fill" {
            let texte = args["text"].as_str().unwrap_or_default();
            // Le champ garde son contenu: on le vide avant d'ecrire, sinon on
            // concatene a ce qui etait deja la.
            let _ = enigo.key(enigo::Key::Control, Direction::Press);
            let _ = enigo.key(enigo::Key::Unicode('a'), Direction::Click);
            let _ = enigo.key(enigo::Key::Control, Direction::Release);
            enigo
                .text(texte)
                .map_err(|e| anyhow!("typing failed: {e}"))?;
            return Ok(ResultatAbeille::ok(format!(
                "ref_{numero} <{}> {} exposes no value pattern, so it was clicked and typed \
                 into instead. Read again to check what landed.",
                cible.genre, cible.nom
            )));
        }
        Ok(ResultatAbeille::ok(format!(
            "ref_{numero} <{}> {} exposes no automation pattern, so it was clicked at \
             ({px:.0},{py:.0}) instead.",
            cible.genre, cible.nom
        )))
    }

    #[cfg(not(windows))]
    fn geste_ref(
        _action: &str,
        _args: &Value,
        _numero: usize,
        _enigo: &mut Enigo,
        _facteur: f64,
        _glisse_ms: u64,
    ) -> Result<ResultatAbeille> {
        Err(anyhow!(
            "Acting by ref needs the accessibility tree, which is only wired for Windows \
             today. Use screenshot plus x,y coordinates on this system."
        ))
    }

    /// Presse, deplace en paliers, relache.
    ///
    /// En un seul saut, beaucoup d'interfaces ne voient aucun deplacement et
    /// annulent le glisser. Les paliers ne suffisent pas non plus partout: le
    /// glisser-deposer HTML5, celui d'Electron et la plupart des gestionnaires
    /// de fichiers exigent que le bouton reste enfonce un court moment AVANT
    /// que ca bouge, sinon ils traitent le geste comme un clic. D'ou la pause
    /// au depart, et une seconde a l'arrivee pour laisser la cible s'armer
    /// avant le relachement.
    fn glisser_bouton(
        enigo: &mut Enigo,
        bouton: Button,
        tx: f64,
        ty: f64,
        facteur: f64,
        pose_ms: u64,
    ) -> Result<()> {
        enigo
            .button(bouton, Direction::Press)
            .map_err(|e| anyhow!("cannot press the button: {e}"))?;
        std::thread::sleep(Duration::from_millis(pose_ms));
        let depart = enigo.location().unwrap_or((0, 0));
        let (dx, dy) = (depart.0 as f64, depart.1 as f64);
        let (ax, ay) = (tx * facteur, ty * facteur);
        for i in 1..=12 {
            let t = i as f64 / 12.0;
            let _ = enigo.move_mouse(
                (dx + (ax - dx) * t).round() as i32,
                (dy + (ay - dy) * t).round() as i32,
                Coordinate::Abs,
            );
            std::thread::sleep(Duration::from_millis(16));
        }
        std::thread::sleep(Duration::from_millis(pose_ms));
        enigo
            .button(bouton, Direction::Release)
            .map_err(|e| anyhow!("cannot release the button: {e}"))?;
        Ok(())
    }

    /// Le bouton demande, pour les gestes qui en acceptent un.
    fn bouton_demande(args: &Value) -> Result<Button> {
        match args["button"].as_str().unwrap_or("left") {
            "left" => Ok(Button::Left),
            "right" => Ok(Button::Right),
            "middle" => Ok(Button::Middle),
            autre => Err(anyhow!(
                "button must be left, right or middle, not '{autre}'."
            )),
        }
    }

    fn geste_souris(
        action: &str,
        args: &Value,
        enigo: &mut Enigo,
        facteur: f64,
        glisse_ms: u64,
    ) -> Result<ResultatAbeille> {
        // Un numero l'emporte sur des coordonnees: c'est le chemin sur, et
        // c'est le seul utilisable par un modele sans vision.
        if let Some(n) = args["ref"].as_u64() {
            return Self::geste_ref(action, args, n as usize, enigo, facteur, glisse_ms);
        }
        // Un clic sans coordonnees agit la ou est deja le curseur, ce qui est
        // legitime juste apres un mouse_move ou un hover.
        if action == "fill" {
            return Err(anyhow!(
                "fill needs a 'ref' number from read: it writes into a named control, not                  into a point on screen. To type where the focus already is, use type."
            ));
        }

        let point = match point_demande(args, "") {
            Some((x, y)) => Some(resoudre(x, y)?),
            None => None,
        };

        if let Some((px, py)) = point {
            autoriser_geste(px, py, enigo, facteur)?;
            glisser_vers(enigo, px, py, facteur, glisse_ms);
        } else if action != "scroll" {
            // Sans point explicite, on verifie quand meme la position courante:
            // cliquer a l'aveugle sur une fenetre de LaRuche reste interdit.
            if let Ok((ex, ey)) = enigo.location() {
                autoriser_geste(ex as f64 / facteur, ey as f64 / facteur, enigo, facteur)?;
            }
        }

        let ou = match point {
            Some((px, py)) => format!(" at ({px:.0},{py:.0}) on the desktop"),
            None => " at the current position".to_string(),
        };

        match action {
            "mouse_move" => Ok(ResultatAbeille::ok(format!("Moved the mouse{ou}."))),

            // La paire symetrique de key_down et key_up, qui manquait.
            // `left_click_drag` fait tout d'un bloc, ce qui suffit pour deplacer
            // une icone et pour rien d'autre: dessiner sur un canevas, lire la
            // valeur d'un curseur en cours de course, ou deposer sur une cible
            // qui doit s'ouvrir au survol demandent de garder le bouton enfonce
            // pendant que d'autres appels se produisent.
            "mouse_down" | "mouse_up" => {
                let nom = args["button"].as_str().unwrap_or("left").to_string();
                let bouton = Self::bouton_demande(args)?;
                let sens = if action == "mouse_down" {
                    Direction::Press
                } else {
                    Direction::Release
                };
                enigo
                    .button(bouton, sens)
                    .map_err(|e| anyhow!("button failed: {e}"))?;
                let mut e = etat().lock().unwrap();
                e.boutons_enfonces.retain(|(n, _)| *n != nom);
                if action == "mouse_down" {
                    e.boutons_enfonces.push((nom.clone(), Instant::now()));
                }
                let tenus = e.boutons_enfonces.len();
                drop(e);
                Ok(ResultatAbeille::ok(format!(
                    "{nom} button {}{ou}.{}",
                    if action == "mouse_down" {
                        "pressed"
                    } else {
                        "released"
                    },
                    if tenus > 0 {
                        " Still down. Move with mouse_move, then mouse_up to drop. \
                         It is released automatically after a minute."
                    } else {
                        ""
                    }
                )))
            }

            "left_click" | "right_click" | "middle_click" | "double_click" | "triple_click" => {
                let bouton = match action {
                    "right_click" => Button::Right,
                    "middle_click" => Button::Middle,
                    _ => Button::Left,
                };
                let fois = match action {
                    "double_click" => 2,
                    "triple_click" => 3,
                    _ => 1,
                };
                // Le halo se dessine UNE fois, avant la serie. Il etait redessine
                // entre chaque clic, et `UpdateLayeredWindow` au milieu d'un
                // double clic pouvait pousser les deux appuis au-dela de
                // `GetDoubleClickTime` (500ms par defaut): l'interface voyait
                // alors deux clics simples, ce qui n'ouvre rien.
                if let Ok((cx, cy)) = enigo.location() {
                    decor::curseur(
                        (cx as f64 / facteur) as i32,
                        (cy as f64 / facteur) as i32,
                        true,
                    );
                }
                for _ in 0..fois {
                    enigo
                        .button(bouton, Direction::Click)
                        .map_err(|e| anyhow!("click failed: {e}"))?;
                }
                Ok(ResultatAbeille::ok(format!(
                    "{}{ou}.",
                    match action {
                        "right_click" => "Right click",
                        "middle_click" => "Middle click",
                        "double_click" => "Double click",
                        "triple_click" => "Triple click",
                        _ => "Click",
                    }
                )))
            }
            "left_click_drag" => {
                let (tx, ty) = match point_demande(args, "to") {
                    Some((x, y)) => resoudre(x, y)?,
                    None => {
                        return Err(anyhow!(
                            "left_click_drag needs 'to_x' and 'to_y', the point to drop on. \
                             'x' and 'y' are the point to grab, and default to where the \
                             cursor already is."
                        ))
                    }
                };
                autoriser_geste(tx, ty, enigo, facteur)?;
                let bouton = Self::bouton_demande(args)?;
                let pose = args["hold_ms"].as_u64().unwrap_or(120).min(5000);
                Self::glisser_bouton(enigo, bouton, tx, ty, facteur, pose)?;
                Ok(ResultatAbeille::ok(format!(
                    "Dragged to ({tx:.0},{ty:.0}) on the desktop."
                )))
            }
            "scroll" => {
                let direction = args["direction"].as_str().unwrap_or("down");
                let crans = args["amount"].as_i64().unwrap_or(3).clamp(1, 50) as i32;
                let (axe, signe) = match direction {
                    "down" => (Axis::Vertical, 1),
                    "up" => (Axis::Vertical, -1),
                    "right" => (Axis::Horizontal, 1),
                    "left" => (Axis::Horizontal, -1),
                    _ => {
                        return Err(anyhow!(
                            "scroll direction must be up, down, left or right."
                        ))
                    }
                };
                enigo
                    .scroll(crans * signe, axe)
                    .map_err(|e| anyhow!("scroll failed: {e}"))?;
                Ok(ResultatAbeille::ok(format!(
                    "Scrolled {direction} {crans} notch(es){ou}."
                )))
            }
            _ => unreachable!("action filtree en amont"),
        }
    }

    fn geste_clavier(action: &str, args: &Value, enigo: &mut Enigo) -> Result<ResultatAbeille> {
        let spec = args["key"]
            .as_str()
            .or_else(|| args["text"].as_str())
            .ok_or_else(|| {
                anyhow!(
                    "key needs a 'key', for instance \"Enter\", \"Escape\", \"F5\" or \
                     \"Control+c\"."
                )
            })?;
        let (mods, touche) = parse_touche(spec).ok_or_else(|| {
            anyhow!(
                "Unrecognised key '{spec}'. Use a single character, a named key (Enter, Tab, \
                 Escape, Backspace, Delete, Space, Home, End, PageUp, PageDown, Up, Down, \
                 Left, Right, F1 to F12), optionally prefixed with Control+, Shift+, Alt+ \
                 or Meta+."
            )
        })?;

        // key_down et key_up existent pour ce que key ne peut pas faire: tenir
        // une touche pendant que d'autres gestes se produisent, ce dont un jeu
        // ou un raccourci de glisser-deposer a besoin.
        if action != "key" {
            let sens = if action == "key_down" {
                Direction::Press
            } else {
                Direction::Release
            };
            for m in &mods {
                enigo
                    .key(*m, sens)
                    .map_err(|e| anyhow!("modifier failed: {e}"))?;
            }
            enigo
                .key(touche, sens)
                .map_err(|e| anyhow!("key failed: {e}"))?;
            // Le registre, pour que l'oubli cesse d'etre a la charge du modele.
            {
                let mut e = etat().lock().unwrap();
                e.touches_enfoncees.retain(|(n, _)| n != spec);
                if action == "key_down" {
                    e.touches_enfoncees.push((spec.to_string(), Instant::now()));
                }
            }
            let encore = etat().lock().unwrap().touches_enfoncees.len();
            return Ok(ResultatAbeille::ok(format!(
                "{spec} {}.{}",
                if action == "key_down" {
                    "held down"
                } else {
                    "released"
                },
                if encore > 0 {
                    format!(
                        " Still held: {}. Release with key_up, or release_all. \
                         Anything still down after a minute is released automatically, \
                         because a key left down poisons the user's own machine too.",
                        etat()
                            .lock()
                            .unwrap()
                            .touches_enfoncees
                            .iter()
                            .map(|(n, _)| n.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                } else {
                    String::new()
                }
            )));
        }

        let repeat = args["repeat"].as_u64().unwrap_or(1).clamp(1, 50);
        let maintien = args["hold_ms"].as_u64().unwrap_or(0).min(MAINTIEN_MAX_MS);
        // Un raccourci part vers la fenetre au premier plan, pas vers celle que
        // le modele a en tete. Un Ctrl+S mal adresse enregistre la mauvaise
        // chose, et c'est arrive.
        let ou = ou_part_la_frappe();

        for m in &mods {
            enigo
                .key(*m, Direction::Press)
                .map_err(|e| anyhow!("modifier failed: {e}"))?;
        }
        let mut echec = None;
        for _ in 0..repeat {
            if maintien > 0 {
                if let Err(e) = enigo.key(touche, Direction::Press) {
                    echec = Some(anyhow!("key failed: {e}"));
                    break;
                }
                std::thread::sleep(Duration::from_millis(maintien));
                let _ = enigo.key(touche, Direction::Release);
            } else if let Err(e) = enigo.key(touche, Direction::Click) {
                echec = Some(anyhow!("key failed: {e}"));
                break;
            }
        }
        // Les modificateurs sont relaches quoi qu'il arrive, y compris apres un
        // echec: c'est la seule chose qu'on ne peut pas se permettre de rater.
        for m in mods.iter().rev() {
            let _ = enigo.key(*m, Direction::Release);
        }
        if let Some(e) = echec {
            return Err(e);
        }
        Ok(ResultatAbeille::ok(format!(
            "Pressed {spec}{}{}.{ou}",
            if repeat > 1 {
                format!(" x{repeat}")
            } else {
                String::new()
            },
            if maintien > 0 {
                format!(", held {maintien}ms")
            } else {
                String::new()
            }
        )))
    }
}

fn base64_png(octets: Vec<u8>) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(octets)
}

#[async_trait]
impl Abeille for Ordinateur {
    fn nom(&self) -> &str {
        "computer"
    }

    fn description(&self) -> &str {
        "Drive the machine itself: mouse, keyboard and screen, outside the browser. Use it \
         for desktop applications, installers, native dialogs, anything that is not a web \
         page. For a web page prefer the `browser` tool, which reads the DOM and clicks by \
         element number: it is faster, cheaper and does not miss. \
         Actions: windows, focus_window, read, find, focus, screens, screenshot, \
         cursor_position, mouse_move, left_click, right_click, middle_click, double_click, \
         triple_click, left_click_drag, mouse_down, mouse_up, scroll, fill, type, key, \
         key_down, key_up, release_all, wait, read_clipboard, write_clipboard, move_window, minimize_window, maximize_window, restore_window, close_window. \
         THE FAST PATH NEEDS NO SCREENSHOT AT ALL, and it is the only one that works without \
         vision: windows lists what is open, focus_window brings one to the front, read \
         returns a NUMBERED map of its controls (ref_1 <button> OK), and click or fill act \
         on those numbers. Acting by ref calls the control through the OS accessibility API, \
         so it is deterministic, it cannot miss, and it does not move the mouse. Refs are \
         renumbered by every read and lost when the window changes, so read again after \
         acting. read works on Windows today. \
         Fall back to pixels only when read finds nothing, which happens with hand-drawn \
         interfaces (games, canvas, poorly tagged Electron). Then take a screenshot, and note \
         that COORDINATES ARE ALWAYS PIXELS OF THAT SCREENSHOT, never desktop pixels. The \
         tool converts, including on scaled and mixed-DPI displays. Multi-monitor: screens \
         lists them, screenshot takes `screen`. Two things it refuses by \
         design: acting on LaRuche's own windows, and continuing once the human has moved \
         the mouse."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["windows", "focus_window", "read", "find", "focus", "screens",
                             "screenshot", "cursor_position", "mouse_move", "left_click",
                             "right_click", "middle_click", "double_click", "triple_click",
                             "left_click_drag", "mouse_down", "mouse_up", "scroll", "fill",
                             "type", "key", "key_down", "key_up", "release_all", "wait",
                             "read_clipboard", "write_clipboard", "move_window",
                             "minimize_window", "maximize_window", "restore_window",
                             "close_window"],
                    "description": "windows: list the open windows. focus_window: bring one to the front by a piece of its title. read: numbered map of the controls of the front window (or of `window`), the no-screenshot path. find: the same map filtered to controls whose label contains `text`, for a window too big to read whole. focus: give focus to a ref without acting on it. screens: list the monitors. screenshot: capture one, and set the coordinate system for everything after it. cursor_position: where the pointer is. mouse_move: move onto a ref or a point WITHOUT clicking, which is what opens hover menus. left/right/middle/double/triple_click: act on a ref, or click at x,y, or click where the cursor already is. left_click_drag: grab (a ref, or x,y) and release at to_x,to_y, holding briefly at each end so HTML5 and Electron drops register. mouse_down/mouse_up: hold a button across other calls, for drawing, for reading a slider mid-drag, and for drops that need the pointer to dwell. scroll: on a ref, brings it into view through its own pattern, the only way to reach an off-screen control without a screenshot; otherwise wheel notches at a point. fill: write into a ref, including a number into a slider or a spinner. type: type text into whatever has focus. key: press a key or a chord. key_down/key_up: hold a key across other gestures. release_all: release every key and button still held. wait: pause while the interface catches up. read_clipboard/write_clipboard: text in and out, which is how you move more than a few lines. move_window places a window by x, y, width, height, any subset. minimize_window, maximize_window, restore_window and close_window take `window` like focus_window; close_window is the cross, not a kill, so the app may prompt or refuse."
                },
                "ref": { "type": "integer", "description": "Element number from read, without the ref_ prefix. Preferred over x,y: it cannot miss, and it is the only path that works without vision." },
                "window": { "type": "string", "description": "For read and focus_window: a piece of the window title. read defaults to the front window." },
                "x": { "type": "number", "description": "Horizontal position, in pixels of the LAST screenshot. Only needed when read finds no control to act on. `coordinate: [x, y]` is accepted instead." },
                "y": { "type": "number", "description": "Vertical position, in pixels of the LAST screenshot" },
                "to_x": { "type": "number", "description": "For left_click_drag: where to release" },
                "to_y": { "type": "number", "description": "For left_click_drag: where to release" },
                "button": { "type": "string", "enum": ["left", "right", "middle"], "description": "For mouse_down, mouse_up and left_click_drag: which button. Default left. Right and middle drags are how CAD and 3D tools orbit and pan." },
                "ms": { "type": "integer", "description": "For wait: how long to pause, 50 to 30000, default 1000" },
                "width": { "type": "integer", "description": "For move_window: new width in desktop pixels" },
                "height": { "type": "integer", "description": "For move_window: new height in desktop pixels" },
                "screen": { "description": "For screenshot: screen id, rank (1 = first) or name. Default: the primary screen." },
                "max_width": { "type": "integer", "description": "For screenshot: width of the returned image, default 1280. Larger is more legible and more expensive." },
                "text": { "type": "string", "description": "For type, fill and write_clipboard: the text to write. For fill on a slider or a spinner: the number to set. For find: the wording to look for." },
                "key": { "type": "string", "description": "For key, key_down, key_up: Enter, Tab, Escape, Backspace, Delete, Space, Home, End, PageUp, PageDown, Up, Down, Left, Right, F1 to F12, or a single character. Chord with Control+, Shift+, Alt+, Meta+." },
                "repeat": { "type": "integer", "description": "For key: press it this many times, default 1, max 50" },
                "hold_ms": { "type": "integer", "description": "For key: hold it down this long each press, default 0" },
                "direction": { "type": "string", "enum": ["up", "down", "left", "right"], "description": "For scroll, default down" },
                "amount": { "type": "integer", "description": "For scroll: wheel notches, default 3" },
                "glow": { "type": "boolean", "description": "Amber frame on the screen being driven, plus a floating panel naming each action and a ring following the cursor. Default true. Set false for a clean screenshot." },
                "animate": { "type": "boolean", "description": "Glide the cursor to its target instead of jumping, so a human can follow. Default true; needs glow on. Set false to act instantly on a long sequence." },
                "speed": { "type": "number", "description": "Animation time multiplier, default 1. Above 1 slows the cursor down so it is easier to watch." }
            },
            "required": ["action"]
        })
    }

    fn niveau_danger(&self) -> NiveauDanger {
        // Le niveau est uniforme parce que le trait ne voit pas les arguments;
        // la granularite reelle vient de `cle_pattern`, qui classe l'approbation
        // par action, donc approuver une capture n'approuve pas un clic.
        NiveauDanger::NeedsApproval
    }

    async fn executer(&self, args: Value, _ctx: &ContextExecution) -> Result<ResultatAbeille> {
        if std::env::var("LARUCHE_COMPUTER").as_deref() == Ok("0") {
            return Ok(ResultatAbeille::err(
                "GUI control is disabled on this node (LARUCHE_COMPUTER=0).".to_string(),
            ));
        }
        match tokio::task::spawn_blocking(move || Ordinateur::executer_bloquant(args)).await {
            Ok(Ok(r)) => Ok(r),
            Ok(Err(e)) => Ok(ResultatAbeille::err(e.to_string())),
            Err(e) => Ok(ResultatAbeille::err(format!("GUI task failed: {e}"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn une_fenetre_a_cheval_appartient_a_lecran_qui_en_montre_le_plus() {
        // Deux 1920x1080 cote a cote, le second commencant a x=1920.
        let ecrans = [(0, 0, 1920, 1080), (1920, 0, 1920, 1080)];

        // Franchement sur le premier, puis franchement sur le second.
        assert_eq!(ecran_du_rect(&ecrans, 100, 100, 800, 600), Some(1));
        assert_eq!(ecran_du_rect(&ecrans, 2200, 100, 800, 600), Some(2));

        // A cheval, 90% a droite. Le coin haut gauche est sur l'ecran 1 et
        // repondre 1 serait le bug qu'on evite: l'humain voit cette fenetre
        // sur l'ecran 2, et c'est celui-la qu'il faut capturer.
        assert_eq!(
            ecran_du_rect(&ecrans, 1820, 100, 1000, 600),
            Some(2),
            "le recouvrement doit l'emporter sur le coin haut gauche"
        );

        // Exactement a la frontiere, majorite a gauche.
        assert_eq!(ecran_du_rect(&ecrans, 1520, 100, 800, 600), Some(1));

        // Entierement hors de tout moniteur: on ne devine pas.
        assert_eq!(ecran_du_rect(&ecrans, 5000, 5000, 200, 200), None);

        // Un seul ecran: tout y tombe, et jamais None pour une fenetre visible.
        assert_eq!(ecran_du_rect(&[(0, 0, 1920, 1080)], 0, 0, 100, 100), Some(1));
    }

    #[test]
    fn le_cadre_convertit_dans_les_deux_sens() {
        // Un 4K reduit a 1280 de large, sur le deuxieme ecran d'un montage.
        let c = Cadre {
            ecran: 1,
            origine_x: 1920,
            origine_y: 0,
            physique_l: 3840,
            physique_h: 2160,
            image_l: 1280,
            image_h: 720,
        };
        // Le centre de l'image est le centre physique de CET ecran, decale de
        // son origine dans le bureau virtuel.
        let (px, py) = c.vers_physique(640.0, 360.0);
        assert!((px - (1920.0 + 1920.0)).abs() < 0.5, "x physique: {px}");
        assert!((py - 1080.0).abs() < 0.5, "y physique: {py}");
        // Et le retour retombe sur ses pieds.
        let (ix, iy) = c.vers_image(px, py);
        assert!((ix - 640.0).abs() < 0.5, "x image: {ix}");
        assert!((iy - 360.0).abs() < 0.5, "y image: {iy}");
    }

    #[test]
    fn le_coin_haut_gauche_reste_le_coin_haut_gauche() {
        let c = Cadre {
            ecran: 0,
            origine_x: -1080,
            origine_y: -200,
            physique_l: 1080,
            physique_h: 1920,
            image_l: 540,
            image_h: 960,
        };
        assert_eq!(c.vers_physique(0.0, 0.0), (-1080.0, -200.0));
    }

    #[test]
    fn touches_nommees_et_accords() {
        use enigo::Key;
        assert!(matches!(parse_touche("Enter"), Some((m, Key::Return)) if m.is_empty()));
        // La casse ne doit pas compter: un modele ecrit les deux.
        assert!(matches!(parse_touche("escape"), Some((_, Key::Escape))));
        assert!(matches!(parse_touche("F5"), Some((_, Key::F5))));
        assert!(matches!(parse_touche("Up"), Some((_, Key::UpArrow))));

        let (mods, touche) = parse_touche("Control+Shift+t").expect("accord");
        assert_eq!(mods.len(), 2);
        assert!(matches!(touche, Key::Unicode('t')));

        // Une faute de frappe doit echouer, pas presser autre chose.
        assert!(parse_touche("Ctrl+Nope").is_none());
        assert!(parse_touche("").is_none());
    }

    /// Sans capture prealable, il n'y a pas de systeme de coordonnees partage.
    /// Interpreter les nombres comme des pixels physiques marcherait sur un
    /// montage simple et raterait partout ailleurs, donc on refuse.
    #[test]
    fn pointer_sans_capture_est_refuse() {
        etat().lock().unwrap().cadre = None;
        let e = resoudre(10.0, 10.0).expect_err("doit refuser");
        assert!(e.to_string().contains("screenshot first"), "{e}");
    }

    /// Bout en bout en LECTURE SEULE sur la machine reelle: enumerer, capturer,
    /// lire le curseur. Ne touche a rien, ne bouge rien. Ignore par defaut, une
    /// machine de CI n'ayant pas d'ecran.
    ///
    ///   cargo test -p laruche-essaim --lib ordinateur_lecture -- --ignored --nocapture
    #[tokio::test]
    #[ignore = "requires a real display"]
    async fn ordinateur_lecture_seule() {
        let ctx = ContextExecution::default();

        let out = Ordinateur
            .executer(json!({ "action": "screens" }), &ctx)
            .await
            .unwrap();
        assert!(out.success, "screens: {:?}", out.error);
        assert!(out.output.contains("screen 1"), "{}", out.output);
        println!("{}", out.output);

        let out = Ordinateur
            .executer(json!({ "action": "screenshot" }), &ctx)
            .await
            .unwrap();
        assert!(out.success, "screenshot: {:?}", out.error);
        assert_eq!(out.images.len(), 1, "aucune image rendue au modele");
        assert!(out.images[0].len() > 5000, "image suspectement petite");
        println!("{}\nbase64: {} octets", out.output, out.images[0].len());

        // La capture doit avoir pose un systeme de coordonnees utilisable.
        let out = Ordinateur
            .executer(json!({ "action": "cursor_position" }), &ctx)
            .await
            .unwrap();
        assert!(out.success, "cursor_position: {:?}", out.error);
        assert!(
            out.output.contains("in the last screenshot"),
            "le cadre n'a pas ete pose: {}",
            out.output
        );
        println!("{}", out.output);
    }

    /// Bout en bout AVEC action: bouge reellement la souris de l'utilisateur.
    /// A lancer sciemment, jamais dans une suite automatique.
    ///
    ///   cargo test -p laruche-essaim --lib ordinateur_geste -- --ignored --nocapture
    #[tokio::test]
    #[ignore = "moves the real mouse pointer"]
    async fn ordinateur_geste_reel() {
        let ctx = ContextExecution::default();
        Ordinateur
            .executer(json!({ "action": "screenshot" }), &ctx)
            .await
            .unwrap();

        // Vise le centre de l'image, donc le centre de l'ecran, quel que soit
        // le facteur d'echelle: c'est tout le contrat de coordonnees.
        let cadre = etat().lock().unwrap().cadre.expect("capture faite");
        let (cx, cy) = (cadre.image_l as f64 / 2.0, cadre.image_h as f64 / 2.0);
        let out = Ordinateur
            .executer(json!({ "action": "mouse_move", "x": cx, "y": cy }), &ctx)
            .await
            .unwrap();
        assert!(out.success, "mouse_move: {:?}", out.error);

        let out = Ordinateur
            .executer(json!({ "action": "cursor_position" }), &ctx)
            .await
            .unwrap();
        println!("{}", out.output);
        // Le curseur doit avoir atterri la ou on l'a envoye, en coordonnees
        // IMAGE. Un ecart important signerait une calibration ratee, ce qui est
        // exactement le bug que pyautogui produit sur un ecran mis a l'echelle.
        let lu = out
            .output
            .split_once('(')
            .and_then(|(_, r)| r.split_once(')'))
            .map(|(v, _)| v.to_string())
            .expect("position lisible");
        let (lx, ly) = lu.split_once(',').expect("x,y");
        let (lx, ly): (f64, f64) = (lx.trim().parse().unwrap(), ly.trim().parse().unwrap());
        assert!(
            (lx - cx).abs() < 4.0 && (ly - cy).abs() < 4.0,
            "vise ({cx:.0},{cy:.0}), atterri ({lx:.0},{ly:.0}): calibration fausse"
        );
    }

    /// Montre le decor et le photographie, pour qu'on puisse le REGARDER.
    /// Ecrit la capture dans le repertoire temporaire et affiche son chemin.
    ///
    ///   cargo test -p laruche-essaim --lib halo_visuel -- --ignored --nocapture
    #[tokio::test]
    #[ignore = "moves the real mouse and paints on the real screen"]
    async fn halo_visuel() {
        use base64::Engine;
        let ctx = ContextExecution::default();

        // Une premiere capture pose le cadre et allume le decor.
        Ordinateur
            .executer(json!({ "action": "screenshot" }), &ctx)
            .await
            .unwrap();
        let cadre = etat().lock().unwrap().cadre.expect("capture faite");

        // Quelques gestes pour remplir le panneau et promener l'anneau.
        for (x, y) in [(0.25, 0.35), (0.6, 0.55), (0.45, 0.7)] {
            let out = Ordinateur
                .executer(
                    json!({ "action": "mouse_move",
                            "x": cadre.image_l as f64 * x,
                            "y": cadre.image_h as f64 * y }),
                    &ctx,
                )
                .await
                .unwrap();
            assert!(out.success, "mouse_move: {:?}", out.error);
        }

        // Puis on photographie le decor lui-meme. glow reste actif pour qu'il
        // apparaisse dans l'image: c'est tout l'objet du test.
        let out = Ordinateur
            .executer(json!({ "action": "screenshot", "max_width": 1600 }), &ctx)
            .await
            .unwrap();
        assert!(out.success, "screenshot: {:?}", out.error);
        let octets = base64::engine::general_purpose::STANDARD
            .decode(&out.images[0])
            .expect("png valide");
        let chemin = std::env::temp_dir().join("laruche-halo.png");
        std::fs::write(&chemin, octets).expect("ecriture");
        println!("HALO ECRIT: {}", chemin.display());
    }

    /// Le parcours SANS VISION, de bout en bout, sur une vraie application:
    /// lister les fenetres, en adopter une, lire son arbre, ecrire dedans par
    /// numero, verifier que le texte a atterri. Aucune capture d'ecran n'entre
    /// dans cette boucle, ce qui est tout l'objet du chemin par l'arbre.
    ///
    ///   cargo test -p laruche-essaim --lib sans_vision -- --ignored --nocapture
    #[tokio::test]
    #[ignore = "opens Notepad and types into it"]
    async fn parcours_sans_vision() {
        let ctx = ContextExecution::default();
        let temoin = format!("laruche {}", std::process::id());

        std::process::Command::new("notepad.exe")
            .spawn()
            .expect("notepad");
        tokio::time::sleep(Duration::from_millis(1500)).await;

        let out = Ordinateur
            .executer(json!({ "action": "windows" }), &ctx)
            .await
            .unwrap();
        assert!(out.success, "windows: {:?}", out.error);
        println!("{}", out.output);

        let out = Ordinateur
            .executer(
                json!({ "action": "focus_window", "window": "Bloc-notes" }),
                &ctx,
            )
            .await
            .unwrap();
        // Le titre depend de la langue du systeme: on retente en anglais.
        let out = if out.success {
            out
        } else {
            Ordinateur
                .executer(json!({ "action": "focus_window", "window": "Notepad" }), &ctx)
                .await
                .unwrap()
        };
        assert!(out.success, "focus_window: {:?}", out.error);
        println!("{}", out.output);

        let out = Ordinateur
            .executer(json!({ "action": "read" }), &ctx)
            .await
            .unwrap();
        assert!(out.success, "read: {:?}", out.error);
        println!("{}", out.output.chars().take(900).collect::<String>());

        // La zone de saisie est le seul <input> de la fenetre.
        let numero = out
            .output
            .lines()
            .find(|l| l.contains("<input>") || l.contains("<document>"))
            .and_then(|l| l.split_whitespace().next())
            .and_then(|r| r.strip_prefix("ref_"))
            .and_then(|n| n.parse::<u64>().ok())
            .expect("Notepad expose un champ de saisie");

        let out = Ordinateur
            .executer(
                json!({ "action": "fill", "ref": numero, "text": temoin }),
                &ctx,
            )
            .await
            .unwrap();
        assert!(out.success, "fill: {:?}", out.error);
        println!("{}", out.output);

        // Relire l'arbre doit montrer le texte, ce qui prouve que la boucle
        // complete tourne sans qu'une seule image ait ete regardee.
        tokio::time::sleep(Duration::from_millis(400)).await;
        let out = Ordinateur
            .executer(json!({ "action": "read" }), &ctx)
            .await
            .unwrap();
        assert!(
            out.output.contains(&temoin),
            "le texte ecrit ne se relit pas dans l'arbre:
{}",
            out.output.chars().take(600).collect::<String>()
        );
        println!("Texte relu dans l'arbre: ok");
    }

    #[test]
    fn les_synonymes_devidence_sont_acceptes() {
        // `click` est le nom que porte la meme action sur l'outil `browser`, et
        // un modele passe de l'un a l'autre dans la meme conversation.
        assert_eq!(alias_action("click"), "left_click");
        assert_eq!(alias_action("press"), "key");
        assert_eq!(alias_action("drag"), "left_click_drag");
        assert_eq!(alias_action("list_windows"), "windows");
        // Ce qui est deja le bon nom n'est pas touche.
        assert_eq!(alias_action("left_click"), "left_click");
        assert_eq!(alias_action("read"), "read");
        // Et l'inconnu reste inconnu, pour que le message d'erreur le nomme.
        assert_eq!(alias_action("teleporte"), "teleporte");
    }

    /// `coordinate: [x, y]` est la convention des outils de controle d'ecran les
    /// plus repandus. L'ignorer faisait cliquer la ou le curseur se trouvait
    /// deja: l'action reussissait, au mauvais endroit, sans un mot.
    #[test]
    fn le_point_se_lit_dans_les_deux_ecritures() {
        assert_eq!(
            point_demande(&json!({ "x": 12, "y": 34 }), ""),
            Some((12.0, 34.0))
        );
        assert_eq!(
            point_demande(&json!({ "coordinate": [12, 34] }), ""),
            Some((12.0, 34.0))
        );
        assert_eq!(
            point_demande(&json!({ "coordinates": [12, 34] }), ""),
            Some((12.0, 34.0))
        );
        // Le prefixe sert au point d'arrivee d'un glisser.
        assert_eq!(
            point_demande(&json!({ "to_coordinate": [7, 8] }), "to"),
            Some((7.0, 8.0))
        );
        assert_eq!(
            point_demande(&json!({ "to_x": 7, "to_y": 8 }), "to"),
            Some((7.0, 8.0))
        );
        // x sans y n'est pas un point: mieux vaut rien qu'a moitie.
        assert_eq!(point_demande(&json!({ "x": 12 }), ""), None);
        assert_eq!(point_demande(&json!({ "coordinate": [12] }), ""), None);
        assert_eq!(point_demande(&json!({}), ""), None);
        // Et le prefixe ne va pas se servir dans le point de depart.
        assert_eq!(point_demande(&json!({ "x": 1, "y": 2 }), "to"), None);
    }

    /// Le message d'erreur est le seul endroit ou un modele apprend la liste des
    /// actions quand il s'est trompe. Il etait reste sur l'ancienne, donc il
    /// renvoyait vers les pixels une conversation qui aurait du passer par
    /// l'arbre d'accessibilite.
    #[tokio::test]
    async fn laction_inconnue_nomme_dabord_le_chemin_fiable() {
        let out = Ordinateur
            .executer(json!({ "action": "teleporte" }), &ContextExecution::default())
            .await
            .unwrap();
        assert!(!out.success);
        let message = out.error.unwrap_or_default();
        for attendu in ["windows", "focus_window", "read", "fill", "focus"] {
            assert!(
                message.contains(attendu),
                "`{attendu}` absent du message d'erreur: {message}"
            );
        }
    }

    #[test]
    fn le_schema_et_la_description_couvrent_toutes_les_actions() {
        // On parcourt le schema LUI-MEME plutot qu'une liste recopiee a cote.
        // La liste en dur ne gardait que le passe: les actions ajoutees ensuite
        // n'etaient verifiees par personne, et une action absente de la
        // description est une action que le modele n'utilisera jamais.
        let s = Ordinateur.schema();
        let actions = s["properties"]["action"]["enum"].as_array().unwrap();
        assert!(actions.len() >= 26, "le schema a maigri sans raison");
        for a in actions {
            let a = a.as_str().unwrap();
            assert!(
                Ordinateur.description().contains(a),
                "action {a} absente de la description"
            );
        }
    }
}
