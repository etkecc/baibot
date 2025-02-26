use mxlink::MessageResponseType;

use crate::{
    entity::{MessageContext, MessagePayload},
    Bot,
};

pub mod agent;
pub(super) mod mime;
pub mod text_to_speech;

pub async fn get_text_body_or_complain<'a>(
    bot: &Bot,
    message_context: &'a MessageContext,
) -> Option<&'a str> {
    match &message_context.payload() {
        MessagePayload::Text(text_message_content) => Some(&text_message_content.body),
        _ => {
            bot.messaging()
                .send_text_markdown_no_fail(
                    message_context.room(),
                    "This command only works with text messages.".to_owned(),
                    MessageResponseType::InThread(message_context.thread_info().clone()),
                )
                .await;

            None
        }
    }
}
