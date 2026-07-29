//! Le QR du demarrage doit tenir dans une fenetre de terminal ordinaire. Un code
//! illisible ou coupe ne sert a rien : c'est le seul chemin vers un telephone qu'une
//! installation neuve propose.
use qrcode::render::unicode;
use qrcode::{EcLevel, QrCode};

fn rendu(url: &str) -> String {
    let code = QrCode::with_error_correction_level(url.as_bytes(), EcLevel::L).unwrap();
    code.render::<unicode::Dense1x2>()
        .dark_color(unicode::Dense1x2::Light)
        .light_color(unicode::Dense1x2::Dark)
        .quiet_zone(true)
        .build()
}

#[test]
fn tient_dans_quatre_vingts_colonnes() {
    for url in [
        "http://192.168.1.42:8419",
        "https://192.168.100.200:8419",
        "http://10.0.0.7:8419",
    ] {
        let r = rendu(url);
        let lignes: Vec<&str> = r.lines().collect();
        let large = lignes.iter().map(|l| l.chars().count()).max().unwrap();
        // 4 colonnes d'indentation sont ajoutees a l'affichage.
        assert!(large + 4 <= 80, "{url}: QR trop large ({large} colonnes)");
        assert!(lignes.len() <= 24, "{url}: QR trop haut ({} lignes)", lignes.len());
        println!("  {url:32} -> {large} col x {} lig", lignes.len());
    }
}
