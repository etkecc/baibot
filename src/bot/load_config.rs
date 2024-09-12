use std::env;
use std::path::PathBuf;

use anyhow::anyhow;

use crate::agent::AgentPurpose;

pub use crate::entity::cfg::{defaults as cfg_defaults, env as cfg_env, Config};

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
    let mut config: Config = serde_yaml::from_str(&config_str)?;

    // Allow environment variables to override some configuration keys
    for (key, value) in env::vars() {
        match key.as_str() {
            cfg_env::BAIBOT_HOMESERVER_SERVER_NAME => config.homeserver.server_name = value,
            cfg_env::BAIBOT_HOMESERVER_URL => config.homeserver.url = value,
            cfg_env::BAIBOT_USER_MXID_LOCALPART => config.user.mxid_localpart = value,
            cfg_env::BAIBOT_USER_PASSWORD => config.user.password = value,
            cfg_env::BAIBOT_USER_ENCRYPTION_RECOVERY_PASSPHRASE => {
                config.user.encryption.recovery_passphrase = Some(value);
            }
            cfg_env::BAIBOT_USER_NAME => config.user.name = value,
            cfg_env::BAIBOT_COMMAND_PREFIX => config.command_prefix = value,
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
            cfg_env::BAIBOT_INITIAL_GLOBAL_CONFIG_USER_PATTERNS => {
                config.initial_global_config.user_patterns = Some(
                    value
                        .split(' ')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect(),
                );
            }
            _ => {}
        }
    }

    config.validate().map_err(|s| anyhow!(s))?;

    Ok(config)
}
