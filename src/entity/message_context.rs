use mxlink::matrix_sdk::ruma::{OwnedEventId, OwnedUserId, RoomId};
use mxlink::matrix_sdk::Room;

use mxlink::ThreadInfo;

use super::{
    globalconfig::GlobalConfig, roomconfig::RoomConfig, MessagePayload, RoomConfigContext,
    TriggerEventInfo,
};

#[derive(Debug)]
pub struct MessageContext {
    room: Room,
    room_config_context: RoomConfigContext,
    admin_whitelist_regexes: Vec<regex::Regex>,
    trigger_event_info: TriggerEventInfo,
    thread_info: ThreadInfo,
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
        }
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

    pub fn thread_info(&self) -> &ThreadInfo {
        &self.thread_info
    }

    pub fn sender_can_manage_global_config(&self) -> anyhow::Result<bool> {
        Ok(self.trigger_event_info.sender_is_admin)
    }

    pub fn sender_can_manage_room_local_agents(&self) -> anyhow::Result<bool> {
        Ok(self.sender_can_manage_global_config()?
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

    fn sender_is_allowed_room_local_agent_manager(&self) -> anyhow::Result<bool> {
        match &self
            .global_config()
            .access
            .room_local_agent_manager_patterns
        {
            None => Ok(false),
            Some(patterns) => {
                let allowed_regexes = mxidwc::parse_patterns_vector(patterns)?;

                Ok(mxidwc::match_user_id(
                    self.sender_id().as_str(),
                    &allowed_regexes,
                ))
            }
        }
    }
}
