//! Live test for `web_discover` against its reference case.
//!
//! `ds.lordtry.com` is the failure this tool was built from: a 2016 HTTrack
//! mirror whose `/file/` directory holds five `.dsparty` saves that no agent
//! could reach. Three traps stack there - a `href="#"` JS menu, a directory
//! whose `index.html` hides the Apache listing, and an English singular
//! directory name on a French site.
//!
//! Ignored by default: it hits the network, and a third-party site going down
//! must never turn the suite red. Run it on purpose:
//!
//! ```text
//! cargo test -p laruche-essaim --test web_discover_vivant -- --ignored --nocapture
//! ```

use laruche_essaim::abeille::{Abeille, ContextExecution};
use laruche_essaim::abeilles::web_discover::WebDiscover;

const CIBLE: &str = "https://www.ds.lordtry.com/";

/// The five saves that started all this, by file name.
const SAUVEGARDES: &[&str] = &[
    "bijoux_1.dsparty",
    "cristal.dsparty",
    "serpent.dsparty",
    "temp.dsparty",
];

#[tokio::test]
#[ignore = "hits the network"]
async fn trouve_les_sauvegardes_que_les_autres_outils_ratent() {
    let outil = WebDiscover;
    let sortie = outil
        .executer(
            serde_json::json!({
                "url": CIBLE,
                "mode": "auto",
                "ext": "dsparty",
                "max_results": 50
            }),
            &ContextExecution::default(),
        )
        .await
        .expect("tool must not error out")
        .output;

    println!("{sortie}");

    for nom in SAUVEGARDES {
        assert!(
            sortie.contains(nom),
            "missing {nom} - the reference case regressed"
        );
    }
    // Reporting them is not enough: they must be CONFIRMED live, which is the
    // whole difference between this tool and a wordlist that guesses.
    assert!(
        sortie.contains("LIVE, confirmed"),
        "no live section in the output"
    );
}

/// The archive channel alone should land the saves: one request, no JS, no
/// wordlist, no load on the target. It is the cheapest path to the answer.
#[tokio::test]
#[ignore = "hits the network"]
async fn le_canal_archive_suffit_a_lui_seul() {
    let outil = WebDiscover;
    let sortie = outil
        .executer(
            serde_json::json!({ "url": CIBLE, "mode": "archive", "ext": "dsparty" }),
            &ContextExecution::default(),
        )
        .await
        .expect("tool must not error out")
        .output;

    println!("{sortie}");
    assert!(
        sortie.contains("temp.dsparty"),
        "the CDX index no longer yields the saves"
    );
}

/// `/file/` serves an `index.html`, so the Apache listing is hidden. The tool
/// must fall back to reading that page's links instead of reporting nothing.
#[tokio::test]
#[ignore = "hits the network"]
async fn un_repertoire_sans_listing_livre_quand_meme_ses_liens() {
    let outil = WebDiscover;
    let sortie = outil
        .executer(
            serde_json::json!({
                "url": "https://www.ds.lordtry.com/file/",
                "mode": "listing",
                "ext": "dsparty"
            }),
            &ContextExecution::default(),
        )
        .await
        .expect("tool must not error out")
        .output;

    println!("{sortie}");
    assert!(
        sortie.contains(".dsparty"),
        "index.html fallback failed on a listing-less directory"
    );
}
