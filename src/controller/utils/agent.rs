use mxlink::MessageResponseType;

use crate::{
    agent::{
        utils::{get_effective_agent_for_purpose, AgentForPurposeDeterminationError},
        AgentInstance, AgentPurpose,
    },
    entity::MessageContext,
    strings, Bot,
};

pub async fn get_effective_agent_for_purpose_or_complain<'a>(
    bot: &'a Bot,
    message_context: &MessageContext,
    agent_purpose: AgentPurpose,
    response_type: MessageResponseType,
    complain_when_purpose_unsupported: bool,
) -> Option<AgentInstance> {
    let agent_info = get_effective_agent_for_purpose(
        bot.agent_manager(),
        message_context.room_config_context(),
        agent_purpose,
    )
    .await;

    match agent_info {
        Ok(agent_info) => Some(agent_info.instance),
        Err(err) => {
            let error_message = match err {
                AgentForPurposeDeterminationError::Unknown(err_string) => Some(err_string),
                AgentForPurposeDeterminationError::NoneConfigured => None,
                AgentForPurposeDeterminationError::ConfiguredButMissing(agent_identifier) => Some(
                    strings::room_config::configures_agent_for_purpose_but_does_not_exist(
                        &agent_identifier,
                        agent_purpose,
                    ),
                ),
                AgentForPurposeDeterminationError::ConfiguredButLacksSupport(agent_identifier) => {
                    if complain_when_purpose_unsupported {
                        Some(strings::room_config::configures_agent_for_purpose_but_agent_does_not_support_it(
                            &agent_identifier,
                            agent_purpose,
                        ))
                    } else {
                        None
                    }
                }
            };

            if let Some(error_message) = error_message {
                bot.messaging()
                    .send_error_markdown_no_fail(
                        message_context.room(),
                        &error_message,
                        response_type,
                    )
                    .await;
            };

            None
        }
    }
}
