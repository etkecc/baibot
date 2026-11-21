use mxlink::MessageResponseType;
use mxlink::matrix_sdk::ruma::UserId;

use super::determination::{NewRule, RuleCommandScope, can_delegate};
use crate::authorization::rule::{
    TOOL_PREFIX, is_inactive_tool_selector, new_rule_id, permission_name_emoji,
};
use crate::authorization::{
    AccessRule, Authorizer, Permission, RuleEffect, RuleScope, room_power_levels,
};
use crate::utils::text::{escape_markdown, inline_code};
use crate::{
    Bot,
    entity::{MessageContext, globalconfig, roomconfig},
    strings,
};

enum Outcome {
    Added(AccessRule),
    Removed(AccessRule),
    NotFound(String),
    /// The sender may not add or remove this rule (any longer).
    NotDelegable,
}

impl Outcome {
    fn changed(&self) -> bool {
        matches!(self, Self::Added(_) | Self::Removed(_))
    }
}

pub async fn handle_list(bot: &Bot, message_context: &MessageContext) -> anyhow::Result<()> {
    let access = &message_context.global_config().access;

    let mut message = String::new();

    message.push_str(&format!(
        "**{}**\n",
        strings::access::global_rules_heading()
    ));
    message.push_str(&format_rules(access.rules.as_deref().unwrap_or_default()));

    message.push_str(&format!(
        "\n\n**{}**\n",
        strings::access::room_rules_heading()
    ));
    message.push_str(&format_rules(&message_context.room_config().access.rules));

    message.push_str("\n\n");
    message.push_str(strings::access::rules_evaluation_note());

    reply(bot, message_context, &message, false).await;

    Ok(())
}

pub async fn handle_add(
    bot: &Bot,
    message_context: &MessageContext,
    scope: RuleCommandScope,
    new_rule: &NewRule,
) -> anyhow::Result<()> {
    let sender_id = message_context.sender_id();

    let outcome = update_rules(bot, message_context, scope, |rules, authorizer| {
        if !may_change_rule(authorizer, sender_id, scope, &new_rule.permissions) {
            return Outcome::NotDelegable;
        }

        let rule = AccessRule {
            id: new_rule_id(rules),
            effect: new_rule.effect.clone(),
            permissions: new_rule.permissions.clone(),
            users: new_rule.users.clone(),
            min_power_level: new_rule.min_power_level,
        };
        rules.push(rule.clone());
        Outcome::Added(rule)
    })
    .await?;

    report(bot, message_context, scope, outcome).await;

    Ok(())
}

pub async fn handle_remove(
    bot: &Bot,
    message_context: &MessageContext,
    scope: RuleCommandScope,
    id: &str,
) -> anyhow::Result<()> {
    let sender_id = message_context.sender_id();

    let outcome = update_rules(bot, message_context, scope, |rules, authorizer| {
        let Some(index) = rules.iter().position(|rule| rule.id == id) else {
            return Outcome::NotFound(id.to_owned());
        };

        // Removing a deny rule grants as much as adding an allow rule, so the same limits apply.
        if !may_change_rule(authorizer, sender_id, scope, &rules[index].permissions) {
            return Outcome::NotDelegable;
        }

        Outcome::Removed(rules.remove(index))
    })
    .await?;

    report(bot, message_context, scope, outcome).await;

    Ok(())
}

/// Tells whether the sender may add or remove a rule with the given permissions.
///
/// The authorizer reflects the current configuration (see `update_rules`),
/// so permissions taken away since the command was sent are respected.
fn may_change_rule(
    authorizer: &Authorizer,
    sender_id: &UserId,
    scope: RuleCommandScope,
    permissions: &[String],
) -> bool {
    if authorizer.is_admin(sender_id) {
        return true;
    }

    match scope {
        RuleCommandScope::Global => false,
        RuleCommandScope::Room => {
            authorizer.allows(sender_id, Permission::ManageAccess)
                && can_delegate(permissions, |permission| {
                    authorizer.allows(sender_id, permission)
                })
        }
    }
}

pub async fn handle_explain(
    bot: &Bot,
    message_context: &MessageContext,
    user_id: &Option<String>,
) -> anyhow::Result<()> {
    let user_id = match user_id {
        None => message_context.sender_id().to_owned(),
        Some(user_id) => match UserId::parse(user_id.as_str()) {
            Ok(user_id) => user_id,
            Err(_) => {
                reply(
                    bot,
                    message_context,
                    &strings::access::invalid_user_id(user_id),
                    true,
                )
                .await;
                return Ok(());
            }
        },
    };

    let authorizer = message_context.authorizer();

    let mut message = String::new();

    if authorizer.is_admin(&user_id) {
        // Administrators can do anything, except that tools follow the rules for them too, so only tools need explaining.
        message.push_str(&strings::access::explain_admin(user_id.as_str()));
        message.push_str("\n\n");
        message.push_str(&explain_tools(authorizer, &user_id));
        reply(bot, message_context, &message, false).await;
        return Ok(());
    }

    message.push_str(&strings::access::explain_heading(user_id.as_str()));
    message.push('\n');

    // Invitations are decided before the bot is in the room, so only with global rules
    // and without power levels. Explain them the same way.
    let invite_authorizer =
        Authorizer::new(bot.admin_pattern_regexes(), message_context.global_config());

    for permission in Permission::ROOM.into_iter().chain([Permission::Invite]) {
        let authorizer = if permission == Permission::Invite {
            &invite_authorizer
        } else {
            authorizer
        };

        let allowed_by = authorizer.matching_rules(&user_id, permission, RuleEffect::Allow);
        let denied_by = authorizer.matching_rules(&user_id, permission, RuleEffect::Deny);

        let line = if !denied_by.is_empty() {
            strings::access::explain_denied(
                permission.emoji(),
                permission.name(),
                &format_rule_refs(&denied_by),
            )
        } else if !allowed_by.is_empty() {
            strings::access::explain_allowed(
                permission.emoji(),
                permission.name(),
                &format_rule_refs(&allowed_by),
            )
        } else {
            strings::access::explain_not_allowed(permission.emoji(), permission.name())
        };

        message.push_str(&line);
        message.push('\n');
    }

    message.push('\n');
    message.push_str(&explain_tools(authorizer, &user_id));
    message.push_str("\n\n");
    message.push_str(strings::access::rules_evaluation_note());

    reply(bot, message_context, &message, false).await;

    Ok(())
}

/// Explains which of the known tools the user may use in the room, and why.
fn explain_tools(authorizer: &Authorizer, user_id: &UserId) -> String {
    let mut lines = vec![strings::access::explain_tools_heading().to_owned()];

    for tool in crate::tools::catalog::known() {
        let selector = format!("{TOOL_PREFIX}{}", tool.id);

        let denied_by = authorizer.matching_tool_rules(user_id, tool.id, RuleEffect::Deny);
        let allowed_by = authorizer.matching_tool_rules(user_id, tool.id, RuleEffect::Allow);

        lines.push(if !denied_by.is_empty() {
            strings::access::explain_tool_denied(
                tool.emoji,
                &selector,
                tool.summary,
                &format_rule_refs(&denied_by),
            )
        } else if !allowed_by.is_empty() {
            strings::access::explain_tool_allowed(
                tool.emoji,
                &selector,
                tool.summary,
                &format_rule_refs(&allowed_by),
            )
        } else if tool.default_enabled {
            strings::access::explain_tool_enabled_by_default(tool.emoji, &selector, tool.summary)
        } else {
            strings::access::explain_tool_not_allowed(tool.emoji, &selector, tool.summary)
        });
    }

    lines.join("\n")
}

/// Changes the global rules or the target room's rules, persisting only if `change` changed them.
///
/// `change` gets an authorizer built from the current configuration (read while holding the lock),
/// not the one from when the command was sent, so that it can check the sender's permissions against
/// the very rules it changes.
async fn update_rules(
    bot: &Bot,
    message_context: &MessageContext,
    scope: RuleCommandScope,
    change: impl FnOnce(&mut Vec<AccessRule>, &Authorizer) -> Outcome,
) -> anyhow::Result<Outcome> {
    let mut unchanged = None;

    let changed = match scope {
        RuleCommandScope::Global => {
            globalconfig::update(bot.global_config_manager(), |global_config| {
                // Global rules are only changed by administrators, which the global rules don't affect.
                let authorizer = Authorizer::new(bot.admin_pattern_regexes(), global_config);

                let rules = global_config.access.rules.get_or_insert_with(Vec::new);

                let outcome = change(rules, &authorizer);
                if outcome.changed() {
                    Some(outcome)
                } else {
                    unchanged = Some(outcome);
                    None
                }
            })
            .await?
        }
        RuleCommandScope::Room => {
            let target_room = message_context.target_room();
            let global_config = bot.global_config().await?;
            let power_levels = room_power_levels(target_room).await;

            roomconfig::update(bot.room_config_manager(), target_room, |room_config| {
                let authorizer = Authorizer::new(bot.admin_pattern_regexes(), &global_config)
                    .for_room(&room_config.access.rules, power_levels);

                let outcome = change(&mut room_config.access.rules, &authorizer);
                if outcome.changed() {
                    Some(outcome)
                } else {
                    unchanged = Some(outcome);
                    None
                }
            })
            .await?
        }
    };

    changed
        .or(unchanged)
        .ok_or_else(|| anyhow::anyhow!("Rule change produced no outcome"))
}

async fn report(
    bot: &Bot,
    message_context: &MessageContext,
    scope: RuleCommandScope,
    outcome: Outcome,
) {
    let scope = match scope {
        RuleCommandScope::Global => RuleScope::Global,
        RuleCommandScope::Room => RuleScope::Room,
    };

    let (message, is_error) = match outcome {
        Outcome::Added(rule) => (
            strings::access::rule_added(scope_name(scope), &format_rule(&rule)),
            false,
        ),
        Outcome::Removed(rule) => (
            strings::access::rule_removed(scope_name(scope), &format_rule(&rule)),
            false,
        ),
        Outcome::NotFound(id) => (
            strings::access::rule_not_found(
                &message_context.command_prefix_for_target_room(bot.command_prefix()),
                &id,
            ),
            true,
        ),
        Outcome::NotDelegable => (strings::access::not_delegable(), true),
    };

    reply(bot, message_context, &message, is_error).await;
}

async fn reply(bot: &Bot, message_context: &MessageContext, message: &str, is_error: bool) {
    let response_type =
        MessageResponseType::Reply(message_context.thread_info().root_event_id.clone());

    if is_error {
        bot.messaging()
            .send_error_markdown_no_fail(message_context.room(), message, response_type)
            .await;
    } else {
        bot.messaging()
            .send_text_markdown_no_fail(message_context.room(), message.to_owned(), response_type)
            .await;
    }
}

fn scope_name(scope: RuleScope) -> &'static str {
    match scope {
        RuleScope::Global => "global",
        RuleScope::Room => "room",
    }
}

fn format_rule_refs(refs: &[(RuleScope, String)]) -> String {
    refs.iter()
        .map(|(scope, id)| format!("{} rule {}", scope_name(*scope), inline_code(id)))
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_rules(rules: &[AccessRule]) -> String {
    if rules.is_empty() {
        return strings::access::no_rules().to_owned();
    }

    rules
        .iter()
        .map(|rule| format!("- {}", format_rule(rule)))
        .collect::<Vec<_>>()
        .join("\n")
}

pub(super) fn format_rule(rule: &AccessRule) -> String {
    let (marker, effect) = match &rule.effect {
        RuleEffect::Allow => ("✅", "allow".to_owned()),
        RuleEffect::Deny => ("🚫", "deny".to_owned()),
        RuleEffect::Unknown(value) => (
            "❔",
            format!("{} (unknown, ignored)", escape_markdown(value)),
        ),
    };

    let permissions = rule
        .permissions
        .iter()
        .map(|name| {
            let mut text = match permission_name_emoji(name) {
                Some(emoji) => format!("{emoji} {}", inline_code(name)),
                None => inline_code(name),
            };

            // Rules may outlive the tools they name (e.g. ones an administrator no longer defines).
            if is_inactive_tool_selector(name) {
                text.push_str(strings::access::inactive_tool_selector_suffix());
            }

            text
        })
        .collect::<Vec<_>>()
        .join(", ");

    let users = rule
        .users
        .iter()
        .map(|user| inline_code(user))
        .collect::<Vec<_>>()
        .join(", ");

    let mut text = format!(
        "{marker} {}: {effect} {permissions} for {users}",
        inline_code(&rule.id),
    );

    if let Some(min_power_level) = rule.min_power_level {
        text.push_str(&format!(" (power level {min_power_level} or more)"));
    }

    text
}

#[cfg(test)]
mod tests {
    use mxlink::matrix_sdk::ruma::UserId;

    use super::{format_rule, may_change_rule};
    use crate::authorization::{AccessRule, Authorizer, RuleEffect};
    use crate::controller::access::determination::RuleCommandScope;
    use crate::entity::globalconfig::GlobalConfig;

    fn rule(id: &str, effect: RuleEffect, permissions: &[&str], users: &[&str]) -> AccessRule {
        AccessRule {
            id: id.to_owned(),
            effect,
            permissions: permissions.iter().map(|p| p.to_string()).collect(),
            users: users.iter().map(|u| u.to_string()).collect(),
            min_power_level: None,
        }
    }

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    fn authorizer(room_rules: &[AccessRule]) -> Authorizer {
        let admin_regexes =
            mxidwc::parse_patterns_vector(&["@admin:example.com".to_owned()]).unwrap();
        let mut global_config = GlobalConfig::new(vec![]);
        global_config.access.rules = Some(vec![rule(
            "r1",
            RuleEffect::Allow,
            &["text-generation", "speech-to-text"],
            &["@*:example.com"],
        )]);
        Authorizer::new(&admin_regexes, &global_config).for_room(room_rules, None)
    }

    #[test]
    fn room_managers_change_only_rules_of_features_they_hold() {
        let manager = UserId::parse("@manager:example.com").unwrap();
        let rules = [rule(
            "r1",
            RuleEffect::Allow,
            &["manage-access"],
            &["@manager:example.com"],
        )];
        let authorizer = authorizer(&rules);

        let may = |scope, permissions: &[&str]| {
            may_change_rule(&authorizer, &manager, scope, &strings(permissions))
        };

        assert!(may(RuleCommandScope::Room, &["speech-to-text"]));
        assert!(!may(RuleCommandScope::Room, &["image-generation"]));
        assert!(!may(RuleCommandScope::Room, &["use"]));
        assert!(!may(RuleCommandScope::Room, &["manage-access"]));
        assert!(!may(RuleCommandScope::Global, &["speech-to-text"]));

        let admin = UserId::parse("@admin:example.com").unwrap();
        assert!(may_change_rule(
            &authorizer,
            &admin,
            RuleCommandScope::Global,
            &strings(&["manage-access"])
        ));
    }

    #[test]
    fn room_managers_lose_the_ability_with_their_permission() {
        let manager = UserId::parse("@manager:example.com").unwrap();
        let rules = [
            rule(
                "r1",
                RuleEffect::Allow,
                &["manage-access"],
                &["@manager:example.com"],
            ),
            rule(
                "r2",
                RuleEffect::Deny,
                &["manage-access"],
                &["@manager:example.com"],
            ),
        ];

        assert!(!may_change_rule(
            &authorizer(&rules),
            &manager,
            RuleCommandScope::Room,
            &strings(&["speech-to-text"])
        ));
    }

    #[test]
    fn room_managers_cannot_change_tool_rules() {
        let manager = UserId::parse("@manager:example.com").unwrap();
        let rules = [rule(
            "r1",
            RuleEffect::Allow,
            &["manage-access", "use"],
            &["@manager:example.com"],
        )];
        let authorizer = authorizer(&rules);

        assert!(!may_change_rule(
            &authorizer,
            &manager,
            RuleCommandScope::Room,
            &strings(&["tool:builtin.room.history"])
        ));
        assert!(!may_change_rule(
            &authorizer,
            &manager,
            RuleCommandScope::Room,
            &strings(&["text-generation", "tool:builtin.room.history"])
        ));

        let admin = UserId::parse("@admin:example.com").unwrap();
        assert!(may_change_rule(
            &authorizer,
            &admin,
            RuleCommandScope::Room,
            &strings(&["tool:builtin.room.history"])
        ));
    }

    #[test]
    fn formatted_rules_mark_tools() {
        let rule = rule(
            "r4",
            RuleEffect::Deny,
            &["tool:builtin.room.history", "tool:removed-tool"],
            &["@*:example.com"],
        );

        assert_eq!(
            format_rule(&rule),
            "🚫 `r4`: deny 🧰 `tool:builtin.room.history`, 🧰 `tool:removed-tool` (no such tool, ignored) for `@*:example.com`"
        );
    }

    #[test]
    fn formatted_rules_cannot_change_formatting() {
        let mut rule = rule(
            "r3",
            RuleEffect::Deny,
            &["use", "video-generation"],
            &["@a`**b**`:example.com", "@*:example.com"],
        );
        rule.min_power_level = Some(50);

        assert_eq!(
            format_rule(&rule),
            "🚫 `r3`: deny 🌟 `use`, `video-generation` for @a\\`\\*\\*b\\*\\*\\`:example.com, `@*:example.com` (power level 50 or more)"
        );
    }
}
