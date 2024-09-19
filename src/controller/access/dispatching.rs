use mxlink::MessageResponseType;

use crate::{entity::MessageContext, strings, Bot};

use super::AccessControllerType;

pub async fn dispatch_controller(
    handler: &AccessControllerType,
    message_context: &MessageContext,
    bot: &Bot,
) -> anyhow::Result<()> {
    // Only the help command is available without access control, so that all users can get familiar with how the bot's access system works.
    match handler {
        AccessControllerType::Help => {}
        _ => {
            if !message_context.sender_can_manage_global_config() {
                bot.messaging()
                    .send_error_markdown_no_fail(
                        message_context.room(),
                        strings::global_config::no_permissions_to_administrate(),
                        MessageResponseType::Reply(
                            message_context.thread_info().root_event_id.clone(),
                        ),
                    )
                    .await;

                return Ok(());
            }
        }
    };

    match handler {
        AccessControllerType::Help => super::help::handle(bot, message_context).await,
        AccessControllerType::GetUsers => super::users::handle_get(bot, message_context).await,
        AccessControllerType::SetUsers(patterns) => {
            super::users::handle_set(bot, message_context, patterns).await
        }
        AccessControllerType::GetRoomLocalAgentManagers => {
            super::room_local_agent_managers::handle_get(bot, message_context).await
        }
        AccessControllerType::SetRoomLocalAgentManagers(patterns) => {
            super::room_local_agent_managers::handle_set(bot, message_context, patterns).await
        }
    }
}
