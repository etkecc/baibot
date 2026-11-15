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

    /// Lists the rooms the bot is in, which can be managed remotely via `InRoom`.
    RoomList,

    /// Runs a command against another room (`in <room-id-or-alias> <command>`), replying in the current room.
    InRoom(
        mxlink::matrix_sdk::ruma::OwnedRoomOrAliasId,
        Box<ControllerType>,
    ),
}

impl ControllerType {
    /// The permission the sender needs for this command, checked centrally before dispatching.
    ///
    /// `None` means that the command is available to anyone who may talk to the bot in the room
    /// (see `Authorizer::allows_anything_in_room`), which the incoming message gate already checks.
    pub fn required_permission(&self) -> Option<Permission> {
        match self {
            Self::Access(controller_type) => controller_type.required_permission(),
            Self::Agent(controller_type) => controller_type.required_permission(),
            Self::Config(controller_type) => controller_type.required_permission(),
            // Only administrators can manage other rooms for now.
            // The command itself is checked again against the target room when dispatching it.
            Self::RoomList | Self::InRoom(_, _) => Some(Permission::Administer),
            Self::ImageGeneration(_) | Self::ImageEdit(_) | Self::StickerGeneration(_) => {
                Some(Permission::ImageGeneration)
            }
            // Chat completion runs several stages (speech-to-text, text generation, text-to-speech),
            // each checking its own permission.
            Self::ChatCompletion(_) => None,
            Self::Ignore
            | Self::Help
            | Self::UsageHelp
            | Self::Unknown
            | Self::Error(_)
            | Self::ErrorInThread(_, _)
            | Self::ProviderHelp => None,
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
            (ControllerType::Help, None),
            (determine("help"), None),
            (
                ControllerType::ImageGeneration("x".to_owned()),
                Some(Permission::ImageGeneration),
            ),
            (ControllerType::Access(AccessControllerType::Help), None),
            (
                ControllerType::Access(AccessControllerType::GetUsers),
                Some(Permission::Administer),
            ),
            (
                ControllerType::Access(AccessControllerType::SetRoomLocalAgentManagers(None)),
                Some(Permission::Administer),
            ),
            (ControllerType::Agent(AgentControllerType::List), None),
            (
                ControllerType::Agent(AgentControllerType::CreateRoomLocal {
                    provider: "openai".to_owned(),
                    agent_id: "id".to_owned(),
                }),
                Some(Permission::ManageAgents),
            ),
            (
                ControllerType::Agent(AgentControllerType::CreateGlobal {
                    provider: "openai".to_owned(),
                    agent_id: "id".to_owned(),
                }),
                Some(Permission::Administer),
            ),
            (details(room_local()), Some(Permission::ManageAgents)),
            (
                ControllerType::Agent(AgentControllerType::Details {
                    agent_identifier: static_(),
                    reveal_secrets: true,
                }),
                Some(Permission::Administer),
            ),
            (
                ControllerType::Access(AccessControllerType::SetUsers(None)),
                Some(Permission::Administer),
            ),
            (
                ControllerType::Access(AccessControllerType::GetRoomLocalAgentManagers),
                Some(Permission::Administer),
            ),
            (ControllerType::Agent(AgentControllerType::Help), None),
            (
                determine("room text-generation context-management-enabled"),
                Some(Permission::ManageConfig),
            ),
            (details(global()), Some(Permission::Administer)),
            (details(static_()), Some(Permission::Administer)),
            (delete(room_local()), Some(Permission::ManageAgents)),
            (delete(global()), Some(Permission::Administer)),
            (delete(static_()), None),
            (determine("status"), Some(Permission::ManageConfig)),
            (ControllerType::RoomList, Some(Permission::Administer)),
            (
                ControllerType::InRoom(
                    "!room:example.com".try_into().unwrap(),
                    Box::new(ControllerType::Help),
                ),
                Some(Permission::Administer),
            ),
            (
                determine("global text-generation context-management-enabled"),
                Some(Permission::Administer),
            ),
            (
                determine("room set-handler catch-all static/agent-id"),
                Some(Permission::ManageConfig),
            ),
            (
                determine("global set-handler catch-all static/agent-id"),
                Some(Permission::Administer),
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
