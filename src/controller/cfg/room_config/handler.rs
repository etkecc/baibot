use mxlink::MessageResponseType;

use crate::{
    Bot,
    agent::{AgentPurpose, PublicIdentifier},
    entity::MessageContext,
    strings,
};

use crate::entity::roomconfig::RoomConfigurationManager;

pub async fn handle_get(
    bot: &Bot,
    message_context: &MessageContext,
    purpose: AgentPurpose,
) -> anyhow::Result<()> {
    let agent_id = message_context
        .room_config()
        .settings
        .handler
        .get_by_purpose(purpose);

    let Some(agent_id) = agent_id else {
        bot.messaging()
            .send_error_markdown_no_fail(
                message_context.room(),
                &strings::room_config::room_not_configured_with_specific_agent_for_purpose(purpose),
                MessageResponseType::Reply(message_context.thread_info().root_event_id.clone()),
            )
            .await;

        return Ok(());
    };

    let Some(agent_identifier) = PublicIdentifier::from_str(agent_id.as_str()) else {
        bot.messaging()
            .send_error_markdown_no_fail(
                message_context.room(),
                &strings::agent::invalid_id_generic(),
                MessageResponseType::Reply(message_context.thread_info().root_event_id.clone()),
            )
            .await;

        return Ok(());
    };

    let agent_exists = bot
        .agent_manager()
        .available_room_agents_by_room_config_context(message_context.room_config_context())
        .iter()
        .any(|agent| *agent.identifier() == agent_identifier);

    if agent_exists {
        bot.messaging()
            .send_text_markdown_no_fail(
                message_context.room(),
                strings::room_config::configured_to_use_agent_for_purpose(
                    &agent_identifier,
                    purpose,
                ),
                MessageResponseType::Reply(message_context.thread_info().root_event_id.clone()),
            )
            .await;
    } else {
        bot.messaging()
            .send_text_markdown_no_fail(
                message_context.room(),
                strings::room_config::configures_agent_for_purpose_but_does_not_exist(
                    &agent_identifier,
                    purpose,
                ),
                MessageResponseType::Reply(message_context.thread_info().root_event_id.clone()),
            )
            .await;

        return Ok(());
    }

    Ok(())
}

pub async fn handle_set(
    bot: &Bot,
    room_config_manager: &tokio::sync::Mutex<RoomConfigurationManager>,
    message_context: &MessageContext,
    purpose: AgentPurpose,
    agent_identifier: &Option<PublicIdentifier>,
) -> anyhow::Result<()> {
    if let Some(agent_identifier) = agent_identifier {
        let agent_exists = bot
            .agent_manager()
            .available_room_agents_by_room_config_context(message_context.room_config_context())
            .iter()
            .any(|agent| agent.identifier() == agent_identifier);

        if !agent_exists {
            bot.messaging()
                .send_error_markdown_no_fail(
                    message_context.room(),
                    &strings::agent::agent_with_given_identifier_missing(agent_identifier),
                    MessageResponseType::Reply(message_context.thread_info().root_event_id.clone()),
                )
                .await;

            return Ok(());
        }
    }

    let mut new_room_config = message_context.room_config().clone();

    let agent_id = agent_identifier
        .as_ref()
        .map(|agent_identifier| agent_identifier.as_string());

    new_room_config
        .settings
        .handler
        .set_by_purpose(purpose, agent_id);

    room_config_manager
        .lock()
        .await
        .persist(message_context.room(), &new_room_config)
        .await?;

    let message = match agent_identifier {
        Some(agent_identifier) => {
            strings::room_config::reconfigured_to_use_agent_for_purpose(agent_identifier, purpose)
        }
        None => strings::room_config::reconfigured_to_not_specify_agent_for_purpose(purpose),
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
