use mxlink::MessageResponseType;

use crate::entity::RoomConfigContext;
use crate::{Bot, strings};

pub async fn handle(
    bot: &Bot,
    room: &mxlink::matrix_sdk::Room,
    room_config_context: &RoomConfigContext,
) -> anyhow::Result<()> {
    if !bot.post_join_self_introduction_enabled() {
        tracing::debug!(
            "Post-join self-introduction is disabled - not sending introduction message"
        );

        return Ok(());
    }

    let agent_manager = bot.agent_manager();

    bot.messaging()
        .send_text_markdown_no_fail(
            room,
            strings::introduction::create_on_join_introduction(
                bot.name(),
                bot.command_prefix(),
                agent_manager,
                room_config_context,
            )
            .await,
            MessageResponseType::InRoom,
        )
        .await;

    Ok(())
}
