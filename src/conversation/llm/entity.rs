#[derive(Debug, Clone, PartialEq)]
pub enum Author {
    Prompt,
    Assistant,
    User,
}

#[derive(Debug, Clone)]
pub struct Message {
    pub author: Author,
    pub message_text: String,
}

#[derive(Debug)]
pub struct Conversation {
    pub messages: Vec<Message>,
}
