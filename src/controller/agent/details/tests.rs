#[test]
fn redact_secrets() {
    let mut config: serde_yaml_ng::Value = serde_yaml_ng::from_str(
        r#"
base_url: https://api.example.com/v1
api_key: sk-secret
text_generation:
  model_id: some-model
  max_response_tokens: 1000
nested:
  - api_key: another-secret
"#,
    )
    .unwrap();

    super::redact_secrets(&mut config);

    let expected: serde_yaml_ng::Value = serde_yaml_ng::from_str(
        r#"
base_url: https://api.example.com/v1
api_key: REDACTED
text_generation:
  model_id: some-model
  max_response_tokens: 1000
nested:
  - api_key: REDACTED
"#,
    )
    .unwrap();

    assert_eq!(config, expected);
}

#[test]
fn redact_secrets_keeps_missing_api_key_visible() {
    let mut config: serde_yaml_ng::Value = serde_yaml_ng::from_str("api_key: null").unwrap();

    super::redact_secrets(&mut config);

    assert_eq!(
        config,
        serde_yaml_ng::from_str::<serde_yaml_ng::Value>("api_key: null").unwrap()
    );
}
