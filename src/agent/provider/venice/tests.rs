use mxlink::matrix_sdk::ruma::OwnedMxcUri;
use mxlink::matrix_sdk::ruma::events::room::message::{
    FileMessageEventContent, ImageMessageEventContent,
};
use mxlink::mime;

use super::super::ControllerTrait;
use crate::agent::AgentPurpose;
use crate::conversation::llm::{
    Author as LLMAuthor, FileDetails, ImageDetails, Message as LLMMessage,
    MessageContent as LLMMessageContent,
};

use super::config::{Config, VeniceParameters, WebSearchMode};
use super::controller::Controller;
use super::utils::convert_llm_messages_to_venice;
use super::wire::{
    ContentPart, EditImageRequest, GenerateImageRequest, MessageContent, SpeechRequest,
};

#[test]
fn config_round_trips_with_venice_parameters() {
    let yaml = r#"
base_url: https://api.venice.ai/api/v1
api_key: test-key
text_generation:
  model_id: kimi-k2-5
  temperature: 0.7
  max_response_tokens: 1024
  max_context_tokens: 65536
  venice_parameters:
    enable_web_search: "auto"
    enable_web_citations: true
speech_to_text:
  model_id: nvidia/parakeet-tdt-0.6b-v3
"#;

    let config: Config = serde_yaml_ng::from_str(yaml).expect("config should deserialize");

    let tg = config.text_generation.expect("text_generation present");
    let vp = tg.venice_parameters.expect("venice_parameters present");

    assert!(matches!(vp.enable_web_search, Some(WebSearchMode::Auto)));
    assert_eq!(vp.enable_web_citations, Some(true));
    assert_eq!(vp.character_slug, None);

    // The bag must serialize the enum to the exact wire string, and an unset knob must be ABSENT
    // (not `null`) so the strict `additionalProperties: false` body is honored.
    let json = serde_json::to_string(&vp).expect("serialize venice_parameters");
    assert!(
        json.contains("\"enable_web_search\":\"auto\""),
        "web search should be the literal \"auto\": {json}"
    );
    assert!(
        !json.contains("character_slug"),
        "an unset knob must be omitted entirely: {json}"
    );
    assert!(!json.contains("null"), "no nulls belong in the body: {json}");
}

#[test]
fn converts_image_to_data_uri_and_skips_files() {
    let messages = vec![
        LLMMessage {
            author: LLMAuthor::User,
            sender_id: None,
            timestamp: chrono::Utc::now(),
            content: LLMMessageContent::Text("describe this".to_owned()),
        },
        LLMMessage {
            author: LLMAuthor::User,
            sender_id: None,
            timestamp: chrono::Utc::now(),
            content: LLMMessageContent::Image(ImageDetails::new(
                ImageMessageEventContent::plain(
                    "pic.png".to_owned(),
                    OwnedMxcUri::from("mxc://example.com/abc"),
                ),
                mime::IMAGE_PNG,
                vec![1, 2, 3],
            )),
        },
        LLMMessage {
            author: LLMAuthor::User,
            sender_id: None,
            timestamp: chrono::Utc::now(),
            content: LLMMessageContent::File(FileDetails::new(
                FileMessageEventContent::plain(
                    "doc.pdf".to_owned(),
                    OwnedMxcUri::from("mxc://example.com/def"),
                ),
                mime::APPLICATION_PDF,
                vec![4, 5, 6],
            )),
        },
    ];

    let converted = convert_llm_messages_to_venice(messages);

    // Text and image survive; the file is warn-skipped.
    assert_eq!(converted.len(), 2);

    match &converted[0].content {
        MessageContent::Text(text) => assert_eq!(text, "describe this"),
        other => panic!("expected bare text, got {other:?}"),
    }

    match &converted[1].content {
        MessageContent::Parts(parts) => match &parts[0] {
            ContentPart::ImageUrl { image_url } => assert!(
                image_url.url.starts_with("data:image/png;base64,"),
                "image should be inlined as a data URI: {}",
                image_url.url
            ),
        },
        other => panic!("expected image parts, got {other:?}"),
    }
}

#[test]
fn supports_purpose_truth_table() {
    let config: Config = serde_yaml_ng::from_str(
        r#"
base_url: https://api.venice.ai/api/v1
api_key: test-key
text_generation:
  model_id: kimi-k2-5
speech_to_text:
  model_id: nvidia/parakeet-tdt-0.6b-v3
"#,
    )
    .expect("config should deserialize");

    let controller = Controller::new(config);

    assert!(controller.supports_purpose(AgentPurpose::TextGeneration));
    assert!(controller.supports_purpose(AgentPurpose::SpeechToText));
    assert!(controller.supports_purpose(AgentPurpose::CatchAll));
    assert!(!controller.supports_purpose(AgentPurpose::TextToSpeech));
    assert!(!controller.supports_purpose(AgentPurpose::ImageGeneration));
}

#[test]
fn supports_purpose_true_when_image_and_tts_blocks_present() {
    let config: Config = serde_yaml_ng::from_str(
        r#"
base_url: https://api.venice.ai/api/v1
api_key: test-key
text_to_speech:
  model_id: tts-kokoro
image_generation:
  model_id: chroma
"#,
    )
    .expect("config should deserialize");

    let controller = Controller::new(config);

    assert!(controller.supports_purpose(AgentPurpose::TextToSpeech));
    assert!(controller.supports_purpose(AgentPurpose::ImageGeneration));
}

#[test]
fn speech_request_serializes_voice_and_omits_unset() {
    let request = SpeechRequest {
        model: "tts-kokoro".to_owned(),
        input: "hello".to_owned(),
        voice: Some("af_sky".to_owned()),
        speed: None,
        response_format: Some("mp3".to_owned()),
        prompt: None,
        temperature: None,
        top_p: None,
    };

    let json = serde_json::to_string(&request).expect("serialize SpeechRequest");

    assert!(
        json.contains("\"voice\":\"af_sky\""),
        "voice should be present: {json}"
    );
    assert!(
        !json.contains("temperature"),
        "an unset knob must be omitted (not null): {json}"
    );
    assert!(!json.contains("null"), "no nulls belong in the body: {json}");
}

#[test]
fn generate_image_request_pins_flags_and_omits_unset() {
    let request = GenerateImageRequest {
        model: "chroma".to_owned(),
        prompt: "a cat".to_owned(),
        return_binary: false,
        variants: 1,
        negative_prompt: None,
        cfg_scale: None,
        steps: None,
        style_preset: None,
        seed: None,
        safe_mode: None,
        hide_watermark: None,
        format: None,
        width: None,
        height: None,
        aspect_ratio: None,
        resolution: None,
        quality: None,
        lora_strength: None,
        embed_exif_metadata: None,
        enable_web_search: None,
    };

    let json = serde_json::to_string(&request).expect("serialize GenerateImageRequest");

    assert!(json.contains("\"model\":\"chroma\""), "{json}");
    assert!(
        json.contains("\"return_binary\":false"),
        "return_binary must be pinned false: {json}"
    );
    assert!(
        json.contains("\"variants\":1"),
        "variants must be pinned 1: {json}"
    );
    assert!(
        !json.contains("cfg_scale"),
        "an unset knob must be omitted: {json}"
    );
    assert!(!json.contains("null"), "no nulls belong in the body: {json}");
}

#[test]
fn edit_image_request_carries_model_and_base64_image() {
    let request = EditImageRequest {
        model: "firered-image-edit".to_owned(),
        prompt: "make it a sunrise".to_owned(),
        image: "aGVsbG8=".to_owned(),
        output_format: None,
        aspect_ratio: None,
        resolution: None,
        safe_mode: None,
    };

    let json = serde_json::to_string(&request).expect("serialize EditImageRequest");

    assert!(
        json.contains("\"model\":\"firered-image-edit\""),
        "{json}"
    );
    assert!(
        json.contains("\"image\":\"aGVsbG8=\""),
        "the base64 image string must be present: {json}"
    );
    assert!(
        !json.contains("output_format"),
        "an unset knob must be omitted: {json}"
    );
}

#[test]
fn web_search_mode_off_deserializes_from_bare_yaml_off() {
    // `off` is a YAML-1.1 boolean but a plain string under serde_yaml_ng's YAML-1.2 core schema,
    // so it deserializes straight into the lowercase `WebSearchMode::Off`. This pins that the
    // sample config and docs can use the bare, unquoted `off` without it parsing as a boolean.
    let params: VeniceParameters =
        serde_yaml_ng::from_str("enable_web_search: off").expect("bare `off` should deserialize");

    assert!(matches!(params.enable_web_search, Some(WebSearchMode::Off)));
}
