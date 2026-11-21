#[cfg(test)]
mod tests;

use std::future::Future;

use mxlink::matrix_sdk::Room;
use mxlink::matrix_sdk::deserialized_responses::TimelineEvent;
use mxlink::matrix_sdk::ruma::{OwnedEventId, OwnedRoomId};

use quick_cache::sync::Cache;

/// Event IDs are only looked up within a given room, so the room is part of the cache key.
/// Otherwise, an event cached while handling one room could be returned for another room.
type CacheKey = (OwnedRoomId, OwnedEventId);

pub struct RoomEventFetcher {
    lru_cache: Option<Cache<CacheKey, TimelineEvent>>,
}

impl RoomEventFetcher {
    pub fn new(lru_cache_size: Option<usize>) -> Self {
        let lru_cache = lru_cache_size.map(Cache::new);

        Self { lru_cache }
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
                let event = fetch().await?;

                let _ = guard.insert(event.clone());

                tracing::trace!("Returning now-cached event");

                Ok(event)
            }
        }
    }
}
