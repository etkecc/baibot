use mxlink::MessageResponseType;

use crate::entity::{roomconfig::RoomSettings, MessageContext};
use crate::{strings, Bot};

pub async fn handle_set<T>(
    bot: &Bot,
    message_context: &MessageContext,
    value: &Option<T>,
    setter_callback: Box<dyn FnOnce(&mut RoomSettings) + Send>,
) -> anyhow::Result<()>
where
    T: std::fmt::Display,
{
    let mut room_config = message_context.room_config().clone();
    setter_callback(&mut room_config.settings);

    bot.room_config_manager()
        .lock()
        .await
        .persist(message_context.room(), &room_config)
        .await?;

    let message = match value {
        Some(value) => strings::room_config::value_was_set_to(value),
        None => strings::room_config::value_was_unset(),
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
