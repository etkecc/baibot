use super::{
    AgentDefinition, AgentProvider, PublicIdentifier,
    provider::{self, ControllerType},
};

// Dead-code is allowed. We do not use these enum struct payloads directly,
// but these errors are being print-formatted (`{:?}`) in error messages, so we wish to keep them.
#[derive(Debug)]
#[allow(dead_code)]
pub enum Error {
    // Contains the error message from the validation function
    ConfigFailsValidation(String),
    // Contains the agent ID
    ConfigForAgentIsNotAMapping(String),
    // Contains the error from the constructor function
    ConstructionFailed(anyhow::Error),
    // Contains the error from the YAML deserialization function
    Yaml(serde_yaml_ng::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone)]
pub struct AgentInstance {
    identifier: PublicIdentifier,
    definition: AgentDefinition,
    controller: ControllerType,
}

impl AgentInstance {
    pub fn new(
        identifier: PublicIdentifier,
        definition: AgentDefinition,
        controller: ControllerType,
    ) -> Self {
        Self {
            identifier,
            definition,
            controller,
        }
    }

    pub fn identifier(&self) -> &PublicIdentifier {
        &self.identifier
    }

    pub fn definition(&self) -> &AgentDefinition {
        &self.definition
    }

    pub fn controller(&self) -> &ControllerType {
        &self.controller
    }
}

pub(super) fn create(
    identifier: PublicIdentifier,
    definition: AgentDefinition,
) -> Result<AgentInstance> {
    let controller = create_controller_from_provider_and_json_value_config(
        &definition.id,
        &definition.provider,
        definition.config.clone(),
    )?;

    Ok(AgentInstance::new(identifier, definition, controller))
}

pub fn create_from_provider_and_yaml_value_config(
    provider: &AgentProvider,
    identifier: &PublicIdentifier,
    config: serde_yaml_ng::Value,
) -> Result<AgentInstance> {
    let definition = AgentDefinition::new(identifier.prefixless(), provider.to_owned(), config);

    create(identifier.to_owned(), definition)
}

fn create_controller_from_provider_and_json_value_config(
    agent_id: &str,
    provider: &AgentProvider,
    config: serde_yaml_ng::Value,
) -> Result<ControllerType> {
    match provider {
        AgentProvider::Anthropic => {
            provider::anthropic::create_controller_from_yaml_value_config(agent_id, config)
        }
        AgentProvider::Groq => {
            provider::openai_compat::create_controller_from_yaml_value_config(agent_id, config)
        }
        AgentProvider::Mistral => {
            provider::openai_compat::create_controller_from_yaml_value_config(agent_id, config)
        }
        AgentProvider::LocalAI => {
            provider::openai_compat::create_controller_from_yaml_value_config(agent_id, config)
        }
        AgentProvider::Ollama => {
            provider::openai_compat::create_controller_from_yaml_value_config(agent_id, config)
        }
        AgentProvider::OpenAI => {
            provider::openai::create_controller_from_yaml_value_config(agent_id, config)
        }
        AgentProvider::OpenAICompat => {
            provider::openai_compat::create_controller_from_yaml_value_config(agent_id, config)
        }
        AgentProvider::OpenRouter => {
            provider::openai_compat::create_controller_from_yaml_value_config(agent_id, config)
        }
        AgentProvider::TogetherAI => {
            provider::openai_compat::create_controller_from_yaml_value_config(agent_id, config)
        }
    }
}

pub fn default_config_for_provider(provider: &AgentProvider) -> serde_yaml_ng::Value {
    match provider {
        AgentProvider::Anthropic => {
            let config = super::provider::anthropic::default_config();
            serde_yaml_ng::to_value(config).expect("Failed to serialize config")
        }
        AgentProvider::Groq => {
            let config = super::provider::groq::default_config();
            serde_yaml_ng::to_value(config).expect("Failed to serialize config")
        }
        AgentProvider::LocalAI => {
            let config = super::provider::localai::default_config();
            serde_yaml_ng::to_value(config).expect("Failed to serialize config")
        }
        AgentProvider::Mistral => {
            let config = super::provider::mistral::default_config();
            serde_yaml_ng::to_value(config).expect("Failed to serialize config")
        }
        AgentProvider::Ollama => {
            let config = super::provider::ollama::default_config();
            serde_yaml_ng::to_value(config).expect("Failed to serialize config")
        }
        AgentProvider::OpenAI => {
            let config = super::provider::openai::default_config();
            serde_yaml_ng::to_value(config).expect("Failed to serialize config")
        }
        AgentProvider::OpenAICompat => {
            let config = super::provider::openai_compat::default_config();
            serde_yaml_ng::to_value(config).expect("Failed to serialize config")
        }
        AgentProvider::OpenRouter => {
            let config = super::provider::openrouter::default_config();
            serde_yaml_ng::to_value(config).expect("Failed to serialize config")
        }
        AgentProvider::TogetherAI => {
            let config = super::provider::togetherai::default_config();
            serde_yaml_ng::to_value(config).expect("Failed to serialize config")
        }
    }
}
