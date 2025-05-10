use anthropic::types::{ContentBlock, ImageSource, Message, MessagesRequest, MessagesRequestBuilder, Role};

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
            LLMMessageContent::Text(text) => vec![ContentBlock::Text { text: text.clone() }],
            LLMMessageContent::Image(image_details) => {
                vec![ContentBlock::Image {
                    source: ImageSource::Base64 {
                        media_type: image_details.mime.to_string(),
                        data: crate::utils::base64::base64_encode(&image_details.data),
                    },
                }]
            }
        };

        let message = Message { role, content };

        messages.push(message);
    }

    MessagesRequestBuilder::default()
        .messages(messages)
        .stream(false)
        .build()
        .expect("Failed to build messages request")
}
