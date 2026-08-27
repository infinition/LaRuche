//! Le halo: dire a l'humain que la machine n'est plus a lui seul.
//!
//! Meme role et meme langage visuel que l'indicateur du navigateur, mais une
//! page web se decore avec un shadow root et trois lignes de CSS, alors qu'un
//! bureau demande des fenetres a l'OS. Ce qui est dessine:
//!
//!   - quatre barres ambrees sur les bords de l'ecran actif, qui respirent;
//!   - un panneau flottant qui nomme chaque action, deplacable a la souris;
//!   - un anneau qui suit le curseur et se contracte a chaque clic;
//!   - un flash blanc apres une capture, pour la signaler sans y apparaitre.
//!
//! # Pourquoi quatre barres et pas une fenetre plein ecran
//!
//! `UpdateLayeredWindow` reverse la totalite du bitmap a chaque image. En plein
//! ecran, un 2560x1440 coute 14 Mo par image, donc 440 Mo/s a trente images par
//! seconde, pour dessiner un cadre creux. Quatre barres de six pixels coutent
//! 60 Ko. C'est la meme image a l'ecran et deux mille fois moins de travail.
//!
//! # Discipline
//!
//! Tout est au mieux: le halo est un confort pour l'humain, jamais une raison
//! de faire echouer une action. Chaque appel public avale ses erreurs, et si le
//! fil de rendu meurt, l'outil continue sans decor.

#![allow(unsafe_code)]

use std::sync::mpsc::{channel, Sender};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, CreateFontW, DeleteDC, DeleteObject, SelectObject,
    SetBkMode, SetTextColor, TextOutW, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
    AC_SRC_ALPHA, AC_SRC_OVER, BLENDFUNCTION, CLIP_DEFAULT_PRECIS, DEFAULT_CHARSET,
    DEFAULT_PITCH, DEFAULT_QUALITY, FF_DONTCARE, FW_BOLD, FW_NORMAL, HBITMAP, HDC, HGDIOBJ,
    OUT_DEFAULT_PRECIS, TRANSPARENT,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::*;

/// Teinte de la ruche, la meme que dans le navigateur.
const AMBRE: (u8, u8, u8) = (245, 166, 35);
const FOND_PANNEAU: (u8, u8, u8) = (18, 16, 12);
const TEXTE: (u8, u8, u8) = (240, 230, 210);
const TITRE: (u8, u8, u8) = (245, 209, 139);

const EPAISSEUR_BARRE: i32 = 6;
const PANNEAU_L: i32 = 320;
const PANNEAU_H: i32 = 168;
const ANNEAU: i32 = 72;
const LIGNES_MAX: usize = 6;
/// Apres ce silence, le decor s'efface tout seul. Meme raison que dans la page:
/// l'outil ne sait pas quand le tour du modele se termine, mais l'absence
/// d'action, elle, se mesure.
const REPOS_MS: u128 = 12_000;

enum Msg {
    Ligne(String),
    Curseur { x: i32, y: i32, presse: bool },
    Ecran(RECT),
    Flash,
    Eteindre,
}

static CANAL: OnceLock<Mutex<Option<Sender<Msg>>>> = OnceLock::new();

fn canal() -> &'static Mutex<Option<Sender<Msg>>> {
    CANAL.get_or_init(|| Mutex::new(None))
}

fn envoyer(m: Msg) {
    let mut garde = match canal().lock() {
        Ok(g) => g,
        Err(_) => return,
    };
    if garde.is_none() {
        let (tx, rx) = channel::<Msg>();
        // Le fil possede les fenetres: sous Windows une fenetre appartient au
        // fil qui l'a creee, et seule sa boucle de messages peut la servir.
        std::thread::Builder::new()
            .name("laruche-halo".into())
            .spawn(move || unsafe { boucle(rx) })
            .ok();
        *garde = Some(tx);
    }
    if let Some(tx) = garde.as_ref() {
        if tx.send(m).is_err() {
            // Fil mort: on repartira de zero au prochain appel.
            *garde = None;
        }
    }
}

/// Annonce une action dans le panneau. Appele AVANT le geste, pour que l'humain
/// sache ce qui est vise pendant que le curseur s'y rend.
pub fn ligne(texte: &str) {
    envoyer(Msg::Ligne(texte.to_string()));
}

/// Deplace l'anneau. `presse` joue l'animation de clic.
pub fn curseur(x: i32, y: i32, presse: bool) {
    envoyer(Msg::Curseur { x, y, presse });
}

/// L'ecran sur lequel le cadre doit se dessiner.
pub fn ecran(gauche: i32, haut: i32, droite: i32, bas: i32) {
    envoyer(Msg::Ecran(RECT {
        left: gauche,
        top: haut,
        right: droite,
        bottom: bas,
    }));
}

pub fn flash() {
    envoyer(Msg::Flash);
}

pub fn eteindre() {
    envoyer(Msg::Eteindre);
}

// ───────────────────────────────── rendu ─────────────────────────────────

/// Un bitmap ARGB pre-multiplie, la seule forme que `UpdateLayeredWindow`
/// accepte, avec le DC qui va avec pour que GDI puisse ecrire dedans.
struct Toile {
    dc: HDC,
    bitmap: HBITMAP,
    ancien: HGDIOBJ,
    pixels: *mut u32,
    l: i32,
    h: i32,
}

impl Toile {
    unsafe fn neuve(l: i32, h: i32) -> Option<Toile> {
        let dc = CreateCompatibleDC(None);
        if dc.is_invalid() {
            return None;
        }
        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: l,
                // Negatif: DIB descendante, donc la ligne 0 est en haut, comme
                // partout ailleurs dans ce fichier.
                biHeight: -h,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();
        let bitmap = CreateDIBSection(Some(dc), &info, DIB_RGB_COLORS, &mut bits, None, 0).ok()?;
        let ancien = SelectObject(dc, bitmap.into());
        Some(Toile {
            dc,
            bitmap,
            ancien,
            pixels: bits as *mut u32,
            l,
            h,
        })
    }

    fn effacer(&self) {
        unsafe {
            std::ptr::write_bytes(self.pixels, 0, (self.l * self.h) as usize);
        }
    }

    /// Pose un pixel en ARGB pre-multiplie: la couche alpha doit deja etre
    /// appliquee aux trois canaux, sinon le compositeur affiche des halos noirs.
    ///
    /// Non utilise pour l'instant, garde parce que c'est la brique de base et
    /// que la retrouver ailleurs couterait plus que ces dix lignes.
    #[allow(dead_code)]
    fn poser(&self, x: i32, y: i32, (r, v, b): (u8, u8, u8), a: u8) {
        if x < 0 || y < 0 || x >= self.l || y >= self.h {
            return;
        }
        let f = a as u32;
        let px = (f << 24)
            | (((r as u32 * f) / 255) << 16)
            | (((v as u32 * f) / 255) << 8)
            | ((b as u32 * f) / 255);
        unsafe {
            *self.pixels.offset((y * self.l + x) as isize) = px;
        }
    }

    /// Melange par-dessus ce qui est deja la, pour superposer sans effacer.
    fn melanger(&self, x: i32, y: i32, (r, v, b): (u8, u8, u8), a: u8) {
        if x < 0 || y < 0 || x >= self.l || y >= self.h || a == 0 {
            return;
        }
        unsafe {
            let p = self.pixels.offset((y * self.l + x) as isize);
            let dst = *p;
            let (da, dr, dv, db) = (
                (dst >> 24) & 0xFF,
                (dst >> 16) & 0xFF,
                (dst >> 8) & 0xFF,
                dst & 0xFF,
            );
            let sa = a as u32;
            let inv = 255 - sa;
            let na = sa + da * inv / 255;
            let nr = (r as u32 * sa) / 255 + dr * inv / 255;
            let nv = (v as u32 * sa) / 255 + dv * inv / 255;
            let nb = (b as u32 * sa) / 255 + db * inv / 255;
            *p = (na << 24) | (nr << 16) | (nv << 8) | nb;
        }
    }

    fn rect(&self, x0: i32, y0: i32, x1: i32, y1: i32, c: (u8, u8, u8), a: u8) {
        for y in y0.max(0)..y1.min(self.h) {
            for x in x0.max(0)..x1.min(self.l) {
                self.melanger(x, y, c, a);
            }
        }
    }

    /// Rectangle aux coins arrondis, dessine par distance: plus court qu'une
    /// region GDI et il donne un bord anti-aliase gratuitement.
    fn rect_arrondi(&self, cadre: (i32, i32, i32, i32), rayon: i32, c: (u8, u8, u8), a: u8) {
        let (x0, y0, x1, y1) = cadre;
        let r = rayon as f32;
        for y in y0.max(0)..y1.min(self.h) {
            for x in x0.max(0)..x1.min(self.l) {
                // Distance au rectangle interieur, nulle partout sauf dans les
                // quatre coins, ou elle dessine l'arrondi.
                let dx = ((x0 + rayon - x) as f32).max((x - (x1 - 1 - rayon)) as f32).max(0.0);
                let dy = ((y0 + rayon - y) as f32).max((y - (y1 - 1 - rayon)) as f32).max(0.0);
                let d = (dx * dx + dy * dy).sqrt();
                let couverture = (r + 0.5 - d).clamp(0.0, 1.0);
                if couverture > 0.0 {
                    self.melanger(x, y, c, (a as f32 * couverture) as u8);
                }
            }
        }
    }

    /// Anneau anti-aliase, l'element le plus visible du decor.
    fn anneau(&self, cx: f32, cy: f32, rayon: f32, epaisseur: f32, c: (u8, u8, u8), a: u8) {
        let ext = rayon + epaisseur / 2.0 + 1.0;
        for y in (cy - ext) as i32..=(cy + ext) as i32 {
            for x in (cx - ext) as i32..=(cx + ext) as i32 {
                let d = (((x as f32 - cx).powi(2)) + ((y as f32 - cy).powi(2))).sqrt();
                let ecart = (d - rayon).abs();
                let couverture = (epaisseur / 2.0 + 0.5 - ecart).clamp(0.0, 1.0);
                if couverture > 0.0 {
                    self.melanger(x, y, c, (a as f32 * couverture) as u8);
                }
            }
        }
    }

    /// Texte GDI avec une couche alpha correcte.
    ///
    /// GDI ne connait pas l'alpha: il ecrit les trois canaux et laisse le
    /// quatrieme a zero, ce qui rend le texte invisible dans une fenetre
    /// superposee. On dessine donc en blanc sur noir dans une toile a part et
    /// on s'en sert comme masque, ce qui conserve l'anticrenelage.
    unsafe fn texte(&self, x: i32, y: i32, s: &str, c: (u8, u8, u8), gras: bool, taille: i32) {
        let l = self.l.min(PANNEAU_L * 2);
        let h = taille + 8;
        let Some(masque) = Toile::neuve(l, h) else {
            return;
        };
        masque.effacer();
        let police = CreateFontW(
            -taille,
            0,
            0,
            0,
            if gras { FW_BOLD.0 as i32 } else { FW_NORMAL.0 as i32 },
            0,
            0,
            0,
            DEFAULT_CHARSET,
            OUT_DEFAULT_PRECIS,
            CLIP_DEFAULT_PRECIS,
            DEFAULT_QUALITY,
            (DEFAULT_PITCH.0 | FF_DONTCARE.0) as u32,
            w!("Segoe UI"),
        );
        let ancienne = SelectObject(masque.dc, police.into());
        SetBkMode(masque.dc, TRANSPARENT);
        SetTextColor(masque.dc, COLORREF(0x00FF_FFFF));
        let large: Vec<u16> = s.encode_utf16().collect();
        let _ = TextOutW(masque.dc, 0, 0, &large);
        SelectObject(masque.dc, ancienne);
        let _ = DeleteObject(police.into());

        for my in 0..h {
            for mx in 0..l {
                let p = *masque.pixels.offset((my * l + mx) as isize);
                // Le masque est opaque en blanc: la luminance EST la couverture.
                let lum = (p & 0xFF) as u8;
                if lum > 0 {
                    self.melanger(x + mx, y + my, c, lum);
                }
            }
        }
    }

    /// Envoie la toile a la fenetre superposee.
    unsafe fn pousser(&self, hwnd: HWND, x: i32, y: i32) {
        let taille = SIZE {
            cx: self.l,
            cy: self.h,
        };
        let source = POINT { x: 0, y: 0 };
        let position = POINT { x, y };
        let melange = BLENDFUNCTION {
            BlendOp: AC_SRC_OVER as u8,
            BlendFlags: 0,
            SourceConstantAlpha: 255,
            AlphaFormat: AC_SRC_ALPHA as u8,
        };
        let _ = UpdateLayeredWindow(
            hwnd,
            None,
            Some(&position),
            Some(&taille),
            Some(self.dc),
            Some(&source),
            COLORREF(0),
            Some(&melange),
            ULW_ALPHA,
        );
    }
}

impl Drop for Toile {
    fn drop(&mut self) {
        unsafe {
            SelectObject(self.dc, self.ancien);
            let _ = DeleteObject(self.bitmap.into());
            let _ = DeleteDC(self.dc);
        }
    }
}

unsafe extern "system" fn proc_fenetre(
    hwnd: HWND,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
) -> LRESULT {
    match msg {
        // Le panneau est la seule fenetre qui prend la souris, et le seul geste
        // qu'elle accepte est d'etre deplacee: repondre HTCAPTION donne le
        // glisser complet sans une ligne de gestion de la souris.
        WM_NCHITTEST => LRESULT(HTCAPTION as isize),
        WM_DESTROY => LRESULT(0),
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

/// Cree une fenetre superposee, sans bordure, hors barre des taches, toujours
/// au-dessus. `traversante` la rend invisible aux clics, ce que toutes les
/// pieces du decor veulent sauf le panneau.
unsafe fn fenetre(classe: PCWSTR, l: i32, h: i32, traversante: bool) -> Option<HWND> {
    let mut style = WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE;
    if traversante {
        style |= WS_EX_TRANSPARENT;
    }
    CreateWindowExW(
        style,
        classe,
        w!("LaRuche"),
        WS_POPUP,
        0,
        0,
        l,
        h,
        None,
        None,
        None,
        None,
    )
    .ok()
}

struct Decor {
    barres: [HWND; 4],
    panneau: HWND,
    anneau: HWND,
    flash: HWND,
    /// Position posee par l'utilisateur en deplacant le panneau: une fois qu'il
    /// l'a bouge, on ne le replace plus jamais nous-memes.
    panneau_pose: bool,
    ecran: RECT,
    lignes: Vec<(String, String)>,
    curseur: (i32, i32),
    presse_a: Option<Instant>,
    flash_a: Option<Instant>,
    derniere_activite: Instant,
    visible: bool,
}

unsafe fn boucle(rx: std::sync::mpsc::Receiver<Msg>) {
    let instance = match GetModuleHandleW(None) {
        Ok(i) => i,
        Err(_) => return,
    };
    let classe = w!("LaRucheHalo");
    let wc = WNDCLASSW {
        lpfnWndProc: Some(proc_fenetre),
        hInstance: instance.into(),
        lpszClassName: classe,
        ..Default::default()
    };
    RegisterClassW(&wc);

    let ecran_defaut = RECT {
        left: 0,
        top: 0,
        right: GetSystemMetrics(SM_CXSCREEN),
        bottom: GetSystemMetrics(SM_CYSCREEN),
    };
    let l = ecran_defaut.right - ecran_defaut.left;
    let h = ecran_defaut.bottom - ecran_defaut.top;

    let Some(barre_haut) = fenetre(classe, l, EPAISSEUR_BARRE, true) else {
        return;
    };
    let (Some(barre_bas), Some(barre_gauche), Some(barre_droite)) = (
        fenetre(classe, l, EPAISSEUR_BARRE, true),
        fenetre(classe, EPAISSEUR_BARRE, h, true),
        fenetre(classe, EPAISSEUR_BARRE, h, true),
    ) else {
        return;
    };
    let (Some(panneau), Some(anneau_w), Some(flash_w)) = (
        fenetre(classe, PANNEAU_L, PANNEAU_H, false),
        fenetre(classe, ANNEAU, ANNEAU, true),
        fenetre(classe, l, h, true),
    ) else {
        return;
    };

    let mut d = Decor {
        barres: [barre_haut, barre_bas, barre_gauche, barre_droite],
        panneau,
        anneau: anneau_w,
        flash: flash_w,
        panneau_pose: false,
        ecran: ecran_defaut,
        lignes: Vec::new(),
        curseur: (l / 2, h / 2),
        presse_a: None,
        flash_a: None,
        derniere_activite: Instant::now(),
        visible: false,
    };

    // Trente images par seconde: assez pour que la respiration et l'anneau
    // soient fluides, assez peu pour que le decor ne coute rien.
    SetTimer(Some(d.barres[0]), 1, 33, None);

    let mut message = MSG::default();
    loop {
        while let Ok(m) = rx.try_recv() {
            match m {
                Msg::Ligne(t) => {
                    let maintenant = chrono::Local::now().format("%H:%M:%S").to_string();
                    d.lignes.insert(0, (maintenant, t));
                    d.lignes.truncate(LIGNES_MAX);
                    d.derniere_activite = Instant::now();
                    allumer(&mut d);
                }
                Msg::Curseur { x, y, presse } => {
                    d.curseur = (x, y);
                    if presse {
                        d.presse_a = Some(Instant::now());
                    }
                    d.derniere_activite = Instant::now();
                    allumer(&mut d);
                }
                Msg::Ecran(r) => {
                    d.ecran = r;
                    d.derniere_activite = Instant::now();
                    allumer(&mut d);
                    poser_barres(&d);
                }
                Msg::Flash => {
                    d.flash_a = Some(Instant::now());
                    d.derniere_activite = Instant::now();
                }
                Msg::Eteindre => eteindre_decor(&mut d),
            }
        }

        while PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }

        if d.visible {
            if d.derniere_activite.elapsed().as_millis() > REPOS_MS {
                eteindre_decor(&mut d);
            } else {
                peindre(&mut d);
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(33));
    }
}

unsafe fn allumer(d: &mut Decor) {
    if d.visible {
        return;
    }
    d.visible = true;
    poser_barres(d);
    for w in d.barres {
        let _ = ShowWindow(w, SW_SHOWNOACTIVATE);
    }
    let _ = ShowWindow(d.anneau, SW_SHOWNOACTIVATE);
    if !d.panneau_pose {
        let _ = SetWindowPos(
            d.panneau,
            Some(HWND_TOPMOST),
            d.ecran.left + 24,
            d.ecran.bottom - PANNEAU_H - 48,
            PANNEAU_L,
            PANNEAU_H,
            SWP_NOACTIVATE,
        );
    }
    let _ = ShowWindow(d.panneau, SW_SHOWNOACTIVATE);
}

unsafe fn eteindre_decor(d: &mut Decor) {
    d.visible = false;
    d.lignes.clear();
    for w in d.barres {
        let _ = ShowWindow(w, SW_HIDE);
    }
    let _ = ShowWindow(d.panneau, SW_HIDE);
    let _ = ShowWindow(d.anneau, SW_HIDE);
    let _ = ShowWindow(d.flash, SW_HIDE);
}

unsafe fn poser_barres(d: &Decor) {
    let (x, y) = (d.ecran.left, d.ecran.top);
    let l = d.ecran.right - d.ecran.left;
    let h = d.ecran.bottom - d.ecran.top;
    let places = [
        (x, y, l, EPAISSEUR_BARRE),
        (x, d.ecran.bottom - EPAISSEUR_BARRE, l, EPAISSEUR_BARRE),
        (x, y, EPAISSEUR_BARRE, h),
        (d.ecran.right - EPAISSEUR_BARRE, y, EPAISSEUR_BARRE, h),
    ];
    for (w, (px, py, pl, ph)) in d.barres.iter().zip(places) {
        let _ = SetWindowPos(*w, Some(HWND_TOPMOST), px, py, pl, ph, SWP_NOACTIVATE);
    }
}

unsafe fn peindre(d: &mut Decor) {
    let t = d.derniere_activite.elapsed().as_millis() as f32 / 1000.0;
    // Respiration: une sinusoide lente, exactement comme l'animation `lr-breathe`
    // du navigateur, pour que les deux surfaces se ressemblent.
    let souffle = 0.55 + 0.45 * ((t * 2.6).sin() * 0.5 + 0.5);

    let l = d.ecran.right - d.ecran.left;
    let h = d.ecran.bottom - d.ecran.top;
    let places = [
        (d.ecran.left, d.ecran.top, l, EPAISSEUR_BARRE, 0),
        (
            d.ecran.left,
            d.ecran.bottom - EPAISSEUR_BARRE,
            l,
            EPAISSEUR_BARRE,
            1,
        ),
        (d.ecran.left, d.ecran.top, EPAISSEUR_BARRE, h, 2),
        (
            d.ecran.right - EPAISSEUR_BARRE,
            d.ecran.top,
            EPAISSEUR_BARRE,
            h,
            3,
        ),
    ];
    for (px, py, pl, ph, i) in places {
        let Some(toile) = Toile::neuve(pl, ph) else {
            continue;
        };
        toile.effacer();
        // Degrade du bord vers l'interieur: la lueur s'eteint au lieu de couper
        // net, ce qui evite l'impression d'un cadre colle par-dessus l'ecran.
        for k in 0..EPAISSEUR_BARRE {
            let force = 1.0 - (k as f32 / EPAISSEUR_BARRE as f32);
            let a = (225.0 * force * souffle) as u8;
            match i {
                0 => toile.rect(0, k, pl, k + 1, AMBRE, a),
                1 => toile.rect(0, ph - 1 - k, pl, ph - k, AMBRE, a),
                2 => toile.rect(k, 0, k + 1, ph, AMBRE, a),
                _ => toile.rect(pl - 1 - k, 0, pl - k, ph, AMBRE, a),
            }
        }
        toile.pousser(d.barres[i as usize], px, py);
    }

    // L'anneau suit le curseur, et se contracte brievement a chaque clic.
    if let Some(toile) = Toile::neuve(ANNEAU, ANNEAU) {
        toile.effacer();
        let c = ANNEAU as f32 / 2.0;
        let presse = d
            .presse_a
            .map(|p| p.elapsed().as_millis() as f32 / 450.0)
            .filter(|p| *p <= 1.0);
        match presse {
            Some(p) => {
                // Onde de choc: un anneau qui grandit et s'efface.
                let rayon = 8.0 + 22.0 * p;
                toile.anneau(c, c, rayon, 3.0, AMBRE, (230.0 * (1.0 - p)) as u8);
                toile.anneau(c, c, 9.0, 3.0, AMBRE, 235);
            }
            None => {
                toile.anneau(c, c, 13.0, 2.5, AMBRE, (200.0 * souffle) as u8);
                toile.anneau(c, c, 4.0, 4.0, AMBRE, 230);
            }
        }
        toile.pousser(d.anneau, d.curseur.0 - ANNEAU / 2, d.curseur.1 - ANNEAU / 2);
    }

    // Le panneau.
    if let Some(toile) = Toile::neuve(PANNEAU_L, PANNEAU_H) {
        toile.effacer();
        toile.rect_arrondi((0, 0, PANNEAU_L, PANNEAU_H), 12, FOND_PANNEAU, 216);
        // Bordure: le meme arrondi une fraction plus grand, en ambre, puis le
        // fond redessine par-dessus laisse un lisere d'un pixel.
        toile.rect_arrondi((0, 0, PANNEAU_L, 30), 12, AMBRE, 34);
        toile.rect(0, 30, PANNEAU_L, 31, AMBRE, 90);
        toile.anneau(18.0, 15.0, 3.5, 3.0, AMBRE, (255.0 * souffle) as u8);
        toile.texte(32, 6, "LaRuche", TITRE, true, 15);
        toile.texte(PANNEAU_L - 96, 8, "controle actif", TITRE, false, 12);

        let mut y = 40;
        for (heure, texte) in d.lignes.iter() {
            let couleur = if y == 40 { TITRE } else { TEXTE };
            toile.texte(12, y, heure, (138, 127, 102), false, 12);
            toile.texte(70, y, texte, couleur, false, 13);
            y += 20;
            if y > PANNEAU_H - 18 {
                break;
            }
        }
        let (mut px, mut py) = (d.ecran.left + 24, d.ecran.bottom - PANNEAU_H - 48);
        if d.panneau_pose {
            let mut r = RECT::default();
            if GetWindowRect(d.panneau, &mut r).is_ok() {
                px = r.left;
                py = r.top;
            }
        }
        toile.pousser(d.panneau, px, py);
        // Une fois la fenetre affichee, l'utilisateur peut l'avoir deplacee: a
        // partir de la, sa position fait foi, plus la notre.
        d.panneau_pose = true;
    }

    // Le flash de capture, en fondu.
    if let Some(depuis) = d.flash_a {
        {
            let p = depuis.elapsed().as_millis() as f32 / 380.0;
            if p >= 1.0 {
                d.flash_a = None;
                let _ = ShowWindow(d.flash, SW_HIDE);
            } else {
                // Une fenetre a alpha constant suffit ici, et evite de reverser
                // un bitmap plein ecran a chaque image.
                let a = (200.0 * (1.0 - p) * (p * 6.0).min(1.0)) as u8;
                let _ = SetLayeredWindowAttributes(d.flash, COLORREF(0x00FF_FFFF), a, LWA_ALPHA);
                let _ = SetWindowPos(
                    d.flash,
                    Some(HWND_TOPMOST),
                    d.ecran.left,
                    d.ecran.top,
                    d.ecran.right - d.ecran.left,
                    d.ecran.bottom - d.ecran.top,
                    SWP_NOACTIVATE | SWP_SHOWWINDOW,
                );
            }
        }
    }
}
