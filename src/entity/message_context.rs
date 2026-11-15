use mxlink::matrix_sdk::Room;
use mxlink::matrix_sdk::ruma::{OwnedEventId, OwnedUserId, RoomId};

use mxlink::ThreadInfo;

use crate::authorization::{Authorizer, Permission};

use super::{
    MessagePayload, RoomConfigContext, TriggerEventInfo, globalconfig::GlobalConfig,
    roomconfig::RoomConfig,
};

#[derive(Debug)]
pub struct MessageContext {
    /// The room the triggering message was sent in. Replies go here.
    room: Room,
    /// The room the command acts on. Usually the same as `room`, but differs for remote commands (`in <room> ...`).
    target_room: Room,
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
            target_room: room.clone(),
            room,
            room_config_context,
            admin_whitelist_regexes,
            trigger_event_info,
            thread_info,

            bot_display_name: None,
        }
    }

    /// Returns a copy of this context which acts on `target_room` (with its configuration), while still replying in the original room.
    pub fn retargeted(&self, target_room: Room, room_config_context: RoomConfigContext) -> Self {
        Self {
            room: self.room.clone(),
            target_room,
            room_config_context,
            admin_whitelist_regexes: self.admin_whitelist_regexes.clone(),
            trigger_event_info: self.trigger_event_info.clone(),
            thread_info: self.thread_info.clone(),

            bot_display_name: self.bot_display_name.clone(),
        }
    }

    pub fn with_bot_display_name(mut self, value: Option<String>) -> Self {
        self.bot_display_name = value;
        self
    }

    pub fn bot_display_name(&self) -> &Option<String> {
        &self.bot_display_name
    }

    /// The room the triggering message was sent in, where replies go.
    pub fn room(&self) -> &Room {
        &self.room
    }

    /// The room the command acts on (whose configuration `room_config()` returns).
    pub fn target_room(&self) -> &Room {
        &self.target_room
    }

    /// The command prefix to suggest for commands acting on the target room:
    /// the bot's prefix normally, or `<prefix> in <room-id>` when managing another room.
    pub fn command_prefix_for_target_room(&self, command_prefix: &str) -> String {
        if self.target_room.room_id() == self.room.room_id() {
            command_prefix.to_owned()
        } else {
            format!("{command_prefix} in {}", self.target_room.room_id())
        }
    }

    /// The ID of the room the triggering message was sent in (see `room()`), not of the target room.
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

    pub fn authorizer(&self) -> Authorizer<'_> {
        Authorizer::new(&self.admin_whitelist_regexes, self.global_config())
    }

    pub fn sender_can_manage_global_config(&self) -> bool {
        // This only depends on the static administrator patterns, so it cannot fail.
        matches!(
            self.authorizer()
                .allows(self.sender_id(), Permission::ManageGlobalConfig),
            Ok(true)
        )
    }

    pub fn sender_can_manage_room_local_agents(&self) -> mxidwc::Result<bool> {
        self.authorizer()
            .allows(self.sender_id(), Permission::ManageRoomLocalAgents)
    }

    pub fn combined_admin_and_user_regexes(&self) -> Vec<regex::Regex> {
        let authorizer = self.authorizer();

        match authorizer.context_author_regexes() {
            Ok(regexes) => regexes,
            Err(err) => {
                tracing::warn!(
                    "Error parsing user patterns for room {}: {:?}",
                    self.room.room_id(),
                    err
                );

                authorizer.admin_regexes()
            }
        }
    }
}
