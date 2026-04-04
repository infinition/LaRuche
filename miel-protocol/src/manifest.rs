//! Cognitive Manifest - the core data structure broadcast by every LaRuche node.
//!
//! The manifest is compressed and embedded in the mDNS multicast packet.
//! It describes everything a client needs to know about this node's
//! capabilities, load, and availability.

use crate::capabilities::CapabilitySet;
use crate::qos::QosLevel;
use chrono::{DateTime, Utc};
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use uuid::Uuid;

/// The Cognitive Manifest broadcast by each LaRuche node via Miel.
///
/// Contains everything a client or peer node needs to route requests
/// intelligently: capabilities, load, health, and identity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveManifest {
    /// Unique node identifier (persistent across reboots)
    pub node_id: Uuid,

    /// Human-friendly node name (e.g., "laruche-salon", "laruche-bureau-3")
    pub node_name: String,

    /// Miel protocol version for compatibility checking
    pub protocol_version: String,

    /// Hardware tier of this node
    pub hardware_tier: HardwareTier,

    /// All AI capabilities available on this node
    pub capabilities: CapabilitySet,

    /// Current resource usage
    pub resources: ResourceStatus,

    /// Inference performance metrics
    pub performance: PerformanceMetrics,

    /// Network and swarm information
    pub swarm_info: SwarmInfo,

    /// API endpoint for inference requests
    pub api_endpoint: ApiEndpoint,

    /// Timestamp of this manifest generation
    pub timestamp: DateTime<Utc>,

    /// Node uptime in seconds
    pub uptime_secs: u64,
}

/// Hardware tier of the LaRuche node.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum HardwareTier {
    /// ESP32 / Coral - micro-models only (~30€)
    Nano,
    /// RK3588 / Orange Pi - consumer LLM (~100€)
    Core,
    /// Jetson Orin / Mac Mini - professional (~400-800€)
    Pro,
    /// Multi-GPU rack - enterprise (~2000€+)
    Max,
}

impl HardwareTier {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Nano => "LaRuche Nano",
            Self::Core => "LaRuche Core",
            Self::Pro => "LaRuche Pro",
            Self::Max => "LaRuche Max",
        }
    }
}

/// Current resource usage of the node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceStatus {
    /// CPU usage percentage (0-100)
    pub cpu_usage_pct: f32,

    /// Memory used in MB
    pub memory_used_mb: u64,

    /// Total memory in MB
    pub memory_total_mb: u64,

    /// GPU/NPU usage percentage (0-100), if available
    pub accelerator_usage_pct: Option<f32>,

    /// VRAM used in MB, if available
    pub vram_used_mb: Option<u64>,

    /// VRAM total in MB, if available
    pub vram_total_mb: Option<u64>,

    /// Temperature in Celsius
    pub temperature_c: Option<f32>,

    /// Disk usage percentage
    pub disk_usage_pct: Option<f32>,
}

/// Inference performance metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Current inference speed in tokens/sec
    pub tokens_per_sec: f32,

    /// Average request latency in ms
    pub avg_latency_ms: f32,

    /// Number of requests currently in queue
    pub queue_depth: u32,

    /// Maximum QoS level this node accepts right now
    pub accepting_qos: QosLevel,
}

/// Swarm membership information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmInfo {
    /// Whether this node is part of a swarm
    pub in_swarm: bool,

    /// Swarm cluster ID (shared by all nodes in the swarm)
    pub swarm_id: Option<Uuid>,

    /// Number of peers in the swarm
    pub peer_count: u32,

    /// Combined VRAM across swarm in MB
    pub swarm_vram_total_mb: Option<u64>,

    /// Whether this node is the swarm coordinator
    pub is_coordinator: bool,
}

/// API endpoint information for clients.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiEndpoint {
    /// IP address (IPv4 or IPv6)
    pub host: String,

    /// Port for the inference API
    pub port: u16,

    /// Port for the web dashboard
    pub dashboard_port: u16,

    /// Whether TLS is enabled
    pub tls: bool,
}

impl CognitiveManifest {
    /// Create a new manifest with default values for a given tier.
    pub fn new(node_name: String, tier: HardwareTier) -> Self {
        Self {
            node_id: Uuid::new_v4(),
            node_name,
            protocol_version: crate::PROTOCOL_VERSION.to_string(),
            hardware_tier: tier,
            capabilities: CapabilitySet::new(),
            resources: ResourceStatus {
                cpu_usage_pct: 0.0,
                memory_used_mb: 0,
                memory_total_mb: match tier {
                    HardwareTier::Nano => 512,
                    HardwareTier::Core => 8192,
                    HardwareTier::Pro => 16384,
                    HardwareTier::Max => 65536,
                },
                accelerator_usage_pct: None,
                vram_used_mb: None,
                vram_total_mb: None,
                temperature_c: None,
                disk_usage_pct: None,
            },
            performance: PerformanceMetrics {
                tokens_per_sec: 0.0,
                avg_latency_ms: 0.0,
                queue_depth: 0,
                accepting_qos: QosLevel::Low,
            },
            swarm_info: SwarmInfo {
                in_swarm: false,
                swarm_id: None,
                peer_count: 0,
                swarm_vram_total_mb: None,
                is_coordinator: false,
            },
            api_endpoint: ApiEndpoint {
                host: "0.0.0.0".to_string(),
                port: crate::DEFAULT_API_PORT,
                dashboard_port: crate::DEFAULT_DASHBOARD_PORT,
                tls: false,
            },
            timestamp: Utc::now(),
            uptime_secs: 0,
        }
    }

    /// Serialize and compress the manifest for mDNS broadcast.
    pub fn to_compressed(&self) -> Result<Vec<u8>, crate::error::MielError> {
        let json = serde_json::to_vec(self)?;
        let mut encoder = GzEncoder::new(Vec::new(), Compression::fast());
        encoder.write_all(&json)?;
        let compressed = encoder.finish()?;
        Ok(compressed)
    }

    /// Decompress and deserialize a manifest from mDNS data.
    pub fn from_compressed(data: &[u8]) -> Result<Self, crate::error::MielError> {
        let mut decoder = GzDecoder::new(data);
        let mut json = Vec::new();
        decoder.read_to_end(&mut json)?;
        let manifest: Self = serde_json::from_slice(&json)?;
        Ok(manifest)
    }

    /// Convert to key-value pairs for mDNS TXT record.
    /// mDNS TXT records have a 255-byte limit per key-value pair,
    /// so we split the manifest into essential fields.
    pub fn to_txt_properties(&self) -> Vec<(String, String)> {
        let mut props = vec![
            ("land_v".to_string(), self.protocol_version.clone()),
            ("node_id".to_string(), self.node_id.to_string()),
            ("name".to_string(), self.node_name.clone()),
            ("tier".to_string(), format!("{:?}", self.hardware_tier).to_lowercase()),
            ("tps".to_string(), format!("{:.1}", self.performance.tokens_per_sec)),
            ("mem_pct".to_string(), format!("{:.0}", self.memory_usage_pct())),
            ("queue".to_string(), self.performance.queue_depth.to_string()),
            ("port".to_string(), self.api_endpoint.port.to_string()),
            ("dash_port".to_string(), self.api_endpoint.dashboard_port.to_string()),
        ];

        // Add model info (primary model name)
        if let Some(cap) = self.capabilities.capabilities.first() {
            props.push(("model".to_string(), cap.model_name.clone()));
        }

        // Add capability flags
        for flag in self.capabilities.to_flags() {
            props.push((flag, "1".to_string()));
        }

        // Add temperature if available
        if let Some(temp) = self.resources.temperature_c {
            props.push(("temp_c".to_string(), format!("{:.1}", temp)));
        }

        // Add swarm info
        if self.swarm_info.in_swarm {
            props.push(("swarm".to_string(), "1".to_string()));
            props.push(("peers".to_string(), self.swarm_info.peer_count.to_string()));
            if self.swarm_info.is_coordinator {
                props.push(("coordinator".to_string(), "1".to_string()));
            }
        }

        props
    }

    /// Reconstruct essential manifest info from mDNS TXT record.
    pub fn from_txt_properties(props: &[(String, String)], host: &str) -> Option<PartialManifest> {
        let mut manifest = PartialManifest {
            protocol_version: None,
            node_id: None,
            node_name: None,
            tier: None,
            tokens_per_sec: None,
            memory_usage_pct: None,
            queue_depth: None,
            port: None,
            dashboard_port: None,
            capabilities: Vec::new(),
            temperature_c: None,
            in_swarm: false,
            peer_count: 0,
            is_coordinator: false,
            model: None,
            host: host.to_string(),
        };

        for (key, value) in props {
            match key.as_str() {
                "land_v" => manifest.protocol_version = Some(value.clone()),
                "node_id" => manifest.node_id = Uuid::parse_str(value).ok(),
                "name" => manifest.node_name = Some(value.clone()),
                "tier" => manifest.tier = Some(value.clone()),
                "tps" => manifest.tokens_per_sec = value.parse().ok(),
                "mem_pct" => manifest.memory_usage_pct = value.parse().ok(),
                "queue" => manifest.queue_depth = value.parse().ok(),
                "port" => manifest.port = value.parse().ok(),
                "dash_port" => manifest.dashboard_port = value.parse().ok(),
                "model" => manifest.model = Some(value.clone()),
                "temp_c" => manifest.temperature_c = value.parse().ok(),
                "swarm" => manifest.in_swarm = value == "1",
                "peers" => manifest.peer_count = value.parse().unwrap_or(0),
                "coordinator" => manifest.is_coordinator = value == "1",
                key if key.starts_with("capability:") => {
                    if let Some(cap) = crate::capabilities::Capability::from_flag(
                        key.strip_prefix("capability:").unwrap_or("")
                    ) {
                        manifest.capabilities.push(cap);
                    }
                }
                _ => {}
            }
        }

        // Require at minimum: node_id and port
        if manifest.node_id.is_some() && manifest.port.is_some() {
            Some(manifest)
        } else {
            None
        }
    }

    fn memory_usage_pct(&self) -> f32 {
        if self.resources.memory_total_mb > 0 {
            (self.resources.memory_used_mb as f32 / self.resources.memory_total_mb as f32) * 100.0
        } else {
            0.0
        }
    }
}

/// Partial manifest reconstructed from mDNS TXT properties.
/// Used by clients that discover nodes via mDNS.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartialManifest {
    pub protocol_version: Option<String>,
    pub node_id: Option<Uuid>,
    pub node_name: Option<String>,
    pub tier: Option<String>,
    pub tokens_per_sec: Option<f32>,
    pub memory_usage_pct: Option<f32>,
    pub queue_depth: Option<u32>,
    pub port: Option<u16>,
    pub dashboard_port: Option<u16>,
    pub capabilities: Vec<crate::capabilities::Capability>,
    pub temperature_c: Option<f32>,
    pub in_swarm: bool,
    pub peer_count: u32,
    pub is_coordinator: bool,
    pub model: Option<String>,
    pub host: String,
}

impl Default for PartialManifest {
    fn default() -> Self {
        Self {
            protocol_version: None,
            node_id: None,
            node_name: None,
            tier: None,
            tokens_per_sec: None,
            memory_usage_pct: None,
            queue_depth: None,
            port: None,
            dashboard_port: None,
            capabilities: Vec::new(),
            temperature_c: None,
            in_swarm: false,
            peer_count: 0,
            is_coordinator: false,
            model: None,
            host: String::new(),
        }
    }
}

impl PartialManifest {
    /// Get the full API URL for this node.
    pub fn api_url(&self) -> Option<String> {
        self.port.map(|p| crate::endpoint_url(&self.host, p, false))
    }

    /// Get the dashboard URL for this node.
    pub fn dashboard_url(&self) -> Option<String> {
        self.dashboard_port
            .map(|p| crate::endpoint_url(&self.host, p, false))
    }

    /// Check if this manifest's protocol version is compatible.
    pub fn is_version_compatible(&self) -> bool {
        match &self.protocol_version {
            Some(v) => {
                let our_major = crate::PROTOCOL_VERSION.split('.').next().unwrap_or("0");
                let their_major = v.split('.').next().unwrap_or("0");
                our_major == their_major
            }
            None => true, // No version info = assume compatible
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capabilities::{Capability, CapabilityInfo};

    #[test]
    fn test_manifest_compression_roundtrip() {
        let mut manifest = CognitiveManifest::new("test-node".into(), HardwareTier::Core);
        manifest.capabilities.add(CapabilityInfo {
            capability: Capability::Llm,
            model_name: "mistral-7b".into(),
            model_size: Some("7B".into()),
            quantization: Some("Q4_K_M".into()),
            max_context_length: Some(8192),
        });

        let compressed = manifest.to_compressed().unwrap();
        let decompressed = CognitiveManifest::from_compressed(&compressed).unwrap();

        assert_eq!(manifest.node_id, decompressed.node_id);
        assert_eq!(manifest.node_name, decompressed.node_name);
        assert!(decompressed.capabilities.has(Capability::Llm));
    }

    #[test]
    fn test_txt_properties_roundtrip() {
        let mut manifest = CognitiveManifest::new("laruche-salon".into(), HardwareTier::Core);
        manifest.capabilities.add(CapabilityInfo {
            capability: Capability::Llm,
            model_name: "mistral-7b".into(),
            model_size: None,
            quantization: None,
            max_context_length: None,
        });
        manifest.capabilities.add(CapabilityInfo {
            capability: Capability::Rag,
            model_name: "bge-small".into(),
            model_size: None,
            quantization: None,
            max_context_length: None,
        });
        manifest.performance.tokens_per_sec = 15.3;

        let props = manifest.to_txt_properties();
        let partial = CognitiveManifest::from_txt_properties(&props, "192.168.1.42").unwrap();

        assert_eq!(partial.node_id, Some(manifest.node_id));
        assert_eq!(partial.node_name.as_deref(), Some("laruche-salon"));
        assert_eq!(partial.model.as_deref(), Some("mistral-7b"));
        assert!(partial.capabilities.contains(&Capability::Llm));
        assert!(partial.capabilities.contains(&Capability::Rag));
    }
}
