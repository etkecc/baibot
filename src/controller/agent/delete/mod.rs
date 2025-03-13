use mxlink::MessageResponseType;

use crate::entity::{
    MessageContext, globalconfig::GlobalConfigurationManager, roomconfig::RoomConfigurationManager,
};
use crate::{Bot, agent::PublicIdentifier, strings};

pub async fn handle(
    bot: &Bot,
    room_config_manager: &tokio::sync::Mutex<RoomConfigurationManager>,
    global_config_manager: &tokio::sync::Mutex<GlobalConfigurationManager>,
    message_context: &MessageContext,
    agent_identifier: &PublicIdentifier,
) -> anyhow::Result<()> {
    let agents = bot
        .agent_manager()
        .available_room_agents_by_room_config_context(message_context.room_config_context());

    let agent = agents.iter().find(|a| a.identifier() == agent_identifier);

    let Some(_) = agent else {
        bot.messaging()
            .send_error_markdown_no_fail(
                message_context.room(),
                &strings::agent::agent_with_given_identifier_missing(agent_identifier),
                MessageResponseType::Reply(message_context.thread_info().root_event_id.clone()),
            )
            .await;

        return Ok(());
    };

    match &agent_identifier {
        PublicIdentifier::DynamicRoomLocal(_) => {
            if !message_context.sender_can_manage_room_local_agents()? {
                bot.messaging()
                    .send_error_markdown_no_fail(
                        message_context.room(),
                        &strings::agent::not_allowed_to_manage_room_local_agents_in_room(),
                        MessageResponseType::Reply(
                            message_context.thread_info().root_event_id.clone(),
                        ),
                    )
                    .await;

                return Ok(());
            }

            delete_room_local_agent(bot, room_config_manager, message_context, agent_identifier)
                .await
        }
        PublicIdentifier::DynamicGlobal(_) => {
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

            delete_global_agent(
                bot,
                global_config_manager,
                message_context,
                agent_identifier,
            )
            .await
        }
        PublicIdentifier::Static(_) => {
            bot.messaging()
                .send_error_markdown_no_fail(
                    message_context.room(),
                    &strings::agent::not_allowed_to_manage_static_agents(),
                    MessageResponseType::Reply(message_context.thread_info().root_event_id.clone()),
                )
                .await;

            Ok(())
        }
    }
}

async fn delete_room_local_agent(
    bot: &Bot,
    room_config_manager: &tokio::sync::Mutex<RoomConfigurationManager>,
    message_context: &MessageContext,
    agent_id: &PublicIdentifier,
) -> anyhow::Result<()> {
    let mut room_config = message_context.room_config().clone();

    let mut was_deleted = false;

    let agent_id_prefixless = agent_id.prefixless();

    let mut agents = Vec::new();
    for agent_config in room_config.agents {
        if agent_config.id == agent_id_prefixless {
            was_deleted = true;
        } else {
            agents.push(agent_config.clone());
        }
    }

    if !was_deleted {
        bot.messaging()
            .send_error_markdown_no_fail(
                message_context.room(),
                &strings::agent::agent_with_given_identifier_missing(agent_id),
                MessageResponseType::Reply(message_context.thread_info().root_event_id.clone()),
            )
            .await;

        return Ok(());
    }

    room_config.agents = agents;

    let room_config_manager = room_config_manager.lock().await;

    // We may unset all handlers in the room config which refer to this agent.
    // We intentionally do not do this, because we do not support "agent edit" yet and ask people to do "agent delete" and "agent create" instead.
    // We'd rather not magically reconfigure the room on agent deletion and obstruct this use case.

    room_config_manager
        .persist(message_context.room(), &room_config)
        .await?;

    bot.messaging()
        .send_success_markdown_no_fail(
            message_context.room(),
            &strings::agent::removed_room_local(agent_id, bot.command_prefix()),
            MessageResponseType::Reply(message_context.thread_info().root_event_id.clone()),
        )
        .await;

    Ok(())
}

async fn delete_global_agent(
    bot: &Bot,
    global_config_manager: &tokio::sync::Mutex<GlobalConfigurationManager>,
    message_context: &MessageContext,
    agent_id: &PublicIdentifier,
) -> anyhow::Result<()> {
    let mut global_config = message_context.global_config().clone();

    let mut was_deleted = false;

    let agent_id_prefixless = agent_id.prefixless();

    let mut agents = Vec::new();
    for agent_config in global_config.agents {
        if agent_config.id == agent_id_prefixless {
            was_deleted = true;
        } else {
            agents.push(agent_config.clone());
        }
    }

    if !was_deleted {
        bot.messaging()
            .send_error_markdown_no_fail(
                message_context.room(),
                &strings::agent::agent_with_given_identifier_missing(agent_id),
                MessageResponseType::Reply(message_context.thread_info().root_event_id.clone()),
            )
            .await;

        return Ok(());
    }

    global_config.agents = agents;

    global_config_manager
        .lock()
        .await
        .persist(&global_config)
        .await?;

    bot.messaging()
        .send_success_markdown_no_fail(
            message_context.room(),
            &strings::agent::removed_global(agent_id, bot.command_prefix()),
            MessageResponseType::Reply(message_context.thread_info().root_event_id.clone()),
        )
        .await;

    Ok(())
}
