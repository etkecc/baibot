#[cfg(test)]
mod tests;

use super::super::ControllerType;

#[derive(Debug, PartialEq)]
pub enum AccessControllerType {
    Help,

    GetUsers,
    SetUsers(Option<Vec<String>>),

    GetRoomLocalAgentManagers,
    SetRoomLocalAgentManagers(Option<Vec<String>>),
}

pub fn determine_controller(text: &str) -> ControllerType {
    if text.starts_with("users") {
        return ControllerType::Access(AccessControllerType::GetUsers);
    }

    if let Some(patterns_string) = text.strip_prefix("set-users") {
        let patterns_string = patterns_string.trim().to_owned();

        let patterns_option = if patterns_string.is_empty() {
            None
        } else {
            let patterns_vector = patterns_string
                .split(" ")
                .map(|s| s.to_string())
                .collect::<Vec<String>>();

            Some(patterns_vector)
        };

        return ControllerType::Access(AccessControllerType::SetUsers(patterns_option));
    }

    if text.starts_with("room-local-agent-managers") {
        return ControllerType::Access(AccessControllerType::GetRoomLocalAgentManagers);
    }

    if let Some(patterns_string) = text.strip_prefix("set-room-local-agent-managers") {
        let patterns_string = patterns_string.trim().to_owned();

        let patterns_option = if patterns_string.is_empty() {
            None
        } else {
            let patterns_vector = patterns_string
                .split(" ")
                .map(|s| s.to_string())
                .collect::<Vec<String>>();

            Some(patterns_vector)
        };

        return ControllerType::Access(AccessControllerType::SetRoomLocalAgentManagers(
            patterns_option,
        ));
    }

    ControllerType::Access(AccessControllerType::Help)
}
