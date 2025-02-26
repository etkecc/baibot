use crate::agent::AgentPurpose;

pub fn heading() -> String {
    "🛠️ Configuration".to_owned()
}

pub fn intro_short() -> &'static str {
    "Various settings for this bot can be configured **📍 per-room** and **🌐 globally**."
}

pub fn intro_long() -> String {
    format!(
        "{}\n\n{}\n{}\n\n{}",
        intro_short(),
        "Room-specific configuration values override the global configuration.",
        "When no configuration values are set, the bot uses hardcoded defaults.",
        "In commands below, **replace the `CONFIG_TYPE` value** with either `room` (for room-specific configuration) or `global` (for global configuration)."
    )
}

pub fn status_heading() -> String {
    "📃 Status".to_owned()
}

pub fn status_intro(command_prefix: &str) -> String {
    format!("To **show a summary** of the configuration affecting the current room: `{command_prefix} config status`")
}

pub fn handlers_heading() -> String {
    "🤖 Handler Agents".to_owned()
}

pub fn handlers_intro_common() -> String {
    format!(
        "{}\n\n{}",
        "Different messages (text, audio, requests for image generation, etc.) in the room are handled differently and can potentially be served by different agents.",
        "When no specific agent is configured for a given purpose, the catch-all agent would be used.",
    )
}

pub fn handlers_intro_purposes() -> String {
    let mut message = String::new();

    message.push_str("The following purposes are available:");
    message.push('\n');

    for purpose in AgentPurpose::choices() {
        message.push_str(&format!(
            "\n- {} {}: {}",
            purpose.emoji(),
            purpose.as_str(),
            super::super::agent::purpose_howto(purpose),
        ));
    }

    message
}

pub fn handlers_show(command_prefix: &str) -> String {
    format!("**Show** the currently configured agent for the given purpose: `{command_prefix} config CONFIG_TYPE handler PURPOSE`")
}

pub fn handlers_set(command_prefix: &str) -> String {
    format!("**Set** the agent to be used for the given purpose: `{command_prefix} config CONFIG_TYPE set-handler PURPOSE AGENT_ID`")
}

pub fn handlers_unset(command_prefix: &str) -> String {
    format!("**Unset** the agent to be used for the given purpose: `{command_prefix} config CONFIG_TYPE set-handler PURPOSE`")
}

pub fn text_generation_heading() -> String {
    format!(
        "{} {}",
        AgentPurpose::TextGeneration.emoji(),
        AgentPurpose::TextGeneration.heading()
    )
}

pub fn text_generation_common() -> String {
    let text_generation_description =
        "Text Generation is the bot's ability to generate text based on the input it receives.";

    let input_types = format!(
        "This input may be received directly as text, or as audio (a voice message) transcribed to text by the bot itself (see {} {}).",
        AgentPurpose::SpeechToText.emoji(),
        AgentPurpose::SpeechToText.heading()
    );

    format!("{}\n{}", text_generation_description, input_types)
}

pub fn text_generation_prefix_requirement_type_heading() -> &'static str {
    "🗟 Prefix Requirement Type"
}

pub fn text_generation_prefix_requirement_type_intro() -> String {
    "Controls whether all messages trigger text generation or just those prefixed in a certain way."
        .to_owned()
}

pub fn text_generation_prefix_requirement_type_outro(bot_username: &str) -> String {
    format!("Regardless of the setting, the bot will always respond to **direct mentions** (e.g. `@{bot_username}`).")
}

pub fn text_generation_auto_usage_heading() -> &'static str {
    "🪄 Auto usage"
}

pub fn text_generation_auto_usage_intro() -> String {
    "Controls how automatic text-generation functions.".to_owned()
}

pub fn text_generation_context_management_heading() -> &'static str {
    "♻️ Context Management"
}

pub fn text_generation_context_management_intro() -> String {
    format!(
        "{}\n{}",
        "Controls the bot's ability to **intelligently drop old messages from the conversation context** when it gets too large.",
        "This feature relies on [tokenization](https://en.wikipedia.org/wiki/Large_language_model#Tokenization) performed by the [tiktoken-rs](https://github.com/zurawiki/tiktoken-rs) library which is [poorly well-maintained](https://github.com/zurawiki/tiktoken-rs/issues/50) and only works well for [OpenAI](./providers.md#openai) models.",
    )
}

pub fn text_generation_prompt_override_heading() -> &'static str {
    "⌨️ Prompt Override"
}

pub fn text_generation_prompt_override_intro() -> String {
    "Lets you override the [system prompt](https://huggingface.co/docs/transformers/en/tasks/prompting) parameter configured at the agent level.".to_string()
}

pub fn text_generation_temperature_override_heading() -> &'static str {
    "🌡️ Temperature Override"
}

pub fn text_generation_temperature_override_intro() -> String {
    "Lets you override the [temperature](https://blogs.novita.ai/what-are-large-language-model-settings-temperature-top-p-and-max-tokens/#what-is-llm-temperature) (randomness / creativity) parameter configured at the agent level.".to_string()
}

pub fn current_setting_show(command_prefix: &str, setting_path_parts: &str) -> String {
    format!(
        "**Show** the current setting: `{command_prefix} config CONFIG_TYPE {setting_path_parts}`"
    )
}

pub fn current_setting_set(command_prefix: &str, setting_path_parts: &str) -> String {
    format!("**Set**: `{command_prefix} config CONFIG_TYPE {setting_path_parts}`")
}

pub fn current_setting_unset(command_prefix: &str, setting_path_parts: &str) -> String {
    format!("**Unset**: `{command_prefix} config CONFIG_TYPE {setting_path_parts}`")
}

pub fn the_following_configuration_values_are_recognized(
    values: Vec<impl std::fmt::Display>,
) -> String {
    let values_with_backticks = values
        .iter()
        .map(|v| format!("`{}`", v))
        .collect::<Vec<String>>();

    format!(
        "The following configuration values are recognized: {}",
        values_with_backticks.join(", ")
    )
}

pub fn speech_to_text_heading() -> String {
    format!(
        "{} {}",
        AgentPurpose::SpeechToText.emoji(),
        AgentPurpose::SpeechToText.heading()
    )
}

pub fn speech_to_text_common() -> String {
    let intro = "Speech-to-Text is the bot's ability to **turn audio (voice) messages into text**.";

    let text_gen = format!(
        "The generated text can be used for {} {}, or not (transcription only).",
        AgentPurpose::TextGeneration.emoji(),
        AgentPurpose::TextGeneration.heading()
    );

    let text_to_speech = format!(
        "The bot may also turn the generated text response back into a voice message (see {} {}).",
        AgentPurpose::TextToSpeech.emoji(),
        AgentPurpose::TextToSpeech.heading()
    );

    format!("{}\n{}\n{}", intro, text_gen, text_to_speech)
}

pub fn speech_to_text_flow_type_heading() -> &'static str {
    "🪄 Flow Type"
}

pub fn speech_to_text_flow_type_intro() -> &'static str {
    "Controls how voice messages are handled."
}

pub fn speech_to_text_msg_type_for_non_threaded_only_transcribed_messages_heading() -> &'static str
{
    "🪄 Message Type for non-threaded only-transcribed messages"
}

pub fn speech_to_text_msg_type_for_non_threaded_only_transcribed_messages_intro() -> &'static str {
    "Controls how the transcribed text of voice messages is sent to the chat when Flow Type = `only_transcribe`."
}

pub fn speech_to_text_language_heading() -> &'static str {
    "🔤 Language"
}

pub fn speech_to_text_language_intro() -> &'static str {
    "Lets you specify the language of the input voice messages, to avoid using auto-detection.\nSupplying the input language using a 2-letter code (e.g. `ja`) as per [ISO-639-1](https://en.wikipedia.org/wiki/List_of_ISO_639-1_codes) may improve accuracy & latency.\n\nIf different users are using different languages, do not specify a language."
}

pub fn text_to_speech_heading() -> String {
    format!(
        "{} {}",
        AgentPurpose::TextToSpeech.emoji(),
        AgentPurpose::TextToSpeech.heading()
    )
}

pub fn text_to_speech_common() -> &'static str {
    "Text-to-Speech is the bot's ability to **turn text messages into voice messages**."
}

pub fn text_to_speech_bot_msgs_flow_type_heading() -> &'static str {
    "🪄 Bot Messages Flow Type"
}

pub fn text_to_speech_bot_msgs_flow_type_intro() -> String {
    "Controls how automatic text-to-speech functions for **messages sent by the bot**.".to_owned()
}

pub fn text_to_speech_user_msgs_flow_type_heading() -> &'static str {
    "🪄 User Messages Flow Type"
}

pub fn text_to_speech_user_msgs_flow_type_intro() -> String {
    "Controls how automatic text-to-speech functions for **messages sent by users**.\n**Only works when automatic text-generation is disabled** (see Text Generation / Auto usage).".to_owned()
}

pub fn text_to_speech_speed_override_heading() -> &'static str {
    "🗲 Speed override"
}

pub fn text_to_speech_speed_override_intro() -> String {
    format!(
        "{}\n{}",
        "Lets you speed up/down speech relative to the default speed (`1.0` when unset).",
        "Values typically range from `0.25` to `4.0`, but may vary depending on the selected model.",
    )
}

pub fn text_to_speech_voice_override_heading() -> &'static str {
    "👫 Voice override"
}

pub fn text_to_speech_voice_override_intro() -> String {
    format!(
        "{}\n\n{}",
        "Lets you change the default voice configured in the agent configuration.",
        "Possible values (e.g. `onyx`) depend on the model you're using. For example, for OpenAI's Whisper model, [these voices](https://platform.openai.com/docs/guides/text-to-speech/voice-options) are available.",
    )
}

pub fn image_generation_heading() -> String {
    format!(
        "{} {}",
        AgentPurpose::ImageGeneration.emoji(),
        AgentPurpose::ImageGeneration.heading()
    )
}

pub fn image_generation_common() -> &'static str {
    "Image-generation is the bot's ability to **generate images** based on text prompts.\n\nThis feature is not configurable at the moment."
}
