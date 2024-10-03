#[cfg(test)]
mod tests;

use std::sync::Arc;

use mxlink::matrix_sdk::ruma::{OwnedEventId, OwnedUserId};
use mxlink::matrix_sdk::{
    deserialized_responses::TimelineEvent,
    ruma::events::{
        relation::Thread,
        room::message::{
            sanitize::remove_plain_reply_fallback, MessageType, OriginalSyncRoomMessageEvent,
            Relation, RoomMessageEventContent,
        },
        AnyMessageLikeEvent, AnyMessageLikeEventContent, AnyTimelineEvent, MessageLikeEvent,
    },
    Room,
};
use mxlink::{MatrixLink, ThreadGetMessagesParams, ThreadInfo};

use super::{MatrixMessage, MatrixMessageProcessingParams, MatrixMessageType, RoomEventFetcher};
use crate::entity::{InteractionContext, InteractionTrigger, MessagePayload};

struct DetailedMessagePayload {
    is_mentioning_bot: bool,
    message_payload: MessagePayload,
}

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

pub async fn get_matrix_messages_in_reply_chain(
    event_fetcher: &Arc<RoomEventFetcher>,
    room: &Room,
    event_id: OwnedEventId,
) -> Result<Vec<MatrixMessage>, mxlink::matrix_sdk::Error> {
    let messages_native =
        get_matrix_messages_in_reply_chain_native(event_fetcher, room, event_id).await?;

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

async fn get_matrix_messages_in_reply_chain_native(
    event_fetcher: &Arc<RoomEventFetcher>,
    room: &Room,
    event_id: OwnedEventId,
) -> Result<Vec<AnyMessageLikeEvent>, mxlink::matrix_sdk::Error> {
    let mut next_event_id = Some(event_id.clone());

    let mut messages: Vec<AnyMessageLikeEvent> = Vec::new();
    let mut handled_event_ids: Vec<OwnedEventId> = Vec::new();

    while let Some(next_event_id_in_loop) = next_event_id {
        let event = event_fetcher
            .fetch_event_in_room(&next_event_id_in_loop, room)
            .await
            .unwrap();

        if handled_event_ids.contains(&next_event_id_in_loop) {
            tracing::warn!(
                "Not following loop-causing event: {}",
                next_event_id_in_loop
            );
            break;
        }

        handled_event_ids.push(next_event_id_in_loop.clone());

        let event_deserialized = event.event.deserialize()?;

        let AnyTimelineEvent::MessageLike(message_like_event) = event_deserialized else {
            tracing::warn!(
                "Not proceeding past non-MessageLike event: {:?}",
                event_deserialized
            );
            break;
        };

        next_event_id = match message_like_event.clone() {
            AnyMessageLikeEvent::RoomEncrypted(_) => None,
            AnyMessageLikeEvent::RoomMessage(room_message) => {
                if let MessageLikeEvent::Original(room_message_original) = room_message {
                    match room_message_original.content.relates_to {
                        Some(Relation::Reply { in_reply_to }) => Some(in_reply_to.event_id.clone()),
                        _ => None,
                    }
                } else {
                    None
                }
            }
            _ => None,
        };

        messages.push(message_like_event);
    }

    messages.reverse();

    Ok(messages)
}

pub async fn process_matrix_messages(
    messages: &[MatrixMessage],
    params: &MatrixMessageProcessingParams,
) -> Vec<MatrixMessage> {
    let mut messages_filtered: Vec<MatrixMessage> = Vec::new();

    for (i, message) in messages.iter().enumerate() {
        if !is_message_from_allowed_sender(
            message,
            &params.bot_user_id,
            params.allowed_users.as_deref(),
        ) {
            continue;
        }

        let mut message = message.clone();

        if i == 0 && !params.first_message_prefixes_to_strip.is_empty() {
            let mut message_text = message.message_text.clone();

            for prefix in &params.first_message_prefixes_to_strip {
                if let Some(message_text_stripped) = message_text.strip_prefix(prefix) {
                    message_text = message_text_stripped.to_owned();
                }
            }

            message.message_text = message_text.trim().to_owned();
        }

        // We only strip `bot_user_prefixes_to_strip`-defined prefixes from messages that mention the bot user.
        if !params.bot_user_prefixes_to_strip.is_empty()
            && message.mentioned_users.contains(&params.bot_user_id)
        {
            let mut message_text = message.message_text.clone();

            for prefix in &params.bot_user_prefixes_to_strip {
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

/// Tells if the given message is from an allowed sender.
///
/// If allowed_users is None, all messages are allowed.
/// If allowed_users is Some, only messages from the allowed users (and the `bot_user_id`) are allowed.
fn is_message_from_allowed_sender(
    matrix_message: &MatrixMessage,
    bot_user_id: &OwnedUserId,
    allowed_users: Option<&[regex::Regex]>,
) -> bool {
    if matrix_message.sender_id == *bot_user_id {
        return true;
    }

    if let Some(allowed_users) = allowed_users {
        if mxidwc::match_user_id(matrix_message.sender_id.as_str(), allowed_users) {
            return true;
        }
    } else {
        // No allowed users configured, so all messages are allowed
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

    let is_reply = matches!(room_message.relates_to, Some(Relation::Reply { .. }));

    let text = if is_reply {
        // For regular replies, we need to strip the fallback-for-rich replies part.
        // See: https://spec.matrix.org/v1.11/client-server-api/#fallbacks-for-rich-replies
        remove_plain_reply_fallback(&text).to_owned()
    } else {
        text
    };

    let timestamp = chrono::DateTime::<chrono::Utc>::from(
        matrix_native_event
            .origin_server_ts()
            .to_system_time()
            .unwrap_or_else(std::time::SystemTime::now),
    );

    let mentioned_users = room_message
        .mentions
        .map(|m| m.user_ids.iter().map(|u| u.to_owned()).collect())
        .unwrap_or(vec![]);

    Some(MatrixMessage {
        sender_id: matrix_native_event.sender().to_owned(),
        message_type: if is_notice {
            MatrixMessageType::Notice
        } else {
            MatrixMessageType::Text
        },
        message_text: text,
        mentioned_users,
        timestamp,
    })
}

/// Determines the interaction context for an incoming (new) room event.
///
/// This context is created based on the "newest message" (`current_event`), which is:
/// - either a top-level message, which may or may not be mentioning the bot
///     - this function will inspect the event and will likely start a new threaded conversation
///
/// - or a thread reply
///     - this function will inspect the thread root event and will return the interaction context
///     - if the bot only reacts to prefixed messsages (or mentions), this function may ignore the given thread reply, unless it mentions the bot (which causes a synthetic "first message" to be produced)
///     - if the thread root event is not found, is redacted, or is of some unsupported MessagePayload type, this function will return `None`
///
/// - or an in-room (non-threaded) reply to a room message, which may or may not be mentioning the bot
///     - replies that do not mention the bot cause this function to return `None`
///     - other replies create a interaction context which points to a "first message" which is synthetic
#[tracing::instrument(name = "determine_interaction_context_for_room_event", skip_all, fields(room_id = room.room_id().as_str(), event_id = current_event.event_id.as_str()))]
pub async fn determine_interaction_context_for_room_event(
    bot_user_id: &OwnedUserId,
    room: &Room,
    current_event: &OriginalSyncRoomMessageEvent,
    current_event_payload: &MessagePayload,
    event_fetcher: &Arc<RoomEventFetcher>,
) -> anyhow::Result<Option<InteractionContext>> {
    let current_event_is_mentioning_bot =
        is_event_mentioning_bot(&current_event.content, bot_user_id);

    let Some(relation) = &current_event.content.relates_to else {
        // This is a top-level message. We consider it the start of the thread.
        let thread_info = ThreadInfo::new(
            current_event.event_id.clone(),
            current_event.event_id.clone(),
        );

        return Ok(Some(InteractionContext {
            thread_info,
            trigger: InteractionTrigger {
                is_mentioning_bot: current_event_is_mentioning_bot,
                payload: current_event_payload.clone(),
            },
        }));
    };

    match relation {
        Relation::Thread(thread) => {
            determine_interaction_context_for_room_event_related_to_thread(
                bot_user_id,
                room,
                current_event,
                event_fetcher,
                current_event_is_mentioning_bot,
                thread,
            )
            .await
        }
        Relation::Reply { in_reply_to } => {
            determine_interaction_context_for_room_event_related_to_reply(
                current_event,
                current_event_is_mentioning_bot,
                in_reply_to.event_id.clone(),
            )
            .await
        }

        // This is a replacement or something else. It's not something we support.
        _ => return Ok(None),
    }
}

async fn determine_interaction_context_for_room_event_related_to_thread(
    bot_user_id: &OwnedUserId,
    room: &Room,
    current_event: &OriginalSyncRoomMessageEvent,
    event_fetcher: &Arc<RoomEventFetcher>,
    current_event_is_mentioning_bot: bool,
    thread: &Thread,
) -> anyhow::Result<Option<InteractionContext>> {
    let thread_info = ThreadInfo::new(thread.event_id.clone(), current_event.event_id.clone());

    tracing::trace!(
        ?current_event_is_mentioning_bot,
        is_thread_root_only = thread_info.is_thread_root_only(),
        "Dealing with a thread reply",
    );

    if current_event_is_mentioning_bot && !thread_info.is_thread_root_only() {
        // If the current event is a thread reply and is mentioning the bot,
        // it's probably someone trying to involve us in the threaded conversation.
        // See: https://github.com/etkecc/baibot/issues/15
        //
        // In such cases, we don't care what the thread root event is like or what the current event is like,
        // we want text-generation to be triggered for this whole thread regardless.
        return Ok(Some(InteractionContext {
            thread_info,
            trigger: InteractionTrigger {
                is_mentioning_bot: true,
                payload: MessagePayload::SynthethicChatCompletionTriggerInThread,
            },
        }));
    }

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

    let thread_start_detailed_message_payload = timeline_event_to_detailed_message_payload(
        &thread.event_id,
        thread_start_timeline_event,
        thread_info.clone(),
        bot_user_id,
    )?;

    let Some(detailed_message_payload) = thread_start_detailed_message_payload else {
        return Ok(None);
    };

    Ok(Some(InteractionContext {
        thread_info,
        trigger: InteractionTrigger {
            is_mentioning_bot: detailed_message_payload.is_mentioning_bot,
            payload: detailed_message_payload.message_payload,
        },
    }))
}

async fn determine_interaction_context_for_room_event_related_to_reply(
    current_event: &OriginalSyncRoomMessageEvent,
    current_event_is_mentioning_bot: bool,
    reply_to_event_id: OwnedEventId,
) -> anyhow::Result<Option<InteractionContext>> {
    tracing::trace!(?current_event_is_mentioning_bot, "Dealing with a reply");

    if !current_event_is_mentioning_bot {
        // If the current event is not mentioning the bot, we don't care about it.
        tracing::trace!("Ignoring reply event which does not mention the bot");
        return Ok(None);
    }

    let thread_info = ThreadInfo::new(reply_to_event_id.clone(), current_event.event_id.clone());

    Ok(Some(InteractionContext {
        thread_info,
        trigger: InteractionTrigger {
            is_mentioning_bot: true,
            payload: MessagePayload::SynthethicChatCompletionTriggerForReply,
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
        // As of 2024-10-03, at least Element iOS does not support the new Mentions specification
        // and is still quite widespread.
        //
        // It may be even better to match not only against the MXID, but also against the bot's
        // room-specific display name.
        //
        // We may consider dropping this string-matching behavior altogether in the future,
        // so improving this compatibility block is not a high priority.
        event_content.body().contains(bot_user_id.as_str())
    }
}

fn timeline_event_to_detailed_message_payload(
    timeline_event_id: &OwnedEventId,
    timeline_event: TimelineEvent,
    thread_info: ThreadInfo,
    bot_user_id: &OwnedUserId,
) -> anyhow::Result<Option<DetailedMessagePayload>> {
    let timeline_event_deserialized = match timeline_event.event.deserialize() {
        Ok(value) => value,
        Err(err) => {
            return Err(anyhow::format_err!(
                "Failed to deserialize timeline event {}: {:?}",
                timeline_event_id,
                err
            ));
        }
    };

    let AnyTimelineEvent::MessageLike(thread_start_message_like_event) =
        timeline_event_deserialized
    else {
        tracing::trace!(
            "Ignoring non-MessageLike timeline event: {:?}",
            timeline_event_deserialized
        );
        return Ok(None);
    };

    let (is_mentioning_bot, message_payload) = match thread_start_message_like_event {
        AnyMessageLikeEvent::RoomEncrypted(room_message) => {
            tracing::warn!(
                "Could not inspect event {} because it failed to decrypt: {:?}",
                timeline_event_id.clone(),
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
                        "Ignoring event message of unknown type",
                    );
                    return Ok(None);
                };

                let is_mentioning_bot =
                    is_event_mentioning_bot(&room_message_original.content, bot_user_id);

                (is_mentioning_bot, room_message_payload)
            } else {
                tracing::error!("Ignoring event message which appears to be redacted");

                return Ok(None);
            }
        }
        other => {
            tracing::trace!("Ignoring unknown MessageLike event: {:?}", other);
            return Ok(None);
        }
    };

    Ok(Some(DetailedMessagePayload {
        is_mentioning_bot,
        message_payload,
    }))
}

/// Creates a list of prefixes to strip from the beginning of message texts that mention the bot user.
///
/// Different clients do mentions differently.
/// The body text containing the mention usually contains one of:
/// - the full user ID (includes a @ prefix by default)
/// - the localpart (with a @ prefix)
/// - the localpart (without a @ prefix)
/// - the display name (with a @ prefix)
/// - the display name (without a @ prefix)
///
/// Some add a `: ` suffix after the mention.
///
/// There's no guarantee that the mention is at the start even.
/// It being there is most common and we try to strip it from there
/// as best as we can.
pub fn create_list_of_bot_user_prefixes_to_strip(
    bot_user_id: &OwnedUserId,
    bot_display_name: &Option<String>,
) -> Vec<String> {
    let bot_user_id_localpart = bot_user_id.localpart();

    let mut prefixes_to_strip = vec![
        bot_user_id.as_str().to_owned(),
        format!("@{}", bot_user_id_localpart),
        bot_user_id_localpart.to_owned(),
    ];

    if let Some(bot_display_name) = bot_display_name {
        prefixes_to_strip.push(format!("@{}", bot_display_name));
        prefixes_to_strip.push(bot_display_name.to_owned());
    }

    prefixes_to_strip.push(":".to_owned());

    prefixes_to_strip
}
