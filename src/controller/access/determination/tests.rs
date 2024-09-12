#[test]
fn determine_controller() {
    struct TestCase {
        name: &'static str,
        input: &'static str,
        expected: super::ControllerType,
    }

    let test_cases = vec![
        TestCase {
            name: "Top-level is help",
            input: "",
            expected: super::ControllerType::Access(super::AccessControllerType::Help),
        },
        TestCase {
            name: "Anything else goes to top-level",
            input: "whatever",
            expected: super::ControllerType::Access(super::AccessControllerType::Help),
        },
        TestCase {
            name: "Users",
            input: "users",
            expected: super::ControllerType::Access(super::AccessControllerType::GetUsers),
        },
        TestCase {
            name: "Set-users",
            input: "set-users @user:example.com @bot.*:example.org",
            expected: super::ControllerType::Access(super::AccessControllerType::SetUsers(Some(
                vec![
                    "@user:example.com".to_owned(),
                    "@bot.*:example.org".to_owned(),
                ],
            ))),
        },
        TestCase {
            name: "Room-local-agent-managers",
            input: "room-local-agent-managers",
            expected: super::ControllerType::Access(
                super::AccessControllerType::GetRoomLocalAgentManagers,
            ),
        },
        TestCase {
            name: "Set-room-local-agent-managers",
            input: "set-room-local-agent-managers @user:example.com @bot.*:example.org",
            expected: super::ControllerType::Access(
                super::AccessControllerType::SetRoomLocalAgentManagers(Some(vec![
                    "@user:example.com".to_owned(),
                    "@bot.*:example.org".to_owned(),
                ])),
            ),
        },
    ];

    for test_case in test_cases {
        let result = super::determine_controller(test_case.input);
        assert_eq!(result, test_case.expected, "Test case: {}", test_case.name);
    }
}
