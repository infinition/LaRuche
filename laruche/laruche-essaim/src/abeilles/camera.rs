//! `camera`: une image de la webcam, rendue au modele.
//!
//! Le chemin est celui de `screenshot`: on capture, on encode en PNG, et on
//! rend l'image dans le resultat pour qu'elle apparaisse dans la conversation.
//! Ce qui change, c'est ce qu'on filme.
//!
//! # Pourquoi une dependance et pas trois implementations
//!
//! Il n'existe aucune API portable pour lire une camera. C'est Media Foundation
//! sur Windows, AVFoundation sur macOS, V4L2 sur Linux, trois modeles d'objets
//! sans rien de commun. `nokhwa` met une facade unique par-dessus les trois, et
//! c'est exactement le genre de probleme ou une caisse vaut mieux que du code a
//! maintenir sur trois piles video qu'on ne teste jamais toutes.
//!
//! Derriere la feature `camera`, HORS du defaut: un noeud sans camera, un
//! serveur, une image CI, n'ont pas a compiler des liaisons vers trois piles
//! video pour un outil qu'ils n'appelleront jamais.
//!
//! # Ce qui compte plus que le code
//!
//! Allumer la camera de quelqu'un est le geste le plus intrusif que cet agent
//! puisse poser. Plus qu'un clic, plus qu'une lecture de fichier: il y a
//! quelqu'un devant, et il ne l'a pas forcement demande a cet instant.
//!
//! Trois choses en decoulent, et aucune n'est negociable:
//!
//!   - l'outil demande une approbation, comme `shell_exec` ou `computer`;
//!   - la camera est RELACHEE des la prise. La garder ouverte laisserait la
//!     diode allumee, bloquerait les autres applications, et transformerait une
//!     photo en surveillance;
//!   - une seule image par appel. Il n'y a pas de mode continu, et ce n'est pas
//!     un oubli: un outil qui filme en continu n'est pas le meme objet, et il ne
//!     se decide pas dans un coin de fichier.
//!
//! La diode materielle de la webcam reste la seule garantie qui ne depend pas de
//! nous. C'est bien ainsi, et c'est une raison de plus de relacher vite.

use crate::abeille::{Abeille, ContextExecution, NiveauDanger, ResultatAbeille};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_json::{json, Value};

use nokhwa::pixel_format::RgbFormat;
use nokhwa::utils::{ApiBackend, CameraIndex, RequestedFormat, RequestedFormatType};
use nokhwa::Camera;

/// Images jetees avant de garder la bonne.
///
/// La premiere image d'une webcam est presque toujours noire ou grise: le
/// capteur n'a pas fini son auto-exposition ni sa balance des blancs. Rendre
/// cette image la donnerait au modele une piece sombre et une reponse fausse,
/// sans que rien ne signale le probleme. Cinq images suffisent sur tout ce qui a
/// ete essaye, et coutent deux dixiemes de seconde.
const IMAGES_DE_CHAUFFE: usize = 5;

pub struct AbeilleCamera;

/// Liste ce que la machine expose comme cameras.
fn lister() -> Result<Vec<(u32, String)>> {
    let infos = nokhwa::query(ApiBackend::Auto)
        .map_err(|e| anyhow!("cannot enumerate cameras: {e}"))?;
    Ok(infos
        .into_iter()
        .map(|i| {
            let index = match i.index() {
                CameraIndex::Index(n) => *n,
                CameraIndex::String(_) => 0,
            };
            (index, i.human_name())
        })
        .collect())
}

/// Prend UNE image, et referme.
fn capturer(index: u32) -> Result<(Vec<u8>, u32, u32)> {
    let format = RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestResolution);
    let mut camera = Camera::new(CameraIndex::Index(index), format).map_err(|e| {
        anyhow!(
            "cannot open camera {index}: {e}. On macOS and Windows the user must have granted \
             camera access to the application running LaRuche; on Linux the user must be able \
             to read /dev/video{index}."
        )
    })?;
    camera
        .open_stream()
        .map_err(|e| anyhow!("cannot start the camera stream: {e}"))?;

    // On jette les premieres images, puis on garde la suivante.
    let mut derniere = None;
    for _ in 0..=IMAGES_DE_CHAUFFE {
        derniere = Some(
            camera
                .frame()
                .map_err(|e| anyhow!("cannot read a frame: {e}"))?,
        );
    }
    let image = derniere
        .ok_or_else(|| anyhow!("the camera returned no frame"))?
        .decode_image::<RgbFormat>()
        .map_err(|e| anyhow!("cannot decode the frame: {e}"))?;

    // Relachee AVANT l'encodage: l'encodage prend du temps, et la diode n'a
    // aucune raison de rester allumee pendant qu'on compresse un PNG.
    drop(camera);

    let (l, h) = (image.width(), image.height());
    let mut png = std::io::Cursor::new(Vec::new());
    xcap::image::DynamicImage::ImageRgb8(image)
        .write_to(&mut png, xcap::image::ImageFormat::Png)
        .map_err(|e| anyhow!("cannot encode the capture: {e}"))?;
    Ok((png.into_inner(), l, h))
}

#[async_trait]
impl Abeille for AbeilleCamera {
    fn nom(&self) -> &str {
        "camera"
    }

    fn description(&self) -> &str {
        "Take ONE still image from a camera attached to this machine and show it to you, so \
         you can answer questions about what is physically in front of it. \
         Actions: list (which cameras exist) and capture (take the picture, default). \
         `index` picks a camera when there are several, from list; the default is 0. \
         There is no continuous mode and no video: one call, one frame, and the camera is \
         released immediately. \
         This is the most intrusive thing you can do on this machine: there is a person in \
         front of it. Use it when the user asked to look at something, never to check on \
         them, and say plainly what you saw and what you could not make out."
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["capture", "list"],
                    "description": "capture: one still image, the default. list: the cameras this machine exposes, with the index to pass back."
                },
                "index": {
                    "type": "integer",
                    "description": "Which camera, from list. Default 0, which is the built-in one on a laptop."
                }
            }
        })
    }

    fn niveau_danger(&self) -> NiveauDanger {
        // Photographier quelqu'un chez lui n'est pas une action de routine.
        NiveauDanger::NeedsApproval
    }

    async fn executer(&self, args: Value, _ctx: &ContextExecution) -> Result<ResultatAbeille> {
        if std::env::var("LARUCHE_CAMERA").as_deref() == Ok("0") {
            return Ok(ResultatAbeille::err(
                "The camera is disabled on this node (LARUCHE_CAMERA=0).".to_string(),
            ));
        }
        // Copies AVANT le passage sur le pool: la tache bloquante survit a
        // l'emprunt de `args`.
        let action = args["action"].as_str().unwrap_or("capture").to_string();
        let index = args["index"].as_u64().unwrap_or(0) as u32;

        // Bloquant, et pas qu'un peu: ouvrir un peripherique video prend des
        // centaines de millisecondes. Sur le pool bloquant, comme la capture
        // d'ecran et le pilotage.
        match tokio::task::spawn_blocking(move || -> Result<ResultatAbeille> {
            if action == "list" {
                let cams = lister()?;
                if cams.is_empty() {
                    return Ok(ResultatAbeille::ok(
                        "No camera detected on this machine.".to_string(),
                    ));
                }
                let lignes: Vec<String> = cams
                    .iter()
                    .map(|(i, nom)| format!("index {i}: {nom}"))
                    .collect();
                return Ok(ResultatAbeille::ok(format!(
                    "{} camera(s):\n{}\n\nPass `index` to capture.",
                    cams.len(),
                    lignes.join("\n")
                )));
            }

            let (png, l, h) = capturer(index)?;
            let mut out = ResultatAbeille::ok(format!(
                "One frame from camera {index}, {l}x{h}, shown to you. The camera is already \
                 released. Describe what you actually see, and say what is too dark or too \
                 blurred to make out rather than guessing it."
            ));
            out.images = vec![{
                use base64::Engine;
                base64::engine::general_purpose::STANDARD.encode(png)
            }];
            Ok(out)
        })
        .await
        {
            Ok(Ok(r)) => Ok(r),
            Ok(Err(e)) => Ok(ResultatAbeille::err(e.to_string())),
            Err(e) => Ok(ResultatAbeille::err(format!("camera task failed: {e}"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_schema_dit_ce_que_loutil_fait_et_ne_fait_pas() {
        let s = AbeilleCamera.schema();
        let actions = s["properties"]["action"]["enum"].as_array().unwrap();
        assert_eq!(actions.len(), 2, "capture et list, rien de plus");

        // L'absence de mode continu est une decision, pas un oubli: si un jour
        // quelqu'un l'ajoute, ce test lui demandera d'y penser deux fois.
        let d = AbeilleCamera.description();
        assert!(d.contains("no continuous mode"));
        assert!(d.contains("released immediately"));
    }

    #[test]
    fn lister_ne_panique_pas_sans_camera() {
        // Sur une machine sans camera, sans pilote, ou dans une image CI,
        // l'enumeration doit rendre une erreur ou une liste vide, jamais paniquer.
        let _ = lister();
    }
}
