use crate::{
    agent::{AgentInstance, AgentProvider, AgentPurpose, ControllerTrait, PublicIdentifier},
    utils::text::block_quote,
};

pub fn invalid_id_generic() -> String {
    "The provided agent ID is not valid.\n\nIt must have a prefix (like `static/`, `global/` or `room-local/`) followed by a unique identifier which does not contain `/` or spaces.".to_string()
}

pub fn invalid_id_validation_error(validation_error: String) -> String {
    format!("The provided agent ID is not valid. {}", validation_error)
}

pub fn agent_with_given_identifier_missing(agent_identifier: &PublicIdentifier) -> String {
    format!("There is no agent with an ID of `{}`.", agent_identifier)
}

pub fn already_exists_see_help(agent_id: &str, command_prefix: &str) -> String {
    format!(
        "An agent with the ID `{}` already exists. Send a `{} help` command to the room (**not in this thread**) for more information.",
        agent_id, command_prefix
    )
}

pub fn incorrect_creation_invocation(command_prefix: &str) -> String {
    format!(
        "Incorrect command invocation. This command expects a provider ID and an agent ID. See `{command_prefix} agent` for help."
    )
}

pub fn incorrect_invocation_expects_agent_id_arg(command_prefix: &str) -> String {
    format!(
        "Incorrect command invocation. This command expects an agent ID. See `{command_prefix} agent` for help."
    )
}

pub fn not_allowed_to_manage_room_local_agents_in_room() -> String {
    "You are not allowed to manage room-local agents in this room.".to_string()
}

pub fn not_allowed_to_manage_static_agents() -> String {
    "Statically defined agents cannot be managed via commands to the bot. Consider editing the bot configuration.".to_string()
}

pub fn configuration_does_not_result_in_a_working_agent(err: anyhow::Error) -> String {
    format!(
        "The provided configuration does not result in a working agent. The following error was encountered when trying to talk to the agent API:\n```\n{}```",
        err,
    )
}

pub fn configuration_agent_will_ping() -> &'static str {
    "Checking this agent's API. Please wait.."
}

pub fn configuration_agent_ping_inconclusive() -> String {
    "Basic check results are inconclusive - this agent may or may not work.".to_string()
}

pub fn configuration_agent_ping_ok() -> String {
    "Basic checks succeeded.".to_string()
}

pub fn created(agent_identifier: &PublicIdentifier) -> String {
    format!("Agent `{}` created.", agent_identifier)
}

pub fn post_creation_helpful_commands(
    agent_identifier: &PublicIdentifier,
    agent_instance: &AgentInstance,
    command_prefix: &str,
) -> String {
    let mut message = String::new();

    message.push_str(&format!(
        "To make use of the new agent, set it as a handler for a given purpose ({}, {}, etc.) either globally or in this room.",
        AgentPurpose::TextGeneration,
        AgentPurpose::SpeechToText,
    ));
    message.push_str("\n\n");

    let supported_purposes: Vec<&AgentPurpose> = AgentPurpose::choices()
        .into_iter()
        .filter(|&p| {
            if *p == AgentPurpose::CatchAll {
                true
            } else {
                agent_instance.controller().supports_purpose(*p)
            }
        })
        .collect();

    if !supported_purposes.is_empty() {
        message.push_str(
            "Choose and send to the room (**not in this thread**) one or a few of these commands:",
        );
        message.push('\n');

        let is_room_local = matches!(agent_identifier, PublicIdentifier::DynamicRoomLocal(_));

        for purpose in supported_purposes {
            message.push_str(&format!(
                "\n- {}",
                &set_as_purpose_handler_in_room(agent_identifier, purpose, command_prefix,)
            ));

            if !is_room_local {
                message.push_str(&format!(
                    "\n- {}",
                    &set_as_purpose_handler_globally(agent_identifier, purpose, command_prefix,)
                ));
            }
        }
    } else {
        message.push_str(
            "This agent does not support any handler purposes and cannot really be made use of.",
        );
    }

    message.push_str("\n\n");

    message.push_str(&format!(
        "For more information about configuring handlers, see `{command_prefix} config`\n",
    ));

    message
}

fn set_as_purpose_handler_in_room(
    agent_identifier: &PublicIdentifier,
    purpose: &AgentPurpose,
    command_prefix: &str,
) -> String {
    let purpose_emoji = purpose.emoji();

    format!(
        "{purpose_emoji} Set as **{purpose}** handler in **this room**: `{command_prefix} config room set-handler {purpose} {agent_identifier}`",
    )
}

fn set_as_purpose_handler_globally(
    agent_identifier: &PublicIdentifier,
    purpose: &AgentPurpose,
    command_prefix: &str,
) -> String {
    let purpose_emoji = purpose.emoji();

    format!(
        "{purpose_emoji} Set as fallback **{purpose}** handler **globally**: `{command_prefix} config global set-handler {purpose} {agent_identifier}`",
    )
}

pub fn configuration_not_a_valid_yaml_hashmap(err: String) -> String {
    format!(
        "The provided configuration is not a valid YAML hashmap:\n```\n{}\n```",
        err
    )
}

pub fn creation_guide(
    agent_identifier: &PublicIdentifier,
    provider: &AgentProvider,
    pretty_yaml: &str,
) -> String {
    let mut message = String::from("");
    message.push_str(creation_welcome(agent_identifier, provider).as_str());
    message.push('\n');
    message.push_str(creation_example_config(pretty_yaml).as_str());

    message.push_str("\n\n");
    message.push_str(creation_howto().as_str());
    message.push_str("\n\n");
    message.push_str(creation_raw_or_codeblock_ok().as_str());

    message
}

fn creation_welcome(agent_identifier: &PublicIdentifier, provider: &AgentProvider) -> String {
    format!(
        "You're defining a new agent (`{}`) powered by the `{}` provider.\n\nSend [YAML](https://en.wikipedia.org/wiki/YAML) configuration that describes it.",
        agent_identifier,
        provider.to_static_str(),
    )
}

fn creation_example_config(pretty_yaml: &str) -> String {
    format!("Below is an example:\n```yml\n{}\n```", pretty_yaml.trim())
}

fn creation_howto() -> String {
    format!(
        "{}\n\n{}",
        "Copy, modify (with your own values) and send back the configuration to this message thread.",
        "You may omit certain configuration keys (or set them to `null`) - this signals to the bot that certain capabilities are not supported by your agent.",
    )
}

fn creation_raw_or_codeblock_ok() -> String {
    "You can send the configuration as-is in a plain-text message or optionally wrap it in a [Markdown codeblock](https://www.markdownguide.org/extended-syntax/#fenced-code-blocks).".to_string()
}

pub fn removed_room_local(agent_identifier: &PublicIdentifier, command_prefix: &str) -> String {
    let mut message = String::new();

    message.push_str(&removed(agent_identifier));
    message.push_str("\n\n");

    message.push_str("This room may still have it configured as a handler. If so, handlers will fail with a friendly error message.");
    message.push_str("\n\n");

    message.push_str(&format!(
        "Use the `{command_prefix} config status` command to see the handlers for this room.",
    ));

    message
}

pub fn removed_global(agent_identifier: &PublicIdentifier, command_prefix: &str) -> String {
    let mut message = String::new();

    message.push_str(&removed(agent_identifier));
    message.push_str("\n\n");

    message.push_str("Neither per-room, nor global handlers were adjusted. Some rooms may still try to use this now-removed agent for a given purpose. If so, handlers for them will fail with a friendly error message.");
    message.push_str("\n\n");

    message.push_str(&format!(
        "Use the `{command_prefix} config status` command to see the handlers for this room, as well as those configured as a global fallback."
    ));
    message.push('\n');
    message.push_str("This does not cover everything, but it's a start.");

    message
}

fn removed(agent_identifier: &PublicIdentifier) -> String {
    format!("Agent `{}` removed.", agent_identifier)
}

pub fn purpose_unrecognized(purpose: &str) -> String {
    format!("The `{}` purpose is unrecognized.", purpose)
}

pub fn purpose_howto(purpose: &AgentPurpose) -> &'static str {
    match purpose {
        AgentPurpose::CatchAll => "used as a fallback, when no specific handler is configured",
        AgentPurpose::TextGeneration => "communicating with you via text",
        AgentPurpose::SpeechToText => "turning your voice messages into text",
        AgentPurpose::TextToSpeech => "turning bot or users text messages into voice messages",
        AgentPurpose::ImageGeneration => "generating images based on instructions",
    }
}

pub fn agent_list_empty() -> String {
    "No agents are available.".to_string()
}

pub fn non_empty_agent_list_block(agents: &Vec<AgentInstance>) -> String {
    let mut message = String::new();

    message.push_str(&agent_list_intro());
    message.push('\n');

    for agent in agents {
        let provider_info = agent.definition().provider.info();

        let provider_display = match provider_info.homepage_url {
            Some(url) => format!("[{}]({})", provider_info.name, url),
            None => provider_info.name.to_string(),
        };

        message.push_str(&format!(
            "- `{}` ({}), powered by {}\n",
            agent.identifier(),
            create_support_badges_text(agent.controller()),
            provider_display,
        ));
    }

    message
}

fn agent_list_intro() -> String {
    "The following agents are available:".to_string()
}

pub fn agent_list_legend_intro() -> String {
    "Legend:".to_string()
}

pub fn error_while_serving_purpose(
    agent_identifier: &PublicIdentifier,
    purpose: &AgentPurpose,
    err: impl std::fmt::Display,
) -> String {
    format!(
        "There was a problem performing {} via the `{}` agent:\n\n{}",
        purpose,
        agent_identifier,
        block_quote(&err.to_string())
    )
}

pub fn empty_response_returned(agent_identifier: &PublicIdentifier) -> String {
    format!("The `{agent_identifier}` agent returned an empty response.")
}

pub fn no_configuration_for_purpose_so_cannot_be_used(purpose: &AgentPurpose) -> String {
    format!(
        "This agent does not contain configuration for {} {}, so it cannot be used for that.",
        purpose.emoji(),
        purpose
    )
}

pub fn no_configuration_for_purpose_after_conversion_so_cannot_be_used(
    purpose: &AgentPurpose,
) -> String {
    format!(
        "This agent's configuration was converted to the OpenAI format, but conversion failed. There is no configuration for {} {}, so it cannot be used for that.",
        purpose.emoji(),
        purpose
    )
}

pub fn create_support_badges_text(controller: &impl ControllerTrait) -> String {
    let mut support_badges = vec![];

    for purpose in AgentPurpose::choices() {
        if *purpose == AgentPurpose::CatchAll {
            // This is not a real purpose that users care about here
            continue;
        }

        if controller.supports_purpose(*purpose) {
            support_badges.push(purpose.emoji());
        }
    }

    if support_badges.is_empty() {
        return "❌".to_owned();
    }

    support_badges.join(" ")
}
