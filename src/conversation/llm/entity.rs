use chrono::{DateTime, Utc};
use mxlink::matrix_sdk::ruma::events::room::message::ImageMessageEventContent;
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
    pub timestamp: DateTime<Utc>,
    pub content: MessageContent,
}

#[derive(Debug, Clone)]
pub struct ImageDetails {
    pub event_content: ImageMessageEventContent,
    pub mime: Mime,
    pub data: Vec<u8>,
}

impl ImageDetails {
    pub fn new(event_content: ImageMessageEventContent, mime: Mime, data: Vec<u8>) -> Self {
        Self { event_content, mime, data }
    }

    pub fn filename(&self) -> String {
        self.event_content.filename.clone().unwrap_or(self.event_content.body.clone())
    }
}

impl Into<ImageSource> for &ImageDetails {
    fn into(self) -> ImageSource {
        ImageSource::new(self.filename(), self.data.clone(), self.mime.clone())
    }
}

#[derive(Debug, Clone)]
pub enum MessageContent {
    Text(String),
    Image(ImageDetails),
}

impl PartialEq for MessageContent {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (MessageContent::Text(a), MessageContent::Text(b)) => a == b,
            (MessageContent::Image(a), MessageContent::Image(b)) => {
                // We can probably do better than this by inspecting `.event_conten1t.source`, but for now this is good enough.
                a.filename() == b.filename()
            },
            _ => false,
        }
    }
}
#[derive(Debug)]
pub struct Conversation {
    pub messages: Vec<Message>,
}

impl Conversation {
    /// Combine consecutive messages by the same author into a single message.
    ///
    /// Certain models (like Anthropic) cannot tolerate consecutive messages by the same author,
    /// so combining them helps avoid issues.
    /// See: https://github.com/etkecc/baibot/issues/13
    pub fn combine_consecutive_messages(&self) -> Conversation {
        // We'll likely get fewer messages, but let's reserve the maximum we expect.
        let mut new_messages = Vec::with_capacity(self.messages.len());
        let mut last_seen_text_from_author: Option<Author> = None;

        for message in &self.messages {
            let MessageContent::Text(message_text_content) = &message.content else {
                last_seen_text_from_author = None;
                new_messages.push(message.clone());
                continue;
            };

            let Some(last_seen_author_clone) = last_seen_text_from_author.clone() else {
                last_seen_text_from_author = Some(message.author.clone());
                new_messages.push(message.clone());
                continue;
            };

            if message.author != last_seen_author_clone {
                last_seen_text_from_author = Some(message.author.clone());
                new_messages.push(message.clone());
                continue;
            }

            let last_message = new_messages.last_mut().unwrap();
            if let MessageContent::Text(ref mut text) = last_message.content {
                text.push('\n');
                text.push_str(message_text_content);
            }
        }

        Conversation {
            messages: new_messages,
        }
    }

    pub fn start_time(&self) -> Option<DateTime<Utc>> {
        self.messages.first().map(|message| message.timestamp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use mxlink::matrix_sdk::ruma::OwnedMxcUri;
    use mxlink::mime;

    #[test]
    fn combine_consecutive_messages() {
        let timestamp_1 = Utc.with_ymd_and_hms(2024, 9, 20, 18, 34, 15).unwrap();

        let timestamp_2 = Utc.with_ymd_and_hms(2024, 9, 21, 18, 34, 16).unwrap();

        let timestamp_3 = Utc.with_ymd_and_hms(2024, 9, 22, 18, 34, 17).unwrap();

        let timestamp_4 = Utc.with_ymd_and_hms(2024, 9, 23, 18, 34, 18).unwrap();

        let image_event_content = ImageMessageEventContent::plain(
            "image.png".to_string(),
            OwnedMxcUri::from("mxc://example.com/1234567890"),
        );

        let conversation = Conversation {
            messages: vec![
                // User's turn
                Message {
                    author: Author::User,
                    content: MessageContent::Text("Hello".to_string()),
                    timestamp: timestamp_1,
                },
                Message {
                    author: Author::User,
                    content: MessageContent::Text("How are you?".to_string()),
                    timestamp: timestamp_2,
                },
                Message {
                    author: Author::User,
                    content: MessageContent::Text("I'm OK, btw.".to_string()),
                    timestamp: timestamp_3,
                },
                Message {
                    author: Author::User,
                    content: MessageContent::Image(ImageDetails::new(
                        image_event_content.clone(),
                        mime::IMAGE_PNG,
                        vec![],
                    )),
                    timestamp: timestamp_4,
                },
                Message {
                    author: Author::User,
                    content: MessageContent::Text("Above is an image.".to_string()),
                    timestamp: timestamp_4,
                },
                Message {
                    author: Author::User,
                    content: MessageContent::Text("Would you take a look at it?".to_string()),
                    timestamp: timestamp_4,
                },
                // Assistant's turn
                Message {
                    author: Author::Assistant,
                    content: MessageContent::Text("Hi there!".to_string()),
                    timestamp: timestamp_2,
                },
                Message {
                    author: Author::Assistant,
                    content: MessageContent::Text("I'm doing well, thank you.".to_string()),
                    timestamp: timestamp_3,
                },
                // User's turn
                Message {
                    author: Author::User,
                    content: MessageContent::Text("That's great!".to_string()),
                    timestamp: timestamp_3,
                },
            ],
        };

        let conversation = conversation.combine_consecutive_messages();

        assert_eq!(conversation.messages.len(), 5);

        assert_eq!(conversation.messages[0].author, Author::User);
        assert_eq!(
            conversation.messages[0].content,
            MessageContent::Text("Hello\nHow are you?\nI'm OK, btw.".to_string())
        );
        assert_eq!(conversation.messages[0].timestamp, timestamp_1);

        assert_eq!(conversation.messages[1].author, Author::User);
        assert_eq!(
            conversation.messages[1].content,
            MessageContent::Image(ImageDetails::new(
                image_event_content.clone(),
                mime::IMAGE_PNG,
                vec![],
            ))
        );

        assert_eq!(conversation.messages[2].author, Author::User);
        assert_eq!(
            conversation.messages[2].content,
            MessageContent::Text("Above is an image.\nWould you take a look at it?".to_string())
        );
        assert_eq!(conversation.messages[2].timestamp, timestamp_4);

        assert_eq!(conversation.messages[3].author, Author::Assistant);
        assert_eq!(
            conversation.messages[3].content,
            MessageContent::Text("Hi there!\nI'm doing well, thank you.".to_string())
        );
        assert_eq!(conversation.messages[3].timestamp, timestamp_2);

        assert_eq!(conversation.messages[4].author, Author::User);
        assert_eq!(
            conversation.messages[4].content,
            MessageContent::Text("That's great!".to_string())
        );
        assert_eq!(conversation.messages[4].timestamp, timestamp_3);
    }
}
