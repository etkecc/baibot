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

/// An event (cached under `event_id`) whose latest edit (bundled by the server) is `edit_event_id`.
fn event_with_edit(event_id: &str, edit_event_id: &str) -> TimelineEvent {
    let raw = Raw::from_json(
        serde_json::value::to_raw_value(&serde_json::json!({
            "type": "m.room.message",
            "event_id": event_id,
            "sender": "@user:example.com",
            "origin_server_ts": 1,
            "content": { "msgtype": "m.text", "body": "original" },
            "unsigned": {
                "m.relations": {
                    "m.replace": {
                        "type": "m.room.message",
                        "event_id": edit_event_id,
                        "sender": "@user:example.com",
                        "origin_server_ts": 2,
                        "content": {
                            "msgtype": "m.text",
                            "body": "* edited",
                            "m.new_content": { "msgtype": "m.text", "body": "edited" },
                            "m.relates_to": { "rel_type": "m.replace", "event_id": event_id },
                        },
                    },
                },
            },
        }))
        .unwrap(),
    );

    TimelineEvent::from_plaintext(raw)
}

#[tokio::test]
async fn redactions_forget_the_redacted_event_and_events_it_edited() {
    let fetcher = RoomEventFetcher::new(Some(10));
    let fetch_count = AtomicUsize::new(0);

    let room_id = OwnedRoomId::try_from("!a:example.com").unwrap();
    let other_room_id = OwnedRoomId::try_from("!b:example.com").unwrap();
    let redacted_id = OwnedEventId::try_from("$redacted:example.com").unwrap();
    let edited_id = OwnedEventId::try_from("$edited:example.com").unwrap();
    let edit_id = OwnedEventId::try_from("$edit:example.com").unwrap();
    let unrelated_id = OwnedEventId::try_from("$unrelated:example.com").unwrap();

    let cache = |room_id: &OwnedRoomId, event_id: &OwnedEventId, event: TimelineEvent| {
        let room_id = room_id.clone();
        let event_id = event_id.clone();
        let fetcher = &fetcher;
        async move {
            fetcher
                .get_or_fetch(room_id, event_id, || async { Ok::<_, Infallible>(event) })
                .await
                .unwrap();
        }
    };

    cache(&room_id, &redacted_id, event_with_body("to be redacted")).await;
    cache(
        &room_id,
        &edited_id,
        event_with_edit("$edited:example.com", "$edit:example.com"),
    )
    .await;
    cache(&room_id, &unrelated_id, event_with_body("unrelated")).await;
    // The same IDs in another room are left alone.
    cache(&other_room_id, &redacted_id, event_with_body("other room")).await;
    cache(
        &other_room_id,
        &edited_id,
        event_with_edit("$edited:example.com", "$edit:example.com"),
    )
    .await;

    // Redacting a message forgets it.
    fetcher.forget_redacted(&room_id, &redacted_id);
    let event = fetch(
        &fetcher,
        &room_id,
        &redacted_id,
        "redacted now",
        &fetch_count,
    )
    .await;
    assert_eq!(body_of(&event), "redacted now");

    // Redacting an edit forgets the message it edited (whose cached copy carries the edit).
    fetcher.forget_redacted(&room_id, &edit_id);
    let event = fetch(
        &fetcher,
        &room_id,
        &edited_id,
        "edit redacted now",
        &fetch_count,
    )
    .await;
    assert_eq!(body_of(&event), "edit redacted now");

    let event = fetch(&fetcher, &room_id, &unrelated_id, "refetched", &fetch_count).await;
    assert_eq!(body_of(&event), "unrelated");
    let event = fetch(
        &fetcher,
        &other_room_id,
        &redacted_id,
        "refetched",
        &fetch_count,
    )
    .await;
    assert_eq!(body_of(&event), "other room");
    let event = fetch(
        &fetcher,
        &other_room_id,
        &edited_id,
        "refetched",
        &fetch_count,
    )
    .await;
    assert_eq!(body_of(&event), "original");

    assert_eq!(fetch_count.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn events_fetched_while_the_cache_got_outdated_are_not_cached() {
    let room_id = OwnedRoomId::try_from("!a:example.com").unwrap();
    let event_id = OwnedEventId::try_from("$event:example.com").unwrap();

    // A redaction or an edit may arrive while a fetch is in flight (whose response may predate it).
    let outdate: [fn(&RoomEventFetcher, &OwnedRoomId, &OwnedEventId); 2] = [
        |fetcher, room_id, event_id| fetcher.forget_redacted(room_id, event_id),
        |fetcher, room_id, event_id| fetcher.forget(room_id, event_id),
    ];

    for outdate in outdate {
        let fetcher = RoomEventFetcher::new(Some(10));
        let fetch_count = AtomicUsize::new(0);

        let event = fetcher
            .get_or_fetch(room_id.clone(), event_id.clone(), || async {
                outdate(&fetcher, &room_id, &event_id);
                Ok::<_, Infallible>(event_with_body("stale"))
            })
            .await
            .unwrap();
        assert_eq!(body_of(&event), "stale");

        let event = fetch(&fetcher, &room_id, &event_id, "fresh", &fetch_count).await;
        assert_eq!(body_of(&event), "fresh");
        assert_eq!(fetch_count.load(Ordering::SeqCst), 1);

        // Once nothing changes meanwhile, fetched events are cached again.
        let event = fetch(&fetcher, &room_id, &event_id, "refetched", &fetch_count).await;
        assert_eq!(body_of(&event), "fresh");
        assert_eq!(fetch_count.load(Ordering::SeqCst), 1);
    }
}
