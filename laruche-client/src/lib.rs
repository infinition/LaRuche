//! # LaRuche Client SDK
//!
//! Discover and query LaRuche AI nodes on your local network.
//! Zero configuration required.
//!
//! ## Quick Start
//!
//! ```rust
//! use laruche_client::LaRuche;
//!
//! #[tokio::main]
//! async fn main() {
//!     let laruche = LaRuche::discover().await.unwrap();
//!     let response = laruche.ask("Bonjour !").await.unwrap();
//!     println!("{}", response.text);
//! }
//! ```
//!
//! ## With capability routing
//!
//! ```rust
//! use laruche_client::{LaRuche, Capability};
//!
//! let laruche = LaRuche::discover().await.unwrap();
//!
//! // Route to a code-specialized model
//! let code = laruche.ask_with(
//!     "Write a Python function to sort a list",
//!     Capability::Code,
//! ).await.unwrap();
//!
//! // Route to an audio model
//! let transcript = laruche.transcribe(audio_bytes).await.unwrap();
//! ```

use land_protocol::capabilities::Capability;
use land_protocol::discovery::{DiscoveredNode, LandListener};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LaRucheError {
    #[error("No LaRuche node found on the network")]
    NoNodeFound,

    #[error("No node with capability '{0}' found")]
    CapabilityNotFound(String),

    #[error("Connection error: {0}")]
    Connection(#[from] reqwest::Error),

    #[error("Discovery error: {0}")]
    Discovery(#[from] land_protocol::error::LandError),

    #[error("API error: {status} - {message}")]
    Api { status: u16, message: String },

    #[error("Timeout waiting for response")]
    Timeout,
}

/// Response from a LaRuche inference request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaRucheResponse {
    /// The generated text
    pub text: String,

    /// Which model produced this response
    pub model: String,

    /// Number of tokens generated
    pub tokens: u32,

    /// Latency in milliseconds
    pub latency_ms: u64,

    /// Which node handled the request
    pub node_name: String,
}

/// Main client for interacting with LaRuche nodes.
///
/// # Zero-config usage
///
/// The LaRuche client automatically discovers AI nodes on your
/// local network via the LAND protocol. No URL, no API key,
/// no configuration needed.
pub struct LaRuche {
    nodes: Vec<DiscoveredNode>,
    http: reqwest::Client,
    _listener: LandListener,
}

// Re-export for convenience
pub use land_protocol::capabilities::Capability as Cap;

impl LaRuche {
    /// Discover LaRuche nodes on the local network.
    ///
    /// Waits up to 3 seconds for nodes to respond.
    /// Returns the client with all discovered nodes.
    pub async fn discover() -> Result<Self, LaRucheError> {
        Self::discover_timeout(Duration::from_secs(3)).await
    }

    /// Discover with a custom timeout.
    pub async fn discover_timeout(timeout: Duration) -> Result<Self, LaRucheError> {
        let mut listener = LandListener::new()?;
        let nodes_map = listener.start()?;

        // Wait for discovery
        tokio::time::sleep(timeout).await;

        let nodes: Vec<DiscoveredNode> = nodes_map.read().await.values().cloned().collect();

        if nodes.is_empty() {
            return Err(LaRucheError::NoNodeFound);
        }

        tracing::info!(count = nodes.len(), "Discovered LaRuche nodes");

        Ok(Self {
            nodes,
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(60))
                .build()
                .unwrap(),
            _listener: listener,
        })
    }

    /// Connect to a specific LaRuche node by URL (skip discovery).
    pub fn connect(url: &str) -> Self {
        use land_protocol::manifest::PartialManifest;

        let manifest = PartialManifest {
            protocol_version: None,
            node_id: None,
            node_name: Some("direct".into()),
            tier: None,
            tokens_per_sec: None,
            memory_usage_pct: None,
            queue_depth: None,
            port: Some(
                url.split(':')
                    .last()
                    .and_then(|p| p.parse().ok())
                    .unwrap_or(land_protocol::DEFAULT_API_PORT),
            ),
            dashboard_port: None,
            capabilities: Vec::new(),
            temperature_c: None,
            in_swarm: false,
            peer_count: 0,
            is_coordinator: false,
            host: url
                .replace("http://", "")
                .replace("https://", "")
                .split(':')
                .next()
                .unwrap_or("127.0.0.1")
                .to_string(),
        };

        Self {
            nodes: vec![DiscoveredNode {
                manifest,
                discovered_at: chrono::Utc::now(),
                last_seen: chrono::Utc::now(),
            }],
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(60))
                .build()
                .unwrap(),
            _listener: LandListener::new().unwrap(),
        }
    }

    /// Ask LaRuche a question (uses best available LLM node).
    ///
    /// ```rust
    /// let response = laruche.ask("Explain quantum computing").await?;
    /// println!("{}", response.text);
    /// ```
    pub async fn ask(&self, prompt: &str) -> Result<LaRucheResponse, LaRucheError> {
        self.ask_with(prompt, Capability::Llm).await
    }

    /// Ask with a specific capability requirement.
    ///
    /// Routes the request to a node that has the requested capability.
    pub async fn ask_with(
        &self,
        prompt: &str,
        capability: Capability,
    ) -> Result<LaRucheResponse, LaRucheError> {
        let node = self.find_best_node(capability)?;
        let url = node
            .manifest
            .api_url()
            .ok_or(LaRucheError::NoNodeFound)?;

        let body = serde_json::json!({
            "prompt": prompt,
            "capability": capability.to_string(),
            "qos": "normal",
        });

        let resp = self
            .http
            .post(format!("{url}/infer"))
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(LaRucheError::Api {
                status: resp.status().as_u16(),
                message: resp.text().await.unwrap_or_default(),
            });
        }

        let data: serde_json::Value = resp.json().await?;

        Ok(LaRucheResponse {
            text: data["response"].as_str().unwrap_or("").to_string(),
            model: data["model"].as_str().unwrap_or("unknown").to_string(),
            tokens: data["tokens_generated"].as_u64().unwrap_or(0) as u32,
            latency_ms: data["latency_ms"].as_u64().unwrap_or(0),
            node_name: data["node_name"].as_str().unwrap_or("unknown").to_string(),
        })
    }

    /// List all discovered nodes.
    pub fn nodes(&self) -> &[DiscoveredNode] {
        &self.nodes
    }

    /// Get the number of available nodes.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Find nodes with a specific capability.
    pub fn find_nodes(&self, capability: Capability) -> Vec<&DiscoveredNode> {
        self.nodes
            .iter()
            .filter(|n| n.manifest.capabilities.contains(&capability))
            .collect()
    }

    /// Find the best node for a capability (lowest queue, highest speed).
    fn find_best_node(&self, capability: Capability) -> Result<&DiscoveredNode, LaRucheError> {
        let candidates: Vec<&DiscoveredNode> = if self.nodes.len() == 1 {
            // Single node: use it regardless of capabilities (POC simplicity)
            vec![&self.nodes[0]]
        } else {
            self.find_nodes(capability)
        };

        if candidates.is_empty() {
            return Err(LaRucheError::CapabilityNotFound(capability.to_string()));
        }

        // Sort by: lowest queue depth, then highest tokens/sec
        let best = candidates
            .into_iter()
            .min_by(|a, b| {
                let qa = a.manifest.queue_depth.unwrap_or(u32::MAX);
                let qb = b.manifest.queue_depth.unwrap_or(u32::MAX);
                qa.cmp(&qb).then_with(|| {
                    let ta = a.manifest.tokens_per_sec.unwrap_or(0.0);
                    let tb = b.manifest.tokens_per_sec.unwrap_or(0.0);
                    tb.partial_cmp(&ta).unwrap_or(std::cmp::Ordering::Equal)
                })
            })
            .unwrap();

        Ok(best)
    }
}
