use mxlink::matrix_sdk::ruma::{OwnedUserId, UserId};

use super::{Authorizer, Permission};
use crate::entity::globalconfig::GlobalConfig;

const ALL_PERMISSIONS: [Permission; 4] = [
    Permission::Use,
    Permission::Invite,
    Permission::ManageRoomLocalAgents,
    Permission::ManageGlobalConfig,
];

fn user(user_id: &str) -> OwnedUserId {
    UserId::parse(user_id).unwrap()
}

fn patterns(patterns: &[&str]) -> Option<Vec<String>> {
    Some(patterns.iter().map(|p| p.to_string()).collect())
}

fn global_config(
    user_patterns: Option<Vec<String>>,
    room_local_agent_manager_patterns: Option<Vec<String>>,
) -> GlobalConfig {
    let mut global_config = GlobalConfig::new(user_patterns);
    global_config.access.room_local_agent_manager_patterns = room_local_agent_manager_patterns;
    global_config
}

fn admin_regexes() -> Vec<regex::Regex> {
    mxidwc::parse_patterns_vector(&["@admin:example.com".to_owned()]).unwrap()
}

fn allowed_permissions(authorizer: &Authorizer, user_id: &str) -> Vec<Permission> {
    ALL_PERMISSIONS
        .into_iter()
        .filter(|permission| authorizer.allows(&user(user_id), *permission).unwrap())
        .collect()
}

#[test]
fn admins_have_every_permission() {
    let admin_regexes = admin_regexes();
    let global_config = global_config(None, None);
    let authorizer = Authorizer::new(&admin_regexes, &global_config);

    assert!(authorizer.is_admin(&user("@admin:example.com")));
    assert_eq!(
        allowed_permissions(&authorizer, "@admin:example.com"),
        ALL_PERMISSIONS
    );
}

#[test]
fn allowed_users_can_use_and_invite() {
    let admin_regexes = admin_regexes();
    let global_config = global_config(patterns(&["@*:example.com"]), None);
    let authorizer = Authorizer::new(&admin_regexes, &global_config);

    assert!(!authorizer.is_admin(&user("@someone:example.com")));
    assert_eq!(
        allowed_permissions(&authorizer, "@someone:example.com"),
        [Permission::Use, Permission::Invite]
    );
    assert_eq!(allowed_permissions(&authorizer, "@someone:other.com"), []);
}

#[test]
fn room_local_agent_managers_only_get_that_permission() {
    let admin_regexes = admin_regexes();
    let global_config = global_config(
        patterns(&["@user:example.com"]),
        patterns(&["@manager:example.com"]),
    );
    let authorizer = Authorizer::new(&admin_regexes, &global_config);

    assert_eq!(
        allowed_permissions(&authorizer, "@manager:example.com"),
        [Permission::ManageRoomLocalAgents]
    );
    assert_eq!(
        allowed_permissions(&authorizer, "@user:example.com"),
        [Permission::Use, Permission::Invite]
    );
}

#[test]
fn nobody_but_admins_without_stored_lists() {
    let admin_regexes = admin_regexes();
    let global_config = global_config(None, None);
    let authorizer = Authorizer::new(&admin_regexes, &global_config);

    assert_eq!(allowed_permissions(&authorizer, "@someone:example.com"), []);
}

#[test]
fn invalid_stored_patterns_fail_checks_that_depend_on_them() {
    let admin_regexes = admin_regexes();
    let global_config = global_config(patterns(&["not-a-pattern"]), None);
    let authorizer = Authorizer::new(&admin_regexes, &global_config);

    assert!(
        authorizer
            .allows(&user("@someone:example.com"), Permission::Use)
            .is_err()
    );
    assert!(
        authorizer
            .allows(&user("@someone:example.com"), Permission::Invite)
            .is_err()
    );
    // Administrators don't depend on the stored patterns.
    assert!(
        authorizer
            .allows(&user("@admin:example.com"), Permission::Use)
            .unwrap()
    );
    // Invalid user patterns don't affect unrelated permissions.
    assert!(
        !authorizer
            .allows(
                &user("@someone:example.com"),
                Permission::ManageRoomLocalAgents
            )
            .unwrap()
    );
}

#[test]
fn context_authors_are_admins_and_allowed_users() {
    let admin_regexes = admin_regexes();
    let global_config = global_config(
        patterns(&["@user:example.com"]),
        patterns(&["@manager:example.com"]),
    );
    let authorizer = Authorizer::new(&admin_regexes, &global_config);
    let regexes = authorizer.context_author_regexes().unwrap();

    assert!(mxidwc::match_user_id("@admin:example.com", &regexes));
    assert!(mxidwc::match_user_id("@user:example.com", &regexes));
    assert!(!mxidwc::match_user_id("@manager:example.com", &regexes));
}

#[test]
fn context_authors_fail_on_invalid_user_patterns() {
    let admin_regexes = admin_regexes();
    let global_config = global_config(patterns(&["@user:example.com", "not-a-pattern"]), None);
    let authorizer = Authorizer::new(&admin_regexes, &global_config);

    assert!(authorizer.context_author_regexes().is_err());

    let regexes = authorizer.admin_regexes();
    assert!(mxidwc::match_user_id("@admin:example.com", &regexes));
    assert!(!mxidwc::match_user_id("@user:example.com", &regexes));
}

#[test]
fn invalid_manager_patterns_fail_manager_checks_only() {
    let admin_regexes = admin_regexes();
    let global_config = global_config(
        patterns(&["@*:example.com"]),
        patterns(&["@manager:example.com", "not-a-pattern"]),
    );
    let authorizer = Authorizer::new(&admin_regexes, &global_config);

    assert!(
        authorizer
            .allows(
                &user("@manager:example.com"),
                Permission::ManageRoomLocalAgents
            )
            .is_err()
    );
    assert!(
        authorizer
            .allows(
                &user("@admin:example.com"),
                Permission::ManageRoomLocalAgents
            )
            .unwrap()
    );
    assert!(
        authorizer
            .allows(&user("@manager:example.com"), Permission::Use)
            .unwrap()
    );
}
