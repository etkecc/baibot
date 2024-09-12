#[cfg(test)]
mod tests;

use mxlink::matrix_sdk::ruma::OwnedUserId;

use super::chat_completion::ChatCompletionControllerType;
use crate::{
    entity::{
        roomconfig::TextGenerationPrefixRequirementType, MessageContext, MessagePayload,
        ThreadContextFirstMessage,
    },
    strings,
};

use super::ControllerType;

pub fn determine_controller(
    command_prefix: &str,
    first_thread_message: &ThreadContextFirstMessage,
    message_context: &MessageContext,
    bot_user_id: &OwnedUserId,
    bot_display_name: &Option<String>,
) -> ControllerType {
    match &first_thread_message.payload {
        MessagePayload::Text(text_message_content) => {
            let prefix_requirement_type = message_context
                .room_config_context()
                .text_generation_prefix_requirement_type();

            determine_text_controller(
                command_prefix,
                &text_message_content.body,
                prefix_requirement_type,
                first_thread_message.is_mentioning_bot,
                bot_user_id,
                bot_display_name,
            )
        }
        MessagePayload::Encrypted(thread_info) => {
            if thread_info.is_thread_root_only() {
                ControllerType::Error(strings::error::message_is_encrypted().to_owned())
            } else {
                ControllerType::ErrorInThread(
                    strings::error::first_message_in_thread_is_encrypted().to_owned(),
                    thread_info.clone(),
                )
            }
        }
        MessagePayload::Audio(_) => {
            ControllerType::ChatCompletion(ChatCompletionControllerType::ViaAudio)
        }
        MessagePayload::Reaction { .. } => {
            panic!("Handling reaction as first message in thread does not make sense")
        }
    }
}

fn determine_text_controller(
    command_prefix: &str,
    text: &str,
    room_text_generation_prefix_requirement_type: TextGenerationPrefixRequirementType,
    is_mentioning_bot: bool,
    bot_user_id: &OwnedUserId,
    bot_display_name: &Option<String>,
) -> ControllerType {
    let text = text.trim();

    if text.starts_with(&format!("{command_prefix} help")) || text == command_prefix {
        return ControllerType::Help;
    }

    if let Some(remaining) = text.strip_prefix(&format!("{command_prefix} access")) {
        return super::access::determine_controller(remaining.trim());
    }

    if let Some(remaining) = text.strip_prefix(&format!("{command_prefix} provider")) {
        return super::provider::determine_controller(remaining.trim());
    }

    if let Some(remaining) = text.strip_prefix(&format!("{command_prefix} agent")) {
        return super::agent::determine_controller(command_prefix, remaining.trim());
    }

    if let Some(remaining) = text.strip_prefix(&format!("{command_prefix} config")) {
        return super::cfg::determine_controller(remaining.trim());
    }

    if let Some(prompt) = text.strip_prefix(&format!("{command_prefix} image")) {
        return ControllerType::ImageGeneration(prompt.trim().to_owned());
    }

    if let Some(prompt) = text.strip_prefix(&format!("{command_prefix} sticker")) {
        return ControllerType::StickerGeneration(prompt.trim().to_owned());
    }

    if let Some(remaining) = text.strip_prefix(&format!("{command_prefix} usage")) {
        return super::usage::determine_controller(remaining.trim());
    }

    // Regular text message that does not match any command.
    // If it mentions the bot, it's a chat completion.
    // Otherwise, it depends on the prefix requirement for text generation - it may be routed for chat completion or ignored.

    if is_mentioning_bot {
        // Different clients do mentions differently.
        // The body text containing the mention usually contains one of:
        // - the full user ID (includes a @ prefix by default)
        // - the localpart (with a @ prefix)
        // - the localpart (without a @ prefix)
        // - the display name (with a @ prefix)
        // - the display name (without a @ prefix)
        //
        // Some add a `: ` suffix after the mention.
        //
        // There's no guarantee that the mention is at the start even.
        // It being there is most common and we try to strip it from there
        // as best as we can.
        let bot_user_id_localpart = bot_user_id.localpart();

        let mut prefixes_to_strip = vec![
            bot_user_id.as_str().to_owned(),
            format!("@{}", bot_user_id_localpart),
            bot_user_id_localpart.to_owned(),
        ];

        if let Some(bot_display_name) = bot_display_name {
            prefixes_to_strip.push(format!("@{}", bot_display_name));
            prefixes_to_strip.push(bot_display_name.to_owned());
        }

        prefixes_to_strip.push(":".to_owned());

        return ControllerType::ChatCompletion(ChatCompletionControllerType::ViaText {
            prefixes_to_strip,
        });
    }

    match room_text_generation_prefix_requirement_type {
        TextGenerationPrefixRequirementType::CommandPrefix => {
            if text.starts_with(command_prefix) {
                ControllerType::ChatCompletion(ChatCompletionControllerType::ViaText {
                    prefixes_to_strip: vec![command_prefix.to_owned()],
                })
            } else {
                ControllerType::Ignore
            }
        }
        TextGenerationPrefixRequirementType::No => {
            ControllerType::ChatCompletion(ChatCompletionControllerType::ViaText {
                prefixes_to_strip: vec![],
            })
        }
    }
}
