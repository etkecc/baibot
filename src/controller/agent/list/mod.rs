use mxlink::MessageResponseType;

use crate::agent::AgentPurpose;
use crate::strings;
use crate::{entity::MessageContext, Bot};

pub async fn handle(bot: &Bot, message_context: &MessageContext) -> anyhow::Result<()> {
    let agents = bot
        .agent_manager()
        .available_room_agents_by_room_config_context(message_context.room_config_context());

    let mut message = String::new();
    if agents.is_empty() {
        message.push_str(strings::agent::agent_list_empty().as_str());
    } else {
        message.push_str(&strings::agent::non_empty_agent_list_block(&agents));
        message.push_str("\n\n");

        message.push_str(strings::agent::agent_list_legend_intro().as_str());
        for purpose in AgentPurpose::choices() {
            message.push_str(&format!(
                "\n- {} `{}` ({})",
                purpose.emoji(),
                purpose.as_str(),
                strings::agent::purpose_howto(purpose),
            ));
        }
    }

    bot.messaging()
        .send_text_markdown_no_fail(
            message_context.room(),
            message,
            MessageResponseType::Reply(message_context.thread_info().root_event_id.clone()),
        )
        .await;

    Ok(())
}
