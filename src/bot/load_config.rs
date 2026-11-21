use std::env;
use std::path::PathBuf;

use anyhow::anyhow;

use crate::agent::AgentPurpose;

pub use crate::entity::cfg::{Avatar, Config, defaults as cfg_defaults, env as cfg_env};

pub fn load() -> anyhow::Result<Config> {
    let config_file_path = env::var(cfg_env::BAIBOT_CONFIG_FILE_PATH)
        .unwrap_or_else(|_| cfg_defaults::config_file_path().to_owned());
    let config_file_path = PathBuf::from(config_file_path);

    if !config_file_path.exists() {
        return Err(anyhow!(
            "Config file ({}) not found. Adjust the {} environment variable to use another config file.",
            config_file_path.display(),
            cfg_env::BAIBOT_CONFIG_FILE_PATH,
        ));
    }

    let config_str = std::fs::read_to_string(config_file_path)?;
    let mut config: Config = serde_yaml_ng::from_str(&config_str)?;

    apply_env_overrides(&mut config, env::vars())?;

    config.validate().map_err(|s| anyhow!(s))?;

    Ok(config)
}

/// Lets environment variables override some configuration keys.
fn apply_env_overrides(
    config: &mut Config,
    vars: impl IntoIterator<Item = (String, String)>,
) -> anyhow::Result<()> {
    for (key, value) in vars {
        match key.as_str() {
            cfg_env::BAIBOT_HOMESERVER_SERVER_NAME => config.homeserver.server_name = value,
            cfg_env::BAIBOT_HOMESERVER_URL => config.homeserver.url = value,
            cfg_env::BAIBOT_USER_MXID_LOCALPART => config.user.mxid_localpart = value,
            cfg_env::BAIBOT_USER_PASSWORD => {
                config.user.password = optional_non_empty(value);
            }
            cfg_env::BAIBOT_USER_ACCESS_TOKEN => {
                config.user.access_token = optional_non_empty(value);
            }
            cfg_env::BAIBOT_USER_DEVICE_ID => {
                config.user.device_id = optional_non_empty(value);
            }
            cfg_env::BAIBOT_USER_ENCRYPTION_RECOVERY_PASSPHRASE => {
                config.user.encryption.recovery_passphrase = Some(value);
            }
            cfg_env::BAIBOT_USER_ENCRYPTION_RECOVERY_RESET_ALLOWED => {
                config.user.encryption.recovery_reset_allowed = value.parse::<bool>()?;
            }
            cfg_env::BAIBOT_USER_NAME => config.user.name = value,
            cfg_env::BAIBOT_USER_AVATAR => {
                config.user.avatar = Avatar::from_string(value);
            }
            cfg_env::BAIBOT_COMMAND_PREFIX => config.command_prefix = value,
            cfg_env::BAIBOT_ROOM_POST_JOIN_SELF_INTRODUCTION_ENABLED => {
                config.room.post_join_self_introduction_enabled = value.parse::<bool>()?;
            }
            cfg_env::BAIBOT_ROOM_CATCH_UP_TOLERANCE_SECONDS => {
                config.room.catch_up_tolerance_seconds = value.parse::<u64>()?;
            }
            cfg_env::BAIBOT_LOGGING => {
                config.logging = value;
            }
            cfg_env::BAIBOT_ACCESS_ADMIN_PATTERNS => {
                config.access.admin_patterns = value
                    .split(' ')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
            cfg_env::BAIBOT_PERSISTENCE_DATA_DIR_PATH => {
                config.persistence.data_dir_path = Some(value);
            }
            cfg_env::BAIBOT_PERSISTENCE_SESSION_ENCRYPTION_KEY => {
                config.persistence.session_encryption_key = Some(value);
            }
            cfg_env::BAIBOT_PERSISTENCE_CONFIG_ENCRYPTION_KEY => {
                config.persistence.config_encryption_key = Some(value);
            }
            cfg_env::BAIBOT_INITIAL_GLOBAL_CONFIG_HANDLER_CATCH_ALL => {
                let value = if value.is_empty() { None } else { Some(value) };

                config
                    .initial_global_config
                    .handler
                    .set_by_purpose(AgentPurpose::CatchAll, value);
            }
            cfg_env::BAIBOT_INITIAL_GLOBAL_CONFIG_HANDLER_TEXT_GENERATION => {
                let value = if value.is_empty() { None } else { Some(value) };

                config
                    .initial_global_config
                    .handler
                    .set_by_purpose(AgentPurpose::TextGeneration, value);
            }
            cfg_env::BAIBOT_INITIAL_GLOBAL_CONFIG_HANDLER_TEXT_TO_SPEECH => {
                let value = if value.is_empty() { None } else { Some(value) };

                config
                    .initial_global_config
                    .handler
                    .set_by_purpose(AgentPurpose::TextToSpeech, value);
            }
            cfg_env::BAIBOT_INITIAL_GLOBAL_CONFIG_HANDLER_SPEECH_TO_TEXT => {
                let value = if value.is_empty() { None } else { Some(value) };

                config
                    .initial_global_config
                    .handler
                    .set_by_purpose(AgentPurpose::SpeechToText, value);
            }
            cfg_env::BAIBOT_INITIAL_GLOBAL_CONFIG_HANDLER_IMAGE_GENERATION => {
                let value = if value.is_empty() { None } else { Some(value) };

                config
                    .initial_global_config
                    .handler
                    .set_by_purpose(AgentPurpose::ImageGeneration, value);
            }
            cfg_env::BAIBOT_INITIAL_GLOBAL_CONFIG_USER_PATTERNS_REMOVED => {
                return Err(anyhow!(
                    "The {} environment variable is no longer supported. Remove it and define initial_global_config.access_rules in the configuration file instead (see docs/access.md).",
                    cfg_env::BAIBOT_INITIAL_GLOBAL_CONFIG_USER_PATTERNS_REMOVED,
                ));
            }
            _ => {}
        }
    }

    Ok(())
}

fn optional_non_empty(value: String) -> Option<String> {
    if value.is_empty() { None } else { Some(value) }
}

#[cfg(test)]
mod tests {
    use super::{Config, apply_env_overrides, cfg_env};

    fn sample_config() -> Config {
        serde_yaml_ng::from_str(include_str!("../../etc/app/config.yml.dist"))
            .expect("the sample configuration should parse")
    }

    #[test]
    fn removed_initial_user_patterns_env_var_is_rejected() {
        for value in ["@*:example.com", ""] {
            let mut config = sample_config();
            let vars = [(
                cfg_env::BAIBOT_INITIAL_GLOBAL_CONFIG_USER_PATTERNS_REMOVED.to_owned(),
                value.to_owned(),
            )];

            let err = apply_env_overrides(&mut config, vars)
                .expect_err(value)
                .to_string();
            assert!(err.contains("access_rules"), "{err}");
        }
    }

    #[test]
    fn sample_config_has_valid_initial_access_rules() {
        let config = sample_config();
        assert_eq!(config.initial_global_config.access_rules.len(), 2);
        assert!(!config.initial_global_config.removed_user_patterns_present);
        config
            .initial_global_config
            .validate()
            .expect("the sample initial rules should be valid");
    }
}
