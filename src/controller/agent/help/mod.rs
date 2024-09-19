use mxlink::MessageResponseType;

use crate::{entity::MessageContext, strings, Bot};

pub async fn handle(bot: &Bot, message_context: &MessageContext) -> anyhow::Result<()> {
    // Anyone can access this help command, because certain subcommands ("list")
    // are also useful to regular users and it'd be great for them to learn about them.

    let mut message = String::new();

    let can_manage_agents = message_context.sender_can_manage_room_local_agents()?;

    message.push_str(&format!("## {}", strings::help::agent::heading()));
    message.push_str("\n\n");
    message.push_str(&strings::help::agent::intro(bot.command_prefix()));
    message.push('\n');
    message.push_str(&strings::help::agent::intro_capabilities());
    message.push_str("\n\n");
    message.push_str(&strings::help::agent::intro_handler_relation(
        bot.command_prefix(),
    ));

    if can_manage_agents {
        message.push_str("\n\n");

        message.push_str(strings::help::available_commands_intro());
        message.push('\n');

        message.push_str(&strings::help::agent::list_agents(bot.command_prefix()));
        message.push('\n');

        message.push_str(strings::help::agent::create_agent_intro());
        message.push('\n');
        message.push_str(&strings::help::agent::create_agent_room_local(
            bot.command_prefix(),
        ));
        message.push('\n');

        if message_context.sender_can_manage_global_config() {
            message.push_str(&strings::help::agent::create_agent_global(
                bot.command_prefix(),
            ));
            message.push('\n');
        }

        message.push_str(&strings::help::agent::create_agent_example(
            bot.command_prefix(),
        ));
        message.push('\n');

        message.push_str(&strings::help::agent::show_agent_details(
            bot.command_prefix(),
        ));
        message.push('\n');

        message.push_str(&strings::help::agent::delete_agent(bot.command_prefix()));

        message.push_str("\n\n");

        message.push_str(strings::help::agent::available_commands_outro_update_note());
    } else {
        message.push_str("\n\n");

        message.push_str(strings::help::agent::no_permission_to_create_agents());
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
