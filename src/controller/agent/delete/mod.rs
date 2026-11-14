use mxlink::MessageResponseType;

use crate::entity::{
    MessageContext,
    globalconfig::{self, GlobalConfigurationManager},
    roomconfig::{self, RoomConfigurationManager},
};
use crate::{
    Bot,
    agent::{AgentDefinition, PublicIdentifier},
    strings,
};

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

    // Access is checked centrally, before dispatching (see `AgentControllerType::required_permission()`).
    match &agent_identifier {
        PublicIdentifier::DynamicRoomLocal(_) => {
            delete_room_local_agent(bot, room_config_manager, message_context, agent_identifier)
                .await
        }
        PublicIdentifier::DynamicGlobal(_) => {
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
    let agent_id_prefixless = agent_id.prefixless();

    // We may unset all handlers in the room config which refer to this agent.
    // We intentionally do not do this, because we do not support "agent edit" yet and ask people to do "agent delete" and "agent create" instead.
    // We'd rather not magically reconfigure the room on agent deletion and obstruct this use case.
    let was_deleted =
        roomconfig::update(room_config_manager, message_context.room(), |room_config| {
            remove_agent(&mut room_config.agents, &agent_id_prefixless)
        })
        .await?
        .is_some();

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
    let agent_id_prefixless = agent_id.prefixless();

    let was_deleted = globalconfig::update(global_config_manager, |global_config| {
        remove_agent(&mut global_config.agents, &agent_id_prefixless)
    })
    .await?
    .is_some();

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

    bot.messaging()
        .send_success_markdown_no_fail(
            message_context.room(),
            &strings::agent::removed_global(agent_id, bot.command_prefix()),
            MessageResponseType::Reply(message_context.thread_info().root_event_id.clone()),
        )
        .await;

    Ok(())
}

/// Removes the agent with the given (prefixless) id, returning `None` if there was no such agent.
fn remove_agent(agents: &mut Vec<AgentDefinition>, agent_id_prefixless: &str) -> Option<()> {
    let count_before = agents.len();
    agents.retain(|agent| agent.id != agent_id_prefixless);

    (agents.len() != count_before).then_some(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn remove_agent() {
        let definition = |id: &str| {
            crate::agent::AgentDefinition::new(
                id.to_owned(),
                crate::agent::AgentProvider::OpenAI,
                serde_yaml_ng::Value::Null,
            )
        };

        let mut agents = vec![definition("a"), definition("b")];

        assert_eq!(super::remove_agent(&mut agents, "missing"), None);
        assert_eq!(agents.len(), 2);

        assert_eq!(super::remove_agent(&mut agents, "a"), Some(()));
        assert_eq!(
            agents.iter().map(|a| a.id.as_str()).collect::<Vec<_>>(),
            ["b"]
        );
    }
}
