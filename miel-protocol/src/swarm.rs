//! Swarm Intelligence - automatic mesh and tensor sharding between LaRuche nodes.
//!
//! When multiple LaRuche nodes are on the same network, they can
//! automatically pool their VRAM and distribute model layers
//! across nodes (tensor sharding over Ethernet).

use crate::manifest::PartialManifest;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Describes the state of a LaRuche swarm.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmState {
    /// Unique swarm identifier
    pub swarm_id: Uuid,

    /// All peer nodes in the swarm
    pub peers: HashMap<Uuid, SwarmPeer>,

    /// The coordinator node ID
    pub coordinator_id: Uuid,

    /// Combined VRAM across all peers in MB
    pub total_vram_mb: u64,

    /// Combined RAM across all peers in MB
    pub total_ram_mb: u64,

    /// Current sharding configuration
    pub sharding: Option<ShardingConfig>,

    /// Swarm creation time
    pub formed_at: DateTime<Utc>,
}

/// A peer node in the swarm.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmPeer {
    pub node_id: Uuid,
    pub node_name: String,
    pub host: String,
    pub port: u16,
    pub vram_mb: u64,
    pub ram_mb: u64,
    pub last_heartbeat: DateTime<Utc>,
    pub status: PeerStatus,
    /// Which layers this peer is responsible for
    pub assigned_layers: Option<LayerRange>,
}

/// Health status of a swarm peer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PeerStatus {
    /// Online and healthy
    Active,
    /// Responding but under heavy load
    Busy,
    /// Missed recent heartbeats
    Suspect,
    /// Confirmed offline
    Down,
    /// Syncing model data
    Syncing,
}

/// Configuration for tensor sharding across the swarm.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardingConfig {
    /// Model being sharded
    pub model_name: String,

    /// Total model size in MB
    pub model_size_mb: u64,

    /// Total number of layers in the model
    pub total_layers: u32,

    /// Layer assignment per peer
    pub assignments: HashMap<Uuid, LayerRange>,

    /// Whether a redundancy mirror is active
    pub mirror_enabled: bool,
}

/// A range of model layers assigned to a peer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerRange {
    pub start: u32,
    pub end: u32,
}

impl LayerRange {
    pub fn count(&self) -> u32 {
        self.end - self.start + 1
    }
}

impl SwarmState {
    /// Create a new swarm with this node as coordinator.
    pub fn new_as_coordinator(node_id: Uuid, node_name: String, host: String, port: u16, vram_mb: u64, ram_mb: u64) -> Self {
        let mut peers = HashMap::new();
        peers.insert(node_id, SwarmPeer {
            node_id,
            node_name,
            host,
            port,
            vram_mb,
            ram_mb,
            last_heartbeat: Utc::now(),
            status: PeerStatus::Active,
            assigned_layers: None,
        });

        Self {
            swarm_id: Uuid::new_v4(),
            peers,
            coordinator_id: node_id,
            total_vram_mb: vram_mb,
            total_ram_mb: ram_mb,
            sharding: None,
            formed_at: Utc::now(),
        }
    }

    /// Add a peer to the swarm.
    pub fn add_peer(&mut self, manifest: &PartialManifest) -> Option<Uuid> {
        let node_id = manifest.node_id?;
        let peer = SwarmPeer {
            node_id,
            node_name: manifest.node_name.clone().unwrap_or_default(),
            host: manifest.host.clone(),
            port: manifest.port.unwrap_or(crate::DEFAULT_API_PORT),
            vram_mb: 0, // Will be updated from full manifest
            ram_mb: 0,
            last_heartbeat: Utc::now(),
            status: PeerStatus::Syncing,
            assigned_layers: None,
        };

        self.peers.insert(node_id, peer);
        self.recalculate_totals();
        Some(node_id)
    }

    /// Update a peer's resource info (called when full manifest data arrives).
    pub fn update_peer_resources(&mut self, node_id: &Uuid, vram_mb: u64, ram_mb: u64) {
        if let Some(peer) = self.peers.get_mut(node_id) {
            peer.vram_mb = vram_mb;
            peer.ram_mb = ram_mb;
            self.recalculate_totals();
        }
    }

    /// Update a peer's status.
    pub fn update_peer_status(&mut self, node_id: &Uuid, status: PeerStatus) {
        if let Some(peer) = self.peers.get_mut(node_id) {
            peer.status = status;
        }
    }

    /// Remove a peer from the swarm.
    pub fn remove_peer(&mut self, node_id: &Uuid) -> bool {
        let removed = self.peers.remove(node_id).is_some();
        if removed {
            self.recalculate_totals();
        }
        removed
    }

    /// Process a heartbeat from a peer.
    pub fn heartbeat(&mut self, node_id: &Uuid) -> bool {
        if let Some(peer) = self.peers.get_mut(node_id) {
            peer.last_heartbeat = Utc::now();
            if matches!(
                peer.status,
                PeerStatus::Suspect | PeerStatus::Syncing | PeerStatus::Down
            ) {
                peer.status = PeerStatus::Active;
            }
            true
        } else {
            false
        }
    }

    /// Check for stale peers (missed heartbeats).
    /// Returns list of peers marked as suspect or down.
    pub fn check_health(&mut self, timeout_secs: i64) -> Vec<Uuid> {
        let now = Utc::now();
        let mut unhealthy = Vec::new();

        for (id, peer) in &mut self.peers {
            let elapsed = (now - peer.last_heartbeat).num_seconds();
            if elapsed > timeout_secs * 3 && peer.status != PeerStatus::Down {
                peer.status = PeerStatus::Down;
                unhealthy.push(*id);
            } else if elapsed > timeout_secs
                && matches!(
                    peer.status,
                    PeerStatus::Active | PeerStatus::Busy | PeerStatus::Syncing
                )
            {
                peer.status = PeerStatus::Suspect;
                unhealthy.push(*id);
            }
        }

        unhealthy
    }

    /// Calculate a sharding configuration for a model across available peers.
    pub fn plan_sharding(&self, model_name: &str, total_layers: u32, model_size_mb: u64) -> ShardingConfig {
        let active_peers: Vec<&SwarmPeer> = self.peers.values()
            .filter(|p| matches!(p.status, PeerStatus::Active | PeerStatus::Busy))
            .collect();

        let peer_count = active_peers.len() as u32;
        let layers_per_peer = total_layers / peer_count.max(1);
        let mut assignments = HashMap::new();
        let mut current_layer = 0;

        for (i, peer) in active_peers.iter().enumerate() {
            let end_layer = if i as u32 == peer_count - 1 {
                total_layers - 1 // Last peer gets remaining layers
            } else {
                current_layer + layers_per_peer - 1
            };

            assignments.insert(peer.node_id, LayerRange {
                start: current_layer,
                end: end_layer,
            });

            current_layer = end_layer + 1;
        }

        ShardingConfig {
            model_name: model_name.to_string(),
            model_size_mb,
            total_layers,
            assignments,
            mirror_enabled: peer_count >= 3, // Enable redundancy with 3+ nodes
        }
    }

    /// Get active peer count.
    pub fn active_peer_count(&self) -> usize {
        self.peers.values()
            .filter(|p| matches!(p.status, PeerStatus::Active | PeerStatus::Busy))
            .count()
    }

    /// Estimate the combined throughput speedup when tensor sharding is active.
    /// With N peers doing pipeline-parallel inference, throughput scales sub-linearly
    /// due to inter-node communication overhead.
    pub fn estimated_speedup(&self) -> f32 {
        let n = self.active_peer_count() as f32;
        if n <= 1.0 {
            return 1.0;
        }
        // Pipeline parallelism efficiency: ~85% per additional node
        // (accounts for network latency between layer hand-offs)
        let efficiency = 0.85_f32;
        1.0 + (n - 1.0) * efficiency
    }

    /// Check if the swarm has enough active peers with VRAM to shard a model.
    pub fn can_shard(&self, model_size_mb: u64) -> bool {
        let active_vram: u64 = self.peers.values()
            .filter(|p| matches!(p.status, PeerStatus::Active | PeerStatus::Busy))
            .map(|p| p.vram_mb)
            .sum();
        active_vram >= model_size_mb && self.active_peer_count() >= 2
    }

    /// Get a list of active peers sorted by VRAM (descending) for sharding assignment.
    pub fn peers_by_vram(&self) -> Vec<&SwarmPeer> {
        let mut peers: Vec<&SwarmPeer> = self.peers.values()
            .filter(|p| matches!(p.status, PeerStatus::Active | PeerStatus::Busy))
            .collect();
        peers.sort_by(|a, b| b.vram_mb.cmp(&a.vram_mb));
        peers
    }

    fn recalculate_totals(&mut self) {
        self.total_vram_mb = self.peers.values().map(|p| p.vram_mb).sum();
        self.total_ram_mb = self.peers.values().map(|p| p.ram_mb).sum();
    }
}

// ======================== Distributed Inference Protocol ========================

/// A request to perform distributed (sharded) inference across swarm peers.
/// The coordinator splits the work and aggregates results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributedInferenceRequest {
    /// Unique request identifier
    pub request_id: Uuid,

    /// The prompt/input to process
    pub prompt: String,

    /// Model to use (must be sharded across the swarm)
    pub model_name: String,

    /// Maximum tokens to generate
    pub max_tokens: u32,

    /// Temperature for sampling
    pub temperature: f32,

    /// Which sharding config to use
    pub sharding_config_id: Option<Uuid>,
}

/// Status of a distributed inference pipeline stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PipelineStageStatus {
    /// Waiting for input from previous stage
    Waiting,
    /// Currently processing
    Processing,
    /// Completed, output forwarded to next stage
    Completed,
    /// Failed
    Failed,
}

/// Tracks the state of a distributed inference across the swarm.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributedInferenceState {
    /// The original request
    pub request_id: Uuid,

    /// Pipeline stages (one per peer in the shard)
    pub stages: Vec<PipelineStage>,

    /// Overall start time
    pub started_at: DateTime<Utc>,

    /// Overall completion time
    pub completed_at: Option<DateTime<Utc>>,

    /// Tokens generated so far
    pub tokens_generated: u32,

    /// Whether the pipeline is fully complete
    pub is_complete: bool,
}

/// A single stage in the distributed inference pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineStage {
    /// Which peer handles this stage
    pub peer_id: Uuid,

    /// Layer range this peer processes
    pub layers: LayerRange,

    /// Current status
    pub status: PipelineStageStatus,

    /// Processing time for this stage in ms
    pub processing_ms: Option<u64>,
}

impl DistributedInferenceState {
    /// Create a new distributed inference state from a sharding config.
    pub fn new(request_id: Uuid, config: &ShardingConfig) -> Self {
        let stages: Vec<PipelineStage> = config
            .assignments
            .iter()
            .map(|(peer_id, layers)| PipelineStage {
                peer_id: *peer_id,
                layers: layers.clone(),
                status: PipelineStageStatus::Waiting,
                processing_ms: None,
            })
            .collect();

        Self {
            request_id,
            stages,
            started_at: Utc::now(),
            completed_at: None,
            tokens_generated: 0,
            is_complete: false,
        }
    }

    /// Mark a stage as processing.
    pub fn stage_processing(&mut self, peer_id: &Uuid) {
        if let Some(stage) = self.stages.iter_mut().find(|s| s.peer_id == *peer_id) {
            stage.status = PipelineStageStatus::Processing;
        }
    }

    /// Mark a stage as completed with timing info.
    pub fn stage_completed(&mut self, peer_id: &Uuid, processing_ms: u64) {
        if let Some(stage) = self.stages.iter_mut().find(|s| s.peer_id == *peer_id) {
            stage.status = PipelineStageStatus::Completed;
            stage.processing_ms = Some(processing_ms);
        }

        // Check if all stages are complete
        if self.stages.iter().all(|s| s.status == PipelineStageStatus::Completed) {
            self.is_complete = true;
            self.completed_at = Some(Utc::now());
        }
    }

    /// Mark a stage as failed.
    pub fn stage_failed(&mut self, peer_id: &Uuid) {
        if let Some(stage) = self.stages.iter_mut().find(|s| s.peer_id == *peer_id) {
            stage.status = PipelineStageStatus::Failed;
        }
    }

    /// Get total elapsed time in ms.
    pub fn elapsed_ms(&self) -> u64 {
        let end = self.completed_at.unwrap_or_else(Utc::now);
        (end - self.started_at).num_milliseconds().max(0) as u64
    }

    /// Get the bottleneck stage (slowest processing time).
    pub fn bottleneck(&self) -> Option<&PipelineStage> {
        self.stages
            .iter()
            .filter(|s| s.processing_ms.is_some())
            .max_by_key(|s| s.processing_ms.unwrap_or(0))
    }

    /// Calculate effective tokens/sec across the pipeline.
    pub fn effective_tps(&self) -> f32 {
        let elapsed_secs = self.elapsed_ms() as f32 / 1000.0;
        if elapsed_secs > 0.0 {
            self.tokens_generated as f32 / elapsed_secs
        } else {
            0.0
        }
    }
}

impl SwarmState {
    /// Activate sharding: assign layers to peers based on VRAM-proportional distribution.
    /// Returns the sharding config if successful.
    pub fn activate_sharding(
        &mut self,
        model_name: &str,
        total_layers: u32,
        model_size_mb: u64,
    ) -> Option<ShardingConfig> {
        if !self.can_shard(model_size_mb) {
            return None;
        }

        let config = self.plan_sharding_by_vram(model_name, total_layers, model_size_mb);

        // Assign layers to peers
        for (peer_id, range) in &config.assignments {
            if let Some(peer) = self.peers.get_mut(peer_id) {
                peer.assigned_layers = Some(range.clone());
            }
        }

        self.sharding = Some(config.clone());
        Some(config)
    }

    /// Plan sharding with VRAM-proportional layer assignment.
    /// Peers with more VRAM get more layers.
    pub fn plan_sharding_by_vram(
        &self,
        model_name: &str,
        total_layers: u32,
        model_size_mb: u64,
    ) -> ShardingConfig {
        let active_peers = self.peers_by_vram();
        let total_vram: u64 = active_peers.iter().map(|p| p.vram_mb).sum();

        let mut assignments = HashMap::new();
        let mut current_layer = 0;

        for (i, peer) in active_peers.iter().enumerate() {
            let is_last = i == active_peers.len() - 1;
            let layer_count = if is_last {
                // Last peer gets remaining layers
                total_layers - current_layer
            } else if total_vram > 0 {
                // Proportional to VRAM
                ((peer.vram_mb as f64 / total_vram as f64) * total_layers as f64).round() as u32
            } else {
                // Equal distribution fallback
                total_layers / active_peers.len() as u32
            };

            if layer_count > 0 {
                assignments.insert(peer.node_id, LayerRange {
                    start: current_layer,
                    end: current_layer + layer_count - 1,
                });
                current_layer += layer_count;
            }
        }

        let peer_count = active_peers.len() as u32;
        ShardingConfig {
            model_name: model_name.to_string(),
            model_size_mb,
            total_layers,
            assignments,
            mirror_enabled: peer_count >= 3,
        }
    }

    /// Deactivate sharding and clear layer assignments.
    pub fn deactivate_sharding(&mut self) {
        self.sharding = None;
        for peer in self.peers.values_mut() {
            peer.assigned_layers = None;
        }
    }

    /// Check if sharding is currently active.
    pub fn is_sharding_active(&self) -> bool {
        self.sharding.is_some()
    }

    /// Get a summary of the current sharding state for API responses.
    pub fn sharding_summary(&self) -> Option<ShardingSummary> {
        let config = self.sharding.as_ref()?;
        Some(ShardingSummary {
            model_name: config.model_name.clone(),
            total_layers: config.total_layers,
            model_size_mb: config.model_size_mb,
            peer_count: config.assignments.len() as u32,
            estimated_speedup: self.estimated_speedup(),
            mirror_enabled: config.mirror_enabled,
            peer_assignments: config
                .assignments
                .iter()
                .filter_map(|(id, range)| {
                    self.peers.get(id).map(|p| PeerAssignment {
                        node_id: *id,
                        node_name: p.node_name.clone(),
                        layers: range.clone(),
                        vram_mb: p.vram_mb,
                    })
                })
                .collect(),
        })
    }
}

/// Summary of sharding state for API responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardingSummary {
    pub model_name: String,
    pub total_layers: u32,
    pub model_size_mb: u64,
    pub peer_count: u32,
    pub estimated_speedup: f32,
    pub mirror_enabled: bool,
    pub peer_assignments: Vec<PeerAssignment>,
}

/// A peer's layer assignment for API responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerAssignment {
    pub node_id: Uuid,
    pub node_name: String,
    pub layers: LayerRange,
    pub vram_mb: u64,
}

/// Resilience configuration for the LaRuche Resilience system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResilienceConfig {
    /// Enable automatic model mirroring (RAID-1 for AI)
    pub mirror_enabled: bool,

    /// Heartbeat interval in seconds
    pub heartbeat_interval_secs: u64,

    /// Time before marking a peer as suspect
    pub suspect_timeout_secs: u64,

    /// Time before marking a peer as down
    pub down_timeout_secs: u64,

    /// Maximum failover time in ms
    pub max_failover_ms: u64,

    /// Enable distributed checkpointing for long tasks
    pub checkpoint_enabled: bool,

    /// Checkpoint interval in seconds
    pub checkpoint_interval_secs: u64,
}

impl Default for ResilienceConfig {
    fn default() -> Self {
        Self {
            mirror_enabled: true,
            heartbeat_interval_secs: 1,
            suspect_timeout_secs: 3,
            down_timeout_secs: 10,
            max_failover_ms: 500,
            checkpoint_enabled: true,
            checkpoint_interval_secs: 30,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_swarm_formation() {
        let node_a = Uuid::new_v4();
        let mut swarm = SwarmState::new_as_coordinator(
            node_a, "laruche-a".into(), "192.168.1.10".into(), 8419, 8192, 16384,
        );

        assert_eq!(swarm.active_peer_count(), 1);
        assert_eq!(swarm.coordinator_id, node_a);

        // Add peer
        let manifest = PartialManifest {
            node_id: Some(Uuid::new_v4()),
            node_name: Some("laruche-b".into()),
            host: "192.168.1.11".into(),
            port: Some(8419),
            ..PartialManifest::default()
        };
        swarm.add_peer(&manifest);

        assert_eq!(swarm.peers.len(), 2);
    }

    #[test]
    fn test_sharding_plan() {
        let node_a = Uuid::new_v4();
        let mut swarm = SwarmState::new_as_coordinator(
            node_a, "laruche-a".into(), "192.168.1.10".into(), 8419, 4096, 8192,
        );

        let node_b = Uuid::new_v4();
        swarm.peers.insert(node_b, SwarmPeer {
            node_id: node_b,
            node_name: "laruche-b".into(),
            host: "192.168.1.11".into(),
            port: 8419,
            vram_mb: 4096,
            ram_mb: 8192,
            last_heartbeat: Utc::now(),
            status: PeerStatus::Active,
            assigned_layers: None,
        });

        let config = swarm.plan_sharding("llama-70b", 80, 40000);
        assert_eq!(config.assignments.len(), 2);

        // Total layers should cover all 80
        let total: u32 = config.assignments.values()
            .map(|r| r.count())
            .sum();
        assert_eq!(total, 80);
    }

    #[test]
    fn test_syncing_becomes_active_on_heartbeat() {
        let node_a = Uuid::new_v4();
        let mut swarm = SwarmState::new_as_coordinator(
            node_a, "laruche-a".into(), "192.168.1.10".into(), 8419, 8192, 16384,
        );
        let peer_id = Uuid::new_v4();
        let manifest = PartialManifest {
            node_id: Some(peer_id),
            node_name: Some("laruche-b".into()),
            host: "192.168.1.11".into(),
            port: Some(8419),
            ..PartialManifest::default()
        };
        swarm.add_peer(&manifest);
        assert_eq!(swarm.peers.get(&peer_id).unwrap().status, PeerStatus::Syncing);

        assert!(swarm.heartbeat(&peer_id));
        assert_eq!(swarm.peers.get(&peer_id).unwrap().status, PeerStatus::Active);
    }

    #[test]
    fn test_vram_proportional_sharding() {
        let node_a = Uuid::new_v4();
        let mut swarm = SwarmState::new_as_coordinator(
            node_a, "laruche-a".into(), "192.168.1.10".into(), 8419, 8192, 16384,
        );

        // Node B has half the VRAM
        let node_b = Uuid::new_v4();
        swarm.peers.insert(node_b, SwarmPeer {
            node_id: node_b,
            node_name: "laruche-b".into(),
            host: "192.168.1.11".into(),
            port: 8419,
            vram_mb: 4096,
            ram_mb: 8192,
            last_heartbeat: Utc::now(),
            status: PeerStatus::Active,
            assigned_layers: None,
        });

        let config = swarm.plan_sharding_by_vram("llama-70b", 80, 40000);
        assert_eq!(config.assignments.len(), 2);

        // Node A (8GB VRAM) should get ~53 layers, Node B (4GB) ~27
        let a_layers = config.assignments.get(&node_a).unwrap();
        let b_layers = config.assignments.get(&node_b).unwrap();
        assert!(a_layers.count() > b_layers.count(), "Higher VRAM peer should get more layers");

        // Total layers must cover all 80
        assert_eq!(a_layers.count() + b_layers.count(), 80);
    }

    #[test]
    fn test_activate_deactivate_sharding() {
        let node_a = Uuid::new_v4();
        let mut swarm = SwarmState::new_as_coordinator(
            node_a, "laruche-a".into(), "192.168.1.10".into(), 8419, 4096, 8192,
        );

        let node_b = Uuid::new_v4();
        swarm.peers.insert(node_b, SwarmPeer {
            node_id: node_b,
            node_name: "laruche-b".into(),
            host: "192.168.1.11".into(),
            port: 8419,
            vram_mb: 4096,
            ram_mb: 8192,
            last_heartbeat: Utc::now(),
            status: PeerStatus::Active,
            assigned_layers: None,
        });

        // Activate
        let config = swarm.activate_sharding("mistral-7b", 32, 4000);
        assert!(config.is_some());
        assert!(swarm.is_sharding_active());
        assert!(swarm.peers.get(&node_a).unwrap().assigned_layers.is_some());
        assert!(swarm.peers.get(&node_b).unwrap().assigned_layers.is_some());

        // Summary
        let summary = swarm.sharding_summary().unwrap();
        assert_eq!(summary.model_name, "mistral-7b");
        assert_eq!(summary.peer_count, 2);
        assert!(summary.estimated_speedup > 1.0);

        // Deactivate
        swarm.deactivate_sharding();
        assert!(!swarm.is_sharding_active());
        assert!(swarm.peers.get(&node_a).unwrap().assigned_layers.is_none());
    }

    #[test]
    fn test_distributed_inference_state() {
        let node_a = Uuid::new_v4();
        let node_b = Uuid::new_v4();

        let mut assignments = HashMap::new();
        assignments.insert(node_a, LayerRange { start: 0, end: 39 });
        assignments.insert(node_b, LayerRange { start: 40, end: 79 });

        let config = ShardingConfig {
            model_name: "llama-70b".into(),
            model_size_mb: 40000,
            total_layers: 80,
            assignments,
            mirror_enabled: false,
        };

        let mut state = DistributedInferenceState::new(Uuid::new_v4(), &config);
        assert_eq!(state.stages.len(), 2);
        assert!(!state.is_complete);

        // Process stages
        state.stage_processing(&node_a);
        assert_eq!(
            state.stages.iter().find(|s| s.peer_id == node_a).unwrap().status,
            PipelineStageStatus::Processing
        );

        state.stage_completed(&node_a, 150);
        state.stage_processing(&node_b);
        state.stage_completed(&node_b, 200);

        assert!(state.is_complete);
        assert!(state.completed_at.is_some());

        // Bottleneck should be node_b (200ms)
        let bottleneck = state.bottleneck().unwrap();
        assert_eq!(bottleneck.peer_id, node_b);
        assert_eq!(bottleneck.processing_ms, Some(200));
    }

    #[test]
    fn test_estimated_speedup() {
        let node_a = Uuid::new_v4();
        let mut swarm = SwarmState::new_as_coordinator(
            node_a, "laruche-a".into(), "192.168.1.10".into(), 8419, 4096, 8192,
        );

        // 1 node: no speedup
        assert_eq!(swarm.estimated_speedup(), 1.0);

        // Add 2 more peers
        for i in 0..2 {
            let id = Uuid::new_v4();
            swarm.peers.insert(id, SwarmPeer {
                node_id: id,
                node_name: format!("peer-{i}"),
                host: format!("192.168.1.{}", 11 + i),
                port: 8419,
                vram_mb: 4096,
                ram_mb: 8192,
                last_heartbeat: Utc::now(),
                status: PeerStatus::Active,
                assigned_layers: None,
            });
        }

        // 3 nodes: ~2.7x speedup (1.0 + 2 * 0.85)
        let speedup = swarm.estimated_speedup();
        assert!(speedup > 2.5 && speedup < 2.8, "Expected ~2.7x, got {speedup}");
    }

    #[test]
    fn test_can_shard() {
        let node_a = Uuid::new_v4();
        let mut swarm = SwarmState::new_as_coordinator(
            node_a, "laruche-a".into(), "192.168.1.10".into(), 8419, 4096, 8192,
        );

        // 1 node can't shard (need >= 2)
        assert!(!swarm.can_shard(4000));

        let node_b = Uuid::new_v4();
        swarm.peers.insert(node_b, SwarmPeer {
            node_id: node_b,
            node_name: "laruche-b".into(),
            host: "192.168.1.11".into(),
            port: 8419,
            vram_mb: 4096,
            ram_mb: 8192,
            last_heartbeat: Utc::now(),
            status: PeerStatus::Active,
            assigned_layers: None,
        });

        // 2 nodes with 8192 MB total: can shard 4000 MB model
        assert!(swarm.can_shard(4000));
        // But not a 10000 MB model
        assert!(!swarm.can_shard(10000));
    }
}
