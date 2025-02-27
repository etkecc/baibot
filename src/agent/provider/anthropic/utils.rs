use anthropic::types::{ContentBlock, Message, MessagesRequest, MessagesRequestBuilder, Role};

use crate::conversation::llm::{Author as LLMAuthor, Message as LLMMessage};

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

        let content = vec![ContentBlock::Text {
            text: message.message_text,
        }];

        let message = Message { role, content };

        messages.push(message);
    }

    MessagesRequestBuilder::default()
        .messages(messages)
        .stream(false)
        .build()
        .expect("Failed to build messages request")
}
