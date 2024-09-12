mod config;
mod controller;
mod utils;

pub use config::Config;
pub use controller::Controller;

use super::super::AgentInstantiationError;
use super::super::AgentInstantiationResult;
use super::controller::ControllerType;
use super::ConfigTrait;

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

    let controller =
        Controller::new(config).map_err(AgentInstantiationError::ConstructionFailed)?;

    Ok(ControllerType::Anthropic(Box::new(controller)))
}

pub fn default_config() -> Config {
    Config::default()
}
