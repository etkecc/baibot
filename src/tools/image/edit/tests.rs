use mxlink::matrix_sdk::ruma::events::room::message::ImageMessageEventContent;
use mxlink::matrix_sdk::ruma::{OwnedEventId, OwnedMxcUri, OwnedUserId, UserId};

use super::{
    ConversationImage, EditRequest, MAX_IMAGES, Sources, activity, select_conversation_images,
};
use crate::conversation::llm::ImageDetails;

fn image(sender: Option<&str>, name: &str) -> ConversationImage {
    ConversationImage {
        sender_id: sender.map(|sender| OwnedUserId::try_from(sender).unwrap()),
        image: ImageDetails::new(
            ImageMessageEventContent::plain(
                name.to_owned(),
                OwnedMxcUri::from("mxc://example.com/abc"),
            ),
            mxlink::mime::IMAGE_PNG,
            vec![1, 2, 3],
        ),
    }
}

fn names(images: &[ImageDetails]) -> Vec<String> {
    images.iter().map(|image| image.filename()).collect()
}

fn event_id(id: &str) -> OwnedEventId {
    OwnedEventId::try_from(id).unwrap()
}

#[test]
fn requests_use_the_latest_conversation_image_by_default() {
    assert_eq!(
        EditRequest::parse(r#"{"prompt": " make the cat blue "}"#),
        Ok(EditRequest {
            prompt: "make the cat blue".to_owned(),
            sources: Sources::Conversation(1),
        })
    );

    assert_eq!(
        EditRequest::parse(r#"{"prompt": "combine them", "conversation_images": 2}"#)
            .unwrap()
            .sources,
        Sources::Conversation(2)
    );
}

#[test]
fn requests_may_name_images_by_event_id() {
    assert_eq!(
        EditRequest::parse(
            r#"{"prompt": "blue", "image_event_ids": ["$a:example.com", " $b:example.com", "$a:example.com"]}"#
        )
        .unwrap()
        .sources,
        Sources::Events(vec![event_id("$a:example.com"), event_id("$b:example.com")])
    );
}

#[test]
fn invalid_requests_are_explained() {
    let error = |arguments: &str| EditRequest::parse(arguments).unwrap_err();

    assert!(error("").contains("missing field `prompt`"));
    assert_eq!(error(r#"{"prompt": " "}"#), "The prompt is empty.");
    assert!(error(r#"{"prompt": "x", "size": 3}"#).contains("unknown field"));
    assert!(
        error(
            r#"{"prompt": "x", "image_event_ids": ["$a:example.com"], "conversation_images": 1}"#
        )
        .contains("not both")
    );
    assert!(error(r#"{"prompt": "x", "image_event_ids": []}"#).contains("1 to 4"));
    assert!(
        error(r#"{"prompt": "x", "image_event_ids": ["$a:x", "$b:x", "$c:x", "$d:x", "$e:x"]}"#)
            .contains("1 to 4")
    );
    assert_eq!(
        error(r#"{"prompt": "x", "image_event_ids": ["not an id"]}"#),
        "Invalid event ID: not an id"
    );
    assert!(error(r#"{"prompt": "x", "conversation_images": 0}"#).contains("between 1 and"));
    assert!(
        error(&format!(
            r#"{{"prompt": "x", "conversation_images": {}}}"#,
            MAX_IMAGES + 1
        ))
        .contains("between 1 and")
    );
}

#[test]
fn conversation_images_are_the_latest_ones_oldest_first() {
    let images = [
        image(Some("@alice:example.com"), "1.png"),
        image(Some("@mallory:example.com"), "2.png"),
        image(Some("@bot:example.com"), "3.png"),
        image(None, "4.png"),
    ];

    let may_use = |user_id: &UserId| user_id.as_str() != "@mallory:example.com";

    assert_eq!(
        names(&select_conversation_images(&images, 1, may_use).unwrap()),
        ["4.png"]
    );
    assert_eq!(
        names(&select_conversation_images(&images, 2, may_use).unwrap()),
        ["3.png", "4.png"]
    );

    let error = select_conversation_images(&images, 5, may_use).unwrap_err();
    assert_eq!(error, "The conversation has only 4 image(s).");

    let error = select_conversation_images(&[], 1, may_use).unwrap_err();
    assert!(
        error.starts_with("There is no image in this conversation"),
        "{error}"
    );
}

#[test]
fn images_which_may_not_be_used_are_not_replaced_by_older_ones() {
    let images = [
        image(Some("@alice:example.com"), "1.png"),
        image(Some("@mallory:example.com"), "2.png"),
    ];

    let may_use = |user_id: &UserId| user_id.as_str() != "@mallory:example.com";

    for count in [1, 2] {
        let error = select_conversation_images(&images, count, may_use).unwrap_err();
        assert!(error.contains("denied image generation"), "{error}");
    }

    assert_eq!(
        names(&select_conversation_images(&images[..1], 1, may_use).unwrap()),
        ["1.png"]
    );
}

#[test]
fn activity_counts_images_without_model_text() {
    assert_eq!(
        activity(r#"{"prompt": "make it blue"}"#),
        "🎨 Editing an image"
    );
    assert_eq!(
        activity(r#"{"prompt": "combine", "conversation_images": 3}"#),
        "🎨 Combining 3 images"
    );
    assert_eq!(activity("not json"), "🎨 Editing an image");
}
