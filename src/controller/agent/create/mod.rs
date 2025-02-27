#[cfg(test)]
mod tests;

use mxlink::MessageResponseType;

use crate::agent::PublicIdentifier;
use crate::agent::provider::{ControllerTrait, PingResult};
use crate::agent::{AgentDefinition, create_from_provider_and_yaml_value_config};
use crate::agent::{AgentInstance, AgentProvider};
use crate::controller::utils::get_text_body_or_complain;
use crate::entity::globalconfig::GlobalConfigurationManager;
use crate::entity::roomconfig::RoomConfigurationManager;
use crate::strings;
use crate::{Bot, entity::MessageContext};

struct ParsedAgentConfig {
    agent: AgentInstance,
    config: serde_yaml::Value,
}

pub async fn handle_room_local(
    bot: &Bot,
    room_config_manager: &tokio::sync::Mutex<RoomConfigurationManager>,
    message_context: &MessageContext,
    provider: &str,
    agent_id_prefixless: &str,
) -> anyhow::Result<()> {
    if !message_context.sender_can_manage_room_local_agents()? {
        bot.messaging()
            .send_error_markdown_no_fail(
                message_context.room(),
                &strings::agent::not_allowed_to_manage_room_local_agents_in_room(),
                MessageResponseType::Reply(message_context.thread_info().root_event_id.clone()),
            )
            .await;

        return Ok(());
    }

    let Ok(provider) = AgentProvider::from_string(provider) else {
        bot.messaging()
            .send_error_markdown_no_fail(
                message_context.room(),
                &strings::provider::invalid(provider),
                MessageResponseType::Reply(message_context.thread_info().root_event_id.clone()),
            )
            .await;

        return Ok(());
    };

    let agent_identifier = PublicIdentifier::DynamicRoomLocal(agent_id_prefixless.to_owned());
    if let Err(err) = agent_identifier.validate() {
        bot.messaging()
            .send_error_markdown_no_fail(
                message_context.room(),
                &strings::agent::invalid_id_validation_error(err),
                MessageResponseType::Reply(message_context.thread_info().root_event_id.clone()),
            )
            .await;

        return Ok(());
    }

    let agent_exists = bot
        .agent_manager()
        .available_room_agents_by_room_config_context(message_context.room_config_context())
        .iter()
        .any(|agent| *agent.identifier() == agent_identifier);

    if agent_exists {
        bot.messaging()
            .send_error_markdown_no_fail(
                message_context.room(),
                &strings::agent::already_exists_see_help(agent_id_prefixless, bot.command_prefix()),
                MessageResponseType::InThread(message_context.thread_info().clone()),
            )
            .await;

        return Ok(());
    }

    if message_context.thread_info().is_thread_root_only() {
        return send_guide(bot, message_context, &agent_identifier, &provider).await;
    }

    let Some(text_message_content) = get_text_body_or_complain(bot, message_context).await else {
        return Ok(());
    };

    let parsed_config = parse_agent_config_from_message_or_complain(
        bot,
        message_context,
        &provider,
        &agent_identifier,
        text_message_content,
    )
    .await;
    let Some(parsed_config) = parsed_config else {
        return Ok(());
    };

    if !try_to_ping_agent_or_complain(bot, message_context, &parsed_config.agent).await {
        return Ok(());
    }

    let agent_definition = AgentDefinition::new(
        agent_identifier.prefixless(),
        provider,
        parsed_config.config.clone(),
    );

    let mut room_config = message_context.room_config().clone();

    room_config.agents.push(agent_definition.clone());

    room_config_manager
        .lock()
        .await
        .persist(message_context.room(), &room_config)
        .await?;

    send_completion_wrap_up(
        bot,
        message_context,
        &agent_identifier,
        &parsed_config.agent,
    )
    .await;

    Ok(())
}

pub async fn handle_global(
    bot: &Bot,
    global_config_manager: &tokio::sync::Mutex<GlobalConfigurationManager>,
    message_context: &MessageContext,
    provider: &str,
    agent_id_prefixless: &str,
) -> anyhow::Result<()> {
    if !message_context.sender_can_manage_global_config() {
        bot.messaging()
            .send_error_markdown_no_fail(
                message_context.room(),
                strings::global_config::no_permissions_to_administrate(),
                MessageResponseType::Reply(message_context.thread_info().root_event_id.clone()),
            )
            .await;

        return Ok(());
    }

    let Ok(provider) = AgentProvider::from_string(provider) else {
        bot.messaging()
            .send_error_markdown_no_fail(
                message_context.room(),
                &strings::provider::invalid(provider),
                MessageResponseType::Reply(message_context.thread_info().root_event_id.clone()),
            )
            .await;

        return Ok(());
    };

    let agent_identifier = PublicIdentifier::DynamicGlobal(agent_id_prefixless.to_owned());
    if let Err(err) = agent_identifier.validate() {
        bot.messaging()
            .send_error_markdown_no_fail(
                message_context.room(),
                &strings::agent::invalid_id_validation_error(err),
                MessageResponseType::Reply(message_context.thread_info().root_event_id.clone()),
            )
            .await;

        return Ok(());
    }

    let agent_exists = bot
        .agent_manager()
        .available_room_agents_by_room_config_context(message_context.room_config_context())
        .iter()
        .any(|agent| *agent.identifier() == agent_identifier);

    if agent_exists {
        bot.messaging()
            .send_error_markdown_no_fail(
                message_context.room(),
                &strings::agent::already_exists_see_help(agent_id_prefixless, bot.command_prefix()),
                MessageResponseType::InThread(message_context.thread_info().clone()),
            )
            .await;

        return Ok(());
    }

    if message_context.thread_info().is_thread_root_only() {
        return send_guide(bot, message_context, &agent_identifier, &provider).await;
    }

    let Some(text_message_content) = get_text_body_or_complain(bot, message_context).await else {
        return Ok(());
    };

    let parsed_config = parse_agent_config_from_message_or_complain(
        bot,
        message_context,
        &provider,
        &agent_identifier,
        text_message_content,
    )
    .await;
    let Some(parsed_config) = parsed_config else {
        return Ok(());
    };

    if !try_to_ping_agent_or_complain(bot, message_context, &parsed_config.agent).await {
        return Ok(());
    }

    let agent_definition = AgentDefinition::new(
        agent_identifier.prefixless(),
        provider,
        parsed_config.config.clone(),
    );

    let mut global_config = message_context.global_config().clone();
    global_config.agents.push(agent_definition.clone());

    global_config_manager
        .lock()
        .await
        .persist(&global_config)
        .await?;

    send_completion_wrap_up(
        bot,
        message_context,
        &agent_identifier,
        &parsed_config.agent,
    )
    .await;

    Ok(())
}

async fn send_guide(
    bot: &Bot,
    message_context: &MessageContext,
    agent_identifier: &PublicIdentifier,
    provider: &AgentProvider,
) -> anyhow::Result<()> {
    let sample_config = crate::agent::default_config_for_provider(provider);
    let sample_config_pretty_yaml = serde_yaml::to_string(&sample_config)?;

    bot.messaging()
        .send_text_markdown_no_fail(
            message_context.room(),
            strings::agent::creation_guide(agent_identifier, provider, &sample_config_pretty_yaml),
            MessageResponseType::InThread(message_context.thread_info().clone()),
        )
        .await;

    Ok(())
}

fn parse_from_message_to_yaml_value(text: &str) -> Result<serde_yaml::Value, String> {
    let mut text = text.trim();

    if text.starts_with("```") {
        // Try to strip ```yml and ```yaml first and fall back to the generic ``` later.
        text = text.trim_start_matches("```yml");
        text = text.trim_start_matches("```yaml");
        text = text.trim_start_matches("```");
        text = text.trim_end_matches("```");
    }

    let config: serde_yaml::Value = serde_yaml::from_str(text).map_err(|e| e.to_string())?;

    match config {
        serde_yaml::Value::Mapping(_) => {}
        _ => {
            return Err("Not a valid YAML hashmap".to_owned());
        }
    };

    Ok(config)
}

async fn parse_agent_config_from_message_or_complain(
    bot: &Bot,
    message_context: &MessageContext,
    provider: &AgentProvider,
    agent_identifier: &PublicIdentifier,
    text: &str,
) -> Option<ParsedAgentConfig> {
    let config_yaml_value = parse_from_message_to_yaml_value(text);

    let config_yaml_value = match config_yaml_value {
        Ok(config_yaml_value) => config_yaml_value,
        Err(err) => {
            bot.messaging()
                .send_error_markdown_no_fail(
                    message_context.room(),
                    &strings::agent::configuration_not_a_valid_yaml_hashmap(err),
                    MessageResponseType::InThread(message_context.thread_info().clone()),
                )
                .await;

            return None;
        }
    };

    let agent = create_from_provider_and_yaml_value_config(
        provider,
        agent_identifier,
        config_yaml_value.clone(),
    );

    let agent = match agent {
        Ok(agent) => ParsedAgentConfig {
            agent,
            config: config_yaml_value,
        },
        Err(err) => {
            bot.messaging()
                .send_error_markdown_no_fail(
                    message_context.room(),
                    &strings::provider::invalid_configuration_for_provider(provider, err),
                    MessageResponseType::InThread(message_context.thread_info().clone()),
                )
                .await;

            return None;
        }
    };

    Some(agent)
}

async fn try_to_ping_agent_or_complain(
    bot: &Bot,
    message_context: &MessageContext,
    agent_instance: &AgentInstance,
) -> bool {
    bot.messaging()
        .send_notice_markdown_no_fail(
            message_context.room(),
            format!("⏳ {}", strings::agent::configuration_agent_will_ping()),
            MessageResponseType::InThread(message_context.thread_info().clone()),
        )
        .await;

    match agent_instance.controller().ping().await {
        Ok(ping_result) => {
            let message = match ping_result {
                PingResult::Inconclusive => format!(
                    "❓ {}",
                    strings::agent::configuration_agent_ping_inconclusive()
                ),
                PingResult::Successful => {
                    format!("✅ {}", strings::agent::configuration_agent_ping_ok())
                }
            };

            bot.messaging()
                .send_notice_markdown_no_fail(
                    message_context.room(),
                    message,
                    MessageResponseType::InThread(message_context.thread_info().clone()),
                )
                .await;

            true
        }
        Err(err) => {
            bot.messaging()
                .send_error_markdown_no_fail(
                    message_context.room(),
                    &strings::agent::configuration_does_not_result_in_a_working_agent(err),
                    MessageResponseType::InThread(message_context.thread_info().clone()),
                )
                .await;

            false
        }
    }
}

async fn send_completion_wrap_up(
    bot: &Bot,
    message_context: &MessageContext,
    agent_identifier: &PublicIdentifier,
    agent_instance: &AgentInstance,
) {
    bot.messaging()
        .send_success_markdown_no_fail(
            message_context.room(),
            &strings::agent::created(agent_identifier),
            MessageResponseType::InThread(message_context.thread_info().clone()),
        )
        .await;

    bot.messaging()
        .send_tooltip_markdown_no_fail(
            message_context.room(),
            &strings::agent::post_creation_helpful_commands(
                agent_identifier,
                agent_instance,
                bot.command_prefix(),
            ),
            MessageResponseType::InThread(message_context.thread_info().clone()),
        )
        .await;
}
