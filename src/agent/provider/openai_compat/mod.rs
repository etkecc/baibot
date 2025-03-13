// The openai_compat provider aims to support a wider ranger of OpenAI-compatible providers.
//
// The `openai` provider is based on `async-openai`, which only aims to support the OpenAI API spec. See:
// - https://github.com/64bit/async-openai/issues/266
// - https://github.com/64bit/async-openai/blob/05d5a1b4fa6476829dd1a34447b80279cf89d4f8/async-openai/README.md#contributing
//
// This module uses its own configuration, which avoids using strict types tied to OpenAI,
// and thus allows for more flexibility.
//
// Communication with the OpenAI-compatible API is handled by the `openai_api_rust` crate.
// Since this crate is not async-aware, we need to use tokio's `spawn_blocking` to invoke it.
//
// Certain features (e.g. text-to-speech) are not supported by `openai_api_rust` yet, so we may try to delegate them to the `openai` provider.

mod config;
mod controller;
mod utils;

pub use config::Config;
pub use controller::Controller;

use super::super::AgentInstantiationError;
use super::super::AgentInstantiationResult;
use super::ConfigTrait;
use super::controller::ControllerType;

pub fn create_controller_from_yaml_value_config(
    agent_id: &str,
    config: serde_yaml::Value,
) -> AgentInstantiationResult<ControllerType> {
    let config = match &config {
        serde_yaml::Value::Mapping(_) => {
            let config: Config =
                serde_yaml::from_value(config).map_err(AgentInstantiationError::Yaml)?;

            config
                .validate()
                .map_err(AgentInstantiationError::ConfigFailsValidation)?;

            config
        }
        _ => {
            return Err(AgentInstantiationError::ConfigForAgentIsNotAMapping(
                agent_id.to_owned(),
            ));
        }
    };

    Ok(ControllerType::OpenAICompat(Box::new(Controller::new(
        config,
    ))))
}

pub fn default_config() -> Config {
    let mut config = Config::default();

    if let Some(text_generation) = &mut config.text_generation {
        text_generation.model_id = "some-model".to_string();
        text_generation.max_response_tokens = Some(4096);
        text_generation.max_context_tokens = 128_000;
    }

    // We don't support these, so let's remove them from the configuration.
    config.text_to_speech = None;
    config.image_generation = None;

    config.base_url = "".to_owned();

    config
}
