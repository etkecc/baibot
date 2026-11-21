use super::{Avatar, ConfigRoom, ConfigUser, ConfigUserAuth, ConfigUserEncryption};
use crate::entity::cfg::env;

fn base_user() -> ConfigUser {
    ConfigUser {
        mxid_localpart: "baibot".to_owned(),
        password: None,
        access_token: None,
        device_id: None,
        name: "baibot".to_owned(),
        encryption: ConfigUserEncryption {
            recovery_passphrase: None,
            recovery_reset_allowed: false,
        },
        avatar: Avatar::Default,
    }
}

#[test]
fn auth_config_uses_password_mode() {
    let mut user = base_user();
    user.password = Some("secret".to_owned());

    let auth = user
        .auth_config("example.com")
        .expect("password auth should be valid");

    match auth {
        ConfigUserAuth::UserPassword { username, password } => {
            assert_eq!(username, "baibot");
            assert_eq!(password, "secret");
        }
        ConfigUserAuth::AccessToken { .. } => {
            panic!("expected password auth mode");
        }
    }
}

#[test]
fn auth_config_uses_access_token_mode() {
    let mut user = base_user();
    user.access_token = Some("token123".to_owned());
    user.device_id = Some("DEVICE1".to_owned());

    let auth = user
        .auth_config("example.com")
        .expect("access token auth should be valid");

    match auth {
        ConfigUserAuth::AccessToken {
            user_id,
            device_id,
            access_token,
        } => {
            assert_eq!(user_id.as_str(), "@baibot:example.com");
            assert_eq!(device_id.as_str(), "DEVICE1");
            assert_eq!(access_token, "token123");
        }
        ConfigUserAuth::UserPassword { .. } => {
            panic!("expected access token auth mode");
        }
    }
}

#[test]
fn auth_config_rejects_both_auth_methods() {
    let mut user = base_user();
    user.password = Some("secret".to_owned());
    user.access_token = Some("token123".to_owned());
    user.device_id = Some("DEVICE1".to_owned());

    let err = user
        .auth_config("example.com")
        .expect_err("both auth methods should be rejected");

    assert!(
        err.to_string()
            .contains("exactly one authentication method")
    );
}

#[test]
fn auth_config_rejects_missing_auth() {
    let user = base_user();

    let err = user
        .auth_config("example.com")
        .expect_err("missing auth should be rejected");

    assert!(err.to_string().contains("Set one authentication method"));
}

#[test]
fn auth_config_rejects_access_token_without_device_id() {
    let mut user = base_user();
    user.access_token = Some("token123".to_owned());

    let err = user
        .auth_config("example.com")
        .expect_err("access token mode without device_id should be rejected");

    assert!(err.to_string().contains(env::BAIBOT_USER_DEVICE_ID));
}

#[test]
fn auth_config_treats_empty_strings_as_unset() {
    let mut user = base_user();
    user.password = Some(String::new());
    user.access_token = Some(String::new());
    user.device_id = Some(String::new());

    let err = user
        .auth_config("example.com")
        .expect_err("empty auth values should be treated as unset");

    assert!(err.to_string().contains("Set one authentication method"));
}

#[test]
fn room_catch_up_tolerance_defaults_to_300_seconds() {
    let room: ConfigRoom = serde_yaml_ng::from_str("post_join_self_introduction_enabled: true")
        .expect("room config without the key should parse");

    assert_eq!(room.catch_up_tolerance_seconds, 300);
    assert_eq!(
        room.catch_up_tolerance(),
        std::time::Duration::from_secs(300)
    );
    assert_eq!(ConfigRoom::default().catch_up_tolerance_seconds, 300);
}

#[test]
fn room_catch_up_tolerance_can_be_configured() {
    let room: ConfigRoom = serde_yaml_ng::from_str("catch_up_tolerance_seconds: 86400")
        .expect("room config with the key should parse");

    assert_eq!(
        room.catch_up_tolerance(),
        std::time::Duration::from_secs(86_400)
    );
    assert!(room.post_join_self_introduction_enabled);
}

#[test]
fn room_catch_up_tolerance_env_var_name() {
    assert_eq!(
        env::BAIBOT_ROOM_CATCH_UP_TOLERANCE_SECONDS,
        "BAIBOT_ROOM_CATCH_UP_TOLERANCE_SECONDS"
    );
}

fn initial_global_config(yaml: &str) -> super::ConfigInitialGlobalConfig {
    serde_yaml_ng::from_str(yaml).expect("initial global config should parse")
}

#[test]
fn initial_access_rules_become_global_rules() {
    use crate::authorization::RuleEffect;
    use crate::authorization::rule::is_valid_rule_id;
    use crate::entity::globalconfig::GlobalConfig;

    let config = initial_global_config(
        r#"
access_rules:
  - effect: allow
    permissions: [use, invite]
    users: ["@*:example.com"]
  - effect: deny
    permissions: [image-generation]
    users: ["@*:bridge.example.com"]
  - effect: allow
    permissions: [manage-config]
    users: ["@*:example.com"]
    min_power_level: 50
"#,
    );

    config.clone().validate().expect("config should be valid");

    let global_config: GlobalConfig = config.try_into().unwrap();
    assert_eq!(global_config.access.user_patterns, None);

    let rules = global_config.access.rules.unwrap();
    assert_eq!(rules.len(), 3);

    assert_eq!(rules[0].effect, RuleEffect::Allow);
    assert_eq!(rules[0].permissions, vec!["use", "invite"]);
    assert_eq!(rules[0].users, vec!["@*:example.com"]);
    assert_eq!(rules[0].min_power_level, None);

    assert_eq!(rules[1].effect, RuleEffect::Deny);
    assert_eq!(rules[2].min_power_level, Some(50));

    for rule in &rules {
        assert!(is_valid_rule_id(&rule.id), "{}", rule.id);
    }
    assert_ne!(rules[0].id, rules[1].id);
    assert_ne!(rules[1].id, rules[2].id);
    assert_ne!(rules[0].id, rules[2].id);
}

#[test]
fn missing_initial_access_rules_leave_only_administrators() {
    use crate::entity::globalconfig::GlobalConfig;

    let config = initial_global_config("handler: {}");
    config.clone().validate().expect("config should be valid");

    let global_config: GlobalConfig = config.try_into().unwrap();
    assert_eq!(global_config.access.rules, Some(vec![]));
}

#[test]
fn removed_initial_user_patterns_are_rejected() {
    for yaml in [
        "user_patterns: ['@*:example.com']",
        "user_patterns: []",
        "user_patterns: null",
        "user_patterns:",
    ] {
        let config = initial_global_config(yaml);
        let err = config.validate().expect_err(yaml).to_string();
        assert!(err.contains("access_rules"), "{err}");
    }
}

#[test]
fn invalid_initial_access_rules_are_rejected() {
    let cases = [
        // Unknown permission
        "[{effect: allow, permissions: [administer], users: ['@*:example.com']}]",
        "[{effect: allow, permissions: [usee], users: ['@*:example.com']}]",
        // No permissions or users
        "[{effect: allow, permissions: [], users: ['@*:example.com']}]",
        "[{effect: allow, permissions: [use], users: []}]",
        // Bad pattern
        "[{effect: allow, permissions: [use], users: ['example.com']}]",
        // invite with a power level
        "[{effect: allow, permissions: [invite], users: ['@*:example.com'], min_power_level: 50}]",
        // Power level out of range
        "[{effect: allow, permissions: [use], users: ['@*:example.com'], min_power_level: 9007199254740992}]",
    ];

    for rules in cases {
        let config = initial_global_config(&format!("access_rules: {rules}"));
        assert!(config.validate().is_err(), "{rules}");
    }

    let unparsable = [
        // Unknown effect
        "[{effect: maybe, permissions: [use], users: ['@*:example.com']}]",
        // Unknown key (a typo would otherwise drop the power-level condition)
        "[{effect: allow, permissions: [use], users: ['@*:example.com'], min_powerlevel: 50}]",
        // Missing key
        "[{permissions: [use], users: ['@*:example.com']}]",
    ];

    for rules in unparsable {
        let result: Result<super::ConfigInitialGlobalConfig, _> =
            serde_yaml_ng::from_str(&format!("access_rules: {rules}"));
        assert!(result.is_err(), "{rules}");
    }
}
