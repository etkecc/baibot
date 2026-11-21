use mxlink::matrix_sdk::ruma::events::room::message::OriginalSyncRoomMessageEvent;

use crate::entity::MessagePayload;

fn event(json: serde_json::Value) -> OriginalSyncRoomMessageEvent {
    serde_json::from_value(json).unwrap()
}

fn text(payload: &MessagePayload) -> &str {
    match payload {
        MessagePayload::Text(text_content) => &text_content.body,
        _ => panic!("Expected a text payload"),
    }
}

#[test]
fn reacted_to_message_uses_the_latest_edit_and_keeps_the_thread() {
    // A "thinking…" placeholder in a thread, which became the answer via an edit.
    let event = event(serde_json::json!({
        "type": "m.room.message",
        "event_id": "$answer:example.com",
        "sender": "@bot:example.com",
        "origin_server_ts": 1,
        "content": {
            "msgtype": "m.text",
            "body": "thinking…",
            "m.relates_to": {
                "rel_type": "m.thread",
                "event_id": "$root:example.com",
                "is_falling_back": true,
                "m.in_reply_to": { "event_id": "$root:example.com" },
            },
        },
        "unsigned": {
            "m.relations": {
                "m.replace": {
                    "type": "m.room.message",
                    "event_id": "$edit:example.com",
                    "sender": "@bot:example.com",
                    "origin_server_ts": 2,
                    "content": {
                        "msgtype": "m.text",
                        "body": "* The answer",
                        "m.new_content": { "msgtype": "m.text", "body": "The answer" },
                        "m.relates_to": { "rel_type": "m.replace", "event_id": "$answer:example.com" },
                    },
                },
            },
        },
    }));

    let (payload, thread_root_event_id) = super::reacted_to_message(&event).unwrap();

    assert_eq!(text(&payload), "The answer");
    assert_eq!(thread_root_event_id.as_str(), "$root:example.com");
}

#[test]
fn reacted_to_message_outside_a_thread_is_its_own_root() {
    let event = event(serde_json::json!({
        "type": "m.room.message",
        "event_id": "$message:example.com",
        "sender": "@user:example.com",
        "origin_server_ts": 1,
        "content": { "msgtype": "m.text", "body": "Hello" },
    }));

    let (payload, thread_root_event_id) = super::reacted_to_message(&event).unwrap();

    assert_eq!(text(&payload), "Hello");
    assert_eq!(thread_root_event_id.as_str(), "$message:example.com");
}

#[test]
fn reacted_to_message_of_an_unsupported_type_is_ignored() {
    let event = event(serde_json::json!({
        "type": "m.room.message",
        "event_id": "$message:example.com",
        "sender": "@user:example.com",
        "origin_server_ts": 1,
        "content": { "msgtype": "m.location", "body": "Here", "geo_uri": "geo:0,0" },
    }));

    assert!(super::reacted_to_message(&event).is_none());
}
