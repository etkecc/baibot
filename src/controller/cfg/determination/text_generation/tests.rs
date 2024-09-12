#[test]
fn determine_controller_other() {
    use super::ConfigTextGenerationSettingRelatedControllerType;
    use super::ControllerType;

    struct TestCase {
        name: &'static str,
        input: &'static str,
        expected: Result<ConfigTextGenerationSettingRelatedControllerType, ControllerType>,
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
fn determine_controller_context_management() {
    use super::ConfigTextGenerationSettingRelatedControllerType;
    use super::ControllerType;

    struct TestCase {
        name: &'static str,
        input: &'static str,
        expected: Result<ConfigTextGenerationSettingRelatedControllerType, ControllerType>,
    }

    let test_cases = vec![
        TestCase {
            name: "context-management-enabled getter ok",
            input: "context-management-enabled",
            expected: Ok(
                ConfigTextGenerationSettingRelatedControllerType::GetContextManagementEnabled,
            ),
        },
        TestCase {
            name: "context-management-enabled getter extra args",
            input: "context-management-enabled some values here",
            expected: Err(ControllerType::Error(
                crate::strings::cfg::configuration_getter_used_with_extra_text(
                    "context-management-enabled",
                    "some values here",
                ),
            )),
        },
        TestCase {
            name: "context-management-enabled setter",
            input: "set-context-management-enabled true",
            expected: Ok(
                ConfigTextGenerationSettingRelatedControllerType::SetContextManagementEnabled(
                    Some(true),
                ),
            ),
        },
        TestCase {
            name: "context-management-enabled setter uppercase",
            input: "set-context-management-enabled TRUE",
            expected: Ok(
                ConfigTextGenerationSettingRelatedControllerType::SetContextManagementEnabled(
                    Some(true),
                ),
            ),
        },
        TestCase {
            name: "context-management-enabled setter non-bool",
            input: "set-context-management-enabled non-Bool-Value",
            expected: Err(ControllerType::Error(
                crate::strings::cfg::configuration_value_unrecognized("non-Bool-Value"),
            )),
        },
        TestCase {
            name: "context-management-enabled unsetter",
            input: "set-context-management-enabled",
            expected: Ok(
                ConfigTextGenerationSettingRelatedControllerType::SetContextManagementEnabled(None),
            ),
        },
    ];

    for test_case in test_cases {
        let result = super::determine(test_case.input);
        assert_eq!(result, test_case.expected, "Test case: {}", test_case.name);
    }
}

#[test]
fn determine_controller_prefix_requirement_type() {
    use super::ConfigTextGenerationSettingRelatedControllerType;
    use super::ControllerType;
    use crate::entity::roomconfig::TextGenerationPrefixRequirementType;

    struct TestCase {
        name: &'static str,
        input: &'static str,
        expected: Result<ConfigTextGenerationSettingRelatedControllerType, ControllerType>,
    }

    let test_cases = vec![
        TestCase {
            name: "prefix-requirement-type getter ok",
            input: "prefix-requirement-type",
            expected: Ok(
                ConfigTextGenerationSettingRelatedControllerType::GetPrefixRequirementType,
            ),
        },
        TestCase {
            name: "prefix-requirement-type getter extra args",
            input: "prefix-requirement-type some values here",
            expected: Err(ControllerType::Error(
                crate::strings::cfg::configuration_getter_used_with_extra_text(
                    "prefix-requirement-type",
                    "some values here",
                ),
            )),
        },
        TestCase {
            name: "prefix-requirement-type setter (command_prefix)",
            input: "set-prefix-requirement-type command_prefix",
            expected: Ok(
                ConfigTextGenerationSettingRelatedControllerType::SetPrefixRequirementType(Some(
                    TextGenerationPrefixRequirementType::CommandPrefix,
                )),
            ),
        },
        TestCase {
            name: "prefix-requirement-type setter (no)",
            input: "set-prefix-requirement-type no",
            expected: Ok(
                ConfigTextGenerationSettingRelatedControllerType::SetPrefixRequirementType(Some(
                    TextGenerationPrefixRequirementType::No,
                )),
            ),
        },
        TestCase {
            name: "prefix-requirement-type setter",
            input: "set-prefix-requirement-type unknown-Value",
            expected: Err(ControllerType::Error(
                crate::strings::cfg::configuration_value_unrecognized("unknown-Value"),
            )),
        },
        TestCase {
            name: "prefix-requirement-type unsetter",
            input: "set-prefix-requirement-type",
            expected: Ok(
                ConfigTextGenerationSettingRelatedControllerType::SetPrefixRequirementType(None),
            ),
        },
    ];

    for test_case in test_cases {
        let result = super::determine(test_case.input);
        assert_eq!(result, test_case.expected, "Test case: {}", test_case.name);
    }
}

#[test]
fn determine_controller_auto_usage() {
    use super::ConfigTextGenerationSettingRelatedControllerType;
    use super::ControllerType;
    use crate::entity::roomconfig::TextGenerationAutoUsage;

    struct TestCase {
        name: &'static str,
        input: &'static str,
        expected: Result<ConfigTextGenerationSettingRelatedControllerType, ControllerType>,
    }

    let test_cases = vec![
        TestCase {
            name: "auto-usage getter ok",
            input: "auto-usage",
            expected: Ok(ConfigTextGenerationSettingRelatedControllerType::GetAutoUsage),
        },
        TestCase {
            name: "auto-usage getter extra args",
            input: "auto-usage some values here",
            expected: Err(ControllerType::Error(
                crate::strings::cfg::configuration_getter_used_with_extra_text(
                    "auto-usage",
                    "some values here",
                ),
            )),
        },
        TestCase {
            name: "auto-usage setter",
            input: "set-auto-usage only_for_voice",
            expected: Ok(
                ConfigTextGenerationSettingRelatedControllerType::SetAutoUsage(Some(
                    TextGenerationAutoUsage::OnlyForVoice,
                )),
            ),
        },
        TestCase {
            name: "auto-usage setter",
            input: "set-auto-usage unknown-Value",
            expected: Err(ControllerType::Error(
                crate::strings::cfg::configuration_value_unrecognized("unknown-Value"),
            )),
        },
        TestCase {
            name: "auto-usage unsetter",
            input: "set-auto-usage",
            expected: Ok(ConfigTextGenerationSettingRelatedControllerType::SetAutoUsage(None)),
        },
    ];

    for test_case in test_cases {
        let result = super::determine(test_case.input);
        assert_eq!(result, test_case.expected, "Test case: {}", test_case.name);
    }
}

#[test]
fn determine_controller_prompt_override() {
    use super::ConfigTextGenerationSettingRelatedControllerType;
    use super::ControllerType;

    struct TestCase {
        name: &'static str,
        input: &'static str,
        expected: Result<ConfigTextGenerationSettingRelatedControllerType, ControllerType>,
    }

    let test_cases = vec![
        TestCase {
            name: "prompt-override getter ok",
            input: "prompt-override",
            expected: Ok(ConfigTextGenerationSettingRelatedControllerType::GetPromptOverride),
        },
        TestCase {
            name: "prompt-override getter extra args",
            input: "prompt-override some values here",
            expected: Err(ControllerType::Error(
                crate::strings::cfg::configuration_getter_used_with_extra_text(
                    "prompt-override",
                    "some values here",
                ),
            )),
        },
        TestCase {
            name: "prompt-override setter with multiple words",
            input: "set-prompt-override Hello! You are a bot",
            expected: Ok(
                ConfigTextGenerationSettingRelatedControllerType::SetPromptOverride(Some(
                    "Hello! You are a bot".to_owned(),
                )),
            ),
        },
        TestCase {
            name: "prompt-override setter with multi-line",
            input: "set-prompt-override Hello!\n\nYou are a bot",
            expected: Ok(
                ConfigTextGenerationSettingRelatedControllerType::SetPromptOverride(Some(
                    "Hello!\n\nYou are a bot".to_owned(),
                )),
            ),
        },
        TestCase {
            name: "prompt-override unsetter",
            input: "set-prompt-override",
            expected: Ok(ConfigTextGenerationSettingRelatedControllerType::SetPromptOverride(None)),
        },
    ];

    for test_case in test_cases {
        let result = super::determine(test_case.input);
        assert_eq!(result, test_case.expected, "Test case: {}", test_case.name);
    }
}

#[test]
fn determine_controller_temperature_override() {
    use super::ConfigTextGenerationSettingRelatedControllerType;
    use super::ControllerType;

    struct TestCase {
        name: &'static str,
        input: &'static str,
        expected: Result<ConfigTextGenerationSettingRelatedControllerType, ControllerType>,
    }

    let test_cases = vec![
        TestCase {
            name: "temperature-override getter ok",
            input: "temperature-override",
            expected: Ok(ConfigTextGenerationSettingRelatedControllerType::GetTemperatureOverride),
        },
        TestCase {
            name: "temperature-override getter extra args",
            input: "temperature-override some values here",
            expected: Err(ControllerType::Error(
                crate::strings::cfg::configuration_getter_used_with_extra_text(
                    "temperature-override",
                    "some values here",
                ),
            )),
        },
        TestCase {
            name: "temperature-override setter",
            input: "set-temperature-override 0.5",
            expected: Ok(
                ConfigTextGenerationSettingRelatedControllerType::SetTemperatureOverride(Some(0.5)),
            ),
        },
        TestCase {
            name: "temperature-override setter",
            input: "set-temperature-override unknown-Value",
            expected: Err(ControllerType::Error(
                crate::strings::cfg::configuration_value_not_f32("unknown-Value"),
            )),
        },
        TestCase {
            name: "temperature-override unsetter",
            input: "set-temperature-override",
            expected: Ok(
                ConfigTextGenerationSettingRelatedControllerType::SetTemperatureOverride(None),
            ),
        },
    ];

    for test_case in test_cases {
        let result = super::determine(test_case.input);
        assert_eq!(result, test_case.expected, "Test case: {}", test_case.name);
    }
}
