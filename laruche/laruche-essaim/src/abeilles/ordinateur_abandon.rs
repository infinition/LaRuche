//! Le coupe-circuit: un raccourci global qui reprend la main, tout de suite.
//!
//! La garde de reprise a la souris couvre le cas courant, mais elle a deux
//! trous par construction. Elle compare une position, donc elle ne voit rien
//! si l'agent travaille par `ref` sans bouger le curseur. Et elle est
//! consultee ENTRE deux appels, donc elle n'interrompt pas ce qui est deja en
//! train de se produire: une frappe de deux mille caracteres part en entier.
//!
//! `Ctrl+Alt+Shift+H`, H pour halt. Quatre modificateurs parce qu'un raccourci
//! d'urgence ne doit jamais entrer en conflit avec celui d'une application, et
//! parce qu'on ne le declenche pas par megarde en visant autre chose.
//!
//! # Pourquoi un fil a part
//!
//! `RegisterHotKey` depose `WM_HOTKEY` dans la file du FIL qui a enregistre, et
//! il faut donc un fil qui pompe cette file. Le halo en a deja un, mais il
//! demarre paresseusement et pas du tout quand `glow: false`. Un coupe-circuit
//! qui disparait quand on eteint la decoration ne serait pas un coupe-circuit.
//!
//! Le fil bloque dans `GetMessageW`: il ne coute rien tant que rien n'arrive.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Once;

use windows::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, MOD_ALT, MOD_CONTROL, MOD_SHIFT,
};
use windows::Win32::UI::WindowsAndMessaging::{GetMessageW, MSG, WM_HOTKEY};

/// `H`, en code virtuel Windows. Les lettres valent leur majuscule ASCII.
const TOUCHE_H: u32 = 0x48;
const ID_RACCOURCI: i32 = 0x4C52; // "LR"

static DEMANDE: AtomicBool = AtomicBool::new(false);
static UNE_FOIS: Once = Once::new();
static ENREGISTRE: AtomicBool = AtomicBool::new(false);

/// Met le coupe-circuit en place. Idempotent, appelable a chaque geste.
pub fn armer() {
    UNE_FOIS.call_once(|| {
        std::thread::Builder::new()
            .name("laruche-abandon".into())
            .spawn(|| unsafe {
                // `None` comme fenetre: le message arrive dans la file du fil.
                let pose = RegisterHotKey(
                    None,
                    ID_RACCOURCI,
                    MOD_CONTROL | MOD_ALT | MOD_SHIFT,
                    TOUCHE_H,
                );
                if pose.is_err() {
                    // Deja pris par quelqu'un d'autre. Ce n'est pas fatal, mais
                    // il ne faut surtout pas laisser croire que le filet existe.
                    tracing::warn!(
                        "Ctrl+Alt+Shift+H is already taken by another application: the \
                         computer tool has no abort shortcut on this session"
                    );
                    return;
                }
                ENREGISTRE.store(true, Ordering::SeqCst);
                tracing::info!("coupe-circuit arme: Ctrl+Alt+Shift+H");

                let mut message = MSG::default();
                // `GetMessageW` bloque jusqu'a l'arrivee d'un message, donc ce
                // fil ne consomme rien en attendant.
                while GetMessageW(&mut message, None, 0, 0).as_bool() {
                    if message.message == WM_HOTKEY && message.wParam.0 as i32 == ID_RACCOURCI {
                        DEMANDE.store(true, Ordering::SeqCst);
                        tracing::warn!("abandon demande au clavier");
                    }
                }
            })
            .ok();
    });
}

/// Le raccourci a-t-il ete presse depuis la derniere fois qu'on a regarde?
///
/// Lit ET remet a zero: un abandon interrompt le geste en cours et le suivant
/// doit pouvoir repartir, sinon l'agent serait bloque jusqu'au redemarrage.
/// C'est le meme raisonnement que la garde souris, qui rend la main sans
/// devenir un verrou.
pub fn demande() -> bool {
    DEMANDE.swap(false, Ordering::SeqCst)
}

/// Sans remise a zero, pour les boucles qui verifient a chaque pas.
pub fn en_cours() -> bool {
    DEMANDE.load(Ordering::SeqCst)
}

/// Le raccourci est-il reellement en place? Utilise pour ne promettre a
/// l'utilisateur que ce qui existe.
pub fn arme() -> bool {
    ENREGISTRE.load(Ordering::SeqCst)
}
