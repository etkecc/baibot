use crate::utils::text::inline_code;

pub fn failed_to_parse_patterns(err: &str) -> String {
    format!(
        "Failed to parse user patterns: {}",
        crate::utils::text::escape_markdown(err)
    )
}

pub fn missing_permission(emoji: &str, permission_name: &str) -> String {
    format!(
        "You are not allowed to do this here: it requires the {emoji} `{permission_name}` permission."
    )
}

pub fn legacy_command_replaced(command_prefix: &str) -> String {
    format!(
        "Access is now managed with rules instead of user lists. See `{command_prefix} access` for how it works, and `{command_prefix} access explain` for your own permissions."
    )
}

pub fn rule_command_usage(command_prefix: &str) -> String {
    format!(
        "Usage: `{command_prefix} access global|room allow|deny PERMISSIONS USER_PATTERNS [min-power-level=N]` or `{command_prefix} access global|room remove RULE_ID`. Send `{command_prefix} access` for help."
    )
}

pub fn invalid_rule_id(command_prefix: &str, id: &str) -> String {
    format!(
        "{} is not a rule id. Rule ids are shown by `{command_prefix} access list`.",
        inline_code(id)
    )
}

pub fn unknown_permission(command_prefix: &str, name: &str) -> String {
    format!(
        "{} is not a known permission. Send `{command_prefix} access` for the list of permissions.",
        inline_code(name)
    )
}

pub fn unknown_tool(name: &str, known_tool_selectors: &[String]) -> String {
    let known = known_tool_selectors
        .iter()
        .map(|selector| inline_code(selector))
        .collect::<Vec<_>>()
        .join(", ");

    format!(
        "{} doesn't match any tool. Known tool permissions: {known}.",
        inline_code(name)
    )
}

pub fn invite_without_power_level() -> String {
    "The `invite` permission cannot have a power-level condition, because invitations happen before the bot is in a room.".to_owned()
}

pub fn duplicate_min_power_level() -> String {
    "Specify `min-power-level=N` at most once.".to_owned()
}

pub fn explain_takes_one_user() -> String {
    "Specify at most one user to explain the permissions of.".to_owned()
}

pub fn invite_only_in_global_rules() -> String {
    "The `invite` permission can only be granted by global rules, because invitations happen before the bot is in a room.".to_owned()
}

pub fn invalid_min_power_level(value: &str) -> String {
    format!("{} is not a valid power level.", inline_code(value))
}

pub fn invalid_user_id(user_id: &str) -> String {
    format!(
        "{} is not a valid Matrix user ID (like `@someone:example.com`).",
        inline_code(user_id)
    )
}

pub fn not_delegable() -> String {
    "You can only add or remove rules in this room which consist of feature permissions (`use`, `text-generation`, `speech-to-text`, `text-to-speech`, `image-generation`) that you have yourself. Other rules (including tool rules) are managed by administrators.".to_owned()
}

pub fn rule_not_found(command_prefix: &str, id: &str) -> String {
    format!(
        "There is no rule {}. See `{command_prefix} access list` for the current rules.",
        inline_code(id)
    )
}

pub fn rule_added(scope: &str, rule: &str) -> String {
    format!("Added {scope} rule {rule}.")
}

pub fn rule_removed(scope: &str, rule: &str) -> String {
    format!("Removed {scope} rule {rule}.")
}

pub fn global_rules_heading() -> &'static str {
    "Global rules"
}

pub fn room_rules_heading() -> &'static str {
    "Rules for this room"
}

pub fn no_rules() -> &'static str {
    "No rules."
}

pub fn rules_evaluation_note() -> &'static str {
    "💡 Rule order doesn't matter. A permission is granted if a rule allows it and no rule denies it. Built-in tools are allowed by default when you have the feature permissions they need. A deny rule turns a tool off. Administrators can do anything regardless, but tool rules apply to them too."
}

pub fn explain_admin(user_id: &str) -> String {
    format!(
        "{} is an administrator and can do anything, but tool rules apply to them too.",
        inline_code(user_id)
    )
}

pub fn explain_tools_heading() -> &'static str {
    "🧰 Bot tool permissions (using tools also needs 💬 `text-generation`):"
}

pub fn explain_tool_enabled_by_default(emoji: &str, selector: &str, summary: &str) -> String {
    format!("- ✅ {emoji} `{selector}` ({summary}): allowed by default")
}

pub fn explain_tool_allowed(emoji: &str, selector: &str, summary: &str, rules: &str) -> String {
    format!("- ✅ {emoji} `{selector}` ({summary}): allowed by {rules}")
}

pub fn explain_tool_denied(emoji: &str, selector: &str, summary: &str, rules: &str) -> String {
    format!("- 🚫 {emoji} `{selector}` ({summary}): denied by {rules}")
}

pub fn explain_tool_not_allowed(emoji: &str, selector: &str, summary: &str) -> String {
    format!("- ❌ {emoji} `{selector}` ({summary}): no rule allows it")
}

pub fn explain_tool_needs_permission_suffix(
    emoji: &str,
    permission: &str,
    allowed: bool,
) -> String {
    if allowed {
        format!(" (with {emoji} `{permission}`, which is allowed)")
    } else {
        format!(" (needs {emoji} `{permission}`, which is not allowed)")
    }
}

pub fn inactive_tool_selector_suffix() -> &'static str {
    " (no such tool, ignored)"
}

pub fn explain_heading(user_id: &str) -> String {
    format!("Permissions of {} here:", inline_code(user_id))
}

pub fn explain_allowed(emoji: &str, permission: &str, rules: &str) -> String {
    format!("- ✅ {emoji} `{permission}`: allowed by {rules}")
}

pub fn explain_denied(emoji: &str, permission: &str, rules: &str) -> String {
    format!("- 🚫 {emoji} `{permission}`: denied by {rules}")
}

pub fn explain_not_allowed(emoji: &str, permission: &str) -> String {
    format!("- ❌ {emoji} `{permission}`: no rule allows it")
}
