#[test]
fn determine_controller_other() {
    use super::ConfigSpeechToTextSettingRelatedControllerType;
    use super::ControllerType;

    struct TestCase {
        name: &'static str,
        input: &'static str,
        expected: Result<ConfigSpeechToTextSettingRelatedControllerType, ControllerType>,
    }

    let test_cases = vec![TestCase {
        name: "Unknown",
        input: "whatever",
        expected: Err(ControllerType::Unknown),
    }];

    for test_case in test_cases {
        let result = super::determine(test_case.input);
        assert_eq!(result, test_case.expected, "Test case: {}", test_case.name);
    }
}

#[test]
fn determine_controller_flow_type() {
    use super::ConfigSpeechToTextSettingRelatedControllerType;
    use super::ControllerType;
    use crate::entity::roomconfig::SpeechToTextFlowType;

    struct TestCase {
        name: &'static str,
        input: &'static str,
        expected: Result<ConfigSpeechToTextSettingRelatedControllerType, ControllerType>,
    }

    let test_cases = vec![
        TestCase {
            name: "flow-type getter ok",
            input: "flow-type",
            expected: Ok(ConfigSpeechToTextSettingRelatedControllerType::GetFlowType),
        },
        TestCase {
            name: "flow-type getter extra args",
            input: "flow-type some values here",
            expected: Err(ControllerType::Error(
                crate::strings::cfg::configuration_getter_used_with_extra_text(
                    "flow-type",
                    "some values here",
                ),
            )),
        },
        TestCase {
            name: "flow-type setter",
            input: "set-flow-type transcribe_and_generate_text",
            expected: Ok(ConfigSpeechToTextSettingRelatedControllerType::SetFlowType(
                Some(SpeechToTextFlowType::TranscribeAndGenerateText),
            )),
        },
        TestCase {
            name: "flow-type setter",
            input: "set-flow-type unknown-Value",
            expected: Err(ControllerType::Error(
                crate::strings::cfg::configuration_value_unrecognized("unknown-Value"),
            )),
        },
        TestCase {
            name: "flow-type unsetter",
            input: "set-flow-type",
            expected: Ok(ConfigSpeechToTextSettingRelatedControllerType::SetFlowType(
                None,
            )),
        },
    ];

    for test_case in test_cases {
        let result = super::determine(test_case.input);
        assert_eq!(result, test_case.expected, "Test case: {}", test_case.name);
    }
}

#[test]
fn determine_controller_language() {
    use super::ConfigSpeechToTextSettingRelatedControllerType;
    use super::ControllerType;

    struct TestCase {
        name: &'static str,
        input: &'static str,
        expected: Result<ConfigSpeechToTextSettingRelatedControllerType, ControllerType>,
    }

    let test_cases = vec![
        TestCase {
            name: "language getter ok",
            input: "language",
            expected: Ok(ConfigSpeechToTextSettingRelatedControllerType::GetLanguage),
        },
        TestCase {
            name: "language getter extra args",
            input: "language some values here",
            expected: Err(ControllerType::Error(
                crate::strings::cfg::configuration_getter_used_with_extra_text(
                    "language",
                    "some values here",
                ),
            )),
        },
        TestCase {
            name: "language setter 2-letter code (ja)",
            input: "set-language ja",
            expected: Ok(ConfigSpeechToTextSettingRelatedControllerType::SetLanguage(
                Some("ja".to_owned()),
            )),
        },
        // OpenAI does not support 3-letter codes, so we won't be allowing it either
        TestCase {
            name: "language setter 3-letter code (jpn) fails",
            input: "set-language jpn",
            expected: Err(ControllerType::Error(
                crate::strings::speech_to_text::language_code_invalid("jpn"),
            )),
        },
        TestCase {
            name: "language unsetter",
            input: "set-language",
            expected: Ok(ConfigSpeechToTextSettingRelatedControllerType::SetLanguage(
                None,
            )),
        },
    ];

    for test_case in test_cases {
        let result = super::determine(test_case.input);
        assert_eq!(result, test_case.expected, "Test case: {}", test_case.name);
    }
}
