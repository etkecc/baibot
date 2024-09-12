use async_openai::types::{
    ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestMessage,
    ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
};

use crate::conversation::llm::{Author as LLMAuthor, Message as LLMMessage};

pub fn convert_llm_messages_to_openai_messages(
    conversation_messages: Vec<LLMMessage>,
) -> Vec<ChatCompletionRequestMessage> {
    let mut openai_conversation_messages: Vec<ChatCompletionRequestMessage> =
        Vec::with_capacity(conversation_messages.len());

    for message in conversation_messages {
        openai_conversation_messages.push(convert_llm_message_to_openai_message(message));
    }

    openai_conversation_messages
}

fn convert_llm_message_to_openai_message(llm_message: LLMMessage) -> ChatCompletionRequestMessage {
    match llm_message.author {
        LLMAuthor::Prompt => ChatCompletionRequestSystemMessageArgs::default()
            .content(llm_message.message_text)
            .build()
            .expect("Failed building OpenAI system message")
            .into(),
        LLMAuthor::Assistant => ChatCompletionRequestAssistantMessageArgs::default()
            .content(llm_message.message_text)
            .build()
            .expect("Failed building OpenAI assistant message")
            .into(),
        LLMAuthor::User => ChatCompletionRequestUserMessageArgs::default()
            .content(llm_message.message_text)
            .build()
            .expect("Failed building OpenAI user message")
            .into(),
    }
}

pub(super) fn convert_string_to_enum<T>(value: &str) -> Result<T, String>
where
    T: serde::de::DeserializeOwned,
{
    // This is a hacky way to construct an enum from the string we have.
    let enum_result: serde_json::Result<T> = serde_json::from_str(&format!("\"{}\"", value));
    match enum_result {
        Ok(enum_result) => Ok(enum_result),
        Err(err) => {
            tracing::debug!(?err, "Failed to parse into enum");

            Err(format!("The value ({}) is not supported.", value))
        }
    }
}
