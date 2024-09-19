use mxlink::MessageResponseType;

use crate::{agent::PublicIdentifier, entity::MessageContext, strings, Bot};

pub async fn handle(
    bot: &Bot,
    message_context: &MessageContext,
    agent_identifier: &PublicIdentifier,
) -> anyhow::Result<()> {
    let agents = bot
        .agent_manager()
        .available_room_agents_by_room_config_context(message_context.room_config_context());

    let agent = agents.iter().find(|a| a.identifier() == agent_identifier);

    let agent = match agent {
        Some(agent) => agent,
        None => {
            bot.messaging()
                .send_error_markdown_no_fail(
                    message_context.room(),
                    &strings::agent::agent_with_given_identifier_missing(agent_identifier),
                    MessageResponseType::Reply(message_context.thread_info().root_event_id.clone()),
                )
                .await;

            return Ok(());
        }
    };

    // Access checks

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
        }
        PublicIdentifier::Static(_) => {}
    };

    let config_yaml_pretty = serde_yaml::to_string(&agent.definition().config)?;

    bot.messaging()
        .send_text_markdown_no_fail(
            message_context.room(),
            format!(
                "Configuration for agent `{}` (powered by the `{}` provider):\n```yml\n{}\n```",
                agent_identifier,
                agent.definition().provider.to_static_str(),
                config_yaml_pretty.trim(),
            )
            .to_owned(),
            MessageResponseType::Reply(message_context.thread_info().root_event_id.clone()),
        )
        .await;

    Ok(())
}
