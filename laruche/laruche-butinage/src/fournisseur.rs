//! The model (LLM) response **fournisseur**, abstracted behind a trait.
//!
//! Dependency inversion: `laruche-butinage` does not know about concrete
//! providers. The adapter (in `laruche-essaim`) implements this trait and handles
//! streaming, key rotation (`credential_pool`) and model rerouting internally; it
//! only surfaces an aggregated response or a terminal error. On top of it the loop
//! applies the [`crate::meteo`] policy (backoff, give-up).

use crate::issue::{Appel, StopReason};
use crate::messagerie::Message;
use async_trait::async_trait;

/// Token consumption (actual if the provider reports it).
#[derive(Debug, Clone, Copy, Default)]
pub struct Usage {
    pub entree: u32,
    pub sortie: u32,
}

/// Aggregated response of a model call.
#[derive(Debug, Clone, Default)]
pub struct ReponseModele {
    pub texte: String,
    pub stop: StopReason,
    /// Tool calls emitted by the model (API-native or parsed by the adapter).
    pub appels: Vec<Appel>,
    pub usage: Option<Usage>,
    /// Le raisonnement d'un modele "thinking", a rendre au fournisseur au tour
    /// suivant. Voir [`crate::messagerie::Message::reasoning`].
    pub reasoning: Option<String>,
    /// Le modele qui a produit ce raisonnement. Sans lui on ne saurait pas a qui
    /// le rendre, et le renvoyer au mauvais fournisseur casserait la requete.
    pub reasoning_model: Option<String>,
}

/// Error of a model call, carrying what is needed to classify it ([`crate::meteo::ClasseErreur`]).
#[derive(Debug, Clone)]
pub struct ErreurFournisseur {
    pub status: u16,
    pub retry_after: Option<String>,
    pub corps: String,
}

impl std::fmt::Display for ErreurFournisseur {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "fournisseur status={} : {}", self.status, self.corps)
    }
}
impl std::error::Error for ErreurFournisseur {}

/// The source of model responses.
#[async_trait]
pub trait Fournisseur: Send + Sync {
    fn tokens_consommees(&self) -> u64 {
        0
    }
    /// One full call: messages + tool schemas -> aggregated response.
    async fn repondre(
        &self,
        messages: &[Message],
        schemas: &[serde_json::Value],
    ) -> Result<ReponseModele, ErreurFournisseur>;
}

/// One deadline and accounting boundary for every model call, including summaries.
pub(crate) struct FournisseurGarde<'a> {
    pub inner: &'a dyn Fournisseur,
    pub reglages: &'a crate::Reglages,
    pub annulation: Option<&'a std::sync::atomic::AtomicBool>,
    pub entree: std::sync::atomic::AtomicU64,
    pub sortie: std::sync::atomic::AtomicU64,
    pub deja_depense: u64,
}

impl FournisseurGarde<'_> {
    pub fn depense(&self) -> u64 {
        use std::sync::atomic::Ordering::Relaxed;
        self.entree.load(Relaxed) + self.sortie.load(Relaxed)
    }
}

#[async_trait]
impl Fournisseur for FournisseurGarde<'_> {
    fn tokens_consommees(&self) -> u64 {
        self.reglages
            .budget_partage
            .load(std::sync::atomic::Ordering::Relaxed)
            .max(self.deja_depense + self.depense())
    }

    async fn repondre(
        &self,
        messages: &[Message],
        schemas: &[serde_json::Value],
    ) -> Result<ReponseModele, ErreurFournisseur> {
        use std::sync::atomic::Ordering::Relaxed;
        let input = (messages.iter().map(|m| m.cout_chars()).sum::<usize>()
            + schemas.iter().map(|s| s.to_string().len()).sum::<usize>())
        .div_ceil(4) as u64;
        let error = |reason: &str| ErreurFournisseur {
            status: 0,
            retry_after: None,
            corps: reason.into(),
        };
        let reserve = input.saturating_add(self.reglages.reserve_sortie as u64);
        let budget = self.reglages.budget_tokens;
        if self
            .reglages
            .budget_partage
            .fetch_update(Relaxed, Relaxed, |used| {
                let next = used.checked_add(reserve)?;
                (budget == 0 || next <= budget).then_some(next)
            })
            .is_err()
        {
            return Err(ErreurFournisseur {
                status: 402,
                retry_after: None,
                corps: "Mission token budget exhausted before model admission".into(),
            });
        }
        self.entree.fetch_add(input, Relaxed);
        let future = self.inner.repondre(messages, schemas);
        tokio::pin!(future);
        let started = tokio::time::Instant::now();
        loop {
            tokio::select! {
                result = &mut future => {
                    if let Ok(r) = &result {
                        let output = r.usage.map(|u| u.sortie as u64).unwrap_or_else(||
                            (r.texte.len() + r.appels.iter().map(|a| a.args.to_string().len()).sum::<usize>()).div_ceil(4) as u64);
                        self.sortie.fetch_add(output, Relaxed);
                        let actual = r.usage.map(|u| u.entree as u64).unwrap_or(input).saturating_add(output);
                        if actual > reserve { self.reglages.budget_partage.fetch_add(actual - reserve, Relaxed); }
                        else { self.reglages.budget_partage.fetch_sub(reserve - actual, Relaxed); }
                        if let Some(u) = r.usage {
                            if u.entree as u64 >= input { self.entree.fetch_add(u.entree as u64 - input, Relaxed); }
                            else { self.entree.fetch_sub(input - u.entree as u64, Relaxed); }
                        }
                    }
                    return result;
                }
                _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {
                    if self.annulation.is_some_and(|f| f.load(Relaxed)) { return Err(error("Model call cancelled")); }
                    if self.reglages.timeout_modele_secs > 0 && started.elapsed().as_secs() >= self.reglages.timeout_modele_secs {
                        return Err(error("Model call timed out"));
                    }
                }
            }
        }
    }
}
