pub fn missing_command(command_prefix: &str) -> String {
    format!(
        "Specify a command to run in the room, for example: `{command_prefix} in !room-id:example.com config status`"
    )
}

pub fn unsupported_command() -> &'static str {
    "Only `config`, `agent` and `access` commands can be run in another room."
}

pub fn nested_not_supported() -> &'static str {
    "Commands for another room cannot be nested."
}

pub fn alias_not_resolved(alias: &str) -> String {
    format!("The room alias `{alias}` could not be resolved.")
}

pub fn bot_not_in_room(room: &str) -> String {
    format!("The bot is not joined to the room `{room}`.")
}

pub fn cannot_manage_room(room: &str) -> String {
    format!(
        "You cannot manage {} from here: the bot is not in that room, or you are not a member of it.",
        crate::utils::text::inline_code(room)
    )
}

pub fn unknown_room_command(command_prefix: &str) -> String {
    format!(
        "Unknown room command. To list the rooms you can manage, send `{command_prefix} room list`."
    )
}

pub fn room_list_none() -> &'static str {
    "There are no rooms you can manage."
}

pub fn room_list_intro() -> &'static str {
    "The bot is joined to these rooms:"
}

pub fn room_list_intro_non_admin() -> &'static str {
    "You can manage these rooms (you are a member and have a management permission there):"
}

pub fn room_list_how_to_manage(command_prefix: &str) -> String {
    format!(
        "💡 To manage one of them from here, send `{command_prefix} in <room-id-or-alias> <command>`, for example `{command_prefix} in !room-id:example.com config status`."
    )
}

pub fn room_list_joined_members(count: u64) -> String {
    if count == 1 {
        "1 member".to_owned()
    } else {
        format!("{count} members")
    }
}
