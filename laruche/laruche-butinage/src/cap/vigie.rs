//! La **Vigie** — surveillance pure des boucles stériles (inspirée du meilleur
//! d'third-party : un contrôleur sans effet de bord).
//!
//! La Vigie *observe* les appels d'outils et renvoie un [`Signal`]. Elle ne touche
//! à rien : c'est la boucle qui décide quoi faire du signal (injecter un conseil,
//! poser proprement). Elle détecte trois pathologies :
//! 1. **échec exact répété** : même (nom+args) échoue N fois → avertir puis bloquer ;
//! 2. **même outil échoue** : un outil échoue N fois (args variables) → avertir puis poser ;
//! 3. **idempotent sans progrès** : un outil lecture renvoie N fois le même résultat.

use std::collections::HashMap;

/// Décision de la Vigie. Sans effet de bord : la boucle l'applique.
#[derive(Debug, Clone, PartialEq)]
pub enum Signal {
    /// Rien à signaler, on continue.
    Laisser,
    /// Boucle suspecte : injecter ce conseil dans l'observation (l'outil s'exécute quand même).
    Avertir(String),
    /// Trop de répétitions : refuser cet appel précis (résultat synthétique d'erreur).
    Bloquer(String),
    /// Boucle stérile avérée : poser le butinage proprement avec ce motif.
    Poser(String),
}

impl Signal {
    /// L'appel peut-il s'exécuter malgré le signal ?
    pub fn autorise_execution(&self) -> bool {
        matches!(self, Signal::Laisser | Signal::Avertir(_))
    }
    /// Le signal demande-t-il un arrêt du butinage ?
    pub fn demande_arret(&self) -> bool {
        matches!(self, Signal::Poser(_))
    }
}

/// Seuils de détection. Avertissements toujours actifs ; arrêts durs opt-in
/// (`arret_dur`) pour ne pas brider un agent interactif.
#[derive(Debug, Clone)]
pub struct SeuilsVigie {
    pub avertir_echec_exact: u32,
    pub bloquer_echec_exact: u32,
    pub avertir_meme_outil: u32,
    pub poser_meme_outil: u32,
    pub avertir_sans_progres: u32,
    pub bloquer_sans_progres: u32,
    /// Active les blocages/arrêts durs (sinon : avertissements seuls).
    pub arret_dur: bool,
}

impl Default for SeuilsVigie {
    fn default() -> Self {
        Self {
            avertir_echec_exact: 2,
            bloquer_echec_exact: 5,
            avertir_meme_outil: 3,
            poser_meme_outil: 8,
            avertir_sans_progres: 2,
            bloquer_sans_progres: 5,
            arret_dur: true,
        }
    }
}

impl SeuilsVigie {
    /// Profil souple pour modèles forts (Claude/Codex/DeepSeek) : on fait confiance,
    /// avertissements seulement.
    pub fn souple() -> Self {
        Self {
            arret_dur: false,
            ..Self::default()
        }
    }

    /// Profil strict pour modèles locaux faibles (gemma/qwen petits) : arrêts durs
    /// plus précoces pour éviter le runaway.
    pub fn strict() -> Self {
        Self {
            avertir_echec_exact: 2,
            bloquer_echec_exact: 4,
            avertir_meme_outil: 2,
            poser_meme_outil: 6,
            avertir_sans_progres: 2,
            bloquer_sans_progres: 4,
            arret_dur: true,
        }
    }
}

/// Le surveillant. Garde des compteurs par signature/outil pour la durée d'un butinage.
#[derive(Debug, Clone, Default)]
pub struct Vigie {
    seuils: SeuilsVigie,
    echecs_exacts: HashMap<u64, u32>,
    echecs_par_outil: HashMap<String, u32>,
    /// signature → (hash du dernier résultat, nb de répétitions identiques).
    sans_progres: HashMap<u64, (u64, u32)>,
}

impl Vigie {
    pub fn nouvelle(seuils: SeuilsVigie) -> Self {
        Self {
            seuils,
            ..Default::default()
        }
    }

    /// Avant d'exécuter : bloque-t-on cet appel parce qu'il a déjà trop échoué/stagné ?
    pub fn avant_appel(&self, signature: u64) -> Signal {
        if !self.seuils.arret_dur {
            return Signal::Laisser;
        }
        if let Some(&n) = self.echecs_exacts.get(&signature) {
            if n >= self.seuils.bloquer_echec_exact {
                return Signal::Bloquer(format!(
                    "This exact tool call has failed {n} times with identical arguments. \
                     Stop retrying it unchanged — change strategy or report the blocker."
                ));
            }
        }
        if let Some(&(_, r)) = self.sans_progres.get(&signature) {
            if r >= self.seuils.bloquer_sans_progres {
                return Signal::Bloquer(format!(
                    "This read-only call returned the same result {r} times. \
                     Use the result you already have or change the query."
                ));
            }
        }
        Signal::Laisser
    }

    /// Après exécution : met à jour les compteurs et renvoie un éventuel signal.
    ///
    /// - `ok` : l'outil a-t-il réussi ?
    /// - `idempotent` : est-ce un outil lecture (mêmes args ⇒ même effet attendu) ?
    /// - `resultat_hash` : empreinte du résultat (pour détecter l'absence de progrès).
    pub fn apres_appel(
        &mut self,
        nom: &str,
        signature: u64,
        ok: bool,
        idempotent: bool,
        resultat_hash: u64,
    ) -> Signal {
        if !ok {
            let exact = self.echecs_exacts.entry(signature).or_insert(0);
            *exact += 1;
            let exact = *exact;
            self.sans_progres.remove(&signature);

            let par_outil = self.echecs_par_outil.entry(nom.to_string()).or_insert(0);
            *par_outil += 1;
            let par_outil = *par_outil;

            if self.seuils.arret_dur && par_outil >= self.seuils.poser_meme_outil {
                return Signal::Poser(format!(
                    "Tool `{nom}` failed {par_outil} times this run. Stopping to avoid a sterile loop."
                ));
            }
            if exact >= self.seuils.avertir_echec_exact {
                return Signal::Avertir(format!(
                    "`{nom}` has failed {exact} times with identical arguments. This looks like a loop; \
                     inspect the error and change approach instead of retrying unchanged."
                ));
            }
            if par_outil >= self.seuils.avertir_meme_outil {
                return Signal::Avertir(format!(
                    "`{nom}` has failed {par_outil} times this run. Diagnose before retrying: try \
                     different arguments, a narrower scope, or another tool."
                ));
            }
            return Signal::Laisser;
        }

        // Succès : on oublie les échecs de cette signature/outil.
        self.echecs_exacts.remove(&signature);
        self.echecs_par_outil.remove(nom);

        if !idempotent {
            self.sans_progres.remove(&signature);
            return Signal::Laisser;
        }

        // Idempotent : détecte la stagnation (même résultat répété).
        let repet = match self.sans_progres.get(&signature) {
            Some(&(h, r)) if h == resultat_hash => r + 1,
            _ => 1,
        };
        self.sans_progres.insert(signature, (resultat_hash, repet));

        if repet >= self.seuils.avertir_sans_progres {
            return Signal::Avertir(format!(
                "`{nom}` returned the same result {repet} times. Use it or change the query \
                 instead of repeating the call."
            ));
        }
        Signal::Laisser
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rien_a_signaler_au_premier_appel() {
        let mut v = Vigie::nouvelle(SeuilsVigie::default());
        assert_eq!(v.avant_appel(1), Signal::Laisser);
        assert_eq!(v.apres_appel("web", 1, true, true, 42), Signal::Laisser);
    }

    #[test]
    fn echec_exact_repete_avertit_puis_bloque() {
        let mut v = Vigie::nouvelle(SeuilsVigie::default());
        // 1er échec : rien
        assert_eq!(v.apres_appel("web", 7, false, false, 0), Signal::Laisser);
        // 2e échec exact : avertissement (seuil avertir_echec_exact=2)
        assert!(matches!(v.apres_appel("web", 7, false, false, 0), Signal::Avertir(_)));
        // après 5 échecs, avant_appel bloque
        v.apres_appel("web", 7, false, false, 0);
        v.apres_appel("web", 7, false, false, 0);
        v.apres_appel("web", 7, false, false, 0); // total 5
        assert!(matches!(v.avant_appel(7), Signal::Bloquer(_)));
    }

    #[test]
    fn meme_outil_echoue_finit_par_poser() {
        let mut v = Vigie::nouvelle(SeuilsVigie::default());
        // 8 échecs avec des signatures différentes (args variables) → Poser
        let mut dernier = Signal::Laisser;
        for i in 0..8 {
            dernier = v.apres_appel("shell", 1000 + i, false, false, 0);
        }
        assert!(matches!(dernier, Signal::Poser(_)));
    }

    #[test]
    fn idempotent_sans_progres_avertit() {
        let mut v = Vigie::nouvelle(SeuilsVigie::default());
        assert_eq!(v.apres_appel("web", 3, true, true, 999), Signal::Laisser);
        // même résultat (999) une 2e fois → avertissement (seuil 2)
        assert!(matches!(v.apres_appel("web", 3, true, true, 999), Signal::Avertir(_)));
    }

    #[test]
    fn succes_efface_les_echecs() {
        let mut v = Vigie::nouvelle(SeuilsVigie::default());
        v.apres_appel("web", 5, false, false, 0);
        v.apres_appel("web", 5, false, false, 0); // 2 échecs
        v.apres_appel("web", 5, true, false, 0); // succès → reset
        assert_eq!(v.avant_appel(5), Signal::Laisser);
    }

    #[test]
    fn profil_souple_n_arrete_jamais_dur() {
        let mut v = Vigie::nouvelle(SeuilsVigie::souple());
        for _ in 0..20 {
            let s = v.apres_appel("shell", 1, false, false, 0);
            assert!(!s.demande_arret(), "souple ne doit jamais Poser");
        }
        assert_eq!(v.avant_appel(1), Signal::Laisser); // jamais Bloquer
    }
}
