#[test]
fn determine_controller() {
    use super::super::controller_type;
    use crate::agent::{AgentPurpose, PublicIdentifier};

    struct TestCase {
        name: &'static str,
        input: &'static str,
        expected: super::ControllerType,
    }

    let test_cases = vec![
        TestCase {
            name: "Top-level is help",
            input: "",
            expected: super::ControllerType::Config(controller_type::ConfigControllerType::Help),
        },
        TestCase {
            name: "unknown commands is help",
            input: "whatever",
            expected: super::ControllerType::Config(controller_type::ConfigControllerType::Help),
        },

        TestCase {
            name: "Status",
            input: "status",
            expected: super::ControllerType::Config(controller_type::ConfigControllerType::Status),
        },


        TestCase {
            name: "per-room handler getter - catch-all",
            input: "room handler catch-all",
            expected: super::ControllerType::Config(controller_type::ConfigControllerType::SettingsRelated(
                controller_type::SettingsStorageSource::Room,
                controller_type::ConfigSettingRelatedControllerType::GetHandler(AgentPurpose::CatchAll),
            )),
        },
        TestCase {
            name: "per-room handler getter - text-generation",
            input: "room handler text-generation",
            expected: super::ControllerType::Config(controller_type::ConfigControllerType::SettingsRelated(
                controller_type::SettingsStorageSource::Room,
                controller_type::ConfigSettingRelatedControllerType::GetHandler(AgentPurpose::TextGeneration),
            )),
        },
        TestCase {
            name: "per-room handler getter - invalid purpose",
            input: "room handler invalid-purpose-here",
            expected: super::ControllerType::Error(
                crate::strings::agent::purpose_unrecognized("invalid-purpose-here").to_owned()
            ),
        },
        TestCase {
            name: "per-room handler getter - invalid purpose with spaces",
            input: "room handler invalid purpose here",
            expected: super::ControllerType::Error(
                crate::strings::agent::purpose_unrecognized("invalid purpose here").to_owned()
            ),
        },
        TestCase {
            name: "per-room handler setter - too few values",
            input: "room set-handler",
            expected: super::ControllerType::Error(
                crate::strings::cfg::configuration_invocation_incorrect_more_values_expected().to_owned()
            ),
        },
        TestCase {
            name: "per-room handler setter - catch-all",
            input: "room set-handler catch-all static/agent-id",
            expected: super::ControllerType::Config(controller_type::ConfigControllerType::SettingsRelated(
                controller_type::SettingsStorageSource::Room,
                controller_type::ConfigSettingRelatedControllerType::SetHandler(AgentPurpose::CatchAll, Some(
                    PublicIdentifier::Static("agent-id".to_owned())
                )),
            )),
        },
        TestCase {
            name: "per-room handler setter - catch-all with bare agent id",
            input: "room set-handler catch-all agent-id",
            expected: super::ControllerType::Error(
                crate::strings::agent::invalid_id_generic().to_owned()
            ),
        },
        TestCase {
            name: "per-room handler setter - catch-all unsetter",
            input: "room set-handler catch-all",
            expected: super::ControllerType::Config(controller_type::ConfigControllerType::SettingsRelated(
                controller_type::SettingsStorageSource::Room,
                controller_type::ConfigSettingRelatedControllerType::SetHandler(AgentPurpose::CatchAll, None),
            )),
        },
        TestCase {
            name: "per-room handler setter - text-generation",
            input: "room set-handler text-generation room-local/agent-id",
            expected: super::ControllerType::Config(controller_type::ConfigControllerType::SettingsRelated(
                controller_type::SettingsStorageSource::Room,
                controller_type::ConfigSettingRelatedControllerType::SetHandler(AgentPurpose::TextGeneration, Some(
                    PublicIdentifier::DynamicRoomLocal("agent-id".to_owned())
                )),
            )),
        },
        TestCase {
            name: "per-room handler setter - too many values",
            input: "room set-handler text-generation agent-id more values here",
            expected: super::ControllerType::Error(
                crate::strings::agent::invalid_id_generic().to_owned()
            ),
        },

        // We have few global handler test cases. We've exercised the per-room handlers enough.
        // These share the same code path, so we don't need to test all the permutations again.
        TestCase {
            name: "global handler getter - catch-all",
            input: "global handler catch-all",
            expected: super::ControllerType::Config(controller_type::ConfigControllerType::SettingsRelated(
                controller_type::SettingsStorageSource::Global,
                controller_type::ConfigSettingRelatedControllerType::GetHandler(AgentPurpose::CatchAll),
            )),
        },
        TestCase {
            name: "global handler setter - text-generation with global agent",
            input: "global set-handler text-generation global/agent-id",
            expected: super::ControllerType::Config(controller_type::ConfigControllerType::SettingsRelated(
                controller_type::SettingsStorageSource::Global,
                controller_type::ConfigSettingRelatedControllerType::SetHandler(AgentPurpose::TextGeneration, Some(
                    PublicIdentifier::DynamicGlobal("agent-id".to_owned())
                )),
            )),
        },
        // This test case passes, even though the handler function will subsequently reject using room-local agents for global handlers.
        TestCase {
            name: "global handler setter - text-generation with room-local agent",
            input: "global set-handler text-generation room-local/agent-id",
            expected: super::ControllerType::Config(controller_type::ConfigControllerType::SettingsRelated(
                controller_type::SettingsStorageSource::Global,
                controller_type::ConfigSettingRelatedControllerType::SetHandler(AgentPurpose::TextGeneration, Some(
                    PublicIdentifier::DynamicRoomLocal("agent-id".to_owned())
                )),
            )),
        },

        // We'll only test one handler per sub-category to ensure proper routing is done here.
        // Extensive tests for each sub-category are done in their respective modules.

        TestCase {
            name: "per-room text-generation/context-management-enabled getter",
            input: "room text-generation context-management-enabled",
            expected: super::ControllerType::Config(controller_type::ConfigControllerType::SettingsRelated(
                controller_type::SettingsStorageSource::Room,
                controller_type::ConfigSettingRelatedControllerType::TextGeneration(
                    controller_type::ConfigTextGenerationSettingRelatedControllerType::GetContextManagementEnabled,
                ),
            )),
        },
        TestCase {
            name: "global text-generation/context-management-enabled getter",
            input: "global text-generation context-management-enabled",
            expected: super::ControllerType::Config(controller_type::ConfigControllerType::SettingsRelated(
                controller_type::SettingsStorageSource::Global,
                controller_type::ConfigSettingRelatedControllerType::TextGeneration(
                    controller_type::ConfigTextGenerationSettingRelatedControllerType::GetContextManagementEnabled,
                ),
            )),
        },
        TestCase {
            name: "per-room text-to-speech/speed-override getter",
            input: "room text-to-speech speed-override",
            expected: super::ControllerType::Config(controller_type::ConfigControllerType::SettingsRelated(
                controller_type::SettingsStorageSource::Room,
                controller_type::ConfigSettingRelatedControllerType::TextToSpeech(
                    controller_type::ConfigTextToSpeechSettingRelatedControllerType::GetSpeedOverride,
                ),
            )),
        },
        TestCase {
            name: "global text-to-speech/speed-override getter",
            input: "global text-to-speech speed-override",
            expected: super::ControllerType::Config(controller_type::ConfigControllerType::SettingsRelated(
                controller_type::SettingsStorageSource::Global,
                controller_type::ConfigSettingRelatedControllerType::TextToSpeech(
                    controller_type::ConfigTextToSpeechSettingRelatedControllerType::GetSpeedOverride,
                ),
            )),
        },
        TestCase {
            name: "room speech-to-text/flow-type getter",
            input: "room speech-to-text flow-type",
            expected: super::ControllerType::Config(controller_type::ConfigControllerType::SettingsRelated(
                controller_type::SettingsStorageSource::Room,
                controller_type::ConfigSettingRelatedControllerType::SpeechToText(
                    controller_type::ConfigSpeechToTextSettingRelatedControllerType::GetFlowType,
                ),
            )),
        },
        TestCase {
            name: "global speech-to-text/flow-type getter",
            input: "global speech-to-text flow-type",
            expected: super::ControllerType::Config(controller_type::ConfigControllerType::SettingsRelated(
                controller_type::SettingsStorageSource::Global,
                controller_type::ConfigSettingRelatedControllerType::SpeechToText(
                    controller_type::ConfigSpeechToTextSettingRelatedControllerType::GetFlowType,
                ),
            )),
        },
    ];

    for test_case in test_cases {
        let result = super::determine_controller(test_case.input);
        assert_eq!(result, test_case.expected, "Test case: {}", test_case.name);
    }
}
