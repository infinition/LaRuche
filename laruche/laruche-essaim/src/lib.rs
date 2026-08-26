//! # Essaim - Agent Engine for LaRuche
//!
//! Essaim is the agentic framework powering LaRuche. It implements a ReAct-style
//! reasoning loop where an LLM can call tools ("Abeilles") to interact with the
//! world: read files, search the web, execute commands, and more.
//!
//! ## Architecture
//!
//! - **Brain** (`brain.rs`): The ReAct loop - Thought -> Action -> Observation
//! - **Abeille** (`abeille.rs`): Tool trait and registry
//! - **Session** (`session.rs`): Conversation history and persistence
//! - **Streaming** (`streaming.rs`): Ollama streaming response parser
//! - **Prompt** (`prompt.rs`): System prompt builder with tools schema injection

pub mod abeille;
pub mod abeilles;
pub mod background_review;
pub mod blueprints;
pub mod brain;
pub mod budget;
pub mod butinage_pont;
pub mod texte_modele;
pub mod codex_auth;
pub mod config;
pub mod approbation;
pub mod contexte;
pub mod credential_pool;
pub mod cron;
pub mod curation;
pub mod deliberation;
pub mod error_classifier;
pub mod i18n;
pub mod evenements;
pub mod fatigue;
pub mod feed_journal;
pub mod hooks;
pub mod secrets;
pub mod job_queue;
pub mod mcp_client;
pub mod orchestration;
pub mod parsing;
pub mod permissions;
pub mod reactions;
pub mod prompt;
pub mod providers;
pub mod rag;
pub mod reine_file;
pub mod reine_juge;
pub mod reine_live;
#[cfg(test)]
mod reine_tests;
pub mod reine_queue;
pub mod session;
pub mod memoire_hotes;
pub mod stats_outils;
pub mod transport;
pub mod stdout_filter;
pub mod streaming;
pub mod subagent;
pub mod thought_stream;
pub mod threat_patterns;
pub mod tool_budget;
pub mod tool_summary;

pub use abeille::{
    Abeille, AbeilleRegistry, ContextExecution, NiveauDanger, ResultatAbeille, ToolOrigin,
};
pub use brain::{
    boucle_react, boucle_react_memoire, boucle_react_memoire_multimodal, boucle_react_multimodal,
    detecter_contradictions, timeout_for_tool, ApprovalResponse, ChatEvent, EssaimConfig, PlanItem,
};
pub use fatigue::{consolider_fatigue, FatigueMonitor};
pub use laruche_permissions::PermissionMode;
pub use session::{Attachment, Message, Session};
pub use subagent::{
    cascade_providers, config_agent_specialise, config_sous_agent, dispatcher_pertinent,
    lancer_sous_agent, AgentRole, ProviderConfig, SubagentResult,
};
