use super::{AccessControllerType, NewRule, RuleCommandScope, can_delegate, determine_controller};
use crate::authorization::{Permission, RuleEffect};
use crate::controller::ControllerType;

fn determine(text: &str) -> ControllerType {
    determine_controller("!bai", text)
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| value.to_string()).collect()
}

#[test]
fn determines_commands() {
    let cases = vec![
        ("", ControllerType::Access(AccessControllerType::Help)),
        (
            "whatever",
            ControllerType::Access(AccessControllerType::Help),
        ),
        ("list", ControllerType::Access(AccessControllerType::List)),
        (
            "explain",
            ControllerType::Access(AccessControllerType::Explain(None)),
        ),
        (
            "explain @someone:example.com",
            ControllerType::Access(AccessControllerType::Explain(Some(
                "@someone:example.com".to_owned(),
            ))),
        ),
        (
            "room allow speech-to-text,text-to-speech @*:bridge.example.com @a:example.com",
            ControllerType::Access(AccessControllerType::AddRule(
                RuleCommandScope::Room,
                NewRule {
                    effect: RuleEffect::Allow,
                    permissions: strings(&["speech-to-text", "text-to-speech"]),
                    users: strings(&["@*:bridge.example.com", "@a:example.com"]),
                    min_power_level: None,
                },
            )),
        ),
        (
            "room allow speech-to-text, text-to-speech @*:bridge.example.com,@a:example.com",
            ControllerType::Access(AccessControllerType::AddRule(
                RuleCommandScope::Room,
                NewRule {
                    effect: RuleEffect::Allow,
                    permissions: strings(&["speech-to-text", "text-to-speech"]),
                    users: strings(&["@*:bridge.example.com", "@a:example.com"]),
                    min_power_level: None,
                },
            )),
        ),
        (
            "global deny `use`, `invite` for `@a:example.com`, `@a:example.com`",
            ControllerType::Access(AccessControllerType::AddRule(
                RuleCommandScope::Global,
                NewRule {
                    effect: RuleEffect::Deny,
                    permissions: strings(&["use", "invite"]),
                    users: strings(&["@a:example.com"]),
                    min_power_level: None,
                },
            )),
        ),
        (
            "global allow manage-config @*:example.com min-power-level=50",
            ControllerType::Access(AccessControllerType::AddRule(
                RuleCommandScope::Global,
                NewRule {
                    effect: RuleEffect::Allow,
                    permissions: strings(&["manage-config"]),
                    users: strings(&["@*:example.com"]),
                    min_power_level: Some(50),
                },
            )),
        ),
        (
            "global deny use,invite @spammer:example.com",
            ControllerType::Access(AccessControllerType::AddRule(
                RuleCommandScope::Global,
                NewRule {
                    effect: RuleEffect::Deny,
                    permissions: strings(&["use", "invite"]),
                    users: strings(&["@spammer:example.com"]),
                    min_power_level: None,
                },
            )),
        ),
        (
            "room allow use,use,speech-to-text @a:example.com",
            ControllerType::Access(AccessControllerType::AddRule(
                RuleCommandScope::Room,
                NewRule {
                    effect: RuleEffect::Allow,
                    permissions: strings(&["use", "speech-to-text"]),
                    users: strings(&["@a:example.com"]),
                    min_power_level: None,
                },
            )),
        ),
        (
            "room remove k3vq",
            ControllerType::Access(AccessControllerType::RemoveRule(
                RuleCommandScope::Room,
                "k3vq".to_owned(),
            )),
        ),
        (
            "global remove `users`",
            ControllerType::Access(AccessControllerType::RemoveRule(
                RuleCommandScope::Global,
                "users".to_owned(),
            )),
        ),
    ];

    for (input, expected) in cases {
        assert_eq!(determine(input), expected, "{input}");
    }
}

#[test]
fn rejects_invalid_rule_commands() {
    for input in [
        "room",
        "room allow",
        "room allow text-generation",
        "room allow unknown-permission @a:example.com",
        "room allow invite @a:example.com",
        "room allow text-generation not-a-pattern",
        "room allow text-generation @a:example.com min-power-level=high",
        "room remove",
        "room remove fi*st",
        "room grant text-generation @a:example.com",
        "global allow administer @a:example.com",
        "global allow invite @*:example.com min-power-level=50",
        "room allow text-generation @a:example.com min-power-level=1 min-power-level=2",
        "explain @a:example.com @b:example.com",
    ] {
        assert!(
            matches!(determine(input), ControllerType::Error(_)),
            "{input}"
        );
    }
}

#[test]
fn legacy_commands_point_to_rules() {
    for input in [
        "users",
        "set-users @*:example.com",
        "room-local-agent-managers",
        "set-room-local-agent-managers @a:example.com",
    ] {
        assert!(
            matches!(determine(input), ControllerType::Error(_)),
            "{input}"
        );
    }
}

#[test]
fn required_permissions() {
    let rule = || NewRule {
        effect: RuleEffect::Allow,
        permissions: strings(&["use"]),
        users: strings(&["@a:example.com"]),
        min_power_level: None,
    };

    let cases = vec![
        (AccessControllerType::Help, None),
        (AccessControllerType::Explain(None), None),
        (
            AccessControllerType::Explain(Some("@a:example.com".to_owned())),
            Some(Permission::ManageAccess),
        ),
        (AccessControllerType::List, Some(Permission::ManageAccess)),
        (
            AccessControllerType::AddRule(RuleCommandScope::Room, rule()),
            Some(Permission::ManageAccess),
        ),
        (
            AccessControllerType::RemoveRule(RuleCommandScope::Room, "a".to_owned()),
            Some(Permission::ManageAccess),
        ),
        (
            AccessControllerType::AddRule(RuleCommandScope::Global, rule()),
            Some(Permission::Administer),
        ),
        (
            AccessControllerType::RemoveRule(RuleCommandScope::Global, "a".to_owned()),
            Some(Permission::Administer),
        ),
    ];

    for (controller_type, expected) in cases {
        assert_eq!(
            controller_type.required_permission(),
            expected,
            "{controller_type:?}"
        );
    }
}

#[test]
fn delegation_is_limited_to_held_features() {
    let holds_text_and_speech = |permission: Permission| {
        matches!(
            permission,
            Permission::TextGeneration | Permission::SpeechToText | Permission::ManageAccess
        )
    };

    assert!(can_delegate(
        &strings(&["speech-to-text"]),
        holds_text_and_speech
    ));
    assert!(can_delegate(
        &strings(&["text-generation", "speech-to-text"]),
        holds_text_and_speech
    ));
    // Features they don't hold.
    assert!(!can_delegate(
        &strings(&["image-generation"]),
        holds_text_and_speech
    ));
    assert!(!can_delegate(&strings(&["use"]), holds_text_and_speech));
    // Management permissions, even ones they hold.
    assert!(!can_delegate(
        &strings(&["manage-access"]),
        holds_text_and_speech
    ));
    // Unknown names.
    assert!(!can_delegate(&strings(&["video"]), holds_text_and_speech));

    let holds_all_features = |permission: Permission| permission.is_feature();
    assert!(can_delegate(&strings(&["use"]), holds_all_features));
}
