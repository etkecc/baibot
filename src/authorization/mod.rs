//! The single place that decides what a Matrix user may do with the bot.
//!
//! Every access check (incoming messages, reactions, invitations, commands and the selection of
//! conversation context) goes through [`Authorizer`], so that access rules change in one place.

#[cfg(test)]
mod tests;

use mxlink::matrix_sdk::ruma::UserId;

use crate::entity::globalconfig::{GlobalConfig, GlobalConfigAccess};

/// Something a user may be allowed to do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Permission {
    /// Use the bot's features (chat, transcription, text-to-speech, images) and read room settings.
    Use,

    /// Invite the bot into a room.
    Invite,

    /// Create, inspect and delete room-local agents.
    ManageRoomLocalAgents,

    /// Manage the global configuration: global settings, global agents, access lists and the
    /// full configuration of static agents.
    ManageGlobalConfig,
}

/// Answers access questions from the administrator patterns (static configuration)
/// and the access lists stored in the global configuration.
pub struct Authorizer<'a> {
    admin_regexes: &'a [regex::Regex],
    access: &'a GlobalConfigAccess,
}

impl<'a> Authorizer<'a> {
    pub fn new(admin_regexes: &'a [regex::Regex], global_config: &'a GlobalConfig) -> Self {
        Self {
            admin_regexes,
            access: &global_config.access,
        }
    }

    pub fn is_admin(&self, user_id: &UserId) -> bool {
        mxidwc::match_user_id(user_id.as_str(), self.admin_regexes)
    }

    /// Tells whether the user has the given permission.
    ///
    /// Administrators have every permission.
    /// It fails if the stored patterns that the answer depends on cannot be parsed.
    pub fn allows(&self, user_id: &UserId, permission: Permission) -> mxidwc::Result<bool> {
        if self.is_admin(user_id) {
            return Ok(true);
        }

        let patterns = match permission {
            Permission::Use | Permission::Invite => &self.access.user_patterns,
            Permission::ManageRoomLocalAgents => &self.access.room_local_agent_manager_patterns,
            Permission::ManageGlobalConfig => return Ok(false),
        };

        matches_patterns(user_id, patterns)
    }

    /// Returns regexes matching the users whose messages may be included in conversation context:
    /// administrators and users with the [`Permission::Use`] permission.
    pub fn context_author_regexes(&self) -> mxidwc::Result<Vec<regex::Regex>> {
        let mut regexes = self.admin_regexes.to_vec();

        if let Some(user_patterns) = &self.access.user_patterns {
            regexes.extend(mxidwc::parse_patterns_vector(user_patterns)?);
        }

        Ok(regexes)
    }

    /// Returns regexes matching administrators only.
    /// Used for conversation context when the stored user patterns cannot be parsed.
    pub fn admin_regexes(&self) -> Vec<regex::Regex> {
        self.admin_regexes.to_vec()
    }
}

fn matches_patterns(user_id: &UserId, patterns: &Option<Vec<String>>) -> mxidwc::Result<bool> {
    let Some(patterns) = patterns else {
        return Ok(false);
    };

    let regexes = mxidwc::parse_patterns_vector(patterns)?;

    Ok(mxidwc::match_user_id(user_id.as_str(), &regexes))
}
