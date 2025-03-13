use crate::agent::utils::AgentForPurposeDeterminationError;
use crate::agent::utils::get_effective_agent_for_purpose;
use crate::agent::{AgentPurpose, Manager as AgentManager};
use crate::entity::RoomConfigContext;
use crate::entity::roomconfig::TextGenerationPrefixRequirementType;

fn hello() -> &'static str {
    "Hello! 👋"
}

pub fn its_me(name: &str) -> String {
    let mut message = format!(
        "I'm {name} - a bot exposing the power of [AI](https://en.wikipedia.org/wiki/Artificial_intelligence) ([Large Language Models](https://en.wikipedia.org/wiki/Large_language_model)) to you. 🤖"
    );

    if name == crate::entity::cfg::defaults::name() {
        message.push('\n');
        message.push_str("My name is pronounced 'bye' and is a play on [AI](https://en.wikipedia.org/wiki/Artificial_intelligence), referencing the fictional character [🇧🇬 Bai Ganyo](https://en.wikipedia.org/wiki/Bay_Ganyo).");
    }

    message
}

fn purposes_intro() -> &'static str {
    "I can typically be used for the following purposes:"
}

pub async fn create_on_join_introduction(
    name: &str,
    command_prefix: &str,
    agent_manager: &AgentManager,
    room_config_context: &RoomConfigContext,
) -> String {
    let mut message = String::new();

    message.push_str(hello());
    message.push_str("\n\n");
    message.push_str(&create_short_introduction(name));
    message.push_str("\n\n");

    let mut got_text_generation_agent = false;

    message.push_str(purposes_intro());
    for purpose in AgentPurpose::choices() {
        if *purpose == AgentPurpose::CatchAll {
            continue;
        }

        let mut purpose_intro_line = format!(
            "\n- {} {}: {}",
            purpose.emoji(),
            purpose.as_str(),
            super::agent::purpose_howto(purpose),
        );

        let agent_info =
            get_effective_agent_for_purpose(agent_manager, room_config_context, *purpose).await;

        let current_status_text = match agent_info {
            Ok(agent_info) => {
                let agent_instance = agent_info.instance;
                let provider_info = agent_instance.definition().provider.info();

                if *purpose == AgentPurpose::TextGeneration {
                    got_text_generation_agent = true;
                }

                let provider_display = match provider_info.homepage_url {
                    Some(url) => format!("[{}]({})", provider_info.name, url),
                    None => provider_info.name.to_owned(),
                };

                format!(
                    "✅ enabled via the `{}` agent, powered by the {} provider",
                    agent_instance.identifier(),
                    provider_display,
                )
            }
            Err(err) => match err {
                AgentForPurposeDeterminationError::Unknown(err) => {
                    crate::utils::status::create_error_message_text(&err).to_owned()
                }
                AgentForPurposeDeterminationError::NoneConfigured => {
                    "❌ no agent configured".to_string()
                }
                AgentForPurposeDeterminationError::ConfiguredButMissing(agent_identifier) => {
                    format!("❌ configured via `{agent_identifier}`, but the agent is missing")
                }
                AgentForPurposeDeterminationError::ConfiguredButLacksSupport(agent_identifier) => {
                    format!("❌ configured via `{agent_identifier}`, but support is missing")
                }
            },
        };

        purpose_intro_line.push_str(&format!(" ({})", current_status_text));

        message.push_str(&purpose_intro_line);
    }
    message.push_str("\n\n");

    if got_text_generation_agent {
        message.push_str(&make_use_of_me_simply_send_a_message(
            command_prefix,
            room_config_context.text_generation_prefix_requirement_type(),
        ));
    } else {
        message.push_str(&make_use_of_me_agent_creation(
            command_prefix,
            room_config_context.text_generation_prefix_requirement_type(),
        ));
    }

    message
}

pub fn create_short_introduction(name: &str) -> String {
    its_me(name)
}

fn make_use_of_me_simply_send_a_message(
    command_prefix: &str,
    prefix_requirement_type: TextGenerationPrefixRequirementType,
) -> String {
    let message = r#"**To make use of me**:

1. 👋 %send_a_message%
2. 📖 %learn_more%
"#;

    message
        .replace("%command_prefix%", command_prefix)
        .replace(
            "%send_a_message%",
            &send_a_text_message(command_prefix, prefix_requirement_type),
        )
        .replace(
            "%learn_more%",
            &learn_more_from_usage_or_help(command_prefix),
        )
}

fn make_use_of_me_agent_creation(
    command_prefix: &str,
    prefix_requirement_type: TextGenerationPrefixRequirementType,
) -> String {
    let message = r#"**To make use of me**:

1. ☁️ **Choose an agent provider** (e.g. OpenAI, Mistral, etc). Send a `%command_prefix% provider` command to see the list.
2. 🤖 %create_one_or_more_agents%
3. 🤝 %set_new_agent_as_handler%
4. 👋 %send_a_message%
5. 📖 %learn_more%
"#;

    message
        .replace("%command_prefix%", command_prefix)
        .replace(
            "%send_a_message%",
            &send_a_text_message(command_prefix, prefix_requirement_type),
        )
        .replace(
            "%learn_more%",
            &learn_more_from_usage_or_help(command_prefix),
        )
        .replace(
            "%create_one_or_more_agents%",
            &create_one_or_more_agents(command_prefix),
        )
        .replace(
            "%set_new_agent_as_handler%",
            &set_new_agent_as_handler(command_prefix),
        )
}

fn send_a_text_message(
    command_prefix: &str,
    prefix_requirement_type: TextGenerationPrefixRequirementType,
) -> String {
    match prefix_requirement_type {
        TextGenerationPrefixRequirementType::No => {
            "**Send a text message** in this room (e.g. `Hello!`) and see me reply.".to_owned()
        }
        TextGenerationPrefixRequirementType::CommandPrefix => {
            format!(
                "In this room, I'm configured to require the command prefix (`{command_prefix}`) for text messages. **Send a prefixed text message** (e.g. `{command_prefix} Hello!`) and see me reply."
            )
        }
    }
}

fn learn_more_from_usage_or_help(command_prefix: &str) -> String {
    format!(
        "**Learn more** by sending a `{command_prefix} usage` or `{command_prefix} help` command."
    )
}

pub fn create_one_or_more_agents(command_prefix: &str) -> String {
    format!(
        "**Create one or more agents** in this room or globally. The provider help message will show you **🗲 Quick start** commands, but you may also send a `{command_prefix} agent` command to see the guide."
    )
}

pub fn set_new_agent_as_handler(command_prefix: &str) -> String {
    format!(
        "**Set the new agent as a handler** for a given use-purpose like text-generation, image-generation, etc. The agent-creation wizard will tell you how, but you may also send a `{command_prefix} config` command to see the guide (in the 🤖 *Handler Agents* section)."
    )
}
