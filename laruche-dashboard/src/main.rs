//! LaRuche Dashboard
//!
//! Web-based monitoring and management interface for LaRuche nodes.
//! Serves a single-page application with real-time node status,
//! cybersecurity monitoring, and swarm visualization.

use anyhow::Result;
use axum::{response::Html, routing::get, Router};
use tracing::info;

const DASHBOARD_HTML: &str = include_str!("templates/dashboard.html");

async fn dashboard() -> Html<&'static str> {
    Html(DASHBOARD_HTML)
}

async fn health() -> &'static str {
    "OK"
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("laruche_dashboard=info")
        .init();

    let port: u16 = std::env::var("LARUCHE_DASH_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(land_protocol::DEFAULT_DASHBOARD_PORT);

    let app = Router::new()
        .route("/", get(dashboard))
        .route("/health", get(health))
        .layer(tower_http::cors::CorsLayer::permissive());

    info!(port, "LaRuche Dashboard starting");
    info!("Open http://localhost:{port} in your browser");

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
