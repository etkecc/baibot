use mxlink::MessageResponseType;

use crate::{Bot, entity::MessageContext, strings};

pub async fn handle_get<T>(
    bot: &Bot,
    message_context: &MessageContext,
    value: &Option<T>,
) -> anyhow::Result<()>
where
    T: std::fmt::Display,
{
    match value {
        Some(value) => {
            bot.messaging()
                .send_text_markdown_no_fail(
                    message_context.room(),
                    strings::cfg::value_currently_set_to(value),
                    MessageResponseType::Reply(message_context.thread_info().root_event_id.clone()),
                )
                .await;
        }
        None => {
            bot.messaging()
                .send_text_markdown_no_fail(
                    message_context.room(),
                    strings::cfg::value_currently_unset(),
                    MessageResponseType::Reply(message_context.thread_info().root_event_id.clone()),
                )
                .await;
        }
    }

    Ok(())
}
