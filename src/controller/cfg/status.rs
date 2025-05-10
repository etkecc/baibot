use mxlink::MessageResponseType;

use crate::{
    Bot,
    agent::{
        AgentInstance, AgentPurpose, ControllerTrait, Manager as AgentManager, PublicIdentifier,
        utils::get_effective_agent_for_purpose,
    },
    entity::{
        MessageContext, RoomConfigContext,
        roomconfig::{RoomConfig, RoomSettingsHandler},
    },
    strings,
};

pub async fn handle(bot: &Bot, message_context: &MessageContext) -> anyhow::Result<()> {
    let mut message = String::new();

    let agent_manager = bot.agent_manager();

    let agents = agent_manager
        .available_room_agents_by_room_config_context(message_context.room_config_context());

    // Room handlers
    message.push_str(&generate_room_handlers_section(
        &message_context.room_config().settings.handler,
        &agents,
        bot.command_prefix(),
    ));
    message.push_str("\n\n");

    // Global handlers
    message.push_str(&generate_global_config_handlers_section(
        &message_context
            .global_config()
            .fallback_room_settings
            .handler,
        &agents,
        bot.command_prefix(),
    ));
    message.push_str("\n\n");

    // Agents
    message.push_str(&generate_room_agents_section(
        message_context.room_config(),
        &agents,
        bot.command_prefix(),
    ));
    message.push_str("\n\n");

    // Text Generation
    message.push_str(
        &generate_text_generation_section(agent_manager, message_context.room_config_context())
            .await,
    );
    message.push_str("\n\n");

    // Text-to-Speech
    message.push_str(
        &generate_text_to_speech_section(agent_manager, message_context.room_config_context())
            .await,
    );
    message.push_str("\n\n");

    // Speech-to-Text
    message.push_str(
        &generate_speech_to_text_section(agent_manager, message_context.room_config_context())
            .await,
    );
    message.push_str("\n\n");

    // Image Creation
    message.push_str(
        &generate_image_generation_section(agent_manager, message_context.room_config_context())
            .await,
    );
    message.push_str("\n\n");

    bot.messaging()
        .send_text_markdown_no_fail(
            message_context.room(),
            message,
            MessageResponseType::Reply(message_context.thread_info().root_event_id.clone()),
        )
        .await;

    Ok(())
}

fn generate_room_handlers_section(
    handler_config: &RoomSettingsHandler,
    agents: &[AgentInstance],
    command_prefix: &str,
) -> String {
    let mut message = String::new();

    message.push_str(
        format!(
            "## {}\n",
            strings::cfg::status_room_config_handlers_heading()
        )
        .as_str(),
    );
    message.push_str(strings::cfg::status_room_config_handlers_intro());
    message.push_str("\n\n");

    for purpose in AgentPurpose::choices() {
        message.push_str(&generate_handler_line_for_purpose(
            purpose,
            handler_config,
            agents,
            false,
        ));
        message.push('\n');
    }

    message.push_str("\n\n");

    message.push_str(strings::cfg::status_room_config_handlers_outro(command_prefix).as_str());

    message
}

fn generate_global_config_handlers_section(
    handler_config: &RoomSettingsHandler,
    agents: &[AgentInstance],
    command_prefix: &str,
) -> String {
    let mut message = String::new();

    message.push_str(
        format!(
            "## {}\n",
            strings::cfg::status_global_config_handlers_heading()
        )
        .as_str(),
    );
    message.push_str(strings::cfg::status_global_config_handlers_intro());
    message.push_str("\n\n");

    for purpose in AgentPurpose::choices() {
        message.push_str(&generate_handler_line_for_purpose(
            purpose,
            handler_config,
            agents,
            true,
        ));
        message.push('\n');
    }

    message.push_str("\n\n");

    message.push_str(strings::cfg::status_global_config_handlers_outro(command_prefix).as_str());

    message
}

fn generate_handler_line_for_purpose(
    purpose: &AgentPurpose,
    handler_config: &RoomSettingsHandler,
    agents: &[AgentInstance],
    is_for_global_config: bool,
) -> String {
    let agent_id = handler_config.get_by_purpose(*purpose);

    match agent_id {
        Some(agent_id) => {
            let agent = agents
                .iter()
                .find(|a| *a.identifier().as_string() == agent_id);

            strings::cfg::status_handler_line_agent_found(purpose, &agent_id, agent)
        }
        None => match purpose {
            AgentPurpose::CatchAll => {
                if is_for_global_config {
                    return strings::cfg::status_handler_line_catch_all_agent_not_set_globally();
                }

                strings::cfg::status_handler_line_catch_all_agent_not_set_in_room_default_to_global(
                )
            }
            _ => {
                if is_for_global_config {
                    return strings::cfg::status_handler_line_non_catch_all_agent_not_set_globally(
                        purpose,
                    );
                }

                strings::cfg::status_handler_line_non_catch_all_agent_not_set_in_room_default_to_global(
                        purpose,
                    )
            }
        },
    }
}

fn generate_room_agents_section(
    room_config: &RoomConfig,
    agents: &Vec<AgentInstance>,
    command_prefix: &str,
) -> String {
    let mut message = String::new();

    message.push_str(format!("## {}\n", strings::cfg::status_room_agents_heading()).as_str());

    if room_config.agents.is_empty() {
        message.push_str(strings::cfg::status_room_agents_empty());

        message.push_str("\n\n");

        message.push_str(&strings::help::learn_more_send_a_command(
            command_prefix,
            "agent",
        ));

        return message;
    }

    message.push_str(strings::cfg::status_room_agents_intro());
    message.push_str("\n\n");

    for agent in agents {
        let PublicIdentifier::DynamicRoomLocal(_) = agent.identifier() else {
            continue;
        };

        message.push_str(&format!(
            "- `{}` ({})\n",
            agent.identifier(),
            strings::agent::create_support_badges_text(agent.controller()),
        ));
    }

    message.push_str("\n\n");

    message.push_str(strings::cfg::status_room_agents_outro(command_prefix).as_str());

    message
}

async fn generate_text_generation_section(
    agent_manager: &AgentManager,
    room_config_context: &RoomConfigContext,
) -> String {
    let mut message = String::new();

    message.push_str(format!("## {}\n", strings::cfg::status_text_generation_heading()).as_str());

    let text_generation_agent_info = get_effective_agent_for_purpose(
        agent_manager,
        room_config_context,
        AgentPurpose::TextGeneration,
    )
    .await;

    // Effective agent

    let text_generation_agent = match text_generation_agent_info {
        Ok(text_generation_agent_info) => {
            message.push_str(&strings::cfg::status_entry_effective_agent(
                text_generation_agent_info.instance.identifier(),
                text_generation_agent_info.configuration_source,
            ));

            Some(text_generation_agent_info.instance)
        }
        Err(err) => {
            tracing::error!(?err, "Failed to determine text-generation agent");
            message.push_str(&strings::cfg::status_entry_effective_agent_error());
            None
        }
    };

    // Prefix requirement type

    let effective_prefix_requirement_type =
        room_config_context.text_generation_prefix_requirement_type();
    let room_config_prefix_requirement_type = room_config_context
        .room_config
        .settings
        .text_generation
        .prefix_requirement_type;
    let global_config_prefix_requirement_type = room_config_context
        .global_config
        .fallback_room_settings
        .text_generation
        .prefix_requirement_type;

    let prefix_requirement_type_set_where = if room_config_prefix_requirement_type.is_some() {
        strings::cfg::status_badge_set_in_room_config()
    } else if global_config_prefix_requirement_type.is_some() {
        strings::cfg::status_badge_set_in_global_config()
    } else {
        strings::cfg::status_badge_using_hardcoded_default()
    };

    message.push_str(
        &strings::cfg::status_text_generation_entry_prefix_requirement_type(
            effective_prefix_requirement_type,
            prefix_requirement_type_set_where,
        ),
    );

    // Auto usage

    let effective_auto_usage = room_config_context.auto_text_generation_usage();
    let room_config_auto_usage = room_config_context
        .room_config
        .settings
        .text_generation
        .auto_usage;
    let global_config_auto_usage = room_config_context
        .global_config
        .fallback_room_settings
        .text_generation
        .auto_usage;

    let auto_usage_set_where = if room_config_auto_usage.is_some() {
        strings::cfg::status_badge_set_in_room_config()
    } else if global_config_auto_usage.is_some() {
        strings::cfg::status_badge_set_in_global_config()
    } else {
        strings::cfg::status_badge_using_hardcoded_default()
    };

    message.push_str(&strings::cfg::status_text_generation_entry_auto_usage(
        effective_auto_usage,
        auto_usage_set_where,
    ));

    // Context Management

    let effective_context_management =
        room_config_context.text_generation_context_management_enabled();
    let room_config_context_management = room_config_context
        .room_config
        .settings
        .text_generation
        .context_management_enabled;
    let global_config_context_management = room_config_context
        .global_config
        .fallback_room_settings
        .text_generation
        .context_management_enabled;

    let context_management_set_where = if room_config_context_management.is_some() {
        strings::cfg::status_badge_set_in_room_config()
    } else if global_config_context_management.is_some() {
        strings::cfg::status_badge_set_in_global_config()
    } else {
        strings::cfg::status_badge_using_hardcoded_default()
    };

    message.push_str(
        &strings::cfg::status_text_generation_entry_context_management(
            effective_context_management,
            context_management_set_where,
        ),
    );

    // Prompt override

    let text_agent_prompt = if let Some(text_generation_agent) = &text_generation_agent {
        text_generation_agent.controller().text_generation_prompt()
    } else {
        None
    };

    let room_config_prompt_override = room_config_context
        .room_config
        .settings
        .text_generation
        .prompt_override
        .clone();
    let global_config_prompt_override = room_config_context
        .global_config
        .fallback_room_settings
        .text_generation
        .prompt_override
        .clone();

    let (prompt, prompt_set_where) =
        if let Some(room_config_prompt_override) = room_config_prompt_override {
            (
                room_config_prompt_override,
                strings::cfg::status_badge_set_in_room_config(),
            )
        } else if let Some(global_config_prompt_override) = global_config_prompt_override {
            (
                global_config_prompt_override,
                strings::cfg::status_badge_set_in_global_config(),
            )
        } else {
            (
                text_agent_prompt.unwrap_or("".to_owned()),
                strings::cfg::status_badge_set_in_agent_config(),
            )
        };

    message.push_str(&strings::cfg::status_text_generation_entry_prompt(
        &prompt,
        prompt_set_where,
    ));

    // Temperature

    let text_agent_temperature = if let Some(text_generation_agent) = &text_generation_agent {
        text_generation_agent
            .controller()
            .text_generation_temperature()
    } else {
        None
    };

    let room_config_temperature_override = room_config_context
        .room_config
        .settings
        .text_generation
        .temperature_override;
    let global_config_temperature_override = room_config_context
        .global_config
        .fallback_room_settings
        .text_generation
        .temperature_override;

    let (effective_temperature, set_where) = if let Some(room_config_temperature_override) =
        room_config_temperature_override
    {
        (
            Some(room_config_temperature_override),
            strings::cfg::status_badge_set_in_room_config(),
        )
    } else if let Some(global_config_temperature_override) = global_config_temperature_override {
        (
            Some(global_config_temperature_override),
            strings::cfg::status_badge_set_in_global_config(),
        )
    } else {
        (
            text_agent_temperature,
            strings::cfg::status_badge_set_in_agent_config(),
        )
    };

    message.push_str(&strings::cfg::status_text_generation_entry_temperature(
        effective_temperature,
        set_where,
    ));

    message
}

async fn generate_speech_to_text_section(
    agent_manager: &AgentManager,
    room_config_context: &RoomConfigContext,
) -> String {
    let mut message = String::new();

    message.push_str(format!("## {}\n", strings::cfg::status_speech_to_text_heading()).as_str());

    let speech_to_text_agent_info = get_effective_agent_for_purpose(
        agent_manager,
        room_config_context,
        AgentPurpose::SpeechToText,
    )
    .await;

    // Effective agent

    match speech_to_text_agent_info {
        Ok(speech_to_text_agent_info) => {
            message.push_str(&strings::cfg::status_entry_effective_agent(
                speech_to_text_agent_info.instance.identifier(),
                speech_to_text_agent_info.configuration_source,
            ));
        }
        Err(err) => {
            tracing::error!(?err, "Failed to determine speech-to-text agent");
            message.push_str(&strings::cfg::status_entry_effective_agent_error());
        }
    };

    // Flow type

    let effective_flow_type = room_config_context.speech_to_text_flow_type();
    let room_config_flow_type = room_config_context
        .room_config
        .settings
        .speech_to_text
        .flow_type;
    let global_config_flow_type = room_config_context
        .global_config
        .fallback_room_settings
        .speech_to_text
        .flow_type;

    let flow_type_set_where = if room_config_flow_type.is_some() {
        strings::cfg::status_badge_set_in_room_config()
    } else if global_config_flow_type.is_some() {
        strings::cfg::status_badge_set_in_global_config()
    } else {
        strings::cfg::status_badge_using_hardcoded_default()
    };

    message.push_str(&strings::cfg::status_speech_to_text_entry_flow_type(
        effective_flow_type,
        flow_type_set_where,
    ));

    // Msg Type For Non Threaded Only Transcribed Messages

    let effective_msg_type_for_non_threaded_only_transcribed_messages =
        room_config_context.speech_to_text_msg_type_for_non_threaded_only_transcribed_messages();
    let room_config_msg_type_for_non_threaded_only_transcribed_messages = room_config_context
        .room_config
        .settings
        .speech_to_text
        .msg_type_for_non_threaded_only_transcribed_messages;
    let global_config_msg_type_for_non_threaded_only_transcribed_messages = room_config_context
        .global_config
        .fallback_room_settings
        .speech_to_text
        .msg_type_for_non_threaded_only_transcribed_messages;

    let msg_type_for_non_threaded_only_transcribed_messages_set_where =
        if room_config_msg_type_for_non_threaded_only_transcribed_messages.is_some() {
            strings::cfg::status_badge_set_in_room_config()
        } else if global_config_msg_type_for_non_threaded_only_transcribed_messages.is_some() {
            strings::cfg::status_badge_set_in_global_config()
        } else {
            strings::cfg::status_badge_using_hardcoded_default()
        };

    message.push_str(&strings::cfg::status_speech_to_text_entry_msg_type_for_non_threaded_only_transcribed_messages(
        effective_msg_type_for_non_threaded_only_transcribed_messages,
        msg_type_for_non_threaded_only_transcribed_messages_set_where,
    ));

    // Language

    let effective_language = room_config_context.speech_to_text_language();
    let room_config_language = &room_config_context
        .room_config
        .settings
        .speech_to_text
        .language;
    let global_config_language = &room_config_context
        .global_config
        .fallback_room_settings
        .speech_to_text
        .language;

    let language_set_where = if room_config_language.is_some() {
        strings::cfg::status_badge_set_in_room_config()
    } else if global_config_language.is_some() {
        strings::cfg::status_badge_set_in_global_config()
    } else {
        strings::cfg::status_badge_using_hardcoded_default()
    };

    message.push_str(&strings::cfg::status_speech_to_text_entry_language(
        effective_language,
        language_set_where,
    ));

    message
}

async fn generate_text_to_speech_section(
    agent_manager: &AgentManager,
    room_config_context: &RoomConfigContext,
) -> String {
    let mut message = String::new();

    message.push_str(format!("## {}\n", strings::cfg::status_text_to_speech_heading()).as_str());

    let text_to_speech_agent_info = get_effective_agent_for_purpose(
        agent_manager,
        room_config_context,
        AgentPurpose::TextToSpeech,
    )
    .await;

    // Effective agent

    let text_to_speech_agent = match text_to_speech_agent_info {
        Ok(text_to_speech_agent_info) => {
            message.push_str(&strings::cfg::status_entry_effective_agent(
                text_to_speech_agent_info.instance.identifier(),
                text_to_speech_agent_info.configuration_source,
            ));
            Some(text_to_speech_agent_info.instance)
        }
        Err(err) => {
            tracing::error!(?err, "Failed to determine text-to-speech agent");
            message.push_str(&strings::cfg::status_entry_effective_agent_error());
            None
        }
    };

    // Bot messages flow type

    let effective_bot_messages_tts_flow_type =
        room_config_context.text_to_speech_bot_messages_flow_type();
    let room_config_bot_messages_tts_flow_type = room_config_context
        .room_config
        .settings
        .text_to_speech
        .bot_msgs_flow_type;
    let global_config_bot_messages_tts_flow_type = room_config_context
        .global_config
        .fallback_room_settings
        .text_to_speech
        .bot_msgs_flow_type;

    let bot_messages_tts_flow_type_set_where = if room_config_bot_messages_tts_flow_type.is_some() {
        strings::cfg::status_badge_set_in_room_config()
    } else if global_config_bot_messages_tts_flow_type.is_some() {
        strings::cfg::status_badge_set_in_global_config()
    } else {
        strings::cfg::status_badge_using_hardcoded_default()
    };

    message.push_str(
        &strings::cfg::status_text_to_speech_entry_bot_msgs_flow_type(
            effective_bot_messages_tts_flow_type,
            bot_messages_tts_flow_type_set_where,
        ),
    );

    // User messages flow type

    let effective_user_messages_tts_flow_type =
        room_config_context.text_to_speech_user_messages_flow_type();
    let room_config_user_messages_tts_flow_type = room_config_context
        .room_config
        .settings
        .text_to_speech
        .user_msgs_flow_type;
    let global_config_user_messages_tts_flow_type = room_config_context
        .global_config
        .fallback_room_settings
        .text_to_speech
        .user_msgs_flow_type;

    let user_messages_tts_flow_type_set_where = if room_config_user_messages_tts_flow_type.is_some()
    {
        strings::cfg::status_badge_set_in_room_config()
    } else if global_config_user_messages_tts_flow_type.is_some() {
        strings::cfg::status_badge_set_in_global_config()
    } else {
        strings::cfg::status_badge_using_hardcoded_default()
    };

    message.push_str(
        &strings::cfg::status_text_to_speech_entry_user_msgs_flow_type(
            effective_user_messages_tts_flow_type,
            user_messages_tts_flow_type_set_where,
        ),
    );

    // Speed

    let agent_speed = if let Some(text_to_speech_agent) = &text_to_speech_agent {
        text_to_speech_agent.controller().text_to_speech_speed()
    } else {
        None
    };

    let room_config_speed_override = room_config_context
        .room_config
        .settings
        .text_to_speech
        .speed_override;
    let global_config_speed_override = room_config_context
        .global_config
        .fallback_room_settings
        .text_to_speech
        .speed_override;

    let (effective_speed, set_where) =
        if let Some(room_config_speed_override) = room_config_speed_override {
            (
                Some(room_config_speed_override),
                strings::cfg::status_badge_set_in_room_config(),
            )
        } else if let Some(global_config_speed_override) = global_config_speed_override {
            (
                Some(global_config_speed_override),
                strings::cfg::status_badge_set_in_global_config(),
            )
        } else if agent_speed.is_some() {
            (
                agent_speed,
                strings::cfg::status_badge_set_in_agent_config(),
            )
        } else {
            (None, strings::cfg::status_badge_using_hardcoded_default())
        };

    message.push_str(&strings::cfg::status_text_to_speech_entry_speed(
        effective_speed,
        set_where,
    ));

    // Voice

    let agent_voice = if let Some(text_to_speech_agent) = &text_to_speech_agent {
        text_to_speech_agent.controller().text_to_speech_voice()
    } else {
        None
    };

    let room_config_voice_override = room_config_context
        .room_config
        .settings
        .text_to_speech
        .voice_override
        .clone();
    let global_config_voice_override = room_config_context
        .global_config
        .fallback_room_settings
        .text_to_speech
        .voice_override
        .clone();

    let (effective_voice, set_where) =
        if let Some(room_config_voice_override) = room_config_voice_override {
            (
                Some(room_config_voice_override),
                strings::cfg::status_badge_set_in_room_config(),
            )
        } else if let Some(global_config_voice_override) = global_config_voice_override {
            (
                Some(global_config_voice_override),
                strings::cfg::status_badge_set_in_global_config(),
            )
        } else if agent_voice.is_some() {
            (
                agent_voice,
                strings::cfg::status_badge_set_in_agent_config(),
            )
        } else {
            (None, strings::cfg::status_badge_using_hardcoded_default())
        };

    message.push_str(&strings::cfg::status_text_to_speech_entry_voice(
        effective_voice,
        set_where,
    ));

    message
}

async fn generate_image_generation_section(
    agent_manager: &AgentManager,
    room_config_context: &RoomConfigContext,
) -> String {
    let mut message = String::new();

    message.push_str(format!("## {}\n", strings::cfg::status_image_generation_heading()).as_str());

    let image_generation_agent_info = get_effective_agent_for_purpose(
        agent_manager,
        room_config_context,
        AgentPurpose::ImageGeneration,
    )
    .await;

    // Effective agent

    let _image_generation_agent = match image_generation_agent_info {
        Ok(image_generation_agent_info) => {
            message.push_str(&strings::cfg::status_entry_effective_agent(
                image_generation_agent_info.instance.identifier(),
                image_generation_agent_info.configuration_source,
            ));
            Some(image_generation_agent_info.instance)
        }
        Err(err) => {
            tracing::error!(?err, "Failed to determine image generation agent");
            message.push_str(&strings::cfg::status_entry_effective_agent_error());
            None
        }
    };

    message
}
