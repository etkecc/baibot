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

    /// Permission names (see [`Permission::name`] and [`USE`]) and tool selectors (see [`TOOL_PREFIX`]).
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

    /// Tells whether the rule covers the tool with the given id (see [`tool_selector_covers`]).
    pub fn covers_tool(&self, tool_id: &str) -> bool {
        self.permissions
            .iter()
            .any(|name| tool_selector_covers(name, tool_id))
    }
}

/// Tells whether a permission name, as used in rules, covers the given permission.
pub fn permission_name_covers(name: &str, permission: Permission) -> bool {
    if name == USE {
        return permission.is_feature();
    }

    Permission::from_name(name) == Some(permission)
}

/// The emoji shown next to a permission name used in rules, if it is a known one or a tool selector.
pub fn permission_name_emoji(name: &str) -> Option<&'static str> {
    if name == USE {
        return Some("🌟");
    }

    if name.starts_with(TOOL_PREFIX) {
        return Some(TOOL_EMOJI);
    }

    Permission::from_name(name).map(Permission::emoji)
}

/// Tells whether a name is valid in new rules: a known permission name, [`USE`],
/// or a tool selector matching a known tool (see [`is_valid_tool_selector`]).
pub fn is_valid_permission_name(name: &str) -> bool {
    name == USE
        || Permission::from_name(name).is_some()
        || is_valid_tool_selector(name, crate::tools::catalog::known().map(|tool| tool.id))
}

/// The prefix of rule permission names which allow or deny tools (e.g. `tool:builtin.room.history`).
///
/// Tools are not covered by [`USE`]: a rule names them explicitly, unless they are enabled by default.
pub const TOOL_PREFIX: &str = "tool:";

/// The emoji shown next to tool selectors.
pub const TOOL_EMOJI: &str = "🧰";

/// The suffix of a tool selector matching all tools in a namespace (e.g. `tool:holmes.*`).
const TOOL_NAMESPACE_WILDCARD: &str = ".*";

/// Tells whether a rule permission name is a tool selector covering the tool with the given id.
///
/// Selectors are either an exact id (`tool:builtin.room.history`) or a namespace (`tool:holmes.*`, covering
/// `holmes.investigate`, but not `holmes` itself). There is no selector for all tools.
pub fn tool_selector_covers(name: &str, tool_id: &str) -> bool {
    let Some(selector) = name.strip_prefix(TOOL_PREFIX) else {
        return false;
    };

    match selector.strip_suffix(TOOL_NAMESPACE_WILDCARD) {
        Some(namespace) => tool_id
            .strip_prefix(namespace)
            .is_some_and(|rest| rest.starts_with('.')),
        None => selector == tool_id,
    }
}

/// Tells whether a name is a tool selector covering at least one of the given tool ids.
///
/// Rules may keep selectors for tools which no longer exist. They are ignored (see [`is_inactive_tool_selector`]),
/// but new rules may only name existing tools, so that typos don't go unnoticed.
pub fn is_valid_tool_selector<'a>(name: &str, mut tool_ids: impl Iterator<Item = &'a str>) -> bool {
    name.starts_with(TOOL_PREFIX) && tool_ids.any(|tool_id| tool_selector_covers(name, tool_id))
}

/// Tells whether a name has the form of a tool selector (`tool:ID` or `tool:NAMESPACE.*`),
/// whether or not such tools exist. Ids are made of lowercase letters, digits and `-`,
/// with `.` separating namespaces.
pub fn is_well_formed_tool_selector(name: &str) -> bool {
    let Some(selector) = name.strip_prefix(TOOL_PREFIX) else {
        return false;
    };

    let id = selector
        .strip_suffix(TOOL_NAMESPACE_WILDCARD)
        .unwrap_or(selector);

    !id.is_empty()
        && id.split('.').all(|part| {
            !part.is_empty()
                && part
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        })
}

/// Tells whether a name is a tool selector which covers none of the known tools.
pub fn is_inactive_tool_selector(name: &str) -> bool {
    name.starts_with(TOOL_PREFIX)
        && !is_valid_tool_selector(name, crate::tools::catalog::known().map(|tool| tool.id))
}

/// Returns the selectors of all known tools, followed by those of their namespaces (for telling people what they can
/// use in rules).
pub fn known_tool_selectors() -> Vec<String> {
    let ids: Vec<&str> = crate::tools::catalog::known().map(|tool| tool.id).collect();
    tool_selectors(&ids)
}

/// The selectors of the given tool ids (`tool:builtin.image.edit`), followed by those of their namespaces
/// (`tool:builtin.*`, `tool:builtin.image.*`), each once, in order.
fn tool_selectors(tool_ids: &[&str]) -> Vec<String> {
    let mut selectors: Vec<String> = tool_ids
        .iter()
        .map(|id| format!("{TOOL_PREFIX}{id}"))
        .collect();

    for id in tool_ids {
        let mut namespace_end = id.find('.');
        while let Some(end) = namespace_end {
            let selector = format!("{TOOL_PREFIX}{}{TOOL_NAMESPACE_WILDCARD}", &id[..end]);
            if !selectors.contains(&selector) {
                selectors.push(selector);
            }
            namespace_end = id[end + 1..].find('.').map(|next| end + 1 + next);
        }
    }

    selectors
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
    use super::{
        AccessRule, RuleEffect, is_inactive_tool_selector, is_valid_permission_name,
        is_valid_tool_selector, is_well_formed_tool_selector, known_tool_selectors,
        new_rule_id_with, tool_selector_covers, tool_selectors,
    };

    #[test]
    fn tool_selectors_match_ids_and_namespaces() {
        assert!(tool_selector_covers(
            "tool:builtin.room.history",
            "builtin.room.history"
        ));
        assert!(!tool_selector_covers(
            "tool:builtin.room.history",
            "builtin.room.history-2"
        ));
        assert!(!tool_selector_covers(
            "builtin.room.history",
            "builtin.room.history"
        ));

        assert!(tool_selector_covers("tool:holmes.*", "holmes.investigate"));
        assert!(tool_selector_covers(
            "tool:holmes.*",
            "holmes.deep.investigate"
        ));
        assert!(tool_selector_covers(
            "tool:holmes.deep.*",
            "holmes.deep.investigate"
        ));
        assert!(!tool_selector_covers("tool:holmes.*", "holmes"));
        assert!(!tool_selector_covers(
            "tool:holmes.*",
            "holmesgpt.investigate"
        ));
        assert!(!tool_selector_covers(
            "tool:holmes.*",
            "builtin.room.history"
        ));

        // There is no selector for all tools.
        assert!(!tool_selector_covers("tool:*", "builtin.room.history"));
        assert!(!tool_selector_covers("tool:.*", "builtin.room.history"));
        assert!(!tool_selector_covers("tool:", "builtin.room.history"));
    }

    #[test]
    fn tool_selectors_are_valid_only_for_existing_tools() {
        let ids = || ["builtin.room.history", "holmes.investigate"].into_iter();

        assert!(is_valid_tool_selector("tool:builtin.room.history", ids()));
        assert!(is_valid_tool_selector("tool:holmes.investigate", ids()));
        assert!(is_valid_tool_selector("tool:holmes.*", ids()));
        assert!(!is_valid_tool_selector("tool:holmes", ids()));
        assert!(!is_valid_tool_selector("tool:room-histroy", ids()));
        assert!(!is_valid_tool_selector("tool:*", ids()));
        assert!(!is_valid_tool_selector("builtin.room.history", ids()));

        assert!(is_valid_permission_name("tool:builtin.room.history"));
        assert!(is_valid_permission_name("tool:builtin.room.*"));
        assert!(is_valid_permission_name("tool:builtin.image.*"));
        assert!(!is_valid_permission_name("tool:holmes.*"));
        assert!(!is_valid_permission_name("tool:"));

        assert!(!is_inactive_tool_selector("tool:builtin.room.history"));
        assert!(is_inactive_tool_selector("tool:removed-tool"));
        assert!(!is_inactive_tool_selector("text-generation"));
    }

    #[test]
    fn known_tool_selectors_include_namespaces() {
        assert_eq!(
            tool_selectors(&[
                "builtin.image.edit",
                "builtin.image.generate",
                "a.b.c",
                "plain"
            ]),
            [
                "tool:builtin.image.edit",
                "tool:builtin.image.generate",
                "tool:a.b.c",
                "tool:plain",
                "tool:builtin.*",
                "tool:builtin.image.*",
                "tool:a.*",
                "tool:a.b.*",
            ]
        );

        assert!(
            known_tool_selectors()
                .iter()
                .all(|selector| is_valid_permission_name(selector)),
            "{:?}",
            known_tool_selectors()
        );
    }

    #[test]
    fn tool_selectors_are_well_formed_regardless_of_existing_tools() {
        for name in [
            "tool:builtin.room.history",
            "tool:removed-tool",
            "tool:holmes.investigate",
            "tool:holmes.*",
            "tool:a.b.*",
        ] {
            assert!(is_well_formed_tool_selector(name), "{name}");
        }

        for name in [
            "tool:",
            "tool:*",
            "tool:.*",
            "tool:holmes.",
            "tool:.holmes",
            "tool:holmes..x",
            "tool:Room-History",
            "tool:room history",
            "tool:holmes*",
            "builtin.room.history",
        ] {
            assert!(!is_well_formed_tool_selector(name), "{name}");
        }
    }

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
