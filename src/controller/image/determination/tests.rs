#[test]
fn determine_controller() {
    struct TestCase {
        name: &'static str,
        input: &'static str,
        expected: super::ControllerType,
    }

    let test_cases = vec![
        TestCase {
            name: "Top-level is usage help",
            input: "",
            expected: super::ControllerType::UsageHelp,
        },
        TestCase {
            name: "Top-level with some text is usage help",
            input: "Some text",
            expected: super::ControllerType::UsageHelp,
        },
        TestCase {
            name: "Image generation triggered by create prefix",
            input: "create Some prompt",
            expected: super::ControllerType::ImageGeneration("Some prompt".to_owned()),
        },

        TestCase {
            name: "Image edit triggered by edit prefix",
            input: "edit Turn this into an anime-style image",
            expected: super::ControllerType::ImageEdit("Turn this into an anime-style image".to_owned()),
        },
    ];

    for test_case in test_cases {
        let result = super::determine_controller(test_case.input);
        assert_eq!(result, test_case.expected, "Test case: {}", test_case.name);
    }
}
