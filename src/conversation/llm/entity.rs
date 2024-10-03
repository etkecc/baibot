use chrono::{DateTime, Utc};

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
    pub timestamp: DateTime<Utc>,
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
        let mut last_seen_author: Option<Author> = None;

        for message in &self.messages {
            let Some(last_seen_author_clone) = last_seen_author.clone() else {
                last_seen_author = Some(message.author.clone());
                new_messages.push(message.clone());
                continue;
            };

            if message.author != last_seen_author_clone {
                last_seen_author = Some(message.author.clone());
                new_messages.push(message.clone());
                continue;
            }

            new_messages.last_mut().unwrap().message_text.push('\n');
            new_messages
                .last_mut()
                .unwrap()
                .message_text
                .push_str(&message.message_text);
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

    #[test]
    fn combine_consecutive_messages() {
        let timestamp_1 = Utc.with_ymd_and_hms(2024, 9, 20, 18, 34, 15).unwrap();

        let timestamp_2 = Utc.with_ymd_and_hms(2024, 9, 21, 18, 34, 15).unwrap();

        let timestamp_3 = Utc.with_ymd_and_hms(2024, 9, 22, 18, 34, 15).unwrap();

        let conversation = Conversation {
            messages: vec![
                // User's turn
                Message {
                    author: Author::User,
                    message_text: "Hello".to_string(),
                    timestamp: timestamp_1,
                },
                Message {
                    author: Author::User,
                    message_text: "How are you?".to_string(),
                    timestamp: timestamp_2,
                },
                Message {
                    author: Author::User,
                    message_text: "I'm OK, btw.".to_string(),
                    timestamp: timestamp_3,
                },
                // Assistant's turn
                Message {
                    author: Author::Assistant,
                    message_text: "Hi there!".to_string(),
                    timestamp: timestamp_2,
                },
                Message {
                    author: Author::Assistant,
                    message_text: "I'm doing well, thank you.".to_string(),
                    timestamp: timestamp_3,
                },
                // User's turn
                Message {
                    author: Author::User,
                    message_text: "That's great!".to_string(),
                    timestamp: timestamp_3,
                },
            ],
        };

        let conversation = conversation.combine_consecutive_messages();

        assert_eq!(conversation.messages.len(), 3);
        assert_eq!(conversation.messages[0].author, Author::User);
        assert_eq!(
            conversation.messages[0].message_text,
            "Hello\nHow are you?\nI'm OK, btw."
        );
        assert_eq!(conversation.messages[0].timestamp, timestamp_1);

        assert_eq!(conversation.messages[1].author, Author::Assistant);
        assert_eq!(
            conversation.messages[1].message_text,
            "Hi there!\nI'm doing well, thank you."
        );
        assert_eq!(conversation.messages[1].timestamp, timestamp_2);

        assert_eq!(conversation.messages[2].author, Author::User);
        assert_eq!(conversation.messages[2].message_text, "That's great!");
        assert_eq!(conversation.messages[2].timestamp, timestamp_3);
    }
}
