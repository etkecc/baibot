use std::collections::HashSet;

use chrono::{DateTime, Utc};
use mxlink::matrix_sdk::ruma::events::room::message::{
    MessageType, Relation, RoomMessageEventContent, sanitize::remove_plain_reply_fallback,
};
use mxlink::matrix_sdk::ruma::events::{
    AnySyncMessageLikeEvent, AnySyncTimelineEvent, SyncMessageLikeEvent,
};
use mxlink::matrix_sdk::ruma::serde::Raw;
use mxlink::matrix_sdk::ruma::{OwnedEventId, OwnedUserId};

use super::{HistoryContext, HistoryQuery};

/// Consecutive events older than `since` after which reading stops.
///
/// Servers may return events in an order that doesn't quite match their timestamps (e.g. after clock skew),
/// so a single older event doesn't end the read.
const MAX_CONSECUTIVE_EVENTS_BEFORE_SINCE: usize = 20;
use crate::conversation::matrix::latest_room_message_content;

/// Why reading stopped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EndReason {
    /// As many messages as asked for were found (older ones may exist).
    Limit,

    /// Messages older than `since` were reached.
    Since,

    /// The scan limit was reached (older messages may exist).
    ScanLimit,

    /// There are no older events (or the bot can't see further back).
    RoomStart,
}

/// A message, ready for formatting.
#[derive(Clone, Debug, PartialEq)]
pub struct HistoryMessage {
    pub event_id: OwnedEventId,
    pub sender: OwnedUserId,
    pub timestamp: DateTime<Utc>,
    pub text: String,

    /// The message it replies to (in the main timeline), if any.
    pub in_reply_to: Option<OwnedEventId>,

    /// The number of replies in the thread it starts, if any.
    pub thread_replies: Option<u64>,
}

/// Counts of events left out, reported to the model.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SkipCounts {
    pub thread_replies: usize,
    pub undecryptable: usize,
    pub deleted: usize,
    pub excluded_senders: usize,
}

/// Collects messages from events fed newest first, applying the query and the context's filters.
pub struct Collector<'a> {
    query: &'a HistoryQuery,
    senders: Option<HashSet<OwnedUserId>>,
    context: &'a HistoryContext,
    max_scanned: usize,

    /// Consecutive events seen so far which are older than `since`.
    before_since: usize,

    pub scanned: usize,

    /// Newest first.
    pub messages: Vec<HistoryMessage>,

    pub skipped: SkipCounts,
}

impl<'a> Collector<'a> {
    pub fn new(
        query: &'a HistoryQuery,
        senders: Option<HashSet<OwnedUserId>>,
        context: &'a HistoryContext,
        max_scanned: usize,
    ) -> Self {
        Self {
            query,
            senders,
            context,
            max_scanned,
            before_since: 0,
            scanned: 0,
            messages: vec![],
            skipped: SkipCounts::default(),
        }
    }

    /// How many more events may be looked at.
    pub fn remaining_scan(&self) -> usize {
        self.max_scanned.saturating_sub(self.scanned)
    }

    /// Looks at the next (older) event. Returns why to stop, if done.
    pub fn add(&mut self, raw: &Raw<AnySyncTimelineEvent>) -> Option<EndReason> {
        if self.messages.len() >= self.query.limit {
            return Some(EndReason::Limit);
        }

        if self.scanned >= self.max_scanned {
            return Some(EndReason::ScanLimit);
        }

        self.scanned += 1;

        // Events which can't be parsed are left out, like other events which aren't messages.
        let Ok(AnySyncTimelineEvent::MessageLike(event)) = raw.deserialize() else {
            return None;
        };

        let timestamp = timestamp(&event);

        if let Some(since) = self.query.since {
            if timestamp < since {
                self.before_since += 1;
                if self.before_since >= MAX_CONSECUTIVE_EVENTS_BEFORE_SINCE {
                    return Some(EndReason::Since);
                }
                return None;
            }

            self.before_since = 0;
        }

        if let Some(until) = self.query.until
            && timestamp > until
        {
            return None;
        }

        if self.context.exclude_event_ids.contains(event.event_id()) {
            return None;
        }

        let sender = event.sender().to_owned();

        if let Some(senders) = &self.senders
            && !senders.contains(&sender)
        {
            return None;
        }

        match &event {
            AnySyncMessageLikeEvent::RoomEncrypted(SyncMessageLikeEvent::Redacted(_)) => {
                self.skipped.deleted += 1;
            }
            AnySyncMessageLikeEvent::RoomEncrypted(_) => {
                if self.may_include(&sender) {
                    self.skipped.undecryptable += 1;
                } else {
                    self.skipped.excluded_senders += 1;
                }
            }
            AnySyncMessageLikeEvent::RoomMessage(SyncMessageLikeEvent::Redacted(_)) => {
                self.skipped.deleted += 1;
            }
            AnySyncMessageLikeEvent::RoomMessage(SyncMessageLikeEvent::Original(original)) => {
                match &original.content.relates_to {
                    // Edits are applied to the messages they edit (bundled by the server), not shown separately.
                    Some(Relation::Replacement(_)) => return self.done(),
                    Some(Relation::Thread(_)) => {
                        self.skipped.thread_replies += 1;
                        return self.done();
                    }
                    _ => {}
                }

                if !self.may_include(&sender) {
                    self.skipped.excluded_senders += 1;
                    return self.done();
                }

                let (content, is_edited) = latest_room_message_content(original);

                let Some(text) = message_text(&content, is_edited) else {
                    return self.done();
                };

                let in_reply_to = match &original.content.relates_to {
                    Some(Relation::Reply(reply)) => Some(reply.in_reply_to.event_id.clone()),
                    _ => None,
                };

                let thread_replies = original
                    .unsigned
                    .relations
                    .thread
                    .as_ref()
                    .map(|thread| u64::from(thread.count));

                self.messages.push(HistoryMessage {
                    event_id: original.event_id.clone(),
                    sender,
                    timestamp,
                    text,
                    in_reply_to,
                    thread_replies,
                });
            }
            // Reactions, polls, calls, stickers, etc.
            _ => {}
        }

        self.done()
    }

    fn done(&self) -> Option<EndReason> {
        (self.messages.len() >= self.query.limit).then_some(EndReason::Limit)
    }

    fn may_include(&self, sender: &OwnedUserId) -> bool {
        *sender == self.context.bot_user_id || self.context.sender_filter.allows(sender)
    }
}

fn timestamp(event: &AnySyncMessageLikeEvent) -> DateTime<Utc> {
    DateTime::from_timestamp_millis(event.origin_server_ts().get().into()).unwrap_or_default()
}

/// Returns a message's text for the model, or `None` for message types which aren't shown.
///
/// Attachments are described (not downloaded). The fallback of replies (a quote of the message replied to)
/// is stripped from original (unedited) content, like for conversations.
fn message_text(content: &RoomMessageEventContent, is_edited: bool) -> Option<String> {
    let strip_fallback = |body: &str| {
        if !is_edited && matches!(content.relates_to, Some(Relation::Reply { .. })) {
            remove_plain_reply_fallback(body).to_owned()
        } else {
            body.to_owned()
        }
    };

    let attachment = |kind: &str, file_name: &Option<String>, body: &str| match file_name {
        // With a file name, the body is a caption (which, in replies, may quote the message replied to).
        Some(file_name) if file_name != body => {
            format!("[{kind}: {file_name}] {}", strip_fallback(body))
        }
        _ => format!("[{kind}: {}]", strip_fallback(body)),
    };

    let text = match &content.msgtype {
        MessageType::Text(text) => strip_fallback(&text.body),
        MessageType::Notice(notice) => strip_fallback(&notice.body),
        MessageType::Emote(emote) => format!("[emote] {}", strip_fallback(&emote.body)),
        MessageType::Image(image) => attachment("image", &image.filename, &image.body),
        MessageType::File(file) => attachment("file", &file.filename, &file.body),
        MessageType::Video(video) => attachment("video", &video.filename, &video.body),
        MessageType::Audio(audio) if audio.voice.is_some() => "[voice message]".to_owned(),
        MessageType::Audio(audio) => attachment("audio", &audio.filename, &audio.body),
        MessageType::Location(_) => "[location]".to_owned(),
        _ => return None,
    };

    Some(text)
}
