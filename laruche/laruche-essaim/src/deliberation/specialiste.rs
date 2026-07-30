//! Le pool de specialistes: qui peut etre embauche pour une deliberation.
//!
//! Un specialiste n'est PAS une personnalite. C'est une **strategie de raisonnement**
//! posee au-dessus d'une constitution commune a tous (voir [`super::constitution`]).
//! La distinction n'est pas cosmetique: une personnalite fait varier le ton, une
//! strategie fait varier ce qui est cherche. Deux agents « sympathique » et « bourru »
//! donnent la meme reponse en deux voix; deux agents « cherche des preuves » et
//! « cherche des contre-exemples » donnent deux reponses.
//!
//! Comme pour les blueprints: un catalogue livre dans le binaire, plus les
//! specialistes crees par l'utilisateur dans `specialistes.json`, les deux fusionnes.
//! Un specialiste livre peut etre modifie ou renvoye; il n'est jamais impose.

use serde::{Deserialize, Serialize};

/// Ce qu'un specialiste peut faire au terme d'un tour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// Propose une position et la defend. Le cas courant.
    #[default]
    Contributeur,
    /// Attaque les positions des autres et n'en propose aucune.
    ///
    /// Contrainte non negociable qui l'accompagne: il doit enoncer ce qui le ferait
    /// changer d'avis. Une critique sans condition de refutation n'est pas utilisable -
    /// tout est critiquable, et l'arbitre n'aurait aucun moyen de distinguer une
    /// objection sérieuse d'un proces en general.
    Contradicteur,
    /// Compare, arbitre, fusionne. Interdiction d'inventer.
    Arbitre,
    /// Choisit qui parle et combien de tours. Ne resout rien lui-meme.
    Orchestrateur,
}

/// Un membre du pool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Specialiste {
    pub id: String,
    pub nom: String,
    /// Emoji ou data-URL. L'interface le montre a la table; le texte n'en depend pas.
    #[serde(default)]
    pub avatar: String,
    /// Couleur d'accent, pour distinguer les voix dans le transcript.
    #[serde(default)]
    pub couleur: String,
    pub role: Role,
    /// Une phrase, pour l'interface.
    #[serde(default)]
    pub mission: String,
    /// La strategie de raisonnement. C'est le SEUL endroit ou les specialistes
    /// divergent: la constitution, elle, est identique pour tous.
    pub strategie: String,
    /// Profil de fournisseur impose a ce specialiste. Vide = le profil actif.
    ///
    /// C'est un identifiant de `provider-profiles.json`, pas un couple
    /// fournisseur/modele ecrit a la main: on herite ainsi du coffre a secrets pour la
    /// cle, de la visibilite mesh, et de la possibilite de pointer un modele partage
    /// par une autre ruche - sans rien reimplementer ici.
    ///
    /// Le repli est volontairement silencieux et sur le profil ACTIF: un profil
    /// supprime ou une ruche eteinte ne doit pas empecher la deliberation, seulement la
    /// rendre moins diverse. L'interface signale le repli, le moteur ne s'arrete pas.
    ///
    /// C'est le levier le plus important du dispositif, et le moins evident: une
    /// constitution commune reduit la variance de FORME mais pas les biais PARTAGES.
    /// Dix strategies sur un meme modele de base ont les memes angles morts, et
    /// l'accord qui en sort mesure alors la conformite, pas la justesse. La diversite
    /// reelle vient de modeles differents - y compris ceux des autres ruches.
    #[serde(default, alias = "modele")]
    pub profil: String,
    /// Embauche pour les deliberations, ou en reserve dans le pool.
    #[serde(default = "vrai")]
    pub embauche: bool,
    /// Position a la table. L'interface s'en sert pour placer et remplacer.
    #[serde(default)]
    pub ordre: i32,
    /// Livre avec LaRuche. Un specialiste livre peut etre modifie; le drapeau sert
    /// seulement a l'interface, pour proposer un retour a la version d'origine.
    #[serde(default)]
    pub livre: bool,
}

fn vrai() -> bool {
    true
}

impl Specialiste {
    /// Doit-il enoncer ce qui le ferait changer d'avis ?
    ///
    /// Exige des contradicteurs, et de tout specialiste dont la strategie consiste a
    /// chercher des failles. Sans cette contrainte, un tour de critique produit du
    /// bruit que personne ne peut arbitrer.
    pub fn doit_falsifier(&self) -> bool {
        self.role == Role::Contradicteur
    }

    /// Sa position compte-t-elle dans la synthese finale ?
    ///
    /// Un contradicteur ne propose rien: le compter comme une voix « contre » sur une
    /// question ou il n'a pas de these fausserait l'accord affiche.
    pub fn conclut(&self) -> bool {
        matches!(self.role, Role::Contributeur | Role::Arbitre)
    }
}

/// Les specialistes livres avec LaRuche.
///
/// Sept, parce que c'est le point ou l'apport marginal d'un huitieme devient difficile
/// a justifier face a son cout: chaque participant ajoute un appel par tour, et les
/// tours de relecture voient le contexte grossir de tout ce qui a ete dit.
pub fn catalogue() -> Vec<Specialiste> {
    vec![
        Specialiste {
            id: "orchestrateur".into(),
            nom: "Orchestrateur".into(),
            avatar: "🎯".into(),
            couleur: "#f59e0b".into(),
            role: Role::Orchestrateur,
            mission: "Choisit qui parle, et arrete le debat quand le gain devient marginal.".into(),
            strategie: "Tu ne resous rien. Tu lis la demande et tu decides QUI doit y \
                travailler et COMBIEN de tours cela merite.\n\n\
                Une question factuelle ne merite pas une table ronde: un seul specialiste \
                et l'arbitre suffisent. Une question d'architecture merite le desaccord. \
                Une question de securite merite un attaquant.\n\n\
                Tu es responsable du COUT. Chaque participant ajoute un appel par tour, et \
                les tours de relecture font grossir le contexte de tout ce qui a ete dit. \
                Choisir six specialistes quand deux suffisent n'est pas de la prudence, \
                c'est du gaspillage que quelqu'un paie.\n\n\
                Tu arretes le debat quand les positions ont cesse de bouger, pas quand un \
                nombre de tours est atteint."
                .into(),
            profil: String::new(),
            embauche: true,
            ordre: 0,
            livre: true,
        },
        Specialiste {
            id: "scientifique".into(),
            nom: "Scientifique".into(),
            avatar: "🔬".into(),
            couleur: "#22d3ee".into(),
            role: Role::Contributeur,
            mission: "Cherche la verite, mesure l'incertitude.".into(),
            strategie: "Tu cherches ce qui est VERIFIABLE.\n\n\
                Tu remets en question les hypotheses, y compris celles de la question \
                posee. Tu cites tes preuves et tu nommes leur origine. Tu mesures ton \
                incertitude au lieu de l'arrondir.\n\n\
                Tu ne confonds pas « je n'ai pas trouve de contre-exemple » avec « c'est \
                vrai ». Une opinion largement partagee reste une opinion.\n\n\
                Quand une affirmation est testable, tu dis COMMENT la tester - un \
                benchmark, une mesure, une experience - plutot que d'argumenter."
                .into(),
            profil: String::new(),
            embauche: true,
            ordre: 1,
            livre: true,
        },
        Specialiste {
            id: "ingenieur".into(),
            nom: "Ingenieur systeme".into(),
            avatar: "⚙️".into(),
            couleur: "#4ade80".into(),
            role: Role::Contributeur,
            mission: "Transforme une bonne idee en systeme qui tient la charge.".into(),
            strategie: "Tu ne demandes jamais « est-ce que ca marche ». Tu demandes « est-ce \
                que ca tient a l'echelle, et a quel prix ».\n\n\
                Tu chiffres: memoire, calcul, latence, cout. Tu nommes le point de rupture \
                plutot que de dire qu'il y en aura un.\n\n\
                Tu distingues ce qui casse d'un coup de ce qui se degrade lentement - le \
                second est plus dangereux, parce que personne ne le remarque.\n\n\
                Tu identifies l'etat partage, les points de defaillance unique, et ce qui \
                se passe quand un composant repond lentement plutot que pas du tout."
                .into(),
            profil: String::new(),
            embauche: true,
            ordre: 2,
            livre: true,
        },
        Specialiste {
            id: "attaquant".into(),
            nom: "Attaquant".into(),
            avatar: "🗝️".into(),
            couleur: "#f87171".into(),
            role: Role::Contributeur,
            mission: "Cherche comment detourner, casser, exploiter.".into(),
            strategie: "Tu cherches l'usage que personne n'a prevu.\n\n\
                Comment detourner cette idee ? Comment l'attaquer ? Quel est le pire \
                scenario realiste, et qui en profite ?\n\n\
                Tu raisonnes en termes de confiance: qu'est-ce que ce systeme croit sans \
                verifier, et que se passe-t-il si cette croyance est fausse ?\n\n\
                Tu decris des scenarios CONCRETS et enchainables, pas des categories de \
                risque. « Injection possible » n'est pas une trouvaille; « ce champ arrive \
                dans une commande sans echappement, voici la charge » en est une."
                .into(),
            profil: String::new(),
            embauche: true,
            ordre: 3,
            livre: true,
        },
        Specialiste {
            id: "contradicteur".into(),
            nom: "Contradicteur".into(),
            avatar: "⚔️".into(),
            couleur: "#c084fc".into(),
            role: Role::Contradicteur,
            mission: "Considere que tout est faux, et dit ce qui le convaincrait.".into(),
            strategie: "Tu pars du principe que les autres se trompent, et tu cherches ou.\n\n\
                Tu traques les incoherences entre les positions, les contradictions \
                internes, et les raisons pour lesquelles cela peut echouer. Tu ne proposes \
                pas de solution: ce n'est pas ton travail.\n\n\
                MAIS chaque objection doit venir avec ce qui te ferait changer d'avis. Une \
                critique sans condition de refutation est inutilisable: tout est \
                critiquable, et l'arbitre ne pourrait pas distinguer une objection \
                sérieuse d'un proces en general.\n\n\
                Tu attaques les arguments, jamais les autres specialistes. Et si une \
                position resiste a ton examen, tu le dis: c'est un resultat, pas un echec."
                .into(),
            profil: String::new(),
            embauche: true,
            ordre: 4,
            livre: true,
        },
        Specialiste {
            id: "visionnaire".into(),
            nom: "Visionnaire".into(),
            avatar: "🌌".into(),
            couleur: "#818cf8".into(),
            role: Role::Contributeur,
            mission: "Ignore les contraintes actuelles, et marque ce qui est hors de portee."
                .into(),
            strategie: "Tu ignores volontairement ce qui est faisable aujourd'hui.\n\n\
                Tu proposes des architectures, des paradigmes, des chemins que personne n'a \
                pris. Tu as le droit de proposer l'impossible.\n\n\
                Mais tu dois le MARQUER, explicitement, a chaque fois: ce qui est faisable \
                maintenant, ce qui demande un travail important, ce qui n'existe pas encore. \
                Une idee hors de portee presentee comme disponible fait perdre des semaines \
                a quelqu'un.\n\n\
                Une idee inhabituelle n'a d'interet que si tu peux dire QUELLE contrainte \
                elle leve. Sinon c'est de l'originalite decorative."
                .into(),
            profil: String::new(),
            embauche: false,
            ordre: 5,
            livre: true,
        },
        Specialiste {
            id: "optimiseur".into(),
            nom: "Optimiseur".into(),
            avatar: "✂️".into(),
            couleur: "#fbbf24".into(),
            role: Role::Contributeur,
            mission: "Retire. Toujours.".into(),
            strategie: "Tu cherches ce qu'on peut enlever.\n\n\
                Moins de memoire, de calcul, de cout, de dependances, de complexite, de \
                jetons. Une piece retiree ne tombe jamais en panne.\n\n\
                Tu distingues la simplicite REELLE de la simplicite apparente: deplacer la \
                complexite dans une dependance ne la supprime pas, ca la rend seulement \
                invisible - et non corrigeable.\n\n\
                Quand tu proposes de retirer quelque chose, tu dis ce qu'on perd. Une \
                simplification dont le cout n'est pas nomme est une suppression deguisee."
                .into(),
            profil: String::new(),
            embauche: false,
            ordre: 6,
            livre: true,
        },
        Specialiste {
            id: "arbitre".into(),
            nom: "Arbitre".into(),
            avatar: "⚖️".into(),
            couleur: "#e7e7ea".into(),
            role: Role::Arbitre,
            mission: "Compare, arbitre, fusionne. N'invente rien.".into(),
            strategie: "Il t'est INTERDIT d'inventer. Tu ne produis aucune idee qui ne \
                figure pas dans ce que les specialistes ont dit.\n\n\
                Tu compares, tu arbitres, tu fusionnes. Quand deux positions \
                s'excluent, tu dis laquelle tu retiens ET pourquoi - tu ne les moyennes \
                pas.\n\n\
                Tu rends d'abord les DESACCORDS, ensuite les accords. C'est l'information \
                qu'une table ronde produit et qu'un modele seul ne donne jamais; l'enterrer \
                sous un resume ferait perdre tout l'interet du dispositif.\n\n\
                Tu ne confonds pas accord et justesse. Si les specialistes s'accordent sur \
                une base fragile, tu le signales: la verite passe avant le consensus."
                .into(),
            profil: String::new(),
            embauche: true,
            ordre: 99,
            livre: true,
        },
    ]
}

/// Le pool complet: catalogue livre, surcharge par `specialistes.json`.
///
/// Un specialiste du fichier qui porte l'`id` d'un specialiste livre le REMPLACE. C'est
/// ce qui permet de modifier un specialiste livre - avatar, strategie, modele, embauche -
/// sans perdre la possibilite de revenir a l'original en supprimant l'entree.
pub fn pool() -> Vec<Specialiste> {
    let mut tous = catalogue();
    for perso in charger_personnalises() {
        match tous.iter_mut().find(|s| s.id == perso.id) {
            Some(existant) => *existant = Specialiste { livre: true, ..perso },
            None => tous.push(perso),
        }
    }
    tous.sort_by_key(|s| s.ordre);
    tous
}

/// Les specialistes embauches, dans l'ordre de la table.
pub fn embauches() -> Vec<Specialiste> {
    pool().into_iter().filter(|s| s.embauche).collect()
}

pub fn charger_personnalises() -> Vec<Specialiste> {
    std::fs::read_to_string("specialistes.json")
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn sauver_personnalises(liste: &[Specialiste]) -> std::io::Result<()> {
    std::fs::write(
        "specialistes.json",
        serde_json::to_string_pretty(liste).unwrap_or_else(|_| "[]".into()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_catalogue_a_un_orchestrateur_et_un_arbitre() {
        let c = catalogue();
        assert_eq!(c.iter().filter(|s| s.role == Role::Orchestrateur).count(), 1);
        assert_eq!(c.iter().filter(|s| s.role == Role::Arbitre).count(), 1);
    }

    #[test]
    fn les_identifiants_sont_uniques() {
        let c = catalogue();
        let mut ids: Vec<&str> = c.iter().map(|s| s.id.as_str()).collect();
        ids.sort_unstable();
        let avant = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), avant, "identifiant en double dans le catalogue");
    }

    #[test]
    fn un_contradicteur_doit_falsifier_et_ne_conclut_pas() {
        let c = catalogue();
        let contra = c.iter().find(|s| s.role == Role::Contradicteur).unwrap();
        assert!(contra.doit_falsifier());
        // Il n'a pas de these: le compter comme une voix « contre » fausserait l'accord.
        assert!(!contra.conclut());
    }

    #[test]
    fn tous_les_livres_ont_avatar_couleur_et_strategie() {
        for s in catalogue() {
            assert!(!s.avatar.is_empty(), "{} sans avatar", s.id);
            assert!(!s.couleur.is_empty(), "{} sans couleur", s.id);
            assert!(!s.mission.is_empty(), "{} sans mission", s.id);
            // Une strategie courte est une personnalite deguisee: elle ne changerait
            // que le ton, pas ce qui est cherche.
            assert!(s.strategie.len() > 200, "{} : strategie trop maigre", s.id);
        }
    }

    #[test]
    fn un_profil_vide_veut_dire_repli_sur_l_actif() {
        // Aucun specialiste livre n'impose de profil: la diversite se configure, elle
        // ne s'impose pas a une installation qui n'a qu'un seul fournisseur.
        for s in catalogue() {
            assert!(s.profil.is_empty(), "{} impose un profil", s.id);
        }
    }

    #[test]
    fn l_ancien_champ_modele_est_encore_lu() {
        // Un specialiste enregistre avant le renommage doit continuer de fonctionner.
        let j = r#"{"id":"x","nom":"X","role":"contributeur",
                    "strategie":"s","modele":"deepseek-perso"}"#;
        let s: Specialiste = serde_json::from_str(j).unwrap();
        assert_eq!(s.profil, "deepseek-perso");
    }

    #[test]
    fn l_arbitre_passe_en_dernier() {
        let c = catalogue();
        let arbitre = c.iter().find(|s| s.role == Role::Arbitre).unwrap();
        assert!(
            c.iter()
                .filter(|s| s.role != Role::Arbitre)
                .all(|s| s.ordre < arbitre.ordre),
            "l'arbitre doit venir apres tous les autres"
        );
    }
}
