use mxlink::MessageResponseType;

use crate::entity::{MessageContext, roomconfig::RoomSettings};
use crate::{Bot, strings};

pub async fn handle_set<T>(
    bot: &Bot,
    message_context: &MessageContext,
    value: &Option<T>,
    setter_callback: Box<dyn FnOnce(&mut RoomSettings) + Send>,
) -> anyhow::Result<()>
where
    T: std::fmt::Display,
{
    let mut global_config = message_context.global_config().clone();
    setter_callback(&mut global_config.fallback_room_settings);

    bot.global_config_manager()
        .lock()
        .await
        .persist(&global_config)
        .await?;

    let message = match value {
        Some(value) => strings::global_config::value_was_set_to(value),
        None => strings::global_config::value_was_unset(),
    };

    bot.messaging()
        .send_success_markdown_no_fail(
            message_context.room(),
            &message,
            MessageResponseType::Reply(message_context.thread_info().root_event_id.clone()),
        )
        .await;

    Ok(())
}
