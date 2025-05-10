use chrono::{TimeZone, Utc};

use mxlink::matrix_sdk::ruma::OwnedUserId;

use crate::conversation::matrix::{
    MatrixMessage, MatrixMessageContent, MatrixMessageProcessingParams,
};

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

    let allowed_users = vec![parsed_regex];

    assert!(
        super::is_message_from_allowed_sender(&bot_message, &bot_user_id, Some(&allowed_users)),
        "Bot message should be allowed"
    );

    assert!(
        super::is_message_from_allowed_sender(
            &allowed_user_message,
            &bot_user_id,
            Some(&allowed_users)
        ),
        "Allowed user message should be allowed"
    );

    assert!(
        !super::is_message_from_allowed_sender(
            &unallowed_user_message,
            &bot_user_id,
            Some(&allowed_users),
        ),
        "Unallowed user message should be ignored"
    );

    assert!(
        super::is_message_from_allowed_sender(&unallowed_user_message, &bot_user_id, None,),
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

    let allowed_users = vec![parsed_regex];

    let message_processing_params_basic = super::MatrixMessageProcessingParams::new(
        bot_user_id.to_owned(),
        Some(allowed_users.clone()),
    );

    let message_processing_params_with_prefix_stripping =
        super::MatrixMessageProcessingParams::new(
            bot_user_id.to_owned(),
            Some(allowed_users.clone()),
        )
        .with_first_message_prefixes_to_strip(vec!["!bai".to_owned()]);

    let message_processing_params_with_bot_user_prefix_stripping =
        super::MatrixMessageProcessingParams::new(
            bot_user_id.to_owned(),
            Some(allowed_users.clone()),
        )
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
