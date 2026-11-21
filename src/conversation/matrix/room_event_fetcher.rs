#[cfg(test)]
mod tests;

use std::future::Future;
use std::sync::Mutex;

use mxlink::matrix_sdk::Room;
use mxlink::matrix_sdk::deserialized_responses::TimelineEvent;
use mxlink::matrix_sdk::ruma::{EventId, OwnedEventId, OwnedRoomId, RoomId};

use quick_cache::sync::Cache;

/// Event IDs are only looked up within a given room, so the room is part of the cache key.
/// Otherwise, an event cached while handling one room could be returned for another room.
type CacheKey = (OwnedRoomId, OwnedEventId);

pub struct RoomEventFetcher {
    lru_cache: Option<Cache<CacheKey, TimelineEvent>>,
    /// Bumped whenever cached events get outdated (see [`Self::forget`]), so that fetches which were in flight
    /// meanwhile (and may have gotten the outdated event) don't cache what they got.
    ///
    /// Held while outdating cached events and while deciding to cache a fetched event (and caching it),
    /// so the two never interleave (outdating skips entries which are still being fetched).
    generation: Mutex<u64>,
}

impl RoomEventFetcher {
    pub fn new(lru_cache_size: Option<usize>) -> Self {
        let lru_cache = lru_cache_size.map(Cache::new);

        Self {
            lru_cache,
            generation: Mutex::new(0),
        }
    }

    #[tracing::instrument(skip(self), fields(room_id = room.room_id().as_str(), event_id = event_id.as_str()))]
    pub async fn fetch_event_in_room(
        &self,
        event_id: &OwnedEventId,
        room: &Room,
    ) -> mxlink::matrix_sdk::Result<TimelineEvent> {
        self.get_or_fetch(room.room_id().to_owned(), event_id.clone(), || {
            room.event(event_id, None)
        })
        .await
    }

    /// Removes the given event from the cache, so that it's fetched anew next time.
    ///
    /// Call this when an event changes (e.g. when it gets edited), because the cached copy
    /// (including the latest edit bundled with it) would be outdated.
    pub fn forget(&self, room_id: &RoomId, event_id: &EventId) {
        let mut generation = self.lock_generation();
        *generation += 1;

        if let Some(lru_cache) = &self.lru_cache {
            lru_cache.remove(&(room_id.to_owned(), event_id.to_owned()));
        }
    }

    /// Removes what a redaction of the given event outdates from the cache: the event itself,
    /// and events whose cached copy carries it as their latest edit (the server bundles the previous edit, if any, now).
    pub fn forget_redacted(&self, room_id: &RoomId, redacted_event_id: &EventId) {
        let mut generation = self.lock_generation();
        *generation += 1;

        let Some(lru_cache) = &self.lru_cache else {
            return;
        };

        lru_cache.retain(|(cached_room_id, cached_event_id), event| {
            if cached_room_id != room_id {
                return true;
            }

            cached_event_id != redacted_event_id
                && latest_edit_event_id(event).as_deref() != Some(redacted_event_id.as_str())
        });
    }

    fn lock_generation(&self) -> std::sync::MutexGuard<'_, u64> {
        // The counter stays valid even if a holder panicked.
        self.generation
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    async fn get_or_fetch<F, Fut, E>(
        &self,
        room_id: OwnedRoomId,
        event_id: OwnedEventId,
        fetch: F,
    ) -> Result<TimelineEvent, E>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<TimelineEvent, E>>,
    {
        let Some(lru_cache) = &self.lru_cache else {
            return fetch().await;
        };

        let key = (room_id, event_id);

        match lru_cache.get_value_or_guard_async(&key).await {
            Ok(event) => {
                tracing::trace!("Returning existing cached event..");
                Ok(event)
            }
            Err(guard) => {
                let generation_before = *self.lock_generation();

                let event = fetch().await?;

                let generation = self.lock_generation();

                // Something got outdated while we were fetching, possibly this very event, so we don't cache it.
                // Dropping the guard lets the next lookup fetch anew.
                if *generation != generation_before {
                    tracing::trace!(
                        "Returning fetched event without caching it (cache outdated meanwhile)"
                    );
                    return Ok(event);
                }

                let _ = guard.insert(event.clone());

                drop(generation);

                tracing::trace!("Returning now-cached event");

                Ok(event)
            }
        }
    }
}

/// Returns the ID of the latest edit bundled with the event (if any).
fn latest_edit_event_id(event: &TimelineEvent) -> Option<String> {
    let unsigned = event
        .raw()
        .get_field::<serde_json::Value>("unsigned")
        .ok()
        .flatten()?;

    unsigned
        .get("m.relations")?
        .get("m.replace")?
        .get("event_id")?
        .as_str()
        .map(ToOwned::to_owned)
}
