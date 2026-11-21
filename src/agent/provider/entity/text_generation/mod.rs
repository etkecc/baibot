mod prompt_variables;

pub use prompt_variables::TextGenerationPromptVariables;

use crate::conversation::llm::{
    Author, Message, MessageContent, TokenEstimate, shorten_messages_list_to_context_size,
};
use crate::tools::{ToolCall, ToolContinuation, ToolDefinition};

#[derive(Default, Clone)]
pub struct TextGenerationParams {
    pub context_management_enabled: bool,
    pub prompt_override: Option<String>,
    pub temperature_override: Option<f32>,
    pub prompt_variables: TextGenerationPromptVariables,

    /// Functions (tools that the bot runs) offered to the model. Empty means none.
    ///
    /// Providers which do not support function calling, or agents with `bot_tools` turned off,
    /// ignore these.
    pub tools: Vec<ToolDefinition>,

    /// Text appended to the system prompt when tools are offered (e.g. about tools the requester cannot use).
    pub tool_instructions: Option<String>,

    /// Tokens kept free for tool calls and their results when the conversation is trimmed to the context size.
    pub reserved_context_tokens: u32,

    /// Set when continuing a turn after running the tools the model called (see [`TextGenerationResult::ToolCalls`]).
    pub continuation: Option<ToolContinuation>,
}

impl TextGenerationParams {
    /// Builds the system prompt: the room's override or the agent's prompt (with variables filled in),
    /// followed by the tool instructions when tools are offered.
    pub fn system_prompt(&self, agent_prompt: Option<String>) -> String {
        let prompt = self.prompt_variables.format(
            self.prompt_override
                .clone()
                .unwrap_or(agent_prompt.unwrap_or_default())
                .trim(),
        );

        let tool_instructions = self
            .tool_instructions
            .as_deref()
            .map(str::trim)
            .filter(|text| !text.is_empty() && !self.tools.is_empty());

        match tool_instructions {
            Some(tool_instructions) if prompt.is_empty() => tool_instructions.to_owned(),
            Some(tool_instructions) => format!("{prompt}\n\n{tool_instructions}"),
            None => prompt,
        }
    }
}

pub enum TextGenerationResult {
    /// The model's answer.
    Final { text: String },

    /// The model wants the bot to run some tools before it answers.
    ///
    /// Every call needs a result (see [`ToolContinuation`]), which is sent back along with `continuation`.
    ToolCalls {
        /// Text the model wrote before calling the tools, if any.
        text: Option<String>,
        calls: Vec<ToolCall>,
        /// Bytes the provider sends back along with the text and calls (e.g. the model's reasoning),
        /// which take from the tool transcript budget too.
        reasoning_bytes: usize,
        /// Provider-specific state (the transcript so far), needed for continuing the turn.
        continuation: crate::tools::ProviderState,
    },
}

/// The system prompt (as text and as a message, if not empty) and the conversation's messages for the first request
/// of a turn, trimmed to the context size (with context management).
pub struct PreparedConversation {
    pub prompt_text: String,
    pub prompt_message: Option<Message>,
    pub messages: Vec<Message>,
}

/// Builds the system prompt and trims the conversation (with context management) for the first request of a turn.
///
/// Context is kept free for tools (`params.reserved_context_tokens`) only when tools are offered.
/// When even the latest message wouldn't fit next to that, the tools are dropped from `params`
/// (the model answers without them), rather than the message.
pub fn prepare_conversation(
    params: &mut TextGenerationParams,
    agent_prompt: Option<String>,
    messages: Vec<Message>,
    estimate: TokenEstimate<'_>,
    max_response_tokens: u32,
    max_context_tokens: u32,
) -> PreparedConversation {
    let prompt = |params: &TextGenerationParams| {
        let prompt_text = params.system_prompt(agent_prompt.clone());

        let prompt_message = (!prompt_text.is_empty()).then(|| Message {
            author: Author::Prompt,
            sender_id: None,
            content: MessageContent::Text(prompt_text.clone()),
            timestamp: chrono::Utc::now(),
        });

        (prompt_text, prompt_message)
    };

    let (prompt_text, prompt_message) = prompt(params);

    if !params.context_management_enabled {
        return PreparedConversation {
            prompt_text,
            prompt_message,
            messages,
        };
    }

    let reserved_context_tokens = if params.tools.is_empty() {
        0
    } else {
        params.reserved_context_tokens
    };

    // Kept for trimming again without the tools, if needed.
    let all_messages = (reserved_context_tokens > 0).then(|| messages.clone());

    let trimmed = shorten_messages_list_to_context_size(
        estimate,
        &prompt_message,
        messages,
        Some(max_response_tokens + reserved_context_tokens),
        max_context_tokens,
    );

    if let Some(all_messages) = all_messages
        && trimmed.is_empty()
        && !all_messages.is_empty()
    {
        tracing::warn!(
            reserved_context_tokens,
            "Not enough context for the latest message and tools - answering without tools"
        );

        params.tools.clear();

        let (prompt_text, prompt_message) = prompt(params);

        let messages = shorten_messages_list_to_context_size(
            estimate,
            &prompt_message,
            all_messages,
            Some(max_response_tokens),
            max_context_tokens,
        );

        return PreparedConversation {
            prompt_text,
            prompt_message,
            messages,
        };
    }

    PreparedConversation {
        prompt_text,
        prompt_message,
        messages: trimmed,
    }
}

impl TextGenerationResult {
    pub fn text(text: String) -> Self {
        Self::Final { text }
    }
}

#[cfg(test)]
mod tests {
    use super::{TextGenerationParams, prepare_conversation};
    use crate::conversation::llm::{Author, Message, MessageContent, TokenEstimate};
    use crate::tools::ToolDefinition;

    /// A user message of about `tokens` tokens (4 ASCII characters per token).
    fn message(tokens: usize) -> Message {
        Message {
            author: Author::User,
            sender_id: None,
            content: MessageContent::Text("a".repeat(tokens * 4)),
            timestamp: chrono::Utc::now(),
        }
    }

    fn tool_params(reserved_context_tokens: u32) -> TextGenerationParams {
        TextGenerationParams {
            context_management_enabled: true,
            reserved_context_tokens,
            tools: vec![tool()],
            tool_instructions: Some("About tools".to_owned()),
            ..Default::default()
        }
    }

    fn prepare(params: &mut TextGenerationParams, messages: Vec<Message>) -> usize {
        prepare_conversation(
            params,
            None,
            messages,
            TokenEstimate::Approximate,
            100,
            1000,
        )
        .messages
        .len()
    }

    #[test]
    fn context_is_kept_for_tools_only_when_they_are_offered() {
        // With tools, only one of the two messages fits next to the reservation.
        let mut params = tool_params(500);
        assert_eq!(prepare(&mut params, vec![message(300), message(300)]), 1);
        assert_eq!(params.tools.len(), 1);

        // Without tools, nothing is kept for them.
        let mut params = TextGenerationParams {
            tools: vec![],
            ..tool_params(500)
        };
        assert_eq!(prepare(&mut params, vec![message(300), message(300)]), 2);
    }

    #[test]
    fn tools_are_dropped_rather_than_the_latest_message() {
        let mut params = tool_params(500);

        let prepared = prepare_conversation(
            &mut params,
            None,
            vec![message(600)],
            TokenEstimate::Approximate,
            100,
            1000,
        );

        assert_eq!(prepared.messages.len(), 1);
        assert!(params.tools.is_empty());
        // The prompt no longer mentions tools.
        assert_eq!(prepared.prompt_text, "");
        assert!(prepared.prompt_message.is_none());
    }

    fn tool() -> ToolDefinition {
        ToolDefinition {
            name: "room_history".to_owned(),
            description: "Reads messages".to_owned(),
            parameters: serde_json::json!({"type": "object"}),
        }
    }

    #[test]
    fn tool_instructions_follow_the_prompt_only_when_tools_are_offered() {
        let mut params = TextGenerationParams {
            prompt_override: Some(" Room prompt ".to_owned()),
            tool_instructions: Some("About tools".to_owned()),
            ..Default::default()
        };

        assert_eq!(
            params.system_prompt(Some("Agent".to_owned())),
            "Room prompt"
        );

        params.tools = vec![tool()];
        assert_eq!(
            params.system_prompt(Some("Agent".to_owned())),
            "Room prompt\n\nAbout tools"
        );

        params.prompt_override = None;
        assert_eq!(params.system_prompt(None), "About tools");
    }
}
