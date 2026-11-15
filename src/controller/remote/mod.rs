//! Managing a room from another room: `in <room-id-or-alias> <command>`, and listing the rooms that can be managed.

#[cfg(test)]
mod tests;

use mxlink::MessageResponseType;
use mxlink::matrix_sdk::ruma::{OwnedRoomOrAliasId, RoomAliasId, RoomId, RoomOrAliasId};
use mxlink::matrix_sdk::{Room, RoomState};

use crate::authorization::authorizer_for_room;
use crate::entity::roomconfig::TextGenerationPrefixRequirementType;
use crate::entity::{MessageContext, RoomConfigContext};
use crate::{Bot, strings};

use super::ControllerType;

/// Parses `<command_prefix> in <room-id-or-alias> <command>`.
///
/// Returns `None` if the text is not such a command, so that messages like `!bai in which year ...`
/// still reach chat completion. The command is only recognized if the word after `in` is a valid room ID or alias.
pub fn determine_controller(command_prefix: &str, text: &str) -> Option<ControllerType> {
    let (target, command) = parse_target(command_prefix, text)?;

    if command.is_empty() {
        return Some(ControllerType::Error(strings::remote::missing_command(
            command_prefix,
        )));
    }

    let command = format!("{command_prefix} {command}");

    // Nesting is not supported. It is detected without parsing recursively, so deep nesting cannot exhaust the stack.
    let command = if parse_target(command_prefix, &command).is_some() {
        ControllerType::Error(strings::remote::nested_not_supported().to_owned())
    } else {
        super::determination::determine_text_controller(
            command_prefix,
            &command,
            TextGenerationPrefixRequirementType::CommandPrefix,
            false,
        )
    };

    Some(ControllerType::InRoom(target, Box::new(command)))
}

/// Splits `<command_prefix> in <room-id-or-alias> <command>` into the target and the (trimmed) command.
fn parse_target<'a>(command_prefix: &str, text: &'a str) -> Option<(OwnedRoomOrAliasId, &'a str)> {
    let remaining = text
        .strip_prefix(command_prefix)?
        .strip_prefix(char::is_whitespace)?
        .trim_start()
        .strip_prefix("in")?
        .strip_prefix(char::is_whitespace)?
        .trim_start();

    let (target, command) = remaining
        .split_once(char::is_whitespace)
        .unwrap_or((remaining, ""));

    let target = OwnedRoomOrAliasId::try_from(target).ok()?;

    Some((target, command.trim()))
}

fn is_supported_remotely(controller_type: &ControllerType) -> bool {
    matches!(
        controller_type,
        ControllerType::Config(_)
            | ControllerType::Agent(_)
            | ControllerType::Access(_)
            // Errors (like a missing argument) and unknown commands are passed through, so their message is shown.
            | ControllerType::Error(_)
            | ControllerType::Unknown
    )
}

pub async fn handle_in_room(
    bot: &Bot,
    message_context: &MessageContext,
    target: &RoomOrAliasId,
    command: &ControllerType,
) -> anyhow::Result<()> {
    if !is_supported_remotely(command) {
        reply_error(bot, message_context, strings::remote::unsupported_command()).await;
        return Ok(());
    }

    let target_room = match resolve_target(bot, target).await {
        Ok(room) => room,
        Err(message) => {
            reply_error(bot, message_context, &message).await;
            return Ok(());
        }
    };

    let room_config = bot
        .room_config_manager()
        .lock()
        .await
        .get_or_create_for_room(&target_room)
        .await?;

    let authorizer = authorizer_for_room(
        bot.admin_pattern_regexes(),
        message_context.global_config(),
        &room_config,
        &target_room,
    )
    .await;

    let room_config_context =
        RoomConfigContext::new(message_context.global_config().clone(), room_config);

    let message_context = message_context.retargeted(target_room, room_config_context, authorizer);

    tracing::info!(
        target_room_id = message_context.target_room().room_id().as_str(),
        "Dispatching command for another room"
    );

    // Checks the command's own permission (against the target room) and dispatches it.
    Box::pin(super::dispatch_controller(command, &message_context, bot)).await;

    Ok(())
}

/// Finds the joined room for a room ID or alias, or returns a message explaining why it cannot be used.
async fn resolve_target(bot: &Bot, target: &RoomOrAliasId) -> Result<Room, String> {
    let client = bot.matrix_link().client();

    let room_id = match <&RoomId>::try_from(target) {
        Ok(room_id) => room_id.to_owned(),
        Err(alias) => {
            let alias: &RoomAliasId = alias;

            // Prefer the aliases of joined rooms (known locally), to avoid asking the server.
            let joined_room = client.joined_rooms().into_iter().find(|room| {
                room.canonical_alias().as_deref() == Some(alias)
                    || room.alt_aliases().iter().any(|a| a == alias)
            });
            if let Some(room) = joined_room {
                return Ok(room);
            }

            match client.resolve_room_alias(alias).await {
                Ok(response) => response.room_id,
                Err(err) => {
                    tracing::debug!(?alias, ?err, "Failed to resolve room alias");
                    return Err(strings::remote::alias_not_resolved(alias.as_str()));
                }
            }
        }
    };

    match client.get_room(&room_id) {
        Some(room) if room.state() == RoomState::Joined => Ok(room),
        _ => Err(strings::remote::bot_not_in_room(target.as_str())),
    }
}

pub async fn handle_room_list(bot: &Bot, message_context: &MessageContext) -> anyhow::Result<()> {
    let mut lines = Vec::new();

    for room in bot.matrix_link().client().joined_rooms() {
        let name = match room.display_name().await {
            Ok(name) => name.to_string(),
            Err(_) => room.room_id().to_string(),
        };

        lines.push((
            name.to_lowercase(),
            room_line(
                &name,
                room.room_id(),
                room.canonical_alias().as_deref(),
                room.joined_members_count(),
            ),
        ));
    }

    let message = if lines.is_empty() {
        strings::remote::room_list_none().to_owned()
    } else {
        lines.sort();

        let mut message = strings::remote::room_list_intro(bot.command_prefix());
        message.push_str("\n\n");
        for (_, line) in lines {
            message.push_str(&line);
            message.push('\n');
        }
        message
    };

    bot.messaging()
        .send_notice_markdown_no_fail(
            message_context.room(),
            message,
            MessageResponseType::Reply(message_context.thread_info().root_event_id.clone()),
        )
        .await;

    Ok(())
}

fn room_line(
    name: &str,
    room_id: &RoomId,
    alias: Option<&RoomAliasId>,
    joined_members_count: u64,
) -> String {
    let mut line = format!(
        "- **{}**: `{room_id}`",
        crate::utils::text::escape_markdown(name)
    );

    if let Some(alias) = alias {
        line.push_str(&format!(", `{alias}`"));
    }

    line.push_str(&format!(
        " ({})",
        strings::remote::room_list_joined_members(joined_members_count)
    ));

    line
}

async fn reply_error(bot: &Bot, message_context: &MessageContext, message: &str) {
    bot.messaging()
        .send_error_markdown_no_fail(
            message_context.room(),
            message,
            MessageResponseType::Reply(message_context.thread_info().root_event_id.clone()),
        )
        .await;
}
