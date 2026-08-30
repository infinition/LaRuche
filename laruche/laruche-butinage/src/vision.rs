//! On envoie les captures au modele, et on encaisse proprement son refus.
//!
//! Une capture d'outil part maintenant vers le modele, pas seulement vers
//! l'ecran: sans ca il annonce "capture prise" sans avoir rien regarde. Mais
//! tous les endpoints n'acceptent pas une image, y compris pour un modele dont
//! la fiche annonce la vision native: observe sur `deepseek-v4-flash`, qui
//! repond 400 `This model does not support image` sur une simple capture de
//! navigateur, envoyee au format OpenAI standard (`image_url` + data URL).
//!
//! Avant ce module, ce refus tuait le tour entier. C'etait la pire des issues:
//! l'agent mourait au milieu de son travail a cause d'une photo dont il
//! n'avait pas besoin pour continuer.
//!
//! Donc: on envoie. Si le fournisseur refuse, on retient le modele, on retire
//! l'image, on previent le modele qu'elle existe sans qu'il la voie, et le tour
//! continue. Une fois par modele et par session, pas davantage.
//!
//! Aucune liste de modeles ici, ni blanche ni noire, et c'est une decision.
//! Deviner d'apres le nom se trompe dans les deux sens, et la fiche technique
//! n'est pas une preuve: seul compte ce que l'endpoint accepte vraiment.
//!
//! `LARUCHE_VISION=0` coupe l'envoi partout, `=1` le force malgre un refus
//! deja retenu.

use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};

/// Modeles pris en flagrant delit de refus pendant cette session.
fn aveugles() -> &'static Mutex<HashSet<String>> {
    static S: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    S.get_or_init(|| Mutex::new(HashSet::new()))
}

/// Le fournisseur vient de refuser une image: ne plus lui en envoyer.
pub fn marquer_aveugle(modele: &str) {
    if let Ok(mut s) = aveugles().lock() {
        s.insert(modele.to_lowercase());
    }
}

/// Le corps d'erreur dit-il que le modele ne prend pas d'image?
///
/// Chaque fournisseur le formule a sa facon, d'ou la reconnaissance par
/// morceaux plutot que par phrase exacte.
pub fn corps_refuse_image(corps: &str) -> bool {
    let c = corps.to_lowercase();
    // Un souci de FORMAT n'est pas une absence de vision: le modele voit, c'est
    // notre encodage qui ne lui plait pas. Confondre les deux couperait la
    // vision d'un modele qui l'a.
    if c.contains("image format") {
        return false;
    }
    c.contains("does not support image")
        || c.contains("not support images")
        || c.contains("image_url is not supported")
        || c.contains("does not support vision")
        || c.contains("no vision support")
        || c.contains("invalid content type: image")
        || (c.contains("image") && c.contains("not supported"))
}

/// Envoie-t-on les images a ce modele?
///
/// Oui, tant qu'il n'a pas refuse dans cette session.
pub fn modele_voit(modele: &str) -> bool {
    match std::env::var("LARUCHE_VISION").as_deref() {
        Ok("0") | Ok("off") | Ok("false") => return false,
        Ok("1") | Ok("on") | Ok("true") => return true,
        _ => {}
    }
    let m = modele.to_lowercase();
    !aveugles().lock().map(|s| s.contains(&m)).unwrap_or(false)
}

/// Ce qu'on dit au modele a la place de l'image.
///
/// Le silence serait pire que l'erreur: l'outil vient d'annoncer "capture
/// prise, decris ce que tu vois", et un modele qui ne recoit rien decrit quand
/// meme, de memoire ou d'imagination. La phrase doit donc etre explicite sur
/// les trois points: l'image existe, l'utilisateur la voit, toi non.
pub fn note_sans_vision(combien: usize) -> String {
    format!(
        "\n\n[{combien} image(s) produced and displayed to the USER, but this model has no vision \
         and they were NOT sent to you. You cannot see them. Do not describe them and do not guess \
         what they contain: say plainly that you cannot see the image, and get the information \
         from text instead (browser read for a page, the accessibility tree for a window).]"
    )
}

/// Retire les images d'une conversation deja convertie pour le fournisseur.
///
/// On travaille sur le JSON sortant plutot que sur l'historique: l'historique
/// est partage, il survit au tour, et il doit garder l'image pour le jour ou le
/// meme carnet repart sur un modele qui voit.
///
/// Rend le nombre d'images retirees.
pub fn retirer_images(msgs: &mut [serde_json::Value]) -> usize {
    let mut total = 0usize;
    for m in msgs.iter_mut() {
        let combien = m
            .get("images")
            .and_then(|v| v.as_array())
            .map(|a| a.len())
            .unwrap_or(0);
        // `attachments` porte aussi les pieces non-image (audio, fichiers): on
        // n'enleve que ce qui est une image, le reste ne genait personne.
        if let Some(att) = m.get_mut("attachments").and_then(|v| v.as_array_mut()) {
            att.retain(|p| {
                p.get("mime")
                    .or_else(|| p.get("mime_type"))
                    .and_then(|v| v.as_str())
                    .map(|s| !s.starts_with("image/"))
                    .unwrap_or(true)
            });
            let vide = att.is_empty();
            if vide {
                if let Some(o) = m.as_object_mut() {
                    o.remove("attachments");
                }
            }
        }
        if combien == 0 {
            continue;
        }
        if let Some(o) = m.as_object_mut() {
            o.remove("images");
        }
        let note = note_sans_vision(combien);
        let neuf = match m.get("content").and_then(|v| v.as_str()) {
            Some(c) => format!("{c}{note}"),
            None => note.trim().to_string(),
        };
        m["content"] = serde_json::Value::String(neuf);
        total += combien;
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Les tests touchent une variable d'environnement, donc un seul a la fois.
    fn verrou() -> std::sync::MutexGuard<'static, ()> {
        static V: OnceLock<Mutex<()>> = OnceLock::new();
        V.get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|e| e.into_inner())
    }

    #[test]
    fn on_envoie_a_tout_le_monde_jusqua_preuve_du_contraire() {
        let _g = verrou();
        std::env::remove_var("LARUCHE_VISION");
        // Aucun nom ne perd la vision sans que le fournisseur l'ait dit: un
        // modele dont une version voit et l'autre non porte le meme prefixe, et
        // deviner reviendrait a aveugler des modeles capables en silence.
        assert!(modele_voit("deepseek-v4-flash"));
        assert!(modele_voit("qwen2.5-coder:32b"));
        assert!(modele_voit("un-modele-de-2027"));
    }

    #[test]
    fn le_refus_du_fournisseur_est_retenu() {
        let _g = verrou();
        std::env::remove_var("LARUCHE_VISION");
        assert!(modele_voit("modele-cobaye"));
        marquer_aveugle("Modele-Cobaye"); // la casse ne doit pas sauver le modele
        assert!(!modele_voit("modele-cobaye"));
        // Le forcage passe outre: si le refus a ete mal interprete, la personne
        // doit pouvoir le dire.
        std::env::set_var("LARUCHE_VISION", "1");
        assert!(modele_voit("modele-cobaye"));
        std::env::remove_var("LARUCHE_VISION");
    }

    #[test]
    fn on_reconnait_le_refus_dans_le_corps() {
        assert!(corps_refuse_image(
            r#"{"error":{"message":"This model does not support image","type":"invalid_request_error"}}"#
        ));
        assert!(corps_refuse_image(
            "image_url is not supported by this model"
        ));
        assert!(!corps_refuse_image("unsupported image format: bmp"));
        assert!(!corps_refuse_image("rate limit exceeded"));
    }

    #[test]
    fn retirer_laisse_le_texte_et_previent_le_modele() {
        let mut msgs = vec![
            json!({"role":"user","content":"regarde","images":["AAAA","BBBB"],
                   "attachments":[{"kind":"image","mime":"image/png","data":"AAAA"},
                                  {"kind":"audio","mime":"audio/wav","data":"CCCC"}]}),
            json!({"role":"assistant","content":"ok"}),
        ];
        assert_eq!(retirer_images(&mut msgs), 2);
        assert!(msgs[0].get("images").is_none());
        let c = msgs[0]["content"].as_str().unwrap();
        assert!(c.starts_with("regarde"), "le texte de l'utilisateur reste");
        assert!(c.contains("cannot see"), "le modele doit etre prevenu");
        // L'audio n'a rien a voir avec la vision et ne doit pas partir avec.
        let att = msgs[0]["attachments"].as_array().unwrap();
        assert_eq!(att.len(), 1);
        assert_eq!(att[0]["kind"], "audio");
        assert_eq!(msgs[1]["content"], "ok", "les autres messages intacts");
    }

    #[test]
    fn sans_image_rien_ne_bouge() {
        let mut msgs = vec![json!({"role":"user","content":"bonjour"})];
        assert_eq!(retirer_images(&mut msgs), 0);
        assert_eq!(msgs[0]["content"], "bonjour");
    }
}
