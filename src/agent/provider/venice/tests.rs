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

use super::chat::{append_reasoning, derive_prompt_cache_key, render_with_citations};
use super::config::{Config, TextGenerationConfig, VeniceParameters, WebSearchMode};
use super::controller::Controller;
use super::utils::convert_llm_messages_to_venice;
use super::wire::{
    ChatCompletionRequest, ContentPart, EditImageRequest, GenerateImageRequest, MessageContent,
    SpeechRequest, WebSearchCitation,
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
fn converts_text_image_and_file_to_content_parts() {
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

    let converted =
        convert_llm_messages_to_venice(messages).expect("conversion should succeed");

    // Text, image, AND file all survive now: the file is no longer warn-skipped.
    assert_eq!(converted.len(), 3);

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
            other => panic!("expected an image part, got {other:?}"),
        },
        other => panic!("expected image parts, got {other:?}"),
    }

    match &converted[2].content {
        MessageContent::Parts(parts) => match &parts[0] {
            ContentPart::File { file } => {
                assert!(
                    file.file_data.starts_with("data:application/pdf;base64,"),
                    "file should be inlined as a data URI: {}",
                    file.file_data
                );
                assert_eq!(file.filename.as_deref(), Some("doc.pdf"));
            }
            other => panic!("expected a file part, got {other:?}"),
        },
        other => panic!("expected file parts, got {other:?}"),
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

#[test]
fn request_places_sampling_top_level_and_verbosity_in_the_bag() {
    // The whole config-shape decision in one assertion: top-level knobs serialize at the top
    // level, the dual-position `verbosity` serializes inside the bag. Venice silently ignores a
    // top-level knob misplaced into the bag, so this is the guard against a silent no-op.
    let request = ChatCompletionRequest {
        model: "kimi-k2-5".to_owned(),
        messages: vec![],
        temperature: Some(0.5),
        max_completion_tokens: Some(1024),
        top_p: Some(0.5),
        frequency_penalty: None,
        presence_penalty: None,
        repetition_penalty: None,
        reasoning_effort: Some("high".to_owned()),
        prompt_cache_key: Some("00000000cafef00d".to_owned()),
        prompt_cache_retention: Some("24h".to_owned()),
        venice_parameters: Some(VeniceParameters {
            verbosity: Some("high".to_owned()),
            ..Default::default()
        }),
    };

    let json = serde_json::to_value(&request).expect("serialize request");

    assert_eq!(json["top_p"], 0.5);
    assert_eq!(json["reasoning_effort"], "high");
    assert_eq!(json["prompt_cache_retention"], "24h");
    assert_eq!(json["prompt_cache_key"], "00000000cafef00d");

    assert!(
        json.get("verbosity").is_none(),
        "verbosity must not be a top-level field: {json}"
    );
    assert_eq!(json["venice_parameters"]["verbosity"], "high");
    assert!(
        json["venice_parameters"].get("top_p").is_none(),
        "top_p must not be inside the bag: {json}"
    );
}

#[test]
fn config_defaults_prompt_cache_retention_to_24h() {
    // The programmatic default.
    assert_eq!(
        TextGenerationConfig::default()
            .prompt_cache_retention
            .as_deref(),
        Some("24h")
    );

    // A config that omits the key must ALSO default to 24h, via the named serde default. A bare
    // `#[serde(default)]` would yield None here and silently disable caching for such configs.
    let tg: TextGenerationConfig = serde_yaml_ng::from_str("model_id: kimi-k2-5\n")
        .expect("minimal config should deserialize");
    assert_eq!(
        tg.prompt_cache_retention.as_deref(),
        Some("24h"),
        "an omitted retention key must still default to 24h"
    );
}

#[test]
fn cache_key_is_stable_for_same_inputs_and_varies_otherwise() {
    let key = derive_prompt_cache_key("system prompt", "2024-09-20 (Friday), 18:34:15 UTC");

    // Identical inputs produce an identical key: this is what keeps turn 5 routing to the warm
    // server holding turns 1-4 (and what survives a process restart).
    assert_eq!(
        key,
        derive_prompt_cache_key("system prompt", "2024-09-20 (Friday), 18:34:15 UTC"),
        "identical inputs must produce an identical key"
    );
    assert_eq!(key.len(), 16, "the key is a 16-char hex string");
    assert!(key.chars().all(|c| c.is_ascii_hexdigit()));

    // A different conversation start time or a different prompt must change the key.
    assert_ne!(
        key,
        derive_prompt_cache_key("system prompt", "2024-09-21 (Saturday), 09:00:00 UTC"),
        "a different start time must change the key"
    );
    assert_ne!(
        key,
        derive_prompt_cache_key("other prompt", "2024-09-20 (Friday), 18:34:15 UTC"),
        "a different prompt must change the key"
    );
}

#[test]
fn citations_render_inline_refs_and_a_sources_block() {
    let citations = vec![WebSearchCitation {
        title: "Example Source".to_owned(),
        url: "https://example.com/a".to_owned(),
    }];

    let rendered = render_with_citations("the sky is blue^1^".to_owned(), &citations);

    assert!(
        rendered.contains("the sky is blue[1]"),
        "inline ^1^ becomes [1]: {rendered}"
    );
    assert!(
        rendered.contains("Sources:"),
        "a Sources block is appended: {rendered}"
    );
    assert!(
        rendered.contains("[1] [Example Source](https://example.com/a)"),
        "the source renders as a markdown link: {rendered}"
    );
}

#[test]
fn citations_absent_leaves_content_untouched() {
    let content = "plain answer, no web search".to_owned();
    assert_eq!(render_with_citations(content.clone(), &[]), content);
}

#[test]
fn citation_title_and_url_cannot_inject_markdown() {
    // A hostile page sets its title to break out of the link label and its URL to a non-http
    // scheme. Neither may produce a spoofed clickable link in the room.
    let citations = vec![WebSearchCitation {
        title: "evil](http://phish.example) take".to_owned(),
        url: "javascript:alert(1)".to_owned(),
    }];

    let rendered = render_with_citations("result^1^".to_owned(), &citations);

    assert!(
        rendered.contains("evil\\](http://phish.example) take"),
        "the title's brackets must be escaped so it cannot close the link label: {rendered}"
    );
    assert!(
        !rendered.contains("(javascript:alert(1))"),
        "a non-http(s) URL must never become a markdown link target: {rendered}"
    );
}

#[test]
fn chained_and_comma_citation_runs_each_expand_to_separate_refs() {
    let citations = vec![
        WebSearchCitation {
            title: "One".to_owned(),
            url: "https://example.com/1".to_owned(),
        },
        WebSearchCitation {
            title: "Two".to_owned(),
            url: "https://example.com/2".to_owned(),
        },
        WebSearchCitation {
            title: "Three".to_owned(),
            url: "https://example.com/3".to_owned(),
        },
    ];

    // Caret-chained run: Venice shares the caret between consecutive citations (`^2^3^`). The whole
    // run must expand, not just the first, with no orphaned `3^` left behind.
    let chained = render_with_citations("alpha^2^3^ and beta^1^".to_owned(), &citations);
    assert!(
        chained.contains("alpha[2][3] and beta[1]"),
        "a chained ^2^3^ run must expand to [2][3] with no orphaned caret: {chained}"
    );

    // Comma run.
    let comma = render_with_citations("gamma^1,3^".to_owned(), &citations);
    assert!(
        comma.contains("gamma[1][3]"),
        "a comma ^1,3^ run must expand to [1][3]: {comma}"
    );

    // Multi-digit citation indices survive intact.
    let multidigit = render_with_citations("delta^2^10^".to_owned(), &citations);
    assert!(
        multidigit.contains("delta[2][10]"),
        "a multi-digit chained run must expand to [2][10]: {multidigit}"
    );
}

#[test]
fn malformed_citation_degrades_instead_of_failing() {
    // A citation arriving without a `url` must still deserialize (to an empty default) rather than
    // failing the whole response parse and losing an otherwise-good answer.
    let parsed: WebSearchCitation = serde_json::from_str(r#"{"title":"Only a title"}"#)
        .expect("a citation missing `url` should still deserialize");
    assert_eq!(parsed.url, "");

    // Rendering citations with missing fields stays graceful: no empty `[]()` link, no panic.
    let citations = vec![
        WebSearchCitation {
            title: String::new(),
            url: "https://example.com/u".to_owned(),
        },
        WebSearchCitation {
            title: String::new(),
            url: String::new(),
        },
    ];
    let rendered = render_with_citations("answer^1^2^".to_owned(), &citations);
    assert!(
        rendered.contains("[1] [https://example.com/u](https://example.com/u)"),
        "a citation with no title falls back to the URL as link text: {rendered}"
    );
    assert!(
        rendered.contains("[2] (source unavailable)"),
        "a citation with neither title nor URL renders a placeholder: {rendered}"
    );
}

#[test]
fn reasoning_is_appended_only_when_show_reasoning_is_set() {
    let base = "the answer".to_owned();

    // Off (the default): thinking is dropped, never reaching the room.
    let off = append_reasoning(base.clone(), Some("secret thinking".to_owned()), false);
    assert_eq!(off, "the answer");

    // On: thinking is appended below the answer.
    let on = append_reasoning(base.clone(), Some("visible thinking".to_owned()), true);
    assert!(on.contains("the answer"));
    assert!(on.contains("visible thinking"));

    // On but empty or missing reasoning: nothing is appended.
    assert_eq!(
        append_reasoning(base.clone(), Some("   ".to_owned()), true),
        "the answer"
    );
    assert_eq!(append_reasoning(base.clone(), None, true), "the answer");
}

#[test]
fn oversized_file_is_rejected() {
    let messages = vec![LLMMessage {
        author: LLMAuthor::User,
        sender_id: None,
        timestamp: chrono::Utc::now(),
        content: LLMMessageContent::File(FileDetails::new(
            FileMessageEventContent::plain(
                "big.pdf".to_owned(),
                OwnedMxcUri::from("mxc://example.com/big"),
            ),
            mime::APPLICATION_PDF,
            vec![0u8; 25 * 1024 * 1024 + 1],
        )),
    }];

    assert!(
        convert_llm_messages_to_venice(messages).is_err(),
        "a file over the 25MB limit must be rejected"
    );
}
