use mxlink::MessageResponseType;

use crate::{Bot, authorization::Permission, entity::MessageContext, strings};

pub async fn handle(bot: &Bot, message_context: &MessageContext) -> anyhow::Result<()> {
    let command_prefix = bot.command_prefix();

    let sections = [
        build_section_intro(),
        build_section_permissions(),
        build_section_rules(command_prefix, bot.homeserver_name(), message_context),
        build_section_joining_rooms(),
        build_section_administrators(bot.admin_patterns()),
    ];

    bot.messaging()
        .send_text_markdown_no_fail(
            message_context.room(),
            sections.join("\n\n"),
            MessageResponseType::Reply(message_context.thread_info().root_event_id.clone()),
        )
        .await;

    Ok(())
}

fn build_section_intro() -> String {
    format!(
        "## {}\n\n{}",
        strings::help::access::heading(),
        strings::help::access::intro()
    )
}

fn build_section_permissions() -> String {
    format!(
        "### {}\n\n{}\n\n{}\n\n{}",
        strings::help::access::permissions_heading(),
        strings::help::access::permissions_list(),
        strings::help::access::permissions_outro(),
        strings::help::access::agent_management_security_warning(),
    )
}

fn build_section_rules(
    command_prefix: &str,
    homeserver_name: &str,
    message_context: &MessageContext,
) -> String {
    let mut message = String::new();

    message.push_str(&format!("### {}", strings::help::access::rules_heading()));
    message.push_str("\n\n");
    message.push_str(&strings::help::access::rules_intro());
    message.push_str("\n\n");

    message.push_str(strings::the_following_commands_are_available());
    message.push('\n');
    message.push_str(&strings::help::access::rules_command_explain(
        command_prefix,
    ));

    if message_context.sender_can(Permission::ManageAccess) {
        message.push('\n');
        message.push_str(&strings::help::access::rules_command_list(command_prefix));
        message.push('\n');
        message.push_str(&strings::help::access::rules_command_room(command_prefix));

        if message_context.sender_is_admin() {
            message.push('\n');
            message.push_str(&strings::help::access::rules_command_global(command_prefix));
        } else {
            message.push_str("\n\n");
            message.push_str(&strings::help::access::rules_room_delegation_note());
        }

        message.push_str("\n\n");
        message.push_str(&strings::help::access::rules_examples(
            command_prefix,
            homeserver_name,
        ));
    }

    message
}

fn build_section_joining_rooms() -> String {
    format!(
        "### {}\n\n{}",
        strings::help::access::room_auto_join_heading(),
        strings::help::access::room_auto_join_intro()
    )
}

fn build_section_administrators(admin_patterns: &[String]) -> String {
    format!(
        "### {}\n\n{}\n\n{}\n\n{}",
        strings::help::access::administrators_heading(),
        strings::help::access::administrators_intro(),
        strings::help::access::administrators_now_match_patterns(admin_patterns),
        strings::help::access::administrators_outro(),
    )
}
