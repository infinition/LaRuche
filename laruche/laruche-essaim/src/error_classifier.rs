use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorClass {
    Retryable,
    RateLimited { reset_at: Option<i64> },
    ReloginRequired,
    Fatal,
}

impl ErrorClass {
    pub fn est_reessayable(&self) -> bool {
        matches!(self, ErrorClass::Retryable | ErrorClass::RateLimited { .. })
    }

    pub fn exige_relogin(&self) -> bool {
        matches!(self, ErrorClass::ReloginRequired)
    }
}

const RELOGIN_CODES: &[&str] = &[
    "invalid_grant",
    "invalid_token",
    "refresh_token_reused",
    "invalid_api_key",
    "authentication_error",
    "permission_error",
    "account_deactivated",
    "unauthorized_client",
];

const AUTH_PATTERNS: &[&str] = &[
    "invalid api key",
    "invalid_api_key",
    "authentication",
    "unauthorized",
    "forbidden",
    "token expired",
    "token revoked",
    "access denied",
];

const RATE_LIMIT_PATTERNS: &[&str] = &[
    "rate limit",
    "rate_limit",
    "too many requests",
    "throttled",
    "requests per minute",
    "tokens per minute",
    "try again in",
    "please retry after",
    "resource_exhausted",
    "servicequotaexceededexception",
];

const BILLING_PATTERNS: &[&str] = &[
    "insufficient credits",
    "insufficient_quota",
    "insufficient balance",
    "credit balance",
    "credits exhausted",
    "payment required",
    "billing hard limit",
    "out of funds",
    "balance_depleted",
];

const USAGE_LIMIT_PATTERNS: &[&str] = &["usage limit", "quota", "limit exceeded"];
const TRANSIENT_USAGE_SIGNALS: &[&str] = &[
    "try again",
    "retry",
    "resets at",
    "reset in",
    "wait",
    "window",
];

const CONTEXT_PATTERNS: &[&str] = &[
    "context length",
    "context size",
    "maximum context",
    "token limit",
    "too many tokens",
    "context window",
    "prompt is too long",
    "max_model_len",
    "maximum model length",
    "exceeds the maximum number of input tokens",
];

const MODEL_NOT_FOUND_PATTERNS: &[&str] = &[
    "is not a valid model",
    "invalid model",
    "model not found",
    "model_not_found",
    "does not exist",
    "no such model",
    "unknown model",
    "unsupported model",
];

const REQUEST_VALIDATION_PATTERNS: &[&str] = &[
    "unknown parameter",
    "unsupported parameter",
    "unrecognized request argument",
    "unknown_parameter",
    "unsupported_parameter",
];

pub fn parse_retry_after_seconds(retry_after: &str) -> Option<i64> {
    retry_after.trim().parse::<i64>().ok().filter(|n| *n >= 0)
}

pub fn classifier(status: u16, body: &str) -> ErrorClass {
    classifier_avec_retry_after(status, body, None)
}

pub fn classifier_avec_retry_after(
    status: u16,
    body: &str,
    retry_after: Option<&str>,
) -> ErrorClass {
    let message = message_normalisee(body);
    let code = extraire_code(body).unwrap_or_default().to_lowercase();

    if RELOGIN_CODES.iter().any(|expected| code == *expected)
        || contient_un(&message, AUTH_PATTERNS)
    {
        return ErrorClass::ReloginRequired;
    }

    if status == 429 || contient_un(&message, RATE_LIMIT_PATTERNS) {
        return ErrorClass::RateLimited {
            reset_at: reset_at(retry_after),
        };
    }

    if status == 402 || contient_un(&message, BILLING_PATTERNS) {
        if contient_un(&message, USAGE_LIMIT_PATTERNS)
            && contient_un(&message, TRANSIENT_USAGE_SIGNALS)
        {
            return ErrorClass::RateLimited {
                reset_at: reset_at(retry_after),
            };
        }
        return ErrorClass::Fatal;
    }

    if status == 413 || contient_un(&message, CONTEXT_PATTERNS) {
        return ErrorClass::Retryable;
    }

    if contient_un(&message, MODEL_NOT_FOUND_PATTERNS)
        || contient_un(&message, REQUEST_VALIDATION_PATTERNS)
    {
        return ErrorClass::Fatal;
    }

    match status {
        401 | 403 => ErrorClass::ReloginRequired,
        408 | 425 | 500 | 502 | 503 | 504 | 529 => ErrorClass::Retryable,
        s if (400..500).contains(&s) => ErrorClass::Fatal,
        s if (500..600).contains(&s) => ErrorClass::Retryable,
        _ => ErrorClass::Retryable,
    }
}

pub fn classifier_erreur_reseau(err: &str) -> ErrorClass {
    let e = err.to_lowercase();
    let transitoire = [
        "timed out",
        "timeout",
        "deadline exceeded",
        "connection",
        "connect",
        "reset",
        "refused",
        "broken pipe",
        "dns",
        "resolve",
        "unreachable",
        "temporarily",
        "unexpected eof",
        "server disconnected",
    ];
    if transitoire.iter().any(|needle| e.contains(needle)) {
        ErrorClass::Retryable
    } else {
        // A network error matching none of the known-transient patterns is treated as fatal,
        // so the retry loop does not spin on a genuinely unrecoverable failure.
        ErrorClass::Fatal
    }
}

fn reset_at(retry_after: Option<&str>) -> Option<i64> {
    retry_after
        .and_then(parse_retry_after_seconds)
        .map(|secs| chrono::Utc::now().timestamp() + secs)
}

fn contient_un(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

fn message_normalisee(body: &str) -> String {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(body) {
        let mut chunks = Vec::new();
        collect_strings(&value, &mut chunks);
        if !chunks.is_empty() {
            return chunks.join(" ").to_lowercase();
        }
    }
    body.to_lowercase()
}

fn collect_strings(value: &serde_json::Value, chunks: &mut Vec<String>) {
    match value {
        serde_json::Value::String(value) => chunks.push(value.clone()),
        serde_json::Value::Array(values) => {
            for value in values {
                collect_strings(value, chunks);
            }
        }
        serde_json::Value::Object(map) => {
            for value in map.values() {
                collect_strings(value, chunks);
            }
        }
        _ => {}
    }
}

fn extraire_code(body: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(body).ok()?;

    if let Some(error) = value.get("error") {
        match error {
            serde_json::Value::Object(map) => {
                return map
                    .get("code")
                    .or_else(|| map.get("type"))
                    .and_then(|code| code.as_str())
                    .map(|code| code.trim().to_string());
            }
            serde_json::Value::String(code) => return Some(code.trim().to_string()),
            _ => {}
        }
    }

    value
        .get("code")
        .or_else(|| value.get("error_code"))
        .and_then(|code| code.as_str())
        .map(|code| code.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_limit_429_avec_retry_after() {
        let class = classifier_avec_retry_after(429, "{}", Some("30"));
        match class {
            ErrorClass::RateLimited { reset_at: Some(t) } => {
                assert!(t > chrono::Utc::now().timestamp());
            }
            other => panic!("expected RateLimited, got {other:?}"),
        }
    }

    #[test]
    fn rate_limit_par_message_provider() {
        let body = r#"{"error":{"message":"Too many requests, please retry after a minute"}}"#;

        assert_eq!(
            classifier(400, body),
            ErrorClass::RateLimited { reset_at: None }
        );
    }

    #[test]
    fn usage_limit_transitoire_est_rate_limit() {
        let body = r#"{"error":{"message":"Usage limit reached, try again in 5 minutes"}}"#;

        assert_eq!(
            classifier(402, body),
            ErrorClass::RateLimited { reset_at: None }
        );
    }

    #[test]
    fn auth_401_relogin() {
        assert_eq!(classifier(401, "{}"), ErrorClass::ReloginRequired);
        assert_eq!(classifier(403, "{}"), ErrorClass::ReloginRequired);
    }

    #[test]
    fn invalid_grant_force_relogin_meme_en_400() {
        let body = r#"{"error":"invalid_grant","error_description":"token rotated"}"#;

        assert_eq!(classifier(400, body), ErrorClass::ReloginRequired);
    }

    #[test]
    fn openai_shape_invalid_api_key() {
        let body = r#"{"error":{"message":"bad key","type":"invalid_request_error","code":"invalid_api_key"}}"#;

        assert_eq!(classifier(400, body), ErrorClass::ReloginRequired);
    }

    #[test]
    fn contexte_trop_long_est_retryable_pour_compaction() {
        let body = r#"{"error":{"message":"This model's maximum context length is exceeded"}}"#;

        assert_eq!(classifier(400, body), ErrorClass::Retryable);
    }

    #[test]
    fn parametre_inconnu_est_fatal() {
        let body = r#"{"error":{"message":"Unsupported parameter: max_tokens"}}"#;

        assert_eq!(classifier(400, body), ErrorClass::Fatal);
    }

    #[test]
    fn cinq_cents_est_retryable() {
        assert_eq!(classifier(500, "{}"), ErrorClass::Retryable);
        assert_eq!(classifier(503, "oops"), ErrorClass::Retryable);
    }

    #[test]
    fn quatre_cents_generique_est_fatal() {
        assert_eq!(
            classifier(400, r#"{"error":{"code":"bad_param"}}"#),
            ErrorClass::Fatal
        );
        assert_eq!(classifier(404, "{}"), ErrorClass::Fatal);
    }

    #[test]
    fn erreur_reseau_retryable() {
        assert_eq!(
            classifier_erreur_reseau("connection timed out"),
            ErrorClass::Retryable
        );
        assert_eq!(classifier_erreur_reseau("dns error"), ErrorClass::Retryable);
    }

    #[test]
    fn helpers_de_decision() {
        assert!(classifier(429, "{}").est_reessayable());
        assert!(classifier(401, "{}").exige_relogin());
        assert!(!classifier(400, "{}").est_reessayable());
    }
}
