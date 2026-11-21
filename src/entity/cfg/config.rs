use std::path::PathBuf;

use mxlink::helpers::encryption::EncryptionKey;
use mxlink::matrix_sdk::ruma::{OwnedDeviceId, OwnedUserId};
use serde::{Deserialize, Deserializer, Serialize};

use crate::{
    agent::{AgentDefinition, AgentPurpose, PublicIdentifier},
    authorization::{
        AccessRule, Permission, RuleEffect,
        rule::{
            TOOL_PREFIX, USE, is_inactive_tool_selector, is_valid_permission_name,
            is_well_formed_tool_selector, known_tool_selectors, new_rule_id,
        },
    },
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
}

impl Config {
    pub fn validate(&self) -> anyhow::Result<()> {
        self.homeserver.validate()?;
        self.user.validate(&self.homeserver.server_name)?;
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

#[derive(Debug)]
pub enum ConfigUserAuth {
    UserPassword {
        username: String,
        password: String,
    },
    AccessToken {
        user_id: OwnedUserId,
        device_id: OwnedDeviceId,
        access_token: String,
    },
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

/// Configuration for the bot's avatar.
///
/// - `Default`: Use the built-in default avatar (null, empty string, or missing in config)
/// - `Keep`: Don't touch the avatar, keep whatever is already set ("keep" in config)
/// - `Custom(String)`: Use a custom avatar from the specified file path
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub enum Avatar {
    /// Use the built-in default avatar
    #[default]
    Default,
    /// Keep the current avatar, don't change it
    Keep,
    /// Use a custom avatar from the specified file path
    Custom(String),
}

impl<'de> Deserialize<'de> for Avatar {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value: Option<String> = Option::deserialize(deserializer)?;
        Ok(match value {
            None => Avatar::Default,
            Some(s) => Avatar::from_string(s),
        })
    }
}

impl Avatar {
    pub fn from_string(value: String) -> Self {
        if value.is_empty() {
            Avatar::Default
        } else if value.eq_ignore_ascii_case("keep") {
            Avatar::Keep
        } else {
            Avatar::Custom(value)
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigUser {
    pub mxid_localpart: String,

    #[serde(default)]
    pub password: Option<String>,

    #[serde(default)]
    pub access_token: Option<String>,

    #[serde(default)]
    pub device_id: Option<String>,

    #[serde(default = "super::defaults::name")]
    pub name: String,

    #[serde(default)]
    pub encryption: ConfigUserEncryption,

    #[serde(default)]
    pub avatar: Avatar,
}

impl ConfigUser {
    pub fn validate(&self, homeserver_server_name: &str) -> anyhow::Result<()> {
        if self.mxid_localpart.is_empty() {
            return Err(anyhow::anyhow!(
                "The user.mxid_localpart ({}) configuration must be set",
                super::env::BAIBOT_USER_MXID_LOCALPART
            ));
        }

        self.auth_config(homeserver_server_name)?;

        if self.name.is_empty() {
            return Err(anyhow::anyhow!(
                "The name ({}) configuration must be set",
                super::env::BAIBOT_USER_NAME
            ));
        }

        self.encryption.validate()?;

        Ok(())
    }

    pub fn auth_config(&self, homeserver_server_name: &str) -> anyhow::Result<ConfigUserAuth> {
        let password = self.password.as_deref().filter(|value| !value.is_empty());
        let access_token = self
            .access_token
            .as_deref()
            .filter(|value| !value.is_empty());

        match (password, access_token) {
            (Some(_), Some(_)) => Err(anyhow::anyhow!(
                "Set exactly one authentication method: either user.password ({}) OR user.access_token ({}) + user.device_id ({})",
                super::env::BAIBOT_USER_PASSWORD,
                super::env::BAIBOT_USER_ACCESS_TOKEN,
                super::env::BAIBOT_USER_DEVICE_ID
            )),
            (None, None) => Err(anyhow::anyhow!(
                "Set one authentication method: either user.password ({}) OR user.access_token ({}) + user.device_id ({})",
                super::env::BAIBOT_USER_PASSWORD,
                super::env::BAIBOT_USER_ACCESS_TOKEN,
                super::env::BAIBOT_USER_DEVICE_ID
            )),
            (Some(password), None) => Ok(ConfigUserAuth::UserPassword {
                username: self.mxid_localpart.to_owned(),
                password: password.to_owned(),
            }),
            (None, Some(access_token)) => {
                let device_id = self
                    .device_id
                    .as_deref()
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "user.device_id ({}) must be set when using access token authentication",
                            super::env::BAIBOT_USER_DEVICE_ID
                        )
                    })?;

                let user_id = OwnedUserId::try_from(format!(
                    "@{}:{}",
                    self.mxid_localpart, homeserver_server_name
                ))
                .map_err(|e| anyhow::anyhow!("Invalid user ID: {e}"))?;

                Ok(ConfigUserAuth::AccessToken {
                    user_id,
                    device_id: OwnedDeviceId::from(device_id),
                    access_token: access_token.to_owned(),
                })
            }
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ConfigUserEncryption {
    pub recovery_passphrase: Option<String>,
    pub recovery_reset_allowed: bool,
}

impl ConfigUserEncryption {
    pub fn validate(&self) -> anyhow::Result<()> {
        if let Some(passphrase) = &self.recovery_passphrase
            && passphrase.is_empty()
        {
            return Err(anyhow::anyhow!(
                "The user.encryption.recovery_passphrase ({}) configuration must either be null or set to a non-empty passphrase",
                super::env::BAIBOT_USER_ENCRYPTION_RECOVERY_PASSPHRASE
            ));
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

    /// Seconds subtracted from processed event timestamps when updating the global catch-up marker.
    /// Larger values tolerate older incoming events but permit more history and replay.
    #[serde(default = "super::defaults::room_catch_up_tolerance_seconds")]
    pub catch_up_tolerance_seconds: u64,
}

impl ConfigRoom {
    pub fn validate(&self) -> anyhow::Result<()> {
        Ok(())
    }

    pub fn catch_up_tolerance(&self) -> std::time::Duration {
        std::time::Duration::from_secs(self.catch_up_tolerance_seconds)
    }
}

impl Default for ConfigRoom {
    fn default() -> Self {
        Self {
            post_join_self_introduction_enabled:
                super::defaults::room_post_join_self_introduction_enabled(),
            catch_up_tolerance_seconds: super::defaults::room_catch_up_tolerance_seconds(),
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

#[derive(Debug, Clone, Deserialize)]
pub struct ConfigInitialGlobalConfig {
    #[serde(default)]
    pub handler: RoomSettingsHandler,

    /// The global access rules created on the first run.
    #[serde(default)]
    pub access_rules: Vec<ConfigInitialAccessRule>,

    /// Whether the removed `user_patterns` setting is present. It's replaced by `access_rules`.
    /// Detected explicitly, because unknown keys are otherwise ignored and the bot would start
    /// with no initial users.
    #[serde(default, rename = "user_patterns", deserialize_with = "key_is_present")]
    pub removed_user_patterns_present: bool,
}

/// An access rule in the initial global configuration.
/// Like [`AccessRule`], but without an id (generated on the first run).
///
/// Unknown keys are rejected, so that a typo (e.g. in `min_power_level`)
/// doesn't silently create a broader rule.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigInitialAccessRule {
    pub effect: ConfigInitialAccessRuleEffect,

    pub permissions: Vec<String>,

    pub users: Vec<String>,

    #[serde(default)]
    pub min_power_level: Option<i64>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConfigInitialAccessRuleEffect {
    Allow,
    Deny,
}

impl From<ConfigInitialAccessRuleEffect> for RuleEffect {
    fn from(value: ConfigInitialAccessRuleEffect) -> Self {
        match value {
            ConfigInitialAccessRuleEffect::Allow => RuleEffect::Allow,
            ConfigInitialAccessRuleEffect::Deny => RuleEffect::Deny,
        }
    }
}

impl ConfigInitialAccessRule {
    fn validate(&self, index: usize) -> anyhow::Result<()> {
        let key = format!("initial_global_config.access_rules[{index}]");

        if self.permissions.is_empty() {
            return Err(anyhow::anyhow!("{key}.permissions must not be empty"));
        }

        for name in &self.permissions {
            if !is_valid_permission_name(name) {
                if is_well_formed_tool_selector(name) {
                    // Initial rules are validated on every start, but only used on the first one.
                    // A later version may remove a tool they name, which shouldn't keep the bot from starting
                    // (the stored rule is kept and ignored), so unknown tools only get a warning
                    // (see `ConfigInitialGlobalConfig::unknown_tool_permissions`).
                    continue;
                }

                if name.starts_with(TOOL_PREFIX) {
                    return Err(anyhow::anyhow!(
                        "{key}.permissions contains `{name}`, which is not a valid tool permission (`{TOOL_PREFIX}ID` or `{TOOL_PREFIX}NAMESPACE.*`). Known tool permissions: {}",
                        known_tool_selectors().join(", "),
                    ));
                }

                return Err(anyhow::anyhow!(
                    "{key}.permissions contains an unknown permission: `{name}`. Known permissions: {}, or a tool (`{TOOL_PREFIX}ID`)",
                    known_permission_names().join(", "),
                ));
            }
        }

        if self.users.is_empty() {
            return Err(anyhow::anyhow!("{key}.users must not be empty"));
        }

        mxidwc::parse_patterns_vector(&self.users)
            .map_err(|err| anyhow::anyhow!("{key}.users could not be parsed: {err}"))?;

        if let Some(min_power_level) = self.min_power_level {
            if mxlink::matrix_sdk::ruma::Int::new(min_power_level).is_none() {
                return Err(anyhow::anyhow!(
                    "{key}.min_power_level is out of range: {min_power_level}"
                ));
            }

            if self
                .permissions
                .iter()
                .any(|name| name == Permission::Invite.name())
            {
                return Err(anyhow::anyhow!(
                    "{key} combines `{}` with min_power_level, which is not supported, because invitations happen before the bot is in a room",
                    Permission::Invite.name(),
                ));
            }
        }

        Ok(())
    }
}

/// Deserializes any value, recording only that the key is present.
fn key_is_present<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    serde::de::IgnoredAny::deserialize(deserializer)?;
    Ok(true)
}

impl ConfigInitialGlobalConfig {
    /// Returns the tool permissions in the initial access rules which match no known tool
    /// (accepted by [`Self::validate`], so that removing a tool doesn't keep the bot from starting).
    pub fn unknown_tool_permissions(&self) -> Vec<&str> {
        self.access_rules
            .iter()
            .flat_map(|rule| &rule.permissions)
            .filter(|name| is_inactive_tool_selector(name))
            .map(String::as_str)
            .collect()
    }

    pub fn validate(self) -> anyhow::Result<()> {
        if self.removed_user_patterns_present {
            return Err(anyhow::anyhow!(
                "The initial_global_config.user_patterns configuration key is no longer supported. Replace it with initial_global_config.access_rules (see docs/access.md). Example:\n{}",
                INITIAL_ACCESS_RULES_EXAMPLE,
            ));
        }

        for (index, rule) in self.access_rules.iter().enumerate() {
            rule.validate(index)?;
        }

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
        let mut rules: Vec<AccessRule> = vec![];
        for rule in self.access_rules {
            let id = new_rule_id(&rules);
            rules.push(AccessRule {
                id,
                effect: rule.effect.into(),
                permissions: rule.permissions,
                users: rule.users,
                min_power_level: rule.min_power_level,
            });
        }

        let mut entity = GlobalConfig::new(rules);

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

/// Shown when the removed `initial_global_config.user_patterns` setting is found.
const INITIAL_ACCESS_RULES_EXAMPLE: &str = r#"initial_global_config:
  access_rules:
    - effect: allow
      permissions: [use, invite]
      users: ["@*:example.com"]
    - effect: allow
      permissions: [manage-config]
      users: ["@*:example.com"]
      min_power_level: 50"#;

fn known_permission_names() -> Vec<&'static str> {
    std::iter::once(USE)
        .chain(Permission::ROOM.iter().map(|permission| permission.name()))
        .chain([Permission::Invite.name()])
        .collect()
}

#[cfg(test)]
#[path = "config_tests.rs"]
mod config_tests;
