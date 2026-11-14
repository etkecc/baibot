use mxlink::matrix_sdk::ruma::{OwnedEventId, OwnedUserId};

use super::MessagePayload;

#[derive(Debug)]
pub struct TriggerEventInfo {
    pub event_id: OwnedEventId,
    pub sender: OwnedUserId,
    pub payload: MessagePayload,
}

impl TriggerEventInfo {
    pub fn new(event_id: OwnedEventId, sender: OwnedUserId, payload: MessagePayload) -> Self {
        Self {
            event_id,
            sender,
            payload,
        }
    }
}
