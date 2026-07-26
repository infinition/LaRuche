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
            429 => {
                // A 429 for an EXHAUSTED BALANCE (no credit / no resource package, e.g.
                // z.ai code 1113) will never resolve on its own: do not waste retries,
                // treat it as fatal so the loop stops with a clear message. A 429 that is
                // a genuine RATE limit (too fast) stays retryable.
                let c = corps.to_lowercase();
                let solde_vide = c.contains("balance")
                    || c.contains("insufficient")
                    || c.contains("recharge")
                    || c.contains("resource package")
                    || c.contains("billing")
                    || c.contains("payment");
                if solde_vide {
                    ClasseErreur::Fatal
                } else {
                    ClasseErreur::RateLimited {
                        reset_at: parser_retry_after(retry_after),
                    }
                }
            }
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
            400 => {
                // A 400 usually means WE built the request wrong, and sending it again
                // reproduces it exactly: fatal is right.
                //
                // One family is different: the provider says it could not PARSE the body
                // we sent. We know what we emit is valid JSON, because serde_json cannot
                // produce a non-string key or an unterminated value, and a test pins the
                // whole tools payload down (`les_schemas_doutils_produisent_du_json_valide`).
                // So a parse failure means the bytes did not arrive intact. Observed twice
                // on the same setup, at two different offsets of two different requests:
                // "EOF while parsing a string at line 1 column 102271" and
                // "properties.?: key must be a string at line 1 column 81736". A corrupted
                // upload is transport, not logic: the identical call succeeds on retry.
                //
                // SEMANTIC 400s stay fatal (duplicate tool_call_id, unknown model, bad
                // field): those reproduce exactly and retrying only wastes attempts.
                let c = corps.to_lowercase();
                let corps_illisible = c.contains("failed to parse the request body")
                    || c.contains("eof while parsing")
                    || c.contains("unexpected end of")
                    || c.contains("key must be a string");
                if corps_illisible {
                    ClasseErreur::Transitoire
                } else {
                    ClasseErreur::Fatal
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

/// Parses a `Retry-After` header ("42" seconds, absolute epoch, or an HTTP-date
/// like "Wed, 21 Oct 2026 07:28:00 GMT" - used by some proxies/Cloudflare).
fn parser_retry_after(h: Option<&str>) -> Option<i64> {
    let s = h?.trim();
    if let Ok(secs) = s.parse::<i64>() {
        // Heuristic: small number = relative delay; large = absolute epoch.
        if secs < 10_000_000 {
            return Some(chrono::Utc::now().timestamp() + secs);
        }
        return Some(secs);
    }
    if let Ok(date) = chrono::DateTime::parse_from_rfc2822(s) {
        return Some(date.timestamp());
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
    fn classe_429_solde_vide_est_fatal() {
        // z.ai code 1113: empty balance never resolves -> fatal, stop immediately.
        assert_eq!(
            ClasseErreur::classer(429, None, "Insufficient balance or no resource package"),
            ClasseErreur::Fatal
        );
        // A genuine rate limit (too fast) stays retryable.
        assert!(matches!(
            ClasseErreur::classer(429, None, "Rate limit reached for requests"),
            ClasseErreur::RateLimited { .. }
        ));
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

    /// Real case: a ~100 KB request whose body reached deepseek truncated. Nothing
    /// was wrong with what we serialized (serde_json plus reqwest cannot emit
    /// unterminated JSON), so retrying the identical call succeeds. Classifying it
    /// Fatal killed the turn with "no fallback available".
    #[test]
    #[test]
    fn un_400_de_corps_tronque_est_transitoire() {
        let corps = "Failed to parse the request body as JSON: messages[21].content:                      EOF while parsing a string at line 1 column 102271";
        assert_eq!(ClasseErreur::classer(400, None, corps), ClasseErreur::Transitoire);
        assert_eq!(
            ClasseErreur::classer(400, None, "unexpected end of input"),
            ClasseErreur::Transitoire
        );
        // Second real case, same setup, different offset: the corruption landed mid-body.
        let cle = "Failed to parse the request body as JSON:                    tools[16].function.parameters.properties.steps.items.properties.?:                    key must be a string at line 1 column 81736";
        assert_eq!(ClasseErreur::classer(400, None, cle), ClasseErreur::Transitoire);
        // A 400 that is genuinely OUR fault must stay fatal: retrying reproduces it.
        let logique = "Duplicate value for 'tool_call_id' of call_00_abc in message[2]";
        assert_eq!(ClasseErreur::classer(400, None, logique), ClasseErreur::Fatal);
        assert_eq!(
            ClasseErreur::classer(400, None, "invalid model name"),
            ClasseErreur::Fatal
        );
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
    fn retry_after_http_date_est_parse() {
        let epoch = parser_retry_after(Some("Wed, 21 Oct 2026 07:28:00 GMT"));
        assert_eq!(epoch, Some(1792567680));
        assert_eq!(parser_retry_after(Some("n'importe quoi")), None);
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
