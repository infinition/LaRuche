//! Mise au gabarit des images avant l'envoi au modele.
//!
//! Une image collee dans le chat fait couramment 2 a 4 Mo. Encodee en base64
//! elle en fait un tiers de plus, et la requete entiere depasse la limite de
//! taille du fournisseur, qui la coupe. Le symptome ne ressemble alors pas du
//! tout a un probleme d'image:
//!
//! ```text
//! 400 Failed to parse the request body as JSON:
//!     messages[1].content[1].image_url.url: EOF while parsing a string
//!     at line 1 column 2776858
//! ```
//!
//! La colonne de l'erreur EST la taille exacte du corps: le serveur a lu
//! jusqu'au bout de ce qu'il a bien voulu accepter, et s'est arrete au milieu
//! de la chaine base64. Notre corps, lui, est complet et valide. Retenter a
//! l'identique redonne exactement la meme erreur, trois fois, puis le tour meurt.
//!
//! Or ces octets ne servaient a rien. Les modeles de vision decoupent l'image
//! en tuiles de quelques centaines de pixels et ne voient jamais la pleine
//! resolution: au-dela d'environ 1568 px sur le grand cote, on paye du temps,
//! des jetons et un risque de troncature pour une information que personne ne
//! regarde. Reduire n'est donc pas une degradation, c'est enlever ce que le
//! modele allait jeter.
//!
//! Ce qui compte, dans l'ordre:
//!
//!   - on ne touche a une image que si elle depasse le gabarit. Une petite
//!     capture part telle quelle, bit pour bit;
//!   - le PNG reste du PNG tant qu'il rentre. Les captures d'ecran sont pleines
//!     de texte fin, et le JPEG en fait de la bouillie, ce qui est exactement
//!     l'information qu'on demande au modele de lire;
//!   - le JPEG n'arrive qu'en dernier recours, quand le PNG reduit ne rentre
//!     toujours pas: une photo un peu adoucie vaut mieux qu'un tour mort.

use base64::Engine;

/// Grand cote maximum, en pixels.
///
/// Au-dela, les fournisseurs redimensionnent eux-memes avant de decouper en
/// tuiles. Envoyer plus, c'est envoyer des octets qui seront jetes a l'arrivee.
/// 1280 est aussi la largeur a laquelle l'outil `computer` rend deja ses
/// captures, donc le cas courant ne perd rien du tout.
pub const COTE_MAX: u32 = 1280;

/// Taille maximale de la chaine base64 d'une image.
///
/// Le premier reglage etait a 900 ko, en raisonnant sur la limite annoncee par
/// les fournisseurs. C'etait le mauvais raisonnement, et ca a coute plusieurs
/// heures: DeepSeek documente 32 Mio par image et refuse en pratique bien plus
/// tot. Le seul chiffre qu'on ait mesure sur cet endpoint, c'est une image de
/// 164 ko en base64 acceptee, et des captures plus lourdes refusees.
///
/// Donc on ne vise plus la limite documentee, on vise sous le seul point qu'on
/// a vu passer. Et ca ne coute rien: DeepSeek facture 384 jetons par image
/// quelle que soit sa resolution, et tous les modeles de vision redecoupent en
/// tuiles de quelques centaines de pixels. Les octets economises ici sont des
/// octets que personne ne regarde.
pub const B64_MAX: usize = 150_000;

/// Gabarit reduit, applique a un fournisseur qui a quand meme bute.
///
/// Deuxieme et derniere chance avant de renoncer aux images pour ce modele. A
/// 768 px un modele de vision voit encore tout ce qu'il sait voir.
pub const COTE_SERRE: u32 = 768;
pub const B64_SERRE: usize = 80_000;

/// Taille maximale du CORPS de la requete, en octets.
///
/// Le plafond par image ne suffit pas, et c'est ce qui a coute le plus de
/// temps sur ce sujet. Une capture de 56 ko passe le controle par image et fait
/// pourtant echouer la requete, parce que ce qui compte pour l'endpoint c'est
/// le corps entier: messages, transcript des outils, et les schemas des outils
/// qui pesent a eux seuls 6 a 10 ko.
///
/// Deux corps mesures, refuses tous les deux:
///
/// ```text
/// 82 732 octets  (image 56 419, soit 68 % du corps)
/// 92 842 octets  (image 63 247, soit 68 % du corps)
/// ```
///
/// Dans les deux cas l'erreur nomme la FIN du corps, et le message change selon
/// ce qui s'y trouve: `tools[9]...` ou `messages[11].content`, puisque les cles
/// sont serialisees dans l'ordre alphabetique et que `tools` ferme la marche.
/// Ce n'est donc pas un probleme de contenu, et on a cherche longtemps du cote
/// du format a cause de ca.
///
/// 60 ko: sous les deux mesures refusees, avec de la marge pour un transcript
/// qui grossit en cours de mission. Une image ramenee a cette taille reste
/// parfaitement lisible pour un modele de vision, qui la redecoupe de toute
/// facon en tuiles de quelques centaines de pixels.
pub const CORPS_MAX: usize = 60_000;

/// Ramene une conversation deja convertie sous `CORPS_MAX`, schemas compris.
///
/// Rend la taille estimee finale. Reduit les images par paliers plutot que de
/// viser juste du premier coup: la taille d'un PNG ne se predit pas depuis ses
/// dimensions, seule la reencoder le dit.
pub fn au_budget_corps(
    msgs: &mut [serde_json::Value],
    taille_schemas: usize,
    plafond: usize,
) -> usize {
    let poids = |m: &[serde_json::Value]| -> usize {
        m.iter()
            .map(|v| serde_json::to_string(v).map(|s| s.len()).unwrap_or(0))
            .sum::<usize>()
            + taille_schemas
    };

    let mut taille = poids(msgs);
    if taille <= plafond {
        return taille;
    }

    // Du plus large au plus etroit. On s'arrete des que ca rentre: inutile
    // d'abimer une capture plus que necessaire.
    for (cote, b64) in [(1024u32, 90_000usize), (768, 55_000), (512, 30_000), (384, 16_000)] {
        for m in msgs.iter_mut() {
            reduire_message(m, cote, b64);
        }
        taille = poids(msgs);
        if taille <= plafond {
            return taille;
        }
    }
    taille
}

/// Applique un gabarit a toutes les images d'un message, quel que soit le
/// format sous lequel elles y sont rangees.
fn reduire_message(m: &mut serde_json::Value, cote: u32, plafond: usize) {
    if let Some(arr) = m.get_mut("images").and_then(|v| v.as_array_mut()) {
        for img in arr.iter_mut() {
            let Some(b64) = img.as_str() else { continue };
            if let Some(r) = au_gabarit_borne("image/png", b64, cote, plafond) {
                *img = serde_json::Value::String(r.b64);
            }
        }
    }
    if let Some(arr) = m.get_mut("attachments").and_then(|v| v.as_array_mut()) {
        for p in arr.iter_mut() {
            if p.get("kind").and_then(|v| v.as_str()) != Some("image") {
                continue;
            }
            let mime = p
                .get("mime_type")
                .or_else(|| p.get("mime"))
                .and_then(|v| v.as_str())
                .unwrap_or("image/png")
                .to_string();
            let Some(b64) = p.get("data").and_then(|v| v.as_str()) else {
                continue;
            };
            if let Some(r) = au_gabarit_borne(&mime, b64, cote, plafond) {
                p["data"] = serde_json::Value::String(r.b64);
                p["mime_type"] = serde_json::Value::String(r.mime.clone());
                if p.get("mime").is_some() {
                    p["mime"] = serde_json::Value::String(r.mime);
                }
            }
        }
    }
}

/// Une image prete a partir: son type MIME et sa charge base64.
pub struct Image {
    pub mime: String,
    pub b64: String,
}

/// Ramene une image au gabarit, ou la rend inchangee si elle y est deja.
///
/// `mime` est celui annonce par l'appelant; il peut mentir, le decodage se fie
/// a la signature du fichier. Rend `None` si l'image est illisible: l'appelant
/// garde alors l'original, parce qu'un format qu'on ne sait pas decoder peut
/// tres bien etre un format que le fournisseur, lui, accepte.
pub fn au_gabarit(mime: &str, b64: &str) -> Option<Image> {
    au_gabarit_borne(mime, b64, COTE_MAX, B64_MAX)
}

/// Meme chose, avec des bornes choisies par l'appelant.
pub fn au_gabarit_borne(mime: &str, b64: &str, cote_max: u32, b64_max: usize) -> Option<Image> {
    if b64.len() <= b64_max {
        return None;
    }
    let brut = base64::engine::general_purpose::STANDARD.decode(b64).ok()?;
    let img = image::load_from_memory(&brut).ok()?;
    let _ = mime;

    // Le gabarit ne se joue pas qu'aux pixels: une photo bruitee de 1400 px
    // pese plus lourd qu'une capture d'ecran de 3000 px, parce que le bruit ne
    // se compresse pas. Reduire une seule fois ne garantit donc rien. On
    // descend par paliers, et on ne s'arrete qu'une fois reellement sous la
    // limite: c'est la seule facon d'avoir une promesse tenable a rendre a
    // l'appelant.
    let paliers: Vec<u32> = [cote_max, 1120, 896, 768, 560, 392]
        .into_iter()
        .filter(|c| *c <= cote_max)
        .collect();
    let grand = img.width().max(img.height());
    let mut dernier: Option<Image> = None;

    for &cote in &paliers {
        if cote > grand && dernier.is_some() {
            continue; // deja plus petite que ce palier, inutile de repasser
        }
        // `thumbnail` plutot que `resize`: sur une reduction forte il est bien
        // plus rapide, pour un resultat que personne ne distingue une fois
        // l'image redecoupee en tuiles par le modele.
        let vue = if grand > cote {
            let f = cote as f32 / grand as f32;
            img.thumbnail(
                ((img.width() as f32 * f) as u32).max(1),
                ((img.height() as f32 * f) as u32).max(1),
            )
        } else {
            img.clone()
        };

        // D'abord PNG: les captures d'ecran sont pleines de texte fin, et il
        // reste net. Souvent suffisant une fois la taille divisee.
        if let Some(png) = encoder(&vue, image::ImageFormat::Png) {
            let png64 = base64::engine::general_purpose::STANDARD.encode(&png);
            if png64.len() <= b64_max {
                return Some(Image {
                    mime: "image/png".into(),
                    b64: png64,
                });
            }
        }

        // Sinon JPEG. Qualite 82: au-dessus le gain de taille s'effondre, en
        // dessous les artefacts commencent a manger les caracteres.
        let mut jpg = std::io::Cursor::new(Vec::new());
        if vue
            .to_rgb8()
            .write_with_encoder(image::codecs::jpeg::JpegEncoder::new_with_quality(
                &mut jpg, 82,
            ))
            .is_ok()
        {
            let jpg64 = base64::engine::general_purpose::STANDARD.encode(jpg.into_inner());
            let tient = jpg64.len() <= b64_max;
            dernier = Some(Image {
                mime: "image/jpeg".into(),
                b64: jpg64,
            });
            if tient {
                return dernier;
            }
        }
    }

    // Rien n'est passe sous la limite, meme a 392 px. On rend quand meme la
    // plus petite version obtenue: elle est loin sous l'originale, donc plus
    // proche de passer, et un refus du fournisseur n'est plus fatal.
    dernier
}

fn encoder(img: &image::DynamicImage, f: image::ImageFormat) -> Option<Vec<u8>> {
    let mut buf = std::io::Cursor::new(Vec::new());
    img.write_to(&mut buf, f).ok()?;
    Some(buf.into_inner())
}

/// Applique le gabarit a une conversation deja convertie pour le fournisseur.
///
/// Rend le nombre d'images reduites.
pub fn au_gabarit_conversation(msgs: &mut [serde_json::Value], serre: bool) -> usize {
    let (cote, plafond) = if serre {
        (COTE_SERRE, B64_SERRE)
    } else {
        (COTE_MAX, B64_MAX)
    };
    let mut n = 0usize;
    for m in msgs.iter_mut() {
        // `images`: la liste base64 nue (format Ollama).
        if let Some(arr) = m.get_mut("images").and_then(|v| v.as_array_mut()) {
            for img in arr.iter_mut() {
                let Some(b64) = img.as_str() else { continue };
                if let Some(r) = au_gabarit_borne("image/png", b64, cote, plafond) {
                    *img = serde_json::Value::String(r.b64);
                    n += 1;
                }
            }
        }
        // `attachments`: les pieces typees, d'ou sortent les blocs `image_url`.
        if let Some(arr) = m.get_mut("attachments").and_then(|v| v.as_array_mut()) {
            for p in arr.iter_mut() {
                if p.get("kind").and_then(|v| v.as_str()) != Some("image") {
                    continue;
                }
                let mime = p
                    .get("mime_type")
                    .or_else(|| p.get("mime"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("image/png")
                    .to_string();
                let Some(b64) = p.get("data").and_then(|v| v.as_str()) else {
                    continue;
                };
                if let Some(r) = au_gabarit_borne(&mime, b64, cote, plafond) {
                    p["data"] = serde_json::Value::String(r.b64);
                    p["mime_type"] = serde_json::Value::String(r.mime.clone());
                    if p.get("mime").is_some() {
                        p["mime"] = serde_json::Value::String(r.mime);
                    }
                    n += 1;
                }
            }
        }
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    /// PNG de bruit, assez gros pour ne pas se laisser compresser.
    /// Une capture d'ecran plausible: des aplats, des bandes, quelques bords
    /// nets. Ca ressemble a un bureau, et ca se compresse comme un bureau.
    fn capture_ecran(l: u32, h: u32) -> String {
        let mut img = image::RgbImage::new(l, h);
        for (x, y, p) in img.enumerate_pixels_mut() {
            let fenetre = x > l / 8 && x < l - l / 8 && y > h / 6 && y < h - h / 6;
            let barre = y % 37 < 2;
            *p = if barre && fenetre {
                image::Rgb([200, 200, 205])
            } else if fenetre {
                image::Rgb([24, 24, 27])
            } else {
                image::Rgb([9, 9, 11])
            };
        }
        let mut buf = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(img)
            .write_to(&mut buf, image::ImageFormat::Png)
            .unwrap();
        base64::engine::general_purpose::STANDARD.encode(buf.into_inner())
    }

    fn png_lourd(cote: u32) -> String {
        let mut img = image::RgbImage::new(cote, cote);
        let mut x = 0x12345678u32;
        for p in img.pixels_mut() {
            // Generateur trivial: il faut du bruit, pas du hasard de qualite.
            x = x.wrapping_mul(1664525).wrapping_add(1013904223);
            *p = image::Rgb([(x >> 16) as u8, (x >> 8) as u8, x as u8]);
        }
        let mut buf = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(img)
            .write_to(&mut buf, image::ImageFormat::Png)
            .unwrap();
        base64::engine::general_purpose::STANDARD.encode(buf.into_inner())
    }

    #[test]
    fn une_petite_image_nest_pas_touchee() {
        // Le point le plus important du module: ne rien abimer sans raison. Une
        // capture qui rentre doit partir exactement telle qu'elle est arrivee.
        let petite = png_lourd(64);
        assert!(petite.len() < B64_MAX);
        assert!(au_gabarit("image/png", &petite).is_none());
    }

    #[test]
    fn une_grosse_image_rentre_dans_le_gabarit() {
        // 3000 px: l'ordre de grandeur d'une capture d'ecran collee dans le
        // chat, celle qui faisait tronquer la requete.
        let grosse = png_lourd(3000);
        assert!(
            grosse.len() > B64_MAX,
            "le test ne prouve rien si l'image tient deja: {} octets",
            grosse.len()
        );
        let r = au_gabarit("image/png", &grosse).expect("doit etre reduite");
        assert!(
            r.b64.len() <= B64_MAX,
            "toujours trop grosse: {} octets en {}",
            r.b64.len(),
            r.mime
        );
        // Elle reste decodable: reduire ne doit pas produire un fichier casse.
        let brut = base64::engine::general_purpose::STANDARD
            .decode(&r.b64)
            .unwrap();
        let img = image::load_from_memory(&brut).unwrap();
        assert!(img.width().max(img.height()) <= COTE_MAX);
    }

    #[test]
    fn le_bruit_incompressible_descend_de_palier() {
        // Cas ou reduire une seule fois ne suffit pas: du bruit pur en 1400 px
        // est deja sous le cote maximum, et pese pourtant plus qu'une capture
        // de 3000 px. Sans les paliers, on rendait une image hors gabarit en
        // pretendant l'avoir mise dedans.
        let bruit = png_lourd(1400);
        assert!(bruit.len() > B64_MAX);
        let r = au_gabarit("image/png", &bruit).expect("doit etre reduite");
        assert!(
            r.b64.len() <= B64_MAX,
            "les paliers n'ont pas suffi: {} octets",
            r.b64.len()
        );
    }

    #[test]
    fn la_conversation_est_traitee_en_place() {
        let grosse = png_lourd(1400);
        let mut msgs = vec![serde_json::json!({
            "role": "user",
            "content": "regarde",
            "attachments": [
                {"kind": "image", "mime_type": "image/png", "data": grosse},
                {"kind": "audio", "mime_type": "audio/wav", "data": "AAAA"}
            ]
        })];
        assert_eq!(au_gabarit_conversation(&mut msgs, false), 1);
        let att = msgs[0]["attachments"].as_array().unwrap();
        assert!(att[0]["data"].as_str().unwrap().len() <= B64_MAX);
        // L'audio n'est pas une image et ne doit pas etre touche.
        assert_eq!(att[1]["data"], "AAAA");
    }

    #[test]
    /// Le cas mesure sur DeepSeek: une capture qui passe le controle PAR IMAGE
    /// et fait quand meme deborder la requete. C'est le corps entier qui compte.
    #[test]
    fn le_corps_entier_repasse_sous_le_plafond() {
        // L'ecran mesure dans le cas reel: 2560x1440, avant toute reduction.
        let gros = capture_ecran(2560, 1440);
        let mut msgs = vec![serde_json::json!({
            "role": "user",
            "content": "regarde",
            "attachments": [{"kind": "image", "mime_type": "image/png", "data": gros}],
        })];

        // Les schemas des outils pesent leur part: ils sont dans le corps aussi.
        let schemas = 8_000usize;
        let avant: usize = msgs
            .iter()
            .map(|m| serde_json::to_string(m).unwrap().len())
            .sum::<usize>()
            + schemas;
        assert!(avant > CORPS_MAX, "le cas de test doit deborder: {avant}");

        let apres = au_budget_corps(&mut msgs, schemas, CORPS_MAX);
        assert!(
            apres <= CORPS_MAX,
            "le corps doit repasser sous {CORPS_MAX}, obtenu {apres}"
        );

        // Et il reste une image: reduire n'est pas supprimer.
        let reste = msgs[0]["attachments"][0]["data"].as_str().unwrap_or("");
        assert!(!reste.is_empty(), "l'image ne doit pas disparaitre");
    }

    /// Un corps qui tient deja n'est pas touche: on n'abime pas une capture
    /// pour rien.
    #[test]
    fn un_corps_qui_tient_est_laisse_intact() {
        let petit = capture_ecran(64, 64);
        let mut msgs = vec![serde_json::json!({
            "role": "user",
            "content": "ok",
            "attachments": [{"kind": "image", "mime_type": "image/png", "data": petit.clone()}],
        })];
        au_budget_corps(&mut msgs, 1_000, CORPS_MAX);
        assert_eq!(msgs[0]["attachments"][0]["data"].as_str(), Some(petit.as_str()));
    }

    #[test]
    fn une_donnee_illisible_est_laissee_telle_quelle() {
        // Un format qu'on ne sait pas decoder peut etre un format que le
        // fournisseur accepte: on ne le detruit pas, on le laisse passer.
        let faux = "x".repeat(B64_MAX + 10);
        assert!(au_gabarit("image/webp", &faux).is_none());
    }
}
