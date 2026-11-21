#[cfg(test)]
mod tests;

use std::sync::Arc;

use mxlink::matrix_sdk::ruma::{EventId, OwnedEventId, OwnedUserId, UserId};
use mxlink::matrix_sdk::{
    Room,
    deserialized_responses::TimelineEvent,
    ruma::events::{
        AnySyncMessageLikeEvent, AnySyncTimelineEvent, SyncMessageLikeEvent,
        relation::Thread,
        room::message::{
            MessageType, OriginalSyncRoomMessageEvent, Relation, RoomMessageEventContent,
            sanitize::remove_plain_reply_fallback,
        },
    },
};
use mxlink::{MatrixLink, ThreadGetMessagesParams, ThreadInfo};
use tracing::Instrument;

use super::{
    ConversationBound, MatrixMessage, MatrixMessageContent, MatrixMessageProcessingParams,
    RoomEventFetcher,
};
use crate::entity::{InteractionContext, InteractionTrigger, MessagePayload};
use crate::utils::mime::get_mime_type_from_file_name;

struct DetailedMessagePayload {
    is_mentioning_bot: bool,
    message_payload: MessagePayload,
}

/// Returns the thread's messages (oldest first), up to the given bound (see [`ConversationBound`]).
pub async fn get_matrix_messages_in_thread(
    matrix_link: &MatrixLink,
    room: &Room,
    thread_id: OwnedEventId,
    bound: &ConversationBound,
) -> Result<Vec<MatrixMessage>, mxlink::matrix_sdk::Error> {
    let messages_native = matrix_link
        .threads()
        .get_messages(room, thread_id.clone(), ThreadGetMessagesParams::default())
        .await?;

    let (mut messages_native, ending_events) =
        bound_thread_events(messages_native, &thread_id, bound, matrix_link.user_id());

    // The conversation ends with the event being answered (and our transcription of it), in that order.
    // The server may not list those in the thread yet, so we fetch them ourselves if needed.
    // Without them there's nothing to answer, so failing to fetch them (or finding them deleted in the meantime)
    // fails the conversation. Otherwise, it would end with an earlier message, which is not what we're answering.
    for (event_id, event) in ending_events {
        let event = match event {
            Some(event) => event,
            None => {
                tracing::debug!(
                    event_id = event_id.as_str(),
                    "An event the conversation ends with is not listed in its thread yet, so fetching it"
                );

                let AnySyncTimelineEvent::MessageLike(event) =
                    room.event(&event_id, None).await?.raw().deserialize()?
                else {
                    continue;
                };

                without_late_edit(event, bound, matrix_link.user_id())
            }
        };

        if event.is_redacted() {
            return Err(mxlink::matrix_sdk::Error::UnknownError(
                "the message being answered was deleted".into(),
            ));
        }

        messages_native.push(event);
    }

    let mut messages: Vec<MatrixMessage> = Vec::new();

    for matrix_native_message in messages_native {
        let message_result =
            convert_matrix_native_event_to_matrix_message(matrix_link, &matrix_native_message)
                .await?;

        if let Some(message) = message_result {
            messages.push(message);
        }
    }

    Ok(messages)
}

/// Splits the thread's events (sorted oldest first) into those sent before the bound's event (see [`ConversationBound`])
/// and the events the conversation ends with (the bound's event, then the explicitly included ones), leaving out the rest.
///
/// Events sent within the bound's millisecond may come in any order, so only those sent strictly before it are kept,
/// and the thread's root (which every other event in the thread follows), which always comes first
/// (when answering the root itself, no other events come before it).
/// The events the conversation ends with come in the bound's order, each with the event if it was among the thread's
/// events (the server may not list them in the thread yet).
/// Edits sent at or after the bound's time are dropped from the kept events (their earlier text is used),
/// except for the bot's own (like the "thinking…" notice of an earlier answer turning into that answer).
#[allow(clippy::type_complexity)]
fn bound_thread_events(
    events: Vec<AnySyncMessageLikeEvent>,
    thread_id: &EventId,
    bound: &ConversationBound,
    bot_user_id: &UserId,
) -> (
    Vec<AnySyncMessageLikeEvent>,
    Vec<(OwnedEventId, Option<AnySyncMessageLikeEvent>)>,
) {
    let mut ending_events: Vec<(OwnedEventId, Option<AnySyncMessageLikeEvent>)> = Vec::new();
    for event_id in std::iter::once(&bound.event_id).chain(bound.also_include.iter()) {
        if !ending_events.iter().any(|(id, _)| id == event_id) {
            ending_events.push((event_id.clone(), None));
        }
    }

    // Every other event in the thread follows its root, whatever their timestamps say.
    let answering_root = bound.event_id == thread_id;

    let mut earlier_events = Vec::new();

    for event in events {
        let event = without_late_edit(event, bound, bot_user_id);

        if let Some((_, slot)) = ending_events
            .iter_mut()
            .find(|(event_id, _)| event_id == event.event_id())
        {
            *slot = Some(event);
        } else if event.event_id() == thread_id {
            earlier_events.insert(0, event);
        } else if !answering_root && event.origin_server_ts() < bound.origin_server_ts {
            earlier_events.push(event);
        }
    }

    (earlier_events, ending_events)
}

/// Drops the latest edit bundled with the event if it was sent at or after the bound's time (see [`bound_thread_events`]).
fn without_late_edit(
    mut event: AnySyncMessageLikeEvent,
    bound: &ConversationBound,
    bot_user_id: &UserId,
) -> AnySyncMessageLikeEvent {
    if let AnySyncMessageLikeEvent::RoomMessage(SyncMessageLikeEvent::Original(message)) =
        &mut event
        && let Some(replacement) = &message.unsigned.relations.replace
        && replacement.origin_server_ts >= bound.origin_server_ts
        && replacement.sender != bot_user_id
    {
        message.unsigned.relations.replace = None;
    }

    event
}

pub async fn get_matrix_messages_in_reply_chain(
    matrix_link: &MatrixLink,
    event_fetcher: &Arc<RoomEventFetcher>,
    room: &Room,
    event_id: OwnedEventId,
) -> Result<Vec<MatrixMessage>, mxlink::matrix_sdk::Error> {
    let messages_native =
        get_matrix_messages_in_reply_chain_native(event_fetcher, room, event_id).await?;

    let mut messages: Vec<MatrixMessage> = Vec::new();

    for matrix_native_message in messages_native {
        let message_result =
            convert_matrix_native_event_to_matrix_message(matrix_link, &matrix_native_message)
                .await?;

        if let Some(message) = message_result {
            messages.push(message);
        }
    }

    Ok(messages)
}

async fn get_matrix_messages_in_reply_chain_native(
    event_fetcher: &Arc<RoomEventFetcher>,
    room: &Room,
    event_id: OwnedEventId,
) -> Result<Vec<AnySyncMessageLikeEvent>, mxlink::matrix_sdk::Error> {
    let mut next_event_id = Some(event_id.clone());

    let mut messages: Vec<AnySyncMessageLikeEvent> = Vec::new();
    let mut handled_event_ids: Vec<OwnedEventId> = Vec::new();

    while let Some(next_event_id_in_loop) = next_event_id {
        let event = match event_fetcher
            .fetch_event_in_room(&next_event_id_in_loop, room)
            .await
        {
            Ok(event) => event,
            // An earlier message in the chain may be unavailable (e.g. sent before the bot could see the room's history),
            // so we use the part of the chain we could fetch.
            Err(err) if !messages.is_empty() => {
                tracing::warn!(
                    ?err,
                    "Not following reply chain past event that could not be fetched: {}",
                    next_event_id_in_loop
                );
                break;
            }
            Err(err) => return Err(err),
        };

        if handled_event_ids.contains(&next_event_id_in_loop) {
            tracing::warn!(
                "Not following loop-causing event: {}",
                next_event_id_in_loop
            );
            break;
        }

        handled_event_ids.push(next_event_id_in_loop.clone());

        let event_deserialized = event.raw().deserialize()?;

        let AnySyncTimelineEvent::MessageLike(message_like_event) = event_deserialized else {
            tracing::warn!(
                "Not proceeding past non-MessageLike event: {:?}",
                event_deserialized
            );
            break;
        };

        next_event_id = match message_like_event.clone() {
            AnySyncMessageLikeEvent::RoomEncrypted(_) => None,
            AnySyncMessageLikeEvent::RoomMessage(room_message) => {
                if let SyncMessageLikeEvent::Original(room_message_original) = room_message {
                    match room_message_original.content.relates_to {
                        Some(Relation::Reply(reply)) => Some(reply.in_reply_to.event_id.clone()),
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
        if !is_message_from_allowed_sender(message, &params.bot_user_id, &params.sender_filter) {
            continue;
        }

        let mut message = message.clone();

        if i == 0
            && !params.first_message_prefixes_to_strip.is_empty()
            && let MatrixMessageContent::Text(message_text) = &message.content
        {
            let mut message_text = message_text.clone();

            for prefix in &params.first_message_prefixes_to_strip {
                if let Some(message_text_stripped) = message_text.strip_prefix(prefix) {
                    message_text = message_text_stripped.to_owned();
                }
            }

            message.content = MatrixMessageContent::Text(message_text.trim().to_owned());
        }

        // We only strip `bot_user_prefixes_to_strip`-defined prefixes from messages that mention the bot user.
        if !params.bot_user_prefixes_to_strip.is_empty()
            && message.mentioned_users.contains(&params.bot_user_id)
            && let MatrixMessageContent::Text(message_text) = &message.content
        {
            let mut message_text = message_text.clone();

            for prefix in &params.bot_user_prefixes_to_strip {
                if let Some(message_text_stripped) = message_text.strip_prefix(prefix) {
                    message_text = message_text_stripped.to_owned();
                }
            }

            message.content = MatrixMessageContent::Text(message_text.trim().to_owned());
        }

        messages_filtered.push(message);
    }

    messages_filtered
}

/// Tells if the given message is from the bot itself or from a sender the filter allows.
fn is_message_from_allowed_sender(
    matrix_message: &MatrixMessage,
    bot_user_id: &OwnedUserId,
    sender_filter: &super::SenderFilter,
) -> bool {
    matrix_message.sender_id == *bot_user_id || sender_filter.allows(&matrix_message.sender_id)
}

/// Returns the message's content with the latest edit (if any) applied, and whether an edit was applied.
///
/// Edits are `m.replace` events, which we don't fetch separately.
/// Instead, we rely on the server bundling the latest one in the original event's `unsigned` data (Matrix v1.7+).
/// Only the new message type and mentions are taken from the edit, so the original's relations (thread, reply) remain.
///
/// This matters for messages edited after being sent, such as the bot's "thinking…" notices,
/// which only become the actual answer via an edit.
pub(crate) fn latest_room_message_content(
    event: &OriginalSyncRoomMessageEvent,
) -> (RoomMessageEventContent, bool) {
    let mut content = event.content.clone();

    if let Some(replacement_event) = &event.unsigned.relations.replace
        // Only the original sender may edit a message.
        && replacement_event.sender == event.sender
        && let Some(Relation::Replacement(replacement)) = &replacement_event.content.relates_to
        && replacement.event_id == event.event_id
    {
        content.apply_replacement(replacement.new_content.clone());
        return (content, true);
    }

    (content, false)
}

/// Returns what the message currently says, and whether an edit was applied:
/// the content it sets for a standalone edit (`m.replace` event), else the content with the latest edit applied
/// (see [`latest_room_message_content`]).
pub(crate) fn effective_room_message_content(
    event: &OriginalSyncRoomMessageEvent,
) -> (RoomMessageEventContent, bool) {
    match &event.content.relates_to {
        Some(Relation::Replacement(replacement)) => {
            let mut content = event.content.clone();
            content.apply_replacement(replacement.new_content.clone());
            (content, true)
        }
        _ => latest_room_message_content(event),
    }
}

/// Tells whether a message's text is an agent's configuration, as shown by the bot (see `agent details`),
/// which may contain secrets (like API keys), so it must never become part of a conversation.
///
/// The [`super::CONTAINS_SECRETS_FIELD`] mark can't be checked for conversations (messages come deserialized,
/// without unknown fields), but the bot only marks such configuration messages.
pub(crate) fn is_agent_configuration(text: &str) -> bool {
    // "Configuration for agent `ID` (powered by the `PROVIDER` provider):\n```yml\n...".
    // The agent's id may span lines, but the provider's name (and what follows it) doesn't.
    let Some(rest) = text.strip_prefix(super::AGENT_CONFIGURATION_MESSAGE_PREFIX) else {
        return false;
    };

    rest.split_once("` (powered by the `")
        .and_then(|(_, rest)| rest.split_once("` provider):\n```yml\n"))
        .is_some_and(|(provider, _)| {
            !provider.is_empty()
                && provider
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        })
}

/// Returns what a conversation gets of a message: its content (with the latest edit applied), text and whether
/// it's a notice, or `None` if it's left out (unsupported message types, and agent configurations).
///
/// A standalone edit (e.g. when someone replies to the edit itself) is taken for the content it sets.
pub(crate) fn conversation_message_content(
    event: &OriginalSyncRoomMessageEvent,
) -> Option<(RoomMessageEventContent, String, bool)> {
    let (content, is_edited) = effective_room_message_content(event);

    let (text, is_notice) = message_text(&content, is_edited)?;

    if is_agent_configuration(&text) {
        return None;
    }

    Some((content, text, is_notice))
}

/// Returns the message's text and whether it's a notice, or `None` for unsupported message types.
///
/// The fallback for rich replies is stripped from the original body of replies (also within threads).
/// Edited content (`m.new_content`) carries no such fallback, so it's left as is.
fn message_text(room_message: &RoomMessageEventContent, is_edited: bool) -> Option<(String, bool)> {
    let (text, is_notice) = match &room_message.msgtype {
        MessageType::Text(text_content) => (text_content.body.clone(), false),
        MessageType::Notice(notice_content) => (notice_content.body.clone(), true),
        MessageType::Image(image_content) => (image_content.body.clone(), false),
        MessageType::File(file_content) => (file_content.body.clone(), false),
        _ => return None,
    };

    // Replies (also within threads) may carry a fallback: a quote of the message replied to, in the form
    // `> <@sender:server> ...` (`> * <@sender:server> ...` for emotes). Clients may also leave it out, and a thread
    // message's `m.in_reply_to` may just be a fallback for clients without threads, so only a quote in that form is
    // taken for one, not text that merely starts like a quote.
    let is_reply = matches!(room_message.relates_to, Some(Relation::Reply { .. }))
        || matches!(&room_message.relates_to, Some(Relation::Thread(thread)) if thread.in_reply_to.is_some());

    let is_reply = is_reply && (text.starts_with("> <@") || text.starts_with("> * <@"));

    let text = if is_reply && !is_edited {
        // For regular replies, we need to strip the fallback-for-rich replies part.
        // See: https://spec.matrix.org/v1.11/client-server-api/#fallbacks-for-rich-replies
        remove_plain_reply_fallback(&text).to_owned()
    } else {
        text
    };

    Some((text, is_notice))
}

pub async fn convert_matrix_native_event_to_matrix_message(
    matrix_link: &MatrixLink,
    matrix_native_event: &AnySyncMessageLikeEvent,
) -> Result<Option<MatrixMessage>, mxlink::matrix_sdk::Error> {
    let AnySyncMessageLikeEvent::RoomMessage(SyncMessageLikeEvent::Original(original)) =
        matrix_native_event
    else {
        // Redacted message, some other event type, etc.
        return Ok(None);
    };

    let Some((room_message, text, is_notice)) = conversation_message_content(original) else {
        return Ok(None);
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

    if let MessageType::Image(image_content) = &room_message.msgtype {
        let media_request = mxlink::matrix_sdk::media::MediaRequestParameters {
            source: image_content.source.to_owned(),
            format: mxlink::matrix_sdk::media::MediaFormat::File,
        };

        let file_name = image_content
            .filename
            .clone()
            // The body may quote the message replied to (stripped from `text`).
            .unwrap_or(text.clone());

        let mime_type = get_mime_type_from_file_name(&file_name);

        tracing::debug!("Determined mime type {} for file {}", mime_type, file_name);

        let span = tracing::debug_span!("get_media_content", file_name = %file_name, mime_type = %mime_type);

        let media_bytes = matrix_link
            .client()
            .media()
            .get_media_content(&media_request, true)
            .instrument(span)
            .await?;

        return Ok(Some(MatrixMessage {
            sender_id: matrix_native_event.sender().to_owned(),
            content: MatrixMessageContent::Image(
                with_file_name(image_content, |content| &mut content.filename, &file_name),
                mime_type,
                media_bytes,
            ),
            mentioned_users,
            timestamp,
        }));
    }

    if let MessageType::File(file_content) = &room_message.msgtype {
        let media_request = mxlink::matrix_sdk::media::MediaRequestParameters {
            source: file_content.source.to_owned(),
            format: mxlink::matrix_sdk::media::MediaFormat::File,
        };

        let file_name = file_content
            .filename
            .clone()
            // The body may quote the message replied to (stripped from `text`).
            .unwrap_or(text.clone());

        let mime_type = file_content
            .info
            .as_ref()
            .and_then(|info| info.mimetype.clone())
            .and_then(|mimetype| mimetype.parse::<mxlink::mime::Mime>().ok())
            .unwrap_or_else(|| get_mime_type_from_file_name(&file_name));

        tracing::debug!("Determined mime type {} for file {}", mime_type, file_name);

        if mime_type == mxlink::mime::APPLICATION_OCTET_STREAM {
            tracing::debug!(
                "Skipping file {} with unsupported MIME type {}. It will be represented as a text message.",
                file_name,
                mime_type,
            );

            return Ok(Some(MatrixMessage {
                sender_id: matrix_native_event.sender().to_owned(),
                content: MatrixMessageContent::Text(format!(
                    "[A file ({}) was attached but skipped because its content type ({}) is not supported. Let the user know.]",
                    file_name, mime_type,
                )),
                mentioned_users,
                timestamp,
            }));
        }

        let span = tracing::debug_span!("get_media_content", file_name = %file_name, mime_type = %mime_type);

        let media_bytes = matrix_link
            .client()
            .media()
            .get_media_content(&media_request, true)
            .instrument(span)
            .await?;

        tracing::debug!(
            "Downloaded {} bytes for file {}",
            media_bytes.len(),
            file_name
        );

        return Ok(Some(MatrixMessage {
            sender_id: matrix_native_event.sender().to_owned(),
            content: MatrixMessageContent::File(
                with_file_name(file_content, |content| &mut content.filename, &file_name),
                mime_type,
                media_bytes,
            ),
            mentioned_users,
            timestamp,
        }));
    }

    Ok(Some(MatrixMessage {
        sender_id: matrix_native_event.sender().to_owned(),
        content: if is_notice {
            MatrixMessageContent::Notice(text)
        } else {
            MatrixMessageContent::Text(text)
        },
        mentioned_users,
        timestamp,
    }))
}

/// Returns a copy of an attachment's content with its file name set, so that providers use it (and not the body,
/// which may quote the message replied to).
fn with_file_name<T: Clone>(
    content: &T,
    file_name_field: impl Fn(&mut T) -> &mut Option<String>,
    file_name: &str,
) -> T {
    let mut content = content.clone();
    *file_name_field(&mut content) = Some(file_name.to_owned());
    content
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
///     - replies that mention the bot create a interaction context which points to a "first message" which is synthetic
///     - when `continue_replies_to_bot` is set (the `reply` reply style), non-audio replies to the bot's own messages are handled the same way, even without a mention
///     - other replies cause this function to return `None`
#[tracing::instrument(name = "determine_interaction_context_for_room_event", skip_all, fields(room_id = room.room_id().as_str(), event_id = current_event.event_id.as_str()))]
pub async fn determine_interaction_context_for_room_event(
    bot_user_id: &OwnedUserId,
    bot_display_name: &Option<String>,
    room: &Room,
    current_event: &OriginalSyncRoomMessageEvent,
    current_event_payload: &MessagePayload,
    event_fetcher: &Arc<RoomEventFetcher>,
    continue_replies_to_bot: bool,
) -> anyhow::Result<Option<InteractionContext>> {
    let current_event_is_mentioning_bot =
        is_event_mentioning_bot(&current_event.content, bot_user_id, bot_display_name);

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
                bot_display_name,
                room,
                current_event,
                event_fetcher,
                current_event_is_mentioning_bot,
                thread,
            )
            .await
        }
        Relation::Reply(reply) => {
            // In the `reply` reply style, replying to one of our own messages continues the conversation,
            // the same way a reply mentioning us does. Audio is excluded, as its speech-to-text flow is thread-based.
            let is_continuing_reply_to_bot = continue_replies_to_bot
                && !current_event_is_mentioning_bot
                && !matches!(current_event_payload, MessagePayload::Audio(_))
                && is_event_sent_by(
                    event_fetcher,
                    room,
                    &reply.in_reply_to.event_id,
                    bot_user_id,
                )
                .await;

            determine_interaction_context_for_room_event_related_to_reply(
                current_event,
                current_event_is_mentioning_bot || is_continuing_reply_to_bot,
                reply.in_reply_to.event_id.clone(),
            )
            .await
        }

        // This is a replacement or something else. It's not something we support.
        _ => return Ok(None),
    }
}

async fn determine_interaction_context_for_room_event_related_to_thread(
    bot_user_id: &OwnedUserId,
    bot_display_name: &Option<String>,
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
        bot_display_name,
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
    current_event_is_addressing_bot: bool,
    reply_to_event_id: OwnedEventId,
) -> anyhow::Result<Option<InteractionContext>> {
    tracing::trace!(?current_event_is_addressing_bot, "Dealing with a reply");

    if !current_event_is_addressing_bot {
        // If the current event is not addressing the bot (via a mention or by replying to it), we don't care about it.
        tracing::trace!("Ignoring reply event which does not address the bot");
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

/// Tells if the given event was sent by the given user.
/// Failures to fetch the event are logged and treated as "no".
async fn is_event_sent_by(
    event_fetcher: &Arc<RoomEventFetcher>,
    room: &Room,
    event_id: &OwnedEventId,
    user_id: &OwnedUserId,
) -> bool {
    let event = match event_fetcher.fetch_event_in_room(event_id, room).await {
        Ok(event) => event,
        Err(err) => {
            tracing::warn!(?err, %event_id, "Failed to fetch replied-to event");
            return false;
        }
    };

    event
        .raw()
        .get_field::<OwnedUserId>("sender")
        .ok()
        .flatten()
        .is_some_and(|sender| sender == *user_id)
}

fn is_event_mentioning_bot(
    event_content: &RoomMessageEventContent,
    bot_user_id: &OwnedUserId,
    bot_display_name: &Option<String>,
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
        // We may consider dropping this string-matching behavior altogether in the future,
        // so improving this compatibility block is not a high priority.
        if event_content.body().contains(bot_user_id.as_str()) {
            return true;
        }

        if let Some(bot_display_name) = bot_display_name {
            return event_content.body().contains(bot_display_name);
        }

        false
    }
}

fn timeline_event_to_detailed_message_payload(
    timeline_event_id: &OwnedEventId,
    timeline_event: TimelineEvent,
    thread_info: ThreadInfo,
    bot_user_id: &OwnedUserId,
    bot_display_name: &Option<String>,
) -> anyhow::Result<Option<DetailedMessagePayload>> {
    let timeline_event_deserialized = match timeline_event.raw().deserialize() {
        Ok(value) => value,
        Err(err) => {
            return Err(anyhow::format_err!(
                "Failed to deserialize timeline event {}: {:?}",
                timeline_event_id,
                err
            ));
        }
    };

    let AnySyncTimelineEvent::MessageLike(thread_start_message_like_event) =
        timeline_event_deserialized
    else {
        tracing::trace!(
            "Ignoring non-MessageLike timeline event: {:?}",
            timeline_event_deserialized
        );
        return Ok(None);
    };

    let (is_mentioning_bot, message_payload) = match thread_start_message_like_event {
        AnySyncMessageLikeEvent::RoomEncrypted(room_message) => {
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
        AnySyncMessageLikeEvent::RoomMessage(room_message) => {
            if let SyncMessageLikeEvent::Original(room_message_original) = room_message {
                let room_message_payload: Result<MessagePayload, String> =
                    room_message_original.content.msgtype.clone().try_into();

                let Ok(room_message_payload) = room_message_payload else {
                    tracing::debug!(
                        msg_type = room_message_original.content.msgtype(),
                        "Ignoring event message of unknown type",
                    );
                    return Ok(None);
                };

                let is_mentioning_bot = is_event_mentioning_bot(
                    &room_message_original.content,
                    bot_user_id,
                    bot_display_name,
                );

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
