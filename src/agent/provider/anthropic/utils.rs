use anthropic::types::{ContentBlock, Message, MessagesRequest, MessagesRequestBuilder, Role};

use crate::conversation::llm::{Author as LLMAuthor, Message as LLMMessage, MessageContent as LLMMessageContent};

pub(super) fn create_anthropic_message_request(llm_messages: Vec<LLMMessage>) -> MessagesRequest {
    let mut messages = vec![];

    for message in llm_messages {
        let role = match message.author {
            LLMAuthor::User => Role::User,
            LLMAuthor::Assistant => Role::Assistant,
            LLMAuthor::Prompt => {
                continue;
            }
        };

        let content = match &message.content {
            LLMMessageContent::Text(text) => Some(vec![ContentBlock::Text { text: text.clone() }]),
            LLMMessageContent::Image(_image_details) => {
                // This cannot be implemented yet, because the Anthropic library does not support it.
                // The code below requires the library to be forked and changed a bit.
                // vec![ContentBlock::Image {
                //     source: ImageSource {
                //         r#type: "base64".to_string(),
                //         media_type: mime_type.to_string(),
                //         data: crate::utils::base64::base64_encode(image_details.data),
                //     },
                // }]
                tracing::warn!("Image content is not supported by the Anthropic library yet. Skipping it.");
                None
            }
        };

        if let Some(content) = content {
            let message = Message { role, content };

            messages.push(message);
        }
    }

    MessagesRequestBuilder::default()
        .messages(messages)
        .stream(false)
        .build()
        .expect("Failed to build messages request")
}
