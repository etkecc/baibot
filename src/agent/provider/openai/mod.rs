//! Provider for OpenAI's own API only.
//!
//! OpenAI-compatible services (LocalAI, etc.) use the `openai_compat` provider instead,
//! so code here does not need to handle their models or options.

mod config;
mod controller;
mod realtime_tts;
mod utils;

pub use config::Config;
pub use controller::Controller;

use super::super::AgentInstantiationError;
use super::super::AgentInstantiationResult;
use super::ConfigTrait;
use super::controller::ControllerType;

pub const OPENAI_IMAGE_MODEL_GPT_IMAGE_2_5_SUNBURST: &str = "gpt-image-2.5-sunburst";

pub fn create_controller_from_yaml_value_config(
    agent_id: &str,
    config: serde_yaml_ng::Value,
) -> AgentInstantiationResult<ControllerType> {
    let config = match &config {
        serde_yaml_ng::Value::Mapping(_) => {
            let config: Config =
                serde_yaml_ng::from_value(config).map_err(AgentInstantiationError::Yaml)?;

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

    Ok(ControllerType::OpenAI(Box::new(Controller::new(config))))
}

pub fn default_config() -> Config {
    Config::default()
}
