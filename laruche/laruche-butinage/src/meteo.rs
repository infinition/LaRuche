//! The **flight meteo**: provider error conditions and the reaction to adopt.
//!
//! Separates *classification* (what kind of error?) from *reaction* (what to do?),
//! both pure and tested. The loop applies the reaction (sleep, key rotation
//! via `credential_pool`, model failover, stop).

/// Provider error kind, normalized across backends.
#[derive(Debug, Clone, PartialEq)]
pub enum ClasseErreur {
    /// Quota/rate exceeded. `reset_at` = epoch seconds if known (`Retry-After`/`reset` header).
    RateLimited { reset_at: Option<i64> },
    /// Invalid/expired authentication: model failover is pointless.
    ReloginRequis,
    /// Transient failure (5xx, network reset, timeout): retry on the SAME model.
    Transitoire,
    /// Definitive error (4xx except 401/403/429, invalid request): model failover.
    Fatal,
}

impl ClasseErreur {
    /// Classifies an error from the HTTP status and hints (body, retry-after header).
    pub fn classer(status: u16, retry_after: Option<&str>, corps: &str) -> ClasseErreur {
        match status {
            429 => ClasseErreur::RateLimited {
                reset_at: parser_retry_after(retry_after),
            },
            401 | 403 => {
                // Some providers return 403 for a plain exhausted quota.
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
            // 0 = transport error (no HTTP response) -> transient.
            0 => ClasseErreur::Transitoire,
            _ => ClasseErreur::Fatal,
        }
    }

    pub fn exige_relogin(&self) -> bool {
        matches!(self, ClasseErreur::ReloginRequis)
    }
}

/// What the loop must do in response to the error.
#[derive(Debug, Clone, PartialEq)]
pub enum Reaction {
    /// Sleep N seconds then retry the same model/key.
    Patienter(u64),
    /// Try the next available API key (`credential_pool` rotation).
    RotationCle,
    /// Switch to a fallback model (failover).
    Deroutement,
    /// Give up with this reason (relogin required, or recourses exhausted).
    Stopper(String),
}

/// Reaction policy. Pure: `(classe, tentative, caps) -> Reaction`.
///
/// - `tentative`: current attempt number (1-based).
/// - `max_rate_limit`: max waits on rate-limit before trying something else.
/// - `max_transitoire`: max retries on a transient failure.
/// - `cle_dispo`: is another API key available (rotation possible)?
/// - `repli_dispo`: does a fallback model exist (failover possible)?
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
            Reaction::Stopper("invalid authentication, reconnect the provider".into())
        }
        ClasseErreur::RateLimited { reset_at } => {
            if cle_dispo {
                // Another key is free: rotate rather than wait.
                Reaction::RotationCle
            } else if tentative <= max_rate_limit {
                Reaction::Patienter(delai_rate_limit(*reset_at, tentative, now))
            } else if repli_dispo {
                Reaction::Deroutement
            } else {
                Reaction::Stopper(format!(
                    "persistent rate limit after {max_rate_limit} wait(s)"
                ))
            }
        }
        ClasseErreur::Transitoire => {
            if tentative <= max_transitoire {
                Reaction::Patienter(delai_backoff(tentative))
            } else if repli_dispo {
                Reaction::Deroutement
            } else {
                Reaction::Stopper(format!("persistent transient failure after {max_transitoire} attempts"))
            }
        }
        ClasseErreur::Fatal => {
            if repli_dispo {
                Reaction::Deroutement
            } else {
                Reaction::Stopper("fatal provider error, no fallback available".into())
            }
        }
    }
}

/// Bounded exponential backoff (1s, 2s, 4s, 8s... capped at 30s).
pub fn delai_backoff(tentative: usize) -> u64 {
    let base = 1u64 << (tentative.saturating_sub(1)).min(5); // 1..=32
    base.min(30)
}

/// Wait delay on rate-limit: prefers `Retry-After`/`reset`, otherwise a
/// reasonable RPM window that grows with attempts.
pub fn delai_rate_limit(reset_at: Option<i64>, tentative: usize, now: i64) -> u64 {
    if let Some(reset) = reset_at {
        let delta = reset - now;
        if delta > 0 {
            // +2s margin, capped at 5 min.
            return ((delta as u64) + 2).min(300);
        }
    }
    // Without header: 65s, 90s, 120s...
    match tentative {
        1 => 65,
        2 => 90,
        _ => 120,
    }
}

/// Parses a `Retry-After` header ("42" seconds, or absolute epoch).
fn parser_retry_after(h: Option<&str>) -> Option<i64> {
    let s = h?.trim();
    if let Ok(secs) = s.parse::<i64>() {
        // Heuristic: small number = relative delay; large = absolute epoch.
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
        // attempt beyond max + fallback available -> failover
        assert_eq!(reagir(&cl, 3, 2, 3, false, true, 0), Reaction::Deroutement);
        // no fallback -> stop
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
        assert_eq!(delai_backoff(100), 30); // capped
    }

    #[test]
    fn rate_limit_utilise_reset_at() {
        // reset in 40s -> ~42s wait
        let d = delai_rate_limit(Some(1000), 1, 960);
        assert_eq!(d, 42);
        // without header -> RPM window
        assert_eq!(delai_rate_limit(None, 1, 0), 65);
        assert_eq!(delai_rate_limit(None, 2, 0), 90);
    }
}
