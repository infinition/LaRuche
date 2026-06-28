//! ChatGPT Codex subscription authentication (OAuth), replicated from third-party.
//!
//! Lets you use the ChatGPT subscription quota (Plus/Pro) instead of a billed
//! API key. The flow is the OpenAI *device code*:
//!   1. request a `user_code` + `device_auth_id`
//!   2. the user opens https://auth.openai.com/codex/device and enters the code
//!   3. poll until obtaining an `authorization_code` + `code_verifier` (PKCE)
//!   4. exchange for an `access_token` + `refresh_token`
//!
//! Tokens are stored in `~/.laruche/auth.json` (a dedicated LaRuche session,
//! separate from the Codex CLI / VS Code to avoid token rotation conflicts).
//! We can also import/auto-repair from `~/.codex/auth.json` if present.
//!
//! At inference time we hit `https://chatgpt.com/backend-api/codex/responses`
//! (Responses API) with the anti-Cloudflare headers (`originator: codex_cli_rs`,
//! `ChatGPT-Account-ID` extracted from the JWT claim).

use anyhow::{anyhow, bail, Context, Result};
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

// --- Constants (identical to third-party / codex-rs) -----------------------------

pub const CODEX_OAUTH_CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
pub const CODEX_OAUTH_ISSUER: &str = "https://auth.openai.com";
pub const CODEX_OAUTH_TOKEN_URL: &str = "https://auth.openai.com/oauth/token";
pub const DEFAULT_CODEX_BASE_URL: &str = "https://chatgpt.com/backend-api/codex";
/// Refresh the access token when it has less than 120 s of validity left.
pub const ACCESS_TOKEN_REFRESH_SKEW_SECONDS: i64 = 120;

// --- Storage model ----------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CodexTokens {
    pub access_token: String,
    pub refresh_token: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_refresh: Option<String>,
}

impl CodexTokens {
    pub fn is_complete(&self) -> bool {
        !self.access_token.trim().is_empty() && !self.refresh_token.trim().is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct ProviderState {
    #[serde(default)]
    tokens: Option<CodexTokens>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    last_refresh: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct AuthStore {
    #[serde(default)]
    providers: HashMap<String, ProviderState>,
}

/// Path of the LaRuche auth store: `~/.laruche/auth.json`.
pub fn auth_store_path() -> PathBuf {
    home_dir().join(".laruche").join("auth.json")
}

fn home_dir() -> PathBuf {
    // Windows: USERPROFILE; Unix: HOME.
    if let Ok(p) = std::env::var("USERPROFILE") {
        if !p.trim().is_empty() {
            return PathBuf::from(p);
        }
    }
    if let Ok(p) = std::env::var("HOME") {
        if !p.trim().is_empty() {
            return PathBuf::from(p);
        }
    }
    PathBuf::from(".")
}

fn load_store() -> AuthStore {
    let path = auth_store_path();
    match std::fs::read_to_string(&path) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
        Err(_) => AuthStore::default(),
    }
}

fn save_store(store: &AuthStore) -> Result<()> {
    let path = auth_store_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating directory {}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(store)?;
    std::fs::write(&path, json).with_context(|| format!("writing {}", path.display()))?;
    // 0600 permissions on Unix (best-effort): the store holds secrets.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

/// Removes the stored Codex tokens (sign out).
pub fn clear_codex_tokens() -> Result<()> {
    let mut store = load_store();
    store.providers.remove("openai-codex");
    save_store(&store)
}

/// Reads the stored Codex tokens (None if absent / incomplete).
pub fn read_codex_tokens() -> Option<CodexTokens> {
    let store = load_store();
    let state = store.providers.get("openai-codex")?;
    let tokens = state.tokens.clone()?;
    if tokens.is_complete() {
        Some(tokens)
    } else {
        None
    }
}

/// Persists the Codex tokens in the LaRuche store.
pub fn save_codex_tokens(tokens: &CodexTokens) -> Result<()> {
    let mut store = load_store();
    let last_refresh = tokens.last_refresh.clone().unwrap_or_else(|| now_iso8601());
    store.providers.insert(
        "openai-codex".to_string(),
        ProviderState {
            tokens: Some(CodexTokens {
                last_refresh: Some(last_refresh.clone()),
                ..tokens.clone()
            }),
            last_refresh: Some(last_refresh),
        },
    );
    save_store(&store)
}

// --- JWT decoding (no signature verification: we just read the claims) -------

fn decode_jwt_claims(token: &str) -> Option<serde_json::Value> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() < 2 {
        return None;
    }
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(parts[1])
        .ok()?;
    serde_json::from_slice(&payload).ok()
}

/// `true` if the access token expires within `skew_seconds` (or is unreadable).
pub fn access_token_is_expiring(access_token: &str, skew_seconds: i64) -> bool {
    let claims = match decode_jwt_claims(access_token) {
        Some(c) => c,
        None => return true, // Opaque/unreadable token: force a refresh.
    };
    let exp = match claims.get("exp").and_then(|v| v.as_i64()) {
        Some(e) => e,
        None => return false, // No exp: assume valid.
    };
    let now = chrono::Utc::now().timestamp();
    now + skew_seconds >= exp
}

/// Extracts the `chatgpt_account_id` from the `https://api.openai.com/auth` claim.
pub fn account_id_from_token(access_token: &str) -> Option<String> {
    let claims = decode_jwt_claims(access_token)?;
    claims
        .get("https://api.openai.com/auth")?
        .get("chatgpt_account_id")?
        .as_str()
        .map(|s| s.to_string())
}

fn now_iso8601() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

// --- Inference headers (anti-Cloudflare, like codex-rs) ---------------------

/// Headers required to avoid Cloudflare 403s on chatgpt.com/backend-api/codex.
pub fn codex_headers(access_token: &str) -> HashMap<String, String> {
    let mut headers = HashMap::new();
    headers.insert(
        "User-Agent".to_string(),
        "codex_cli_rs/0.0.0 (LaRuche)".to_string(),
    );
    headers.insert("originator".to_string(), "codex_cli_rs".to_string());
    if let Some(acct) = account_id_from_token(access_token) {
        headers.insert("ChatGPT-Account-ID".to_string(), acct);
    }
    headers
}

// --- Import / auto-repair from the Codex CLI (~/.codex/auth.json) ------------

#[derive(Debug, Deserialize)]
struct CodexCliAuth {
    tokens: Option<CodexCliTokens>,
}

#[derive(Debug, Deserialize)]
struct CodexCliTokens {
    access_token: Option<String>,
    refresh_token: Option<String>,
}

/// Tries to read valid tokens from `~/.codex/auth.json` (Codex CLI).
/// Never modifies that shared file.
pub fn import_codex_cli_tokens() -> Option<CodexTokens> {
    let codex_home = std::env::var("CODEX_HOME")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| home_dir().join(".codex"));
    let auth_path = codex_home.join("auth.json");
    let raw = std::fs::read_to_string(&auth_path).ok()?;
    let parsed: CodexCliAuth = serde_json::from_str(&raw).ok()?;
    let t = parsed.tokens?;
    let access = t.access_token.unwrap_or_default();
    let refresh = t.refresh_token.unwrap_or_default();
    if access.trim().is_empty() || refresh.trim().is_empty() {
        return None;
    }
    // Reject already-expired tokens: importing them would leave the user
    // stuck on a "connected" state with no usable credential.
    if access_token_is_expiring(&access, 0) {
        return None;
    }
    Some(CodexTokens {
        access_token: access,
        refresh_token: refresh,
        last_refresh: None,
    })
}

// --- OAuth refresh ----------------------------------------------------------

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: Option<String>,
    refresh_token: Option<String>,
    #[allow(dead_code)]
    id_token: Option<String>,
}

/// Refreshes the access token using the refresh token. Returns the new tokens
/// (the refresh token may be rotated by the server).
pub async fn refresh_codex_oauth(refresh_token: &str) -> Result<CodexTokens> {
    if refresh_token.trim().is_empty() {
        bail!("missing refresh_token: re-run `laruche auth codex`.");
    }
    let client = reqwest::Client::new();
    let resp = client
        .post(CODEX_OAUTH_TOKEN_URL)
        .header("Accept", "application/json")
        .form(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", CODEX_OAUTH_CLIENT_ID),
        ])
        .timeout(Duration::from_secs(20))
        .send()
        .await
        .context("calling the Codex OAuth refresh endpoint")?;

    let status = resp.status();
    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
        bail!("Codex quota exhausted (429): credentials still valid, retry later.");
    }
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        bail!("Codex refresh failed (HTTP {status}): {body}");
    }
    let payload: TokenResponse = resp
        .json()
        .await
        .context("invalid Codex refresh JSON")?;
    let access = payload
        .access_token
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| anyhow!("Codex refresh without access_token"))?;
    let next_refresh = payload
        .refresh_token
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| refresh_token.to_string());
    Ok(CodexTokens {
        access_token: access.trim().to_string(),
        refresh_token: next_refresh.trim().to_string(),
        last_refresh: Some(now_iso8601()),
    })
}

/// Resolves a usable Codex access token:
///   - reads the store, imports from the Codex CLI if empty,
///   - refreshes if the access token is expiring,
///   - auto-repairs via the Codex CLI if the refresh fails (cross-store rotation),
///   - persists any refreshed token.
pub async fn resolve_codex_access_token() -> Result<String> {
    let mut tokens = match read_codex_tokens() {
        Some(t) => t,
        None => import_codex_cli_tokens().ok_or_else(|| {
            anyhow!("No Codex credential. Run `laruche auth codex` to sign in.")
        })?,
    };

    if !access_token_is_expiring(&tokens.access_token, ACCESS_TOKEN_REFRESH_SKEW_SECONDS) {
        return Ok(tokens.access_token);
    }

    // Refresh required.
    match refresh_codex_oauth(&tokens.refresh_token).await {
        Ok(fresh) => {
            save_codex_tokens(&fresh)?;
            Ok(fresh.access_token)
        }
        Err(e) => {
            // Auto-repair: adopt the canonical token from the Codex CLI
            // (the cached refresh_token may have been consumed by another client).
            if let Some(imported) = import_codex_cli_tokens() {
                save_codex_tokens(&imported)?;
                tokens = imported;
                return Ok(tokens.access_token);
            }
            Err(e.context("Codex refresh failed and no CLI fallback available"))
        }
    }
}

// --- Device code login flow -------------------------------------------------

#[derive(Debug, Deserialize)]
struct DeviceCodeResponse {
    user_code: Option<String>,
    device_auth_id: Option<String>,
    #[serde(default)]
    interval: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct DevicePollResponse {
    authorization_code: Option<String>,
    code_verifier: Option<String>,
}

/// Runs the interactive device code flow and returns the tokens (not persisted).
/// `on_prompt` is called with the URL and code to show to the user.
pub async fn device_code_login<F>(on_prompt: F) -> Result<CodexTokens>
where
    F: FnOnce(&str, &str),
{
    let client = reqwest::Client::new();

    // Step 1: request the device code (with backoff on 429).
    let mut device: Option<DeviceCodeResponse> = None;
    let max_attempts = 4;
    for attempt in 1..=max_attempts {
        let resp = client
            .post(format!(
                "{CODEX_OAUTH_ISSUER}/api/accounts/deviceauth/usercode"
            ))
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({ "client_id": CODEX_OAUTH_CLIENT_ID }))
            .timeout(Duration::from_secs(15))
            .send()
            .await
            .context("requesting Codex device code")?;

        if resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            if attempt < max_attempts {
                let delay = (2u64.pow(attempt)).min(60);
                eprintln!("OpenAI is rate-limiting sign-ins (429); retrying in {delay}s...");
                tokio::time::sleep(Duration::from_secs(delay)).await;
                continue;
            }
            bail!("OpenAI is rate-limiting Codex sign-ins (HTTP 429). Retry in a minute.");
        }
        if !resp.status().is_success() {
            bail!("Device code request: status {}", resp.status());
        }
        device = Some(resp.json().await.context("invalid device code JSON")?);
        break;
    }

    let device = device.ok_or_else(|| anyhow!("no device code response"))?;
    let user_code = device
        .user_code
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("device code response without user_code"))?;
    let device_auth_id = device
        .device_auth_id
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("device code response without device_auth_id"))?;
    let poll_interval = device
        .interval
        .as_ref()
        .and_then(value_to_u64)
        .unwrap_or(5)
        .max(3);

    // Step 2: show the URL + code to the user.
    on_prompt(&format!("{CODEX_OAUTH_ISSUER}/codex/device"), &user_code);

    // Step 3: poll until authorized (max 15 min).
    let deadline = std::time::Instant::now() + Duration::from_secs(15 * 60);
    let mut code_resp: Option<DevicePollResponse> = None;
    while std::time::Instant::now() < deadline {
        tokio::time::sleep(Duration::from_secs(poll_interval)).await;
        let poll = client
            .post(format!(
                "{CODEX_OAUTH_ISSUER}/api/accounts/deviceauth/token"
            ))
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "device_auth_id": device_auth_id,
                "user_code": user_code,
            }))
            .timeout(Duration::from_secs(15))
            .send()
            .await
            .context("polling Codex device auth")?;
        match poll.status().as_u16() {
            200 => {
                code_resp = Some(poll.json().await.context("invalid polling JSON")?);
                break;
            }
            403 | 404 => continue, // not signed in yet
            other => bail!("Device auth polling: status {other}"),
        }
    }

    let code_resp = code_resp.ok_or_else(|| anyhow!("Sign-in expired after 15 minutes."))?;
    let authorization_code = code_resp
        .authorization_code
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("polling response without authorization_code"))?;
    let code_verifier = code_resp
        .code_verifier
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("polling response without code_verifier"))?;

    // Step 4: exchange the code for tokens (PKCE).
    let redirect_uri = format!("{CODEX_OAUTH_ISSUER}/deviceauth/callback");
    let token_resp = client
        .post(CODEX_OAUTH_TOKEN_URL)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", authorization_code.as_str()),
            ("redirect_uri", redirect_uri.as_str()),
            ("client_id", CODEX_OAUTH_CLIENT_ID),
            ("code_verifier", code_verifier.as_str()),
        ])
        .timeout(Duration::from_secs(15))
        .send()
        .await
        .context("exchanging the Codex authorization code")?;

    if token_resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
        bail!("OpenAI is rate-limiting the token exchange (429). Retry in a minute.");
    }
    if !token_resp.status().is_success() {
        let status = token_resp.status();
        let body = token_resp.text().await.unwrap_or_default();
        bail!("Codex token exchange: status {status}: {body}");
    }
    let payload: TokenResponse = token_resp.json().await.context("invalid exchange JSON")?;
    let access = payload
        .access_token
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| anyhow!("token exchange without access_token"))?;
    let refresh = payload.refresh_token.unwrap_or_default();

    Ok(CodexTokens {
        access_token: access.trim().to_string(),
        refresh_token: refresh.trim().to_string(),
        last_refresh: Some(now_iso8601()),
    })
}

fn value_to_u64(v: &serde_json::Value) -> Option<u64> {
    match v {
        serde_json::Value::Number(n) => n.as_u64(),
        serde_json::Value::String(s) => s.parse().ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_jwt(claims: serde_json::Value) -> String {
        let header = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(br#"{"alg":"none"}"#);
        let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(serde_json::to_vec(&claims).unwrap());
        format!("{header}.{payload}.sig")
    }

    #[test]
    fn account_id_extrait_du_claim() {
        let token = make_jwt(serde_json::json!({
            "https://api.openai.com/auth": { "chatgpt_account_id": "acct_123" },
            "exp": 9999999999i64
        }));
        assert_eq!(account_id_from_token(&token).as_deref(), Some("acct_123"));
    }

    #[test]
    fn token_expirant_detecte() {
        let past = chrono::Utc::now().timestamp() - 10;
        let token = make_jwt(serde_json::json!({ "exp": past }));
        assert!(access_token_is_expiring(&token, 0));

        let future = chrono::Utc::now().timestamp() + 3600;
        let token2 = make_jwt(serde_json::json!({ "exp": future }));
        assert!(!access_token_is_expiring(&token2, 120));
    }

    #[test]
    fn token_opaque_force_refresh() {
        assert!(access_token_is_expiring("pas-un-jwt", 0));
    }

    #[test]
    fn headers_contiennent_originator() {
        let token = make_jwt(serde_json::json!({
            "https://api.openai.com/auth": { "chatgpt_account_id": "acct_xyz" }
        }));
        let h = codex_headers(&token);
        assert_eq!(
            h.get("originator").map(|s| s.as_str()),
            Some("codex_cli_rs")
        );
        assert_eq!(
            h.get("ChatGPT-Account-ID").map(|s| s.as_str()),
            Some("acct_xyz")
        );
        assert!(h.contains_key("User-Agent"));
    }
}
