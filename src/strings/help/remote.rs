pub fn remote_management_heading() -> String {
    "🛰️ Managing rooms remotely".to_owned()
}

pub fn remote_management(command_prefix: &str) -> String {
    format!(
        "Rooms can also be managed from any other room where you can use the bot (for example, a private room with the bot), with `config`, `agent` and `access` commands:\n- **List** the rooms you can manage: `{command_prefix} room list`\n- **Run** a command in another room: `{command_prefix} in ROOM_ID_OR_ALIAS COMMAND`, for example `{command_prefix} in #room:example.com config status`"
    )
}
