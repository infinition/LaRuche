//! Compat shim: this file used to hold the whole ReAct agent engine. The
//! legacy loop was removed (2026-07-02, superseded by `butinage_pont`) and
//! the remaining shared types/helpers were dissolved into focused modules:
//! [`crate::config`], [`crate::evenements`], [`crate::parsing`],
//! [`crate::permissions`], [`crate::contexte`], [`crate::curation`].
//!
//! This module only re-exports their public surface under the historical
//! `brain::` path so existing callers across the workspace (node, cli,
//! evals, and other essaim modules) do not need to change their imports.

pub use crate::config::{EssaimConfig, ReineConfig};
pub use crate::contexte::{
    boucle_react, boucle_react_memoire, boucle_react_memoire_multimodal, boucle_react_multimodal,
    boucle_react_multimodal_ext, build_capability_index, charger_doc_systeme,
    demande_recherche_longue, indexer_abeilles_memoire, indexer_abeilles_memoire_ex,
    schema_outils_pour_prompt,
};
pub use crate::curation::{
    consolider_memoire, consolider_node, detecter_contradictions, extraire_json_array,
    node_id_valide,
};
pub use crate::evenements::{
    ApprovalReceiver, ApprovalResponse, ChatEvent, PlanItem, SteerReceiver,
};
pub(crate) use crate::parsing::parse_tool_calls_json_brut;
pub use crate::parsing::{parse_plan, parse_tool_calls, ToolCall};
pub use crate::permissions::{decision_permission, garde_injection, timeout_for_tool};
