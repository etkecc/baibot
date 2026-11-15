use super::super::ControllerType;
use super::super::agent::AgentControllerType;
use super::super::chat_completion::ChatCompletionControllerType;
use super::super::determination::determine_text_controller;
use crate::entity::roomconfig::TextGenerationPrefixRequirementType;

fn determine(text: &str) -> ControllerType {
    determine_text_controller("!bai", text, TextGenerationPrefixRequirementType::No, false)
}

fn in_room(target: &str, command: ControllerType) -> ControllerType {
    ControllerType::InRoom(target.try_into().unwrap(), Box::new(command))
}

#[test]
fn determine_controller() {
    let cases = vec![
        ("!bai room list", ControllerType::RoomList),
        (
            "!bai in !abc:example.com agent list",
            in_room(
                "!abc:example.com",
                ControllerType::Agent(AgentControllerType::List),
            ),
        ),
        (
            "!bai in   #room:example.com\n agent list ",
            in_room(
                "#room:example.com",
                ControllerType::Agent(AgentControllerType::List),
            ),
        ),
        (
            "!bai in !abc:example.com config status",
            in_room("!abc:example.com", determine("!bai config status")),
        ),
        (
            "!bai in !abc:example.com hello",
            in_room(
                "!abc:example.com",
                ControllerType::ChatCompletion(ChatCompletionControllerType::TextCommand),
            ),
        ),
        (
            "!bai in !abc:example.com in #other:example.com agent list",
            in_room(
                "!abc:example.com",
                ControllerType::Error(crate::strings::remote::nested_not_supported().to_owned()),
            ),
        ),
        (
            "!bai in !abc:example.com",
            ControllerType::Error(crate::strings::remote::missing_command("!bai")),
        ),
        // Not a room ID or alias, so it's a regular chat completion.
        (
            "!bai in which year was Matrix created?",
            ControllerType::ChatCompletion(ChatCompletionControllerType::TextCommand),
        ),
        // A room ID needs no server part (room version 12), so this is a remote command for the room `!important`.
        // The bot replies that it is not in that room. Such chat prompts are rare enough to accept this.
        (
            "!bai in !important news",
            in_room("!important", determine("!bai news")),
        ),
        (
            "!bai information",
            ControllerType::ChatCompletion(ChatCompletionControllerType::TextCommand),
        ),
        (
            "!bai room list of mine",
            ControllerType::ChatCompletion(ChatCompletionControllerType::TextCommand),
        ),
    ];

    for (text, expected) in cases {
        assert_eq!(determine(text), expected, "{text}");
    }
}

#[test]
fn only_config_and_agent_commands_are_supported_remotely() {
    assert!(super::is_supported_remotely(&determine(
        "!bai config status"
    )));
    assert!(super::is_supported_remotely(&determine("!bai agent list")));
    assert!(!super::is_supported_remotely(&determine("!bai help")));
    assert!(!super::is_supported_remotely(&determine(
        "!bai access users"
    )));
    assert!(!super::is_supported_remotely(&determine(
        "!bai image a cat"
    )));
    assert!(!super::is_supported_remotely(&determine("!bai hello")));
    assert!(!super::is_supported_remotely(&determine(
        "!bai in !abc:example.com agent list"
    )));
    // Errors are passed through so their message is shown.
    assert!(super::is_supported_remotely(&determine(
        "!bai agent delete"
    )));
}

#[test]
fn deeply_nested_commands_are_parsed_shallowly() {
    let text = format!("!bai {}agent list", "in !abc:example.com ".repeat(10_000));

    let ControllerType::InRoom(_, command) = determine(&text) else {
        panic!("expected InRoom");
    };
    assert!(matches!(*command, ControllerType::Error(_)));
}

#[test]
fn escape_markdown() {
    assert_eq!(
        super::escape_markdown("**Room**\n- [x](y) `z`"),
        "\\*\\*Room\\*\\* - \\[x\\]\\(y\\) \\`z\\`"
    );
}

#[test]
fn room_line() {
    let room_id = <&mxlink::matrix_sdk::ruma::RoomId>::try_from("!abc:example.com").unwrap();
    let alias = <&mxlink::matrix_sdk::ruma::RoomAliasId>::try_from("#room:example.com").unwrap();

    assert_eq!(
        super::room_line("Room *1*", room_id, Some(alias), 1),
        "- **Room \\*1\\***: `!abc:example.com`, `#room:example.com` (1 member)"
    );
    assert_eq!(
        super::room_line("Room", room_id, None, 3),
        "- **Room**: `!abc:example.com` (3 members)"
    );
}
