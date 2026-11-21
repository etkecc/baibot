mod prompt_variables;

pub use prompt_variables::TextGenerationPromptVariables;

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
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "provider adapters construct this once they support function calling"
        )
    )]
    ToolCalls {
        /// Text the model wrote before calling the tools, if any.
        text: Option<String>,
        calls: Vec<ToolCall>,
        /// Provider-specific state (the transcript so far), needed for continuing the turn.
        continuation: crate::tools::ProviderState,
    },
}

impl TextGenerationResult {
    pub fn text(text: String) -> Self {
        Self::Final { text }
    }
}

#[cfg(test)]
mod tests {
    use super::TextGenerationParams;
    use crate::tools::ToolDefinition;

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
