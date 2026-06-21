use crate::conversation::llm::{
    Author as LLMAuthor, Message as LLMMessage, MessageContent as LLMMessageContent,
};
use crate::utils::base64::base64_encode;

use super::wire::{ChatMessage, ContentPart, ImageUrl, MessageContent};

pub fn convert_llm_messages_to_venice(messages: Vec<LLMMessage>) -> Vec<ChatMessage> {
    let mut venice_messages: Vec<ChatMessage> = Vec::with_capacity(messages.len());

    for message in messages {
        if let Some(venice_message) = convert_llm_message_to_venice(message) {
            venice_messages.push(venice_message);
        }
    }

    venice_messages
}

fn convert_llm_message_to_venice(message: LLMMessage) -> Option<ChatMessage> {
    let role = match message.author {
        LLMAuthor::Prompt => "system",
        LLMAuthor::Assistant => "assistant",
        LLMAuthor::User => "user",
    };

    match message.content {
        LLMMessageContent::Text(text) => Some(ChatMessage {
            role: role.to_owned(),
            content: MessageContent::Text(text),
        }),
        LLMMessageContent::Image(image_details) => {
            // Inline the image as a base64 data URI, the same shape the OpenAI vision content
            // part uses. This is the gap the openai_compat provider can't fill (it drops images).
            let data_uri = format!(
                "data:{};base64,{}",
                image_details.mime,
                base64_encode(&image_details.data)
            );

            Some(ChatMessage {
                role: role.to_owned(),
                content: MessageContent::Parts(vec![ContentPart::ImageUrl {
                    image_url: ImageUrl { url: data_uri },
                }]),
            })
        }
        LLMMessageContent::File(_file_details) => {
            tracing::warn!(
                "The Venice provider does not support file content. This file message will be skipped."
            );
            None
        }
    }
}
