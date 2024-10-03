use chrono::{DateTime, Utc};
use regex::Regex;

use mxlink::matrix_sdk::ruma::OwnedUserId;

#[derive(Clone)]
pub struct MatrixMessage {
    pub sender_id: OwnedUserId,
    pub message_type: MatrixMessageType,
    pub message_text: String,
    pub mentioned_users: Vec<OwnedUserId>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Clone)]
pub enum MatrixMessageType {
    Text,
    Notice,
}

#[derive(Clone)]
pub struct MatrixMessageProcessingParams {
    pub(crate) bot_user_id: OwnedUserId,

    /// The prefixes that will be stripped when processing the messages in the context (thread or reply chain),
    /// which are found to be mentioning the bot user (`bot_user_id`).
    pub(crate) bot_user_prefixes_to_strip: Vec<String>,

    /// The prefixes that will be stripped when processing the 1st message in the context (thread or reply chain).
    pub(crate) first_message_prefixes_to_strip: Vec<String>,

    /// A list of users whose messages are allowed.
    /// If None, all messages are allowed.
    /// If Some, only messages from the allowed users (and the bot itself, `bot_user_id`) are allowed.
    pub(crate) allowed_users: Option<Vec<Regex>>,
}

impl MatrixMessageProcessingParams {
    pub fn new(bot_user_id: OwnedUserId, allowed_users: Option<Vec<Regex>>) -> Self {
        Self {
            bot_user_id,
            bot_user_prefixes_to_strip: vec![],

            first_message_prefixes_to_strip: vec![],

            allowed_users,
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
