#[test]
fn determine_text_controller() {
    use super::super::chat_completion::ChatCompletionControllerType;
    use super::ControllerType;
    use crate::controller;

    let command_prefix = "!bai";

    struct TestCase {
        name: &'static str,
        input: &'static str,
        is_mentioning_bot: bool,
        expected: ControllerType,
        // This value only matters for some of the tests.
        // We default to using the No variant for most tests where it's irrelevant.
        room_text_generation_prefix_requirement_type: super::TextGenerationPrefixRequirementType,
    }

    // We only have top-level test cases here.
    // Each submodule defines its own test cases.
    let test_cases = vec![
        TestCase {
            name: "Help",
            input: "!bai help",
            is_mentioning_bot: false,
            room_text_generation_prefix_requirement_type:
                super::TextGenerationPrefixRequirementType::No,
            expected: ControllerType::Help,
        },
        TestCase {
            name: "Prefix only leads to help",
            input: "!bai",
            is_mentioning_bot: false,
            room_text_generation_prefix_requirement_type:
                super::TextGenerationPrefixRequirementType::No,
            expected: ControllerType::Help,
        },
        TestCase {
            name: "Prefix and unknown command leads to chat completion",
            input: "!bai something-else",
            is_mentioning_bot: false,
            room_text_generation_prefix_requirement_type:
                super::TextGenerationPrefixRequirementType::No,
            expected: ControllerType::ChatCompletion(ChatCompletionControllerType::TextCommand),
        },
        TestCase {
            name: "Access top-level",
            input: "!bai access",
            is_mentioning_bot: false,
            room_text_generation_prefix_requirement_type:
                super::TextGenerationPrefixRequirementType::No,
            expected: ControllerType::Access(controller::access::AccessControllerType::Help),
        },
        TestCase {
            name: "Provider",
            input: "!bai provider",
            is_mentioning_bot: false,
            room_text_generation_prefix_requirement_type:
                super::TextGenerationPrefixRequirementType::No,
            expected: ControllerType::ProviderHelp,
        },
        TestCase {
            name: "Usage",
            input: "!bai usage",
            is_mentioning_bot: false,
            room_text_generation_prefix_requirement_type:
                super::TextGenerationPrefixRequirementType::No,
            expected: ControllerType::UsageHelp,
        },
        TestCase {
            name: "Agent top-level",
            input: "!bai agent",
            is_mentioning_bot: false,
            room_text_generation_prefix_requirement_type:
                super::TextGenerationPrefixRequirementType::No,
            expected: ControllerType::Agent(controller::agent::AgentControllerType::Help),
        },
        TestCase {
            name: "Config top-level",
            input: "!bai config",
            is_mentioning_bot: false,
            room_text_generation_prefix_requirement_type:
                super::TextGenerationPrefixRequirementType::No,
            expected: ControllerType::Config(controller::cfg::ConfigControllerType::Help),
        },
        TestCase {
            name: "Generic image command causes usage help",
            input: "!bai image Draw a cat!",
            is_mentioning_bot: false,
            room_text_generation_prefix_requirement_type:
                super::TextGenerationPrefixRequirementType::No,
            expected: ControllerType::UsageHelp,
        },
        TestCase {
            name: "Image generation",
            input: "!bai image create Draw a cat!",
            is_mentioning_bot: false,
            room_text_generation_prefix_requirement_type:
                super::TextGenerationPrefixRequirementType::No,
            expected: ControllerType::ImageGeneration("Draw a cat!".to_owned()),
        },
        TestCase {
            name: "Sticker generation",
            input: "!bai sticker A surprised cat",
            is_mentioning_bot: false,
            room_text_generation_prefix_requirement_type:
                super::TextGenerationPrefixRequirementType::No,
            expected: ControllerType::StickerGeneration("A surprised cat".to_owned()),
        },
        TestCase {
            name: "Regular text triggers completion when prefix not required",
            input: "Regular text goes here",
            is_mentioning_bot: false,
            room_text_generation_prefix_requirement_type:
                super::TextGenerationPrefixRequirementType::No,
            expected: ControllerType::ChatCompletion(ChatCompletionControllerType::TextDirect),
        },
        TestCase {
            name: "Regular text is ignored when prefix is required",
            input: "Regular text goes here",
            is_mentioning_bot: false,
            room_text_generation_prefix_requirement_type:
                super::TextGenerationPrefixRequirementType::CommandPrefix,
            expected: ControllerType::Ignore,
        },
        TestCase {
            name: "Command-prefixed text triggers completion when prefix is required",
            input: "!bai Regular text goes here",
            is_mentioning_bot: false,
            room_text_generation_prefix_requirement_type:
                super::TextGenerationPrefixRequirementType::CommandPrefix,
            expected: ControllerType::ChatCompletion(ChatCompletionControllerType::TextCommand),
        },
        TestCase {
            name: "Command-prefixed text triggers completion even when prefix is not required",
            input: "!bai Regular text goes here",
            is_mentioning_bot: false,
            room_text_generation_prefix_requirement_type:
                super::TextGenerationPrefixRequirementType::No,
            expected: ControllerType::ChatCompletion(ChatCompletionControllerType::TextCommand),
        },
        TestCase {
            name: "Regular message with bot mention triggers completion (no prefix requirement)",
            input: "Regular text goes here",
            is_mentioning_bot: true,
            room_text_generation_prefix_requirement_type:
                super::TextGenerationPrefixRequirementType::No,
            expected: ControllerType::ChatCompletion(ChatCompletionControllerType::TextMention),
        },
        // This test case is the same as the one above, just with a different prefix requirement setting.
        // We expect the same result.
        TestCase {
            name: "Regular message with bot mention triggers completion (command prefix requirement)",
            input: "Regular text goes here",
            is_mentioning_bot: true,
            room_text_generation_prefix_requirement_type:
                super::TextGenerationPrefixRequirementType::CommandPrefix,
            expected: ControllerType::ChatCompletion(ChatCompletionControllerType::TextMention),
        },
    ];

    for test_case in test_cases {
        let result = super::determine_text_controller(
            command_prefix,
            test_case.input,
            test_case.room_text_generation_prefix_requirement_type,
            test_case.is_mentioning_bot,
        );
        assert_eq!(result, test_case.expected, "Test case: {}", test_case.name);
    }
}
