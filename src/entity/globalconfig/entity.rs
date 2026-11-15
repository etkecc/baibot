use mxlink::matrix_sdk::ruma::events::macros::EventContent;

use serde::{Deserialize, Serialize};

use mxlink::helpers::account_data_config::GlobalConfig as GlobalConfigTrait;
use mxlink::helpers::account_data_config::GlobalConfigCarrierContent as GlobalConfigCarrierContentTrait;

use crate::agent::AgentDefinition;
use crate::authorization::AccessRule;
use crate::authorization::rule::rules_from_legacy_access;
use crate::entity::roomconfig::RoomSettings;

#[derive(Clone, Debug, Default, Deserialize, Serialize, EventContent)]
#[ruma_event(type = "cc.etke.baibot.global_config", kind = GlobalAccountData)]
pub struct GlobalConfigCarrierContent {
    pub payload: String,
}

impl GlobalConfigCarrierContentTrait for GlobalConfigCarrierContent {
    fn payload(&self) -> &str {
        &self.payload
    }

    fn new(payload: String) -> Self {
        Self { payload }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct GlobalConfig {
    pub fallback_room_settings: RoomSettings,

    pub access: GlobalConfigAccess,

    pub agents: Vec<AgentDefinition>,
}

impl GlobalConfig {
    pub fn new(user_patterns: Option<Vec<String>>) -> Self {
        // `user_patterns` (from the initial configuration) seeds the rules.
        let mut access = GlobalConfigAccess {
            user_patterns,
            room_local_agent_manager_patterns: None,
            rules: None,
        };
        access.migrate_legacy_lists();

        Self {
            fallback_room_settings: RoomSettings::default(),

            access,

            agents: vec![],
        }
    }
}

impl GlobalConfigTrait for GlobalConfig {}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct GlobalConfigAccess {
    // Contains a list of patterns that will be used to specify the "allowed bot users".
    // These remain as patterns and are turned into regex and made use of on demand.
    // Example: `["@*:example.com"]`
    pub user_patterns: Option<Vec<String>>,

    // Contains a list of patterns that will be used to specify "allowed room-local agent managers".
    // These remain as patterns and are turned into regex and made use of on demand.
    // Example: `["@*:example.com"]`
    pub room_local_agent_manager_patterns: Option<Vec<String>>,

    // Global access rules (see `crate::authorization`).
    //
    // `None` means that the configuration predates access rules and still needs to be migrated
    // (see `migrate_legacy_lists()`), which happens whenever the configuration is loaded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rules: Option<Vec<AccessRule>>,
}

impl GlobalConfigAccess {
    /// Turns the lists used by older versions (allowed users and room-local agent managers)
    /// into rules, and removes them.
    ///
    /// Lists found next to existing rules (set again by an older version after a downgrade) are
    /// removed without being turned into rules, since the rules already say who has access.
    ///
    /// Returns whether anything changed, in which case the configuration needs to be persisted.
    pub fn migrate_legacy_lists(&mut self) -> bool {
        let has_lists =
            self.user_patterns.is_some() || self.room_local_agent_manager_patterns.is_some();

        match &self.rules {
            None => {
                self.rules = Some(rules_from_legacy_access(self));
            }
            Some(_) if has_lists => {
                tracing::warn!(
                    user_patterns = ?self.user_patterns,
                    room_local_agent_manager_patterns = ?self.room_local_agent_manager_patterns,
                    "Removing user lists of an older version, which access rules have replaced"
                );
            }
            Some(_) => return false,
        }

        self.user_patterns = None;
        self.room_local_agent_manager_patterns = None;

        true
    }
}
