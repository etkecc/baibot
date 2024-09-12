use std::sync::Arc;

use tokio::sync::Mutex;
use tokio::time::Duration;

use mxlink::helpers::account_data_config::ConfigError;

use super::CatchUpMarkerManager;

/// A service that records roughly until when we're caught up on processing events.
/// Roughly, because we account for potential federation delay and we don't persist the marker too often.
///
/// If the matrix-sdk's state-store is kept intact, we (usually) won't be given the same event twice.
/// In such a happy path, we don't need to keep track of anything and there's no problem.
///
/// If the state-store is lost (a very rare, but possible event), we can recover our encryption keys, etc.,
/// but the Matrix SDK would try to feed us the same events again.
/// Responding to many old events again is annoying to users and can be a huge waste of resources.
///
/// In order to handle state-store-loss better, we need to record until when we're caught up in storage that won't get lost (such as account data for the user).
/// Because state-store-loss is a very rare event, we don't need to be very exact about the specific timestamp we're caught up to.
/// In fact, being behind is necessary, to allow for federation delay (see `federation_delay_tolerance_duration`).
pub struct DelayedCatchUpMarkerManager {
    catch_up_marker_manager: Arc<Mutex<CatchUpMarkerManager>>,

    /// `persist_interval_duration` affects how often we persist the catch-up marker to Account Data
    /// A too small value means there's needless overhead.
    /// The downside to a larger interval value (and a larger federation delay tolerance value) is that that a state-store loss will mean that
    /// we will reprocess some of the same events.
    /// Since this is a very rare event and the downside is not so bad, a large value is recommended.
    persist_interval_duration: Duration,

    /// `federation_delay_tolerance_duration` affects what federation delay we will tolerate.
    /// A larger delay than this may mean we ignore events that are actually new to us.
    /// This is necessary because the timestamp given to us (see `catch_up()`) is based on the "origin server" timestamp.
    /// If federation is slow, we may actually receive old events later on - they'd still be new to us,
    /// but we may ignore them if we've marked this "origin server timestamp" value as "caught up".
    federation_delay_tolerance_duration: Duration,

    /// Holds the timestamp to use for updating the catch-up marker's `caught_up_until_event_origin_server_ts_millis`.
    /// A value of `0` is used to indicate that no update is scheduled and the next iteration should skip updating the marker.
    next_catch_up_marker_event_origin_server_ts_millis: Arc<tokio::sync::Mutex<i64>>,
}

impl DelayedCatchUpMarkerManager {
    pub fn new(
        catch_up_marker_manager: CatchUpMarkerManager,
        persist_interval_duration: Duration,
        federation_delay_tolerance_duration: Duration,
    ) -> Self {
        let next_catch_up_marker_event_origin_server_ts_millis =
            Arc::new(tokio::sync::Mutex::new(0));

        let catch_up_marker_manager = Arc::new(Mutex::new(catch_up_marker_manager));

        Self {
            catch_up_marker_manager,
            persist_interval_duration,
            federation_delay_tolerance_duration,

            next_catch_up_marker_event_origin_server_ts_millis,
        }
    }

    #[tracing::instrument(name = "catch_up", skip(self))]
    pub async fn catch_up(&self, event_origin_server_ts_millis: i64) {
        tracing::trace!("Locking to catch-up..");

        let mut next_catch_up_marker_event_origin_server_ts_millis_guard = self
            .next_catch_up_marker_event_origin_server_ts_millis
            .lock()
            .await;

        if *next_catch_up_marker_event_origin_server_ts_millis_guard > event_origin_server_ts_millis
        {
            tracing::trace!(
                ?next_catch_up_marker_event_origin_server_ts_millis_guard,
                "Already have a more recent timestamp scheduled",
            );
            return;
        }

        *next_catch_up_marker_event_origin_server_ts_millis_guard = event_origin_server_ts_millis;

        tracing::info!("Configured catch-up timestamp for the next update");
    }

    /// Tells if we're caught up until the given timestamp.
    ///
    /// This intentionally uses the latest (cached) data stored in catch_up_marker_manager (Account Data), not the `next_catch_up_marker_event_origin_server_ts_millis` value.
    /// `next_catch_up_marker_event_origin_server_ts_millis` is used for scheduling the next update only.
    /// The actual timestamp that will get persisted durign the update will actually be adjusted by `federation_delay_tolerance_duration`,
    /// so comparing against `next_catch_up_marker_event_origin_server_ts_millis` in its raw form would be incorrect.
    #[tracing::instrument(name = "is_caught_up", skip(self))]
    pub(crate) async fn is_caught_up(
        &self,
        event_origin_ts_millis: i64,
    ) -> Result<bool, ConfigError> {
        tracing::trace!("Locking to check if caught up..");

        let mut manager = self.catch_up_marker_manager.lock().await;

        let marker = manager.get_or_create().await?;

        let is_caught_up =
            marker.caught_up_until_event_origin_server_ts_millis >= event_origin_ts_millis;

        tracing::debug!(
            ?is_caught_up,
            ?marker.caught_up_until_event_origin_server_ts_millis,
            "Determined caught-up status"
        );

        Ok(is_caught_up)
    }

    pub async fn start(&self) {
        let inner = Arc::clone(&self.catch_up_marker_manager);
        let persist_interval_duration = self.persist_interval_duration;
        let federation_delay_tolerance = self.federation_delay_tolerance_duration;
        let next_catch_up_marker_event_origin_server_ts_millis =
            Arc::clone(&self.next_catch_up_marker_event_origin_server_ts_millis);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(persist_interval_duration);

            loop {
                interval.tick().await;

                tracing::trace!("Catch-up manager doing work..");

                let mut next_catch_up_marker_event_origin_server_ts_millis_guard =
                    next_catch_up_marker_event_origin_server_ts_millis
                        .lock()
                        .await;

                if *next_catch_up_marker_event_origin_server_ts_millis_guard == 0 {
                    tracing::trace!("No scheduled updates to the catch-up marker");
                    continue;
                }

                let mut manager = inner.lock().await;

                let marker = manager.get_or_create().await;
                let mut marker = match marker {
                    Ok(marker) => marker,
                    Err(err) => {
                        tracing::error!(?err, "Failed to get or create catch-up marker");
                        continue;
                    }
                };

                // To allow for some federation delay (specified in federation_delay_tolerance),
                // we adjust the value we'll actually persist with that delay duration.
                // For more information, see the documentation for `Self`.
                let caught_up_until_event_origin_server_ts_millis =
                    *next_catch_up_marker_event_origin_server_ts_millis_guard
                        - (federation_delay_tolerance.as_millis() as i64);

                marker.caught_up_until_event_origin_server_ts_millis =
                    caught_up_until_event_origin_server_ts_millis;

                tracing::debug!(
                    ?caught_up_until_event_origin_server_ts_millis,
                    next_catch_up_marker_event_origin_server_ts_millis = format!(
                        "{:?}",
                        next_catch_up_marker_event_origin_server_ts_millis_guard
                    ),
                    "Updating catch-up marker..",
                );

                let result = manager.persist(&marker).await;
                if let Err(err) = result {
                    tracing::error!(?err, "Failed to persist catch-up marker");
                }

                *next_catch_up_marker_event_origin_server_ts_millis_guard = 0;
            }
        });
    }
}
