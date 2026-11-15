//! The single place that decides what a Matrix user may do with the bot.
//!
//! Every access check (incoming messages, reactions, invitations, commands and the selection of
//! conversation context) goes through [`Authorizer`], so that access rules change in one place.
//!
//! Administrators (static configuration) may do anything.
//! Everyone else gets permissions from access rules (see [`AccessRule`]),
//! stored in the global configuration and in each room's configuration.
//! A user has a permission if some allow rule matches and no deny rule does.

pub mod rule;

#[cfg(test)]
mod tests;

use mxlink::matrix_sdk::ruma::events::room::power_levels::RoomPowerLevels;
use mxlink::matrix_sdk::ruma::{Int, UserId};

pub use rule::{AccessRule, RuleEffect};

use crate::entity::globalconfig::GlobalConfig;

/// Something a user may be allowed to do.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Permission {
    /// Chat completion (including image and file input).
    TextGeneration,

    /// Transcribing voice messages.
    SpeechToText,

    /// Turning text into voice.
    TextToSpeech,

    /// Generating and editing images, and generating stickers.
    ImageGeneration,

    /// Inviting the bot into a room. Only granted by global rules.
    Invite,

    /// Viewing and changing a room's settings.
    ManageConfig,

    /// Creating, inspecting and deleting room-local agents.
    ManageAgents,

    /// Viewing and changing a room's access rules.
    ManageAccess,

    /// Everything else, which only administrators can do: the global configuration (settings,
    /// agents and rules), the full configuration of static and global agents, and managing other rooms.
    Administer,
}

impl Permission {
    /// The features, which the `use` rule permission covers.
    pub const FEATURES: [Permission; 4] = [
        Self::TextGeneration,
        Self::SpeechToText,
        Self::TextToSpeech,
        Self::ImageGeneration,
    ];

    /// Permissions that rules can grant within a room.
    pub const ROOM: [Permission; 7] = [
        Self::TextGeneration,
        Self::SpeechToText,
        Self::TextToSpeech,
        Self::ImageGeneration,
        Self::ManageConfig,
        Self::ManageAgents,
        Self::ManageAccess,
    ];

    /// The name used in access rules and commands.
    pub fn name(self) -> &'static str {
        match self {
            Self::TextGeneration => "text-generation",
            Self::SpeechToText => "speech-to-text",
            Self::TextToSpeech => "text-to-speech",
            Self::ImageGeneration => "image-generation",
            Self::Invite => "invite",
            Self::ManageConfig => "manage-config",
            Self::ManageAgents => "manage-agents",
            Self::ManageAccess => "manage-access",
            Self::Administer => "administer",
        }
    }

    /// Parses a permission name that rules may contain ([`Self::Administer`] is not one).
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ROOM
            .into_iter()
            .chain([Self::Invite])
            .find(|permission| permission.name() == name)
    }

    pub fn is_feature(self) -> bool {
        Self::FEATURES.contains(&self)
    }
}

/// Where a rule comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuleScope {
    Global,
    Room,
}

#[derive(Clone, Debug)]
struct CompiledRule {
    scope: RuleScope,
    rule: AccessRule,
    /// `None` if the rule's patterns cannot be parsed (see `Authorizer::add_rules`).
    user_regexes: Option<Vec<regex::Regex>>,
}

impl CompiledRule {
    fn matches_user(&self, user_id: &UserId) -> bool {
        match &self.user_regexes {
            Some(regexes) => mxidwc::match_user_id(user_id.as_str(), regexes),
            None => self.rule.effect == RuleEffect::Deny,
        }
    }
}

/// Answers access questions from the administrator patterns (static configuration),
/// the global access rules and, when created for a room, that room's rules and power levels.
#[derive(Clone, Debug)]
pub struct Authorizer {
    admin_regexes: Vec<regex::Regex>,
    rules: Vec<CompiledRule>,
    power_levels: Option<RoomPowerLevels>,
}

impl Authorizer {
    /// Creates an authorizer which only knows the global rules (no room).
    pub fn new(admin_regexes: &[regex::Regex], global_config: &GlobalConfig) -> Self {
        let mut authorizer = Self {
            admin_regexes: admin_regexes.to_vec(),
            rules: vec![],
            power_levels: None,
        };

        authorizer.add_rules(RuleScope::Global, &global_config.access.effective_rules());

        authorizer
    }

    /// Adds a room's rules and power levels to an authorizer created by [`Self::new`].
    ///
    /// Without power levels, rules with a power-level condition don't match.
    pub fn for_room(
        mut self,
        room_rules: &[AccessRule],
        power_levels: Option<RoomPowerLevels>,
    ) -> Self {
        self.add_rules(RuleScope::Room, room_rules);
        self.power_levels = power_levels;
        self
    }

    /// Adds rules, compiling their user patterns.
    ///
    /// Patterns are validated when rules are added, so invalid ones only come from damaged configuration.
    /// Such rules fail closed: an allow rule matches nobody and a deny rule matches everyone
    /// (except administrators, who are never affected by rules and can repair them).
    fn add_rules(&mut self, scope: RuleScope, rules: &[AccessRule]) {
        for rule in rules {
            let user_regexes = match mxidwc::parse_patterns_vector(&rule.users) {
                Ok(regexes) => Some(regexes),
                Err(err) => {
                    tracing::error!(
                        ?scope,
                        rule_id = rule.id,
                        ?err,
                        "Failed parsing the user patterns of an access rule - treating it as matching nobody (allow) or everyone (deny)"
                    );
                    None
                }
            };

            self.rules.push(CompiledRule {
                scope,
                rule: rule.clone(),
                user_regexes,
            });
        }
    }

    pub fn is_admin(&self, user_id: &UserId) -> bool {
        mxidwc::match_user_id(user_id.as_str(), &self.admin_regexes)
    }

    /// Tells whether the user has the given permission.
    ///
    /// Administrators have every permission.
    pub fn allows(&self, user_id: &UserId, permission: Permission) -> bool {
        if self.is_admin(user_id) {
            return true;
        }

        if permission == Permission::Administer {
            return false;
        }

        self.matches(user_id, permission, RuleEffect::Allow)
            && !self.matches(user_id, permission, RuleEffect::Deny)
    }

    /// Tells whether the user has any permission in the room (besides inviting),
    /// which lets them talk to the bot there (help, listings, and commands that check further).
    pub fn allows_anything_in_room(&self, user_id: &UserId) -> bool {
        self.is_admin(user_id)
            || Permission::ROOM
                .into_iter()
                .any(|permission| self.allows(user_id, permission))
    }

    /// Tells whether a deny rule takes the given permission away from the user.
    /// Administrators are never denied anything.
    pub fn denies(&self, user_id: &UserId, permission: Permission) -> bool {
        !self.is_admin(user_id) && self.matches(user_id, permission, RuleEffect::Deny)
    }

    fn matches(&self, user_id: &UserId, permission: Permission, effect: RuleEffect) -> bool {
        self.rules.iter().any(|compiled| {
            compiled.rule.effect == effect
                // Only global rules may grant (or take away) invitations.
                && (permission != Permission::Invite || compiled.scope == RuleScope::Global)
                && compiled.rule.covers(permission)
                && compiled.matches_user(user_id)
                && self.meets_power_level(user_id, compiled.rule.min_power_level)
        })
    }

    fn meets_power_level(&self, user_id: &UserId, min_power_level: Option<i64>) -> bool {
        let Some(min_power_level) = min_power_level else {
            return true;
        };

        let (Some(power_levels), Some(min_power_level)) =
            (&self.power_levels, Int::new(min_power_level))
        else {
            return false;
        };

        power_levels.for_user(user_id) >= min_power_level
    }
}

/// Creates an authorizer for a Matrix room: global rules, the room's rules and its current power levels.
pub async fn authorizer_for_room(
    admin_regexes: &[regex::Regex],
    global_config: &GlobalConfig,
    room_config: &crate::entity::roomconfig::RoomConfig,
    room: &mxlink::matrix_sdk::Room,
) -> Authorizer {
    let power_levels = match room.power_levels().await {
        Ok(power_levels) => Some(power_levels),
        Err(err) => {
            tracing::warn!(
                room_id = room.room_id().as_str(),
                ?err,
                "Failed to get room power levels - access rules with a power-level condition will not match"
            );
            None
        }
    };

    Authorizer::new(admin_regexes, global_config).for_room(&room_config.access.rules, power_levels)
}
