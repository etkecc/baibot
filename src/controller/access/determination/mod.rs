#[cfg(test)]
mod tests;

use super::super::ControllerType;
use crate::authorization::rule::{USE, is_valid_permission_name, is_valid_rule_id};
use crate::authorization::{Permission, RuleEffect};
use crate::strings;

/// Where an access rule command applies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuleCommandScope {
    Global,
    Room,
}

/// An access rule to add, as parsed from a command.
#[derive(Debug, PartialEq)]
pub struct NewRule {
    pub effect: RuleEffect,
    pub permissions: Vec<String>,
    pub users: Vec<String>,
    pub min_power_level: Option<i64>,
}

#[derive(Debug, PartialEq)]
pub enum AccessControllerType {
    Help,

    /// Lists the global rules and the room's rules.
    List,

    AddRule(RuleCommandScope, NewRule),

    RemoveRule(RuleCommandScope, String),

    /// Explains the permissions of a user (the sender if `None`) in the room.
    Explain(Option<String>),
}

const MIN_POWER_LEVEL_PREFIX: &str = "min-power-level=";

pub fn determine_controller(command_prefix: &str, text: &str) -> ControllerType {
    let mut words = text.split_whitespace();

    let Some(first) = words.next() else {
        return ControllerType::Access(AccessControllerType::Help);
    };

    match first {
        "list" => ControllerType::Access(AccessControllerType::List),

        "explain" if words.clone().count() > 1 => {
            ControllerType::Error(strings::access::explain_takes_one_user())
        }

        "explain" => ControllerType::Access(AccessControllerType::Explain(
            words.next().map(str::to_owned),
        )),

        "global" | "room" => {
            let scope = if first == "global" {
                RuleCommandScope::Global
            } else {
                RuleCommandScope::Room
            };

            match determine_rule_command(command_prefix, scope, words.collect()) {
                Ok(controller_type) => ControllerType::Access(controller_type),
                Err(message) => ControllerType::Error(message),
            }
        }

        "users" | "set-users" | "room-local-agent-managers" | "set-room-local-agent-managers" => {
            ControllerType::Error(strings::access::legacy_command_replaced(command_prefix))
        }

        _ => ControllerType::Access(AccessControllerType::Help),
    }
}

fn determine_rule_command(
    command_prefix: &str,
    scope: RuleCommandScope,
    words: Vec<&str>,
) -> Result<AccessControllerType, String> {
    let Some((action, arguments)) = words.split_first() else {
        return Err(strings::access::rule_command_usage(command_prefix));
    };

    match *action {
        "allow" | "deny" => {
            let effect = if *action == "allow" {
                RuleEffect::Allow
            } else {
                RuleEffect::Deny
            };

            parse_new_rule(command_prefix, scope, effect, arguments)
                .map(|rule| AccessControllerType::AddRule(scope, rule))
        }

        "remove" => match arguments {
            [id] => {
                let id = id.trim_matches('`').to_lowercase();
                if is_valid_rule_id(&id) {
                    Ok(AccessControllerType::RemoveRule(scope, id))
                } else {
                    Err(strings::access::invalid_rule_id(command_prefix, &id))
                }
            }
            _ => Err(strings::access::rule_command_usage(command_prefix)),
        },

        _ => Err(strings::access::rule_command_usage(command_prefix)),
    }
}

fn parse_new_rule(
    command_prefix: &str,
    scope: RuleCommandScope,
    effect: RuleEffect,
    arguments: &[&str],
) -> Result<NewRule, String> {
    // Forgiving about separators: permissions and user patterns may be separated by commas, spaces,
    // or both. User patterns always start with `@` (and cannot contain commas), so everything else
    // is a permission. A `for` before the user patterns (as in the rule list) is accepted too.
    let mut permissions: Vec<String> = vec![];
    let mut users: Vec<String> = vec![];
    let mut min_power_level = None;

    let tokens = arguments
        .iter()
        .flat_map(|argument| argument.split(','))
        .map(|token| token.trim().trim_matches('`'))
        .filter(|token| !token.is_empty());

    for token in tokens {
        if let Some(value) = token.strip_prefix(MIN_POWER_LEVEL_PREFIX) {
            let value = value
                .parse::<i64>()
                .ok()
                .filter(|value| mxlink::matrix_sdk::ruma::Int::new(*value).is_some())
                .ok_or_else(|| strings::access::invalid_min_power_level(value))?;

            if min_power_level.is_some() {
                return Err(strings::access::duplicate_min_power_level());
            }

            min_power_level = Some(value);
        } else if token.starts_with('@') {
            if !users.iter().any(|existing| existing == token) {
                users.push(token.to_owned());
            }
        } else if token == "for" {
            continue;
        } else if !permissions.iter().any(|existing| existing == token) {
            permissions.push(token.to_owned());
        }
    }

    if permissions.is_empty() {
        return Err(strings::access::rule_command_usage(command_prefix));
    }

    for name in &permissions {
        if !is_valid_permission_name(name) {
            return Err(strings::access::unknown_permission(command_prefix, name));
        }

        if scope == RuleCommandScope::Room && name == Permission::Invite.name() {
            return Err(strings::access::invite_only_in_global_rules());
        }
    }

    if users.is_empty() {
        return Err(strings::access::rule_command_usage(command_prefix));
    }

    if min_power_level.is_some()
        && permissions
            .iter()
            .any(|name| name == Permission::Invite.name())
    {
        return Err(strings::access::invite_without_power_level());
    }

    if let Err(err) = mxidwc::parse_patterns_vector(&users) {
        return Err(strings::access::failed_to_parse_patterns(&err.to_string()));
    }

    Ok(NewRule {
        effect,
        permissions,
        users,
        min_power_level,
    })
}

impl AccessControllerType {
    pub fn required_permission(&self) -> Option<Permission> {
        match self {
            // Anyone can see the help, so that all users can get familiar with how the bot's access system works.
            Self::Help => None,
            // Anyone can see why they themselves can (or cannot) do something.
            Self::Explain(None) => None,
            Self::Explain(Some(_)) | Self::List => Some(Permission::ManageAccess),
            Self::AddRule(RuleCommandScope::Global, _)
            | Self::RemoveRule(RuleCommandScope::Global, _) => Some(Permission::Administer),
            // Which room rules the sender may add or remove is checked further when handling the command.
            Self::AddRule(RuleCommandScope::Room, _)
            | Self::RemoveRule(RuleCommandScope::Room, _) => Some(Permission::ManageAccess),
        }
    }
}

/// Tells whether a non-administrator holding `held` permissions in a room may add or remove
/// a room rule with the given permission names.
///
/// They may only manage rules made of feature permissions (or `use`), and only of features they hold themselves.
pub fn can_delegate(permissions: &[String], holds: impl Fn(Permission) -> bool) -> bool {
    permissions.iter().all(|name| {
        if name == USE {
            return Permission::FEATURES.into_iter().all(&holds);
        }

        match Permission::from_name(name) {
            Some(permission) => permission.is_feature() && holds(permission),
            None => false,
        }
    })
}
