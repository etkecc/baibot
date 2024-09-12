use regex::Regex;

#[derive(Clone)]
pub struct MatrixMessage {
    pub sender_id: String,
    pub message_type: MatrixMessageType,
    pub message_text: String,
}

#[derive(Clone)]
pub enum MatrixMessageType {
    Text,
    Notice,
}

#[derive(Default, Clone)]
pub struct MatrixMessageProcessingParams {
    pub(crate) bot_user_id: String,
    pub(crate) allowed_users: Vec<Regex>,

    // If non-empty, these prefixes will be stripped when processing the message
    pub(crate) first_message_stripped_prefixes: Vec<String>,
}

impl MatrixMessageProcessingParams {
    pub fn new(bot_user_id: String, allowed_users: Vec<Regex>) -> Self {
        Self {
            bot_user_id,
            allowed_users,
            ..Default::default()
        }
    }

    pub fn with_first_message_stripped_prefixes(mut self, value: Vec<String>) -> Self {
        self.first_message_stripped_prefixes = value;
        self
    }
}
