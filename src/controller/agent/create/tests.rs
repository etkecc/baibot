#[test]
fn agent_config_parsing_works() {
    struct TestCase {
        input: String,
        expected: Option<serde_yaml::Value>,
    }

    let provider = crate::agent::AgentProvider::OpenAI;
    let sample_config = crate::agent::default_config_for_provider(&provider);
    let sample_config_pretty_yaml = serde_yaml::to_string(&sample_config).unwrap();

    let test_cases = vec![
        // Invalid input
        TestCase {
            input: r#"Hello"#.to_owned(),
            expected: None,
        },
        // Plain text
        TestCase {
            input: sample_config_pretty_yaml.clone(),
            expected: Some(sample_config.clone()),
        },
        // Generic code block
        TestCase {
            input: format!("```\n{}```", sample_config_pretty_yaml),
            expected: Some(sample_config.clone()),
        },
        // YAML code block (yaml)
        TestCase {
            input: format!("```yaml\n{}```", sample_config_pretty_yaml),
            expected: Some(sample_config.clone()),
        },
        // YAML code block (yml)
        TestCase {
            input: format!("```yml\n{}```", sample_config_pretty_yaml),
            expected: Some(sample_config.clone()),
        },
        // JSON code block
        TestCase {
            input: format!("```json\n{}```", sample_config_pretty_yaml),
            expected: None,
        },
    ];

    for (i, test_case) in test_cases.iter().enumerate() {
        let result = super::parse_from_message_to_yaml_value(&test_case.input);
        match result {
            Ok(config) => {
                assert_eq!(
                    config,
                    test_case.expected.clone().unwrap(),
                    "Test case {} failed",
                    i
                );
            }
            Err(_) => {
                assert_eq!(test_case.expected, None, "Test case {} failed", i);
            }
        }
    }
}
