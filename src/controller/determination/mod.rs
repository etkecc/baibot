#[cfg(test)]
mod tests;

use super::chat_completion::ChatCompletionControllerType;
use crate::{
    entity::{
        InteractionTrigger, MessageContext, MessagePayload,
        roomconfig::TextGenerationPrefixRequirementType,
    },
    strings,
};

use super::ControllerType;

pub fn determine_controller(
    command_prefix: &str,
    first_thread_message: &InteractionTrigger,
    message_context: &MessageContext,
) -> ControllerType {
    match &first_thread_message.payload {
        MessagePayload::SynthethicChatCompletionTriggerInThread => {
            ControllerType::ChatCompletion(ChatCompletionControllerType::ThreadMention)
        }
        MessagePayload::SynthethicChatCompletionTriggerForReply => {
            ControllerType::ChatCompletion(ChatCompletionControllerType::ReplyMention)
        }
        MessagePayload::Text(text_message_content) => {
            let prefix_requirement_type = message_context
                .room_config_context()
                .text_generation_prefix_requirement_type();

            determine_text_controller(
                command_prefix,
                &text_message_content.body,
                prefix_requirement_type,
                first_thread_message.is_mentioning_bot,
            )
        }
        MessagePayload::Image(_image_message_content) => {
            let prefix_requirement_type = message_context
                .room_config_context()
                .text_generation_prefix_requirement_type();

            match prefix_requirement_type {
                TextGenerationPrefixRequirementType::CommandPrefix => ControllerType::Ignore,
                TextGenerationPrefixRequirementType::No => {
                    ControllerType::ChatCompletion(ChatCompletionControllerType::Image)
                }
            }
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
        MessagePayload::File(_file_message_content) => {
            let prefix_requirement_type = message_context
                .room_config_context()
                .text_generation_prefix_requirement_type();

            match prefix_requirement_type {
                TextGenerationPrefixRequirementType::CommandPrefix => ControllerType::Ignore,
                TextGenerationPrefixRequirementType::No => {
                    ControllerType::ChatCompletion(ChatCompletionControllerType::File)
                }
            }
        }
        MessagePayload::Audio(_) => {
            ControllerType::ChatCompletion(ChatCompletionControllerType::Audio)
        }
        MessagePayload::Reaction { .. } => {
            panic!("Handling reaction as first message in thread does not make sense")
        }
    }
}

pub(super) fn determine_text_controller(
    command_prefix: &str,
    text: &str,
    room_text_generation_prefix_requirement_type: TextGenerationPrefixRequirementType,
    is_mentioning_bot: bool,
) -> ControllerType {
    let text = text.trim();

    if text == command_prefix {
        return ControllerType::Help;
    }

    // Remote commands (`in ROOM COMMAND`) have their own parsing.
    if let Some(controller_type) = super::remote::determine_controller(command_prefix, text) {
        return controller_type;
    }

    // Commands are matched by whole words, so that a prompt like `agentic AI is...` is not taken for
    // the `agent` command. Plural forms are accepted too, since they are easy to type by mistake.
    if let Some((word, rest)) = split_command_word(command_prefix, text) {
        match word {
            "help" => return ControllerType::Help,
            "room" | "rooms" => {
                return if rest.is_empty() || rest == "list" {
                    ControllerType::RoomList
                } else {
                    ControllerType::Error(strings::remote::unknown_room_command(command_prefix))
                };
            }
            "access" => return super::access::determine_controller(command_prefix, rest),
            "provider" | "providers" => return super::provider::determine_controller(rest),
            "agent" | "agents" => return super::agent::determine_controller(command_prefix, rest),
            "config" => return super::cfg::determine_controller(rest),
            "image" | "images" => return super::image::determine_controller(rest),
            "sticker" | "stickers" => {
                return ControllerType::StickerGeneration(rest.to_owned());
            }
            "usage" => return super::usage::determine_controller(rest),
            _ => {}
        }
    }

    // Regular text message that does not match any command.
    // If it mentions the bot, it's a chat completion.
    // Otherwise, it depends on the prefix requirement for text generation - it may be routed for chat completion or ignored.

    if is_mentioning_bot {
        return ControllerType::ChatCompletion(ChatCompletionControllerType::TextMention);
    }

    // Regardless of what the prefix requirement is, if we encounter a command prefix, we'll consider it a chat completion via command prefix invokation.
    // This is to correctly indicate to the chat completion controller that a command prefix was used,
    // so that it can be stripped from the beginning of the message.
    if text.starts_with(command_prefix) {
        return ControllerType::ChatCompletion(ChatCompletionControllerType::TextCommand);
    }

    // We're dealing with a regular message that does not start with a command prefix.

    match room_text_generation_prefix_requirement_type {
        TextGenerationPrefixRequirementType::CommandPrefix => {
            // A prefix is required, but we've already checked (above) that the message does not start with a command prefix.
            // It's to be ignored.
            ControllerType::Ignore
        }
        TextGenerationPrefixRequirementType::No => {
            ControllerType::ChatCompletion(ChatCompletionControllerType::TextDirect)
        }
    }
}

/// Splits `PREFIX WORD REST` into the command word and the (trimmed) rest.
fn split_command_word<'a>(command_prefix: &str, text: &'a str) -> Option<(&'a str, &'a str)> {
    let remaining = text.strip_prefix(command_prefix)?;
    if !remaining.starts_with(char::is_whitespace) {
        return None;
    }

    let remaining = remaining.trim_start();

    Some(match remaining.split_once(char::is_whitespace) {
        Some((word, rest)) => (word, rest.trim()),
        None => (remaining, ""),
    })
}
