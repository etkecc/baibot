use mxlink::matrix_sdk::ruma::{MilliSecondsSinceUnixEpoch, OwnedEventId, OwnedUserId};

use super::MessagePayload;

#[derive(Debug, Clone)]
pub struct TriggerEventInfo {
    pub event_id: OwnedEventId,
    pub sender: OwnedUserId,
    pub origin_server_ts: MilliSecondsSinceUnixEpoch,
    pub payload: MessagePayload,
}

impl TriggerEventInfo {
    pub fn new(
        event_id: OwnedEventId,
        sender: OwnedUserId,
        origin_server_ts: MilliSecondsSinceUnixEpoch,
        payload: MessagePayload,
    ) -> Self {
        Self {
            event_id,
            sender,
            origin_server_ts,
            payload,
        }
    }
}
