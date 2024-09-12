pub fn heading() -> String {
    "🔒 Access".to_owned()
}

pub fn intro() -> String {
    "This bot employs access control to decide who can use its services and manage its configuration.".to_string()
}

pub fn room_auto_join_heading() -> String {
    "👋 Joining rooms".to_owned()
}

pub fn room_auto_join_intro() -> String {
    "The bot automatically joins rooms when invited by someone considered a bot user (see below)."
        .to_string()
}

pub fn users_heading() -> String {
    "👥 Users".to_owned()
}

pub fn users_intro() -> String {
    "The bot will ignore messages (and room invitations) from unallowed users.".to_string()
}

pub fn users_access() -> String {
    "Users can **use all the bot's features** (text-generation, speech-to-text, etc.), but cannot manage the bot's configuration.".to_string()
}

pub fn users_command_get(command_prefix: &str) -> String {
    format!("- **Show** the currently allowed users: `{command_prefix} access users`")
}

pub fn users_command_set(command_prefix: &str) -> String {
    format!("- **Set** the list of allowed users: `{command_prefix} access set-users SPACE_SEPARATED_PATTERNS`")
}

pub fn example_user_patterns(own_server_name: &str) -> String {
    format!("Example patterns: `@*:{own_server_name} @*:another.com @someone:company.org`")
}

pub fn administrators_heading() -> String {
    "👮‍♂️ Administrators".to_owned()
}

pub fn administrators_intro() -> String {
    "Administrators can **manage the bot's configuration and access control**.".to_string()
}

pub fn administrators_now_match_patterns(patterns: &[String]) -> String {
    format!(
        "The bot can be administrated by users with a [Matrix user id](https://spec.matrix.org/v1.11/#users) matching the following patterns: `{}`",
        patterns.join(" "),
    )
}

pub fn administrators_outro() -> String {
    "Administrators cannot be changed without adjusting the bot's configuration on the server."
        .to_string()
}

pub fn room_local_agent_managers_heading() -> String {
    "💼 Room-local agent managers".to_owned()
}

pub fn room_local_agent_managers_intro(command_prefix: &str) -> String {
    format!("Room-local agent managers are users privileged to **create their own agents** (see `{command_prefix} agent`) in rooms.")
}

pub fn room_local_agent_managers_security_warning() -> String {
    "Letting regular users create agents which contact arbitrary network services **may be a security issue**.".to_string()
}

pub fn room_local_agent_managers_command_get(command_prefix: &str) -> String {
    format!("- **Show** the currently allowed users: `{command_prefix} access room-local-agent-managers`")
}

pub fn room_local_agent_managers_command_set(command_prefix: &str) -> String {
    format!("- **Set** the list of allowed users: `{command_prefix} access set-room-local-agent-managers SPACE_SEPARATED_PATTERNS`")
}
