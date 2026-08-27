//! Une frappe part vers la fenetre au premier plan, et le modele doit savoir
//! laquelle.
//!
//! Sans cette phrase, `type` repondait "tape dans ce qui a le focus", ce qui est
//! exact et sans valeur. Un modele a ecrit trois phrases dans le navigateur en
//! croyant remplir le Bloc-notes reste sur l'autre ecran, et il lui a fallu
//! quatre commandes PowerShell pour comprendre.

use laruche_essaim::abeille::Abeille;
use laruche_essaim::abeille::ContextExecution;
use laruche_essaim::abeilles::ordinateur::Ordinateur;

#[tokio::test]
#[ignore = "requires a real desktop session"]
async fn une_frappe_dit_ou_elle_part() {
    // Texte vide: rien n'est tape, seul le compte rendu nous interesse.
    let out = Ordinateur
        .executer(
            serde_json::json!({ "action": "type", "text": "" }),
            &ContextExecution::default(),
        )
        .await
        .unwrap();
    assert!(out.success, "{:?}", out.error);
    assert!(
        out.output.contains("Focus: \""),
        "le compte rendu ne nomme pas la fenetre visee: {}",
        out.output
    );
}
