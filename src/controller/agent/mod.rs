use crate::{entity::MessageContext, Bot};

pub mod create;
pub mod delete;
pub mod details;
pub mod determination;
pub mod help;
pub mod list;

pub use determination::{determine_controller, AgentControllerType};

pub async fn dispatch_controller(
    handler: &AgentControllerType,
    message_context: &MessageContext,
    bot: &Bot,
) -> anyhow::Result<()> {
    match handler {
        AgentControllerType::CreateRoomLocal { provider, agent_id } => {
            create::handle_room_local(
                bot,
                bot.room_config_manager(),
                message_context,
                provider,
                agent_id,
            )
            .await
        }
        AgentControllerType::CreateGlobal { provider, agent_id } => {
            create::handle_global(
                bot,
                bot.global_config_manager(),
                message_context,
                provider,
                agent_id,
            )
            .await
        }
        AgentControllerType::List => list::handle(bot, message_context).await,
        AgentControllerType::Details(agent_identifier) => {
            details::handle(bot, message_context, agent_identifier).await
        }
        AgentControllerType::Delete(agent_identifier) => {
            delete::handle(
                bot,
                bot.room_config_manager(),
                bot.global_config_manager(),
                message_context,
                agent_identifier,
            )
            .await
        }
        AgentControllerType::Help => help::handle(bot, message_context).await,
    }
}
