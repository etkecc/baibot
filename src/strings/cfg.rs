use crate::{
    agent::{
        utils::AgentForPurposeDeterminationInfoConfigurationSource, AgentInstance, AgentPurpose,
        PublicIdentifier,
    },
    entity::roomconfig::{
        SpeechToTextFlowType, SpeechToTextMessageTypeForNonThreadedOnlyTranscribedMessages,
        TextGenerationAutoUsage, TextGenerationPrefixRequirementType,
        TextToSpeechBotMessagesFlowType, TextToSpeechUserMessagesFlowType,
    },
    utils::text::block_quote,
};

pub fn error_config_type_not_replaced() -> String {
    "The `CONFIG_TYPE` placeholder in the command was not replaced.\n\nIt should either be set to `room` (for room-specific configuration) or `global` (for global configuration).".to_owned()
}

pub fn create_display_text_for_value(value: impl std::fmt::Display) -> String {
    let value = value.to_string();

    if value.to_string().contains("\n") {
        format!("\n\n{}\n", block_quote(&value))
    } else {
        format!(" `{}`", value)
    }
}

pub fn value_currently_set_to(value: impl std::fmt::Display) -> String {
    format!(
        "This configuration value is currently set to:{}",
        create_display_text_for_value(value)
    )
}

pub fn value_currently_unset() -> String {
    "This configuration value is currently unset.".to_owned()
}

pub fn configuration_invocation_incorrect_more_values_expected() -> String {
    "The invocation for this command is incorrect. More values are expected in your command."
        .to_string()
}

pub fn configuration_getter_used_with_extra_text(
    getter_name: &str,
    remaining_text: &str,
) -> String {
    format!("You're invoking a getter command (`{getter_name}`), but passing additional text (`{remaining_text}`) as if you're invoking a setter.\n\nPerhaps you meant to invoke `set-{getter_name}`?")
}

pub fn configuration_value_unrecognized(value: &str) -> String {
    format!("The value `{}` is not a recognized choice.", value)
}

pub fn configuration_value_not_f32(value: &str) -> String {
    format!("The value `{}` could not be converted to a [floating point number](https://en.wikipedia.org/wiki/Floating-point_arithmetic).", value)
}

pub fn status_room_config_handlers_heading() -> &'static str {
    "📍 Room-specific handlers"
}

pub fn status_room_config_handlers_intro() -> &'static str {
    "This **room's configuration** specifies the following handlers (**taking priority** over global handlers):"
}

pub fn status_room_config_handlers_outro(command_prefix: &str) -> String {
    let mut message = String::new();

    message.push_str(&format!(
        "Use `{command_prefix} config room set-handler` commands (see how via `{command_prefix} config`) to configure the handlers for this room.",
    ));

    message.push_str("\n\n");

    message.push_str(
        "If a particular handler is unset, the catch-all handler would be used. If the catch-all handler is also unset, the global configuration would be used.",
    );

    message
}

pub fn status_global_config_handlers_heading() -> &'static str {
    "🌐 Global handlers"
}

pub fn status_global_config_handlers_intro() -> &'static str {
    "The **global configuration** specifies the following handlers:"
}

pub fn status_global_config_handlers_outro(command_prefix: &str) -> String {
    let mut message = String::new();

    message.push_str(&format!(
        "Use `{command_prefix} config global set-handler` commands (see how via `{command_prefix} config`) to configure the default handlers globally.",
    ));

    message.push_str("\n\n");

    message.push_str("If a particular handler is unset, the catch-all agent would be used.");

    message
}

fn status_agent_not_found() -> &'static str {
    "not found"
}

pub fn status_handler_line_agent_found(
    purpose: &AgentPurpose,
    agent_id: &str,
    agent: Option<&AgentInstance>,
) -> String {
    let agent_status = match agent {
        Some(agent) => super::agent::create_support_badges_text(agent.controller()),
        None => status_agent_not_found().to_string(),
    };

    format!(
        "- {} {}: `{}` ({})",
        purpose.emoji(),
        purpose,
        agent_id,
        agent_status,
    )
    .to_owned()
}

pub fn status_handler_line_catch_all_agent_not_set_globally() -> String {
    format!(
        "- {} {}: *not set*",
        AgentPurpose::CatchAll.emoji(),
        AgentPurpose::CatchAll,
    )
    .to_owned()
}

pub fn status_handler_line_catch_all_agent_not_set_in_room_default_to_global() -> String {
    format!(
        "- {} {}: *not set, defaulting to global config*",
        AgentPurpose::CatchAll.emoji(),
        AgentPurpose::CatchAll,
    )
    .to_owned()
}

pub fn status_handler_line_non_catch_all_agent_not_set_globally(purpose: &AgentPurpose) -> String {
    format!(
        "- {} {}: *not set, defaulting to {}*",
        purpose.emoji(),
        purpose,
        AgentPurpose::CatchAll,
    )
    .to_owned()
}

pub fn status_handler_line_non_catch_all_agent_not_set_in_room_default_to_global(
    purpose: &AgentPurpose,
) -> String {
    format!(
        "- {} {}: *not set, defaulting to {} or global config*",
        purpose.emoji(),
        purpose,
        AgentPurpose::CatchAll,
    )
    .to_owned()
}

pub fn status_room_agents_heading() -> &'static str {
    "🤖 Room-specific agents"
}

pub fn status_room_agents_intro() -> &'static str {
    "The following agents have been defined in this room:"
}

pub fn status_room_agents_empty() -> &'static str {
    "No agents have been defined in this room."
}

pub fn status_room_agents_outro(command_prefix: &str) -> String {
    let mut message = String::new();

    message.push_str(
        format!("Use `{command_prefix} agent create-room-local` commands (see how via `{command_prefix} help`) to define agents in this room.").as_str()
    );

    message.push_str("\n\n");

    message.push_str(format!(
        "You may also use any of the globally defined agents. See `{command_prefix} agent list` to see the full list of agents."
    ).as_str());

    message
}

pub fn status_text_generation_heading() -> String {
    format!(
        "{} {}",
        AgentPurpose::TextGeneration.emoji(),
        AgentPurpose::TextGeneration.heading()
    )
}

pub fn status_speech_to_text_heading() -> String {
    format!(
        "{} {}",
        AgentPurpose::SpeechToText.emoji(),
        AgentPurpose::SpeechToText.heading()
    )
}

pub fn status_text_to_speech_heading() -> String {
    format!(
        "{} {}",
        AgentPurpose::TextToSpeech.emoji(),
        AgentPurpose::TextToSpeech.heading()
    )
}

pub fn status_image_generation_heading() -> String {
    format!(
        "{} {}",
        AgentPurpose::ImageGeneration.emoji(),
        AgentPurpose::ImageGeneration.heading()
    )
}

pub fn status_text_generation_entry_prefix_requirement_type(
    value: TextGenerationPrefixRequirementType,
    set_where: &str,
) -> String {
    format!("- 🗟 Prefix Requirement Type: `{}` ({})\n", value, set_where)
}

pub fn status_text_generation_entry_auto_usage(
    value: TextGenerationAutoUsage,
    set_where: &str,
) -> String {
    format!("- 🪄 Auto usage: `{}` ({})\n", value, set_where)
}

pub fn status_text_generation_entry_context_management(value: bool, set_where: &str) -> String {
    format!("- ♻️ Context management: `{}` ({})\n", value, set_where)
}

pub fn status_text_generation_entry_prompt(value: &str, set_where: &str) -> String {
    let value = value.trim();

    if value.is_empty() {
        format!("- ⌨️ Prompt: not using a prompt ({})\n", set_where)
    } else {
        format!("- ⌨️ Prompt ({}):\n\n{}\n\n", set_where, block_quote(value))
    }
}

pub fn status_text_generation_entry_temperature(value: Option<f32>, set_where: &str) -> String {
    let formatted = match value {
        Some(value) => format!("`{:.1}` ({})", value, set_where),
        None => "not set".to_string(),
    };

    format!("- 🌡️ Temperature: {}\n", formatted)
}

pub fn status_speech_to_text_entry_flow_type(
    value: SpeechToTextFlowType,
    set_where: &str,
) -> String {
    format!("- 🪄 Flow type: `{}` ({})\n", value, set_where)
}

pub fn status_speech_to_text_entry_msg_type_for_non_threaded_only_transcribed_messages(
    value: SpeechToTextMessageTypeForNonThreadedOnlyTranscribedMessages,
    set_where: &str,
) -> String {
    format!(
        "- 🪄 Message type for non-threaded only-transcribed messages: `{}` ({})\n",
        value, set_where
    )
}

pub fn status_speech_to_text_entry_language(value: Option<String>, set_where: &str) -> String {
    let formatted = match value {
        Some(value) => format!("`{}` ({})", value, set_where),
        None => "not set, using auto-detection".to_string(),
    };

    format!("- 🔤 Language: {}\n", formatted)
}

pub fn status_text_to_speech_entry_bot_msgs_flow_type(
    value: TextToSpeechBotMessagesFlowType,
    set_where: &str,
) -> String {
    format!(
        "- 🪄 Flow type for bot messages: `{}` ({})\n",
        value, set_where
    )
}

pub fn status_text_to_speech_entry_user_msgs_flow_type(
    value: TextToSpeechUserMessagesFlowType,
    set_where: &str,
) -> String {
    format!(
        "- 🪄 Flow type for user messages: `{}` ({})\n",
        value, set_where
    )
}

pub fn status_text_to_speech_entry_speed(value: Option<f32>, set_where: &str) -> String {
    let formatted = match value {
        Some(value) => format!("`{:.1}` ({})", value, set_where),
        None => "not set".to_string(),
    };

    format!("- ⚡ Speed: {}\n", formatted)
}

pub fn status_text_to_speech_entry_voice(value: Option<String>, set_where: &str) -> String {
    let formatted = match value {
        Some(value) => format!("`{}` ({})", value, set_where),
        None => "not set".to_string(),
    };

    format!("- 👫 Voice: {}\n", formatted)
}

pub fn status_entry_effective_agent_error() -> String {
    "- 🤖 Effective handler agent: error determining agent\n".to_string()
}

pub fn status_entry_effective_agent(
    value: &PublicIdentifier,
    source: AgentForPurposeDeterminationInfoConfigurationSource,
) -> String {
    let set_where = match source {
        AgentForPurposeDeterminationInfoConfigurationSource::Room => {
            status_badge_set_in_room_config()
        }
        AgentForPurposeDeterminationInfoConfigurationSource::Global => {
            status_badge_set_in_global_config()
        }
    };

    format!(
        "- 🤖 Effective handler agent: `{}` ({})\n",
        value, set_where
    )
}

pub fn status_badge_set_in_room_config() -> &'static str {
    "**📍 set in room**"
}

pub fn status_badge_set_in_global_config() -> &'static str {
    "**🌐 set globally**"
}

pub fn status_badge_using_hardcoded_default() -> &'static str {
    "📝 using hardcoded default"
}

pub fn status_badge_set_in_agent_config() -> &'static str {
    "🤖 set at the agent level"
}
