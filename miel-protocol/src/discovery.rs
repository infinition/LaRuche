//! mDNS-based discovery for the Miel protocol.
//!
//! Uses DNS-SD (Service Discovery) over multicast DNS to broadcast
//! and discover LaRuche nodes on the local network. Every 2 seconds,
//! each LaRuche node broadcasts its Cognitive Manifest.

use crate::error::MielError;
use crate::manifest::{CognitiveManifest, PartialManifest};
use crate::SERVICE_TYPE;
use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{watch, RwLock};
use tracing::{debug, info, warn};
use uuid::Uuid;
const NODE_STALE_TIMEOUT_SECS: i64 = 45;

/// Broadcasts this LaRuche node's presence on the local network via mDNS.
pub struct MielBroadcaster {
    daemon: ServiceDaemon,
    service_fullname: Option<String>,
    node_id: Uuid,
}

impl MielBroadcaster {
    /// Create a new broadcaster for the given manifest.
    pub fn new() -> Result<Self, MielError> {
        let daemon = ServiceDaemon::new()?;
        Ok(Self {
            daemon,
            service_fullname: None,
            node_id: Uuid::new_v4(),
        })
    }

    /// Register this node on the network with the given manifest.
    pub fn register(&mut self, manifest: &CognitiveManifest) -> Result<(), MielError> {
        let uuid_str = manifest.node_id.to_string();
        let instance_name = format!("laruche-{}", &uuid_str[..8]);

        // Build TXT record properties from the manifest
        let txt_props = manifest.to_txt_properties();
        let txt_refs: Vec<(&str, &str)> = txt_props
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();

        let service_info = ServiceInfo::new(
            SERVICE_TYPE,
            &instance_name,
            &format!("{}.local.", manifest.node_name),
            &manifest.api_endpoint.host,
            manifest.api_endpoint.port,
            &txt_refs[..],
        )?;

        self.service_fullname = Some(service_info.get_fullname().to_string());
        self.node_id = manifest.node_id;
        self.daemon.register(service_info)?;

        info!(
            node_id = %manifest.node_id,
            name = %manifest.node_name,
            port = manifest.api_endpoint.port,
            "Miel: Node registered on network"
        );

        Ok(())
    }

    /// Update the broadcast with fresh manifest data (e.g., new load metrics).
    pub fn update(&self, manifest: &CognitiveManifest) -> Result<(), MielError> {
        if self.service_fullname.is_some() {
            let uuid_str = manifest.node_id.to_string();
            let instance_name = format!("laruche-{}", &uuid_str[..8]);
            let txt_props = manifest.to_txt_properties();
            let txt_refs: Vec<(&str, &str)> = txt_props
                .iter()
                .map(|(k, v)| (k.as_str(), v.as_str()))
                .collect();

            let service_info = ServiceInfo::new(
                SERVICE_TYPE,
                &instance_name,
                &format!("{}.local.", manifest.node_name),
                &manifest.api_endpoint.host,
                manifest.api_endpoint.port,
                &txt_refs[..],
            )?;

            self.daemon.register(service_info)?;
            debug!(node_id = %self.node_id, "Miel: Manifest updated");
        }
        Ok(())
    }

    /// Unregister from the network (graceful shutdown).
    pub fn unregister(&self) -> Result<(), MielError> {
        if let Some(ref fullname) = self.service_fullname {
            self.daemon.unregister(fullname)?;
            info!(node_id = %self.node_id, "Miel: Node unregistered from network");
        }
        Ok(())
    }
}

impl Drop for MielBroadcaster {
    fn drop(&mut self) {
        let _ = self.unregister();
        let _ = self.daemon.shutdown();
    }
}

/// Discovered node on the network.
#[derive(Debug, Clone)]
pub struct DiscoveredNode {
    pub manifest: PartialManifest,
    pub discovered_at: chrono::DateTime<chrono::Utc>,
    pub last_seen: chrono::DateTime<chrono::Utc>,
    pub service_fullname: String,
}

/// Listens for LaRuche nodes on the local network via mDNS.
pub struct MielListener {
    daemon: ServiceDaemon,
    nodes: Arc<RwLock<HashMap<String, DiscoveredNode>>>,
    shutdown_tx: Option<watch::Sender<bool>>,
}

impl MielListener {
    /// Create a new listener.
    pub fn new() -> Result<Self, MielError> {
        let daemon = ServiceDaemon::new()?;
        Ok(Self {
            daemon,
            nodes: Arc::new(RwLock::new(HashMap::new())),
            shutdown_tx: None,
        })
    }

    /// Start listening for LaRuche nodes in the background.
    /// Returns a handle to the discovered nodes map.
    pub fn start(&mut self) -> Result<Arc<RwLock<HashMap<String, DiscoveredNode>>>, MielError> {
        let receiver = self.daemon.browse(SERVICE_TYPE)?;
        let nodes = self.nodes.clone();
        let (shutdown_tx, mut shutdown_rx) = watch::channel(false);
        self.shutdown_tx = Some(shutdown_tx);

        tokio::spawn(async move {
            info!("Miel: Listening for LaRuche nodes on the network...");

            loop {
                tokio::select! {
                    _ = shutdown_rx.changed() => {
                        if *shutdown_rx.borrow() {
                            info!("Miel: Listener shutting down");
                            break;
                        }
                    }
                    event = tokio::task::spawn_blocking({
                        let receiver = receiver.clone();
                        move || receiver.recv()
                    }) => {
                        match event {
                            Ok(Ok(event)) => {
                                Self::handle_event(&nodes, event).await;
                            }
                            Ok(Err(e)) => {
                                warn!("Miel: mDNS receive error: {e}");
                            }
                            Err(e) => {
                                warn!("Miel: Task error: {e}");
                                break;
                            }
                        }
                    }
                }
            }
        });

        Ok(self.nodes.clone())
    }

    async fn handle_event(
        nodes: &Arc<RwLock<HashMap<String, DiscoveredNode>>>,
        event: ServiceEvent,
    ) {
        match event {
            ServiceEvent::ServiceResolved(info) => {
                let host = info
                    .get_addresses()
                    .iter()
                    .map(|a| a.to_string())
                    .find(|a| a.parse::<std::net::Ipv4Addr>().is_ok())
                    .or_else(|| {
                        info.get_addresses()
                            .iter()
                            .map(|a| a.to_string())
                            .find(|a| !a.starts_with("fe80"))
                    })
                    .or_else(|| info.get_addresses().iter().next().map(|a| a.to_string()))
                    .unwrap_or_default();

                let properties: Vec<(String, String)> = info
                    .get_properties()
                    .iter()
                    .map(|p| (p.key().to_string(), p.val_str().to_string()))
                    .collect();

                if let Some(manifest) = CognitiveManifest::from_txt_properties(&properties, &host) {
                    let node_key = manifest
                        .node_id
                        .map(|id| id.to_string())
                        .unwrap_or_else(|| info.get_fullname().to_string());

                    let name = manifest
                        .node_name
                        .clone()
                        .unwrap_or_else(|| "unknown".to_string());

                    info!(
                        name = %name,
                        host = %host,
                        capabilities = ?manifest.capabilities,
                        "Miel: Discovered LaRuche node"
                    );

                    let mut nodes = nodes.write().await;
                    let now = chrono::Utc::now();
                    let service_fullname = info.get_fullname().to_string();
                    nodes
                        .entry(node_key)
                        .and_modify(|n| {
                            n.manifest = manifest.clone();
                            n.last_seen = now;
                            n.service_fullname = service_fullname.clone();
                        })
                        .or_insert(DiscoveredNode {
                            manifest,
                            discovered_at: now,
                            last_seen: now,
                            service_fullname,
                        });
                }
            }
            ServiceEvent::ServiceRemoved(_, fullname) => {
                // mDNS stacks can emit transient REMOVE events during refresh/re-announce.
                // Keep nodes until stale timeout unless they truly stop broadcasting.
                info!(
                    service = %fullname,
                    stale_timeout_secs = NODE_STALE_TIMEOUT_SECS,
                    "Miel: service remove observed; waiting for stale timeout before eviction"
                );
            }
            ServiceEvent::SearchStarted(_) => {
                debug!("Miel: Search started");
            }
            _ => {}
        }
    }

    /// Get a snapshot of all currently discovered nodes.
    pub async fn get_nodes(&self) -> HashMap<String, DiscoveredNode> {
        let now = chrono::Utc::now();
        let mut nodes = self.nodes.write().await;
        nodes.retain(|_, n| (now - n.last_seen).num_seconds() <= NODE_STALE_TIMEOUT_SECS);
        nodes.clone()
    }

    /// Find nodes matching specific capabilities.
    pub async fn find_by_capabilities(
        &self,
        required: &[crate::capabilities::Capability],
    ) -> Vec<DiscoveredNode> {
        let nodes = self.get_nodes().await;
        nodes
            .values()
            .filter(|n| {
                required
                    .iter()
                    .all(|cap| n.manifest.capabilities.contains(cap))
            })
            .cloned()
            .collect()
    }

    /// Find the best node for a given capability based on load and speed.
    pub async fn find_best(
        &self,
        capability: crate::capabilities::Capability,
    ) -> Option<DiscoveredNode> {
        let candidates = self.find_by_capabilities(&[capability]).await;
        candidates.into_iter().min_by(|a, b| {
            // Prefer: lower queue depth, then higher tokens/sec
            let queue_a = a.manifest.queue_depth.unwrap_or(u32::MAX);
            let queue_b = b.manifest.queue_depth.unwrap_or(u32::MAX);
            queue_a.cmp(&queue_b).then_with(|| {
                let tps_a = a.manifest.tokens_per_sec.unwrap_or(0.0);
                let tps_b = b.manifest.tokens_per_sec.unwrap_or(0.0);
                tps_b
                    .partial_cmp(&tps_a)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
        })
    }

    /// Stop listening.
    pub fn stop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(true);
        }
    }
}

impl Drop for MielListener {
    fn drop(&mut self) {
        self.stop();
        let _ = self.daemon.shutdown();
    }
}
