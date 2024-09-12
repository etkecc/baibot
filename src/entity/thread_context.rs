use mxlink::ThreadInfo;

use super::MessagePayload;

pub struct ThreadContext {
    pub info: ThreadInfo,
    pub first_message: ThreadContextFirstMessage,
}

pub struct ThreadContextFirstMessage {
    pub is_mentioning_bot: bool,
    pub payload: MessagePayload,
}
