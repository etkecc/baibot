pub fn users_no_patterns() -> String {
    "No user patterns are configured, so the bot can only be used by administrators.".to_owned()
}

pub fn users_now_match_patterns(patterns: &[String]) -> String {
    format!(
        "The bot can be used by users with a [Matrix user id](https://spec.matrix.org/v1.11/#users) matching the following patterns: `{}`",
        patterns.join(" "),
    )
}

pub fn room_local_agent_managers_no_patterns() -> String {
    "No room-local agent manager patterns are configured, so new agents can only be created by administrators.".to_owned()
}

pub fn room_local_agent_managers_now_match_patterns(patterns: &[String]) -> String {
    format!(
        "The bot allows users with a [Matrix user id](https://spec.matrix.org/v1.11/#users) matching the following patterns to manage agents: `{}`",
        patterns.join(" "),
    )
}

pub fn failed_to_parse_patterns(err: &str) -> String {
    format!("Failed to parse patterns: {}", err)
}
