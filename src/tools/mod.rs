//! Tools (function calling): things the model may ask the bot to do while answering,
//! such as reading the room's recent messages.
//!
//! The model gets the tool definitions along with the conversation. When it calls tools, the bot runs
//! them and sends the results back, until the model answers (see [`run_text_generation`]).
//! Provider adapters only translate between their wire format and the types here.

pub mod catalog;
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "used by the builtin.room.history tool, once it's registered"
    )
)]
#[cfg_attr(test, allow(dead_code))]
pub mod room_history;
mod run;

pub use run::{ToolExecutor, run_text_generation, tool_budget};

use std::any::Any;
use std::sync::Arc;

/// A tool as offered to the model.
#[derive(Clone, Debug, PartialEq)]
pub struct ToolDefinition {
    /// The function name sent to the provider (letters, digits and `_` only).
    pub name: String,

    /// What the tool does and when to use it, for the model.
    pub description: String,

    /// A JSON schema object describing the arguments.
    pub parameters: serde_json::Value,
}

/// A tool call requested by the model.
#[derive(Clone, Debug, PartialEq)]
pub struct ToolCall {
    /// The provider's id for this call, which its result refers to.
    pub id: String,

    /// The function name (see [`ToolDefinition::name`]).
    pub name: String,

    /// The arguments, as a JSON text (as the model wrote them, so possibly invalid).
    pub arguments: String,
}

/// The result of a tool call, sent back to the model.
#[derive(Clone, Debug, PartialEq)]
pub struct ToolResult {
    /// The id of the call (see [`ToolCall::id`]).
    pub call_id: String,

    /// The function name of the call.
    pub name: String,

    pub content: String,

    /// Whether the call failed (`content` then says why).
    pub is_error: bool,
}

impl ToolResult {
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "tools use this once there are some")
    )]
    pub fn success(call: &ToolCall, content: String) -> Self {
        Self {
            call_id: call.id.clone(),
            name: call.name.clone(),
            content,
            is_error: false,
        }
    }

    pub fn error(call: &ToolCall, content: impl Into<String>) -> Self {
        Self {
            call_id: call.id.clone(),
            name: call.name.clone(),
            content: content.into(),
            is_error: true,
        }
    }
}

/// Provider-specific state for continuing a turn after tool calls (usually the transcript so far).
///
/// Each provider adapter stores its own type and gets it back with [`Self::into_inner`].
#[derive(Clone)]
pub struct ProviderState(Arc<dyn Any + Send + Sync>);

impl ProviderState {
    pub fn new<T: Any + Send + Sync>(state: T) -> Self {
        Self(Arc::new(state))
    }

    /// Takes the state out, without copying it unless it's shared.
    pub fn into_inner<T: Any + Send + Sync + Clone>(self) -> anyhow::Result<T> {
        let state = self
            .0
            .downcast::<T>()
            .map_err(|_| anyhow::anyhow!("Unexpected provider state for continuing a turn"))?;

        Ok(Arc::try_unwrap(state).unwrap_or_else(|shared| (*shared).clone()))
    }
}

impl std::fmt::Debug for ProviderState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ProviderState")
    }
}

/// What a provider needs to continue a turn after the bot ran the tools the model called.
#[derive(Clone, Debug)]
pub struct ToolContinuation {
    pub state: ProviderState,

    /// One result per call, in the order of the calls.
    pub results: Vec<ToolResult>,
}

/// An executor for replies without tools, which rejects every call.
pub struct NoTools;

impl ToolExecutor for NoTools {
    async fn execute(&self, call: &ToolCall, _max_output_bytes: usize) -> ToolResult {
        ToolResult::error(call, format!("Unknown tool: {}", call.name))
    }
}
