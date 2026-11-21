//! The single place that decides what a Matrix user may do with the bot.
//!
//! Every access check (incoming messages, reactions, invitations, commands and the selection of
//! conversation context) goes through [`Authorizer`], so that access rules change in one place.
//!
//! Administrators (static configuration) may do anything.
//! Everyone else gets permissions from access rules (see [`AccessRule`]),
//! stored in the global configuration and in each room's configuration.
//! A user has a permission if some allow rule matches and no deny rule does.
//!
//! Tools (see [`Authorizer::allows_tool`]) are the exception: rules decide about them for administrators too,
//! so that a deny rule really turns a tool off.

pub mod rule;

#[cfg(test)]
mod tests;

use mxlink::matrix_sdk::ruma::events::room::power_levels::RoomPowerLevels;
use mxlink::matrix_sdk::ruma::{Int, UserId};

pub use rule::{AccessRule, RuleEffect};

use crate::entity::globalconfig::GlobalConfig;
use crate::tools::catalog::ToolInfo;

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

    /// Permissions for managing a room (which also allow managing it remotely).
    pub const MANAGEMENT: [Permission; 3] =
        [Self::ManageConfig, Self::ManageAgents, Self::ManageAccess];

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

    /// The emoji shown next to the permission's name (matching the documentation's headings).
    pub fn emoji(self) -> &'static str {
        match self {
            Self::TextGeneration => "💬",
            Self::SpeechToText => "🦻",
            Self::TextToSpeech => "🗣️",
            Self::ImageGeneration => "🖌️",
            Self::Invite => "👋",
            Self::ManageConfig => "🛠️",
            Self::ManageAgents => "🤖",
            Self::ManageAccess => "🔒",
            Self::Administer => "👮",
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

        authorizer.add_rules(
            RuleScope::Global,
            global_config.access.rules.as_deref().unwrap_or_default(),
        );

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
    /// Patterns are validated when rules are added, so invalid ones (or an empty list) only come from damaged
    /// configuration. Such rules fail closed: an allow rule matches nobody and a deny rule matches everyone
    /// (except administrators, who are only affected for tools and can repair them).
    fn add_rules(&mut self, scope: RuleScope, rules: &[AccessRule]) {
        for rule in rules {
            let user_regexes = match mxidwc::parse_patterns_vector(&rule.users) {
                Ok(_) if rule.users.is_empty() => {
                    tracing::error!(
                        ?scope,
                        rule_id = rule.id.as_str(),
                        "An access rule has no user patterns - treating it as matching nobody (allow) or everyone (deny)"
                    );
                    None
                }
                Ok(regexes) => Some(regexes),
                Err(err) => {
                    tracing::error!(
                        ?scope,
                        rule_id = rule.id.as_str(),
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
        !self.matching_rules(user_id, permission, effect).is_empty()
    }

    /// Returns the rules (scope and id) with the given effect that apply to the user for a permission.
    /// Used for evaluating, and for explaining decisions to users.
    pub fn matching_rules(
        &self,
        user_id: &UserId,
        permission: Permission,
        effect: RuleEffect,
    ) -> Vec<(RuleScope, String)> {
        self.rules_matching(user_id, effect, |compiled| {
            // Only global rules may grant (or take away) invitations.
            (permission != Permission::Invite || compiled.scope == RuleScope::Global)
                && compiled.rule.covers(permission)
        })
    }

    /// Tells whether the user may use the tool.
    ///
    /// Takes the tool's description (not just an id), so that callers check the rule id (`builtin.room.history`),
    /// not the function name offered to the model (`builtin_room_history`), together with the tool's default.
    ///
    /// A tool enabled by default may be used unless a rule denies it. Other tools need a rule allowing them.
    /// Unlike other permissions, this applies to administrators too.
    ///
    /// This is only about the tool itself: whoever offers tools to the model also checks that the user
    /// may chat (see [`Permission::TextGeneration`]) and that the agent supports tools.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "used when offering tools, once there are some")
    )]
    pub fn allows_tool(&self, user_id: &UserId, tool: &ToolInfo) -> bool {
        (tool.default_enabled
            || !self
                .matching_tool_rules(user_id, tool.id, RuleEffect::Allow)
                .is_empty())
            && self
                .matching_tool_rules(user_id, tool.id, RuleEffect::Deny)
                .is_empty()
    }

    /// Returns the rules (scope and id) with the given effect that apply to the user for a tool.
    /// Used for evaluating, and for explaining decisions to users.
    pub fn matching_tool_rules(
        &self,
        user_id: &UserId,
        tool_id: &str,
        effect: RuleEffect,
    ) -> Vec<(RuleScope, String)> {
        self.rules_matching(user_id, effect, |compiled| {
            compiled.rule.covers_tool(tool_id)
        })
    }

    fn rules_matching(
        &self,
        user_id: &UserId,
        effect: RuleEffect,
        covers: impl Fn(&CompiledRule) -> bool,
    ) -> Vec<(RuleScope, String)> {
        self.rules
            .iter()
            .filter(|compiled| {
                compiled.rule.effect == effect
                    && covers(compiled)
                    && compiled.matches_user(user_id)
                    && self.meets_power_level(user_id, compiled.rule.min_power_level)
            })
            .map(|compiled| (compiled.scope, compiled.rule.id.clone()))
            .collect()
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
    Authorizer::new(admin_regexes, global_config)
        .for_room(&room_config.access.rules, room_power_levels(room).await)
}

/// Returns the room's current power levels, or `None` (logging why) if they are unknown.
/// Without power levels, rules with a power-level condition don't match.
pub async fn room_power_levels(room: &mxlink::matrix_sdk::Room) -> Option<RoomPowerLevels> {
    match room.power_levels().await {
        Ok(power_levels) => Some(power_levels),
        Err(err) => {
            tracing::warn!(
                room_id = room.room_id().as_str(),
                ?err,
                "Failed to get room power levels - access rules with a power-level condition will not match"
            );
            None
        }
    }
}
