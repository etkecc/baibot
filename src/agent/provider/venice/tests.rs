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

use super::chat::{
    append_reasoning, create_request, derive_prompt_cache_key, render_with_citations,
    strip_thinking_blocks, without_tools,
};
use super::config::{Config, TextGenerationConfig, VeniceParameters, WebSearchMode};
use super::controller::Controller;
use super::utils::{
    convert_llm_messages_to_venice, has_tool_calls, tool_calls, tool_result_messages, tools,
};
use super::wire::{
    ChatCompletionRequest, ChatCompletionResponse, ContentPart, EditImageRequest,
    GenerateImageRequest, MessageContent, ResponseMessage, SpeechRequest, WebSearchCitation,
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
    assert!(
        !json.contains("null"),
        "no nulls belong in the body: {json}"
    );
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

    let converted = convert_llm_messages_to_venice(messages).expect("conversion should succeed");

    // Text, image, AND file all survive now: the file is no longer warn-skipped.
    assert_eq!(converted.len(), 3);

    match converted[0].content.as_ref().unwrap() {
        MessageContent::Text(text) => assert_eq!(text, "describe this"),
        other => panic!("expected bare text, got {other:?}"),
    }

    match converted[1].content.as_ref().unwrap() {
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

    match converted[2].content.as_ref().unwrap() {
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
    assert!(
        !json.contains("null"),
        "no nulls belong in the body: {json}"
    );
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
    assert!(
        !json.contains("null"),
        "no nulls belong in the body: {json}"
    );
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

    assert!(json.contains("\"model\":\"firered-image-edit\""), "{json}");
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
        tools: vec![],
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

    // On: thinking is appended below the answer in a collapsible <details> block (folded by
    // default, expandable in clients that support it).
    let on = append_reasoning(base.clone(), Some("  visible thinking  ".to_owned()), true);
    assert!(on.starts_with("the answer"));
    assert!(on.contains("<details><summary>💭 Reasoning</summary>"));
    assert!(on.contains("</details>"));
    // The reasoning sits as its own markdown block (blank lines around it) and is trimmed.
    assert!(on.contains("\n\nvisible thinking\n\n"));

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

fn tool_definition() -> crate::tools::ToolDefinition {
    crate::tools::ToolDefinition {
        name: "room_history".to_owned(),
        description: "Reads the room's recent messages.".to_owned(),
        parameters: serde_json::json!({"type": "object", "properties": {}}),
    }
}

fn tool_params(tool_instructions: &str) -> crate::agent::provider::entity::TextGenerationParams {
    crate::agent::provider::entity::TextGenerationParams {
        tools: vec![tool_definition()],
        tool_instructions: Some(tool_instructions.to_owned()),
        ..Default::default()
    }
}

fn user_conversation() -> crate::conversation::llm::Conversation {
    crate::conversation::llm::Conversation {
        messages: vec![LLMMessage {
            author: LLMAuthor::User,
            sender_id: None,
            timestamp: chrono::Utc::now(),
            content: LLMMessageContent::Text("Hi".to_owned()),
        }],
    }
}

fn response_message(json: serde_json::Value) -> ResponseMessage {
    serde_json::from_value(json).expect("response message should deserialize")
}

#[test]
fn bot_tools_are_on_unless_turned_off() {
    let config: TextGenerationConfig =
        serde_yaml_ng::from_str("model_id: kimi-k2-5").expect("config should deserialize");
    assert!(config.bot_tools);
    assert!(TextGenerationConfig::default().bot_tools);

    let config: TextGenerationConfig =
        serde_yaml_ng::from_str("model_id: kimi-k2-5\nbot_tools: false")
            .expect("config should deserialize");
    assert!(!config.bot_tools);
}

#[test]
fn offered_tools_are_sent_as_functions_and_none_are_omitted() {
    let config = TextGenerationConfig::default();

    let request = create_request(&config, user_conversation(), &tool_params("Use them."))
        .expect("request should build");
    let json = serde_json::to_value(&request).expect("serialize request");

    assert_eq!(
        json["tools"],
        serde_json::json!([{"type": "function", "function": {
            "name": "room_history",
            "description": "Reads the room's recent messages.",
            "parameters": {"type": "object", "properties": {}}
        }}])
    );

    let request = create_request(&config, user_conversation(), &Default::default())
        .expect("request should build");
    let json = serde_json::to_value(&request).expect("serialize request");
    assert!(json.get("tools").is_none(), "no tools, no field: {json}");
}

#[test]
fn without_tools_the_tool_instructions_and_their_cache_key_go_too() {
    let config = TextGenerationConfig {
        prompt: Some("Be brief.".to_owned()),
        ..Default::default()
    };
    let mut params = tool_params("Use the tools.");

    let mut request =
        create_request(&config, user_conversation(), &params).expect("request should build");
    let key_with_tools = request.prompt_cache_key.clone();

    let plain_request = create_request(&config, user_conversation(), &Default::default())
        .expect("request should build");

    without_tools(&mut request, &mut params, &config, user_conversation())
        .expect("request should build");

    assert!(request.tools.is_empty());
    assert!(params.tools.is_empty());
    assert!(matches!(
        request.messages[0].content.as_ref(),
        Some(MessageContent::Text(text)) if text == "Be brief."
    ));
    assert_ne!(request.prompt_cache_key, key_with_tools);
    assert_eq!(request.prompt_cache_key, plain_request.prompt_cache_key);
}

#[test]
fn without_tools_a_system_prompt_of_only_tool_instructions_goes() {
    let config = TextGenerationConfig {
        prompt: None,
        ..Default::default()
    };
    let mut params = tool_params("Use the tools.");

    let mut request =
        create_request(&config, user_conversation(), &params).expect("request should build");
    assert_eq!(request.messages[0].role, "system");

    without_tools(&mut request, &mut params, &config, user_conversation())
        .expect("request should build");

    assert_eq!(request.messages.len(), 1);
    assert_eq!(request.messages[0].role, "user");
}

#[test]
fn tool_calls_are_read_and_sent_back_with_venices_search_results() {
    let offered = tools(&[tool_definition()]);

    // Venice's web search documents arrive as a call too, and items may be odd (`null`, no function name).
    let message = response_message(serde_json::json!({
        "role": "assistant",
        "content": "Let me check.",
        "reasoning_content": "The user wants the room's history.",
        "tool_calls": [
            {"id": "call_abc", "type": "function", "function": {"name": "room_history", "arguments": "{\"limit\":5}"}},
            {"id": "web_search_call", "type": "function", "function": {"name": "web_search", "arguments": "{\"documents\":[]}"}},
            null,
            {"id": "call_null", "type": "function", "function": {"name": null}},
            {"type": "function", "function": {"name": "room_history", "arguments": {"limit": 3}}},
            {"id": "call_abc", "type": "function", "function": {"name": "made_up", "arguments": null}}
        ]
    }));

    assert!(has_tool_calls(&message, &offered));
    assert!(!has_tool_calls(&message, &[]));

    let (calls, assistant_message, venice_results) = tool_calls(message, &offered);

    let calls: Vec<(&str, &str, &str)> = calls
        .iter()
        .map(|call| {
            (
                call.id.as_str(),
                call.name.as_str(),
                call.arguments.as_str(),
            )
        })
        .collect();
    assert_eq!(
        calls,
        [
            ("call_abc", "room_history", r#"{"limit":5}"#),
            ("call_1", "room_history", r#"{"limit":3}"#),
            // Unknown names count as the model's (so it hears they failed), with ids kept unique.
            ("call_2", "made_up", "{}"),
        ]
    );

    assert_eq!(
        serde_json::to_value(assistant_message).expect("serialize message"),
        serde_json::json!({
            "role": "assistant",
            "content": "Let me check.",
            "reasoning_content": "The user wants the room's history.",
            "tool_calls": [
                // Venice's search results go back too (answered below, first), so the model keeps them.
                {"id": "web_search_call", "type": "function", "function": {"name": "web_search", "arguments": "{\"documents\":[]}"}},
                {"id": "call_abc", "type": "function", "function": {"name": "room_history", "arguments": "{\"limit\":5}"}},
                {"id": "call_1", "type": "function", "function": {"name": "room_history", "arguments": "{\"limit\":3}"}},
                {"id": "call_2", "type": "function", "function": {"name": "made_up", "arguments": "{}"}}
            ]
        })
    );

    assert_eq!(
        serde_json::to_value(venice_results).expect("serialize messages"),
        serde_json::json!([
            {"role": "tool", "content": "{\"documents\":[]}", "tool_call_id": "web_search_call"}
        ])
    );
}

#[test]
fn venices_own_calls_get_unique_ids_when_sent_back() {
    let offered = tools(&[tool_definition()]);

    let message = response_message(serde_json::json!({
        "role": "assistant",
        "content": null,
        "tool_calls": [
            {"id": "web_search_call", "type": "function", "function": {"name": "room_history", "arguments": "{}"}},
            {"id": "web_search_call", "type": "function", "function": {"name": "web_search", "arguments": {"documents": []}}},
            {"type": "function", "function": {"name": "venice_web_search_documents", "arguments": "{\"documents\":[]}"}},
            {"id": "web_search_call", "type": "function", "function": {"name": "web_search", "arguments": "{\"documents\":[{\"id\":\"1\"}]}"}},
            // Search calls without documents (e.g. a query made up by the model, or nothing) aren't results.
            {"id": "call_fake", "type": "function", "function": {"name": "web_search", "arguments": "{\"query\":\"Rust\"}"}},
            {"id": "call_fake2", "type": "function", "function": {"name": "web_search", "arguments": "{}"}},
            {"id": "call_fake3", "type": "function", "function": {"name": "venice_web_search_documents", "arguments": "[]"}},
            {"id": "call_x", "type": "function", "function": {"name": "x_search", "arguments": null}},
            // Venice's in name only (e.g. made up by the model): not sent back as if it had run.
            {"id": "call_y", "type": "function", "function": {"name": "venice_lookup", "arguments": "{\"query\":\"x\"}"}}
        ]
    }));

    let (calls, assistant_message, venice_results) = tool_calls(message, &offered);

    // An offered name with Venice's id is still Venice's (not run), but only search calls are sent back.
    assert!(calls.is_empty());

    let ids: Vec<&str> = assistant_message
        .tool_calls
        .iter()
        .map(|call| call.id.as_str())
        .collect();
    assert_eq!(
        ids,
        ["web_search_call", "web_search_call_", "web_search_call__"]
    );

    let result_ids: Vec<&str> = venice_results
        .iter()
        .map(|message| message.tool_call_id.as_deref().unwrap_or_default())
        .collect();
    assert_eq!(result_ids, ids);
    assert_eq!(
        assistant_message.tool_calls[0].function.arguments,
        r#"{"documents":[]}"#
    );
}

#[test]
fn only_venices_own_calls_are_no_tool_calls() {
    let offered = tools(&[tool_definition()]);

    let message = response_message(serde_json::json!({
        "role": "assistant",
        "content": "Here is what I found.^1^",
        "tool_calls": [
            {"id": "web_search_call", "type": "function", "function": {"name": "web_search", "arguments": "{}"}},
            {"id": "call_x", "type": "function", "function": {"name": "venice_web_search_documents", "arguments": "{}"}}
        ]
    }));
    assert!(!has_tool_calls(&message, &offered));

    let message = response_message(
        serde_json::json!({"role": "assistant", "content": "Hi", "tool_calls": null}),
    );
    assert!(!has_tool_calls(&message, &offered));
}

#[test]
fn a_response_with_tool_calls_deserializes() {
    let response: ChatCompletionResponse = serde_json::from_value(serde_json::json!({
        "id": "chatcmpl-1",
        "choices": [{
            "index": 0,
            "finish_reason": "tool_calls",
            "logprobs": null,
            "message": {
                "role": "assistant",
                "content": null,
                "reasoning_content": null,
                "tool_calls": [{"id": "call_1", "type": "function", "function": {"name": "room_history", "arguments": "{}"}}]
            }
        }]
    }))
    .expect("response should deserialize");

    let choice = &response.choices[0];
    assert_eq!(choice.finish_reason.as_deref(), Some("tool_calls"));
    assert!(choice.message.content.is_none());
    assert!(has_tool_calls(
        &choice.message,
        &tools(&[tool_definition()])
    ));
}

#[test]
fn tool_results_become_tool_messages() {
    let call = |id: &str| crate::tools::ToolCall {
        id: id.to_owned(),
        name: "room_history".to_owned(),
        arguments: "{}".to_owned(),
    };

    let messages = tool_result_messages(vec![
        crate::tools::ToolResult::success(&call("call_1"), "Alice: hi".to_owned()),
        crate::tools::ToolResult::error(&call("call_2"), "Timed out"),
    ]);

    assert_eq!(
        serde_json::to_value(messages).expect("serialize messages"),
        serde_json::json!([
            {"role": "tool", "content": "Alice: hi", "tool_call_id": "call_1"},
            {"role": "tool", "content": "Error: Timed out", "tool_call_id": "call_2"}
        ])
    );
}

/// A stand-in for the Venice API: answers requests with the given responses (status and JSON body) in order, one
/// connection each, and keeps the JSON bodies of the requests.
struct StubServer {
    base_url: String,
    requests: std::sync::Arc<std::sync::Mutex<Vec<serde_json::Value>>>,
}

impl StubServer {
    fn start(responses: Vec<(u16, serde_json::Value)>) -> Self {
        use std::io::{BufRead, BufReader, Read, Write};

        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let base_url = format!("http://{}/api/v1", listener.local_addr().expect("address"));
        let requests = std::sync::Arc::new(std::sync::Mutex::new(vec![]));

        let recorded = requests.clone();
        std::thread::spawn(move || {
            for (status, body) in responses {
                let Ok((stream, _)) = listener.accept() else {
                    return;
                };
                // A broken exchange fails the test instead of hanging it.
                let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(10)));
                let mut reader = BufReader::new(stream);

                let mut content_length = 0;
                loop {
                    let mut line = String::new();
                    if reader.read_line(&mut line).unwrap_or(0) == 0 || line == "\r\n" {
                        break;
                    }
                    if let Some((name, value)) = line.split_once(':')
                        && name.eq_ignore_ascii_case("content-length")
                    {
                        content_length = value.trim().parse().unwrap_or(0);
                    }
                }

                let mut request_body = vec![0; content_length];
                if reader.read_exact(&mut request_body).is_err() {
                    return;
                }
                recorded
                    .lock()
                    .unwrap()
                    .push(serde_json::from_slice(&request_body).unwrap_or_default());

                let body = body.to_string();
                let response = format!(
                    "HTTP/1.1 {status} Stub\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = reader.get_mut().write_all(response.as_bytes());
            }
        });

        Self { base_url, requests }
    }

    fn requests(&self) -> Vec<serde_json::Value> {
        self.requests.lock().unwrap().clone()
    }
}

/// A client for [`StubServer`]: no proxies (it's on the loopback interface) and a deadline, so a broken exchange fails
/// the test instead of hanging it.
fn stub_client() -> reqwest::Client {
    reqwest::Client::builder()
        .no_proxy()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .expect("client")
}

fn stub_config(base_url: &str) -> Config {
    Config {
        base_url: base_url.to_owned(),
        api_key: "test-key".to_owned(),
        text_generation: Some(TextGenerationConfig {
            model_id: "some-model".to_owned(),
            prompt: Some("Be brief.".to_owned()),
            reasoning_effort: Some("high".to_owned()),
            ..Default::default()
        }),
        speech_to_text: None,
        text_to_speech: None,
        image_generation: None,
    }
}

fn completion(message: serde_json::Value, finish_reason: &str) -> serde_json::Value {
    serde_json::json!({
        "id": "chatcmpl-1",
        "choices": [{"index": 0, "finish_reason": finish_reason, "message": message}]
    })
}

fn final_text(result: crate::agent::provider::entity::TextGenerationResult) -> String {
    match result {
        crate::agent::provider::entity::TextGenerationResult::Final { text } => text,
        crate::agent::provider::entity::TextGenerationResult::ToolCalls { .. } => {
            panic!("expected a final answer, got tool calls")
        }
    }
}

#[tokio::test]
async fn unsupported_fields_and_tools_are_dropped_and_remembered() {
    let capability_rejection = serde_json::json!({
        "error": "Invalid request parameters",
        "issues": [{"path": ["tools"], "message": "tools is not supported by this model"}]
    });

    let server = StubServer::start(vec![
        (
            400,
            serde_json::json!({"error": "Extra inputs are not permitted, field: 'reasoning_effort', value: 'high'"}),
        ),
        (400, capability_rejection),
        (
            200,
            completion(
                serde_json::json!({"role": "assistant", "content": "Hello!"}),
                "stop",
            ),
        ),
        (
            200,
            completion(
                serde_json::json!({"role": "assistant", "content": "Hello again!"}),
                "stop",
            ),
        ),
    ]);

    let config = stub_config(&server.base_url);
    let http = stub_client();
    let cache = super::recovery::UnsupportedFieldsCache::default();

    let result = super::chat::generate_text(
        &config,
        &http,
        &cache,
        user_conversation(),
        tool_params("Use the tools."),
    )
    .await
    .expect("the reply should work without the unsupported parts");
    assert_eq!(final_text(result), "Hello!");

    let requests = server.requests();
    assert_eq!(requests.len(), 3);
    assert_eq!(requests[0]["reasoning_effort"], "high");
    assert!(requests[0]["tools"].is_array());
    assert_eq!(
        requests[0]["messages"][0]["content"],
        "Be brief.\n\nUse the tools."
    );

    assert!(requests[1].get("reasoning_effort").is_none());
    assert!(requests[1]["tools"].is_array());

    // Rebuilt without the tools, still without what the model rejected.
    assert!(requests[2].get("tools").is_none());
    assert!(requests[2].get("reasoning_effort").is_none());
    assert_eq!(requests[2]["messages"][0]["content"], "Be brief.");
    assert_ne!(
        requests[2]["prompt_cache_key"],
        requests[1]["prompt_cache_key"]
    );

    let known = cache.known_for("some-model");
    assert!(known.contains("reasoning_effort"));
    assert!(known.contains(super::recovery::TOOLS_FIELD));

    // The next reply leaves both out from the start, with the same cache key.
    let result = super::chat::generate_text(
        &config,
        &http,
        &cache,
        user_conversation(),
        tool_params("Use the tools."),
    )
    .await
    .expect("the next reply should work at once");
    assert_eq!(final_text(result), "Hello again!");

    let requests = server.requests();
    assert_eq!(requests.len(), 4);
    assert!(requests[3].get("reasoning_effort").is_none());
    assert!(requests[3].get("tools").is_none());
    assert_eq!(requests[3]["messages"][0]["content"], "Be brief.");
    assert_eq!(
        requests[3]["prompt_cache_key"],
        requests[2]["prompt_cache_key"]
    );
}

#[tokio::test]
async fn tools_are_not_given_up_when_the_retry_fails_too() {
    let failed_retries = [
        (500, serde_json::json!({"error": "Internal error"})),
        (200, serde_json::json!({"choices": []})),
        (200, serde_json::json!({"unexpected": true})),
        (
            200,
            completion(
                serde_json::json!({"role": "assistant", "content": " "}),
                "stop",
            ),
        ),
        // Blank, but with sources (which would make the reply look like something).
        (
            200,
            serde_json::json!({
                "choices": [{"index": 0, "finish_reason": "stop", "message": {"role": "assistant", "content": " "}}],
                "venice_parameters": {"web_search_citations": [{"title": "Rust", "url": "https://www.rust-lang.org/"}]}
            }),
        ),
    ];

    for retry in failed_retries {
        // A blank answer is no answer either (the caller treats it as a failure), so not counted as working.
        let server = StubServer::start(vec![
            (
                400,
                serde_json::json!({"error": "Model some-model does not support function calling"}),
            ),
            retry.clone(),
        ]);

        let config = stub_config(&server.base_url);
        let cache = super::recovery::UnsupportedFieldsCache::default();

        // Whatever comes of it, the model keeps its tools.
        let _ = super::chat::generate_text(
            &config,
            &stub_client(),
            &cache,
            user_conversation(),
            tool_params("Use the tools."),
        )
        .await;

        assert_eq!(server.requests().len(), 2, "{retry:?}");
        assert!(
            !cache
                .known_for("some-model")
                .contains(super::recovery::TOOLS_FIELD),
            "{retry:?}"
        );
    }
}

#[test]
fn thinking_is_kept_in_requests_with_tools_only() {
    let config = TextGenerationConfig::default();
    let strip_flag = |request: &ChatCompletionRequest| {
        request
            .venice_parameters
            .as_ref()
            .and_then(|vp| vp.strip_thinking_response)
    };

    let mut params = tool_params("Use the tools.");
    let mut request =
        create_request(&config, user_conversation(), &params).expect("request should build");
    assert_eq!(strip_flag(&request), Some(false));

    // Without tools (also after the model rejected them), Venice strips it as configured.
    without_tools(&mut request, &mut params, &config, user_conversation())
        .expect("request should build");
    assert_eq!(strip_flag(&request), Some(true));

    let request = create_request(&config, user_conversation(), &Default::default())
        .expect("request should build");
    assert_eq!(strip_flag(&request), Some(true));

    // Also when asked for with a model ID suffix, which takes precedence over the field.
    let config = TextGenerationConfig {
        model_id: "some-model:enable_web_search=on&strip_thinking_response=true".to_owned(),
        venice_parameters: None,
        ..Default::default()
    };
    let mut params = tool_params("Use the tools.");
    let mut request =
        create_request(&config, user_conversation(), &params).expect("request should build");
    assert_eq!(
        request.model,
        "some-model:enable_web_search=on&strip_thinking_response=false"
    );

    without_tools(&mut request, &mut params, &config, user_conversation())
        .expect("request should build");
    assert_eq!(request.model, config.model_id);

    // A suffix turning it off wins over the field.
    let config = TextGenerationConfig {
        model_id: "some-model:strip_thinking_response=false".to_owned(),
        ..Default::default()
    };
    let request = create_request(&config, user_conversation(), &tool_params("Use the tools."))
        .expect("request should build");
    assert_eq!(request.model, config.model_id);
}

#[tokio::test]
async fn the_bot_strips_thinking_only_when_the_agent_wants_it() {
    let thinking_answer = || {
        (
            200,
            completion(
                serde_json::json!({"role": "assistant", "content": "<think>Hmm.</think>Hi!"}),
                "stop",
            ),
        )
    };

    let cases = [
        // The field asks for it, a suffix says no.
        (
            "some-model:strip_thinking_response=false",
            Some(true),
            "<think>Hmm.</think>Hi!",
        ),
        ("some-model", Some(true), "Hi!"),
        ("some-model", Some(false), "<think>Hmm.</think>Hi!"),
    ];

    for (model_id, strip_field, expected) in cases {
        let server = StubServer::start(vec![thinking_answer()]);

        let mut config = stub_config(&server.base_url);
        let text_generation = config.text_generation.as_mut().unwrap();
        text_generation.model_id = model_id.to_owned();
        text_generation
            .venice_parameters
            .as_mut()
            .unwrap()
            .strip_thinking_response = strip_field;

        let result = super::chat::generate_text(
            &config,
            &stub_client(),
            &super::recovery::UnsupportedFieldsCache::default(),
            user_conversation(),
            tool_params("Use the tools."),
        )
        .await
        .expect("reply");

        assert_eq!(final_text(result), expected, "{model_id} {strip_field:?}");
    }
}

#[test]
fn thinking_blocks_are_stripped() {
    assert_eq!(
        strip_thinking_blocks("<think>Hmm.\nOk.</think>\n\nHello <b>there</b>."),
        "Hello <b>there</b>."
    );
    // Cut off by the response limit.
    assert_eq!(strip_thinking_blocks("<think>Still thinking about"), "");
    // Only thinking at the start, not tags in the reply (e.g. in a code example).
    assert_eq!(
        strip_thinking_blocks("Use `<think>x</think>` tags."),
        "Use `<think>x</think>` tags."
    );
    assert_eq!(strip_thinking_blocks("  Plain.  "), "  Plain.  ");
}

#[tokio::test]
async fn a_turn_with_web_search_and_tool_calls_keeps_the_search_results_and_citations() {
    // As seen live: Venice searches in the first round only, so the second one has neither results nor citations.
    let server = StubServer::start(vec![
        (
            200,
            serde_json::json!({
                "id": "chatcmpl-1",
                "choices": [{
                    "index": 0,
                    "finish_reason": "tool_calls",
                    "message": {
                        "role": "assistant",
                        "content": null,
                        "reasoning_content": "Need the history.",
                        "tool_calls": [
                            {"id": "web_search_call", "type": "function", "function": {"name": "web_search", "arguments": "{\"documents\":[{\"id\":\"1\"}]}"}},
                            {"id": "call_1", "type": "function", "function": {"name": "room_history", "arguments": "{\"limit\":5}"}}
                        ]
                    }
                }],
                "venice_parameters": {"web_search_citations": [{"title": "Rust", "url": "https://www.rust-lang.org/"}]}
            }),
        ),
        (
            200,
            completion(
                serde_json::json!({
                    "role": "assistant",
                    "content": "<think>Alice asked; cite it.</think>\nAlice asked about Rust^1^."
                }),
                "stop",
            ),
        ),
    ]);

    let config = stub_config(&server.base_url);
    let http = stub_client();
    let cache = super::recovery::UnsupportedFieldsCache::default();

    let result = super::chat::generate_text(
        &config,
        &http,
        &cache,
        user_conversation(),
        tool_params("Use the tools."),
    )
    .await
    .expect("the first round should work");

    let crate::agent::provider::entity::TextGenerationResult::ToolCalls {
        text,
        calls,
        continuation,
        ..
    } = result
    else {
        panic!("expected tool calls");
    };
    assert_eq!(text, None);
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].id, "call_1");

    let params = crate::agent::provider::entity::TextGenerationParams {
        continuation: Some(crate::tools::ToolContinuation {
            state: continuation,
            results: vec![crate::tools::ToolResult::success(
                &calls[0],
                "Alice: what about Rust?".to_owned(),
            )],
        }),
        ..tool_params("Use the tools.")
    };

    let result = super::chat::generate_text(
        &config,
        &http,
        &cache,
        crate::conversation::llm::Conversation { messages: vec![] },
        params,
    )
    .await
    .expect("the second round should work");

    let text = final_text(result);
    assert!(text.starts_with("Alice asked about Rust[1]."), "{text}");
    assert!(text.contains("https://www.rust-lang.org/"), "{text}");

    let requests = server.requests();
    assert_eq!(requests.len(), 2);

    // The continuation is the first request, the model's calls and Venice's search, and their results.
    let messages = requests[1]["messages"].as_array().expect("messages");
    assert_eq!(messages.len(), 5);
    assert_eq!(
        messages[..2],
        requests[0]["messages"].as_array().unwrap()[..]
    );
    assert_eq!(
        messages[2],
        serde_json::json!({
            "role": "assistant",
            "reasoning_content": "Need the history.",
            // In the order of the results below (Venice's first), for templates which drop the call ids.
            "tool_calls": [
                {"id": "web_search_call", "type": "function", "function": {"name": "web_search", "arguments": "{\"documents\":[{\"id\":\"1\"}]}"}},
                {"id": "call_1", "type": "function", "function": {"name": "room_history", "arguments": "{\"limit\":5}"}}
            ]
        })
    );
    assert_eq!(
        messages[3],
        serde_json::json!({"role": "tool", "content": "{\"documents\":[{\"id\":\"1\"}]}", "tool_call_id": "web_search_call"})
    );
    assert_eq!(
        messages[4],
        serde_json::json!({"role": "tool", "content": "Alice: what about Rust?", "tool_call_id": "call_1"})
    );
    // The thinking stays in the transcript, and is stripped from the answer instead.
    assert_eq!(
        requests[0]["venice_parameters"]["strip_thinking_response"],
        false
    );
    assert_eq!(requests[1]["tools"], requests[0]["tools"]);
    assert_eq!(
        requests[1]["venice_parameters"],
        requests[0]["venice_parameters"]
    );
}

#[tokio::test]
async fn a_thinking_only_retry_is_no_answer() {
    let server = StubServer::start(vec![
        (
            400,
            serde_json::json!({"error": "Model some-model does not support function calling"}),
        ),
        (
            200,
            completion(
                serde_json::json!({"role": "assistant", "content": "<think>Still working</think>"}),
                "stop",
            ),
        ),
    ]);

    let mut config = stub_config(&server.base_url);
    config
        .text_generation
        .as_mut()
        .unwrap()
        .venice_parameters
        .as_mut()
        .unwrap()
        .strip_thinking_response = Some(false);
    let cache = super::recovery::UnsupportedFieldsCache::default();

    let _ = super::chat::generate_text(
        &config,
        &stub_client(),
        &cache,
        user_conversation(),
        tool_params("Use the tools."),
    )
    .await;

    assert_eq!(server.requests().len(), 2);
    assert!(
        !cache
            .known_for("some-model")
            .contains(super::recovery::TOOLS_FIELD)
    );
}

#[tokio::test]
async fn earlier_sources_go_to_the_answer_only() {
    let call = |id: &str| serde_json::json!({"id": id, "type": "function", "function": {"name": "room_history", "arguments": "{}"}});

    let server = StubServer::start(vec![
        (
            200,
            serde_json::json!({
                "id": "chatcmpl-1",
                "choices": [{
                    "index": 0,
                    "finish_reason": "tool_calls",
                    "message": {
                        "role": "assistant",
                        "content": null,
                        "tool_calls": [
                            {"id": "web_search_call", "type": "function", "function": {"name": "web_search", "arguments": "{\"documents\":[]}"}},
                            call("call_1")
                        ]
                    }
                }],
                "venice_parameters": {"web_search_citations": [{"title": "Rust", "url": "https://www.rust-lang.org/"}]}
            }),
        ),
        (
            200,
            completion(
                serde_json::json!({"role": "assistant", "content": "Checking more^1^.", "tool_calls": [call("call_2")]}),
                "tool_calls",
            ),
        ),
        (
            200,
            completion(
                serde_json::json!({"role": "assistant", "content": "Rust^1^."}),
                "stop",
            ),
        ),
    ]);

    let config = stub_config(&server.base_url);
    let http = stub_client();
    let cache = super::recovery::UnsupportedFieldsCache::default();

    let mut params = tool_params("Use the tools.");
    let mut conversation = user_conversation();
    let mut preambles = vec![];

    let text = loop {
        let result = super::chat::generate_text(&config, &http, &cache, conversation, params)
            .await
            .expect("each round should work");

        let crate::agent::provider::entity::TextGenerationResult::ToolCalls {
            text,
            calls,
            continuation,
            ..
        } = result
        else {
            break final_text(result);
        };

        preambles.push(text);
        conversation = crate::conversation::llm::Conversation { messages: vec![] };
        params = crate::agent::provider::entity::TextGenerationParams {
            continuation: Some(crate::tools::ToolContinuation {
                state: continuation,
                results: vec![crate::tools::ToolResult::success(&calls[0], "-".to_owned())],
            }),
            ..tool_params("Use the tools.")
        };
    };

    // A preamble counts against the tool budget, so the first round's sources aren't added to later ones again.
    assert_eq!(preambles, [None, Some("Checking more^1^.".to_owned())]);
    assert!(text.starts_with("Rust[1]."), "{text}");
    assert!(text.contains("https://www.rust-lang.org/"), "{text}");
}
