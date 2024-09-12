#[test]
fn determine_controller() {
    use crate::agent::PublicIdentifier;

    struct TestCase {
        name: &'static str,
        input: &'static str,
        expected: super::ControllerType,
    }

    let command_prefix = "!bai";

    let test_cases = vec![
        TestCase {
            name: "Top-level is help",
            input: "",
            expected: super::ControllerType::Agent(super::AgentControllerType::Help),
        },
        TestCase {
            name: "Anything else goes to top-level",
            input: "whatever",
            expected: super::ControllerType::Agent(super::AgentControllerType::Help),
        },
        TestCase {
            name: "List",
            input: "list",
            expected: super::ControllerType::Agent(super::AgentControllerType::List),
        },
        TestCase {
            name: "details",
            input: "details static/agent-id",
            expected: super::ControllerType::Agent(super::AgentControllerType::Details(
                PublicIdentifier::Static("agent-id".to_owned()),
            )),
        },
        TestCase {
            name: "details with invalid agent identifier",
            input: "details agent-id",
            expected: super::ControllerType::Error(crate::strings::agent::invalid_id_generic()),
        },
        TestCase {
            name: "create-room-local no arguments",
            input: "create-room-local",
            expected: super::ControllerType::Error(
                crate::strings::agent::incorrect_creation_invocation(command_prefix),
            ),
        },
        TestCase {
            name: "create-room-local only with provider",
            input: "create-room-local openai",
            expected: super::ControllerType::Error(
                crate::strings::agent::incorrect_creation_invocation(command_prefix),
            ),
        },
        TestCase {
            name: "create-room-local correct",
            input: "create-room-local openai my-agent-id",
            expected: super::ControllerType::Agent(super::AgentControllerType::CreateRoomLocal {
                provider: "openai".to_owned(),
                agent_id: "my-agent-id".trim().to_owned(),
            }),
        },
        TestCase {
            name: "create-global extra arguments",
            input: "create-global openai my-agent-id more arguments here",
            expected: super::ControllerType::Error(
                crate::strings::agent::incorrect_creation_invocation(command_prefix),
            ),
        },
        TestCase {
            name: "create-global no arguments",
            input: "create-global",
            expected: super::ControllerType::Error(
                crate::strings::agent::incorrect_creation_invocation(command_prefix),
            ),
        },
        TestCase {
            name: "create-global only with provider",
            input: "create-global openai",
            expected: super::ControllerType::Error(
                crate::strings::agent::incorrect_creation_invocation(command_prefix),
            ),
        },
        TestCase {
            name: "create-global correct",
            input: "create-global openai my-agent-id",
            expected: super::ControllerType::Agent(super::AgentControllerType::CreateGlobal {
                provider: "openai".to_owned(),
                agent_id: "my-agent-id".trim().to_owned(),
            }),
        },
        TestCase {
            name: "create-global extra arguments",
            input: "create-global openai my-agent-id more arguments here",
            expected: super::ControllerType::Error(
                crate::strings::agent::incorrect_creation_invocation(command_prefix),
            ),
        },
        TestCase {
            name: "delete no arguments",
            input: "delete",
            expected: super::ControllerType::Error(
                crate::strings::agent::incorrect_invocation_expects_agent_id_arg(command_prefix),
            ),
        },
        TestCase {
            name: "delete too many arguments",
            input: "delete agent-id extra arguments",
            expected: super::ControllerType::Error(
                crate::strings::agent::incorrect_invocation_expects_agent_id_arg(command_prefix),
            ),
        },
        TestCase {
            name: "delete",
            input: "delete static/agent-id",
            expected: super::ControllerType::Agent(super::AgentControllerType::Delete(
                PublicIdentifier::Static("agent-id".to_owned()),
            )),
        },
        TestCase {
            name: "delete with invalid agent identifier",
            input: "delete agent-id",
            expected: super::ControllerType::Error(crate::strings::agent::invalid_id_generic()),
        },
    ];

    for test_case in test_cases {
        let result = super::determine_controller(command_prefix, test_case.input);
        assert_eq!(result, test_case.expected, "Test case: {}", test_case.name);
    }
}
