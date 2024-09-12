use mxlink::matrix_sdk::ruma::events::macros::EventContent;

use serde::{Deserialize, Serialize};

use mxlink::helpers::account_data_config::GlobalConfig as GlobalConfigTrait;
use mxlink::helpers::account_data_config::GlobalConfigCarrierContent as GlobalConfigCarrierContentTrait;

use crate::agent::AgentDefinition;
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
        Self {
            fallback_room_settings: RoomSettings::default(),

            access: GlobalConfigAccess {
                user_patterns,
                room_local_agent_manager_patterns: None,
            },

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
}
