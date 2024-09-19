use mxlink::MessageResponseType;

use crate::{entity::MessageContext, strings, Bot};

pub async fn handle(bot: &Bot, message_context: &MessageContext) -> anyhow::Result<()> {
    let mut message = String::new();
    message.push_str(&build_section_intro());
    message.push_str("\n\n");
    message.push_str(&build_section_joining_rooms());
    message.push_str("\n\n");
    message.push_str(&build_section_users(
        bot.command_prefix(),
        bot.homeserver_name(),
        message_context,
    ));
    message.push_str("\n\n");
    message.push_str(&build_section_administrators(bot.admin_patterns()));
    message.push_str("\n\n");
    message.push_str(&build_section_room_local_agent_managers(
        bot.command_prefix(),
        bot.homeserver_name(),
        message_context,
    ));

    bot.messaging()
        .send_text_markdown_no_fail(
            message_context.room(),
            message,
            MessageResponseType::Reply(message_context.thread_info().root_event_id.clone()),
        )
        .await;

    Ok(())
}

fn build_section_intro() -> String {
    let mut message = String::new();

    message.push_str(&format!("## {}", strings::help::access::heading()));
    message.push_str("\n\n");
    message.push_str(&strings::help::access::intro());

    message
}

fn build_section_joining_rooms() -> String {
    let mut message = String::new();

    message.push_str(&format!(
        "### {}",
        strings::help::access::room_auto_join_heading()
    ));
    message.push_str("\n\n");
    message.push_str(&strings::help::access::room_auto_join_intro());
    message.push_str("\n\n");

    message
}

fn build_section_users(
    command_prefix: &str,
    homeserver_name: &str,
    message_context: &MessageContext,
) -> String {
    let mut message = String::new();

    message.push_str(&format!("### {}", strings::help::access::users_heading()));
    message.push_str("\n\n");
    message.push_str(&strings::help::access::users_intro());
    message.push('\n');
    message.push_str(&strings::help::access::users_access());
    message.push_str("\n\n");
    if let Some(user_patterns) = &message_context.global_config().access.user_patterns {
        if user_patterns.is_empty() {
            message.push_str(&strings::access::users_no_patterns());
        } else {
            message.push_str(&strings::access::users_now_match_patterns(user_patterns));
        }
    } else {
        message.push_str(&strings::access::users_no_patterns());
    }

    if message_context.sender_can_manage_global_config() {
        message.push_str("\n\n");

        message.push_str(strings::the_following_commands_are_available());
        message.push('\n');

        message.push_str(&strings::help::access::users_command_get(command_prefix));
        message.push('\n');

        message.push_str(&strings::help::access::users_command_set(command_prefix));
        message.push_str("\n\n");

        message.push_str(&strings::help::access::example_user_patterns(
            homeserver_name,
        ));
    }

    message
}

fn build_section_administrators(admin_patterns: &[String]) -> String {
    let mut message = String::new();

    message.push_str(&format!(
        "### {}",
        strings::help::access::administrators_heading()
    ));
    message.push_str("\n\n");
    message.push_str(&strings::help::access::administrators_intro());
    message.push_str("\n\n");
    message.push_str(&strings::help::access::administrators_now_match_patterns(
        admin_patterns,
    ));
    message.push_str("\n\n");
    message.push_str(&strings::help::access::administrators_outro());

    message
}

fn build_section_room_local_agent_managers(
    command_prefix: &str,
    homeserver_name: &str,
    message_context: &MessageContext,
) -> String {
    let mut message = String::new();

    message.push_str(&format!(
        "### {}",
        strings::help::access::room_local_agent_managers_heading()
    ));
    message.push_str("\n\n");
    message.push_str(&strings::help::access::room_local_agent_managers_intro(
        command_prefix,
    ));
    message.push('\n');
    message.push_str(&strings::help::access::room_local_agent_managers_security_warning());
    message.push_str("\n\n");
    if let Some(user_patterns) = &message_context
        .global_config()
        .access
        .room_local_agent_manager_patterns
    {
        if user_patterns.is_empty() {
            message.push_str(&strings::access::room_local_agent_managers_no_patterns());
        } else {
            message.push_str(
                &strings::access::room_local_agent_managers_now_match_patterns(user_patterns),
            );
        }
    } else {
        message.push_str(&strings::access::room_local_agent_managers_no_patterns());
    }

    if message_context.sender_can_manage_global_config() {
        message.push_str("\n\n");
        message.push_str(strings::the_following_commands_are_available());
        message.push('\n');

        message.push_str(
            &strings::help::access::room_local_agent_managers_command_get(command_prefix),
        );
        message.push('\n');

        message.push_str(
            &strings::help::access::room_local_agent_managers_command_set(command_prefix),
        );
        message.push_str("\n\n");

        message.push_str(&strings::help::access::example_user_patterns(
            homeserver_name,
        ));
    }

    message
}
