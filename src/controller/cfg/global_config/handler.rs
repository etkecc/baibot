use mxlink::MessageResponseType;

use crate::{
    agent::{AgentPurpose, PublicIdentifier},
    entity::{globalconfig::GlobalConfigurationManager, MessageContext},
    strings, Bot,
};

pub async fn handle_get(
    bot: &Bot,
    message_context: &MessageContext,
    purpose: AgentPurpose,
) -> anyhow::Result<()> {
    let agent_id = message_context
        .global_config()
        .fallback_room_settings
        .handler
        .get_by_purpose(purpose);

    let Some(agent_id) = agent_id else {
        bot.messaging()
            .send_text_markdown_no_fail(
                message_context.room(),
                strings::global_config::global_config_lacks_specific_agent_for_purpose(purpose),
                MessageResponseType::Reply(message_context.thread_info().root_event_id.clone()),
            )
            .await;

        return Ok(());
    };

    let agent_identifier = match PublicIdentifier::from_str(agent_id.as_str()) {
        Some(agent_identifier) => agent_identifier,
        None => {
            bot.messaging()
                .send_error_markdown_no_fail(
                    message_context.room(),
                    &strings::agent::invalid_id_generic(),
                    MessageResponseType::Reply(message_context.thread_info().root_event_id.clone()),
                )
                .await;

            return Ok(());
        }
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
                strings::global_config::configured_to_use_agent_for_purpose(
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
                strings::global_config::configures_agent_for_purpose_but_does_not_exist(
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
    global_config_manager: &tokio::sync::Mutex<GlobalConfigurationManager>,
    message_context: &MessageContext,
    purpose: AgentPurpose,
    agent_identifier: &Option<PublicIdentifier>,
) -> anyhow::Result<()> {
    if let Some(agent_identifier) = agent_identifier {
        let is_allowed = match &agent_identifier {
            PublicIdentifier::Static(_) => true,
            PublicIdentifier::DynamicGlobal(_) => true,
            PublicIdentifier::DynamicRoomLocal(_) => false,
        };

        if !is_allowed {
            bot.messaging()
                .send_error_markdown_no_fail(
                    message_context.room(),
                    &strings::global_config::not_allowed_to_use_agent_in_global_config(
                        agent_identifier,
                    ),
                    MessageResponseType::Reply(message_context.thread_info().root_event_id.clone()),
                )
                .await;

            return Ok(());
        }

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

    let agent_id = agent_identifier
        .as_ref()
        .map(|agent_identifier| agent_identifier.as_string());

    let mut global_config = global_config_manager.lock().await.get_or_create().await?;

    global_config
        .fallback_room_settings
        .handler
        .set_by_purpose(purpose, agent_id);

    global_config_manager
        .lock()
        .await
        .persist(&global_config)
        .await?;

    let message = match agent_identifier {
        Some(agent_identifier) => {
            strings::global_config::reconfigured_to_use_agent_for_purpose(agent_identifier, purpose)
        }
        None => strings::global_config::reconfigured_to_not_specify_agent_for_purpose(purpose),
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
