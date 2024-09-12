use mxlink::MessageResponseType;

use crate::{agent::AgentProvider, entity::MessageContext, strings, Bot};

use super::ControllerType;

pub fn determine_controller(_text: &str) -> ControllerType {
    ControllerType::ProviderHelp
}

pub async fn handle_help(message_context: &MessageContext, bot: &Bot) -> anyhow::Result<()> {
    if !message_context.sender_can_manage_room_local_agents()? {
        bot.messaging()
            .send_error_markdown_no_fail(
                message_context.room(),
                &strings::provider::not_allowed(),
                MessageResponseType::Reply(message_context.thread_info().root_event_id.clone()),
            )
            .await;

        return Ok(());
    }

    let mut message = String::new();
    message.push_str(&format!("## {}", strings::help::provider::heading()));
    message.push_str("\n\n");
    message.push_str(&strings::help::provider::intro());
    message.push_str("\n\n");
    message.push_str(&strings::provider::providers_list_intro());
    message.push_str("\n\n");

    // How to choose
    message.push_str(&format!(
        "### {}",
        strings::provider::help_how_to_choose_heading()
    ));
    message.push_str("\n\n");
    message.push_str(&strings::provider::help_how_to_choose_description(
        bot.command_prefix(),
    ));
    message.push_str("\n\n");

    // How to use
    message.push_str(&format!(
        "### {}",
        strings::provider::help_how_to_use_heading()
    ));
    message.push_str("\n\n");
    message.push_str(&strings::provider::help_how_to_use_description(
        bot.command_prefix(),
    ));
    message.push_str("\n\n");

    for provider in AgentProvider::choices() {
        let provider_info = provider.info();

        message.push_str(&format!(
            "### {}",
            strings::provider::help_provider_heading(
                provider_info.name,
                &provider_info.homepage_url.as_ref().map(|s| s.to_string())
            )
        ));

        message.push_str("\n\n");

        message.push_str(&strings::provider::help_provider_details(
            provider.to_static_str(),
            &provider_info,
        ));

        message.push_str("- 🗲 Quick start:\n");
        message.push_str(&format!(
            "\t- create a room-local agent: `{command_prefix} agent create-room-local {provider_id} my-{provider_id}-agent`",
            command_prefix = bot.command_prefix(),
            provider_id = provider.to_static_str(),
        ));
        message.push('\n');
        message.push_str(&format!(
            "\t- create a global agent: `{command_prefix} agent create-global {provider_id} my-{provider_id}-agent`",
            command_prefix = bot.command_prefix(),
            provider_id = provider.to_static_str(),
        ));

        message.push_str("\n\n");
    }

    bot.messaging()
        .send_text_markdown_no_fail(
            message_context.room(),
            message,
            MessageResponseType::Reply(message_context.thread_info().root_event_id.clone()),
        )
        .await;

    Ok(())
}
