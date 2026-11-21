use crate::agent::ControllerTrait;
use crate::agent::provider::{TextGenerationParams, TextGenerationResult};
use crate::conversation::llm::Conversation;

use super::{ToolCall, ToolContinuation, ToolResult};

/// Rounds of tool calls the bot runs for a single reply. Calls in later rounds get an error result
/// asking the model to answer with what it has.
const DEFAULT_MAX_ROUNDS: usize = 3;

/// Tool calls the bot runs for a single reply.
const DEFAULT_MAX_CALLS: usize = 6;

/// The most tool transcript (in bytes: tool output, plus the model's calls and the text it wrote along with them)
/// for a single reply.
///
/// Short fixed error results (e.g. for calls past a limit) are counted, but may go slightly over it
/// (at most [`MAX_CALLS_PER_BATCH`] of them per round).
pub const MAX_OUTPUT_BYTES_PER_REPLY: usize = 64 * 1024;

/// Tool calls in a single response, beyond which the turn ends (with the text the model wrote, if any).
const MAX_CALLS_PER_BATCH: usize = 16;

const UNKNOWN_TOOL_MESSAGE: &str = "Unknown tool. Only call the tools you were given.";

const LIMIT_REACHED_MESSAGE: &str = "Tool limit reached for this reply. Do not call any more tools: answer with the information you already have.";

const OUTPUT_BUDGET_USED_UP_MESSAGE: &str = "The tool output budget for this reply is used up. Do not call any more tools: answer with the information you already have.";

const TRUNCATION_NOTE: &str =
    "\n[Truncated: the tool result exceeded the output budget for this reply.]";

/// Limits for the tool calls of a single reply.
#[derive(Clone, Copy, Debug)]
pub struct ToolLimits {
    pub max_rounds: usize,
    pub max_calls: usize,
    pub max_output_bytes: usize,
}

impl ToolLimits {
    pub fn with_max_output_bytes(max_output_bytes: usize) -> Self {
        Self {
            max_rounds: DEFAULT_MAX_ROUNDS,
            max_calls: DEFAULT_MAX_CALLS,
            max_output_bytes: max_output_bytes.min(MAX_OUTPUT_BYTES_PER_REPLY),
        }
    }
}

impl Default for ToolLimits {
    fn default() -> Self {
        Self::with_max_output_bytes(MAX_OUTPUT_BYTES_PER_REPLY)
    }
}

/// Most tokens kept free for tool calls and their results when trimming the conversation (about [`MAX_OUTPUT_BYTES_PER_REPLY`]).
const MAX_RESERVED_CONTEXT_TOKENS: u32 = 16 * 1024;

/// Bytes of tool output allowed per reserved token. A token is roughly 4 bytes (about 4 ASCII or 2 Cyrillic
/// characters, as in the approximate token estimate), so this leaves about a quarter of the reservation for the
/// rest of the tool transcript (call arguments, text the model writes along with its calls, error results).
const OUTPUT_BYTES_PER_TOKEN: usize = 3;

/// Determines how many context tokens to keep free for tool calls and results when trimming the conversation,
/// and the matching limits.
///
/// With context management, a quarter of the model's context (at most [`MAX_RESERVED_CONTEXT_TOKENS`]) is left
/// free by the conversation for the tool transcript, most of which tool output may take.
/// Without it, only the fixed limit ([`MAX_OUTPUT_BYTES_PER_REPLY`]) applies.
pub fn tool_budget(
    context_management_enabled: bool,
    max_context_tokens: Option<u32>,
) -> (u32, ToolLimits) {
    match max_context_tokens {
        Some(max_context_tokens) if context_management_enabled && max_context_tokens > 0 => {
            let reserved = (max_context_tokens / 4).min(MAX_RESERVED_CONTEXT_TOKENS);
            (
                reserved,
                ToolLimits::with_max_output_bytes(reserved as usize * OUTPUT_BYTES_PER_TOKEN),
            )
        }
        _ => (0, ToolLimits::default()),
    }
}

/// Runs the tools the model calls.
pub trait ToolExecutor {
    /// Runs a tool call. Failures are reported in the result (see [`ToolResult::error`]), so the model can react.
    ///
    /// `max_output_bytes` is how much output the model can still get for this reply.
    /// Longer output is truncated, so tools should try to stay within it.
    fn execute(
        &self,
        call: &ToolCall,
        max_output_bytes: usize,
    ) -> impl std::future::Future<Output = ToolResult> + Send;
}

/// Generates a reply, running the tools the model calls (see `params.tools`) until it answers.
///
/// Every call gets a result. Calls of tools which weren't offered get an error result. After `limits.max_rounds` rounds or `limits.max_calls` calls, further calls get
/// an error result asking the model to answer. If it calls tools again after that, the reply is whatever text
/// it wrote along with those calls, or an error if there is none.
pub async fn run_text_generation(
    controller: &impl ControllerTrait,
    conversation: Conversation,
    params: TextGenerationParams,
    executor: &impl ToolExecutor,
    limits: ToolLimits,
) -> anyhow::Result<String> {
    let mut conversation = Some(conversation);
    let mut continuation: Option<ToolContinuation> = None;

    let mut rounds = 0;
    let mut calls_count = 0;
    let mut transcript_bytes = 0;
    let mut limit_reached = false;

    loop {
        let mut round_params = params.clone();
        round_params.continuation = continuation.take();

        // The conversation is only needed for the first request.
        // Afterwards, providers continue from their own state (see `ToolContinuation`).
        let round_conversation = conversation
            .take()
            .unwrap_or(Conversation { messages: vec![] });

        let result = controller
            .generate_text(round_conversation, round_params)
            .await?;

        let (text, calls, reasoning_bytes, state) = match result {
            TextGenerationResult::Final { text } => return Ok(text),
            TextGenerationResult::ToolCalls {
                text,
                calls,
                reasoning_bytes,
                continuation,
            } => (text, calls, reasoning_bytes, continuation),
        };

        // A batch this large is a model gone astray. Its error results alone could overflow the context.
        let too_many_calls = calls.len() > MAX_CALLS_PER_BATCH;

        if calls.is_empty() || limit_reached || too_many_calls || params.tools.is_empty() {
            if !calls.is_empty() {
                tracing::warn!(
                    calls_count = calls.len(),
                    limit_reached,
                    "The model called tools it may not call (anymore) - replying with what it wrote"
                );
            }

            return text.filter(|text| !text.trim().is_empty()).ok_or_else(|| {
                anyhow::anyhow!(
                    "The model kept calling tools instead of answering. Try asking differently."
                )
            });
        }

        rounds += 1;

        // What the model wrote (and its reasoning) is sent back along with the results (each call's id twice: in the call
        // and in its result), so it takes from the same budget.
        transcript_bytes += reasoning_bytes
            + text.as_deref().map_or(0, str::len)
            + calls
                .iter()
                .map(|call| 2 * call.id.len() + call.name.len() + call.arguments.len())
                .sum::<usize>();

        // Sending this back would overflow what was set aside for tools, so answer with what there is.
        if transcript_bytes > limits.max_output_bytes {
            tracing::warn!(
                transcript_bytes,
                "The model's tool calls exceed the budget for this reply - replying with what it wrote"
            );

            return text.filter(|text| !text.trim().is_empty()).ok_or_else(|| {
                anyhow::anyhow!(
                    "The model's tool calls were too large for this reply. Try asking differently."
                )
            });
        }

        let mut results = Vec::with_capacity(calls.len());
        for call in &calls {
            if rounds > limits.max_rounds || calls_count >= limits.max_calls {
                limit_reached = true;
                transcript_bytes += LIMIT_REACHED_MESSAGE.len();
                results.push(ToolResult::error(call, LIMIT_REACHED_MESSAGE));
                continue;
            }

            // Calls count towards the limits even when they fail.
            calls_count += 1;

            // Models sometimes call tools they weren't given (made-up names, or ones from earlier conversations).
            if !params.tools.iter().any(|tool| tool.name == call.name) {
                transcript_bytes += UNKNOWN_TOOL_MESSAGE.len();
                results.push(ToolResult::error(call, UNKNOWN_TOOL_MESSAGE));
                continue;
            }

            let remaining_output_bytes = limits.max_output_bytes.saturating_sub(transcript_bytes);
            if remaining_output_bytes == 0 {
                limit_reached = true;
                transcript_bytes += OUTPUT_BUDGET_USED_UP_MESSAGE.len();
                results.push(ToolResult::error(call, OUTPUT_BUDGET_USED_UP_MESSAGE));
                continue;
            }

            let start_time = std::time::Instant::now();
            let mut result = executor.execute(call, remaining_output_bytes).await;
            // Providers match results to calls by id, so make sure it's the call's.
            result.call_id = call.id.clone();
            result.name = call.name.clone();

            truncate_to(&mut result.content, remaining_output_bytes);
            transcript_bytes += result.content.len();

            tracing::debug!(
                tool = call.name,
                duration = ?start_time.elapsed(),
                is_error = result.is_error,
                output_bytes = result.content.len(),
                "Ran tool call"
            );

            results.push(result);
        }

        continuation = Some(ToolContinuation { state, results });
    }
}

/// Shortens text to at most `max_bytes` (including a note saying it was truncated), on a character boundary.
fn truncate_to(text: &mut String, max_bytes: usize) {
    if text.len() <= max_bytes {
        return;
    }

    // When even the note doesn't fit, the text is just cut.
    let with_note = max_bytes >= TRUNCATION_NOTE.len() * 2;

    let mut end = if with_note {
        max_bytes - TRUNCATION_NOTE.len()
    } else {
        max_bytes
    };
    while !text.is_char_boundary(end) {
        end -= 1;
    }

    text.truncate(end);
    if with_note {
        text.push_str(TRUNCATION_NOTE);
    }
}

#[cfg(test)]
mod tests;
