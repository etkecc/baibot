use serde::{Deserialize, Serialize};

use super::Permission;
use crate::entity::globalconfig::GlobalConfigAccess;

/// The rule permission name that covers all features (see [`Permission::FEATURES`]).
///
/// It is evaluated when checking access (not expanded when a rule is saved),
/// so features added to it later reach existing `use` rules.
pub const USE: &str = "use";

/// The power level at which migrated allowed users may manage a room's configuration (moderators).
pub const LEGACY_MANAGE_CONFIG_MIN_POWER_LEVEL: i64 = 50;

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(from = "String", into = "String")]
pub enum RuleEffect {
    Allow,
    Deny,

    /// An effect unknown to this version (written by a newer one). Such rules are ignored.
    /// The original value is kept, so that saving the configuration doesn't lose it.
    Unknown(String),
}

impl From<String> for RuleEffect {
    fn from(value: String) -> Self {
        match value.as_str() {
            "allow" => Self::Allow,
            "deny" => Self::Deny,
            _ => Self::Unknown(value),
        }
    }
}

impl From<RuleEffect> for String {
    fn from(value: RuleEffect) -> Self {
        match value {
            RuleEffect::Allow => "allow".to_owned(),
            RuleEffect::Deny => "deny".to_owned(),
            RuleEffect::Unknown(value) => value,
        }
    }
}

/// An access rule, stored in the global configuration or in a room's configuration.
///
/// Rules are unordered. A user has a permission if some allow rule matches and no deny rule does.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct AccessRule {
    /// Identifies the rule within its list (for removal). Not a position.
    pub id: u32,

    pub effect: RuleEffect,

    /// Permission names (see [`Permission::name`] and [`USE`]).
    /// Kept as strings, so that names unknown to this version don't make the configuration unreadable.
    /// Unknown names are ignored when evaluating.
    pub permissions: Vec<String>,

    /// MXID patterns (e.g. `@*:example.com`).
    pub users: Vec<String>,

    /// If set, the rule only matches users whose power level in the room is at or above this value.
    /// Without a room (e.g. for invitations), such rules never match.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_power_level: Option<i64>,
}

impl AccessRule {
    pub fn covers(&self, permission: Permission) -> bool {
        self.permissions
            .iter()
            .any(|name| permission_name_covers(name, permission))
    }
}

/// Tells whether a permission name, as used in rules, covers the given permission.
pub fn permission_name_covers(name: &str, permission: Permission) -> bool {
    if name == USE {
        return permission.is_feature();
    }

    Permission::from_name(name) == Some(permission)
}

/// Returns the next free rule id for a list of rules.
pub fn next_rule_id(rules: &[AccessRule]) -> u32 {
    rules.iter().map(|rule| rule.id).max().unwrap_or(0) + 1
}

/// Derives global rules from the lists used by older versions
/// (allowed users and room-local agent managers).
///
/// Allowed users can use all features and invite the bot, and can manage the configuration
/// of rooms where they are moderators (or more).
/// Room-local agent managers can manage agents.
pub fn rules_from_legacy_access(access: &GlobalConfigAccess) -> Vec<AccessRule> {
    let mut rules = vec![];

    let mut push = |effect: RuleEffect, permissions: &[&str], users: &[String], min_power_level| {
        rules.push(AccessRule {
            id: next_rule_id(&rules),
            effect,
            permissions: permissions.iter().map(|name| name.to_string()).collect(),
            users: users.to_vec(),
            min_power_level,
        });
    };

    if let Some(user_patterns) = &access.user_patterns
        && !user_patterns.is_empty()
    {
        push(
            RuleEffect::Allow,
            &[USE, Permission::Invite.name()],
            user_patterns,
            None,
        );
        push(
            RuleEffect::Allow,
            &[Permission::ManageConfig.name()],
            user_patterns,
            Some(LEGACY_MANAGE_CONFIG_MIN_POWER_LEVEL),
        );
    }

    if let Some(manager_patterns) = &access.room_local_agent_manager_patterns
        && !manager_patterns.is_empty()
    {
        push(
            RuleEffect::Allow,
            &[Permission::ManageAgents.name()],
            manager_patterns,
            None,
        );
    }

    rules
}
