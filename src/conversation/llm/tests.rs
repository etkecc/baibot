use mxlink::matrix_sdk::ruma::OwnedUserId;

use crate::utils::status::create_error_message_text;
use crate::utils::text_to_speech::create_transcribed_message_text;

use super::*;

#[test]
fn test_messages_by_the_bot_are_identified_correctly() {
    let bot_user_id =
        OwnedUserId::try_from("@bot:example.com").expect("Failed to parse bot user ID");

    let matrix_message = super::super::matrix::MatrixMessage {
        sender_id: bot_user_id.to_owned(),
        message_type: super::super::matrix::MatrixMessageType::Text,
        message_text: "Hello!".to_owned(),
        mentioned_users: vec![],
        timestamp: chrono::Utc::now(),
    };

    let llm_message = convert_matrix_message_to_llm_message(&matrix_message, &bot_user_id).unwrap();

    assert_eq!(llm_message.author, Author::Assistant);
    assert_eq!(llm_message.message_text, "Hello!");
}

#[test]
fn test_notice_messages_by_bot_with_speech_to_text_prefix_are_cleaned_up_and_considered_sent_by_user(
) {
    let bot_user_id =
        OwnedUserId::try_from("@bot:example.com").expect("Failed to parse bot user ID");

    let source_message_text = "Hello!";
    let message_text = create_transcribed_message_text(source_message_text);

    assert_ne!(source_message_text, message_text);

    let matrix_message = super::super::matrix::MatrixMessage {
        sender_id: bot_user_id.to_owned(),
        message_type: super::super::matrix::MatrixMessageType::Notice,
        message_text,
        mentioned_users: vec![],
        timestamp: chrono::Utc::now(),
    };

    let llm_message = convert_matrix_message_to_llm_message(&matrix_message, &bot_user_id).unwrap();

    assert_eq!(llm_message.author, Author::User);
    assert_eq!(llm_message.message_text, source_message_text);
}

#[test]
fn test_notice_error_messages_by_bot_are_ignored() {
    let bot_user_id =
        OwnedUserId::try_from("@bot:example.com").expect("Failed to parse bot user ID");

    let source_message_text = "Some error happened";
    let message_text = create_error_message_text(source_message_text);

    assert_ne!(source_message_text, message_text);

    let matrix_message = super::super::matrix::MatrixMessage {
        sender_id: bot_user_id.to_owned(),
        message_type: super::super::matrix::MatrixMessageType::Notice,
        message_text,
        mentioned_users: vec![],
        timestamp: chrono::Utc::now(),
    };

    let llm_message = convert_matrix_message_to_llm_message(&matrix_message, &bot_user_id);

    assert!(llm_message.is_none());
}

#[test]
fn test_other_notice_messages_by_the_bot_are_ignored() {
    // Also see `test_notice_error_messages_by_bot_are_ignored()`.
    // That one passes accidentally, because we ignore all messages by the bot that are notices
    // (except for speech-to-text-created transcriptions - see `test_notice_messages_by_bot_with_speech_to_text_prefix_are_cleaned_up_and_considered_sent_by_user()`).
    // This test is to make sure that we don't accidentally start accepting other notice messages.

    let bot_user_id =
        OwnedUserId::try_from("@bot:example.com").expect("Failed to parse bot user ID");

    let message_text = "Something something";

    let matrix_message = super::super::matrix::MatrixMessage {
        sender_id: bot_user_id.to_owned(),
        message_type: super::super::matrix::MatrixMessageType::Notice,
        message_text: message_text.to_owned(),
        mentioned_users: vec![],
        timestamp: chrono::Utc::now(),
    };

    let llm_message = convert_matrix_message_to_llm_message(&matrix_message, &bot_user_id);

    assert!(llm_message.is_none());
}
