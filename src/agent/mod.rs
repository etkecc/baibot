mod definition;
mod identifier;
mod instantiation;
mod manager;
pub mod provider;
mod purpose;
pub mod utils;

pub use identifier::PublicIdentifier;
pub use manager::Manager;

pub use definition::AgentDefinition;

pub use instantiation::create_from_provider_and_yaml_value_config;
pub use instantiation::default_config_for_provider;
pub use instantiation::AgentInstance;
pub use instantiation::Error as AgentInstantiationError;
pub use instantiation::Result as AgentInstantiationResult;

pub use provider::{AgentProvider, AgentProviderInfo, ControllerTrait};
pub use purpose::AgentPurpose;

pub(super) fn default_prompt() -> &'static str {
    "You are a brief, but helpful bot called {{ baibot_name }} powered by the {{ baibot_model_id }} model. The date/time of this conversation's start is: {{ baibot_conversation_start_time_utc }}."
}
