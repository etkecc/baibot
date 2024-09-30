use crate::agent::AgentPurpose;

use super::text::{block_quote, block_unquote};

/// Creates a text message which is based on transcribed audio.
/// This text message is prefixed with an emoji and blockquoted, to indicate that it is a transcription.
/// To reverse the process, use `parse_transcribed_message_text()`.
///
/// It should be noted that in certain cases (Transcribe-only mode), transcriptions are posted as regular notice messages which do not include
/// the `> 🦻` prefixing. That is, not every transcribed message will pass through here (intentionally).
pub fn create_transcribed_message_text(text: &str) -> String {
    block_quote(&format!("{} {}", AgentPurpose::SpeechToText.emoji(), text))
}

/// Parses a transcribed message text, reversing the process done by `create_transcribed_message_text()`.
/// If the provided text string does not match the expected format, None is returned.
///
/// It should be noted that in certain cases (Transcribe-only mode), transcriptions are posted as regular notice messages which do not include
/// the `> 🦻` prefixing. This function will not handle these properly.
pub fn parse_transcribed_message_text(text: &str) -> Option<String> {
    if !text.starts_with("> ") {
        return None;
    }

    let unquoted = block_unquote(text);

    let emoji_prefix = format!("{} ", AgentPurpose::SpeechToText.emoji());

    if let Some(original) = unquoted.strip_prefix(&emoji_prefix) {
        return Some(original.to_string());
    }

    None
}

pub mod test {
    #[test]
    fn test_transcribed_message_text_creation() {
        let text = "Hello there!\nHow are you?";
        let expected = format!(
            "> {} Hello there!\n> How are you?",
            crate::agent::AgentPurpose::SpeechToText.emoji()
        );
        assert_eq!(expected, super::create_transcribed_message_text(text));
    }

    #[test]
    fn test_transcribed_message_text_parsing() {
        // All good
        let text = format!(
            "> {} Hello there!\n> How are you?",
            crate::agent::AgentPurpose::SpeechToText.emoji()
        );
        let expected = "Hello there!\nHow are you?";
        assert_eq!(
            Some(expected.to_owned()),
            super::parse_transcribed_message_text(&text)
        );

        // No blockquote
        let text = format!(
            "{} Hello there!\nHow are you?",
            crate::agent::AgentPurpose::SpeechToText.emoji()
        );
        assert_eq!(None, super::parse_transcribed_message_text(&text));

        // No emoji
        let text = "> Hello there!\n> How are you?";
        assert_eq!(None, super::parse_transcribed_message_text(text));

        // Different emoji
        let text = "> 🌸 Hello there!\n> How are you?";
        assert_eq!(None, super::parse_transcribed_message_text(text));
    }
}
