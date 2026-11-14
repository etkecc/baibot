use crate::{Bot, entity::MessageContext};

use super::AccessControllerType;

pub async fn dispatch_controller(
    handler: &AccessControllerType,
    message_context: &MessageContext,
    bot: &Bot,
) -> anyhow::Result<()> {
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
