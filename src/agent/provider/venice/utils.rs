use crate::conversation::llm::{
    Author as LLMAuthor, Message as LLMMessage, MessageContent as LLMMessageContent,
};
use crate::utils::base64::base64_encode;

use super::wire::{ChatMessage, ContentPart, FilePart, ImageUrl, MessageContent};

/// Venice's documented file-input ceiling is 25MB on the decoded bytes (swagger `file_data`).
/// We check it here so an oversized file gets a clear message instead of an opaque 413 from the
/// API; the 413 status branch in `chat.rs` is the backstop if a file slips past this guard.
const MAX_FILE_BYTES: usize = 25 * 1024 * 1024;

pub fn convert_llm_messages_to_venice(
    messages: Vec<LLMMessage>,
) -> anyhow::Result<Vec<ChatMessage>> {
    let mut venice_messages: Vec<ChatMessage> = Vec::with_capacity(messages.len());

    for message in messages {
        venice_messages.push(convert_llm_message_to_venice(message)?);
    }

    Ok(venice_messages)
}

fn convert_llm_message_to_venice(message: LLMMessage) -> anyhow::Result<ChatMessage> {
    let role = match message.author {
        LLMAuthor::Prompt => "system",
        LLMAuthor::Assistant => "assistant",
        LLMAuthor::User => "user",
    };

    match message.content {
        LLMMessageContent::Text(text) => Ok(ChatMessage {
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

            Ok(ChatMessage {
                role: role.to_owned(),
                content: MessageContent::Parts(vec![ContentPart::ImageUrl {
                    image_url: ImageUrl { url: data_uri },
                }]),
            })
        }
        LLMMessageContent::File(file_details) => {
            // Inline the file as a base64 data URI in a `file` content part. This is the input
            // type the openai_compat provider drops; baibot already extracts the bytes upstream.
            // The message reaches the room, so it carries no user-controlled filename: a crafted
            // name could otherwise inject markdown (a spoofed link) into the bot's reply.
            if file_details.data.len() > MAX_FILE_BYTES {
                return Err(anyhow::anyhow!(
                    "The attached file is too large for Venice (the limit is 25MB)."
                ));
            }

            let data_uri = format!(
                "data:{};base64,{}",
                file_details.mime,
                base64_encode(&file_details.data)
            );

            Ok(ChatMessage {
                role: role.to_owned(),
                content: MessageContent::Parts(vec![ContentPart::File {
                    file: FilePart {
                        file_data: data_uri,
                        filename: Some(file_details.filename()),
                    },
                }]),
            })
        }
    }
}
