use super::{Avatar, ConfigUser, ConfigUserAuth, ConfigUserEncryption};
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
