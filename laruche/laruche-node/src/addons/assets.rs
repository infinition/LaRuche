use super::AssetLookupError;
use crate::AppState;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{header, HeaderMap, HeaderValue, Response, StatusCode};
use std::sync::Arc;

pub(crate) async fn serve(
    State(state): State<Arc<AppState>>,
    request_headers: HeaderMap,
    Path((id, version, path)): Path<(String, String, String)>,
) -> Response<Body> {
    let resolved = {
        let registry = state.addons.read().await;
        registry.resolve_ui_asset(&id, &version, &path)
    };
    let resolved = match resolved {
        Ok(path) => path,
        Err(error) => return lookup_error(error),
    };

    // Recheck immediately before opening. Installed packages are immutable by
    // contract, but a manually edited data directory must not turn a validated
    // regular file into an unbounded stream or link after resolution.
    let metadata = match tokio::fs::symlink_metadata(&resolved).await {
        Ok(metadata)
            if metadata.is_file()
                && !metadata.file_type().is_symlink()
                && metadata.len() <= 32 * 1024 * 1024 =>
        {
            metadata
        }
        Ok(_) => return not_found(),
        Err(_) => return not_found(),
    };
    let bytes = match tokio::fs::read(&resolved).await {
        Ok(bytes) if bytes.len() as u64 == metadata.len() => bytes,
        _ => return not_found(),
    };

    let mime = mime_guess::from_path(&resolved).first_or_octet_stream();
    let is_html = mime.essence_str() == "text/html";
    let mut response = Response::new(Body::from(bytes));
    *response.status_mut() = StatusCode::OK;
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(mime.as_ref())
            .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream")),
    );
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=31536000, immutable"),
    );
    headers.insert(
        header::HeaderName::from_static("x-content-type-options"),
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        header::HeaderName::from_static("cross-origin-resource-policy"),
        // The sandboxed document deliberately has an opaque origin. Its own JS,
        // CSS, images and fonts are therefore cross-origin from the browser's
        // point of view even though every URL stays inside this package.
        HeaderValue::from_static("cross-origin"),
    );
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    );
    if is_html {
        if let Some(policy) = html_policy(request_headers.get(header::HOST), &id, &version) {
            headers.insert(header::CONTENT_SECURITY_POLICY, policy);
        }
    }
    response
}

/// A sandbox without `allow-same-origin` gives the document an opaque origin.
/// Consequently CSP's `'self'` cannot load even the addon's own JS/CSS. Name the
/// exact package URL instead: scripts may come from this id/version only, while
/// `connect-src 'none'` keeps LaRuche APIs and the network unreachable.
fn html_policy(host: Option<&HeaderValue>, id: &str, version: &str) -> Option<HeaderValue> {
    let host = host?.to_str().ok()?;
    if host.is_empty()
        || host.len() > 255
        || !host
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b".-:[]".contains(&byte))
    {
        return None;
    }
    let http = format!("http://{host}/addons-assets/{id}/{version}/");
    let https = format!("https://{host}/addons-assets/{id}/{version}/");
    let policy = format!(
        "default-src 'none'; script-src {http} {https}; style-src {http} {https} 'unsafe-inline'; img-src {http} {https} data: blob:; font-src {http} {https}; connect-src 'none'; object-src 'none'; base-uri 'none'; form-action 'none'; frame-ancestors http://{host} https://{host}"
    );
    HeaderValue::from_str(&policy).ok()
}

fn lookup_error(error: AssetLookupError) -> Response<Body> {
    match error {
        AssetLookupError::Io(message) => {
            tracing::warn!(error = %message, "addon asset lookup failed");
            not_found()
        }
        AssetLookupError::NotFound
        | AssetLookupError::Disabled
        | AssetLookupError::InvalidPath
        | AssetLookupError::TooLarge => not_found(),
    }
}

fn not_found() -> Response<Body> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .header(header::CACHE_CONTROL, "no-store")
        .header("x-content-type-options", "nosniff")
        .body(Body::empty())
        .expect("static addon asset error response")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_policy_disallows_network_and_parent_access() {
        let policy = html_policy(
            Some(&HeaderValue::from_static("localhost:8419")),
            "dev.laruche.test",
            "1.2.3",
        )
        .unwrap();
        let policy = policy.to_str().unwrap();
        assert!(policy.contains("connect-src 'none'"));
        assert!(policy.contains("object-src 'none'"));
        assert!(policy.contains("frame-ancestors http://localhost:8419"));
        assert!(policy
            .contains("script-src http://localhost:8419/addons-assets/dev.laruche.test/1.2.3/"));
        assert!(!policy.contains("unsafe-eval"));
        assert!(!policy.contains("script-src *"));
    }

    #[test]
    fn html_policy_rejects_a_malformed_host() {
        assert!(html_policy(
            Some(&HeaderValue::from_static("localhost; script-src *")),
            "dev.laruche.test",
            "1.2.3"
        )
        .is_none());
    }

    #[test]
    fn lookup_failures_do_not_disclose_the_reason() {
        for error in [
            AssetLookupError::NotFound,
            AssetLookupError::Disabled,
            AssetLookupError::InvalidPath,
            AssetLookupError::TooLarge,
        ] {
            let response = lookup_error(error);
            assert_eq!(response.status(), StatusCode::NOT_FOUND);
            assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
        }
    }
}
