//! Bus d'événements structuré pour LaRuche (porté de laruche-architecture-rust).
//!
//! - IDs incrémentaux, timestamp Unix, acteur, payload JSON libre.
//! - Borné en mémoire (capacité), export/import NDJSON, lecture incrémentale `since(id)`,
//!   filtrage par type. Sert d'observabilité/audit (et de socle au Kanban).

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};
use std::time::{SystemTime, UNIX_EPOCH};

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EventKind {
    UserMessage,
    AssistantMessage,
    ToolCall,
    ToolResult,
    PermissionRequest,
    PermissionDecision,
    MemorySaved,
    MemoryReviewed,
    AgentStarted,
    AgentProgress,
    AgentThought,
    AgentCheckpoint,
    AgentFinished,
    SessionFinished,
    ContextWarning,
    CompactStarted,
    CompactFinished,
    LspDiagnostics,
    WatcherFired,
    KanbanTask,
    SystemStatus,
    ControlRequest,
    ControlResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: u64,
    pub kind: EventKind,
    pub timestamp: u64,
    pub actor: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlRequest {
    pub request_id: String,
    pub subtype: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlResponse {
    pub request_id: String,
    pub accepted: bool,
    pub payload: serde_json::Value,
}

/// Bus borné en mémoire. Cloner librement (l'état est interne).
#[derive(Debug, Clone)]
pub struct EventBus {
    next_id: u64,
    capacity: usize,
    events: VecDeque<Event>,
}

impl Default for EventBus {
    fn default() -> Self {
        Self {
            next_id: 1,
            capacity: 10_000,
            events: VecDeque::new(),
        }
    }
}

impl EventBus {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(mut self, capacity: usize) -> Self {
        self.capacity = capacity.max(1);
        self
    }

    pub fn emit<T: Serialize>(
        &mut self,
        kind: EventKind,
        actor: impl Into<String>,
        payload: T,
    ) -> Result<Event> {
        let event = Event {
            id: self.next_id,
            kind,
            timestamp: unix_now(),
            actor: actor.into(),
            payload: serde_json::to_value(payload)?,
        };
        self.next_id += 1;
        self.events.push_back(event.clone());
        while self.events.len() > self.capacity {
            self.events.pop_front();
        }
        Ok(event)
    }

    /// Événements postérieurs à `last_seen_id` (lecture incrémentale).
    pub fn since(&self, last_seen_id: u64) -> Vec<Event> {
        self.events
            .iter()
            .filter(|event| event.id > last_seen_id)
            .cloned()
            .collect()
    }

    pub fn filter(&self, kind: EventKind) -> Vec<Event> {
        self.events
            .iter()
            .filter(|event| event.kind == kind)
            .cloned()
            .collect()
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    pub fn to_ndjson(&self) -> Result<String> {
        let mut lines = Vec::with_capacity(self.events.len());
        for event in &self.events {
            lines.push(serde_json::to_string(event)?);
        }
        Ok(lines.join("\n"))
    }

    pub fn from_ndjson(input: &str) -> Result<Self> {
        let mut bus = EventBus::new();
        let mut max_id = 0;
        for line in input.lines().filter(|line| !line.trim().is_empty()) {
            let event: Event = serde_json::from_str(line)?;
            max_id = max_id.max(event.id);
            bus.events.push_back(event);
        }
        bus.next_id = max_id + 1;
        Ok(bus)
    }
}

/// Helper : construit un objet JSON à partir de paires clé/valeur (ordre stable).
pub fn object(
    entries: impl IntoIterator<Item = (&'static str, serde_json::Value)>,
) -> serde_json::Value {
    let map: BTreeMap<String, serde_json::Value> = entries
        .into_iter()
        .map(|(key, value)| (key.to_string(), value))
        .collect();
    serde_json::Value::Object(map.into_iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_bus_roundtrips_ndjson() {
        let mut bus = EventBus::new();
        bus.emit(
            EventKind::MemorySaved,
            "memory",
            serde_json::json!({"id":"mem-1"}),
        )
        .unwrap();
        let ndjson = bus.to_ndjson().unwrap();
        let restored = EventBus::from_ndjson(&ndjson).unwrap();
        assert_eq!(restored.since(0).len(), 1);
        assert_eq!(restored.since(0)[0].kind, EventKind::MemorySaved);
    }

    #[test]
    fn since_and_filter_work() {
        let mut bus = EventBus::new();
        bus.emit(
            EventKind::UserMessage,
            "user",
            serde_json::json!({"t":"hi"}),
        )
        .unwrap();
        bus.emit(
            EventKind::WatcherFired,
            "watcher",
            object([("file", serde_json::json!("a.txt"))]),
        )
        .unwrap();
        assert_eq!(bus.since(1).len(), 1);
        assert_eq!(bus.filter(EventKind::WatcherFired).len(), 1);
    }

    #[test]
    fn capacity_is_bounded() {
        let mut bus = EventBus::new().with_capacity(2);
        for i in 0..5 {
            bus.emit(
                EventKind::AgentProgress,
                "agent",
                serde_json::json!({ "i": i }),
            )
            .unwrap();
        }
        assert_eq!(bus.len(), 2);
    }
}
