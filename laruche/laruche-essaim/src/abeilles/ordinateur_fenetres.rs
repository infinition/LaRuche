//! Les fenetres elles-memes: les deplacer, les redimensionner, les reduire,
//! les restaurer, les fermer. Et savoir laquelle est hors d'atteinte.
//!
//! `windows` savait deja LISTER, avec la taille et la position, mais rien ne
//! pouvait agir dessus. Deux consequences, et la seconde est un cul-de-sac:
//!
//!   - une fenetre a moitie hors ecran, ou trop petite pour montrer son bouton,
//!     ne pouvait qu'etre subie. Le seul recours etait de viser la barre de
//!     titre et de glisser, ce qui suppose une capture, donc de sortir du chemin
//!     sans vision pour une raison purement mecanique;
//!   - une fenetre reduite etait invisible ET irrecuperable. `windows` filtrait
//!     les minimisees, donc l'agent ne pouvait meme pas savoir que l'application
//!     qu'on lui demandait existait, et aucune action ne la faisait revenir.
//!
//! # L'elevation
//!
//! Un processus non eleve ne peut RIEN envoyer a une fenetre elevee: Windows
//! filtre les entrees synthetiques par niveau d'integrite (UIPI), en silence.
//! Le clic part, ne fait rien, et ne rend aucune erreur. Vu du modele, c'est
//! indiscernable d'un clic mal vise, donc il recommence, indefiniment.
//!
//! On ne peut pas contourner ca, et il ne faut pas: c'est la frontiere de
//! privileges qui protege la machine. Ce qu'on peut faire, c'est le DIRE, ce
//! qui transforme une boucle infinie en une phrase que l'utilisateur comprend.

use anyhow::{anyhow, Result};
use windows::core::BOOL;
use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM, RECT, WPARAM};
use windows::Win32::Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
use windows::Win32::System::Threading::{
    GetCurrentProcess, OpenProcess, OpenProcessToken, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindowRect, GetWindowTextW, GetWindowThreadProcessId, IsIconic,
    IsWindowVisible, IsZoomed, PostMessageW, SetForegroundWindow, SetWindowPos, ShowWindow,
    HWND_NOTOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SW_MAXIMIZE, SW_MINIMIZE,
    SW_RESTORE, WM_CLOSE,
};

/// Ce qu'on sait d'une fenetre sans avoir a la toucher.
pub struct Fenetre {
    pub hwnd: HWND,
    pub titre: String,
    pub pid: u32,
    pub reduite: bool,
    pub agrandie: bool,
    pub rect: (i32, i32, i32, i32),
}

/// Toutes les fenetres visibles, minimisees COMPRISES.
///
/// C'est la difference avec la liste que rend `windows`, qui les ecarte: une
/// fenetre reduite est exactement celle qu'on veut pouvoir restaurer, et la
/// cacher rendait l'operation impossible a formuler.
pub fn lister() -> Result<Vec<Fenetre>> {
    // `EnumWindows` rappelle une fonction par fenetre; on accumule dans un Vec
    // dont on passe l'adresse en LPARAM, ce qui est le contrat de l'API.
    unsafe extern "system" fn visiter(hwnd: HWND, param: LPARAM) -> BOOL {
        let recueil = &mut *(param.0 as *mut Vec<Fenetre>);
        if !unsafe { IsWindowVisible(hwnd) }.as_bool() {
            return true.into();
        }
        let mut tampon = [0u16; 512];
        let n = unsafe { GetWindowTextW(hwnd, &mut tampon) };
        let titre = String::from_utf16_lossy(&tampon[..n as usize]);
        // Sans titre, ce n'est pas quelque chose que l'agent peut viser: ce sont
        // les fenetres techniques du compositeur, et elles sont nombreuses.
        if titre.trim().is_empty() {
            return true.into();
        }
        let mut pid = 0u32;
        unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
        let mut r = RECT::default();
        let rect = if unsafe { GetWindowRect(hwnd, &mut r) }.is_ok() {
            (r.left, r.top, r.right, r.bottom)
        } else {
            (0, 0, 0, 0)
        };
        recueil.push(Fenetre {
            hwnd,
            titre,
            pid,
            reduite: unsafe { IsIconic(hwnd) }.as_bool(),
            agrandie: unsafe { IsZoomed(hwnd) }.as_bool(),
            rect,
        });
        true.into()
    }

    let mut recueil: Vec<Fenetre> = Vec::new();
    unsafe {
        EnumWindows(
            Some(visiter),
            LPARAM(&mut recueil as *mut Vec<Fenetre> as isize),
        )
    }
    .map_err(|e| anyhow!("cannot enumerate windows: {e}"))?;
    Ok(recueil)
}

/// La fenetre dont le titre contient `filtre`, sans tenir compte de la casse.
///
/// Une correspondance exacte l'emporte sur une correspondance partielle: sinon
/// "Notepad" tombait sur "Notepad++" ou sur un explorateur ouvert dans le
/// dossier notepad, au hasard de l'ordre d'enumeration.
pub fn trouver(filtre: &str) -> Result<Fenetre> {
    let besoin = filtre.trim().to_lowercase();
    if besoin.is_empty() {
        return Err(anyhow!("which window? Pass a piece of its title."));
    }
    let toutes = lister()?;
    let mut partielles: Vec<Fenetre> = Vec::new();
    for f in toutes {
        let t = f.titre.to_lowercase();
        if t == besoin {
            return Ok(f);
        }
        if t.contains(&besoin) {
            partielles.push(f);
        }
    }
    match partielles.len() {
        0 => Err(anyhow!(
            "No window whose title contains \"{filtre}\". Call windows to see what is open; \
             the one you want may be minimised, which still lists but is not on screen."
        )),
        1 => Ok(partielles.remove(0)),
        _ => {
            // Plusieurs candidates: nommer les titres plutot que d'en choisir
            // une. Se tromper de fenetre est silencieux et cher.
            let noms: Vec<&str> = partielles.iter().map(|f| f.titre.as_str()).collect();
            Err(anyhow!(
                "\"{filtre}\" matches {} windows: {}. Pass more of the title.",
                noms.len(),
                noms.join(" | ")
            ))
        }
    }
}

/// Ce processus tourne-t-il avec des privileges eleves?
fn eleve(pid: u32) -> Option<bool> {
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut jeton = Default::default();
        let ouvert = OpenProcessToken(handle, TOKEN_QUERY, &mut jeton).is_ok();
        if !ouvert {
            let _ = CloseHandle(handle);
            return None;
        }
        let mut info = TOKEN_ELEVATION::default();
        let mut taille = 0u32;
        let ok = GetTokenInformation(
            jeton,
            TokenElevation,
            Some(&mut info as *mut _ as *mut _),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut taille,
        )
        .is_ok();
        let _ = CloseHandle(jeton);
        let _ = CloseHandle(handle);
        ok.then_some(info.TokenIsElevated != 0)
    }
}

/// Nous-memes.
fn nous_sommes_eleves() -> bool {
    unsafe {
        let mut jeton = Default::default();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut jeton).is_err() {
            return false;
        }
        let mut info = TOKEN_ELEVATION::default();
        let mut taille = 0u32;
        let ok = GetTokenInformation(
            jeton,
            TokenElevation,
            Some(&mut info as *mut _ as *mut _),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut taille,
        )
        .is_ok();
        let _ = CloseHandle(jeton);
        ok && info.TokenIsElevated != 0
    }
}

/// La phrase a dire quand la cible est hors d'atteinte, ou rien.
///
/// Rendue AVANT d'agir, jamais apres: l'interet est precisement d'eviter le
/// geste qui ne fera rien et la boucle qui suit.
pub fn hors_datteinte(pid: u32) -> Option<String> {
    // Un processus eleve pilote par un processus eleve ne pose aucun probleme;
    // c'est l'inverse qui est bloque.
    if nous_sommes_eleves() {
        return None;
    }
    match eleve(pid) {
        Some(true) => Some(
            "that window belongs to a process running as administrator, and LaRuche is not. \
             Windows blocks synthetic input across that boundary (UIPI), silently: the click \
             would be sent, do nothing, and return no error. This cannot be worked around \
             from here, and it should not be. Tell the user that this particular window has \
             to be driven by hand, or that LaRuche has to be restarted as administrator."
                .to_string(),
        ),
        // `None` veut dire qu'on n'a pas pu ouvrir le processus, ce qui est
        // deja, presque toujours, le signe d'un privilege superieur. On le dit
        // plus prudemment, sans l'affirmer.
        None => Some(
            "that window's process could not be inspected, which usually means it runs at a \
             higher privilege level than LaRuche. If the gesture appears to do nothing, that \
             is why, and the window has to be driven by hand."
                .to_string(),
        ),
        Some(false) => None,
    }
}

/// Amene une fenetre au premier plan, en la restaurant si elle etait reduite.
pub fn activer(f: &Fenetre) -> Result<()> {
    if f.reduite {
        let _ = unsafe { ShowWindow(f.hwnd, SW_RESTORE) };
    }
    // `SetForegroundWindow` echoue quand le processus appelant n'a pas le droit
    // de voler le focus, ce qui arrive et n'est pas fatal: la fenetre est
    // restauree, elle clignote dans la barre des taches. On ne transforme pas ca
    // en erreur, on le laisse au rapport.
    let _ = unsafe { SetForegroundWindow(f.hwnd) };
    Ok(())
}

/// Reduire, agrandir, restaurer.
pub fn etat(f: &Fenetre, quoi: &str) -> Result<&'static str> {
    let commande = match quoi {
        "minimize" => SW_MINIMIZE,
        "maximize" => SW_MAXIMIZE,
        "restore" => SW_RESTORE,
        autre => return Err(anyhow!("unknown window state '{autre}'.")),
    };
    let _ = unsafe { ShowWindow(f.hwnd, commande) };
    Ok(match quoi {
        "minimize" => "minimised",
        "maximize" => "maximised",
        _ => "restored",
    })
}

/// Deplace et redimensionne, chacun des deux etant facultatif.
pub fn poser(
    f: &Fenetre,
    x: Option<i32>,
    y: Option<i32>,
    largeur: Option<i32>,
    hauteur: Option<i32>,
) -> Result<()> {
    if x.is_none() && y.is_none() && largeur.is_none() && hauteur.is_none() {
        return Err(anyhow!(
            "move_window needs at least one of x, y, width, height."
        ));
    }
    // Une fenetre agrandie ignore toute pose: Windows la remet en place au
    // prochain rafraichissement. On la restaure d'abord, sinon le geste est un
    // succes rapporte qui ne change rien a l'ecran.
    if f.agrandie || f.reduite {
        let _ = unsafe { ShowWindow(f.hwnd, SW_RESTORE) };
    }
    let (gx, gy) = (f.rect.0, f.rect.1);
    let (gl, gh) = (f.rect.2 - f.rect.0, f.rect.3 - f.rect.1);
    let mut drapeaux = SWP_NOZORDER | SWP_NOACTIVATE;
    if x.is_none() && y.is_none() {
        drapeaux |= SWP_NOMOVE;
    }
    if largeur.is_none() && hauteur.is_none() {
        drapeaux |= SWP_NOSIZE;
    }
    unsafe {
        SetWindowPos(
            f.hwnd,
            Some(HWND_NOTOPMOST),
            x.unwrap_or(gx),
            y.unwrap_or(gy),
            largeur.unwrap_or(gl).max(1),
            hauteur.unwrap_or(gh).max(1),
            drapeaux,
        )
    }
    .map_err(|e| anyhow!("cannot place the window: {e}"))
}

/// Demande la fermeture, comme un clic sur la croix.
///
/// `WM_CLOSE` et non `TerminateProcess`: l'application recoit la demande, peut
/// proposer d'enregistrer, et peut refuser. C'est ce qu'on veut. Tuer un
/// processus depuis cet outil ferait perdre du travail sans rien demander, et
/// `shell_exec` est la pour qui veut vraiment tuer.
pub fn fermer(f: &Fenetre) -> Result<()> {
    unsafe { PostMessageW(Some(f.hwnd), WM_CLOSE, WPARAM(0), LPARAM(0)) }
        .map_err(|e| anyhow!("cannot ask the window to close: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_filtre_vide_est_refuse_plutot_que_de_prendre_au_hasard() {
        // Sans cette garde, `trouver("")` matchait la premiere fenetre venue,
        // parce que toute chaine contient la chaine vide. Fermer "la premiere
        // fenetre venue" est exactement le genre de geste qu'on ne rattrape pas.
        assert!(trouver("").is_err());
        assert!(trouver("   ").is_err());
    }

    #[test]
    fn nous_savons_si_nous_sommes_eleves() {
        // On ne teste pas la valeur, qui depend de la facon dont la suite a ete
        // lancee. On teste que l'appel aboutit: c'est du Win32 non sur, et une
        // regression s'y manifesterait par un plantage, pas par un faux.
        let _ = nous_sommes_eleves();
    }
}
