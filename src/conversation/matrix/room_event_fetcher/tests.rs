use std::convert::Infallible;
use std::sync::atomic::{AtomicUsize, Ordering};

use mxlink::matrix_sdk::deserialized_responses::TimelineEvent;
use mxlink::matrix_sdk::ruma::serde::Raw;
use mxlink::matrix_sdk::ruma::{OwnedEventId, OwnedRoomId};

use super::RoomEventFetcher;

fn event_with_body(body: &str) -> TimelineEvent {
    let raw = Raw::from_json(
        serde_json::value::to_raw_value(&serde_json::json!({
            "type": "m.room.message",
            "event_id": "$event:example.com",
            "sender": "@user:example.com",
            "origin_server_ts": 1,
            "content": { "msgtype": "m.text", "body": body },
        }))
        .unwrap(),
    );

    TimelineEvent::from_plaintext(raw)
}

fn body_of(event: &TimelineEvent) -> String {
    event
        .raw()
        .get_field::<serde_json::Value>("content")
        .unwrap()
        .unwrap()["body"]
        .as_str()
        .unwrap()
        .to_owned()
}

async fn fetch(
    fetcher: &RoomEventFetcher,
    room_id: &OwnedRoomId,
    event_id: &OwnedEventId,
    body: &str,
    fetch_count: &AtomicUsize,
) -> TimelineEvent {
    fetcher
        .get_or_fetch(room_id.clone(), event_id.clone(), || async {
            fetch_count.fetch_add(1, Ordering::SeqCst);
            Ok::<_, Infallible>(event_with_body(body))
        })
        .await
        .unwrap()
}

#[tokio::test]
async fn cache_is_scoped_to_room() {
    let fetcher = RoomEventFetcher::new(Some(10));
    let fetch_count = AtomicUsize::new(0);

    let room_a = OwnedRoomId::try_from("!a:example.com").unwrap();
    let room_b = OwnedRoomId::try_from("!b:example.com").unwrap();
    let event_id = OwnedEventId::try_from("$event:example.com").unwrap();

    let event = fetch(&fetcher, &room_a, &event_id, "from room A", &fetch_count).await;
    assert_eq!(body_of(&event), "from room A");

    // The same event ID in another room must not be served from room A's cache entry.
    let event = fetch(&fetcher, &room_b, &event_id, "from room B", &fetch_count).await;
    assert_eq!(body_of(&event), "from room B");
    assert_eq!(fetch_count.load(Ordering::SeqCst), 2);

    // Repeated lookups in the same room are cached.
    let event = fetch(&fetcher, &room_a, &event_id, "refetched", &fetch_count).await;
    assert_eq!(body_of(&event), "from room A");
    assert_eq!(fetch_count.load(Ordering::SeqCst), 2);

    // Another event in the same room is cached separately.
    let other_event_id = OwnedEventId::try_from("$other:example.com").unwrap();
    let event = fetch(&fetcher, &room_a, &other_event_id, "other", &fetch_count).await;
    assert_eq!(body_of(&event), "other");
    assert_eq!(fetch_count.load(Ordering::SeqCst), 3);

    let event = fetch(&fetcher, &room_a, &event_id, "refetched", &fetch_count).await;
    assert_eq!(body_of(&event), "from room A");
    assert_eq!(fetch_count.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn failed_fetches_are_not_cached() {
    let fetcher = RoomEventFetcher::new(Some(10));

    let room_id = OwnedRoomId::try_from("!a:example.com").unwrap();
    let event_id = OwnedEventId::try_from("$event:example.com").unwrap();

    let result = fetcher
        .get_or_fetch(room_id.clone(), event_id.clone(), || async {
            Err::<TimelineEvent, _>("not found")
        })
        .await;
    assert!(result.is_err());

    let fetch_count = AtomicUsize::new(0);
    let event = fetch(&fetcher, &room_id, &event_id, "found", &fetch_count).await;
    assert_eq!(body_of(&event), "found");
    assert_eq!(fetch_count.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn forgotten_events_are_fetched_again() {
    let fetcher = RoomEventFetcher::new(Some(10));
    let fetch_count = AtomicUsize::new(0);

    let room_id = OwnedRoomId::try_from("!a:example.com").unwrap();
    let event_id = OwnedEventId::try_from("$event:example.com").unwrap();

    let event = fetch(&fetcher, &room_id, &event_id, "original", &fetch_count).await;
    assert_eq!(body_of(&event), "original");

    fetcher.forget(&room_id, &event_id);

    let event = fetch(&fetcher, &room_id, &event_id, "edited", &fetch_count).await;
    assert_eq!(body_of(&event), "edited");
    assert_eq!(fetch_count.load(Ordering::SeqCst), 2);
}
