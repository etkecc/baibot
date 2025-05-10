use async_openai::types::{
    ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestMessage,
    ChatCompletionRequestMessageContentPartImage, ChatCompletionRequestSystemMessageArgs,
    ChatCompletionRequestUserMessageArgs, ChatCompletionRequestUserMessageContent,
    ChatCompletionRequestUserMessageContentPart, ImageUrlArgs,
};

use crate::conversation::llm::{
    Author as LLMAuthor, Message as LLMMessage, MessageContent as LLMMessageContent,
};
use crate::utils::base64::base64_encode;

pub fn convert_llm_messages_to_openai_messages(
    conversation_messages: Vec<LLMMessage>,
) -> Vec<ChatCompletionRequestMessage> {
    let mut openai_conversation_messages: Vec<ChatCompletionRequestMessage> =
        Vec::with_capacity(conversation_messages.len());

    for message in conversation_messages {
        let openai_message = convert_llm_message_to_openai_message(message);
        if let Some(openai_message) = openai_message {
            openai_conversation_messages.push(openai_message);
        }
    }

    openai_conversation_messages
}

fn convert_llm_message_to_openai_message(
    llm_message: LLMMessage,
) -> Option<ChatCompletionRequestMessage> {
    match &llm_message.content {
        LLMMessageContent::Text(text) => Some(match llm_message.author {
            LLMAuthor::Prompt => ChatCompletionRequestSystemMessageArgs::default()
                .content(text.clone())
                .build()
                .expect("Failed building OpenAI system message")
                .into(),
            LLMAuthor::Assistant => ChatCompletionRequestAssistantMessageArgs::default()
                .content(text.clone())
                .build()
                .expect("Failed building OpenAI assistant message")
                .into(),
            LLMAuthor::User => ChatCompletionRequestUserMessageArgs::default()
                .content(text.clone())
                .build()
                .expect("Failed building OpenAI user message")
                .into(),
        }),
        LLMMessageContent::Image(image_details) => {
            let image_url = format!(
                "data:{};base64,{}",
                image_details.mime,
                base64_encode(&image_details.data)
            );

            let part = ChatCompletionRequestUserMessageContentPart::ImageUrl(
                ChatCompletionRequestMessageContentPartImage {
                    image_url: ImageUrlArgs::default()
                        .url(image_url)
                        .build()
                        .expect("Failed building OpenAI image url"),
                },
            );

            let message_content = ChatCompletionRequestUserMessageContent::Array(vec![part]);

            match llm_message.author {
                LLMAuthor::User => Some(
                    ChatCompletionRequestUserMessageArgs::default()
                        .content(message_content)
                        .build()
                        .expect("Failed building OpenAI user message")
                        .into(),
                ),
                _ => {
                    tracing::warn!(
                        "OpenAI API does not support image content for messages authored by {:?}. This message part will be skipped.",
                        llm_message.author
                    );
                    None
                }
            }
        }
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
