use mxlink::MessageResponseType;

use crate::entity::RoomConfigContext;
use crate::{strings, Bot};

pub async fn handle(
    bot: &Bot,
    room: &mxlink::matrix_sdk::Room,
    room_config_context: &RoomConfigContext,
) -> anyhow::Result<()> {
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
