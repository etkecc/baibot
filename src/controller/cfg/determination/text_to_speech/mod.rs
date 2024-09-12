#[cfg(test)]
mod tests;

use crate::{
    controller::ControllerType,
    entity::roomconfig::{TextToSpeechBotMessagesFlowType, TextToSpeechUserMessagesFlowType},
    strings,
};

use super::super::controller_type::ConfigTextToSpeechSettingRelatedControllerType;

pub(super) fn determine(
    text: &str,
) -> Result<ConfigTextToSpeechSettingRelatedControllerType, ControllerType> {
    if let Some(remaining_text) = text.strip_prefix("bot-msgs-flow-type") {
        let remaining_text = remaining_text.trim();

        if !remaining_text.is_empty() {
            return Err(ControllerType::Error(
                strings::cfg::configuration_getter_used_with_extra_text(
                    "bot-msgs-flow-type",
                    remaining_text,
                )
                .to_owned(),
            ));
        }

        return Ok(ConfigTextToSpeechSettingRelatedControllerType::GetBotMessagesFlowType);
    }

    if let Some(value_string) = text.strip_prefix("set-bot-msgs-flow-type") {
        let value_string = value_string.trim().to_owned();

        let value_choice = if value_string.is_empty() {
            None
        } else {
            let value_choice =
                TextToSpeechBotMessagesFlowType::from_str(&value_string.to_lowercase());

            if value_choice.is_none() {
                return Err(ControllerType::Error(
                    strings::cfg::configuration_value_unrecognized(&value_string).to_owned(),
                ));
            }

            value_choice
        };

        return Ok(
            ConfigTextToSpeechSettingRelatedControllerType::SetBotMessagesFlowType(value_choice),
        );
    }

    if let Some(remaining_text) = text.strip_prefix("user-msgs-flow-type") {
        let remaining_text = remaining_text.trim();

        if !remaining_text.is_empty() {
            return Err(ControllerType::Error(
                strings::cfg::configuration_getter_used_with_extra_text(
                    "user-msgs-flow-type",
                    remaining_text,
                )
                .to_owned(),
            ));
        }

        return Ok(ConfigTextToSpeechSettingRelatedControllerType::GetUserMessagesFlowType);
    }

    if let Some(value_string) = text.strip_prefix("set-user-msgs-flow-type") {
        let value_string = value_string.trim().to_owned();

        let value_choice = if value_string.is_empty() {
            None
        } else {
            let value_choice =
                TextToSpeechUserMessagesFlowType::from_str(&value_string.to_lowercase());

            if value_choice.is_none() {
                return Err(ControllerType::Error(
                    strings::cfg::configuration_value_unrecognized(&value_string).to_owned(),
                ));
            }

            value_choice
        };

        return Ok(
            ConfigTextToSpeechSettingRelatedControllerType::SetUserMessagesFlowType(value_choice),
        );
    }

    if let Some(remaining_text) = text.strip_prefix("speed-override") {
        let remaining_text = remaining_text.trim();

        if !remaining_text.is_empty() {
            return Err(ControllerType::Error(
                strings::cfg::configuration_getter_used_with_extra_text(
                    "speed-override",
                    remaining_text,
                )
                .to_owned(),
            ));
        }

        return Ok(ConfigTextToSpeechSettingRelatedControllerType::GetSpeedOverride);
    }

    if let Some(value_string) = text.strip_prefix("set-speed-override") {
        let value_string = value_string.trim().to_owned();

        if value_string.is_empty() {
            return Ok(ConfigTextToSpeechSettingRelatedControllerType::SetSpeedOverride(None));
        }

        let value_f32 = value_string.parse::<f32>();

        let Ok(value_f32) = value_f32 else {
            return Err(ControllerType::Error(
                strings::cfg::configuration_value_not_f32(&value_string).to_owned(),
            ));
        };

        return Ok(
            ConfigTextToSpeechSettingRelatedControllerType::SetSpeedOverride(Some(value_f32)),
        );
    }

    if let Some(remaining_text) = text.strip_prefix("voice-override") {
        let remaining_text = remaining_text.trim();

        if !remaining_text.is_empty() {
            return Err(ControllerType::Error(
                strings::cfg::configuration_getter_used_with_extra_text(
                    "voice-override",
                    remaining_text,
                )
                .to_owned(),
            ));
        }

        return Ok(ConfigTextToSpeechSettingRelatedControllerType::GetVoiceOverride);
    }

    if let Some(value_string) = text.strip_prefix("set-voice-override") {
        let value_string = value_string.trim().to_owned();

        if value_string.is_empty() {
            return Ok(ConfigTextToSpeechSettingRelatedControllerType::SetVoiceOverride(None));
        }

        return Ok(
            ConfigTextToSpeechSettingRelatedControllerType::SetVoiceOverride(Some(value_string)),
        );
    }

    Err(ControllerType::Unknown)
}
