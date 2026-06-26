//! Le **nectar** — la source de contexte de l'abeille (mémoire), abstraite par un trait.
//!
//! Inversion de dépendances : le moteur ne connaît pas la mémoire concrète (graphe
//! cognitif, vecteurs…), seulement ce trait. L'adaptateur (`laruche-essaim`) l'implémente
//! au-dessus de `MemoireCognitive`. Optionnel : un butinage peut tourner sans `Source`.

use async_trait::async_trait;

/// Fournisseur de contexte durable (mémoire).
#[async_trait]
pub trait Source: Send + Sync {
    /// Rappelle un contexte pertinent pour la requête (récupération just-in-time).
    /// `None` si rien de pertinent.
    async fn rappeler(&self, requete: &str) -> Option<String>;

    /// Consigne un fait durable sous un identifiant de nœud pointé (`domaine.sujet`).
    async fn consigner(&self, node_id: &str, fait: &str);
}
