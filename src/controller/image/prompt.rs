use crate::conversation::llm::{Author, Message, MessageContent};

/// Builds a prompt from the original prompt and other messages in the conversation.
///
/// Only messages authored by the user are considered.
///
/// Messages that say "Again" or "Go" (regardless of casing) are ignored. They are considered special messages
/// which trigger re-generation and "start" respectively, and do not need to be included in the prompt criteria.
pub fn build(original_prompt: &str, other_messages: Vec<Message>) -> String {
    let mut prompt = original_prompt.to_owned();

    // Make a new messages vector that only contains messages we care about
    let other_messages: Vec<Message> = other_messages
        .into_iter()
        .filter(|message| {
            if let Author::User = message.author {
                if let MessageContent::Text(text) = &message.content {
                    text.to_lowercase() != "again" && text.to_lowercase() != "go"
                } else {
                    false
                }
            } else {
                false
            }
        })
        .collect();

    if !other_messages.is_empty() {
        prompt.push_str("\nOther criteria:");
        for message in other_messages {
            if let MessageContent::Text(text) = &message.content {
                prompt.push_str(format!("\n- {}", text.replace("\n", ". ").as_str()).as_str());
            }
        }
    }

    prompt
}

#[cfg(test)]
mod tests {
    use super::build;
    use super::{Author, Message, MessageContent};

    struct TestCase {
        original_prompt: &'static str,
        messages: Vec<Message>,
        expected_prompt: &'static str,
    }

    #[test]
    fn test_build_prompt() {
        let timestamp = chrono::Utc::now();

        let test_cases = vec![
            // Simple case
            TestCase {
                original_prompt: "Generate a picture of a cat",
                messages: vec![],
                expected_prompt: "Generate a picture of a cat",
            },
            // Only a single user message
            TestCase {
                original_prompt: "Generate a picture of a dog",
                messages: vec![Message {
                    author: Author::User,
                    content: MessageContent::Text("Must be blue".to_owned()),
                    timestamp,
                }],
                expected_prompt: "Generate a picture of a dog\nOther criteria:\n- Must be blue",
            },
            // Multiple complex user messages dispersed with assistant messages
            TestCase {
                original_prompt: "Generate a picture of an elephant",
                messages: vec![
                    Message {
                        author: Author::User,
                        content: MessageContent::Text("Must be blue".to_owned()),
                        timestamp,
                    },
                    Message {
                        author: Author::Assistant,
                        content: MessageContent::Text("Whatever".to_owned()),
                        timestamp,
                    },
                    Message {
                        author: Author::User,
                        content: MessageContent::Text(
                            "Must be 3-legged.\nMust be flying.".to_owned(),
                        ),
                        timestamp,
                    },
                ],
                expected_prompt: "Generate a picture of an elephant\nOther criteria:\n- Must be blue\n- Must be 3-legged.. Must be flying.",
            },
            // "Again" is ignored.
            TestCase {
                original_prompt: "Generate a picture of a grizzly bear",
                messages: vec![
                    Message {
                        author: Author::User,
                        content: MessageContent::Text("Must be blue".to_owned()),
                        timestamp,
                    },
                    Message {
                        author: Author::Assistant,
                        content: MessageContent::Text("Whatever".to_owned()),
                        timestamp,
                    },
                    Message {
                        author: Author::User,
                        content: MessageContent::Text("Again".to_owned()),
                        timestamp,
                    },
                    Message {
                        author: Author::User,
                        content: MessageContent::Text("again".to_owned()),
                        timestamp,
                    },
                ],
                expected_prompt: "Generate a picture of a grizzly bear\nOther criteria:\n- Must be blue",
            },
        ];

        for test_case in test_cases {
            let actual_prompt = build(test_case.original_prompt, test_case.messages);

            assert_eq!(actual_prompt, test_case.expected_prompt);
        }
    }
}
