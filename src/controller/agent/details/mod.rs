#[cfg(test)]
mod tests;

use mxlink::MessageResponseType;

use crate::{Bot, agent::PublicIdentifier, entity::MessageContext, strings};

pub async fn handle(
    bot: &Bot,
    message_context: &MessageContext,
    agent_identifier: &PublicIdentifier,
    reveal_secrets: bool,
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
        // Static and global agents hold the operator's credentials, so only administrators may print their configuration.
        PublicIdentifier::Static(_) | PublicIdentifier::DynamicGlobal(_) => {
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

    let mut config = agent.definition().config.clone();
    if !reveal_secrets {
        redact_secrets(&mut config);
    }

    let config_yaml_pretty = serde_yaml_ng::to_string(&config)?;

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

/// Keys whose values are secrets and get redacted, wherever they appear in an agent's configuration.
const SECRET_KEYS: &[&str] = &["api_key"];

const REDACTED: &str = "REDACTED";

fn redact_secrets(value: &mut serde_yaml_ng::Value) {
    match value {
        serde_yaml_ng::Value::Mapping(mapping) => {
            for (key, value) in mapping.iter_mut() {
                let is_secret = key.as_str().is_some_and(|key| SECRET_KEYS.contains(&key));

                if is_secret && !value.is_null() {
                    *value = serde_yaml_ng::Value::String(REDACTED.to_owned());
                } else {
                    redact_secrets(value);
                }
            }
        }
        serde_yaml_ng::Value::Sequence(sequence) => {
            sequence.iter_mut().for_each(redact_secrets);
        }
        serde_yaml_ng::Value::Tagged(tagged) => redact_secrets(&mut tagged.value),
        _ => {}
    }
}
