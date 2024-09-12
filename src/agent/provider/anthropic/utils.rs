use anthropic_rs::completion::message::{Content, ContentType, Message, MessageRequest, Role};

use crate::conversation::llm::{Author as LLMAuthor, Message as LLMMessage};

pub(super) fn create_anthropic_message_request(llm_messages: Vec<LLMMessage>) -> MessageRequest {
    let mut messages = vec![];

    for message in llm_messages {
        let role = match message.author {
            LLMAuthor::User => Role::User,
            LLMAuthor::Assistant => Role::Assistant,
            LLMAuthor::Prompt => {
                continue;
            }
        };

        let content = vec![Content {
            content_type: ContentType::Text,
            text: message.message_text,
        }];

        let message = Message { role, content };

        messages.push(message);
    }

    MessageRequest {
        stream: false,
        messages,
        ..Default::default()
    }
}
