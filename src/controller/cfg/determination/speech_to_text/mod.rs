#[cfg(test)]
mod tests;

use crate::{
    controller::ControllerType,
    entity::roomconfig::{
        SpeechToTextFlowType, SpeechToTextMessageTypeForNonThreadedOnlyTranscribedMessages,
    },
    strings,
};

use super::super::controller_type::ConfigSpeechToTextSettingRelatedControllerType;

pub(super) fn determine(
    text: &str,
) -> Result<ConfigSpeechToTextSettingRelatedControllerType, ControllerType> {
    // Flow Type

    if let Some(remaining_text) = text.strip_prefix("flow-type") {
        let remaining_text = remaining_text.trim();

        if !remaining_text.is_empty() {
            return Err(ControllerType::Error(
                strings::cfg::configuration_getter_used_with_extra_text(
                    "flow-type",
                    remaining_text,
                )
                .to_owned(),
            ));
        }

        return Ok(ConfigSpeechToTextSettingRelatedControllerType::GetFlowType);
    }

    if let Some(value_string) = text.strip_prefix("set-flow-type") {
        let value_string = value_string.trim().to_owned();

        let value_choice = if value_string.is_empty() {
            None
        } else {
            let value_choice = SpeechToTextFlowType::from_str(&value_string.to_lowercase());

            if value_choice.is_none() {
                return Err(ControllerType::Error(
                    strings::cfg::configuration_value_unrecognized(&value_string).to_owned(),
                ));
            }

            value_choice
        };

        return Ok(ConfigSpeechToTextSettingRelatedControllerType::SetFlowType(
            value_choice,
        ));
    }

    // msg_type_for_non_threaded_only_transcribed_messages

    if let Some(remaining_text) =
        text.strip_prefix("msg-type-for-non-threaded-only-transcribed-messages")
    {
        let remaining_text = remaining_text.trim();

        if !remaining_text.is_empty() {
            return Err(ControllerType::Error(
                strings::cfg::configuration_getter_used_with_extra_text(
                    "msg-type-for-non-threaded-only-transcribed-messages",
                    remaining_text,
                )
                .to_owned(),
            ));
        }

        return Ok(ConfigSpeechToTextSettingRelatedControllerType::GetMsgTypeForNonThreadedOnlyTranscribedMessages);
    }

    if let Some(value_string) =
        text.strip_prefix("set-msg-type-for-non-threaded-only-transcribed-messages")
    {
        let value_string = value_string.trim().to_owned();

        let value_choice = if value_string.is_empty() {
            None
        } else {
            let value_choice =
                SpeechToTextMessageTypeForNonThreadedOnlyTranscribedMessages::from_str(
                    &value_string.to_lowercase(),
                );

            if value_choice.is_none() {
                return Err(ControllerType::Error(
                    strings::cfg::configuration_value_unrecognized(&value_string).to_owned(),
                ));
            }

            value_choice
        };

        return Ok(ConfigSpeechToTextSettingRelatedControllerType::SetMsgTypeForNonThreadedOnlyTranscribedMessages(
            value_choice,
        ));
    }

    // Language

    if let Some(remaining_text) = text.strip_prefix("language") {
        let remaining_text = remaining_text.trim();

        if !remaining_text.is_empty() {
            return Err(ControllerType::Error(
                strings::cfg::configuration_getter_used_with_extra_text("language", remaining_text)
                    .to_owned(),
            ));
        }

        return Ok(ConfigSpeechToTextSettingRelatedControllerType::GetLanguage);
    }

    if let Some(value_string) = text.strip_prefix("set-language") {
        let value_string = value_string.trim().to_owned();

        let value_string = if value_string.is_empty() {
            None
        } else {
            if value_string.len() != 2 {
                return Err(ControllerType::Error(
                    strings::speech_to_text::language_code_invalid(&value_string).to_owned(),
                ));
            }

            Some(value_string)
        };

        return Ok(ConfigSpeechToTextSettingRelatedControllerType::SetLanguage(
            value_string,
        ));
    }

    Err(ControllerType::Unknown)
}
