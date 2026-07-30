//! L'enchainement des tours: qui parle, dans quel ordre, et quand on s'arrete.
//!
//! Le moteur ne parle a aucun modele lui-meme. Il decide de la SEQUENCE et laisse
//! l'appelant executer chaque tour - c'est ce qui le rend testable sans reseau, et le
//! seul moyen d'avoir des tests sur la partie ou les erreurs coutent le plus cher.

use super::specialiste::{Role, Specialiste};
use super::tour::{Accord, Intervention};
use serde::{Deserialize, Serialize};

/// Ce sur quoi la table ronde travaille.
///
/// Une deliberation ne produit pas toujours une reponse: elle peut produire un
/// correctif, un rapport source, ou un resultat d'experience. Le type de mission change
/// trois choses - qui est embauche par defaut, quels outils sont ouverts, et ce qui est
/// rendu a la fin. Le reste du dispositif (constitution, tours, arbitrage) ne bouge pas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Mission {
    /// Repondre. Le livrable est un texte et ses desaccords.
    #[default]
    Reponse,
    /// Ecrire ou corriger du code local. Le livrable est un diff.
    Code,
    /// Chercher dans des sources. Le livrable est un rapport source.
    Recherche,
    /// Executer pour savoir. Le livrable est un resultat reproductible.
    Experimentation,
}

impl Mission {
    /// Qui est embauche par defaut pour ce type de mission.
    ///
    /// L'orchestrateur peut s'en ecarter - c'est son role - mais partir d'une equipe
    /// adaptee lui evite de la reconstruire a chaque fois, et donne un comportement
    /// previsible quand il n'est pas consulte.
    pub fn equipe_par_defaut(&self) -> &'static [&'static str] {
        match self {
            // Le contradicteur est de toutes les equipes: c'est lui qui empeche la
            // table de se mettre d'accord trop vite, et c'est le principal risque.
            Mission::Reponse => &["scientifique", "contradicteur"],
            Mission::Code => &["ingenieur", "attaquant", "contradicteur", "optimiseur"],
            Mission::Recherche => &["scientifique", "contradicteur", "visionnaire"],
            Mission::Experimentation => &["scientifique", "ingenieur", "contradicteur"],
        }
    }

    /// Ce que la mission exige comme acces, en clair pour l'interface.
    ///
    /// Une deliberation qui ecrit des fichiers ou lance du code n'est pas de la meme
    /// nature qu'une deliberation qui reflechit: l'utilisateur doit le savoir avant de
    /// lancer, pas le decouvrir apres.
    pub fn acces(&self) -> &'static str {
        match self {
            Mission::Reponse => "lecture seule",
            Mission::Code => "lecture et ecriture de fichiers",
            Mission::Recherche => "acces reseau",
            Mission::Experimentation => "execution de code",
        }
    }

    /// Les outils sont-ils REELLEMENT ouverts pour cette mission ?
    ///
    /// Aujourd'hui: non, pour aucune. Les specialistes raisonnent et rendent du texte;
    /// ils n'ecrivent aucun fichier et ne consultent aucune page. `acces()` decrit ce
    /// que la mission exigera, pas ce dont elle dispose - et annoncer « ecriture de
    /// fichiers » a quelqu'un qui demande de creer un site sur son bureau est un
    /// mensonge par omission. L'interface doit donc le dire.
    pub fn outils_disponibles(&self) -> bool {
        false
    }

    /// Ce que la mission produit vraiment, aujourd'hui.
    pub fn livrable(&self) -> &'static str {
        match self {
            Mission::Reponse => "une reponse et ses desaccords",
            Mission::Code => "un plan et du code A COPIER (aucun fichier ecrit)",
            Mission::Recherche => "un plan de recherche (aucune source consultee)",
            Mission::Experimentation => "un protocole (aucun code execute)",
        }
    }
}

/// Ce que l'orchestrateur decide avant que le debat commence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    /// Le type de travail demande.
    #[serde(default)]
    pub mission: Mission,
    /// Identifiants des specialistes retenus, hors orchestrateur et arbitre.
    pub participants: Vec<String>,
    /// Plafond de tours de debat. L'arret reel peut venir plus tot.
    pub tours_max: u8,
    /// Pourquoi cette composition. Affiche a l'utilisateur: un debat dont on ne sait
    /// pas pourquoi il coute six appels est un debat qu'on n'utilisera pas.
    pub raison: String,
}

/// Les bornes du dispositif.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reglages {
    /// Plafond DUR de jetons pour toute la deliberation, synthese comprise.
    ///
    /// Un plafond, pas une bonne intention: sans lui, six specialistes sur six tours
    /// consomment un budget que personne n'a decide.
    pub jetons_max: u32,
    /// Plafond de tours, quoi que demande l'orchestrateur.
    pub tours_max: u8,
    /// Nombre de participants au-dela duquel on refuse.
    pub participants_max: usize,
}

impl Default for Reglages {
    fn default() -> Self {
        Self {
            // ~4 participants x 3 tours a 2k jetons, plus la synthese. De quoi tenir un
            // vrai debat sans qu'une question distraite coute le prix d'une soiree.
            jetons_max: 30_000,
            tours_max: 4,
            // Sept specialistes livres, moins l'orchestrateur et l'arbitre: on plafonne
            // au-dessus de ce que le catalogue permet, pour qu'embaucher tout le monde
            // fonctionne. Le plafond reste la comme garde-fou contre une liste absurde.
            participants_max: 8,
        }
    }
}

/// Ce qu'il faut faire maintenant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Etape {
    /// Chaque participant repond SEUL, sans voir les autres. Premier tour.
    Solo(Vec<String>),
    /// Chacun relit tout le debat et revise sa position.
    Relecture(Vec<String>),
    /// Le contradicteur attaque l'ensemble.
    Contradiction(String),
    /// Chacun repond aux objections du contradicteur.
    Reponse(Vec<String>),
    /// L'arbitre fusionne. Toujours en dernier.
    Synthese(String),
    /// Termine, avec la raison de l'arret.
    Fini(Arret),
}

/// Pourquoi le debat s'est arrete. Toujours affiche: un arret dont on ignore la cause
/// laisse croire a un bug.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Arret {
    /// Plus personne ne bouge: le debat a converge.
    Convergence,
    /// Plafond de tours atteint alors que des positions bougeaient encore.
    ToursEpuises,
    /// Plafond de jetons atteint. La synthese est faite avec ce qu'on a.
    BudgetEpuise,
    /// Aucun participant n'a pu repondre.
    Vide,
}

/// L'etat d'une deliberation en cours.
#[derive(Debug, Clone)]
pub struct Deliberation {
    pub question: String,
    pub plan: Plan,
    pub reglages: Reglages,
    /// Toutes les interventions, dans l'ordre.
    pub interventions: Vec<Intervention>,
    /// Le pool embauche, pour retrouver un role par identifiant.
    pub equipe: Vec<Specialiste>,
    pub tour_courant: u8,
    /// Le contradicteur est-il deja passe ?
    contradiction_faite: bool,
    reponses_faites: bool,
}

impl Deliberation {
    pub fn nouvelle(
        question: impl Into<String>,
        mut plan: Plan,
        reglages: Reglages,
        equipe: Vec<Specialiste>,
    ) -> Self {
        // Les reglages sont un plafond DUR: l'orchestrateur propose, ils tranchent.
        // Un orchestrateur qui demande douze tours est un orchestrateur qui s'est
        // trompe, pas une autorisation de depenser douze tours.
        plan.tours_max = plan.tours_max.min(reglages.tours_max).max(1);
        plan.participants.truncate(reglages.participants_max);
        Self {
            question: question.into(),
            plan,
            reglages,
            interventions: Vec::new(),
            equipe,
            tour_courant: 0,
            contradiction_faite: false,
            reponses_faites: false,
        }
    }

    pub fn jetons_consommes(&self) -> u32 {
        self.interventions.iter().map(|i| i.jetons).sum()
    }

    fn budget_epuise(&self) -> bool {
        self.jetons_consommes() >= self.reglages.jetons_max
    }

    fn role_de(&self, id: &str) -> Option<Role> {
        self.equipe.iter().find(|s| s.id == id).map(|s| s.role)
    }

    fn contradicteur(&self) -> Option<String> {
        self.plan
            .participants
            .iter()
            .find(|id| self.role_de(id) == Some(Role::Contradicteur))
            .cloned()
    }

    fn arbitre(&self) -> Option<String> {
        self.equipe
            .iter()
            .find(|s| s.role == Role::Arbitre)
            .map(|s| s.id.clone())
    }

    /// Les participants qui contribuent, contradicteur exclu: il n'a pas de these.
    fn contributeurs(&self) -> Vec<String> {
        self.plan
            .participants
            .iter()
            .filter(|id| self.role_de(id) != Some(Role::Contradicteur))
            .cloned()
            .collect()
    }

    fn interventions_du_tour(&self, tour: u8) -> Vec<&Intervention> {
        self.interventions.iter().filter(|i| i.tour == tour).collect()
    }

    /// Quelqu'un a-t-il bouge au dernier tour ?
    ///
    /// C'est le critere d'arret, et il vaut mieux qu'un compte de tours: un debat ou
    /// plus personne ne change d'avis ne produira rien de plus au tour suivant, et
    /// trois tours factures pour trois fois la meme reponse sont trois tours perdus.
    fn ca_bouge_encore(&self) -> bool {
        let derniers = self.interventions_du_tour(self.tour_courant);
        if derniers.is_empty() {
            return false;
        }
        derniers.iter().any(|i| i.a_change())
    }

    /// L'etape suivante.
    ///
    /// La sequence suit celle du debat voulu: chacun seul, puis chacun lit les autres,
    /// puis le contradicteur attaque, puis chacun lui repond, puis on synthetise. Mais
    /// chaque transition passe par le budget et par la convergence, donc un debat qui
    /// n'a plus rien a dire s'arrete au tour 2 sans jouer les quatre.
    pub fn prochaine_etape(&self) -> Etape {
        if self.plan.participants.is_empty() {
            return Etape::Fini(Arret::Vide);
        }
        // L'arbitre a rendu: le debat est clos, quels que soient les tours restants.
        // Sans ce garde, la sequence repartait en relecture apres la synthese.
        if self.synthese_faite() {
            return Etape::Fini(if self.budget_epuise() {
                Arret::BudgetEpuise
            } else if self.ca_bouge_encore() {
                Arret::ToursEpuises
            } else {
                Arret::Convergence
            });
        }
        // Le budget passe avant tout le reste, SAUF la synthese: une deliberation sans
        // synthese ne rend rien d'exploitable, donc on garde toujours de quoi conclure.
        if self.budget_epuise() && !self.interventions.is_empty() {
            return match self.arbitre() {
                Some(a) if !self.synthese_faite() => Etape::Synthese(a),
                _ => Etape::Fini(Arret::BudgetEpuise),
            };
        }

        if self.tour_courant == 0 {
            return Etape::Solo(self.plan.participants.clone());
        }
        if self.interventions.is_empty() {
            return Etape::Fini(Arret::Vide);
        }

        // Convergence: plus personne ne bouge, on conclut.
        if self.tour_courant >= 2 && !self.ca_bouge_encore() {
            return match self.arbitre() {
                Some(a) if !self.synthese_faite() => Etape::Synthese(a),
                _ => Etape::Fini(Arret::Convergence),
            };
        }

        // Le contradicteur intervient une fois le debat installe, pas au premier tour:
        // attaquer des positions que personne n'a encore lues ne produit rien.
        if self.tour_courant >= 2 && !self.contradiction_faite {
            if let Some(c) = self.contradicteur() {
                return Etape::Contradiction(c);
            }
        }
        if self.contradiction_faite && !self.reponses_faites {
            return Etape::Reponse(self.contributeurs());
        }

        if self.tour_courant < self.plan.tours_max {
            return Etape::Relecture(self.plan.participants.clone());
        }

        match self.arbitre() {
            Some(a) if !self.synthese_faite() => Etape::Synthese(a),
            _ => Etape::Fini(Arret::ToursEpuises),
        }
    }

    fn synthese_faite(&self) -> bool {
        self.arbitre()
            .map(|a| self.interventions.iter().any(|i| i.specialiste == a))
            .unwrap_or(false)
    }

    /// Enregistre les interventions d'un tour et avance.
    pub fn deposer(&mut self, mut lot: Vec<Intervention>) {
        if lot.is_empty() {
            return;
        }
        let arbitre = self.arbitre();
        let est_synthese = lot.len() == 1 && Some(&lot[0].specialiste) == arbitre.as_ref();
        let est_contradiction = lot.len() == 1
            && self.role_de(&lot[0].specialiste) == Some(Role::Contradicteur);

        // La synthese ne compte pas comme un tour de debat: elle le clot.
        if !est_synthese {
            self.tour_courant += 1;
        }
        for i in lot.iter_mut() {
            i.tour = self.tour_courant;
        }
        if est_contradiction {
            self.contradiction_faite = true;
        } else if self.contradiction_faite && !est_synthese {
            self.reponses_faites = true;
        }
        self.interventions.extend(lot);
    }

    /// La repartition des accords: qui approuve, qui reserve, qui s'oppose.
    ///
    /// Volontairement une repartition et non un pourcentage. Un score unique d'accord
    /// entre modeles mesure la conformite et serait lu comme une confiance - alors que
    /// la constitution dit l'inverse: la verite avant le consensus. On rend donc la
    /// liste, et le lecteur voit qui dissent et sur quoi.
    pub fn repartition(&self) -> Vec<(String, Accord, u8)> {
        let mut vue: Vec<(String, Accord, u8)> = Vec::new();
        // La DERNIERE position de chacun, pas la premiere: c'est celle qui vaut.
        for id in &self.plan.participants {
            if let Some(i) = self.interventions.iter().rfind(|i| &i.specialiste == id) {
                vue.push((id.clone(), i.accord, i.confiance));
            }
        }
        vue
    }

    /// Les specialistes encore en desaccord a la fin. C'est le livrable qui compte.
    pub fn dissidents(&self) -> Vec<String> {
        self.repartition()
            .into_iter()
            .filter(|(_, a, _)| *a == Accord::Oppose)
            .map(|(id, _, _)| id)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::deliberation::specialiste::catalogue;

    fn equipe() -> Vec<Specialiste> {
        catalogue()
    }

    fn plan(ids: &[&str], tours: u8) -> Plan {
        Plan {
            mission: Mission::Reponse,
            participants: ids.iter().map(|s| s.to_string()).collect(),
            tours_max: tours,
            raison: "test".into(),
        }
    }

    fn interv(id: &str, change: bool, accord: Accord) -> Intervention {
        Intervention {
            specialiste: id.into(),
            tour: 0,
            accord,
            confiance: 70,
            changement: if change { "l'argument de X".into() } else { "aucun".into() },
            refutable: String::new(),
            hypotheses: vec![],
            inconnues: vec![],
            position: "une position".into(),
            jetons: 100,
        }
    }

    #[test]
    fn chaque_mission_embauche_un_contradicteur() {
        // C'est le garde contre l'effondrement par complaisance: sans lui, une table
        // ronde converge vers la premiere reponse assuree, pas vers la bonne.
        for m in [Mission::Reponse, Mission::Code, Mission::Recherche, Mission::Experimentation] {
            assert!(
                m.equipe_par_defaut().contains(&"contradicteur"),
                "{m:?} sans contradicteur"
            );
        }
    }

    #[test]
    fn chaque_mission_annonce_son_niveau_d_acces() {
        // L'utilisateur doit savoir AVANT de lancer si la table va ecrire des fichiers.
        for m in [Mission::Reponse, Mission::Code, Mission::Recherche, Mission::Experimentation] {
            assert!(!m.acces().is_empty());
        }
        assert_eq!(Mission::Reponse.acces(), "lecture seule");
        assert_ne!(Mission::Code.acces(), Mission::Reponse.acces());
    }

    #[test]
    fn les_equipes_par_defaut_existent_dans_le_catalogue() {
        let ids: Vec<String> = catalogue().into_iter().map(|s| s.id).collect();
        for m in [Mission::Reponse, Mission::Code, Mission::Recherche, Mission::Experimentation] {
            for id in m.equipe_par_defaut() {
                assert!(ids.iter().any(|c| c == id), "{m:?} embauche {id}, absent du catalogue");
            }
        }
    }

    #[test]
    fn le_premier_tour_est_solo() {
        let d = Deliberation::nouvelle(
            "q",
            plan(&["scientifique", "ingenieur"], 3),
            Reglages::default(),
            equipe(),
        );
        assert_eq!(
            d.prochaine_etape(),
            Etape::Solo(vec!["scientifique".into(), "ingenieur".into()])
        );
    }

    #[test]
    fn les_reglages_plafonnent_l_orchestrateur() {
        let r = Reglages { tours_max: 2, participants_max: 2, ..Default::default() };
        let d = Deliberation::nouvelle(
            "q",
            plan(&["scientifique", "ingenieur", "attaquant", "visionnaire"], 12),
            r,
            equipe(),
        );
        assert_eq!(d.plan.tours_max, 2, "12 tours demandes doivent etre ramenes a 2");
        assert_eq!(d.plan.participants.len(), 2, "4 participants ramenes a 2");
    }

    #[test]
    fn un_plan_vide_ne_delibere_pas() {
        let d = Deliberation::nouvelle("q", plan(&[], 3), Reglages::default(), equipe());
        assert_eq!(d.prochaine_etape(), Etape::Fini(Arret::Vide));
    }

    #[test]
    fn la_convergence_arrete_avant_les_tours_prevus() {
        let mut d = Deliberation::nouvelle(
            "q",
            plan(&["scientifique", "ingenieur"], 4),
            Reglages::default(),
            equipe(),
        );
        d.deposer(vec![
            interv("scientifique", true, Accord::Approuve),
            interv("ingenieur", true, Accord::Approuve),
        ]);
        // Tour 2: plus personne ne bouge.
        d.deposer(vec![
            interv("scientifique", false, Accord::Approuve),
            interv("ingenieur", false, Accord::Approuve),
        ]);
        assert_eq!(
            d.prochaine_etape(),
            Etape::Synthese("arbitre".into()),
            "un debat qui ne bouge plus doit conclure, pas jouer ses 4 tours"
        );
    }

    #[test]
    fn le_contradicteur_n_attaque_pas_au_premier_tour() {
        let mut d = Deliberation::nouvelle(
            "q",
            plan(&["scientifique", "contradicteur"], 4),
            Reglages::default(),
            equipe(),
        );
        // Tour 1: solo, y compris le contradicteur.
        assert!(matches!(d.prochaine_etape(), Etape::Solo(_)));
        d.deposer(vec![
            interv("scientifique", true, Accord::Approuve),
            interv("contradicteur", true, Accord::Oppose),
        ]);
        // Tour 2: ca bouge encore, donc relecture avant contradiction.
        d.deposer(vec![
            interv("scientifique", true, Accord::Reserve),
            interv("contradicteur", true, Accord::Oppose),
        ]);
        assert_eq!(
            d.prochaine_etape(),
            Etape::Contradiction("contradicteur".into()),
            "le contradicteur intervient une fois le debat installe"
        );
    }

    #[test]
    fn apres_la_contradiction_les_autres_repondent() {
        let mut d = Deliberation::nouvelle(
            "q",
            plan(&["scientifique", "ingenieur", "contradicteur"], 4),
            Reglages::default(),
            equipe(),
        );
        d.deposer(vec![interv("scientifique", true, Accord::Approuve)]);
        d.deposer(vec![interv("scientifique", true, Accord::Reserve)]);
        d.deposer(vec![interv("contradicteur", true, Accord::Oppose)]);
        match d.prochaine_etape() {
            Etape::Reponse(qui) => {
                assert!(qui.contains(&"scientifique".to_string()));
                assert!(
                    !qui.contains(&"contradicteur".to_string()),
                    "le contradicteur ne repond pas a lui-meme"
                );
            }
            autre => panic!("attendu Reponse, obtenu {autre:?}"),
        }
    }

    #[test]
    fn le_budget_epuise_laisse_quand_meme_conclure() {
        let r = Reglages { jetons_max: 150, ..Default::default() };
        let mut d = Deliberation::nouvelle("q", plan(&["scientifique"], 4), r, equipe());
        d.deposer(vec![interv("scientifique", true, Accord::Approuve)]); // 100 jetons
        d.deposer(vec![interv("scientifique", true, Accord::Approuve)]); // 200 > 150
        assert!(d.jetons_consommes() > 150);
        assert_eq!(
            d.prochaine_etape(),
            Etape::Synthese("arbitre".into()),
            "on garde toujours de quoi synthetiser, sinon rien n'est exploitable"
        );
    }

    #[test]
    fn la_synthese_ne_compte_pas_comme_un_tour() {
        let mut d = Deliberation::nouvelle("q", plan(&["scientifique"], 4), Reglages::default(), equipe());
        d.deposer(vec![interv("scientifique", true, Accord::Approuve)]);
        let avant = d.tour_courant;
        d.deposer(vec![interv("arbitre", false, Accord::Approuve)]);
        assert_eq!(d.tour_courant, avant, "la synthese clot le debat, elle n'en est pas un tour");
        assert!(matches!(d.prochaine_etape(), Etape::Fini(_)));
    }

    #[test]
    fn la_repartition_prend_la_derniere_position() {
        let mut d = Deliberation::nouvelle("q", plan(&["scientifique"], 4), Reglages::default(), equipe());
        d.deposer(vec![interv("scientifique", true, Accord::Oppose)]);
        d.deposer(vec![interv("scientifique", true, Accord::Approuve)]);
        let r = d.repartition();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].1, Accord::Approuve, "c'est le dernier avis qui vaut");
        assert!(d.dissidents().is_empty());
    }

    #[test]
    fn les_dissidents_sont_nommes() {
        let mut d = Deliberation::nouvelle(
            "q",
            plan(&["scientifique", "ingenieur"], 4),
            Reglages::default(),
            equipe(),
        );
        d.deposer(vec![
            interv("scientifique", true, Accord::Approuve),
            interv("ingenieur", true, Accord::Oppose),
        ]);
        assert_eq!(d.dissidents(), vec!["ingenieur".to_string()]);
    }
}
