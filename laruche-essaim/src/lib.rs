//! # Essaim — Agent Engine for LaRuche
//!
//! Essaim is the agentic framework powering LaRuche. It implements a ReAct-style
//! reasoning loop where an LLM can call tools ("Abeilles") to interact with the
//! world: read files, search the web, execute commands, and more.
//!
//! ## Architecture
//!
//! - **Brain** (`brain.rs`): The ReAct loop — Thought → Action → Observation
//! - **Abeille** (`abeille.rs`): Tool trait and registry
//! - **Session** (`session.rs`): Conversation history and persistence
//! - **Streaming** (`streaming.rs`): Ollama streaming response parser
//! - **Prompt** (`prompt.rs`): System prompt builder with tools schema injection

pub mod abeille;
pub mod abeilles;
pub mod brain;
pub mod cron;
pub mod prompt;
pub mod providers;
pub mod rag;
pub mod session;
pub mod streaming;

pub use abeille::{Abeille, AbeilleRegistry, ContextExecution, NiveauDanger, ResultatAbeille};
pub use brain::{boucle_react, boucle_react_multimodal, ApprovalResponse, ChatEvent, EssaimConfig, PlanItem};
pub use session::{Message, Session};
