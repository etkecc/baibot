use crate::agent::{AgentPurpose, PublicIdentifier};

pub fn room_not_configured_with_specific_agent_for_purpose(purpose: AgentPurpose) -> String {
    format!(
        "This room is not configured to use any specific agent for the `{}` purpose.",
        purpose
    )
}

pub fn configured_to_use_agent_for_purpose(
    agent_identifier: &PublicIdentifier,
    purpose: AgentPurpose,
) -> String {
    format!(
        "This room is configured to use the `{agent_identifier}` agent for the `{purpose}` purpose.",
    )
}

pub fn configures_agent_for_purpose_but_does_not_exist(
    agent_identifier: &PublicIdentifier,
    purpose: AgentPurpose,
) -> String {
    format!(
        "This room is configured to use the `{agent_identifier}` agent for the `{purpose}` purpose, but such an agent does not exist.",
    )
}

pub fn configures_agent_for_purpose_but_agent_does_not_support_it(
    agent_identifier: &PublicIdentifier,
    purpose: AgentPurpose,
) -> String {
    format!(
        "This room is configured to use the `{}` agent for {} (either directly, or through a {} fallback), but this agent does not support being used for {}.",
        agent_identifier,
        purpose,
        AgentPurpose::CatchAll,
        purpose,
    )
}

pub fn reconfigured_to_use_agent_for_purpose(
    agent_identifier: &PublicIdentifier,
    purpose: AgentPurpose,
) -> String {
    format!(
        "This room has been reconfigured to use the `{}` agent for the `{}` purpose.",
        agent_identifier, purpose
    )
}

pub fn reconfigured_to_not_specify_agent_for_purpose(purpose: AgentPurpose) -> String {
    format!(
        "This room has been reconfigured to not specify any agent for the `{}` purpose.",
        purpose
    )
}

pub fn value_was_set_to(value: impl std::fmt::Display) -> String {
    format!(
        "This room-specific configuration value was set to:{}",
        super::cfg::create_display_text_for_value(value)
    )
}

pub fn value_was_unset() -> String {
    "This room-specific configuration value has been unset.".to_owned()
}
