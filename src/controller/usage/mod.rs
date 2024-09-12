use mxlink::MessageResponseType;

use crate::{entity::MessageContext, strings, Bot};

use super::ControllerType;

pub fn determine_controller(_text: &str) -> ControllerType {
    ControllerType::UsageHelp
}

pub async fn handle_help(message_context: &MessageContext, bot: &Bot) -> anyhow::Result<()> {
    bot.messaging()
        .send_text_markdown_no_fail(
            message_context.room(),
            strings::usage::intro(bot.command_prefix()),
            MessageResponseType::Reply(message_context.thread_info().root_event_id.clone()),
        )
        .await;

    Ok(())
}
