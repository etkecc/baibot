use mxlink::ThreadInfo;

use super::MessagePayload;

pub struct InteractionContext {
    pub thread_info: ThreadInfo,
    pub trigger: InteractionTrigger,
}

pub struct InteractionTrigger {
    pub is_mentioning_bot: bool,
    pub payload: MessagePayload,
}
