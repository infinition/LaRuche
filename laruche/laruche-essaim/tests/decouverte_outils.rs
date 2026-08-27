//! Un outil natif doit etre TROUVABLE, pas seulement present.
//!
//! Deux fois de suite, un outil parfaitement enregistre est reste invisible
//! pour le modele, et les deux fois le diagnostic a coute des heures parce que
//! l'outil, lui, repondait present a l'inspection. Ces tests tiennent les deux
//! bouts: il est dans le registre, ET une recherche par mots-cles le remonte.

use laruche_essaim::abeille::{ContextExecution, NiveauDanger};
use laruche_essaim::abeilles::{enregistrer_abeilles_builtin, run_script::ToolSearch};
use laruche_essaim::AbeilleRegistry;
use std::sync::Arc;

fn registre() -> Arc<AbeilleRegistry> {
    let r = Arc::new(AbeilleRegistry::new());
    enregistrer_abeilles_builtin(&r);
    // `tool_search` tient une reference au registre complet, il est donc pose
    // separement par `enregistrer_delegation` une fois celui-ci construit. On
    // reproduit juste ce geste, sans la delegation qui n'a rien a voir ici.
    r.enregistrer(Box::new(ToolSearch {
        registry: r.clone(),
    }));
    r
}

/// Le pilotage de la machine est natif depuis qu'il a remplace le serveur MCP
/// Python. Un `mcp_servers.json` qui exposerait encore un `computer` ne peut
/// plus le masquer (le registre refuse qu'un externe prenne le nom d'un natif),
/// mais encore faut-il que le natif soit la.
#[test]
#[cfg(feature = "gui-control")]
fn computer_est_un_outil_natif() {
    let r = registre();
    let outil = r
        .get("computer")
        .expect("`computer` doit etre enregistre parmi les natifs");
    assert_eq!(outil.niveau_danger(), NiveauDanger::NeedsApproval);
    assert_eq!(outil.origin(), laruche_essaim::abeille::ToolOrigin::Builtin);
}

/// La recherche par mots-cles doit remonter l'outil evident EN PREMIER.
///
/// C'est la requete exacte qu'un modele a lancee en production, sans jamais
/// voir `computer`: la recherche retenait tout ce qui contenait un seul de ces
/// mots, puis coupait a quinze dans l'ordre d'une table de hachage.
#[tokio::test]
#[cfg(feature = "gui-control")]
async fn la_recherche_remonte_loutil_evident_en_premier() {
    let r = registre();
    let recherche = r.get("tool_search").expect("tool_search est natif");
    let out = recherche
        .executer(
            serde_json::json!({ "query": "desktop control screenshot click mouse screen" }),
            &ContextExecution::default(),
        )
        .await
        .expect("la recherche ne doit pas echouer");

    assert!(out.success, "{:?}", out.error);
    let lignes: Vec<&str> = out.output.lines().filter(|l| l.starts_with("- ")).collect();
    assert!(
        lignes.iter().any(|l| l.starts_with("- computer ")),
        "`computer` absent des resultats:\n{}",
        out.output
    );
    // Et pas noye au milieu: un modele faible lit les premieres lignes.
    let rang = lignes
        .iter()
        .position(|l| l.starts_with("- computer "))
        .unwrap();
    assert!(
        rang < 3,
        "`computer` arrive en position {} sur cette requete:\n{}",
        rang + 1,
        out.output
    );
}

/// Le nom pese plus que la description, sinon n'importe quel outil mentionnant
/// "screen" dans six lignes de prose passe devant celui qui s'appelle comme la
/// chose cherchee.
#[tokio::test]
async fn le_nom_prime_sur_la_description() {
    let r = registre();
    let recherche = r.get("tool_search").unwrap();
    let out = recherche
        .executer(
            serde_json::json!({ "query": "browser" }),
            &ContextExecution::default(),
        )
        .await
        .unwrap();
    let premiere = out
        .output
        .lines()
        .find(|l| l.starts_with("- "))
        .unwrap_or_default();
    assert!(
        premiere.starts_with("- browser "),
        "premiere ligne inattendue: {premiere}\n{}",
        out.output
    );
}

/// Une recherche qui ne trouve rien le dit, plutot que de rendre le registre
/// entier faute de filtre.
#[tokio::test]
async fn une_recherche_sans_resultat_le_dit() {
    let r = registre();
    let out = r
        .get("tool_search")
        .unwrap()
        .executer(
            serde_json::json!({ "query": "zzzqwertyuiop" }),
            &ContextExecution::default(),
        )
        .await
        .unwrap();
    assert_eq!(out.output, "No matching tools.");
}
