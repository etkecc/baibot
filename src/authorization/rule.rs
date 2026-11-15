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
    /// Identifies the rule within its list (for removal).
    /// Random (see [`new_rule_id`]), so that it doesn't suggest an order and isn't reused.
    pub id: String,

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

/// The emoji shown next to a permission name used in rules, if it is a known one.
pub fn permission_name_emoji(name: &str) -> Option<&'static str> {
    if name == USE {
        return Some("🌟");
    }

    Permission::from_name(name).map(Permission::emoji)
}

/// Tells whether a name is valid in rules (a known permission name or [`USE`]).
pub fn is_valid_permission_name(name: &str) -> bool {
    name == USE || Permission::from_name(name).is_some()
}

/// Characters of generated rule ids: lowercase letters and digits, without easily confused ones.
const RULE_ID_ALPHABET: &[u8] = b"abcdefghjkmnpqrstuvwxyz23456789";

const RULE_ID_LENGTH: usize = 4;

/// Generates a short random rule id which isn't used in the given list of rules.
pub fn new_rule_id(rules: &[AccessRule]) -> String {
    new_rule_id_with(rules, &mut fastrand::Rng::new())
}

fn new_rule_id_with(rules: &[AccessRule], rng: &mut fastrand::Rng) -> String {
    loop {
        let id: String = (0..RULE_ID_LENGTH)
            .map(|_| RULE_ID_ALPHABET[rng.usize(..RULE_ID_ALPHABET.len())] as char)
            .collect();

        if !rules.iter().any(|rule| rule.id == id) {
            return id;
        }
    }
}

/// Tells whether a text could be a rule id, to reject anything else early.
pub fn is_valid_rule_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 32
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
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
        let id = new_rule_id(&rules);
        rules.push(AccessRule {
            id,
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

#[cfg(test)]
mod tests {
    use super::{AccessRule, RuleEffect, new_rule_id_with};

    #[test]
    fn new_rule_ids_skip_used_ones() {
        let seed = 42;
        let taken = new_rule_id_with(&[], &mut fastrand::Rng::with_seed(seed));

        let rules = [AccessRule {
            id: taken.clone(),
            effect: RuleEffect::Allow,
            permissions: vec![],
            users: vec![],
            min_power_level: None,
        }];

        // The same seed draws the taken id first, so a different one means it was skipped.
        let id = new_rule_id_with(&rules, &mut fastrand::Rng::with_seed(seed));
        assert_ne!(id, taken);
        assert_eq!(id.len(), taken.len());
    }
}
