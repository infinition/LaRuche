//! Executer les etapes: construire les prompts, appeler les modeles, lire les reponses.
//!
//! L'appel au modele passe par le trait [`Appel`] plutot que par un appel direct. Ce
//! n'est pas de l'abstraction gratuite: c'est ce qui permet de tester la boucle
//! complete - enchainement des tours, parallelisme, convergence, budget - sans reseau
//! ni cle d'API. La partie ou une erreur coute le plus cher est ainsi la mieux couverte.

use super::constitution::prompt_specialiste;
use super::moteur::{Deliberation, Etape};
use super::specialiste::Specialiste;
use super::tour::{self, Intervention};
use anyhow::Result;
use async_trait::async_trait;

/// De quoi appeler un modele. Resolu depuis un profil de fournisseur.
#[derive(Debug, Clone, Default)]
pub struct Creds {
    pub provider: String,
    pub model: String,
    pub api_key: String,
    pub api_base: Option<String>,
    pub ollama_url: String,
}

/// Un appel a un modele. Implemente pour de vrai par le noeud, en factice par les tests.
#[async_trait]
pub trait Appel: Send + Sync {
    /// Rend le texte produit et le nombre de jetons consommes.
    async fn demander(&self, creds: &Creds, systeme: &str, utilisateur: &str)
        -> Result<(String, u32)>;
}

/// Resout le profil d'un specialiste en identifiants utilisables.
///
/// Le repli est du ressort de l'implementation: un profil supprime, une ruche eteinte ou
/// un modele retire ne doivent pas interrompre la deliberation, seulement la rendre
/// moins diverse.
pub trait Profils: Send + Sync {
    fn resoudre(&self, profil: &str) -> Creds;
}

/// Rend le debat tel qu'un specialiste doit le lire.
///
/// On donne la DERNIERE position de chacun, pas l'historique complet: relire trois
/// versions successives d'un meme avis gonfle le contexte sans rien apprendre, et
/// pousse le modele a commenter l'evolution plutot que le fond.
fn transcript(d: &Deliberation, sauf: Option<&str>) -> String {
    let mut vu: Vec<&str> = Vec::new();
    let mut blocs: Vec<String> = Vec::new();
    for i in d.interventions.iter().rev() {
        if vu.contains(&i.specialiste.as_str()) {
            continue;
        }
        if Some(i.specialiste.as_str()) == sauf {
            continue;
        }
        vu.push(&i.specialiste);
        let nom = d
            .equipe
            .iter()
            .find(|s| s.id == i.specialiste)
            .map(|s| s.nom.as_str())
            .unwrap_or(&i.specialiste);
        blocs.push(format!(
            "## {nom}\naccord: {} · confiance: {}\n{}",
            i.accord.symbole(),
            i.confiance,
            i.position
        ));
    }
    blocs.reverse();
    blocs.join("\n\n")
}

/// La consigne propre a l'etape, en plus de la question.
fn consigne(etape: &Etape, d: &Deliberation, moi: &str) -> String {
    let q = &d.question;
    match etape {
        Etape::Solo(_) => format!(
            "Question posee:\n\n{q}\n\n\
             Tu travailles SEUL: tu ne verras les autres qu'au tour suivant. Donne ta \
             position initiale."
        ),
        Etape::Relecture(_) => format!(
            "Question posee:\n\n{q}\n\n\
             Voici la derniere position de chaque participant:\n\n{}\n\n\
             Revise la tienne. Si tu ne changes rien, dis-le et explique pourquoi les \
             arguments des autres ne t'ont pas convaincu. Si tu changes, nomme \
             precisement l'argument et son auteur.",
            transcript(d, Some(moi))
        ),
        Etape::Contradiction(_) => format!(
            "Question posee:\n\n{q}\n\n\
             Positions a attaquer:\n\n{}\n\n\
             Cherche les incoherences, les contradictions et les raisons d'echouer. Pour \
             CHAQUE objection, dis ce qui te ferait changer d'avis - une critique sans \
             condition de refutation est inutilisable. Si une position resiste, dis-le.",
            transcript(d, Some(moi))
        ),
        Etape::Reponse(_) => format!(
            "Question posee:\n\n{q}\n\n\
             Le debat jusqu'ici:\n\n{}\n\n\
             Reponds aux objections qui visent ta position. Concede ce qui doit l'etre - \
             une concession argumentee vaut mieux qu'une defense de principe - et tiens \
             ce qui tient.",
            transcript(d, Some(moi))
        ),
        Etape::Synthese(_) => format!(
            "Question posee:\n\n{q}\n\n\
             Positions finales:\n\n{}\n\n\
             Rends d'abord les DESACCORDS qui subsistent, puis les accords, puis ta \
             synthese. Tu n'inventes rien: tout ce que tu ecris doit venir de ce qui \
             precede. Si l'accord repose sur une base fragile, dis-le.",
            transcript(d, None)
        ),
        Etape::Fini(_) => String::new(),
    }
}

/// Qui doit parler a cette etape.
fn acteurs(etape: &Etape) -> Vec<String> {
    match etape {
        Etape::Solo(v) | Etape::Relecture(v) | Etape::Reponse(v) => v.clone(),
        Etape::Contradiction(id) | Etape::Synthese(id) => vec![id.clone()],
        Etape::Fini(_) => Vec::new(),
    }
}

/// Fait parler tout le monde pour une etape, en parallele quand ils sont plusieurs.
///
/// En parallele parce que les participants d'un meme tour ne se lisent pas: les faire
/// parler l'un apres l'autre multiplierait l'attente sans rien changer au resultat. Le
/// tour SUIVANT, lui, est sequentiel par construction - c'est la qu'ils se lisent.
pub async fn jouer_etape(
    d: &Deliberation,
    etape: &Etape,
    appel: &dyn Appel,
    profils: &dyn Profils,
    constitution: &str,
) -> Vec<Intervention> {
    let ids = acteurs(etape);
    if ids.is_empty() {
        return Vec::new();
    }
    let taches = ids.iter().map(|id| {
        let spec: Option<&Specialiste> = d.equipe.iter().find(|s| &s.id == id);
        let consigne = consigne(etape, d, id);
        async move {
            let spec = spec?;
            let systeme = prompt_specialiste(constitution, &spec.strategie);
            let creds = profils.resoudre(&spec.profil);
            match appel.demander(&creds, &systeme, &consigne).await {
                Ok((texte, jetons)) => {
                    let mut i = tour::lire(&spec.id, d.tour_courant, &texte)?;
                    i.jetons = jetons;
                    Some(i)
                }
                Err(e) => {
                    // Un specialiste muet ne fait pas echouer la table: les autres ont
                    // deja travaille, et une synthese a quatre voix vaut mieux que rien.
                    // L'absence se verra dans la repartition, ou il manquera.
                    tracing::warn!(
                        specialiste = %spec.id, error = %e,
                        "deliberation: un specialiste n'a pas repondu"
                    );
                    None
                }
            }
        }
    });
    futures_util::future::join_all(taches)
        .await
        .into_iter()
        .flatten()
        .collect()
}

/// Deroule la deliberation jusqu'a son terme.
pub async fn deliberer(
    d: &mut Deliberation,
    appel: &dyn Appel,
    profils: &dyn Profils,
    constitution: &str,
) {
    // Borne de securite independante de la logique d'arret: si une regression faisait
    // boucler `prochaine_etape`, le budget finirait par l'arreter - mais seulement apres
    // avoir depense le budget. Ce compteur, lui, ne coute rien.
    let mut garde = 0;
    loop {
        garde += 1;
        if garde > 32 {
            tracing::error!("deliberation: trop d'etapes, arret de securite");
            return;
        }
        let etape = d.prochaine_etape();
        if matches!(etape, Etape::Fini(_)) {
            return;
        }
        let lot = jouer_etape(d, &etape, appel, profils, constitution).await;
        if lot.is_empty() {
            // Personne n'a repondu a cette etape: insister ne servirait a rien.
            tracing::warn!("deliberation: etape sans reponse, arret");
            return;
        }
        d.deposer(lot);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::deliberation::moteur::{Mission, Plan, Reglages};
    use crate::deliberation::specialiste::catalogue;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct Stub {
        appels: AtomicUsize,
        /// Nombre de tours pendant lesquels les specialistes changent d'avis.
        tours_mouvants: usize,
    }

    #[async_trait]
    impl Appel for Stub {
        async fn demander(&self, _c: &Creds, _s: &str, u: &str) -> Result<(String, u32)> {
            let n = self.appels.fetch_add(1, Ordering::SeqCst);
            // On distingue les etapes par leur consigne, comme le ferait un modele.
            let change = n < self.tours_mouvants;
            let changement = if change { "l'argument de l'ingenieur" } else { "aucun" };
            let quoi = if u.contains("attaquer") { "objection" } else { "position" };
            Ok((
                format!(
                    "ACCORD: reserve\nCONFIANCE: 60\nCHANGEMENT: {changement}\n\
                     POSITION:\nune {quoi} argumentee."
                ),
                100,
            ))
        }
    }

    struct ProfilsStub;
    impl Profils for ProfilsStub {
        fn resoudre(&self, _p: &str) -> Creds {
            Creds::default()
        }
    }

    fn deli(ids: &[&str], tours: u8) -> Deliberation {
        Deliberation::nouvelle(
            "faut-il faire X ?",
            Plan {
                mission: Mission::Reponse,
                participants: ids.iter().map(|s| s.to_string()).collect(),
                tours_max: tours,
                raison: "test".into(),
            },
            Reglages::default(),
            catalogue(),
        )
    }

    #[tokio::test]
    async fn une_deliberation_converge_et_se_termine() {
        let mut d = deli(&["scientifique", "ingenieur"], 4);
        let appel = Stub { appels: AtomicUsize::new(0), tours_mouvants: 2 };
        deliberer(&mut d, &appel, &ProfilsStub, "CONSTITUTION").await;

        assert!(!d.interventions.is_empty());
        assert!(matches!(d.prochaine_etape(), Etape::Fini(_)));
        // L'arbitre a conclu.
        assert!(d.interventions.iter().any(|i| i.specialiste == "arbitre"));
    }

    #[tokio::test]
    async fn le_transcript_ne_montre_que_la_derniere_position_de_chacun() {
        let mut d = deli(&["scientifique"], 4);
        let appel = Stub { appels: AtomicUsize::new(0), tours_mouvants: 99 };
        // Deux tours pour le meme specialiste.
        let e = d.prochaine_etape();
        let lot = jouer_etape(&d, &e, &appel, &ProfilsStub, "C").await;
        d.deposer(lot);
        let e = d.prochaine_etape();
        let lot = jouer_etape(&d, &e, &appel, &ProfilsStub, "C").await;
        d.deposer(lot);

        let t = transcript(&d, None);
        assert_eq!(
            t.matches("## Scientifique").count(),
            1,
            "un participant ne doit apparaitre qu'une fois, avec sa derniere position"
        );
    }

    #[tokio::test]
    async fn un_specialiste_est_exclu_de_son_propre_transcript() {
        let mut d = deli(&["scientifique", "ingenieur"], 4);
        let appel = Stub { appels: AtomicUsize::new(0), tours_mouvants: 99 };
        let e = d.prochaine_etape();
        let lot = jouer_etape(&d, &e, &appel, &ProfilsStub, "C").await;
        d.deposer(lot);

        let t = transcript(&d, Some("scientifique"));
        assert!(!t.contains("## Scientifique"), "on ne se relit pas soi-meme");
        assert!(t.contains("## Ingenieur"));
    }

    #[tokio::test]
    async fn un_specialiste_muet_ne_fait_pas_echouer_la_table() {
        struct Muet;
        #[async_trait]
        impl Appel for Muet {
            async fn demander(&self, _c: &Creds, _s: &str, _u: &str) -> Result<(String, u32)> {
                Err(anyhow::anyhow!("fournisseur injoignable"))
            }
        }
        let mut d = deli(&["scientifique", "ingenieur"], 4);
        deliberer(&mut d, &Muet, &ProfilsStub, "C").await;
        // Aucune intervention, mais pas de panique ni de boucle.
        assert!(d.interventions.is_empty());
    }

    #[tokio::test]
    async fn la_consigne_solo_ne_montre_aucune_autre_position() {
        let d = deli(&["scientifique", "ingenieur"], 4);
        let c = consigne(&Etape::Solo(vec!["scientifique".into()]), &d, "scientifique");
        assert!(c.contains("SEUL"));
        assert!(!c.contains("##"), "aucun transcript au premier tour");
    }

    #[tokio::test]
    async fn la_synthese_voit_tout_le_monde_y_compris_l_arbitre_exclu_de_rien() {
        let mut d = deli(&["scientifique", "ingenieur"], 4);
        let appel = Stub { appels: AtomicUsize::new(0), tours_mouvants: 0 };
        let e = d.prochaine_etape();
        let lot = jouer_etape(&d, &e, &appel, &ProfilsStub, "C").await;
        d.deposer(lot);
        let c = consigne(&Etape::Synthese("arbitre".into()), &d, "arbitre");
        assert!(c.contains("## Scientifique"));
        assert!(c.contains("## Ingenieur"));
        assert!(c.contains("DESACCORDS"), "les desaccords d'abord");
        assert!(c.contains("n'inventes rien"));
    }

    #[tokio::test]
    async fn le_budget_arrete_meme_si_ca_bouge_encore() {
        let mut d = Deliberation::nouvelle(
            "q",
            Plan {
                mission: Mission::Reponse,
                participants: vec!["scientifique".into(), "ingenieur".into()],
                tours_max: 4,
                raison: "test".into(),
            },
            Reglages { jetons_max: 250, ..Default::default() },
            catalogue(),
        );
        // Le stub fait toujours changer d'avis: sans plafond, ca ne convergerait jamais.
        let appel = Stub { appels: AtomicUsize::new(0), tours_mouvants: usize::MAX };
        deliberer(&mut d, &appel, &ProfilsStub, "C").await;
        assert!(matches!(d.prochaine_etape(), Etape::Fini(_)));
        assert!(
            d.interventions.iter().any(|i| i.specialiste == "arbitre"),
            "meme budget epuise, la synthese doit avoir lieu"
        );
    }
}
