//! Ce que produit une **passe** (une itération) et ce qui termine un **butinage**.
//!
//! `Issue` normalise la sortie du modèle en *faits* exploitables par la politique
//! ([`crate::cap::boussole`]), pour ne jamais décider sur du matching de chaînes.

use serde::{Deserialize, Serialize};

/// Raison d'arrêt **native** du modèle, normalisée à travers les providers.
/// (Corrige le `Some("stop")` codé en dur de certains backends qui cassait la
/// détection de troncature.)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    /// Le modèle a fini naturellement son tour.
    FinTour,
    /// Coupé par la limite de tokens de sortie (→ reprise possible).
    Longueur,
    /// Le modèle veut appeler des outils.
    Outils,
    /// Autre/inconnu (réseau, filtre…).
    Autre,
}

/// Un appel d'outil demandé par le modèle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Appel {
    pub id: String,
    pub nom: String,
    pub args: serde_json::Value,
}

impl Appel {
    pub fn nouveau(nom: impl Into<String>, args: serde_json::Value) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            nom: nom.into(),
            args,
        }
    }

    /// Signature canonique stable (nom + args triés) pour la [`crate::cap::vigie::Vigie`].
    pub fn signature(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.nom.hash(&mut h);
        // serde_json sérialise les objets dans l'ordre d'insertion ; on retrie les clés
        // pour une signature indépendante de l'ordre.
        canonicaliser(&self.args).hash(&mut h);
        h.finish()
    }
}

/// JSON compact à clés triées (récursif) — base d'une signature stable.
pub fn canonicaliser(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::Object(m) => {
            let mut clefs: Vec<&String> = m.keys().collect();
            clefs.sort();
            let corps: Vec<String> = clefs
                .into_iter()
                .map(|k| format!("{}:{}", k, canonicaliser(&m[k])))
                .collect();
            format!("{{{}}}", corps.join(","))
        }
        serde_json::Value::Array(a) => {
            let corps: Vec<String> = a.iter().map(canonicaliser).collect();
            format!("[{}]", corps.join(","))
        }
        autre => autre.to_string(),
    }
}

/// L'issue d'une passe : ce que la boucle observe APRÈS l'appel au modèle.
#[derive(Debug, Clone)]
pub enum Issue {
    /// Le modèle a appelé l'outil explicite `mission_accomplie`.
    MissionAccomplie { resume: String, confiance: f32 },
    /// Le modèle a appelé `clarify` → on rend la main à l'utilisateur.
    Clarification(String),
    /// Le modèle veut exécuter des outils.
    Outils(Vec<Appel>),
    /// Réponse en texte seul (pas d'outil). On joint les faits qui guideront la décision.
    TexteSeul(TexteSeul),
}

/// Faits extraits d'une réponse texte-seul, consommés par la boussole.
#[derive(Debug, Clone)]
pub struct TexteSeul {
    pub texte: String,
    /// Raison d'arrêt native du modèle (si le provider la fournit).
    pub fin_native: Option<StopReason>,
    /// Vrai si l'itinéraire a encore des étapes ouvertes.
    pub plan_inacheve: bool,
    /// Vrai si le texte ressemble à un tool_call cassé (rail de récupération).
    pub malforme: bool,
    /// Vrai si la sortie a été tronquée (stop_reason=longueur ou bloc d'outil non fermé).
    pub tronquee: bool,
}

/// Raison terminale d'un butinage.
#[derive(Debug, Clone, PartialEq)]
pub enum FinDeVol {
    /// Mission accomplie (réponse finale prête).
    Accomplie,
    /// Plafond de passes atteint — possiblement incomplet.
    Plafond,
    /// Erreur fatale (provider/auth) après épuisement des recours.
    Erreur(String),
    /// Interrompue par l'utilisateur.
    Interrompue,
    /// L'abeille demande une clarification (rend la main).
    Clarification(String),
    /// Arrêt propre par la vigie (boucle stérile détectée).
    BoucleSterile(String),
}

/// Résultat final d'un butinage, remonté à l'appelant (node/dashboard).
#[derive(Debug, Clone)]
pub struct Bilan {
    pub texte: String,
    pub fin: FinDeVol,
    pub passes: usize,
}

impl Bilan {
    pub fn nouveau(texte: impl Into<String>, fin: FinDeVol, passes: usize) -> Self {
        Self {
            texte: texte.into(),
            fin,
            passes,
        }
    }

    /// Le butinage a-t-il abouti à une vraie réponse (vs erreur/plafond) ?
    pub fn est_succes(&self) -> bool {
        matches!(
            self.fin,
            FinDeVol::Accomplie | FinDeVol::Clarification(_)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn signature_independante_de_l_ordre_des_clefs() {
        let a = Appel::nouveau("web", json!({"q": "rust", "n": 3}));
        let b = Appel::nouveau("web", json!({"n": 3, "q": "rust"}));
        assert_eq!(a.signature(), b.signature());
    }

    #[test]
    fn signature_distingue_les_args() {
        let a = Appel::nouveau("web", json!({"q": "rust"}));
        let b = Appel::nouveau("web", json!({"q": "go"}));
        assert_ne!(a.signature(), b.signature());
    }

    #[test]
    fn signature_distingue_le_nom() {
        let a = Appel::nouveau("web", json!({"q": "x"}));
        let b = Appel::nouveau("fetch", json!({"q": "x"}));
        assert_ne!(a.signature(), b.signature());
    }

    #[test]
    fn canonicaliser_recursif() {
        let v = json!({"b": [3, {"y": 1, "x": 2}], "a": 1});
        let c = canonicaliser(&v);
        assert_eq!(c, "{a:1,b:[3,{x:2,y:1}]}");
    }

    #[test]
    fn bilan_succes() {
        let b = Bilan::nouveau("ok", FinDeVol::Accomplie, 3);
        assert!(b.est_succes());
        let e = Bilan::nouveau("", FinDeVol::Plafond, 100);
        assert!(!e.est_succes());
    }
}
