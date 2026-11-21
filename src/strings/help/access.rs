pub fn heading() -> String {
    "🔒 Access".to_owned()
}

pub fn intro() -> String {
    "This bot uses access rules to decide who can use its features and manage its configuration, globally and in each room.".to_string()
}

pub fn permissions_heading() -> String {
    "🔑 Permissions".to_owned()
}

pub fn permissions_list() -> String {
    [
        "- 💬 `text-generation`: chat with the bot (including sending it images and files)",
        "- 🦻 `speech-to-text`: have voice messages transcribed",
        "- 🗣️ `text-to-speech`: have messages read aloud",
        "- 🖌️ `image-generation`: generate and edit images, and generate stickers",
        "- 🌟 `use`: all of the above (and features added to it in the future)",
        "- 🛠️ `manage-config`: view and change the room's settings (`config room ...`, `config status`)",
        "- 🤖 `manage-agents`: create, inspect and delete agents in the room",
        "- 🔒 `manage-access`: view and change the room's access rules",
        "- 👋 `invite`: invite the bot into rooms (only granted by global rules)",
    ]
    .join("\n")
}

pub fn permissions_outro() -> String {
    "Anyone with any permission in a room (except 👋 `invite`) can talk to the bot there, see help and list agents. Messages from others are ignored.".to_owned()
}

pub fn rules_heading() -> String {
    "📜 Rules".to_owned()
}

pub fn rules_intro() -> String {
    "Rules are either global (apply in all rooms) or room rules (apply in one room). Each rule allows or denies some permissions to users matching [Matrix user ID](https://spec.matrix.org/v1.11/#users) patterns, optionally only if their power level in the room is at least a given value. A permission is granted if a rule allows it and no rule denies it, so a deny always wins.".to_string()
}

pub fn rules_command_explain(command_prefix: &str) -> String {
    format!(
        "- **Explain** your permissions here: `{command_prefix} access explain` (with 🔒 `manage-access`, also for others: `{command_prefix} access explain @someone:example.com`)"
    )
}

pub fn rules_command_list(command_prefix: &str) -> String {
    format!("- **List** the global rules and this room's rules: `{command_prefix} access list`")
}

pub fn rules_command_room(command_prefix: &str) -> String {
    format!(
        "- **Add** a room rule: `{command_prefix} access room allow|deny PERMISSIONS USER_PATTERNS [min-power-level=N]`\n- **Remove** a room rule: `{command_prefix} access room remove RULE_ID`"
    )
}

pub fn rules_room_delegation_note() -> String {
    "Users with 🔒 `manage-access` (other than administrators) can only add or remove room rules made of feature permissions they have themselves.".to_owned()
}

pub fn rules_command_global(command_prefix: &str) -> String {
    format!(
        "- **Add** a global rule: `{command_prefix} access global allow|deny PERMISSIONS USER_PATTERNS [min-power-level=N]`\n- **Remove** a global rule: `{command_prefix} access global remove RULE_ID`"
    )
}

pub fn rules_examples(command_prefix: &str, own_server_name: &str) -> String {
    format!(
        "Examples (separate several permissions or user patterns with commas or spaces):\n- `{command_prefix} access room allow speech-to-text @*:bridge.{own_server_name}`\n- `{command_prefix} access room allow manage-config @*:{own_server_name} min-power-level=50`\n- `{command_prefix} access room deny image-generation @someone:{own_server_name}`"
    )
}

pub fn room_auto_join_heading() -> String {
    "👋 Joining rooms".to_owned()
}

pub fn room_auto_join_intro() -> String {
    "The bot automatically joins rooms when invited by someone with the 👋 `invite` permission."
        .to_string()
}

pub fn administrators_heading() -> String {
    "👮‍♂️ Administrators".to_owned()
}

pub fn administrators_intro() -> String {
    "Administrators can **do anything**, in any room, regardless of rules. Only they can manage the global configuration, global agents and global rules.".to_string()
}

pub fn administrators_now_match_patterns(patterns: &[String]) -> String {
    format!(
        "The bot can be administrated by users with a [Matrix user ID](https://spec.matrix.org/v1.11/#users) matching the following patterns: `{}`",
        patterns.join(" "),
    )
}

pub fn administrators_outro() -> String {
    "Administrators cannot be changed without adjusting the bot's configuration on the server."
        .to_string()
}

pub fn agent_management_security_warning() -> String {
    "Letting regular users create agents (🤖 `manage-agents`) which contact arbitrary network services **may be a security issue**.".to_string()
}
