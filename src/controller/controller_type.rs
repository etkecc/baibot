use crate::authorization::Permission;

#[derive(Debug, PartialEq)]
pub enum ControllerType {
    // Denotes that the message is to be ignored.
    Ignore,

    Help,

    UsageHelp,

    Unknown,

    Error(String),
    ErrorInThread(String, mxlink::ThreadInfo),

    ProviderHelp,

    Access(super::access::AccessControllerType),

    Agent(super::agent::AgentControllerType),

    Config(super::cfg::ConfigControllerType),

    ChatCompletion(super::chat_completion::ChatCompletionControllerType),

    ImageGeneration(String),
    ImageEdit(String),
    StickerGeneration(String),
}

impl ControllerType {
    /// The permission the sender needs for this command, checked centrally before dispatching.
    pub fn required_permission(&self) -> Permission {
        match self {
            Self::Access(controller_type) => controller_type.required_permission(),
            Self::Agent(controller_type) => controller_type.required_permission(),
            Self::Config(controller_type) => controller_type.required_permission(),
            Self::Ignore
            | Self::Help
            | Self::UsageHelp
            | Self::Unknown
            | Self::Error(_)
            | Self::ErrorInThread(_, _)
            | Self::ProviderHelp
            | Self::ChatCompletion(_)
            | Self::ImageGeneration(_)
            | Self::ImageEdit(_)
            | Self::StickerGeneration(_) => Permission::Use,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ControllerType;
    use crate::agent::PublicIdentifier;
    use crate::authorization::Permission;
    use crate::controller::access::AccessControllerType;
    use crate::controller::agent::AgentControllerType;

    fn determine(text: &str) -> ControllerType {
        crate::controller::cfg::determine_controller(text)
    }

    #[test]
    fn required_permission() {
        let room_local = || PublicIdentifier::DynamicRoomLocal("id".to_owned());
        let global = || PublicIdentifier::DynamicGlobal("id".to_owned());
        let static_ = || PublicIdentifier::Static("id".to_owned());

        let details = |agent_identifier| {
            ControllerType::Agent(AgentControllerType::Details {
                agent_identifier,
                reveal_secrets: false,
            })
        };
        let delete =
            |agent_identifier| ControllerType::Agent(AgentControllerType::Delete(agent_identifier));

        let cases = vec![
            (ControllerType::Help, Permission::Use),
            (
                ControllerType::ImageGeneration("x".to_owned()),
                Permission::Use,
            ),
            (
                ControllerType::Access(AccessControllerType::Help),
                Permission::Use,
            ),
            (
                ControllerType::Access(AccessControllerType::GetUsers),
                Permission::ManageGlobalConfig,
            ),
            (
                ControllerType::Access(AccessControllerType::SetRoomLocalAgentManagers(None)),
                Permission::ManageGlobalConfig,
            ),
            (
                ControllerType::Agent(AgentControllerType::List),
                Permission::Use,
            ),
            (
                ControllerType::Agent(AgentControllerType::CreateRoomLocal {
                    provider: "openai".to_owned(),
                    agent_id: "id".to_owned(),
                }),
                Permission::ManageRoomLocalAgents,
            ),
            (
                ControllerType::Agent(AgentControllerType::CreateGlobal {
                    provider: "openai".to_owned(),
                    agent_id: "id".to_owned(),
                }),
                Permission::ManageGlobalConfig,
            ),
            (details(room_local()), Permission::ManageRoomLocalAgents),
            (
                ControllerType::Agent(AgentControllerType::Details {
                    agent_identifier: static_(),
                    reveal_secrets: true,
                }),
                Permission::ManageGlobalConfig,
            ),
            (
                ControllerType::Access(AccessControllerType::SetUsers(None)),
                Permission::ManageGlobalConfig,
            ),
            (
                ControllerType::Access(AccessControllerType::GetRoomLocalAgentManagers),
                Permission::ManageGlobalConfig,
            ),
            (
                ControllerType::Agent(AgentControllerType::Help),
                Permission::Use,
            ),
            (
                determine("room text-generation context-management-enabled"),
                Permission::Use,
            ),
            (details(global()), Permission::ManageGlobalConfig),
            (details(static_()), Permission::ManageGlobalConfig),
            (delete(room_local()), Permission::ManageRoomLocalAgents),
            (delete(global()), Permission::ManageGlobalConfig),
            (delete(static_()), Permission::Use),
            (determine("status"), Permission::Use),
            (
                determine("global text-generation context-management-enabled"),
                Permission::ManageGlobalConfig,
            ),
            (
                determine("room set-handler catch-all static/agent-id"),
                Permission::Use,
            ),
            (
                determine("global set-handler catch-all static/agent-id"),
                Permission::ManageGlobalConfig,
            ),
        ];

        for (controller_type, expected) in cases {
            assert_eq!(
                controller_type.required_permission(),
                expected,
                "{controller_type:?}"
            );
        }
    }
}
