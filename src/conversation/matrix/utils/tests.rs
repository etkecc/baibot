use crate::conversation::matrix::{
    MatrixMessage, MatrixMessageProcessingParams, MatrixMessageType,
};

#[test]
fn is_message_from_allowed_sender() {
    let bot_user_id = "@bot:example.com";
    let allowed_user_id = "@user.someone:example.com";
    let unallowed_user_id = "@another:example.com";

    let bot_message = MatrixMessage {
        sender_id: bot_user_id.to_owned(),
        message_type: MatrixMessageType::Text,
        message_text: "Hello!".to_owned(),
    };

    let allowed_user_message = MatrixMessage {
        sender_id: allowed_user_id.to_owned(),
        message_type: MatrixMessageType::Text,
        message_text: "Hello!".to_owned(),
    };

    let unallowed_user_message = MatrixMessage {
        sender_id: unallowed_user_id.to_owned(),
        message_type: MatrixMessageType::Text,
        message_text: "Hello!".to_owned(),
    };

    let parsed_regex = match mxidwc::parse_pattern("@user.*:example.com") {
        Ok(value) => value,
        Err(err) => {
            panic!("Error parsing regex: {}", err);
        }
    };

    let allowed_users = vec![parsed_regex];

    assert!(
        super::is_message_from_allowed_sender(&bot_message, bot_user_id, &vec![]),
        "Bot message should be allowed"
    );

    assert!(
        super::is_message_from_allowed_sender(&allowed_user_message, bot_user_id, &allowed_users),
        "Allowed user message should be allowed"
    );

    assert!(
        !super::is_message_from_allowed_sender(
            &unallowed_user_message,
            bot_user_id,
            &allowed_users
        ),
        "Unallowed user message should be ignored"
    );
}

#[tokio::test]
async fn process_matrix_messages_in_thread() {
    let bot_user_id = "@bot:example.com";
    let allowed_user_id = "@user.someone:example.com";
    let unallowed_user_id = "@another:example.com";

    let allowed_user_message = MatrixMessage {
        sender_id: allowed_user_id.to_owned(),
        message_type: MatrixMessageType::Text,
        message_text: "Hello from the user!".to_owned(),
    };

    let allowed_user_message_with_prefix = MatrixMessage {
        sender_id: allowed_user_id.to_owned(),
        message_type: MatrixMessageType::Text,
        message_text: "!bai Hello from the user!".to_owned(),
    };

    let allowed_user_message_with_prefix_no_space = MatrixMessage {
        sender_id: allowed_user_id.to_owned(),
        message_type: MatrixMessageType::Text,
        message_text: "!baiHello from the user!".to_owned(),
    };

    let allowed_user_message_with_prefix_full_width_space = MatrixMessage {
        sender_id: allowed_user_id.to_owned(),
        message_type: MatrixMessageType::Text,
        message_text: "!bai　Hello from the user!".to_owned(),
    };

    let bot_message = MatrixMessage {
        sender_id: bot_user_id.to_owned(),
        message_type: MatrixMessageType::Text,
        message_text: "Hello from the bot!".to_owned(),
    };

    let unallowed_user_message = MatrixMessage {
        sender_id: unallowed_user_id.to_owned(),
        message_type: MatrixMessageType::Text,
        message_text: "Hello from an unallowed user!".to_owned(),
    };

    let parsed_regex = match mxidwc::parse_pattern("@user.*:example.com") {
        Ok(value) => value,
        Err(err) => {
            panic!("Error parsing regex: {}", err);
        }
    };

    let allowed_users = vec![parsed_regex];

    let message_processing_params_basic =
        super::MatrixMessageProcessingParams::new(bot_user_id.to_owned(), allowed_users.clone());

    let message_processing_params_with_prefix_stripping =
        super::MatrixMessageProcessingParams::new(bot_user_id.to_owned(), allowed_users.clone())
            .with_first_message_stripped_prefixes(vec!["!bai".to_owned()]);

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
    ];

    for test_case in test_cases {
        let processed_messages = super::process_matrix_messages_in_thread(
            &test_case.messages,
            &test_case.message_processing_params,
        )
        .await;

        let processed_message_texts = processed_messages
            .iter()
            .map(|message| message.message_text.clone())
            .collect::<Vec<String>>();

        assert_eq!(
            processed_message_texts, test_case.expected_message_texts,
            "Test case {} failed",
            test_case.name,
        );
    }
}
