//! Genere toutes les icones du projet a partir des SVG de `icones-source/`.
//!
//! Une famille, cinq variantes, pour qu'on distingue les binaires dans la barre des
//! taches et le gestionnaire de taches sans lire leur nom:
//!
//! | fichier      | binaire                | signe distinctif                    |
//! |--------------|------------------------|-------------------------------------|
//! | `node.svg`   | laruche-node           | les trois alveoles, ambre: la ruche  |
//! | `bureau.svg` | laruche-bureau         | les alveoles dans un cadre: une vue  |
//! | `client.svg` | LaRuche Client         | cadre + alveole distante, cyan       |
//! | `cli.svg`    | laruche (CLI/TUI)      | une alveole + un prompt, vert        |
//! | `evals.svg`  | laruche-evals          | une alveole + une courbe, violet     |
//!
//! `node.svg` est la reference et reste identique a `icon.svg` du tableau de bord,
//! qui sert l'interface web et la PWA.
//!
//! Ce binaire ne fait pas partie de la construction par defaut:
//!
//! ```text
//! cargo run -p laruche-icones
//! ```

use std::path::{Path, PathBuf};

/// Racine du depot `laruche/`, deduite de l'emplacement de ce crate.
fn racine() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("outils/icones se trouve deux niveaux sous laruche/")
        .to_path_buf()
}

/// D'ou vient une icone.
///
/// Le depot est parti de SVG, ce qui est la bonne matiere pour une icone: une
/// seule definition, nette a toutes les tailles. Mais une icone dessinee
/// ailleurs arrive en PNG, et refuser le PNG obligerait a la vectoriser pour
/// rien, ou a deposer des `.ico` binaires a la main dans le depot sans plus
/// aucune source. Les deux sont acceptes, le PNG l'emporte s'il existe: c'est
/// lui qu'on vient de deposer.
enum Source {
    Vecteur(usvg::Tree),
    Bitmap(image::DynamicImage),
}

/// Rend la source dans un bitmap carre de `taille` pixels de cote.
fn rendre(source: &Source, taille: u32) -> tiny_skia::Pixmap {
    let mut pixmap = tiny_skia::Pixmap::new(taille, taille).expect("taille d'icone non nulle");
    match source {
        Source::Vecteur(arbre) => {
            let echelle = taille as f32 / arbre.size().width();
            resvg::render(
                arbre,
                tiny_skia::Transform::from_scale(echelle, echelle),
                &mut pixmap.as_mut(),
            );
        }
        Source::Bitmap(img) => {
            // Lanczos3, et pas le voisin le plus proche: une icone passe de
            // 1024 px a 16 px, et c'est a 16 px qu'on la regarde le plus
            // souvent. Un filtre grossier la rend illisible exactement la ou
            // elle doit etre reconnaissable d'un coup d'oeil.
            let petite = img.resize_exact(taille, taille, image::imageops::FilterType::Lanczos3);
            pixmap
                .data_mut()
                .copy_from_slice(&petite.to_rgba8().into_raw());
        }
    }
    pixmap
}

/// Charge `nom.png` s'il existe, sinon `nom.svg`.
fn charger(dossier: &Path, nom: &str) -> Result<Source, Box<dyn std::error::Error>> {
    let png = dossier.join(format!("{nom}.png"));
    if png.exists() {
        return Ok(Source::Bitmap(image::open(&png)?));
    }
    let svg = dossier.join(format!("{nom}.svg"));
    let donnees = std::fs::read(&svg)
        .map_err(|e| format!("ni {} ni {}: {e}", png.display(), svg.display()))?;
    Ok(Source::Vecteur(usvg::Tree::from_data(
        &donnees,
        &usvg::Options::default(),
    )?))
}

fn ecrire_png(pixmap: &tiny_skia::Pixmap, chemin: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = chemin.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(chemin, pixmap.encode_png()?)?;
    Ok(())
}

/// Ecrit un `.ico` multi-resolution.
///
/// Plusieurs tailles dans un seul fichier: sinon Windows redimensionne lui-meme et le
/// resultat bave a 16 px, la taille ou l'icone est justement le plus lue.
fn ecrire_ico(source: &Source, chemin: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut paquet = ico::IconDir::new(ico::ResourceType::Icon);
    for taille in [16u32, 32, 48, 64, 128, 256] {
        let pixmap = rendre(source, taille);
        let image = ico::IconImage::from_rgba_data(taille, taille, pixmap.data().to_vec());
        paquet.add_entry(ico::IconDirEntry::encode(&image)?);
    }
    if let Some(parent) = chemin.parent() {
        std::fs::create_dir_all(parent)?;
    }
    paquet.write(std::fs::File::create(chemin)?)?;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let racine = racine();
    let sources = racine.join("icones-source");

    // 1. La ruche (node) alimente aussi l'interface web et la PWA.
    let node = charger(&sources, "node")?;
    let web = racine.join("laruche-dashboard/src/templates/icones");
    for taille in [192u32, 512] {
        // Les navigateurs refusent un SVG pour l'icone installee: Windows et Android
        // veulent du bitmap, et c'est la seule raison pour laquelle « Installer
        // LaRuche » ne donnait pas d'icone correcte dans la barre des taches.
        ecrire_png(&rendre(&node, taille), &web.join(format!("icon-{taille}.png")))?;
    }
    ecrire_ico(&node, &racine.join("laruche-node/icone.ico"))?;
    println!("  node    -> PWA 192/512 + laruche-node/icone.ico");

    // 2. La coque de bureau: jeu complet impose par tauri.conf.json.
    let bureau = charger(&sources, "bureau")?;
    let dossier = racine.join("laruche-bureau/icons");
    for (taille, nom) in [
        (32u32, "32x32.png"),
        (128, "128x128.png"),
        (256, "128x128@2x.png"),
        (512, "icon.png"),
    ] {
        ecrire_png(&rendre(&bureau, taille), &dossier.join(nom))?;
    }
    ecrire_ico(&bureau, &dossier.join("icon.ico"))?;
    println!("  bureau  -> laruche-bureau/icons/ (jeu Tauri complet)");

    // 3. Le client: meme jeu, dossier separe, pour que l'installeur client ne porte
    //    pas l'icone de l'application complete - c'est tout l'inverse de ce qu'il est.
    let client = charger(&sources, "client")?;
    let dossier_client = racine.join("laruche-bureau/icons-client");
    for (taille, nom) in [
        (32u32, "32x32.png"),
        (128, "128x128.png"),
        (256, "128x128@2x.png"),
        (512, "icon.png"),
    ] {
        ecrire_png(&rendre(&client, taille), &dossier_client.join(nom))?;
    }
    ecrire_ico(&client, &dossier_client.join("icon.ico"))?;
    println!("  client  -> laruche-bureau/icons-client/");

    // 4. L'extension Chrome. Elle porte l'icone de l'APPLICATION et non celle du
    //    noeud: dans la barre d'outils du navigateur, elle represente LaRuche
    //    aux yeux de la personne, pas le service qui tourne derriere. Trois
    //    tailles imposees par Chrome, et elles vivent hors de `laruche/`.
    if let Some(depot) = racine.parent() {
        let ext = depot.join("extension-chrome/icones");
        if ext.exists() {
            for taille in [16u32, 48, 128] {
                ecrire_png(
                    &rendre(&bureau, taille),
                    &ext.join(format!("icon-{taille}.png")),
                )?;
            }
            println!("  ext     -> extension-chrome/icones/ (16, 48, 128)");
        }
    }

    // 5. Les binaires en ligne de commande: un .ico chacun, a cote de leur crate.
    for (nom, cible) in [
        ("cli", "laruche-cli/icone.ico"),
        ("evals", "laruche-evals/icone.ico"),
    ] {
        let source = charger(&sources, nom)?;
        ecrire_ico(&source, &racine.join(cible))?;
        println!("  {nom:<7} -> {cible}");
    }

    Ok(())
}
