#[cfg(test)]
mod tests;

use crate::{agent::PublicIdentifier, controller::ControllerType, strings};

#[derive(Debug, PartialEq)]
pub enum AgentControllerType {
    List,
    Details(PublicIdentifier),
    CreateRoomLocal { provider: String, agent_id: String },
    CreateGlobal { provider: String, agent_id: String },
    Delete(PublicIdentifier),
    Help,
}

pub fn determine_controller(command_prefix: &str, text: &str) -> ControllerType {
    if text.starts_with("list") {
        return ControllerType::Agent(AgentControllerType::List);
    }
    if let Some(agent_id_string) = text.strip_prefix("details") {
        let agent_id_string = agent_id_string.trim();

        if agent_id_string.is_empty() || agent_id_string.contains(" ") {
            return ControllerType::Error(
                strings::agent::incorrect_invocation_expects_agent_id_arg(command_prefix),
            );
        }

        let Some(agent_identifier) = PublicIdentifier::from_str(agent_id_string) else {
            return ControllerType::Error(strings::agent::invalid_id_generic());
        };

        return ControllerType::Agent(AgentControllerType::Details(agent_identifier));
    }

    if let Some(remaining_text) = text.strip_prefix("create-room-local") {
        // `remaining_text` should be something like: `PROVIDER ID`
        let remaining_text = remaining_text.trim();

        let parts = remaining_text.split_once(' ');
        let Some((provider, agent_id_string)) = parts else {
            return ControllerType::Error(strings::agent::incorrect_creation_invocation(
                command_prefix,
            ));
        };

        if agent_id_string.contains(" ") {
            return ControllerType::Error(strings::agent::incorrect_creation_invocation(
                command_prefix,
            ));
        }

        return ControllerType::Agent(AgentControllerType::CreateRoomLocal {
            provider: provider.to_owned(),
            agent_id: agent_id_string.trim().to_owned(),
        });
    }

    if let Some(remaining_text) = text.strip_prefix("create-global") {
        // `remaining_text` should be something like: `PROVIDER ID`
        let remaining_text = remaining_text.trim();

        let parts = remaining_text.split_once(' ');
        let Some((provider, agent_id_string)) = parts else {
            return ControllerType::Error(strings::agent::incorrect_creation_invocation(
                command_prefix,
            ));
        };

        if agent_id_string.contains(" ") {
            return ControllerType::Error(strings::agent::incorrect_creation_invocation(
                command_prefix,
            ));
        }

        return ControllerType::Agent(AgentControllerType::CreateGlobal {
            provider: provider.to_owned(),
            agent_id: agent_id_string.trim().to_owned(),
        });
    }

    if let Some(agent_id_string) = text.strip_prefix("delete") {
        let agent_id_string = agent_id_string.trim();

        if agent_id_string.is_empty() || agent_id_string.contains(" ") {
            return ControllerType::Error(
                strings::agent::incorrect_invocation_expects_agent_id_arg(command_prefix),
            );
        }

        let Some(agent_identifier) = PublicIdentifier::from_str(agent_id_string) else {
            return ControllerType::Error(strings::agent::invalid_id_generic());
        };

        return ControllerType::Agent(AgentControllerType::Delete(agent_identifier));
    }

    ControllerType::Agent(AgentControllerType::Help)
}
