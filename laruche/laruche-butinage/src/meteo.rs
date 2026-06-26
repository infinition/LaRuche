//! La **météo du vol** — conditions d'erreur des providers et réaction à adopter.
//!
//! Sépare la *classification* (quel genre d'erreur ?) de la *réaction* (que faire ?),
//! toutes deux pures et testées. La boucle applique la réaction (sleep, rotation de
//! clé via `credential_pool`, déroutement modèle, arrêt).

/// Genre d'erreur provider, normalisé à travers les backends.
#[derive(Debug, Clone, PartialEq)]
pub enum ClasseErreur {
    /// Quota/débit dépassé. `reset_at` = epoch secondes si connu (header `Retry-After`/`reset`).
    RateLimited { reset_at: Option<i64> },
    /// Authentification invalide/expirée — le déroutement modèle est inutile.
    ReloginRequis,
    /// Panne passagère (5xx, reset réseau, timeout) — retry sur le MÊME modèle.
    Transitoire,
    /// Erreur définitive (4xx hors 401/403/429, requête invalide) — déroutement modèle.
    Fatal,
}

impl ClasseErreur {
    /// Classe une erreur à partir du code HTTP et d'indices (corps, header retry-after).
    pub fn classer(status: u16, retry_after: Option<&str>, corps: &str) -> ClasseErreur {
        match status {
            429 => ClasseErreur::RateLimited {
                reset_at: parser_retry_after(retry_after),
            },
            401 | 403 => {
                // Certains providers renvoient 403 pour un simple quota épuisé.
                if corps.to_lowercase().contains("rate")
                    || corps.to_lowercase().contains("quota")
                    || corps.to_lowercase().contains("limit")
                {
                    ClasseErreur::RateLimited {
                        reset_at: parser_retry_after(retry_after),
                    }
                } else {
                    ClasseErreur::ReloginRequis
                }
            }
            500 | 502 | 503 | 504 | 408 | 522 | 524 => ClasseErreur::Transitoire,
            // 0 = erreur de transport (pas de réponse HTTP) → passager.
            0 => ClasseErreur::Transitoire,
            _ => ClasseErreur::Fatal,
        }
    }

    pub fn exige_relogin(&self) -> bool {
        matches!(self, ClasseErreur::ReloginRequis)
    }
}

/// Ce que la boucle doit faire face à l'erreur.
#[derive(Debug, Clone, PartialEq)]
pub enum Reaction {
    /// Dormir N secondes puis réessayer le même modèle/la même clé.
    Patienter(u64),
    /// Tenter la prochaine clé API disponible (rotation `credential_pool`).
    RotationCle,
    /// Basculer sur un modèle de repli (failover).
    Deroutement,
    /// Abandonner avec ce motif (relogin requis, ou recours épuisés).
    Stopper(String),
}

/// Politique de réaction. Pure : `(classe, tentative, plafonds) -> Reaction`.
///
/// - `tentative` : numéro de la tentative courante (1-based).
/// - `max_rate_limit` : nb max d'attentes sur rate-limit avant de tenter autre chose.
/// - `max_transitoire` : nb max de retries sur panne passagère.
/// - `cle_dispo` : une autre clé API est-elle disponible (rotation possible) ?
/// - `repli_dispo` : un modèle de repli existe-t-il (déroutement possible) ?
pub fn reagir(
    classe: &ClasseErreur,
    tentative: usize,
    max_rate_limit: usize,
    max_transitoire: usize,
    cle_dispo: bool,
    repli_dispo: bool,
    now: i64,
) -> Reaction {
    match classe {
        ClasseErreur::ReloginRequis => {
            Reaction::Stopper("authentification invalide — reconnecte le provider".into())
        }
        ClasseErreur::RateLimited { reset_at } => {
            if cle_dispo {
                // Une autre clé est libre : on tourne plutôt que d'attendre.
                Reaction::RotationCle
            } else if tentative <= max_rate_limit {
                Reaction::Patienter(delai_rate_limit(*reset_at, tentative, now))
            } else if repli_dispo {
                Reaction::Deroutement
            } else {
                Reaction::Stopper(format!(
                    "rate limit persistant après {max_rate_limit} attente(s)"
                ))
            }
        }
        ClasseErreur::Transitoire => {
            if tentative <= max_transitoire {
                Reaction::Patienter(delai_backoff(tentative))
            } else if repli_dispo {
                Reaction::Deroutement
            } else {
                Reaction::Stopper(format!("panne passagère persistante après {max_transitoire} essais"))
            }
        }
        ClasseErreur::Fatal => {
            if repli_dispo {
                Reaction::Deroutement
            } else {
                Reaction::Stopper("erreur fatale du provider, aucun repli disponible".into())
            }
        }
    }
}

/// Backoff exponentiel borné (1s, 2s, 4s, 8s… plafonné à 30s).
pub fn delai_backoff(tentative: usize) -> u64 {
    let base = 1u64 << (tentative.saturating_sub(1)).min(5); // 1..=32
    base.min(30)
}

/// Délai d'attente sur rate-limit : privilégie `Retry-After`/`reset`, sinon une
/// fenêtre RPM raisonnable qui croît avec les tentatives.
pub fn delai_rate_limit(reset_at: Option<i64>, tentative: usize, now: i64) -> u64 {
    if let Some(reset) = reset_at {
        let delta = reset - now;
        if delta > 0 {
            // +2s de marge, plafonné à 5 min.
            return ((delta as u64) + 2).min(300);
        }
    }
    // Sans header : 65s, 90s, 120s…
    match tentative {
        1 => 65,
        2 => 90,
        _ => 120,
    }
}

/// Parse un header `Retry-After` (« 42 » secondes, ou epoch absolu).
fn parser_retry_after(h: Option<&str>) -> Option<i64> {
    let s = h?.trim();
    if let Ok(secs) = s.parse::<i64>() {
        // Heuristique : petit nombre = délai relatif ; grand = epoch absolu.
        if secs < 10_000_000 {
            return Some(chrono::Utc::now().timestamp() + secs);
        }
        return Some(secs);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classe_429_est_rate_limited() {
        let c = ClasseErreur::classer(429, Some("30"), "{}");
        assert!(matches!(c, ClasseErreur::RateLimited { reset_at: Some(_) }));
    }

    #[test]
    fn classe_401_est_relogin_sauf_si_quota() {
        assert_eq!(ClasseErreur::classer(401, None, "invalid api key"), ClasseErreur::ReloginRequis);
        assert!(matches!(
            ClasseErreur::classer(403, None, "quota exceeded"),
            ClasseErreur::RateLimited { .. }
        ));
    }

    #[test]
    fn classe_5xx_et_transport_sont_transitoires() {
        assert_eq!(ClasseErreur::classer(503, None, ""), ClasseErreur::Transitoire);
        assert_eq!(ClasseErreur::classer(0, None, ""), ClasseErreur::Transitoire);
    }

    #[test]
    fn classe_400_est_fatale() {
        assert_eq!(ClasseErreur::classer(400, None, "bad request"), ClasseErreur::Fatal);
    }

    #[test]
    fn relogin_stoppe_toujours() {
        let r = reagir(&ClasseErreur::ReloginRequis, 1, 3, 3, true, true, 0);
        assert!(matches!(r, Reaction::Stopper(_)));
    }

    #[test]
    fn rate_limit_tourne_la_cle_si_dispo() {
        let r = reagir(&ClasseErreur::RateLimited { reset_at: None }, 1, 3, 3, true, true, 0);
        assert_eq!(r, Reaction::RotationCle);
    }

    #[test]
    fn rate_limit_patiente_puis_deroute() {
        let cl = ClasseErreur::RateLimited { reset_at: None };
        assert!(matches!(reagir(&cl, 1, 2, 3, false, true, 0), Reaction::Patienter(_)));
        // tentative au-delà du max + repli dispo → déroutement
        assert_eq!(reagir(&cl, 3, 2, 3, false, true, 0), Reaction::Deroutement);
        // pas de repli → stop
        assert!(matches!(reagir(&cl, 3, 2, 3, false, false, 0), Reaction::Stopper(_)));
    }

    #[test]
    fn transitoire_backoff_puis_deroute() {
        let cl = ClasseErreur::Transitoire;
        assert_eq!(reagir(&cl, 1, 1, 2, false, true, 0), Reaction::Patienter(delai_backoff(1)));
        assert_eq!(reagir(&cl, 3, 1, 2, false, true, 0), Reaction::Deroutement);
    }

    #[test]
    fn backoff_borne() {
        assert_eq!(delai_backoff(1), 1);
        assert_eq!(delai_backoff(2), 2);
        assert_eq!(delai_backoff(3), 4);
        assert_eq!(delai_backoff(100), 30); // plafonné
    }

    #[test]
    fn rate_limit_utilise_reset_at() {
        // reset dans 40s → ~42s d'attente
        let d = delai_rate_limit(Some(1000), 1, 960);
        assert_eq!(d, 42);
        // sans header → fenêtre RPM
        assert_eq!(delai_rate_limit(None, 1, 0), 65);
        assert_eq!(delai_rate_limit(None, 2, 0), 90);
    }
}
