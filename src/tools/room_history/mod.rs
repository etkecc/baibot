//! Reading the room's recent messages (the `builtin.room.history` tool).
//!
//! The bot fetches the messages its Matrix account can see and filters them itself (by count, time and sender),
//! so that only what the model asked for, and no messages from people denied text generation, reach the model.
//! The room's main timeline is read backwards from the message being answered, wherever that is (also from within
//! threads). Thread replies are skipped (thread roots say how many replies they have), as are redacted messages and
//! edits (messages show their latest edit).

mod collect;
mod format;
pub mod query;
mod senders;

#[cfg(test)]
mod tests;

use std::collections::{HashMap, HashSet};

use mxlink::matrix_sdk::Room;
use mxlink::matrix_sdk::RoomMemberships;
use mxlink::matrix_sdk::room::MessagesOptions;
use mxlink::matrix_sdk::ruma::api::client::filter::RoomEventFilter;
use mxlink::matrix_sdk::ruma::api::error::ErrorKind;
use mxlink::matrix_sdk::ruma::events::AnySyncTimelineEvent;
use mxlink::matrix_sdk::ruma::events::TimelineEventType;
use mxlink::matrix_sdk::ruma::serde::Raw;
use mxlink::matrix_sdk::ruma::{EventId, OwnedEventId, OwnedUserId, UInt};

use crate::conversation::matrix::SenderFilter;

pub use query::HistoryQuery;

use collect::{Collector, EndReason};

/// The most events looked at (of any kind) for a single read.
const MAX_SCANNED_EVENTS: usize = 500;

/// Events requested around the message being answered (the first page).
const CONTEXT_SIZE: u32 = 50;

/// Events requested per further page.
const PAGE_SIZE: usize = 100;

/// The most further pages requested for a single read (pages may be empty, e.g. when filtered).
const MAX_PAGES: usize = 20;

const FETCH_FAILED_MESSAGE: &str = "Failed to read the room's history.";

/// What reading the history depends on, besides the query.
pub struct HistoryContext {
    /// Messages from the bot itself are always included (and marked as such).
    pub bot_user_id: OwnedUserId,

    /// Whose messages may be included (others are skipped and counted).
    pub sender_filter: SenderFilter,

    /// Messages to leave out, because the model already has them (e.g. the conversation being answered).
    pub exclude_event_ids: HashSet<OwnedEventId>,
}

/// Reads the room's recent messages, from before `before_event_id` (or the query's own starting point),
/// and formats them for the model.
///
/// Returns an error message for the model (not a failure of the bot) if the request can't be served.
pub async fn read(
    room: &Room,
    before_event_id: &EventId,
    query: &HistoryQuery,
    context: &HistoryContext,
    max_output_bytes: usize,
) -> Result<String, String> {
    let members = room_members(room).await;

    let senders = match &query.senders {
        Some(names) => Some(senders::resolve(names, &members)?),
        None => None,
    };

    let mut collector = Collector::new(query, senders, context, MAX_SCANNED_EVENTS);

    let anchor = query.before.as_deref().unwrap_or(before_event_id);

    let end_reason = fetch(&RoomTimeline(room), anchor, &mut collector).await?;

    Ok(format::format(
        &collector,
        end_reason,
        &members,
        &context.bot_user_id,
        max_output_bytes,
    ))
}

/// Display names of the room's members (current and former), for resolving names and labelling messages.
async fn room_members(room: &Room) -> HashMap<OwnedUserId, Option<String>> {
    let memberships = RoomMemberships::JOIN
        | RoomMemberships::INVITE
        | RoomMemberships::LEAVE
        | RoomMemberships::BAN;

    match room.members(memberships).await {
        Ok(members) => members
            .into_iter()
            .map(|member| {
                (
                    member.user_id().to_owned(),
                    member.display_name().map(str::to_owned),
                )
            })
            .collect(),
        Err(err) => {
            log_fetch_error("members", &err);
            HashMap::new()
        }
    }
}

/// Events in reverse chronological order, and where to continue further back (if anywhere).
struct Page {
    events: Vec<Raw<AnySyncTimelineEvent>>,
    next: Option<String>,
}

/// Where events come from (a room, or a fake one in tests).
trait Timeline {
    /// Events before the given one, and where to continue.
    async fn before(&self, event_id: &EventId) -> Result<Page, ()>;

    /// Up to `limit` events further back from a token, and where to continue.
    async fn page(&self, from: &str, limit: usize) -> Result<Page, ()>;
}

struct RoomTimeline<'a>(&'a Room);

impl Timeline for RoomTimeline<'_> {
    async fn before(&self, event_id: &EventId) -> Result<Page, ()> {
        let context = self
            .0
            .event_with_context(event_id, false, UInt::from(CONTEXT_SIZE), None)
            .await
            .map_err(|err| log_fetch_error("context", &err))?;

        Ok(Page {
            // Already in reverse chronological order.
            events: context
                .events_before
                .iter()
                .map(|event| event.raw().clone())
                .collect(),
            next: context.prev_batch_token,
        })
    }

    async fn page(&self, from: &str, limit: usize) -> Result<Page, ()> {
        let mut options = MessagesOptions::backward().from(from);
        options.limit = UInt::try_from(limit).unwrap_or(UInt::MAX);
        options.filter = message_filter();

        let messages = self
            .0
            .messages(options)
            .await
            .map_err(|err| log_fetch_error("messages", &err))?;

        Ok(Page {
            events: messages
                .chunk
                .iter()
                .map(|event| event.raw().clone())
                .collect(),
            next: messages.end,
        })
    }
}

/// Logs a failed request without the error itself, which may contain response bodies (room content).
fn log_fetch_error(request: &str, err: &mxlink::matrix_sdk::Error) {
    tracing::warn!(
        request,
        status = err
            .as_client_api_error()
            .map(|err| err.status_code.as_u16()),
        // Only standard codes: error kinds may carry (parts of) the response, and so may custom codes.
        errcode = err.client_api_error_kind().map(|kind| match kind {
            ErrorKind::_Custom(_) => "(custom)".to_owned(),
            kind => kind.errcode().to_string(),
        }),
        "Failed to read the room's history"
    );
}

/// Feeds the collector with events before the given one, newest first, until it's done.
async fn fetch(
    timeline: &impl Timeline,
    before_event_id: &EventId,
    collector: &mut Collector<'_>,
) -> Result<EndReason, String> {
    let fetch_failed = |()| FETCH_FAILED_MESSAGE.to_owned();

    let first = timeline
        .before(before_event_id)
        .await
        .map_err(fetch_failed)?;

    for event in &first.events {
        if let Some(reason) = collector.add(event) {
            return Ok(reason);
        }
    }

    let mut from = first.next;
    let mut pages = 0;

    loop {
        let Some(token) = from else {
            return Ok(EndReason::RoomStart);
        };

        let remaining = collector.remaining_scan();
        if remaining == 0 || pages >= MAX_PAGES {
            return Ok(EndReason::ScanLimit);
        }

        pages += 1;

        let page = timeline
            .page(&token, remaining.min(PAGE_SIZE))
            .await
            .map_err(fetch_failed)?;

        for event in &page.events {
            if let Some(reason) = collector.add(event) {
                return Ok(reason);
            }
        }

        // A page may be empty (e.g. when it only had filtered-out events) and still lead further back.
        // A token which doesn't change means there's nothing more.
        from = page.next.filter(|next| *next != token);
    }
}

/// Only messages (encrypted or not), so that other events (reactions, state changes) don't use up the scan limit.
fn message_filter() -> RoomEventFilter {
    let mut filter = RoomEventFilter::default();
    filter.types = Some(vec![
        TimelineEventType::RoomMessage.to_string(),
        TimelineEventType::RoomEncrypted.to_string(),
    ]);
    filter
}
