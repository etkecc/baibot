use super::AgentDefinition;
use super::PublicIdentifier;
use super::instantiation;
use super::instantiation::AgentInstance;
use crate::entity::RoomConfigContext;

#[derive(Debug)]
pub struct Manager {
    static_agents: Vec<AgentInstance>,
}

impl Manager {
    pub fn new(static_agent_definitions: Vec<AgentDefinition>) -> anyhow::Result<Self> {
        let mut static_agents = Vec::with_capacity(static_agent_definitions.len());

        for definition in static_agent_definitions {
            let identifier = PublicIdentifier::Static(definition.id.clone());

            match instantiation::create(identifier.clone(), definition.to_owned()) {
                Ok(instance) => static_agents.push(instance),
                Err(e) => {
                    return Err(anyhow::anyhow!(
                        "Failed to create static agent {}: {:?}",
                        identifier,
                        e
                    ));
                }
            }
        }

        Ok(Self { static_agents })
    }

    pub fn available_room_agents_by_room_config_context(
        &self,
        room_config_context: &RoomConfigContext,
    ) -> Vec<AgentInstance> {
        let mut agents: Vec<AgentInstance> = vec![];

        for agent in &self.static_agents {
            agents.push(agent.clone());
        }

        for definition in &room_config_context.global_config.agents {
            let identifier = PublicIdentifier::DynamicGlobal(definition.id.clone());

            match instantiation::create(identifier.clone(), definition.to_owned()) {
                Ok(instance) => agents.push(instance),
                Err(e) => {
                    tracing::warn!("Failed to create {} agent: {:?}. Skipping.", identifier, e);
                }
            }
        }

        for definition in &room_config_context.room_config.agents {
            let identifier = PublicIdentifier::DynamicRoomLocal(definition.id.clone());

            match instantiation::create(identifier.clone(), definition.to_owned()) {
                Ok(instance) => agents.push(instance),
                Err(e) => {
                    tracing::warn!("Failed to create {} agent: {:?}. Skipping.", identifier, e);
                }
            }
        }

        agents
    }
}
