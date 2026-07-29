//! Genere toutes les icones du projet a partir de l'unique source `icon.svg`.
//!
//! Le depot ne contient qu'une seule definition de l'icone. Les PNG (manifeste PWA,
//! coque Tauri) et le `.ico` (fenetre et installeur Windows) en sont derives ici, ce
//! qui evite d'avoir a retoucher huit fichiers a la main quand le logo bouge - et
//! evite surtout qu'ils se mettent a diverger en silence.
//!
//! Ce binaire ne fait pas partie de la construction par defaut. Il se lance a la
//! demande :
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

/// Rend le SVG dans un bitmap carre de `taille` pixels de cote.
///
/// `usvg` conserve le ratio; l'icone est carree (viewBox 512x512), donc la
/// transformation est un simple facteur d'echelle.
fn rendre(arbre: &usvg::Tree, taille: u32) -> tiny_skia::Pixmap {
    let mut pixmap = tiny_skia::Pixmap::new(taille, taille).expect("taille d'icone non nulle");
    let echelle = taille as f32 / arbre.size().width();
    resvg::render(
        arbre,
        tiny_skia::Transform::from_scale(echelle, echelle),
        &mut pixmap.as_mut(),
    );
    pixmap
}

fn ecrire_png(pixmap: &tiny_skia::Pixmap, chemin: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = chemin.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(chemin, pixmap.encode_png()?)?;
    println!("  {:>4}px  {}", pixmap.width(), chemin.display());
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let racine = racine();
    let source = racine.join("laruche-dashboard/src/templates/icon.svg");
    let donnees = std::fs::read(&source)?;
    let arbre = usvg::Tree::from_data(&donnees, &usvg::Options::default())?;
    println!("source: {}\n", source.display());

    // 1. PWA. Les navigateurs refusent un SVG pour l'icone installee sur le bureau:
    //    Windows et Android veulent du bitmap, et c'est la seule raison pour laquelle
    //    « Installer LaRuche » ne donnait pas d'icone correcte dans la barre des taches.
    let web = racine.join("laruche-dashboard/src/templates/icones");
    for taille in [192u32, 512] {
        ecrire_png(&rendre(&arbre, taille), &web.join(format!("icon-{taille}.png")))?;
    }

    // 2. Coque Tauri. Les noms sont imposes par tauri.conf.json.
    let bureau = racine.join("laruche-bureau/icons");
    for (taille, nom) in [
        (32u32, "32x32.png"),
        (128, "128x128.png"),
        (256, "128x128@2x.png"),
        (512, "icon.png"),
    ] {
        ecrire_png(&rendre(&arbre, taille), &bureau.join(nom))?;
    }

    // 3. ICO Windows: fenetre, barre des taches et installeur. Plusieurs resolutions
    //    dans un seul fichier, sinon Windows redimensionne lui-meme et le resultat
    //    bave aux petites tailles.
    let mut paquet = ico::IconDir::new(ico::ResourceType::Icon);
    for taille in [16u32, 32, 48, 64, 128, 256] {
        let pixmap = rendre(&arbre, taille);
        let image = ico::IconImage::from_rgba_data(taille, taille, pixmap.data().to_vec());
        paquet.add_entry(ico::IconDirEntry::encode(&image)?);
    }
    let ico = bureau.join("icon.ico");
    paquet.write(std::fs::File::create(&ico)?)?;
    println!("  multi   {}", ico.display());

    Ok(())
}
