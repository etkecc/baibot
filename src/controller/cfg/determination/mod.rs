#[cfg(test)]
mod tests;

mod speech_to_text;
mod text_generation;
mod text_to_speech;

use crate::{
    agent::{AgentPurpose, PublicIdentifier},
    controller::ControllerType,
    strings,
};

use super::controller_type::{
    ConfigControllerType, ConfigSettingRelatedControllerType, SettingsStorageSource,
};

pub fn determine_controller(text: &str) -> ControllerType {
    if text.starts_with("status") {
        return ControllerType::Config(ConfigControllerType::Status);
    }

    // Someone pasted our instructions verbatim.
    if text.strip_prefix("CONFIG_TYPE").is_some() {
        return ControllerType::Error(strings::cfg::error_config_type_not_replaced());
    }

    if let Some(remaining_text) = text.strip_prefix("room ") {
        return match do_determine_controller(remaining_text.trim()) {
            Ok(handler) => ControllerType::Config(ConfigControllerType::SettingsRelated(
                SettingsStorageSource::Room,
                handler,
            )),
            Err(controller_type) => controller_type,
        };
    }

    if let Some(remaining_text) = text.strip_prefix("global ") {
        return match do_determine_controller(remaining_text.trim()) {
            Ok(handler) => ControllerType::Config(ConfigControllerType::SettingsRelated(
                SettingsStorageSource::Global,
                handler,
            )),
            Err(controller_type) => controller_type,
        };
    }

    ControllerType::Config(ConfigControllerType::Help)
}

fn do_determine_controller(
    text: &str,
) -> Result<ConfigSettingRelatedControllerType, ControllerType> {
    if let Some(purpose_str) = text.strip_prefix("handler") {
        let purpose_str = purpose_str.trim();

        if purpose_str.is_empty() {
            return Err(ControllerType::Error(
                strings::cfg::configuration_invocation_incorrect_more_values_expected().to_owned(),
            ));
        }

        let Some(purpose) = AgentPurpose::from_str(purpose_str) else {
            return Err(ControllerType::Error(
                strings::agent::purpose_unrecognized(purpose_str).to_owned(),
            ));
        };

        return Ok(ConfigSettingRelatedControllerType::GetHandler(purpose));
    }

    if let Some(remaining_text) = text.strip_prefix("set-handler") {
        // Something like:
        // - `PURPOSE ID`
        // - `PURPOSE`
        let remaining_text = remaining_text.trim();

        if remaining_text.is_empty() {
            return Err(ControllerType::Error(
                strings::cfg::configuration_invocation_incorrect_more_values_expected().to_owned(),
            ));
        }

        // This will be None if we're just dealing with `PURPOSE` and lack an `ID`.
        // In such cases, the whole thing is the purpose string.
        let parts = remaining_text.split_once(' ');

        let (purpose_str, agent_id_string_option) = if let Some(parts) = parts {
            (parts.0, Some(parts.1.to_owned()))
        } else {
            (remaining_text, None)
        };

        let Some(purpose) = AgentPurpose::from_str(purpose_str) else {
            return Err(ControllerType::Error(
                strings::agent::purpose_unrecognized(purpose_str).to_owned(),
            ));
        };

        let agent_identifier = match agent_id_string_option {
            Some(agent_id_string) => {
                let Some(agent_identifier) = PublicIdentifier::from_str(&agent_id_string) else {
                    return Err(ControllerType::Error(
                        strings::agent::invalid_id_generic().to_owned(),
                    ));
                };

                Some(agent_identifier)
            }
            None => None,
        };

        return Ok(ConfigSettingRelatedControllerType::SetHandler(
            purpose,
            agent_identifier,
        ));
    }

    if let Some(remaining_text) = text.strip_prefix("text-generation") {
        return match text_generation::determine(remaining_text.trim()) {
            Ok(handler) => Ok(ConfigSettingRelatedControllerType::TextGeneration(handler)),
            Err(controller_type) => Err(controller_type),
        };
    }

    if let Some(remaining_text) = text.strip_prefix("text-to-speech") {
        return match text_to_speech::determine(remaining_text.trim()) {
            Ok(handler) => Ok(ConfigSettingRelatedControllerType::TextToSpeech(handler)),
            Err(controller_type) => Err(controller_type),
        };
    }

    if let Some(remaining_text) = text.strip_prefix("speech-to-text") {
        return match speech_to_text::determine(remaining_text.trim()) {
            Ok(handler) => Ok(ConfigSettingRelatedControllerType::SpeechToText(handler)),
            Err(controller_type) => Err(controller_type),
        };
    }

    Err(ControllerType::Unknown)
}
