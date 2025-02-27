use mxlink::matrix_sdk::Room;
use mxlink::matrix_sdk::deserialized_responses::TimelineEvent;
use mxlink::matrix_sdk::ruma::OwnedEventId;

use quick_cache::sync::Cache;

pub struct RoomEventFetcher {
    lru_cache: Option<Cache<OwnedEventId, TimelineEvent>>,
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
        let Some(lru_cache) = &self.lru_cache else {
            return room.event(event_id, None).await;
        };

        let guard = lru_cache.get_value_or_guard_async(event_id).await;

        match guard {
            Ok(config) => {
                tracing::trace!("Returning existing cached event..");
                return Ok(config);
            }
            Err(guard) => {
                let event = room.event(event_id, None).await?;

                let _ = guard.insert(event.clone());

                tracing::trace!("Returning now-cached event");

                return Ok(event);
            }
        }
    }
}
