use chrono::{DateTime, Utc};
use mxlink::matrix_sdk::ruma::OwnedUserId;
use mxlink::matrix_sdk::ruma::events::room::message::{
    FileMessageEventContent, ImageMessageEventContent,
};
use mxlink::mime::Mime;

use crate::agent::provider::ImageSource;

#[derive(Debug, Clone, PartialEq)]
pub enum Author {
    Prompt,
    Assistant,
    User,
}

#[derive(Debug, Clone)]
pub struct Message {
    pub author: Author,
    pub sender_id: Option<OwnedUserId>,
    pub timestamp: DateTime<Utc>,
    pub content: MessageContent,
}

#[derive(Debug, Clone)]
pub struct ImageDetails {
    pub event_content: ImageMessageEventContent,
    pub mime: Mime,
    /// Shared, so that copies (e.g. kept for tools) are cheap.
    pub data: std::sync::Arc<[u8]>,
}

impl ImageDetails {
    /// `data` may be a slice or a `Vec` (both copied once into the shared buffer).
    pub fn new(
        event_content: ImageMessageEventContent,
        mime: Mime,
        data: impl Into<std::sync::Arc<[u8]>>,
    ) -> Self {
        Self {
            event_content,
            mime,
            data: data.into(),
        }
    }

    pub fn filename(&self) -> String {
        self.event_content
            .filename
            .clone()
            .unwrap_or(self.event_content.body.clone())
    }
}

impl From<ImageDetails> for ImageSource {
    fn from(value: ImageDetails) -> Self {
        ImageSource::new(value.filename(), value.data.to_vec(), value.mime.clone())
    }
}

#[derive(Debug, Clone)]
pub struct FileDetails {
    pub event_content: FileMessageEventContent,
    pub mime: Mime,
    pub data: Vec<u8>,
}

impl FileDetails {
    pub fn new(event_content: FileMessageEventContent, mime: Mime, data: Vec<u8>) -> Self {
        Self {
            event_content,
            mime,
            data,
        }
    }

    pub fn filename(&self) -> String {
        self.event_content
            .filename
            .clone()
            .unwrap_or(self.event_content.body.clone())
    }
}

#[derive(Debug, Clone)]
pub enum MessageContent {
    Text(String),
    Image(ImageDetails),
    File(FileDetails),
}

impl PartialEq for MessageContent {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (MessageContent::Text(a), MessageContent::Text(b)) => a == b,
            (MessageContent::Image(a), MessageContent::Image(b)) => {
                // We can probably do better than this by inspecting `.event_conten1t.source`, but for now this is good enough.
                a.filename() == b.filename()
            }
            (MessageContent::File(a), MessageContent::File(b)) => a.filename() == b.filename(),
            _ => false,
        }
    }
}
#[derive(Debug, Clone, Default)]
pub struct Conversation {
    pub messages: Vec<Message>,
}

impl Conversation {
    pub fn start_time(&self) -> Option<DateTime<Utc>> {
        self.messages.first().map(|message| message.timestamp)
    }
}
