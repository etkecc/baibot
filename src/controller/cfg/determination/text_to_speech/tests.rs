#[test]
fn determine_controller_other() {
    use super::ConfigTextToSpeechSettingRelatedControllerType;
    use super::ControllerType;

    struct TestCase {
        name: &'static str,
        input: &'static str,
        expected: Result<ConfigTextToSpeechSettingRelatedControllerType, ControllerType>,
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
fn determine_controller_bot_msgs_flow_type() {
    use super::ConfigTextToSpeechSettingRelatedControllerType;
    use super::ControllerType;
    use crate::entity::roomconfig::TextToSpeechBotMessagesFlowType;

    struct TestCase {
        name: &'static str,
        input: &'static str,
        expected: Result<ConfigTextToSpeechSettingRelatedControllerType, ControllerType>,
    }

    let test_cases = vec![
        TestCase {
            name: "bot-msgs-flow-type getter ok",
            input: "bot-msgs-flow-type",
            expected: Ok(ConfigTextToSpeechSettingRelatedControllerType::GetBotMessagesFlowType),
        },
        TestCase {
            name: "bot-msgs-flow-type getter extra args",
            input: "bot-msgs-flow-type some values here",
            expected: Err(ControllerType::Error(
                crate::strings::cfg::configuration_getter_used_with_extra_text(
                    "bot-msgs-flow-type",
                    "some values here",
                ),
            )),
        },
        TestCase {
            name: "bot-msgs-flow-type setter",
            input: "set-bot-msgs-flow-type only_for_voice",
            expected: Ok(
                ConfigTextToSpeechSettingRelatedControllerType::SetBotMessagesFlowType(Some(
                    TextToSpeechBotMessagesFlowType::OnlyForVoice,
                )),
            ),
        },
        TestCase {
            name: "bot-msgs-flow-type setter",
            input: "set-bot-msgs-flow-type unknown-Value",
            expected: Err(ControllerType::Error(
                crate::strings::cfg::configuration_value_unrecognized("unknown-Value"),
            )),
        },
        TestCase {
            name: "bot-msgs-flow-type unsetter",
            input: "set-bot-msgs-flow-type",
            expected: Ok(
                ConfigTextToSpeechSettingRelatedControllerType::SetBotMessagesFlowType(None),
            ),
        },
    ];

    for test_case in test_cases {
        let result = super::determine(test_case.input);
        assert_eq!(result, test_case.expected, "Test case: {}", test_case.name);
    }
}

#[test]
fn determine_controller_user_msgs_flow_type() {
    use super::ConfigTextToSpeechSettingRelatedControllerType;
    use super::ControllerType;
    use crate::entity::roomconfig::TextToSpeechUserMessagesFlowType;

    struct TestCase {
        name: &'static str,
        input: &'static str,
        expected: Result<ConfigTextToSpeechSettingRelatedControllerType, ControllerType>,
    }

    let test_cases = vec![
        TestCase {
            name: "user-msgs-flow-type getter ok",
            input: "user-msgs-flow-type",
            expected: Ok(ConfigTextToSpeechSettingRelatedControllerType::GetUserMessagesFlowType),
        },
        TestCase {
            name: "user-msgs-flow-type getter extra args",
            input: "user-msgs-flow-type some values here",
            expected: Err(ControllerType::Error(
                crate::strings::cfg::configuration_getter_used_with_extra_text(
                    "user-msgs-flow-type",
                    "some values here",
                ),
            )),
        },
        TestCase {
            name: "user-msgs-flow-type setter",
            input: "set-user-msgs-flow-type on_demand",
            expected: Ok(
                ConfigTextToSpeechSettingRelatedControllerType::SetUserMessagesFlowType(Some(
                    TextToSpeechUserMessagesFlowType::OnDemand,
                )),
            ),
        },
        TestCase {
            name: "user-msgs-flow-type setter",
            input: "set-user-msgs-flow-type unknown-Value",
            expected: Err(ControllerType::Error(
                crate::strings::cfg::configuration_value_unrecognized("unknown-Value"),
            )),
        },
        TestCase {
            name: "user-msgs-flow-type unsetter",
            input: "set-user-msgs-flow-type",
            expected: Ok(
                ConfigTextToSpeechSettingRelatedControllerType::SetUserMessagesFlowType(None),
            ),
        },
    ];

    for test_case in test_cases {
        let result = super::determine(test_case.input);
        assert_eq!(result, test_case.expected, "Test case: {}", test_case.name);
    }
}

#[test]
fn determine_controller_speed_override() {
    use super::ConfigTextToSpeechSettingRelatedControllerType;
    use super::ControllerType;

    struct TestCase {
        name: &'static str,
        input: &'static str,
        expected: Result<ConfigTextToSpeechSettingRelatedControllerType, ControllerType>,
    }

    let test_cases = vec![
        TestCase {
            name: "speed-override getter ok",
            input: "speed-override",
            expected: Ok(ConfigTextToSpeechSettingRelatedControllerType::GetSpeedOverride),
        },
        TestCase {
            name: "speed-override getter extra args",
            input: "speed-override some values here",
            expected: Err(ControllerType::Error(
                crate::strings::cfg::configuration_getter_used_with_extra_text(
                    "speed-override",
                    "some values here",
                ),
            )),
        },
        TestCase {
            name: "speed-override setter",
            input: "set-speed-override 0.5",
            expected: Ok(
                ConfigTextToSpeechSettingRelatedControllerType::SetSpeedOverride(Some(0.5)),
            ),
        },
        TestCase {
            name: "speed-override setter",
            input: "set-speed-override unknown-Value",
            expected: Err(ControllerType::Error(
                crate::strings::cfg::configuration_value_not_f32("unknown-Value"),
            )),
        },
        TestCase {
            name: "speed-override unsetter",
            input: "set-speed-override",
            expected: Ok(ConfigTextToSpeechSettingRelatedControllerType::SetSpeedOverride(None)),
        },
    ];

    for test_case in test_cases {
        let result = super::determine(test_case.input);
        assert_eq!(result, test_case.expected, "Test case: {}", test_case.name);
    }
}

#[test]
fn determine_controller_voice_override() {
    use super::ConfigTextToSpeechSettingRelatedControllerType;
    use super::ControllerType;

    struct TestCase {
        name: &'static str,
        input: &'static str,
        expected: Result<ConfigTextToSpeechSettingRelatedControllerType, ControllerType>,
    }

    let test_cases = vec![
        TestCase {
            name: "voice-override getter ok",
            input: "voice-override",
            expected: Ok(ConfigTextToSpeechSettingRelatedControllerType::GetVoiceOverride),
        },
        TestCase {
            name: "voice-override getter extra args",
            input: "voice-override some values here",
            expected: Err(ControllerType::Error(
                crate::strings::cfg::configuration_getter_used_with_extra_text(
                    "voice-override",
                    "some values here",
                ),
            )),
        },
        TestCase {
            name: "voice-override setter",
            input: "set-voice-override alex",
            expected: Ok(
                ConfigTextToSpeechSettingRelatedControllerType::SetVoiceOverride(Some(
                    "alex".to_owned(),
                )),
            ),
        },
        TestCase {
            name: "voice-override setter preserves case",
            input: "set-voice-override Alex",
            expected: Ok(
                ConfigTextToSpeechSettingRelatedControllerType::SetVoiceOverride(Some(
                    "Alex".to_owned(),
                )),
            ),
        },
        TestCase {
            name: "voice-override unsetter",
            input: "set-voice-override",
            expected: Ok(ConfigTextToSpeechSettingRelatedControllerType::SetVoiceOverride(None)),
        },
    ];

    for test_case in test_cases {
        let result = super::determine(test_case.input);
        assert_eq!(result, test_case.expected, "Test case: {}", test_case.name);
    }
}
