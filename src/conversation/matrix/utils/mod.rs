#[cfg(test)]
mod tests;

use std::sync::Arc;

use mxlink::matrix_sdk::ruma::{OwnedEventId, OwnedUserId};
use mxlink::matrix_sdk::{
    ruma::events::{
        room::message::{
            MessageType, OriginalSyncRoomMessageEvent, Relation, RoomMessageEventContent,
        },
        AnyMessageLikeEvent, AnyMessageLikeEventContent, AnyTimelineEvent, MessageLikeEvent,
    },
    Room,
};
use mxlink::{MatrixLink, ThreadGetMessagesParams, ThreadInfo};

use super::{MatrixMessage, MatrixMessageProcessingParams, MatrixMessageType, RoomEventFetcher};
use crate::entity::{MessagePayload, ThreadContext, ThreadContextFirstMessage};

pub async fn get_matrix_messages_in_thread(
    matrix_link: MatrixLink,
    room: &Room,
    thread_id: OwnedEventId,
) -> Result<Vec<MatrixMessage>, mxlink::matrix_sdk::Error> {
    let messages_native = matrix_link
        .threads()
        .get_messages(room, thread_id, ThreadGetMessagesParams::default())
        .await?;

    let mut messages: Vec<MatrixMessage> = Vec::new();

    for matrix_native_message in messages_native {
        let Some(message) = convert_matrix_native_event_to_matrix_message(&matrix_native_message)
        else {
            continue;
        };

        messages.push(message);
    }

    Ok(messages)
}

pub async fn process_matrix_messages_in_thread(
    messages: &[MatrixMessage],
    params: &MatrixMessageProcessingParams,
) -> Vec<MatrixMessage> {
    let mut messages_filtered: Vec<MatrixMessage> = Vec::new();

    for (i, message) in messages.iter().enumerate() {
        if !is_message_from_allowed_sender(message, &params.bot_user_id, &params.allowed_users) {
            continue;
        }

        let mut message = message.clone();

        if i == 0 && !params.first_message_stripped_prefixes.is_empty() {
            let mut message_text = message.message_text.clone();

            for prefix in &params.first_message_stripped_prefixes {
                if let Some(message_text_stripped) = message_text.strip_prefix(prefix) {
                    message_text = message_text_stripped.to_owned();
                }
            }

            message.message_text = message_text.trim().to_owned();
        }

        messages_filtered.push(message);
    }

    messages_filtered
}

fn is_message_from_allowed_sender(
    matrix_message: &MatrixMessage,
    bot_user_id: &str,
    allowed_users: &[regex::Regex],
) -> bool {
    if matrix_message.sender_id == bot_user_id {
        return true;
    }

    if mxidwc::match_user_id(&matrix_message.sender_id, allowed_users) {
        return true;
    }

    false
}

pub fn convert_matrix_native_event_to_matrix_message(
    matrix_native_event: &AnyMessageLikeEvent,
) -> Option<MatrixMessage> {
    let Some(content) = matrix_native_event.original_content() else {
        // Redacted message
        return None;
    };

    let AnyMessageLikeEventContent::RoomMessage(room_message) = content else {
        // Some state event, etc.
        return None;
    };

    let (text, is_notice) = match &room_message.msgtype {
        MessageType::Text(text_content) => (text_content.body.clone(), false),
        MessageType::Notice(notice_content) => (notice_content.body.clone(), true),
        _ => return None,
    };

    Some(MatrixMessage {
        sender_id: matrix_native_event.sender().to_string(),
        message_type: if is_notice {
            MatrixMessageType::Notice
        } else {
            MatrixMessageType::Text
        },
        message_text: text,
    })
}

/// Determines the thread context (relationship within the thread + first thread message payload) for an incoming (new) room event.
/// This room event is assumed to be the "newest message" in the thread (or a top-level message).
/// If the given event is a regular reply (not a thread reply), this function will return `None`.
/// If the given event is a top-level message, this function will consider this event as the start of the thread.
/// If the given event is a thread reply, this function will inspect the thread root event and will return the thread context.
/// If the thread root event is not found, is redacted, or is of some unsupported MessagePayload type, this function will return `None`.
pub async fn determine_thread_context_for_room_event(
    bot_user_id: &OwnedUserId,
    room: &Room,
    current_event: &OriginalSyncRoomMessageEvent,
    current_event_payload: &MessagePayload,
    event_fetcher: &Arc<RoomEventFetcher>,
) -> anyhow::Result<Option<ThreadContext>> {
    let Some(relation) = &current_event.content.relates_to else {
        // This is a top-level message. We consider it the start of the thread.
        let thread_info = ThreadInfo::new(
            current_event.event_id.clone(),
            current_event.event_id.clone(),
        );

        let is_mentioning_bot = is_event_mentioning_bot(&current_event.content, bot_user_id);

        return Ok(Some(ThreadContext {
            info: thread_info,
            first_message: ThreadContextFirstMessage {
                is_mentioning_bot,
                payload: current_event_payload.clone(),
            },
        }));
    };

    let Relation::Thread(thread) = relation else {
        // This is a reply or a replacement, etc. It's not a thread.
        // We don't care about this.
        return Ok(None);
    };

    let thread_info = ThreadInfo::new(thread.event_id.clone(), current_event.event_id.clone());

    let start_time = std::time::Instant::now();

    let thread_start_timeline_event = event_fetcher
        .fetch_event_in_room(&thread.event_id, room)
        .await;

    let thread_start_timeline_event = match thread_start_timeline_event {
        Ok(value) => value,
        Err(err) => {
            return Err(anyhow::format_err!(
                "Failed to fetch thread start event {}: {:?}",
                thread.event_id,
                err
            ));
        }
    };

    let duration = start_time.elapsed();

    tracing::trace!(
        thread_id = thread.event_id.as_str(),
        duration = ?duration,
        "Fetched thread start event"
    );

    let thread_start_timeline_event_deserialized =
        match thread_start_timeline_event.event.deserialize() {
            Ok(value) => value,
            Err(err) => {
                return Err(anyhow::format_err!(
                    "Failed to deserialize thread start event {}: {:?}",
                    thread.event_id,
                    err
                ));
            }
        };

    let AnyTimelineEvent::MessageLike(thread_start_message_like_event) =
        thread_start_timeline_event_deserialized
    else {
        tracing::trace!(
            "Ignoring non-MessageLike thread start event: {:?}",
            thread_start_timeline_event_deserialized
        );
        return Ok(None);
    };

    let (thread_start_message_is_mentioning_bot, thread_start_message_payload) =
        match thread_start_message_like_event {
            AnyMessageLikeEvent::RoomEncrypted(room_message) => {
                tracing::warn!(
                    "Could not inspect thread start event {} because it failed to decrypt: {:?}",
                    thread.event_id.clone(),
                    room_message
                );

                // There's no way to know and it doesn't matter anyway.
                let is_mentioning_bot = false;

                (
                    is_mentioning_bot,
                    MessagePayload::Encrypted(thread_info.clone()),
                )
            }
            AnyMessageLikeEvent::RoomMessage(room_message) => {
                if let MessageLikeEvent::Original(room_message_original) = room_message {
                    let room_message_payload: Result<MessagePayload, String> =
                        room_message_original.content.msgtype.clone().try_into();

                    let Ok(room_message_payload) = room_message_payload else {
                        tracing::debug!(
                            msg_type = room_message_original.content.msgtype(),
                            "Ignoring thread start message of unknown type",
                        );
                        return Ok(None);
                    };

                    let is_mentioning_bot =
                        is_event_mentioning_bot(&room_message_original.content, bot_user_id);

                    (is_mentioning_bot, room_message_payload)
                } else {
                    tracing::error!("Ignoring thread start message which appears to be redacted");

                    return Ok(None);
                }
            }
            other => {
                tracing::trace!(
                    "Ignoring unknown MessageLike thread start event: {:?}",
                    other
                );
                return Ok(None);
            }
        };

    Ok(Some(ThreadContext {
        info: thread_info,
        first_message: ThreadContextFirstMessage {
            is_mentioning_bot: thread_start_message_is_mentioning_bot,
            payload: thread_start_message_payload,
        },
    }))
}

fn is_event_mentioning_bot(
    event_content: &RoomMessageEventContent,
    bot_user_id: &OwnedUserId,
) -> bool {
    if let Some(mentions) = &event_content.mentions {
        mentions
            .user_ids
            .iter()
            .any(|user_id| user_id == bot_user_id)
    } else {
        // For compatibility with clients that do not support the new Mentions specification
        // (see https://spec.matrix.org/latest/client-server-api/#user-and-room-mentions),
        // we also do string matching here.
        //
        // It may be even better to match not only against the MXID, but also against the bot's
        // room-specific display name.
        //
        // We may consider dropping this string-matching behavior altogether in the future,
        // so improving this compatibility block is not a high priority.
        event_content.body().contains(bot_user_id.as_str())
    }
}
