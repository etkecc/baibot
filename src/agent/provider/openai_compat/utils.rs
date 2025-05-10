use etke_openai_api_rust::{Message, Role};

use crate::agent::provider::openai::Config as OpenAIConfig;

use crate::conversation::llm::{Author as LLMAuthor, Message as LLMMessage, MessageContent as LLMMessageContent};

pub fn convert_llm_messages_to_openai_messages(
    conversation_messages: Vec<LLMMessage>,
) -> Vec<Message> {
    let mut openai_conversation_messages: Vec<Message> =
        Vec::with_capacity(conversation_messages.len());

    for message in conversation_messages {
        let openai_message = convert_llm_message_to_openai_message(message);
        if let Some(openai_message) = openai_message {
            openai_conversation_messages.push(openai_message);
        }
    }

    openai_conversation_messages
}

fn convert_llm_message_to_openai_message(llm_message: LLMMessage) -> Option<Message> {
    let role = match llm_message.author {
        LLMAuthor::Prompt => Role::System,
        LLMAuthor::Assistant => Role::Assistant,
        LLMAuthor::User => Role::User,
    };

    match &llm_message.content {
        LLMMessageContent::Text(text) => {
            Some(Message {
                role,
                content: text.clone(),
            })
        },
        LLMMessageContent::Image(_image_details) => {
            tracing::warn!("The OpenAI-compat provider's library does not support image content. This image message will be skipped.");
            None
        },
    }
}

pub(super) fn convert_config_to_openai_config_lossy(config: &super::Config) -> OpenAIConfig {
    let text_generation = config
        .text_generation
        .as_ref()
        .and_then(|tg| tg.clone().try_into().ok());

    let speech_to_text = config
        .speech_to_text
        .as_ref()
        .and_then(|stt| stt.clone().try_into().ok());

    let text_to_speech = config
        .text_to_speech
        .as_ref()
        .and_then(|tts| tts.clone().try_into().ok());

    let image_generation = config
        .image_generation
        .as_ref()
        .and_then(|ig| ig.clone().try_into().ok());

    OpenAIConfig {
        api_key: config.api_key.clone().unwrap_or("".to_string()),
        text_generation,
        speech_to_text,
        text_to_speech,
        image_generation,
        base_url: config.base_url.clone(),
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
