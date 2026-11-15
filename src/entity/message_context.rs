use mxlink::matrix_sdk::Room;
use mxlink::matrix_sdk::ruma::{OwnedEventId, OwnedUserId, RoomId};

use mxlink::ThreadInfo;

use crate::authorization::{Authorizer, Permission};
use crate::conversation::matrix::SenderFilter;

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
    /// Answers access questions for the target room.
    authorizer: Authorizer,
    trigger_event_info: TriggerEventInfo,
    thread_info: ThreadInfo,

    bot_display_name: Option<String>,
}

impl MessageContext {
    pub fn new(
        room: Room,
        room_config_context: RoomConfigContext,
        authorizer: Authorizer,
        trigger_event_info: TriggerEventInfo,
        thread_info: ThreadInfo,
    ) -> Self {
        Self {
            target_room: room.clone(),
            room,
            room_config_context,
            authorizer,
            trigger_event_info,
            thread_info,

            bot_display_name: None,
        }
    }

    /// Returns a copy of this context which acts on `target_room` (with its configuration and authorizer),
    /// while still replying in the original room.
    pub fn retargeted(
        &self,
        target_room: Room,
        room_config_context: RoomConfigContext,
        authorizer: Authorizer,
    ) -> Self {
        Self {
            room: self.room.clone(),
            target_room,
            room_config_context,
            authorizer,
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

    /// Answers access questions for the target room.
    pub fn authorizer(&self) -> &Authorizer {
        &self.authorizer
    }

    pub fn sender_is_admin(&self) -> bool {
        self.authorizer.is_admin(self.sender_id())
    }

    pub fn sender_can(&self, permission: Permission) -> bool {
        self.authorizer.allows(self.sender_id(), permission)
    }

    /// Selects the authors whose messages are included in conversation context for a feature.
    ///
    /// Normally, these are the authors who may use the feature in the room.
    /// When the sender explicitly pulls the bot into a conversation (`include_others`),
    /// everyone is included, except authors denied the feature.
    pub fn context_sender_filter(&self, feature: Permission, include_others: bool) -> SenderFilter {
        let authorizer = self.authorizer.clone();

        if include_others {
            SenderFilter::new(move |user_id| !authorizer.denies(user_id, feature))
        } else {
            SenderFilter::new(move |user_id| authorizer.allows(user_id, feature))
        }
    }
}
