use crate::agent::{AgentPurpose, PublicIdentifier};

pub fn no_permissions_to_administrate() -> &'static str {
    "You do not have permission to administrate the global config."
}

pub fn not_allowed_to_use_agent_in_global_config(agent_identifier: &PublicIdentifier) -> String {
    format!(
        "The agent `{}` is not allowed to be used in the global configuration.",
        agent_identifier
    )
}

pub fn global_config_lacks_specific_agent_for_purpose(purpose: AgentPurpose) -> String {
    format!(
        "The global configuration does not specify any agent for the `{}` purpose.",
        purpose
    )
}

pub fn configured_to_use_agent_for_purpose(
    agent_identifier: &PublicIdentifier,
    purpose: AgentPurpose,
) -> String {
    format!(
        "The global configuration specifies that the `{}` agent is to be used for the `{}` purpose.",
        agent_identifier, purpose
    )
}

pub fn configures_agent_for_purpose_but_does_not_exist(
    agent_identifier: &PublicIdentifier,
    purpose: AgentPurpose,
) -> String {
    format!(
        "The global configuration specifies that the `{}` agent is to be used for the `{}` purpose, but such an agent does not exist.",
        agent_identifier, purpose
    )
}

pub fn reconfigured_to_use_agent_for_purpose(
    agent_identifier: &PublicIdentifier,
    purpose: AgentPurpose,
) -> String {
    format!(
        "The global configuration has been adjusted to use the `{}` agent for the `{}` purpose.",
        agent_identifier, purpose
    )
}

pub fn reconfigured_to_not_specify_agent_for_purpose(purpose: AgentPurpose) -> String {
    format!(
        "The global configuration has been adjusted to not specify any agent for the `{}` purpose.",
        purpose
    )
}

pub fn value_was_set_to(value: impl std::fmt::Display) -> String {
    format!(
        "This global configuration value was set to:{}",
        super::cfg::create_display_text_for_value(value)
    )
}

pub fn value_was_unset() -> String {
    "This global configuration value has been unset.".to_owned()
}
