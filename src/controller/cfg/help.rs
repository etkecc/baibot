use mxlink::MessageResponseType;

use crate::{
    entity::{
        roomconfig::{
            SpeechToTextFlowType, SpeechToTextMessageTypeForNonThreadedOnlyTranscribedMessages,
            TextGenerationAutoUsage, TextGenerationPrefixRequirementType,
            TextToSpeechBotMessagesFlowType, TextToSpeechUserMessagesFlowType,
        },
        MessageContext,
    },
    strings, Bot,
};

pub async fn handle(bot: &Bot, message_context: &MessageContext) -> anyhow::Result<()> {
    let mut message = String::new();
    message.push_str(&build_section_intro());
    message.push_str("\n\n");
    message.push_str("\n---\n");
    message.push_str(&build_section_status(bot.command_prefix()));
    message.push_str("\n\n");
    message.push_str("\n---\n");
    message.push_str(&build_section_handlers(bot.command_prefix()));

    message.push_str("\n\n");
    message.push_str("\n---\n");
    message.push_str(&build_section_text_generation(
        bot.command_prefix(),
        bot.user_id().localpart(),
    ));

    message.push_str("\n\n");
    message.push_str("\n---\n");
    message.push_str(&build_section_text_to_speech(bot.command_prefix()));

    message.push_str("\n\n");
    message.push_str("\n---\n");
    message.push_str(&build_section_speech_to_text(bot.command_prefix()));

    message.push_str("\n\n");
    message.push_str("\n---\n");
    message.push_str(&build_section_image_generation());

    bot.messaging()
        .send_text_markdown_no_fail(
            message_context.room(),
            message,
            MessageResponseType::Reply(message_context.thread_info().root_event_id.clone()),
        )
        .await;

    Ok(())
}

fn build_section_intro() -> String {
    let mut message = String::new();

    message.push_str(&format!("## {}", strings::help::cfg::heading()));
    message.push_str("\n\n");
    message.push_str(&strings::help::cfg::intro_long());

    message
}

fn build_section_status(command_prefix: &str) -> String {
    let mut message = String::new();

    message.push_str(&format!("### {}", strings::help::cfg::status_heading()));
    message.push_str("\n\n");
    message.push_str(&strings::help::cfg::status_intro(command_prefix));

    message
}

fn build_section_handlers(command_prefix: &str) -> String {
    let mut message = String::new();

    message.push_str(&format!("### {}", strings::help::cfg::handlers_heading()));
    message.push_str("\n\n");
    message.push_str(&strings::help::cfg::handlers_intro_common());
    message.push('\n');
    message.push_str(&strings::help::cfg::handlers_intro_purposes());
    message.push_str("\n\n");

    message.push_str(strings::help::available_commands_intro());
    message.push('\n');

    message.push_str(&format!(
        "- {}",
        strings::help::cfg::handlers_show(command_prefix)
    ));
    message.push('\n');
    message.push_str(&format!(
        "- {}",
        strings::help::cfg::handlers_set(command_prefix)
    ));
    message.push('\n');
    message.push_str(&format!(
        "- {}",
        strings::help::cfg::handlers_unset(command_prefix)
    ));

    message
}

fn build_section_text_generation(command_prefix: &str, bot_username: &str) -> String {
    let mut message = String::new();

    message.push_str(&format!(
        "### {}",
        strings::help::cfg::text_generation_heading()
    ));
    message.push_str("\n\n");
    message.push_str(&strings::help::cfg::text_generation_common());
    message.push_str("\n\n");

    // Prefix requirement type

    message.push_str(&format!(
        "#### {}",
        strings::help::cfg::text_generation_prefix_requirement_type_heading()
    ));
    message.push_str("\n\n");
    message.push_str(&strings::help::cfg::text_generation_prefix_requirement_type_intro());
    message.push('\n');
    message.push_str(
        &strings::help::cfg::the_following_configuration_values_are_recognized(
            TextGenerationPrefixRequirementType::choices(),
        ),
    );
    message.push_str("\n\n");
    message
        .push_str(&strings::help::cfg::text_generation_prefix_requirement_type_outro(bot_username));
    message.push('\n');
    message.push_str(&format!(
        "- {}",
        &strings::help::cfg::current_setting_show(
            command_prefix,
            "text-generation prefix-requirement-type"
        )
    ));
    message.push('\n');
    message.push_str(&format!(
        "- {}",
        &strings::help::cfg::current_setting_set(
            command_prefix,
            "text-generation set-prefix-requirement-type VALUE"
        )
    ));
    message.push('\n');
    message.push_str(&format!(
        "- {}",
        &strings::help::cfg::current_setting_unset(
            command_prefix,
            "text-generation set-prefix-requirement-type"
        )
    ));
    message.push_str("\n\n");

    // Auto Usage

    message.push_str(&format!(
        "#### {}",
        strings::help::cfg::text_generation_auto_usage_heading()
    ));
    message.push_str("\n\n");
    message.push_str(&strings::help::cfg::text_generation_auto_usage_intro());
    message.push('\n');
    message.push_str(
        &strings::help::cfg::the_following_configuration_values_are_recognized(
            TextGenerationAutoUsage::choices(),
        ),
    );
    message.push_str("\n\n");
    message.push_str(&format!(
        "- {}",
        &strings::help::cfg::current_setting_show(command_prefix, "text-generation auto-usage")
    ));
    message.push('\n');
    message.push_str(&format!(
        "- {}",
        &strings::help::cfg::current_setting_set(
            command_prefix,
            "text-generation set-auto-usage VALUE"
        )
    ));
    message.push('\n');
    message.push_str(&format!(
        "- {}",
        &strings::help::cfg::current_setting_unset(
            command_prefix,
            "text-generation set-auto-usage"
        )
    ));
    message.push_str("\n\n");

    // Context Management

    message.push_str(&format!(
        "#### {}",
        strings::help::cfg::text_generation_context_management_heading()
    ));
    message.push_str("\n\n");
    message.push_str(&strings::help::cfg::text_generation_context_management_intro());
    message.push('\n');
    message.push_str(
        &strings::help::cfg::the_following_configuration_values_are_recognized(vec![true, false]),
    );
    message.push_str("\n\n");
    message.push_str(&format!(
        "- {}",
        &strings::help::cfg::current_setting_show(
            command_prefix,
            "text-generation context-management-enabled"
        )
    ));
    message.push('\n');
    message.push_str(&format!(
        "- {}",
        &strings::help::cfg::current_setting_set(
            command_prefix,
            "text-generation set-context-management-enabled VALUE"
        )
    ));
    message.push('\n');
    message.push_str(&format!(
        "- {}",
        &strings::help::cfg::current_setting_unset(
            command_prefix,
            "text-generation set-context-management-enabled"
        )
    ));
    message.push_str("\n\n");

    // Prompt override

    message.push_str(&format!(
        "#### {}",
        strings::help::cfg::text_generation_prompt_override_heading()
    ));
    message.push_str("\n\n");
    message.push_str(&strings::help::cfg::text_generation_prompt_override_intro());
    message.push_str("\n\n");
    message.push_str(&format!(
        "- {}",
        &strings::help::cfg::current_setting_show(
            command_prefix,
            "text-generation prompt-override"
        )
    ));
    message.push('\n');
    message.push_str(&format!(
        "- {}",
        &strings::help::cfg::current_setting_set(
            command_prefix,
            "text-generation set-prompt-override VALUE"
        )
    ));
    message.push('\n');
    message.push_str(&format!(
        "- {}",
        &strings::help::cfg::current_setting_unset(
            command_prefix,
            "text-generation set-prompt-override"
        )
    ));
    message.push_str("\n\n");

    // Speed override

    message.push_str(&format!(
        "#### {}",
        strings::help::cfg::text_generation_temperature_override_heading()
    ));
    message.push_str("\n\n");
    message.push_str(&strings::help::cfg::text_generation_temperature_override_intro());
    message.push_str("\n\n");
    message.push_str(&format!(
        "- {}",
        &strings::help::cfg::current_setting_show(
            command_prefix,
            "text-generation temperature-override"
        )
    ));
    message.push('\n');
    message.push_str(&format!(
        "- {}",
        &strings::help::cfg::current_setting_set(
            command_prefix,
            "text-generation set-temperature-override VALUE"
        )
    ));
    message.push('\n');
    message.push_str(&format!(
        "- {}",
        &strings::help::cfg::current_setting_unset(
            command_prefix,
            "text-generation set-temperature-override"
        )
    ));

    message
}

fn build_section_speech_to_text(command_prefix: &str) -> String {
    let mut message = String::new();

    message.push_str(&format!(
        "### {}",
        strings::help::cfg::speech_to_text_heading()
    ));
    message.push_str("\n\n");
    message.push_str(&strings::help::cfg::speech_to_text_common());
    message.push_str("\n\n");

    // Flow Type

    message.push_str(&format!(
        "#### {}",
        strings::help::cfg::speech_to_text_flow_type_heading()
    ));
    message.push_str("\n\n");
    message.push_str(strings::help::cfg::speech_to_text_flow_type_intro());
    message.push('\n');
    message.push_str(
        &strings::help::cfg::the_following_configuration_values_are_recognized(
            SpeechToTextFlowType::choices(),
        ),
    );
    message.push_str("\n\n");
    message.push_str(&format!(
        "- {}",
        &strings::help::cfg::current_setting_show(command_prefix, "speech-to-text flow-type")
    ));
    message.push('\n');
    message.push_str(&format!(
        "- {}",
        &strings::help::cfg::current_setting_set(
            command_prefix,
            "speech-to-text set-flow-type VALUE"
        )
    ));
    message.push('\n');
    message.push_str(&format!(
        "- {}",
        &strings::help::cfg::current_setting_unset(command_prefix, "speech-to-text set-flow-type")
    ));
    message.push_str("\n\n");

    // Msg Type For Non Threaded Only Transcribed Messages

    message.push_str(&format!(
        "#### {}",
        strings::help::cfg::speech_to_text_msg_type_for_non_threaded_only_transcribed_messages_heading()
    ));
    message.push_str("\n\n");
    message.push_str(strings::help::cfg::speech_to_text_msg_type_for_non_threaded_only_transcribed_messages_intro());
    message.push('\n');
    message.push_str(
        &strings::help::cfg::the_following_configuration_values_are_recognized(
            SpeechToTextMessageTypeForNonThreadedOnlyTranscribedMessages::choices(),
        ),
    );
    message.push_str("\n\n");
    message.push_str(&format!(
        "- {}",
        &strings::help::cfg::current_setting_show(
            command_prefix,
            "speech-to-text msg-type-for-non-threaded-only-transcribed-messages"
        )
    ));
    message.push('\n');
    message.push_str(&format!(
        "- {}",
        &strings::help::cfg::current_setting_set(
            command_prefix,
            "speech-to-text set-msg-type-for-non-threaded-only-transcribed-messages VALUE"
        )
    ));
    message.push('\n');
    message.push_str(&format!(
        "- {}",
        &strings::help::cfg::current_setting_unset(
            command_prefix,
            "speech-to-text set-msg-type-for-non-threaded-only-transcribed-messages"
        )
    ));
    message.push_str("\n\n");

    // Language

    message.push_str(&format!(
        "#### {}",
        strings::help::cfg::speech_to_text_language_heading()
    ));
    message.push_str("\n\n");
    message.push_str(strings::help::cfg::speech_to_text_language_intro());
    message.push_str("\n\n");
    message.push_str(&format!(
        "- {}",
        &strings::help::cfg::current_setting_show(command_prefix, "speech-to-text language")
    ));
    message.push('\n');
    message.push_str(&format!(
        "- {}",
        &strings::help::cfg::current_setting_set(
            command_prefix,
            "speech-to-text set-language VALUE"
        )
    ));
    message.push('\n');
    message.push_str(&format!(
        "- {}",
        &strings::help::cfg::current_setting_unset(command_prefix, "speech-to-text set-language")
    ));
    message.push_str("\n\n");

    message
}

fn build_section_text_to_speech(command_prefix: &str) -> String {
    let mut message = String::new();

    message.push_str(&format!(
        "### {}",
        strings::help::cfg::text_to_speech_heading()
    ));
    message.push_str("\n\n");
    message.push_str(strings::help::cfg::text_to_speech_common());
    message.push_str("\n\n");

    // Bot Messages Flow Type

    message.push_str(&format!(
        "#### {}",
        strings::help::cfg::text_to_speech_bot_msgs_flow_type_heading()
    ));
    message.push_str("\n\n");
    message.push_str(&strings::help::cfg::text_to_speech_bot_msgs_flow_type_intro());
    message.push('\n');
    message.push_str(
        &strings::help::cfg::the_following_configuration_values_are_recognized(
            TextToSpeechBotMessagesFlowType::choices(),
        ),
    );
    message.push_str("\n\n");
    message.push_str(&format!(
        "- {}",
        &strings::help::cfg::current_setting_show(
            command_prefix,
            "text-to-speech bot-msgs-flow-type"
        )
    ));
    message.push('\n');
    message.push_str(&format!(
        "- {}",
        &strings::help::cfg::current_setting_set(
            command_prefix,
            "text-to-speech set-bot-msgs-flow-type VALUE"
        )
    ));
    message.push('\n');
    message.push_str(&format!(
        "- {}",
        &strings::help::cfg::current_setting_unset(
            command_prefix,
            "text-to-speech set-bot-msgs-flow-type"
        )
    ));
    message.push_str("\n\n");

    // User Messages Flow Type

    message.push_str(&format!(
        "#### {}",
        strings::help::cfg::text_to_speech_user_msgs_flow_type_heading()
    ));
    message.push_str("\n\n");
    message.push_str(&strings::help::cfg::text_to_speech_user_msgs_flow_type_intro());
    message.push('\n');
    message.push_str(
        &strings::help::cfg::the_following_configuration_values_are_recognized(
            TextToSpeechUserMessagesFlowType::choices(),
        ),
    );
    message.push_str("\n\n");
    message.push_str(&format!(
        "- {}",
        &strings::help::cfg::current_setting_show(
            command_prefix,
            "text-to-speech user-msgs-flow-type"
        )
    ));
    message.push('\n');
    message.push_str(&format!(
        "- {}",
        &strings::help::cfg::current_setting_set(
            command_prefix,
            "text-to-speech set-user-msgs-flow-type VALUE"
        )
    ));
    message.push('\n');
    message.push_str(&format!(
        "- {}",
        &strings::help::cfg::current_setting_unset(
            command_prefix,
            "text-to-speech set-user-msgs-flow-type"
        )
    ));
    message.push_str("\n\n");

    // Speed override

    message.push_str(&format!(
        "#### {}",
        strings::help::cfg::text_to_speech_speed_override_heading()
    ));
    message.push_str("\n\n");
    message.push_str(&strings::help::cfg::text_to_speech_speed_override_intro());
    message.push_str("\n\n");
    message.push_str(&format!(
        "- {}",
        &strings::help::cfg::current_setting_show(command_prefix, "text-to-speech speed-override")
    ));
    message.push('\n');
    message.push_str(&format!(
        "- {}",
        &strings::help::cfg::current_setting_set(
            command_prefix,
            "text-to-speech set-speed-override VALUE"
        )
    ));
    message.push('\n');
    message.push_str(&format!(
        "- {}",
        &strings::help::cfg::current_setting_unset(
            command_prefix,
            "text-to-speech set-speed-override"
        )
    ));
    message.push_str("\n\n");

    // Voice override

    message.push_str(&format!(
        "#### {}",
        strings::help::cfg::text_to_speech_voice_override_heading()
    ));
    message.push_str("\n\n");
    message.push_str(&strings::help::cfg::text_to_speech_voice_override_intro());
    message.push_str("\n\n");
    message.push_str(&format!(
        "- {}",
        &strings::help::cfg::current_setting_show(command_prefix, "text-to-speech voice-override")
    ));
    message.push('\n');
    message.push_str(&format!(
        "- {}",
        &strings::help::cfg::current_setting_set(
            command_prefix,
            "text-to-speech set-voice-override VALUE"
        )
    ));
    message.push('\n');
    message.push_str(&format!(
        "- {}",
        &strings::help::cfg::current_setting_unset(
            command_prefix,
            "text-to-speech set-voice-override"
        )
    ));
    message.push_str("\n\n");

    message
}

fn build_section_image_generation() -> String {
    let mut message = String::new();

    message.push_str(&format!(
        "### {}",
        strings::help::cfg::image_generation_heading()
    ));
    message.push_str("\n\n");
    message.push_str(strings::help::cfg::image_generation_common());

    message
}
