use crate::{
    agent::{
        AgentInstance, AgentPurpose, ControllerTrait, Manager as AgentManager, PublicIdentifier,
    },
    entity::RoomConfigContext,
    strings,
};

#[derive(Debug)]
pub struct AgentForPurposeDeterminationInfo {
    pub instance: AgentInstance,
    pub configuration_source: AgentForPurposeDeterminationInfoConfigurationSource,
}

#[derive(Debug)]
pub enum AgentForPurposeDeterminationInfoConfigurationSource {
    Room,
    Global,
}

#[derive(Debug)]
pub enum AgentForPurposeDeterminationError {
    Unknown(String),

    NoneConfigured,

    ConfiguredButMissing(PublicIdentifier),

    ConfiguredButLacksSupport(PublicIdentifier),
}

pub async fn get_effective_agent_for_purpose(
    agent_manager: &AgentManager,
    room_config_context: &RoomConfigContext,
    agent_purpose: AgentPurpose,
) -> Result<AgentForPurposeDeterminationInfo, AgentForPurposeDeterminationError> {
    let (agent_identifier, configuration_source) =
        match get_effective_room_agent_identifier_for_purpose(room_config_context, agent_purpose)
            .await
        {
            Ok((agent_identifier, configuration_source)) => {
                (agent_identifier, configuration_source)
            }
            Err(err) => {
                return Err(AgentForPurposeDeterminationError::Unknown(err));
            }
        };

    let Some(agent_identifier) = agent_identifier else {
        return Err(AgentForPurposeDeterminationError::NoneConfigured);
    };

    let agents = agent_manager.available_room_agents_by_room_config_context(room_config_context);

    let Some(agent_instance) = agents.iter().find(|a| *a.identifier() == agent_identifier) else {
        return Err(AgentForPurposeDeterminationError::ConfiguredButMissing(
            agent_identifier,
        ));
    };

    let agent_instance = agent_instance.clone();

    let supports_purpose = agent_instance.controller().supports_purpose(agent_purpose);

    if !supports_purpose {
        return Err(AgentForPurposeDeterminationError::ConfiguredButLacksSupport(agent_identifier));
    }

    Ok(AgentForPurposeDeterminationInfo {
        instance: agent_instance,
        configuration_source,
    })
}

async fn get_effective_room_agent_identifier_for_purpose(
    room_config_context: &RoomConfigContext,
    purpose: AgentPurpose,
) -> Result<
    (
        Option<PublicIdentifier>,
        AgentForPurposeDeterminationInfoConfigurationSource,
    ),
    String,
> {
    let (agent_id, configuration_source) =
        get_effective_room_agent_raw_id_for_purpose(room_config_context, purpose).await;

    let Some(agent_id) = agent_id else {
        return Ok((None, configuration_source));
    };

    let agent_identifier = match PublicIdentifier::from_str(agent_id.as_str()) {
        Some(agent_identifier) => agent_identifier,
        None => return Err(strings::agent::invalid_id_generic()),
    };

    Ok((Some(agent_identifier), configuration_source))
}

async fn get_effective_room_agent_raw_id_for_purpose(
    room_config_context: &RoomConfigContext,
    purpose: AgentPurpose,
) -> (
    Option<String>,
    AgentForPurposeDeterminationInfoConfigurationSource,
) {
    let agent_id = room_config_context
        .room_config
        .settings
        .handler
        .get_by_purpose_with_catch_all_fallback(purpose);

    if let Some(agent_id) = agent_id {
        return (
            Some(agent_id),
            AgentForPurposeDeterminationInfoConfigurationSource::Room,
        );
    }

    tracing::trace!(
        ?purpose,
        "No specific agent found for purpose in room, falling back to global.",
    );

    (
        get_global_agent_id_for_purpose(room_config_context, purpose).await,
        AgentForPurposeDeterminationInfoConfigurationSource::Global,
    )
}

async fn get_global_agent_id_for_purpose(
    room_config_context: &RoomConfigContext,
    purpose: AgentPurpose,
) -> Option<String> {
    room_config_context
        .global_config
        .fallback_room_settings
        .handler
        .get_by_purpose_with_catch_all_fallback(purpose)
}
