//! Events emitted during the ReAct loop (sent to the WebSocket client) and the
//! small channel types used to steer/approve a running loop.

pub use crate::budget::BudgetStatus;
use serde::{Deserialize, Serialize};

/// Response to an approval request.
#[derive(Debug, Clone)]
pub struct ApprovalResponse {
    pub tool_call_id: String,
    pub approved: bool,
}

/// Channel for receiving approval responses from the UI.
pub type ApprovalReceiver = tokio::sync::mpsc::Receiver<ApprovalResponse>;
pub type SteerReceiver = tokio::sync::mpsc::Receiver<String>;

/// Events emitted during the ReAct loop - sent to the WebSocket client.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ChatEvent {
    #[serde(rename = "token")]
    Token { text: String },

    #[serde(rename = "tool_call")]
    ToolCall {
        name: String,
        args: serde_json::Value,
        #[serde(skip_serializing_if = "Option::is_none")]
        iteration: Option<usize>,
    },

    #[serde(rename = "tool_result")]
    ToolResult {
        name: String,
        result: String,
        success: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        elapsed_ms: Option<u64>,
    },

    #[serde(rename = "approval_request")]
    ApprovalRequest {
        tool_call_id: String,
        name: String,
        args: serde_json::Value,
    },

    #[serde(rename = "done")]
    Done { full_response: String },

    #[serde(rename = "error")]
    Error { message: String },

    #[serde(rename = "status")]
    Status { message: String },

    #[serde(rename = "plan")]
    Plan { items: Vec<PlanItem> },

    #[serde(rename = "thinking")]
    Thinking { text: String },

    #[serde(rename = "thought")]
    Thought {
        phase: String,
        kind: String,
        text: String,
    },

    /// Context compaction happened
    #[serde(rename = "compaction")]
    Compaction {
        messages_before: usize,
        messages_after: usize,
    },

    /// Model failover occurred
    #[serde(rename = "failover")]
    Failover {
        from_model: String,
        to_model: String,
        reason: String,
    },

    /// Token usage and cost estimate
    #[serde(rename = "usage")]
    Usage {
        input_tokens: u32,
        output_tokens: u32,
        cost_usd: f32,
    },

    #[serde(rename = "budget")]
    Budget {
        status: BudgetStatus,
        messages: usize,
    },

    /// A learned OKF skill was auto-injected in THIS turn (learning loop,
    /// automatic recall). The UI shows a chip "Skill applied: <name>".
    #[serde(rename = "skill_applied")]
    SkillApplied { name: String },

    /// The background review proposed a new skill (or an update) from a
    /// successful trajectory. The UI may notify "Skill born: <name>" and refresh the
    /// review queue (`GET /api/memory/proposed`).
    #[serde(rename = "skill_proposed")]
    SkillProposed { name: String },

    /// Lever 2 - tools actually injected for THIS turn (core + retrieved by intent).
    /// The UI shows the transparency: "N tools chosen for your intent" (vs ~30 before).
    #[serde(rename = "tools_selected")]
    ToolsSelected { tools: Vec<String> },
    /// Preview of the payload actually sent to the LLM (debug - eye icon in the UI).
    #[serde(rename = "prompt_debug")]
    PromptDebug {
        /// Exact message array (system + history + ephemeral memory).
        payload: serde_json::Value,
        model: String,
        provider: String,
    },
}

/// A plan/todo item for the agent sidebar.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanItem {
    pub task: String,
    pub status: String,
}
