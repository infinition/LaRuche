//! Error types for the Miel protocol.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum MielError {
    #[error("mDNS error: {0}")]
    Mdns(#[from] mdns_sd::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Authentication failed: {0}")]
    AuthFailed(String),

    #[error("Node not found: {0}")]
    NodeNotFound(String),

    #[error("Swarm error: {0}")]
    Swarm(String),

    #[error("Capacity exceeded: {0}")]
    CapacityExceeded(String),

    #[error("Protocol version mismatch: expected {expected}, got {got}")]
    VersionMismatch { expected: String, got: String },

    #[error("Inference error: {0}")]
    Inference(String),
}
