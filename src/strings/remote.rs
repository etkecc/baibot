pub fn only_administrators() -> &'static str {
    "Only administrators can manage other rooms."
}

pub fn missing_command(command_prefix: &str) -> String {
    format!(
        "Specify a command to run in the room, for example: `{command_prefix} in !room-id:example.com config status`"
    )
}

pub fn unsupported_command() -> &'static str {
    "Only `config` and `agent` commands can be run in another room."
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

pub fn room_list_none() -> &'static str {
    "The bot is not joined to any rooms."
}

pub fn room_list_intro(command_prefix: &str) -> String {
    format!(
        "The bot is joined to these rooms. To manage one of them from here, send `{command_prefix} in <room-id-or-alias> <command>`, for example `{command_prefix} in !room-id:example.com config status`."
    )
}

pub fn room_list_joined_members(count: u64) -> String {
    if count == 1 {
        "1 member".to_owned()
    } else {
        format!("{count} members")
    }
}
