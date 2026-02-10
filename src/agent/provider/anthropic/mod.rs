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

    let controller =
        Controller::new(config).map_err(AgentInstantiationError::ConstructionFailed)?;

    Ok(ControllerType::Anthropic(Box::new(controller)))
}

pub fn default_config() -> Config {
    Config::default()
}
