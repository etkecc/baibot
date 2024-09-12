use mxlink::matrix_sdk::ruma::OwnedRoomId;
use mxlink::matrix_sdk::Room;

use mxlink::MatrixLink;
use quick_cache::sync::Cache;

pub struct RoomDisplayNameFetcher {
    matrix_link: MatrixLink,
    lru_cache: Option<Cache<OwnedRoomId, Option<String>>>,
}

impl RoomDisplayNameFetcher {
    pub fn new(matrix_link: MatrixLink, lru_cache_size: Option<usize>) -> Self {
        let lru_cache = lru_cache_size.map(Cache::new);

        Self {
            matrix_link,
            lru_cache,
        }
    }

    #[tracing::instrument(skip_all, fields(room_id = room.room_id().as_str()))]
    pub async fn own_display_name_in_room(
        &self,
        room: &Room,
    ) -> mxlink::matrix_sdk::Result<Option<String>> {
        let Some(lru_cache) = &self.lru_cache else {
            return self.get_uncached_value(room).await;
        };

        let guard = lru_cache.get_value_or_guard_async(room.room_id()).await;

        match guard {
            Ok(value) => {
                tracing::debug!("Returning existing cached display name..");
                return Ok(value);
            }
            Err(guard) => {
                let value = self.get_uncached_value(room).await?;

                let _ = guard.insert(value.clone());

                tracing::debug!("Returning now-cached display name");

                return Ok(value);
            }
        }
    }

    async fn get_uncached_value(&self, room: &Room) -> mxlink::matrix_sdk::Result<Option<String>> {
        self.matrix_link
            .rooms()
            .own_display_name_in_room(room)
            .await
    }
}
