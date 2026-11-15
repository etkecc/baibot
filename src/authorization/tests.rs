use mxlink::matrix_sdk::ruma::events::room::power_levels::{
    RoomPowerLevels, RoomPowerLevelsEventContent,
};
use mxlink::matrix_sdk::ruma::room_version_rules::AuthorizationRules;
use mxlink::matrix_sdk::ruma::{Int, OwnedUserId, UserId};

use super::rule::{LEGACY_MANAGE_CONFIG_MIN_POWER_LEVEL, USE};
use super::{AccessRule, Authorizer, Permission, RuleEffect};
use crate::entity::globalconfig::GlobalConfig;

const ALL_PERMISSIONS: [Permission; 9] = [
    Permission::TextGeneration,
    Permission::SpeechToText,
    Permission::TextToSpeech,
    Permission::ImageGeneration,
    Permission::Invite,
    Permission::ManageConfig,
    Permission::ManageAgents,
    Permission::ManageAccess,
    Permission::Administer,
];

fn user(user_id: &str) -> OwnedUserId {
    UserId::parse(user_id).unwrap()
}

fn patterns(patterns: &[&str]) -> Option<Vec<String>> {
    Some(patterns.iter().map(|p| p.to_string()).collect())
}

fn rule(
    id: &str,
    effect: RuleEffect,
    permissions: &[&str],
    users: &[&str],
    min_power_level: Option<i64>,
) -> AccessRule {
    AccessRule {
        id: id.to_owned(),
        effect,
        permissions: permissions.iter().map(|p| p.to_string()).collect(),
        users: users.iter().map(|u| u.to_string()).collect(),
        min_power_level,
    }
}

fn legacy_global_config(
    user_patterns: Option<Vec<String>>,
    room_local_agent_manager_patterns: Option<Vec<String>>,
) -> GlobalConfig {
    // Like a configuration stored by a version without access rules.
    let mut global_config = GlobalConfig::default();
    global_config.access.user_patterns = user_patterns;
    global_config.access.room_local_agent_manager_patterns = room_local_agent_manager_patterns;
    global_config
}

fn global_config_with_rules(rules: Vec<AccessRule>) -> GlobalConfig {
    let mut global_config = GlobalConfig::new(None);
    global_config.access.rules = Some(rules);
    global_config
}

fn without_ids(rules: Vec<AccessRule>) -> Vec<AccessRule> {
    rules
        .into_iter()
        .map(|rule| AccessRule {
            id: String::new(),
            ..rule
        })
        .collect()
}

fn admin_regexes() -> Vec<regex::Regex> {
    mxidwc::parse_patterns_vector(&["@admin:example.com".to_owned()]).unwrap()
}

fn power_levels(users: &[(&str, i64)]) -> RoomPowerLevels {
    let mut content = RoomPowerLevelsEventContent::new(&AuthorizationRules::V1);
    for (user_id, level) in users {
        content
            .users
            .insert(user(user_id), Int::new(*level).unwrap());
    }
    RoomPowerLevels::new(content.into(), &AuthorizationRules::V1, vec![])
}

fn allowed_permissions(authorizer: &Authorizer, user_id: &str) -> Vec<Permission> {
    ALL_PERMISSIONS
        .into_iter()
        .filter(|permission| authorizer.allows(&user(user_id), *permission))
        .collect()
}

#[test]
fn permission_names_round_trip() {
    for permission in ALL_PERMISSIONS {
        let parsed = Permission::from_name(permission.name());
        if permission == Permission::Administer {
            assert_eq!(parsed, None);
        } else {
            assert_eq!(parsed, Some(permission));
        }
    }
    assert_eq!(Permission::from_name(USE), None);
}

#[test]
fn admins_have_every_permission() {
    let global_config = global_config_with_rules(vec![rule(
        "r1",
        RuleEffect::Deny,
        &[USE, "manage-config"],
        &["@*:example.com"],
        None,
    )]);
    let authorizer = Authorizer::new(&admin_regexes(), &global_config);

    assert!(authorizer.is_admin(&user("@admin:example.com")));
    assert_eq!(
        allowed_permissions(&authorizer, "@admin:example.com"),
        ALL_PERMISSIONS
    );
    assert!(!authorizer.denies(&user("@admin:example.com"), Permission::TextGeneration));
}

#[test]
fn nobody_but_admins_without_rules() {
    let authorizer = Authorizer::new(&admin_regexes(), &global_config_with_rules(vec![]));

    assert_eq!(allowed_permissions(&authorizer, "@someone:example.com"), []);
    assert!(!authorizer.allows_anything_in_room(&user("@someone:example.com")));
}

#[test]
fn use_covers_features_only() {
    let global_config = global_config_with_rules(vec![rule(
        "r1",
        RuleEffect::Allow,
        &[USE],
        &["@*:example.com"],
        None,
    )]);
    let authorizer = Authorizer::new(&admin_regexes(), &global_config);

    assert_eq!(
        allowed_permissions(&authorizer, "@someone:example.com"),
        Permission::FEATURES
    );
}

#[test]
fn deny_wins_over_allow_across_scopes() {
    let global_config = global_config_with_rules(vec![
        rule("r1", RuleEffect::Allow, &[USE], &["@*:example.com"], None),
        rule(
            "r2",
            RuleEffect::Deny,
            &["image-generation"],
            &["@bob:example.com"],
            None,
        ),
    ]);
    let room_rules = vec![rule(
        "r1",
        RuleEffect::Allow,
        &["image-generation"],
        &["@bob:example.com"],
        None,
    )];
    let authorizer = Authorizer::new(&admin_regexes(), &global_config).for_room(&room_rules, None);

    assert!(!authorizer.allows(&user("@bob:example.com"), Permission::ImageGeneration));
    assert!(authorizer.allows(&user("@bob:example.com"), Permission::SpeechToText));
    assert!(authorizer.denies(&user("@bob:example.com"), Permission::ImageGeneration));
    assert!(authorizer.allows(&user("@alice:example.com"), Permission::ImageGeneration));
}

#[test]
fn room_rules_can_grant_and_deny_within_the_room() {
    let global_config = global_config_with_rules(vec![rule(
        "r1",
        RuleEffect::Allow,
        &[USE],
        &["@*:example.com"],
        None,
    )]);
    let room_rules = vec![
        rule(
            "r1",
            RuleEffect::Allow,
            &["speech-to-text"],
            &["@*:bridge.example.org"],
            None,
        ),
        rule(
            "r2",
            RuleEffect::Deny,
            &["text-generation"],
            &["@carol:example.com"],
            None,
        ),
    ];
    let authorizer = Authorizer::new(&admin_regexes(), &global_config).for_room(&room_rules, None);

    assert_eq!(
        allowed_permissions(&authorizer, "@contact:bridge.example.org"),
        [Permission::SpeechToText]
    );
    assert!(authorizer.allows_anything_in_room(&user("@contact:bridge.example.org")));
    assert!(!authorizer.allows(&user("@carol:example.com"), Permission::TextGeneration));
    assert!(authorizer.allows(&user("@carol:example.com"), Permission::TextToSpeech));
}

#[test]
fn only_global_rules_grant_invitations() {
    let global_config = global_config_with_rules(vec![]);
    let room_rules = vec![rule(
        "r1",
        RuleEffect::Allow,
        &["invite", "text-generation"],
        &["@*:example.com"],
        None,
    )];
    let authorizer = Authorizer::new(&admin_regexes(), &global_config).for_room(&room_rules, None);

    assert!(!authorizer.allows(&user("@someone:example.com"), Permission::Invite));
    assert!(authorizer.allows(&user("@someone:example.com"), Permission::TextGeneration));
}

#[test]
fn invite_alone_does_not_count_as_room_access() {
    let global_config = global_config_with_rules(vec![rule(
        "r1",
        RuleEffect::Allow,
        &["invite"],
        &["@*:example.com"],
        None,
    )]);
    let authorizer = Authorizer::new(&admin_regexes(), &global_config);

    assert!(authorizer.allows(&user("@someone:example.com"), Permission::Invite));
    assert!(!authorizer.allows_anything_in_room(&user("@someone:example.com")));
}

#[test]
fn power_level_condition() {
    let global_config = global_config_with_rules(vec![rule(
        "r1",
        RuleEffect::Allow,
        &["manage-config"],
        &["@*:example.com"],
        Some(50),
    )]);

    let authorizer = Authorizer::new(&admin_regexes(), &global_config).for_room(
        &[],
        Some(power_levels(&[
            ("@mod:example.com", 50),
            ("@owner:example.com", 100),
            ("@helper:example.com", 49),
        ])),
    );

    assert!(authorizer.allows(&user("@mod:example.com"), Permission::ManageConfig));
    assert!(authorizer.allows(&user("@owner:example.com"), Permission::ManageConfig));
    assert!(!authorizer.allows(&user("@helper:example.com"), Permission::ManageConfig));
    assert!(!authorizer.allows(&user("@nobody:example.com"), Permission::ManageConfig));

    // Without power levels (or without a room), the condition is unmet.
    let authorizer = Authorizer::new(&admin_regexes(), &global_config);
    assert!(!authorizer.allows(&user("@owner:example.com"), Permission::ManageConfig));
}

#[test]
fn room_v12_creators_meet_any_power_level() {
    let global_config = global_config_with_rules(vec![rule(
        "r1",
        RuleEffect::Allow,
        &["manage-config"],
        &["@*:example.com"],
        Some(100),
    )]);
    let content = RoomPowerLevelsEventContent::new(&AuthorizationRules::V12);
    let power_levels = RoomPowerLevels::new(
        content.into(),
        &AuthorizationRules::V12,
        vec![user("@creator:example.com")],
    );

    let authorizer =
        Authorizer::new(&admin_regexes(), &global_config).for_room(&[], Some(power_levels));

    assert!(authorizer.allows(&user("@creator:example.com"), Permission::ManageConfig));
    assert!(!authorizer.allows(&user("@other:example.com"), Permission::ManageConfig));
}

#[test]
fn unknown_permissions_and_effects_are_ignored() {
    let global_config: GlobalConfig = serde_json::from_value(serde_json::json!({
        "fallback_room_settings": {"handler": {}},
        "access": {
            "user_patterns": null,
            "room_local_agent_manager_patterns": null,
            "rules": [
                {"id": "a", "effect": "allow", "permissions": ["text-generation", "video-generation"], "users": ["@*:example.com"]},
                {"id": "b", "effect": "audit", "permissions": ["speech-to-text"], "users": ["@*:example.com"]},
            ],
        },
        "agents": [],
    }))
    .unwrap();

    let authorizer = Authorizer::new(&admin_regexes(), &global_config);

    assert_eq!(
        allowed_permissions(&authorizer, "@someone:example.com"),
        [Permission::TextGeneration]
    );
}

#[test]
fn invalid_stored_patterns_fail_closed_except_for_admins() {
    let global_config = global_config_with_rules(vec![
        rule("r1", RuleEffect::Allow, &[USE], &["@*:example.com"], None),
        rule(
            "r2",
            RuleEffect::Allow,
            &["manage-config"],
            &["not-a-pattern"],
            None,
        ),
        rule(
            "r3",
            RuleEffect::Deny,
            &["image-generation"],
            &["not-a-pattern"],
            None,
        ),
    ]);

    let authorizer = Authorizer::new(&admin_regexes(), &global_config);

    // The invalid allow rule grants nothing and the invalid deny rule denies everyone.
    assert_eq!(
        allowed_permissions(&authorizer, "@someone:example.com"),
        [
            Permission::TextGeneration,
            Permission::SpeechToText,
            Permission::TextToSpeech,
        ]
    );
    // Administrators are unaffected, so they can repair the rules.
    assert_eq!(
        allowed_permissions(&authorizer, "@admin:example.com"),
        ALL_PERMISSIONS
    );
}

#[test]
fn legacy_lists_become_rules() {
    let mut global_config = legacy_global_config(
        patterns(&["@*:example.com"]),
        patterns(&["@manager:other.com"]),
    );

    assert!(global_config.access.migrate_legacy_lists());
    // Migrating removes the lists, so a second pass changes nothing.
    assert_eq!(global_config.access.user_patterns, None);
    assert_eq!(global_config.access.room_local_agent_manager_patterns, None);
    assert!(!global_config.access.migrate_legacy_lists());

    let rules = global_config.access.rules.clone().unwrap();
    let ids: std::collections::HashSet<_> = rules.iter().map(|rule| rule.id.clone()).collect();
    assert_eq!(ids.len(), 3, "migrated rules get distinct ids");
    assert_eq!(
        without_ids(rules),
        vec![
            rule(
                "",
                RuleEffect::Allow,
                &[USE, "invite"],
                &["@*:example.com"],
                None
            ),
            rule(
                "",
                RuleEffect::Allow,
                &["manage-config"],
                &["@*:example.com"],
                Some(LEGACY_MANAGE_CONFIG_MIN_POWER_LEVEL)
            ),
            rule(
                "",
                RuleEffect::Allow,
                &["manage-agents"],
                &["@manager:other.com"],
                None
            ),
        ]
    );

    let authorizer = Authorizer::new(&admin_regexes(), &global_config)
        .for_room(&[], Some(power_levels(&[("@mod:example.com", 50)])));

    let mut user_permissions = Permission::FEATURES.to_vec();
    user_permissions.push(Permission::Invite);
    assert_eq!(
        allowed_permissions(&authorizer, "@user:example.com"),
        user_permissions
    );

    user_permissions.push(Permission::ManageConfig);
    assert_eq!(
        allowed_permissions(&authorizer, "@mod:example.com"),
        user_permissions
    );

    // Managers who were not allowed users become active managers (but get nothing else).
    assert_eq!(
        allowed_permissions(&authorizer, "@manager:other.com"),
        [Permission::ManageAgents]
    );
}

#[test]
fn lists_next_to_stored_rules_are_removed() {
    // Lists set again by an older version after a downgrade.
    let mut global_config = legacy_global_config(patterns(&["@*:example.com"]), None);
    global_config.access.rules = Some(vec![]);

    assert!(global_config.access.migrate_legacy_lists());
    assert_eq!(global_config.access.user_patterns, None);
    assert_eq!(global_config.access.rules, Some(vec![]));

    let authorizer = Authorizer::new(&admin_regexes(), &global_config);
    assert_eq!(allowed_permissions(&authorizer, "@user:example.com"), []);
}

#[test]
fn migration_without_lists_stores_empty_rules() {
    let mut global_config = legacy_global_config(None, None);

    assert!(global_config.access.migrate_legacy_lists());
    assert_eq!(global_config.access.rules, Some(vec![]));
}

#[test]
fn lists_set_by_an_older_version_after_a_downgrade_become_the_rules() {
    // The access part of the global configuration, as an older version (without rules) sees it.
    #[derive(serde::Deserialize, serde::Serialize)]
    struct OlderAccess {
        user_patterns: Option<Vec<String>>,
        room_local_agent_manager_patterns: Option<Vec<String>>,
    }

    let mut global_config = legacy_global_config(patterns(&["@*:example.com"]), None);
    global_config.access.migrate_legacy_lists();
    global_config.access.rules.as_mut().unwrap().push(rule(
        "abcd",
        RuleEffect::Deny,
        &[USE],
        &["@blocked:example.com"],
        None,
    ));

    // The older version reads the configuration, sets the allowed users again and saves it.
    let mut json = serde_json::to_value(&global_config).unwrap();
    let mut older_access: OlderAccess = serde_json::from_value(json["access"].clone()).unwrap();
    assert_eq!(older_access.user_patterns, None);
    older_access.user_patterns = patterns(&["@*:other.com"]);
    json["access"] = serde_json::to_value(&older_access).unwrap();

    // Upgrading again turns those lists into rules; rules added in between are gone.
    let mut global_config: GlobalConfig = serde_json::from_value(json).unwrap();
    assert!(global_config.access.migrate_legacy_lists());

    let mut expected = legacy_global_config(patterns(&["@*:other.com"]), None);
    expected.access.migrate_legacy_lists();
    assert_eq!(
        without_ids(global_config.access.rules.unwrap()),
        without_ids(expected.access.rules.unwrap())
    );
    assert_eq!(global_config.access.user_patterns, None);
}

#[test]
fn old_configuration_without_rules_parses() {
    let global_config: GlobalConfig = serde_json::from_value(serde_json::json!({
        "fallback_room_settings": {"handler": {}},
        "access": {
            "user_patterns": ["@*:example.com"],
            "room_local_agent_manager_patterns": null,
        },
        "agents": [],
    }))
    .unwrap();

    assert_eq!(global_config.access.rules, None);
}

#[test]
fn old_room_configuration_without_rules_parses() {
    let room_config: crate::entity::roomconfig::RoomConfig =
        serde_json::from_value(serde_json::json!({
            "settings": {"handler": {}},
            "agents": [],
        }))
        .unwrap();

    assert!(room_config.access.rules.is_empty());
}

#[test]
fn context_filters_follow_the_feature() {
    let global_config = global_config_with_rules(vec![
        rule(
            "r1",
            RuleEffect::Allow,
            &["image-generation"],
            &["@artist:example.com"],
            None,
        ),
        rule(
            "r2",
            RuleEffect::Allow,
            &["text-generation"],
            &["@writer:example.com"],
            None,
        ),
        rule(
            "r3",
            RuleEffect::Deny,
            &["text-generation"],
            &["@blocked:example.com"],
            None,
        ),
    ]);
    let authorizer = Authorizer::new(&admin_regexes(), &global_config);

    let artist = user("@artist:example.com");
    let writer = user("@writer:example.com");
    let blocked = user("@blocked:example.com");

    assert!(authorizer.allows(&artist, Permission::ImageGeneration));
    assert!(!authorizer.allows(&artist, Permission::TextGeneration));
    assert!(authorizer.allows(&writer, Permission::TextGeneration));

    // Explicit mentions include others, except those denied the feature.
    assert!(!authorizer.denies(&artist, Permission::TextGeneration));
    assert!(authorizer.denies(&blocked, Permission::TextGeneration));
}

#[test]
fn room_denies_can_remove_all_room_access() {
    let global_config = global_config_with_rules(vec![rule(
        "r1",
        RuleEffect::Allow,
        &[USE, "invite"],
        &["@*:example.com"],
        None,
    )]);
    let room_rules = vec![
        rule("r1", RuleEffect::Deny, &[USE], &["@bob:example.com"], None),
        // Room rules don't affect invitations, not even denying them.
        rule(
            "r2",
            RuleEffect::Deny,
            &["invite"],
            &["@*:example.com"],
            None,
        ),
    ];
    let authorizer = Authorizer::new(&admin_regexes(), &global_config).for_room(&room_rules, None);

    assert!(!authorizer.allows_anything_in_room(&user("@bob:example.com")));
    assert!(authorizer.allows_anything_in_room(&user("@alice:example.com")));
    assert!(authorizer.allows(&user("@bob:example.com"), Permission::Invite));
}

#[test]
fn rules_round_trip_through_serialization() {
    let rules = vec![
        rule("r1", RuleEffect::Allow, &[USE], &["@*:example.com"], None),
        rule(
            "r2",
            RuleEffect::Deny,
            &["image-generation"],
            &["@bob:example.com"],
            Some(10),
        ),
        rule(
            "r3",
            RuleEffect::Unknown("audit".to_owned()),
            &["text-generation"],
            &["@*:example.com"],
            None,
        ),
    ];

    let json = serde_json::to_value(&rules).unwrap();
    assert_eq!(json[0].get("min_power_level"), None);
    assert_eq!(json[1]["min_power_level"], 10);
    // Unknown effects are preserved, so that a newer version doesn't lose them.
    assert_eq!(json[2]["effect"], "audit");

    let parsed: Vec<AccessRule> = serde_json::from_value(json).unwrap();
    assert_eq!(parsed, rules);

    // Configurations without stored rules don't gain a `rules` field when saved.
    let global_config = legacy_global_config(None, None);
    let json = serde_json::to_value(&global_config).unwrap();
    assert_eq!(json["access"].get("rules"), None);
}

#[test]
fn new_global_config_stores_rules_from_initial_user_patterns() {
    let global_config = GlobalConfig::new(patterns(&["@*:example.com"]));

    assert_eq!(global_config.access.user_patterns, None);
    assert_eq!(
        global_config.access.rules.as_ref().map(|rules| rules.len()),
        Some(2)
    );

    let global_config = GlobalConfig::new(None);
    assert_eq!(global_config.access.rules, Some(vec![]));
}

#[test]
fn new_rule_ids_are_short_and_valid() {
    use super::rule::{is_valid_rule_id, new_rule_id};

    for _ in 0..100 {
        let id = new_rule_id(&[]);
        assert_eq!(id.len(), 4);
        assert!(is_valid_rule_id(&id), "{id}");
    }

    assert!(is_valid_rule_id("k7mq"));
    assert!(!is_valid_rule_id("a`b"));
    assert!(!is_valid_rule_id(""));
}
