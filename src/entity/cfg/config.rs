use std::path::PathBuf;

use crate::controller::chat_completion::message_aggregator::ConfigChatCompletionAggregator;

use mxlink::helpers::encryption::EncryptionKey;
use serde::{Deserialize, Serialize};

use crate::{
    agent::{AgentDefinition, AgentPurpose, PublicIdentifier},
    entity::{globalconfig::GlobalConfig, roomconfig::RoomSettingsHandler},
};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub homeserver: ConfigHomeserver,

    pub user: ConfigUser,

    pub persistence: PersistenceConfig,

    #[serde(default = "super::defaults::command_prefix")]
    pub command_prefix: String,

    #[serde(default)]
    pub room: ConfigRoom,

    pub access: ConfigAccess,

    pub agents: ConfigAgents,

    // Contains the initial global configuration values.
    // Not all properties of the object make sense to be configured statically,
    // so not all of them will be reflected onto the actual global configuration.
    pub initial_global_config: ConfigInitialGlobalConfig,

    #[serde(default = "super::defaults::logging")]
    pub logging: String,

    pub chat_completion_aggregator: ConfigChatCompletionAggregator,

    pub sqlite_db_path: String,
    
    pub uniqe_bot_id: String
}

impl Config {
    pub fn validate(&self) -> anyhow::Result<()> {
        self.homeserver.validate()?;
        self.user.validate()?;
        self.persistence.validate()?;
        self.room.validate()?;
        self.access.validate()?;

        if self.command_prefix.is_empty() {
            return Err(anyhow::anyhow!(
                "The command_prefix ({}) configuration must be set",
                super::env::BAIBOT_COMMAND_PREFIX
            ));
        }

        self.agents.validate()?;
        self.initial_global_config.clone().validate()?;

        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigHomeserver {
    pub server_name: String,
    pub url: String,
}

impl ConfigHomeserver {
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.server_name.is_empty() {
            return Err(anyhow::anyhow!(
                "The homeserver.server_name ({}) configuration must be set",
                super::env::BAIBOT_HOMESERVER_SERVER_NAME
            ));
        }

        if self.url.is_empty() {
            return Err(anyhow::anyhow!(
                "The homeserver.url ({}) configuration must be set",
                super::env::BAIBOT_HOMESERVER_URL
            ));
        }

        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigUser {
    pub mxid_localpart: String,
    pub password: String,

    #[serde(default = "super::defaults::name")]
    pub name: String,

    #[serde(default)]
    pub encryption: ConfigUserEncryption,
}

impl ConfigUser {
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.mxid_localpart.is_empty() {
            return Err(anyhow::anyhow!(
                "The user.mxid_localpart ({}) configuration must be set",
                super::env::BAIBOT_USER_MXID_LOCALPART
            ));
        }

        if self.password.is_empty() {
            return Err(anyhow::anyhow!(
                "The user.password ({}) configuration must be set",
                super::env::BAIBOT_USER_PASSWORD
            ));
        }

        if self.name.is_empty() {
            return Err(anyhow::anyhow!(
                "The name ({}) configuration must be set",
                super::env::BAIBOT_USER_NAME
            ));
        }

        self.encryption.validate()?;

        Ok(())
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ConfigUserEncryption {
    pub recovery_passphrase: Option<String>,
    pub recovery_reset_allowed: bool,
}

impl ConfigUserEncryption {
    pub fn validate(&self) -> anyhow::Result<()> {
        if let Some(passphrase) = &self.recovery_passphrase {
            if passphrase.is_empty() {
                return Err(anyhow::anyhow!(
                    "The user.encryption.recovery_passphrase ({}) configuration must either be null or set to a non-empty passphrase",
                    super::env::BAIBOT_USER_ENCRYPTION_RECOVERY_PASSPHRASE
                ));
            }
        }

        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PersistenceConfig {
    #[serde(default = "super::defaults::persistence_data_dir_path")]
    pub data_dir_path: Option<String>,

    #[serde(default = "super::defaults::persistence_session_file_name")]
    session_file_name: String,

    #[serde(default = "super::defaults::persistence_db_dir_name")]
    db_dir_name: String,

    pub session_encryption_key: Option<String>,

    pub config_encryption_key: Option<String>,
}

impl PersistenceConfig {
    pub fn validate(&self) -> anyhow::Result<()> {
        if let Some(data_dir_path) = &self.data_dir_path {
            let path = PathBuf::from(data_dir_path);
            if !path.exists() {
                return Err(anyhow::anyhow!(
                    "The persistence.data_dir_path ({}) directory ({}) must exist",
                    super::env::BAIBOT_PERSISTENCE_DATA_DIR_PATH,
                    data_dir_path,
                ));
            }
        }

        self.config_encryption_key()
            .map_err(|e| anyhow::anyhow!(e))?;

        Ok(())
    }

    pub fn session_file_path(&self) -> anyhow::Result<PathBuf> {
        let Some(data_dir_path) = &self.data_dir_path else {
            return Err(anyhow::anyhow!(
                "The persistence.data_dir_path ({}) directory must be set",
                super::env::BAIBOT_PERSISTENCE_DATA_DIR_PATH
            ));
        };

        let mut path = PathBuf::from(data_dir_path);
        path.push(&self.session_file_name);

        Ok(path)
    }

    pub fn db_dir_path(&self) -> anyhow::Result<PathBuf> {
        let Some(data_dir_path) = &self.data_dir_path else {
            return Err(anyhow::anyhow!(
                "The persistence.data_dir_path ({}) directory must be set",
                super::env::BAIBOT_PERSISTENCE_DATA_DIR_PATH
            ));
        };

        let mut path = PathBuf::from(data_dir_path);
        path.push(&self.db_dir_name);

        Ok(path)
    }

    pub fn session_encryption_key(&self) -> anyhow::Result<Option<EncryptionKey>> {
        self.parse_encryption_key(&self.session_encryption_key).map_err(|err| {
            anyhow::anyhow!(
                "Encryption key specified in persistence.session_encryption_key ({}) is not valid: {}",
                super::env::BAIBOT_PERSISTENCE_SESSION_ENCRYPTION_KEY,
                err
            )
        })
    }

    pub fn config_encryption_key(&self) -> anyhow::Result<Option<EncryptionKey>> {
        self.parse_encryption_key(&self.config_encryption_key).map_err(|err| {
            anyhow::anyhow!(
                "Encryption key specified in persistence.config_encryption_key ({}) is not valid: {}",
                super::env::BAIBOT_PERSISTENCE_CONFIG_ENCRYPTION_KEY,
                err
            )
        })
    }

    fn parse_encryption_key(
        &self,
        value: &Option<String>,
    ) -> anyhow::Result<Option<EncryptionKey>, String> {
        let key = match value {
            Some(key) => {
                if key.is_empty() {
                    None
                } else {
                    Some(EncryptionKey::from_hex_str(key)?)
                }
            }
            None => None,
        };

        Ok(key)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigRoom {
    #[serde(default = "super::defaults::room_post_join_self_introduction_enabled")]
    pub post_join_self_introduction_enabled: bool,
}

impl ConfigRoom {
    pub fn validate(&self) -> anyhow::Result<()> {
        Ok(())
    }
}

impl Default for ConfigRoom {
    fn default() -> Self {
        Self {
            post_join_self_introduction_enabled:
                super::defaults::room_post_join_self_introduction_enabled(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigAccess {
    // Contains the admin whitelist patterns before parsing into regex.
    // Example: `["@*:example.com"]`
    pub admin_patterns: Vec<String>,
}

impl ConfigAccess {
    // Returns the the mxidwc-parsed regexes for the admin whitelist.
    // Example: `["^@\.*:example\.com$"]`
    pub fn admin_pattern_regexes(&self) -> anyhow::Result<Vec<regex::Regex>> {
        mxidwc::parse_patterns_vector(&self.admin_patterns).map_err(|e| {
            anyhow::anyhow!(
                "Failed parsing access.admin_patterns ({}): {:?}",
                super::env::BAIBOT_ACCESS_ADMIN_PATTERNS,
                e
            )
        })
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        if self.admin_patterns.is_empty() {
            return Err(anyhow::anyhow!(
                "The access.admin_patterns ({}) configuration must contain at least one pattern",
                super::env::BAIBOT_ACCESS_ADMIN_PATTERNS
            ));
        }

        self.admin_pattern_regexes()?;

        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigAgents {
    pub static_definitions: Vec<AgentDefinition>,
}

impl ConfigAgents {
    pub fn validate(&self) -> anyhow::Result<()> {
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigInitialGlobalConfig {
    #[serde(default)]
    pub handler: RoomSettingsHandler,

    pub user_patterns: Option<Vec<String>>,
}

impl ConfigInitialGlobalConfig {
    fn user_pattern_regexes(&self) -> anyhow::Result<Option<Vec<regex::Regex>>> {
        match &self.user_patterns {
            Some(user_patterns) => {
                let user_patterns = mxidwc::parse_patterns_vector(user_patterns).map_err(|e| {
                    anyhow::anyhow!(
                        "Failed parsing initial_global_config.user_patterns ({}): {}",
                        super::env::BAIBOT_INITIAL_GLOBAL_CONFIG_USER_PATTERNS,
                        e
                    )
                })?;

                Ok(Some(user_patterns))
            }
            None => Ok(None),
        }
    }

    pub fn validate(self) -> anyhow::Result<()> {
        self.user_pattern_regexes()?;

        for purpose in AgentPurpose::choices() {
            let agent_id = self.handler.get_by_purpose(*purpose);

            let Some(agent_id) = agent_id else {
                // None is OK
                continue;
            };

            let config_key = format!(
                "initial_global_config.handler.{}",
                purpose.as_str().replace("-", "_")
            );

            if agent_id.is_empty() {
                return Err(anyhow::anyhow!(
                    "The {} configuration key must be pointing to a valid agent id or be set to null",
                    config_key,
                ));
            }

            let agent_identifier = PublicIdentifier::from_str(&agent_id);

            let Some(agent_identifier) = agent_identifier else {
                return Err(anyhow::anyhow!(
                    "The {} configuration key specifies an agent id (`{}`) that cannot be parsed. {}",
                    config_key,
                    agent_id,
                    crate::strings::agent::invalid_id_generic()
                ));
            };

            // We only allow statically-defined agents for now, although DynamicGlobal may make sense too.
            let PublicIdentifier::Static(_) = agent_identifier else {
                return Err(anyhow::anyhow!(
                    "The {} configuration key specifies an agent id (`{}`) which does not refer to a static agent.",
                    config_key,
                    agent_id,
                ));
            };
        }

        let _: GlobalConfig = self.try_into()?;

        Ok(())
    }
}

impl TryInto<GlobalConfig> for ConfigInitialGlobalConfig {
    type Error = anyhow::Error;

    fn try_into(self) -> anyhow::Result<GlobalConfig> {
        let mut entity = GlobalConfig::default();

        if let Some(user_patterns) = self.user_patterns {
            // We'd rather fail parsing this during startup than at runtime
            let _ = mxidwc::parse_patterns_vector(&user_patterns).map_err(|err| {
                anyhow::anyhow!(
                    "Bad initial_global_config.user_patterns ({}): {}",
                    super::env::BAIBOT_INITIAL_GLOBAL_CONFIG_USER_PATTERNS,
                    err
                )
            })?;

            entity.access.user_patterns = if user_patterns.is_empty() {
                None
            } else {
                Some(user_patterns)
            };
        }

        for purpose in AgentPurpose::choices() {
            let agent_id = self.handler.get_by_purpose(*purpose);

            entity
                .fallback_room_settings
                .handler
                .set_by_purpose(*purpose, agent_id);
        }

        Ok(entity)
    }
}
