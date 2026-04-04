//! Quality of Service (QoS) system for the Miel protocol.
//!
//! Manages request prioritization across the LaRuche network.
//! Higher priority requests get processed first; during saturation,
//! lower priority requests can be degraded or queued.

use serde::{Deserialize, Serialize};
use std::collections::BinaryHeap;
use std::cmp::Ordering;
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Priority levels for requests on the LaRuche network.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum QosLevel {
    /// IDE, medical apps, real-time queries
    High,
    /// Chatbots, document RAG
    Normal,
    /// Home automation, batch tasks, indexing
    Low,
}

impl QosLevel {
    pub fn priority_value(&self) -> u8 {
        match self {
            Self::High => 3,
            Self::Normal => 2,
            Self::Low => 1,
        }
    }
}

impl PartialOrd for QosLevel {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for QosLevel {
    fn cmp(&self, other: &Self) -> Ordering {
        self.priority_value().cmp(&other.priority_value())
    }
}

/// QoS policy for a node, defining behavior under load.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QosPolicy {
    /// Maximum concurrent high-priority requests
    pub max_high_concurrent: u32,

    /// Maximum concurrent normal-priority requests
    pub max_normal_concurrent: u32,

    /// Maximum queue depth before rejecting LOW priority
    pub max_queue_low: u32,

    /// Whether to degrade LOW requests to a smaller model when saturated
    pub degrade_low_on_saturation: bool,

    /// Fallback model name for degraded requests
    pub fallback_model: Option<String>,
}

impl Default for QosPolicy {
    fn default() -> Self {
        Self {
            max_high_concurrent: 4,
            max_normal_concurrent: 8,
            max_queue_low: 16,
            degrade_low_on_saturation: true,
            fallback_model: None,
        }
    }
}

/// A queued inference request with priority.
#[derive(Debug, Clone)]
pub struct QueuedRequest {
    pub request_id: Uuid,
    pub qos: QosLevel,
    pub queued_at: DateTime<Utc>,
    pub device_id: Uuid,
    pub payload_size_bytes: usize,
}

impl PartialEq for QueuedRequest {
    fn eq(&self, other: &Self) -> bool {
        self.request_id == other.request_id
    }
}

impl Eq for QueuedRequest {}

impl PartialOrd for QueuedRequest {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for QueuedRequest {
    fn cmp(&self, other: &Self) -> Ordering {
        // Higher QoS first, then earlier timestamp
        self.qos
            .cmp(&other.qos)
            .then_with(|| other.queued_at.cmp(&self.queued_at))
    }
}

/// Priority queue for managing inference requests.
pub struct RequestQueue {
    queue: BinaryHeap<QueuedRequest>,
    policy: QosPolicy,
    active_high: u32,
    active_normal: u32,
    active_low: u32,
}

impl RequestQueue {
    pub fn new(policy: QosPolicy) -> Self {
        Self {
            queue: BinaryHeap::new(),
            policy,
            active_high: 0,
            active_normal: 0,
            active_low: 0,
        }
    }

    /// Enqueue a request. Returns Err if the queue is full for this QoS level.
    pub fn enqueue(&mut self, request: QueuedRequest) -> Result<(), QueuedRequest> {
        // Check if we should reject LOW priority when saturated
        if request.qos == QosLevel::Low {
            let low_count = self.queue.iter().filter(|r| r.qos == QosLevel::Low).count() as u32;
            if low_count >= self.policy.max_queue_low {
                return Err(request);
            }
        }

        self.queue.push(request);
        Ok(())
    }

    /// Dequeue the next highest-priority request.
    pub fn dequeue(&mut self) -> Option<QueuedRequest> {
        let request = self.queue.pop()?;
        match request.qos {
            QosLevel::High => self.active_high += 1,
            QosLevel::Normal => self.active_normal += 1,
            QosLevel::Low => self.active_low += 1,
        }
        Some(request)
    }

    /// Mark a request as completed (decrement active counters).
    pub fn complete(&mut self, qos: QosLevel) {
        match qos {
            QosLevel::High => self.active_high = self.active_high.saturating_sub(1),
            QosLevel::Normal => self.active_normal = self.active_normal.saturating_sub(1),
            QosLevel::Low => self.active_low = self.active_low.saturating_sub(1),
        }
    }

    /// Current queue depth (waiting to be dequeued).
    pub fn depth(&self) -> usize {
        self.queue.len()
    }

    /// Number of in-flight requests by QoS level (high, normal, low).
    /// In-flight = dequeued but not yet completed.
    pub fn active_counts(&self) -> (u32, u32, u32) {
        (self.active_high, self.active_normal, self.active_low)
    }

    /// Total in-flight (dequeued but not completed) requests.
    pub fn total_active(&self) -> u32 {
        self.active_high + self.active_normal + self.active_low
    }

    /// Total load: in-flight + queued.
    pub fn total_pending(&self) -> u32 {
        self.total_active() + self.queue.len() as u32
    }

    /// Check if degradation should be applied for this QoS level.
    /// Uses both in-flight and queued counts for accurate saturation detection.
    pub fn should_degrade(&self, qos: QosLevel) -> bool {
        if qos == QosLevel::Low && self.policy.degrade_low_on_saturation {
            self.total_active() >= self.policy.max_normal_concurrent
                || self.queue.len() as u32 >= self.policy.max_queue_low / 2
        } else {
            false
        }
    }

    /// Get the fallback model for degraded requests.
    pub fn fallback_model(&self) -> Option<&str> {
        self.policy.fallback_model.as_deref()
    }

    /// Get the minimum QoS level currently being accepted based on load.
    /// Returns `Low` when accepting all, `Normal` when saturated (only high+normal),
    /// `High` when heavily saturated (only high priority).
    pub fn accepting_qos(&self) -> QosLevel {
        if self.active_high >= self.policy.max_high_concurrent {
            QosLevel::High
        } else if self.active_normal + self.active_high >= self.policy.max_normal_concurrent {
            QosLevel::Normal
        } else {
            QosLevel::Low
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_request(qos: QosLevel) -> QueuedRequest {
        QueuedRequest {
            request_id: Uuid::new_v4(),
            qos,
            queued_at: Utc::now(),
            device_id: Uuid::new_v4(),
            payload_size_bytes: 100,
        }
    }

    #[test]
    fn test_priority_ordering() {
        let mut queue = RequestQueue::new(QosPolicy::default());

        queue.enqueue(make_request(QosLevel::Low)).unwrap();
        queue.enqueue(make_request(QosLevel::High)).unwrap();
        queue.enqueue(make_request(QosLevel::Normal)).unwrap();

        // Should dequeue in priority order: High, Normal, Low
        assert_eq!(queue.dequeue().unwrap().qos, QosLevel::High);
        assert_eq!(queue.dequeue().unwrap().qos, QosLevel::Normal);
        assert_eq!(queue.dequeue().unwrap().qos, QosLevel::Low);
    }

    #[test]
    fn test_active_counters() {
        let mut queue = RequestQueue::new(QosPolicy::default());

        queue.enqueue(make_request(QosLevel::High)).unwrap();
        queue.enqueue(make_request(QosLevel::Normal)).unwrap();

        let first = queue.dequeue().unwrap();
        assert_eq!(first.qos, QosLevel::High);
        assert_eq!(queue.active_counts(), (1, 0, 0));

        let second = queue.dequeue().unwrap();
        assert_eq!(second.qos, QosLevel::Normal);
        assert_eq!(queue.active_counts(), (1, 1, 0));

        queue.complete(QosLevel::High);
        queue.complete(QosLevel::Normal);
        assert_eq!(queue.active_counts(), (0, 0, 0));
    }
}
