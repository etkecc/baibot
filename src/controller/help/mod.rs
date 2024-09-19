use mxlink::MessageResponseType;

use crate::{entity::MessageContext, strings, Bot};

pub async fn handle(bot: &Bot, message_context: &MessageContext) -> anyhow::Result<()> {
    let sender_can_manage_global_config = message_context.sender_can_manage_global_config();
    let sender_can_manage_room_local_agents =
        message_context.sender_can_manage_room_local_agents()?;

    let mut message = String::from("");
    message.push_str(&format!("## {}\n\n", strings::help::heading_introduction()));
    message.push_str(&strings::introduction::create_short_introduction(
        bot.name(),
    ));

    message.push_str("\n\n");

    // Agents
    message.push_str(&format!("## {}", strings::help::agent::heading()));
    message.push_str("\n\n");
    message.push_str(&strings::help::agent::intro(bot.command_prefix()));
    message.push_str("\n\n");
    message.push_str(&strings::help::agent::intro_handler_relation(
        bot.command_prefix(),
    ));
    message.push_str("\n\n");
    message.push_str(&strings::help::learn_more_send_a_command(
        bot.command_prefix(),
        "agent",
    ));
    message.push_str("\n\n");

    // Providers
    if sender_can_manage_room_local_agents || sender_can_manage_global_config {
        message.push_str(&format!("## {}", strings::help::provider::heading()));
        message.push_str("\n\n");
        message.push_str(&strings::help::provider::intro());
        message.push_str("\n\n");
        message.push_str(&strings::help::learn_more_send_a_command(
            bot.command_prefix(),
            "provider",
        ));
        message.push_str("\n\n");
    }

    // Access
    message.push_str(&format!("## {}", strings::help::access::heading()));
    message.push_str("\n\n");
    message.push_str(&strings::help::access::intro());
    message.push_str("\n\n");
    message.push_str(&strings::help::learn_more_send_a_command(
        bot.command_prefix(),
        "access",
    ));
    message.push_str("\n\n");

    // Configuration
    message.push_str(&format!("## {}", strings::help::cfg::heading()));
    message.push_str("\n\n");
    message.push_str(strings::help::cfg::intro_short());
    message.push_str("\n\n");
    message.push_str(&strings::help::learn_more_send_a_command(
        bot.command_prefix(),
        "config",
    ));
    message.push_str("\n\n");

    // Usage
    message.push_str(&format!("## {}", strings::help::usage::heading()));
    message.push_str("\n\n");
    message.push_str(strings::help::usage::intro());
    message.push_str("\n\n");
    message.push_str(&strings::help::learn_more_send_a_command(
        bot.command_prefix(),
        "usage",
    ));

    bot.messaging()
        .send_text_markdown_no_fail(
            message_context.room(),
            message,
            MessageResponseType::Reply(message_context.thread_info().last_event_id.clone()),
        )
        .await;

    Ok(())
}
