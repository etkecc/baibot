use mxlink::matrix_sdk::Room;
use mxlink::matrix_sdk::ruma::{OwnedEventId, OwnedUserId, RoomId};

use mxlink::ThreadInfo;

use super::{
    MessagePayload, RoomConfigContext, TriggerEventInfo, globalconfig::GlobalConfig,
    roomconfig::RoomConfig,
};

#[derive(Debug, Clone)]
pub struct MessageContext {
    room: Room,
    room_config_context: RoomConfigContext,
    admin_whitelist_regexes: Vec<regex::Regex>,
    trigger_event_info: TriggerEventInfo,
    thread_info: ThreadInfo,

    bot_display_name: Option<String>,
}

impl MessageContext {
    pub fn new(
        room: Room,
        room_config_context: RoomConfigContext,
        admin_whitelist_regexes: Vec<regex::Regex>,
        trigger_event_info: TriggerEventInfo,
        thread_info: ThreadInfo,
    ) -> Self {
        Self {
            room,
            room_config_context,
            admin_whitelist_regexes,
            trigger_event_info,
            thread_info,

            bot_display_name: None,
        }
    }

    pub fn with_bot_display_name(mut self, value: Option<String>) -> Self {
        self.bot_display_name = value;
        self
    }

    pub fn bot_display_name(&self) -> &Option<String> {
        &self.bot_display_name
    }

    pub fn room(&self) -> &Room {
        &self.room
    }

    pub fn room_id(&self) -> &RoomId {
        self.room.room_id()
    }

    pub fn global_config(&self) -> &GlobalConfig {
        &self.room_config_context.global_config
    }

    pub fn room_config(&self) -> &RoomConfig {
        &self.room_config_context.room_config
    }

    pub fn room_config_context(&self) -> &RoomConfigContext {
        &self.room_config_context
    }

    pub fn event_id(&self) -> &OwnedEventId {
        &self.trigger_event_info.event_id
    }

    pub fn sender_id(&self) -> &OwnedUserId {
        &self.trigger_event_info.sender
    }

    pub fn payload(&self) -> &MessagePayload {
        &self.trigger_event_info.payload
    }

    pub fn set_payload(&mut self, payload: MessagePayload) {
        self.trigger_event_info.payload = payload;
    }

    pub fn thread_info(&self) -> &ThreadInfo {
        &self.thread_info
    }

    pub fn sender_can_manage_global_config(&self) -> bool {
        self.trigger_event_info.sender_is_admin
    }

    pub fn sender_can_manage_room_local_agents(&self) -> mxidwc::Result<bool> {
        Ok(self.sender_can_manage_global_config()
            || self.sender_is_allowed_room_local_agent_manager()?)
    }

    pub fn combined_admin_and_user_regexes(&self) -> Vec<regex::Regex> {
        let mut combined = self.admin_whitelist_regexes.clone();

        if let Some(user_patterns) = &self.global_config().access.user_patterns {
            let user_regexes = mxidwc::parse_patterns_vector(user_patterns);

            match user_regexes {
                Ok(user_regexes) => {
                    combined.extend(user_regexes);
                }
                Err(err) => {
                    tracing::warn!(
                        "Error parsing user patterns for room {}: {:?}",
                        self.room.room_id(),
                        err
                    );
                }
            }
        }

        combined
    }

    fn sender_is_allowed_room_local_agent_manager(&self) -> mxidwc::Result<bool> {
        self.room_config_context()
            .is_user_allowed_room_local_agent_manager(self.sender_id().clone())
    }
}
