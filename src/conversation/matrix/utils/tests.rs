use chrono::{TimeZone, Utc};

use mxlink::matrix_sdk::ruma::OwnedUserId;

use crate::conversation::matrix::{
    MatrixMessage, MatrixMessageContent, MatrixMessageProcessingParams, SenderFilter,
};

fn regex_sender_filter(regexes: Vec<regex::Regex>) -> SenderFilter {
    SenderFilter::new(move |user_id| mxidwc::match_user_id(user_id.as_str(), &regexes))
}

#[test]
fn is_message_from_allowed_sender() {
    let bot_user_id =
        OwnedUserId::try_from("@bot:example.com").expect("Failed to parse bot user ID");
    let allowed_user_id = OwnedUserId::try_from("@user.someone:example.com").unwrap();
    let unallowed_user_id = OwnedUserId::try_from("@another:example.com").unwrap();

    let timestamp = Utc.with_ymd_and_hms(2024, 9, 20, 18, 34, 15).unwrap();

    let bot_message = MatrixMessage {
        sender_id: bot_user_id.to_owned(),
        content: MatrixMessageContent::Text("Hello!".to_owned()),
        mentioned_users: vec![],
        timestamp,
    };

    let allowed_user_message = MatrixMessage {
        sender_id: allowed_user_id.to_owned(),
        content: MatrixMessageContent::Text("Hello!".to_owned()),
        mentioned_users: vec![],
        timestamp,
    };

    let unallowed_user_message = MatrixMessage {
        sender_id: unallowed_user_id.to_owned(),
        content: MatrixMessageContent::Text("Hello!".to_owned()),
        mentioned_users: vec![],
        timestamp,
    };

    let parsed_regex = match mxidwc::parse_pattern("@user.*:example.com") {
        Ok(value) => value,
        Err(err) => {
            panic!("Error parsing regex: {}", err);
        }
    };

    let allowed_users = regex_sender_filter(vec![parsed_regex]);

    assert!(
        super::is_message_from_allowed_sender(&bot_message, &bot_user_id, &allowed_users),
        "Bot message should be allowed"
    );

    assert!(
        super::is_message_from_allowed_sender(&allowed_user_message, &bot_user_id, &allowed_users),
        "Allowed user message should be allowed"
    );

    assert!(
        !super::is_message_from_allowed_sender(
            &unallowed_user_message,
            &bot_user_id,
            &allowed_users,
        ),
        "Unallowed user message should be ignored"
    );

    assert!(
        super::is_message_from_allowed_sender(
            &unallowed_user_message,
            &bot_user_id,
            &SenderFilter::new(|_| true),
        ),
        "An empty list of allowed users lets everyone through"
    );
}

#[tokio::test]
async fn process_matrix_messages() {
    let bot_user_id =
        OwnedUserId::try_from("@bot:example.com").expect("Failed to parse bot user ID");
    let allowed_user_id = OwnedUserId::try_from("@user.someone:example.com").unwrap();
    let unallowed_user_id = OwnedUserId::try_from("@another:example.com").unwrap();

    let timestamp = Utc.with_ymd_and_hms(2024, 9, 20, 18, 34, 15).unwrap();

    let allowed_user_message = MatrixMessage {
        sender_id: allowed_user_id.to_owned(),
        content: MatrixMessageContent::Text("Hello from the user!".to_owned()),
        mentioned_users: vec![],
        timestamp,
    };

    let allowed_user_message_with_prefix = MatrixMessage {
        sender_id: allowed_user_id.to_owned(),
        content: MatrixMessageContent::Text("!bai Hello from the user!".to_owned()),
        mentioned_users: vec![],
        timestamp,
    };

    let allowed_user_message_with_prefix_no_space = MatrixMessage {
        sender_id: allowed_user_id.to_owned(),
        content: MatrixMessageContent::Text("!baiHello from the user!".to_owned()),
        mentioned_users: vec![],
        timestamp,
    };

    let allowed_user_message_with_prefix_full_width_space = MatrixMessage {
        sender_id: allowed_user_id.to_owned(),
        content: MatrixMessageContent::Text("!bai　Hello from the user!".to_owned()),
        mentioned_users: vec![],
        timestamp,
    };

    let bot_message = MatrixMessage {
        sender_id: bot_user_id.to_owned(),
        content: MatrixMessageContent::Text("Hello from the bot!".to_owned()),
        mentioned_users: vec![],
        timestamp,
    };

    let allowed_user_message_with_bot_mention = MatrixMessage {
        sender_id: allowed_user_id.to_owned(),
        content: MatrixMessageContent::Text("@baibot: Hello from the user!".to_owned()),
        mentioned_users: vec![bot_user_id.to_owned()],
        timestamp,
    };

    // The message text is the same as above - it mentions the bot, but the actually-mentioned user is another user.
    let allowed_user_message_with_another_user_mention = MatrixMessage {
        sender_id: allowed_user_id.to_owned(),
        content: allowed_user_message_with_bot_mention.content.clone(),
        mentioned_users: vec![allowed_user_id.to_owned()],
        timestamp,
    };

    let unallowed_user_message = MatrixMessage {
        sender_id: unallowed_user_id.to_owned(),
        content: MatrixMessageContent::Text("Hello from an unallowed user!".to_owned()),
        mentioned_users: vec![],
        timestamp,
    };

    let parsed_regex = match mxidwc::parse_pattern("@user.*:example.com") {
        Ok(value) => value,
        Err(err) => {
            panic!("Error parsing regex: {}", err);
        }
    };

    let allowed_users = regex_sender_filter(vec![parsed_regex]);

    let message_processing_params_basic =
        super::MatrixMessageProcessingParams::new(bot_user_id.to_owned(), allowed_users.clone());

    let message_processing_params_with_prefix_stripping =
        super::MatrixMessageProcessingParams::new(bot_user_id.to_owned(), allowed_users.clone())
            .with_first_message_prefixes_to_strip(vec!["!bai".to_owned()]);

    let message_processing_params_with_bot_user_prefix_stripping =
        super::MatrixMessageProcessingParams::new(bot_user_id.to_owned(), allowed_users.clone())
            .with_bot_user_prefixes_to_strip(vec!["@baibot: ".to_owned(), "@baibot".to_owned()]);

    struct TestCase {
        name: String,
        messages: Vec<MatrixMessage>,
        message_processing_params: MatrixMessageProcessingParams,
        expected_message_texts: Vec<String>,
    }

    let test_cases = vec![
        TestCase {
            name: "Messages by unallowed users are ignored".to_owned(),
            messages: vec![
                allowed_user_message.clone(),
                bot_message.clone(),
                unallowed_user_message.clone(),
            ],
            message_processing_params: message_processing_params_basic.clone(),
            expected_message_texts: vec![
                "Hello from the user!".to_owned(),
                "Hello from the bot!".to_owned(),
            ],
        },
        TestCase {
            name: "The first message with a prefix gets stripped if params configure it (regular space)".to_owned(),
            messages: vec![
                allowed_user_message_with_prefix.clone(),
                bot_message.clone(),
                allowed_user_message_with_prefix.clone(),
                unallowed_user_message.clone(),
            ],
            message_processing_params: message_processing_params_with_prefix_stripping.clone(),
            expected_message_texts: vec![
                "Hello from the user!".to_owned(),
                "Hello from the bot!".to_owned(),
                "!bai Hello from the user!".to_owned(),
            ],
        },
        TestCase {
            name: "The first message with a prefix gets stripped if params configure it (no space)".to_owned(),
            messages: vec![
                allowed_user_message_with_prefix_no_space.clone(),
                bot_message.clone(),
                allowed_user_message_with_prefix_no_space.clone(),
                unallowed_user_message.clone(),
            ],
            message_processing_params: message_processing_params_with_prefix_stripping.clone(),
            expected_message_texts: vec![
                "Hello from the user!".to_owned(),
                "Hello from the bot!".to_owned(),
                "!baiHello from the user!".to_owned(),
            ],
        },
        TestCase {
            name: "The first message with a prefix gets stripped if params configure it (full-width-space)".to_owned(),
            messages: vec![
                allowed_user_message_with_prefix_full_width_space.clone(),
                bot_message.clone(),
                allowed_user_message_with_prefix_full_width_space.clone(),
                unallowed_user_message.clone(),
            ],
            message_processing_params: message_processing_params_with_prefix_stripping.clone(),
            expected_message_texts: vec![
                "Hello from the user!".to_owned(),
                "Hello from the bot!".to_owned(),
                "!bai　Hello from the user!".to_owned(),
            ],
        },
        TestCase {
            name: "The first message with a prefix remains untouched if params leave it alone"
                .to_owned(),
            messages: vec![
                allowed_user_message_with_prefix.clone(),
                bot_message.clone(),
                allowed_user_message_with_prefix.clone(),
                unallowed_user_message.clone(),
            ],
            message_processing_params: message_processing_params_basic.clone(),
            expected_message_texts: vec![
                "!bai Hello from the user!".to_owned(),
                "Hello from the bot!".to_owned(),
                "!bai Hello from the user!".to_owned(),
            ],
        },
        TestCase {
            name: "Messages that mention the bot user get the bot user prefix stripped"
                .to_owned(),
            messages: vec![
                allowed_user_message_with_bot_mention.clone(),
                allowed_user_message_with_another_user_mention.clone(),
            ],
            message_processing_params: message_processing_params_with_bot_user_prefix_stripping.clone(),
            expected_message_texts: vec![
                "Hello from the user!".to_owned(),
                "@baibot: Hello from the user!".to_owned(),
            ],
        },
    ];

    for test_case in test_cases {
        let processed_messages = super::process_matrix_messages(
            &test_case.messages,
            &test_case.message_processing_params,
        )
        .await;

        let processed_message_texts = processed_messages
            .iter()
            .map(|message| match &message.content {
                MatrixMessageContent::Text(text) => text.clone(),
                _ => "".to_owned(),
            })
            .collect::<Vec<String>>();

        assert_eq!(
            processed_message_texts, test_case.expected_message_texts,
            "Test case {} failed",
            test_case.name,
        );
    }
}

#[test]
fn create_list_of_bot_user_prefixes_to_strip() {
    let bot_user_id =
        OwnedUserId::try_from("@baibot:example.com").expect("Failed to parse bot user ID");

    // Test case 1: Bot user with no display name
    let bot_display_name = None;
    let prefixes =
        super::create_list_of_bot_user_prefixes_to_strip(&bot_user_id, &bot_display_name);

    assert_eq!(
        prefixes,
        vec![
            "@baibot:example.com".to_string(),
            "@baibot".to_string(),
            "baibot".to_string(),
            ":".to_string()
        ]
    );

    // Test case 2: Bot user with display name
    let bot_display_name = Some("Assistant".to_string());
    let prefixes =
        super::create_list_of_bot_user_prefixes_to_strip(&bot_user_id, &bot_display_name);

    assert_eq!(
        prefixes,
        vec![
            "@baibot:example.com".to_string(),
            "@baibot".to_string(),
            "baibot".to_string(),
            "@Assistant".to_string(),
            "Assistant".to_string(),
            ":".to_string()
        ]
    );
}

fn room_message_event_with_replacement(
    replacement_sender: &str,
    replaced_event_id: &str,
) -> mxlink::matrix_sdk::ruma::events::room::message::OriginalSyncRoomMessageEvent {
    serde_json::from_value(serde_json::json!({
        "type": "m.room.message",
        "event_id": "$original:example.com",
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
                    "sender": replacement_sender,
                    "origin_server_ts": 2,
                    "content": {
                        "msgtype": "m.text",
                        "body": "* The answer",
                        "m.new_content": { "msgtype": "m.text", "body": "The answer" },
                        "m.relates_to": { "rel_type": "m.replace", "event_id": replaced_event_id },
                    },
                },
            },
        },
    }))
    .unwrap()
}

#[test]
fn latest_room_message_content_applies_bundled_edit() {
    let event = room_message_event_with_replacement("@bot:example.com", "$original:example.com");

    let (content, is_edited) = super::latest_room_message_content(&event);

    assert!(is_edited);
    assert_eq!(content.body(), "The answer");
    // The original's relations are kept.
    assert_eq!(
        content.thread().map(|thread| thread.event_id.as_str()),
        Some("$root:example.com")
    );
}

#[test]
fn effective_room_message_content_applies_edits() {
    // An original event with a bundled edit gets the edit's content (e.g. a "thinking…" notice that became the answer).
    let event = room_message_event_with_replacement("@bot:example.com", "$original:example.com");
    let (content, is_edited) = super::effective_room_message_content(&event);
    assert!(is_edited);
    assert_eq!(content.body(), "The answer");

    // A standalone edit is taken for the content it sets (not its `* `-prefixed fallback body).
    let event: mxlink::matrix_sdk::ruma::events::room::message::OriginalSyncRoomMessageEvent =
        serde_json::from_value(serde_json::json!({
            "type": "m.room.message",
            "event_id": "$edit:example.com",
            "sender": "@bot:example.com",
            "origin_server_ts": 2,
            "content": {
                "msgtype": "m.text",
                "body": "* The answer",
                "m.new_content": { "msgtype": "m.text", "body": "The answer" },
                "m.relates_to": { "rel_type": "m.replace", "event_id": "$original:example.com" },
            },
        }))
        .unwrap();
    let (content, is_edited) = super::effective_room_message_content(&event);
    assert!(is_edited);
    assert_eq!(content.body(), "The answer");
}

#[test]
fn latest_room_message_content_ignores_invalid_edits() {
    // Edits by another sender are ignored.
    let event = room_message_event_with_replacement("@other:example.com", "$original:example.com");
    let (content, is_edited) = super::latest_room_message_content(&event);
    assert!(!is_edited);
    assert_eq!(content.body(), "thinking…");

    // Edits of another event are ignored.
    let event = room_message_event_with_replacement("@bot:example.com", "$another:example.com");
    let (content, is_edited) = super::latest_room_message_content(&event);
    assert!(!is_edited);
    assert_eq!(content.body(), "thinking…");
}

#[test]
fn message_text_strips_reply_fallback_only_from_unedited_replies() {
    let reply: mxlink::matrix_sdk::ruma::events::room::message::RoomMessageEventContent =
        serde_json::from_value(serde_json::json!({
            "msgtype": "m.text",
            "body": "> <@alice:example.com> quoted text\n\nThe reply",
            "m.relates_to": { "m.in_reply_to": { "event_id": "$parent:example.com" } },
        }))
        .unwrap();

    // The original body of a reply may include a fallback, which is stripped.
    assert_eq!(
        super::message_text(&reply, false),
        Some(("The reply".to_owned(), false))
    );

    // Edited content has no fallback, so a quote it starts with is kept.
    assert_eq!(
        super::message_text(&reply, true),
        Some((
            "> <@alice:example.com> quoted text\n\nThe reply".to_owned(),
            false
        ))
    );
}

const CONFIGURATION: &str = "Configuration for agent `static/openai` (powered by the `openai` provider):\n```yml\napi_key: sk-123\n```";

fn message_event(
    content: serde_json::Value,
    edited_body: Option<&str>,
) -> mxlink::matrix_sdk::ruma::events::room::message::OriginalSyncRoomMessageEvent {
    let mut event = serde_json::json!({
        "type": "m.room.message",
        "event_id": "$original:example.com",
        "sender": "@bot:example.com",
        "origin_server_ts": 1,
        "content": content,
    });

    if let Some(edited_body) = edited_body {
        event["unsigned"] = serde_json::json!({"m.relations": {"m.replace": {
            "type": "m.room.message",
            "event_id": "$edit:example.com",
            "sender": "@bot:example.com",
            "origin_server_ts": 2,
            "content": {
                "msgtype": "m.text",
                "body": format!("* {edited_body}"),
                "m.new_content": {"msgtype": "m.text", "body": edited_body},
                "m.relates_to": {"rel_type": "m.replace", "event_id": "$original:example.com"},
            },
        }}});
    }

    serde_json::from_value(event).unwrap()
}

fn conversation_text(
    event: &mxlink::matrix_sdk::ruma::events::room::message::OriginalSyncRoomMessageEvent,
) -> Option<String> {
    super::conversation_message_content(event).map(|(_, text, _)| text)
}

#[test]
fn agent_configurations_are_left_out_of_conversations() {
    let text = |body: &str| serde_json::json!({"msgtype": "m.text", "body": body});

    assert_eq!(
        conversation_text(&message_event(text(CONFIGURATION), None)),
        None
    );

    // Edited into one, or out of one: the latest edit counts.
    assert_eq!(
        conversation_text(&message_event(text("hello"), Some(CONFIGURATION))),
        None
    );
    assert_eq!(
        conversation_text(&message_event(text(CONFIGURATION), Some("hello"))).as_deref(),
        Some("hello")
    );

    // A standalone edit (e.g. replied to directly) is taken for the content it sets.
    let edit = serde_json::json!({
        "msgtype": "m.text",
        "body": format!("* {CONFIGURATION}"),
        "m.new_content": {"msgtype": "m.text", "body": CONFIGURATION},
        "m.relates_to": {"rel_type": "m.replace", "event_id": "$other:example.com"},
    });
    assert_eq!(conversation_text(&message_event(edit, None)), None);

    // A reply within a thread, quoting one in its fallback, keeps only the reply.
    let quoted = CONFIGURATION
        .lines()
        .enumerate()
        .map(|(index, line)| {
            if index == 0 {
                format!("> <@bot:example.com> {line}")
            } else {
                format!("> {line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    let thread_reply = serde_json::json!({
        "msgtype": "m.text",
        "body": format!("{quoted}\n\nWhat does this agent do?"),
        "m.relates_to": {
            "rel_type": "m.thread",
            "event_id": "$root:example.com",
            "is_falling_back": false,
            "m.in_reply_to": {"event_id": "$root:example.com"},
        },
    });
    assert_eq!(
        conversation_text(&message_event(thread_reply, None)).as_deref(),
        Some("What does this agent do?")
    );

    // An agent id spanning lines doesn't hide one.
    let multiline_id = CONFIGURATION.replacen("static/openai", "static/open\nai", 1);
    assert_eq!(
        conversation_text(&message_event(text(&multiline_id), None)),
        None
    );

    // Thread metadata alone (falling back) doesn't make text starting like a quote a fallback.
    let falling_back = serde_json::json!({
        "msgtype": "m.text",
        "body": "> < 5 ms\n\nIs that threshold reasonable?",
        "m.relates_to": {
            "rel_type": "m.thread",
            "event_id": "$root:example.com",
            "is_falling_back": true,
            "m.in_reply_to": {"event_id": "$root:example.com"},
        },
    });
    assert_eq!(
        conversation_text(&message_event(falling_back, None)).as_deref(),
        Some("> < 5 ms\n\nIs that threshold reasonable?")
    );

    // Neither does a genuine reply without a fallback.
    let genuine_reply = serde_json::json!({
        "msgtype": "m.text",
        "body": "> < 5 ms\n\nIs that threshold reasonable?",
        "m.relates_to": {
            "rel_type": "m.thread",
            "event_id": "$root:example.com",
            "is_falling_back": false,
            "m.in_reply_to": {"event_id": "$other:example.com"},
        },
    });
    assert_eq!(
        conversation_text(&message_event(genuine_reply, None)).as_deref(),
        Some("> < 5 ms\n\nIs that threshold reasonable?")
    );

    // People talking about configurations are not left out.
    let question = "Configuration for agent `demo` keeps failing. I tried this (using the `openai` provider):\n```yml\nmodel_id: x\n```";
    assert_eq!(
        conversation_text(&message_event(text(question), None)).as_deref(),
        Some(question)
    );
    assert_eq!(
        conversation_text(&message_event(
            text("Configuration for agent `static/openai`: what does temperature do?"),
            None
        ))
        .as_deref(),
        Some("Configuration for agent `static/openai`: what does temperature do?")
    );
}
