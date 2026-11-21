use chrono::{DateTime, Utc};

use mxlink::matrix_sdk::ruma::events::room::message::{
    FileMessageEventContent, ImageMessageEventContent,
};
use mxlink::matrix_sdk::ruma::{MilliSecondsSinceUnixEpoch, OwnedEventId, OwnedUserId, UserId};
use mxlink::mime::Mime;

#[derive(Clone)]
pub struct MatrixMessage {
    pub sender_id: OwnedUserId,
    pub content: MatrixMessageContent,
    pub mentioned_users: Vec<OwnedUserId>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Clone)]
pub enum MatrixMessageContent {
    Text(String),
    Notice(String),
    Image(ImageMessageEventContent, Mime, Vec<u8>),
    File(FileMessageEventContent, Mime, Vec<u8>),
}

#[derive(Clone)]
pub struct MatrixMessageProcessingParams {
    pub(crate) bot_user_id: OwnedUserId,

    /// The prefixes that will be stripped when processing the messages in the context (thread or reply chain),
    /// which are found to be mentioning the bot user (`bot_user_id`).
    pub(crate) bot_user_prefixes_to_strip: Vec<String>,

    /// The prefixes that will be stripped when processing the 1st message in the context (thread or reply chain).
    pub(crate) first_message_prefixes_to_strip: Vec<String>,

    /// Selects the senders whose messages are included (the bot's own messages always are).
    pub(crate) sender_filter: SenderFilter,
}

/// Tells whether messages from a given sender are to be included in a conversation.
#[derive(Clone)]
pub struct SenderFilter(std::sync::Arc<dyn Fn(&UserId) -> bool + Send + Sync>);

impl SenderFilter {
    pub fn new(filter: impl Fn(&UserId) -> bool + Send + Sync + 'static) -> Self {
        Self(std::sync::Arc::new(filter))
    }

    pub fn allows(&self, user_id: &UserId) -> bool {
        (self.0)(user_id)
    }
}

impl MatrixMessageProcessingParams {
    pub fn new(bot_user_id: OwnedUserId, sender_filter: SenderFilter) -> Self {
        Self {
            bot_user_id,
            bot_user_prefixes_to_strip: vec![],

            first_message_prefixes_to_strip: vec![],

            sender_filter,
        }
    }

    pub fn with_bot_user_prefixes_to_strip(mut self, value: Vec<String>) -> Self {
        self.bot_user_prefixes_to_strip = value;
        self
    }

    pub fn with_first_message_prefixes_to_strip(mut self, value: Vec<String>) -> Self {
        self.first_message_prefixes_to_strip = value;
        self
    }
}

/// Where a thread conversation ends: at the event being answered.
///
/// Messages posted in the thread after it (e.g. while the bot is still fetching the thread) are left out,
/// so they can't steer the answer (or the tools it uses) under the triggering sender's permissions.
#[derive(Debug, Clone)]
pub struct ConversationBound {
    /// The event being answered (the last one included).
    pub event_id: OwnedEventId,
    /// The event's timestamp: earlier events are kept, later edits dropped.
    pub origin_server_ts: MilliSecondsSinceUnixEpoch,
    /// Later events which are still included, like the bot's transcription of a voice message being answered.
    pub also_include: Vec<OwnedEventId>,
}

impl ConversationBound {
    pub fn new(event_id: OwnedEventId, origin_server_ts: MilliSecondsSinceUnixEpoch) -> Self {
        Self {
            event_id,
            origin_server_ts,
            also_include: vec![],
        }
    }

    pub fn also_including(mut self, event_id: OwnedEventId) -> Self {
        self.also_include.push(event_id);
        self
    }
}
