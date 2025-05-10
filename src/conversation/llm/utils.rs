use mxlink::matrix_sdk::ruma::OwnedUserId;

use super::entity::{Author, ImageDetails, Message, MessageContent};
use crate::conversation::matrix::{MatrixMessage, MatrixMessageContent};
use crate::utils::text_to_speech as text_to_speech_utils;

pub fn convert_matrix_message_to_llm_message(
    matrix_message: &MatrixMessage,
    bot_user_id: &OwnedUserId,
) -> Option<Message> {
    if matrix_message.sender_id == bot_user_id.as_str() {
        return convert_bot_message(matrix_message);
    }

    convert_user_message(matrix_message)
}

fn convert_bot_message(matrix_message: &MatrixMessage) -> Option<Message> {
    match &matrix_message.content {
        MatrixMessageContent::Text(text) => {
            convert_bot_text_message(text, &matrix_message.timestamp)
        }
        MatrixMessageContent::Notice(text) => {
            convert_bot_notice_message(text, &matrix_message.timestamp)
        }
        MatrixMessageContent::Image(image_content, mime_type, media_bytes) => Some(Message {
            author: Author::Assistant,
            content: MessageContent::Image(ImageDetails::new(
                image_content.clone(),
                mime_type.clone(),
                media_bytes.clone(),
            )),
            timestamp: matrix_message.timestamp.to_owned(),
        }),
    }
}

fn convert_bot_text_message(
    text: &str,
    timestamp: &chrono::DateTime<chrono::Utc>,
) -> Option<Message> {
    Some(Message {
        author: Author::Assistant,
        content: MessageContent::Text(text.to_owned()),
        timestamp: timestamp.to_owned(),
    })
}

fn convert_bot_notice_message(
    text: &str,
    timestamp: &chrono::DateTime<chrono::Utc>,
) -> Option<Message> {
    // Notice messages sent by the bot are usually transcriptions of previous messages sent by the user.
    // Such transcriptions are prefixed with an emoji and blockquoted.
    // If we find a notice that doesn't match this pattern, we skip it.
    //
    // It should be noted that transcriptions are sometimes posted as regular notice (or even text) messages which do not include
    // the `> 🦻` formatting. This function will not handle these properly.

    if let Some(text) = text_to_speech_utils::parse_transcribed_message_text(text) {
        // This is a transcription message. We remove the prefix and consider it as a message sent by the user.
        return Some(Message {
            author: Author::User,
            content: MessageContent::Text(text.to_owned()),
            timestamp: timestamp.to_owned(),
        });
    }

    None
}

fn convert_user_message(matrix_message: &MatrixMessage) -> Option<Message> {
    match &matrix_message.content {
        MatrixMessageContent::Text(text) => Some(Message {
            author: Author::User,
            content: MessageContent::Text(text.clone()),
            timestamp: matrix_message.timestamp.to_owned(),
        }),
        MatrixMessageContent::Notice(text) => Some(Message {
            author: Author::User,
            content: MessageContent::Text(text.clone()),
            timestamp: matrix_message.timestamp.to_owned(),
        }),
        MatrixMessageContent::Image(image_content, mime_type, media_bytes) => Some(Message {
            author: Author::User,
            content: MessageContent::Image(ImageDetails::new(
                image_content.clone(),
                mime_type.clone(),
                media_bytes.clone(),
            )),
            timestamp: matrix_message.timestamp.to_owned(),
        }),
    }
}
