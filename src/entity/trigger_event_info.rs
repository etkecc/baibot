use mxlink::matrix_sdk::ruma::{OwnedEventId, OwnedUserId};

use super::MessagePayload;

#[derive(Debug, Clone)]
pub struct TriggerEventInfo {
    pub event_id: OwnedEventId,
    pub sender: OwnedUserId,
    pub payload: MessagePayload,
    pub sender_is_admin: bool,
}

impl TriggerEventInfo {
    pub fn new(
        event_id: OwnedEventId,
        sender: OwnedUserId,
        payload: MessagePayload,
        sender_is_admin: bool,
    ) -> Self {
        Self {
            event_id,
            sender,
            payload,
            sender_is_admin,
        }
    }
}
