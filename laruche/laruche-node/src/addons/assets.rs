use super::AssetLookupError;
use crate::AppState;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{header, HeaderValue, Response, StatusCode};
use std::sync::Arc;

const HTML_CSP: &str = "default-src 'none'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; font-src 'self'; connect-src 'none'; object-src 'none'; base-uri 'none'; form-action 'none'; frame-ancestors 'self'";

pub(crate) async fn serve(
    State(state): State<Arc<AppState>>,
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
        HeaderValue::from_static("same-origin"),
    );
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    );
    if is_html {
        headers.insert(
            header::CONTENT_SECURITY_POLICY,
            HeaderValue::from_static(HTML_CSP),
        );
    }
    response
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
        assert!(HTML_CSP.contains("connect-src 'none'"));
        assert!(HTML_CSP.contains("object-src 'none'"));
        assert!(HTML_CSP.contains("frame-ancestors 'self'"));
        assert!(!HTML_CSP.contains("unsafe-eval"));
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
