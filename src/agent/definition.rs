use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::provider::AgentProvider;

// Custom serialization for AgentProvider
pub fn serialize_provider_to_string<S>(
    value: &AgentProvider,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(value.to_static_str())
}

// Custom deserialization for AgentProvider
pub fn deserialize_provider_from_string<'de, D>(deserializer: D) -> Result<AgentProvider, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    AgentProvider::from_string(&s).map_err(DeError::custom)
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AgentDefinition {
    pub id: String,

    #[serde(
        serialize_with = "serialize_provider_to_string",
        deserialize_with = "deserialize_provider_from_string"
    )]
    pub provider: AgentProvider,

    pub config: serde_yaml_ng::Value,
}

impl AgentDefinition {
    pub fn new(id: String, provider: AgentProvider, config: serde_yaml_ng::Value) -> Self {
        Self {
            id,
            provider,
            config,
        }
    }
}

impl PartialEq for AgentDefinition {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for AgentDefinition {}
