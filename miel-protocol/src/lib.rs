//! # Miel Protocol (ex-LAND — Local AI Network Discovery)
//!
//! Core library implementing the Miel protocol for automatic discovery
//! and communication between LaRuche nodes on a local network.
//!
//! ## Architecture
//!
//! ```text
//!  ┌──────────────┐     mDNS multicast      ┌──────────────┐
//!  │ LaRuche     │ <----------------------> │ LaRuche     │
//!  │  (LLM+RAG)   │    _ai-inference._tcp    │  (VLM+Audio) │
//!  └──────┬───────┘                          └──────┬───────┘
//!         │              Miel Protocol              │
//!         ▼                                         ▼
//!  ┌──────────────┐                          ┌──────────────┐
//!  │   Manifest    │                          │   Manifest    │
//!  │  Cognitif     │                          │  Cognitif     │
//!  └──────────────┘                          └──────────────┘
//! ```

pub mod capabilities;
pub mod discovery;
pub mod manifest;
pub mod auth;
pub mod qos;
pub mod swarm;
pub mod error;

pub use capabilities::Capability;
pub use discovery::{MielBroadcaster, MielListener};
pub use manifest::CognitiveManifest;
pub use auth::{ProximityAuth, TrustCircle, AuthToken};
pub use qos::{QosLevel, QosPolicy};
pub use swarm::{SwarmState, DistributedInferenceState, ShardingSummary};
pub use error::MielError;

/// Miel protocol version
pub const PROTOCOL_VERSION: &str = "0.2.0";

/// mDNS service type (kept compatible with LAND v0.1)
pub const SERVICE_TYPE: &str = "_ai-inference._tcp.local.";

/// Default multicast interval in seconds
pub const BROADCAST_INTERVAL_SECS: u64 = 2;

/// Default API port for inference
pub const DEFAULT_API_PORT: u16 = 8419;

/// Default dashboard port
pub const DEFAULT_DASHBOARD_PORT: u16 = 8420;

/// Format host for URL usage (adds brackets for raw IPv6 literals).
pub fn format_host_for_url(host: &str) -> String {
    if host.contains(':') && !host.starts_with('[') && !host.ends_with(']') {
        format!("[{host}]")
    } else {
        host.to_string()
    }
}

/// Build an HTTP(S) endpoint URL from host/port.
pub fn endpoint_url(host: &str, port: u16, tls: bool) -> String {
    let scheme = if tls { "https" } else { "http" };
    format!("{scheme}://{}:{port}", format_host_for_url(host))
}

/// Get the local IP address of the machine.
pub fn get_local_ip() -> String {
    local_ip_address::local_ip()
        .map(|ip| ip.to_string())
        .unwrap_or_else(|_| "127.0.0.1".to_string())
}
