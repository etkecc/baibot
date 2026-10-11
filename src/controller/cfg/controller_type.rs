use crate::{
    agent::{AgentPurpose, PublicIdentifier},
    entity::roomconfig::{
        SpeechToTextFlowType, SpeechToTextMessageTypeForNonThreadedOnlyTranscribedMessages,
        TextGenerationAutoUsage, TextGenerationPrefixRequirementType,
        TextGenerationSenderContextMode, TextToSpeechBotMessagesFlowType,
        TextToSpeechUserMessagesFlowType,
    },
};

#[derive(Debug, PartialEq)]
pub enum SettingsStorageSource {
    Room,
    Global,
}

#[derive(Debug, PartialEq)]
pub enum ConfigControllerType {
    Help,

    Status,

    SettingsRelated(SettingsStorageSource, ConfigSettingRelatedControllerType),
}

#[derive(Debug, PartialEq)]
pub enum ConfigSettingRelatedControllerType {
    GetHandler(AgentPurpose),
    SetHandler(AgentPurpose, Option<PublicIdentifier>),

    TextGeneration(ConfigTextGenerationSettingRelatedControllerType),
    SpeechToText(ConfigSpeechToTextSettingRelatedControllerType),
    TextToSpeech(ConfigTextToSpeechSettingRelatedControllerType),
}

#[derive(Debug, PartialEq)]
pub enum ConfigTextGenerationSettingRelatedControllerType {
    GetContextManagementEnabled,
    SetContextManagementEnabled(Option<bool>),

    GetThinkingNoticeEnabled,
    SetThinkingNoticeEnabled(Option<bool>),

    GetPrefixRequirementType,
    SetPrefixRequirementType(Option<TextGenerationPrefixRequirementType>),

    GetAutoUsage,
    SetAutoUsage(Option<TextGenerationAutoUsage>),

    GetPromptOverride,
    SetPromptOverride(Option<String>),

    GetTemperatureOverride,
    SetTemperatureOverride(Option<f32>),

    GetSenderContextMode,
    SetSenderContextMode(Option<TextGenerationSenderContextMode>),
}

#[derive(Debug, PartialEq)]
pub enum ConfigSpeechToTextSettingRelatedControllerType {
    GetFlowType,
    SetFlowType(Option<SpeechToTextFlowType>),

    GetMsgTypeForNonThreadedOnlyTranscribedMessages,
    SetMsgTypeForNonThreadedOnlyTranscribedMessages(
        Option<SpeechToTextMessageTypeForNonThreadedOnlyTranscribedMessages>,
    ),

    GetLanguage,
    SetLanguage(Option<String>),
}

#[derive(Debug, PartialEq)]
pub enum ConfigTextToSpeechSettingRelatedControllerType {
    GetBotMessagesFlowType,
    SetBotMessagesFlowType(Option<TextToSpeechBotMessagesFlowType>),

    GetUserMessagesFlowType,
    SetUserMessagesFlowType(Option<TextToSpeechUserMessagesFlowType>),

    GetSpeedOverride,
    SetSpeedOverride(Option<f32>),

    GetVoiceOverride,
    SetVoiceOverride(Option<String>),
}

impl ConfigSettingRelatedControllerType {
    /// Whether handling this changes stored settings, as opposed to only reading them.
    ///
    /// Anything that isn't one of the known getters counts as a change, so that settings added
    /// later are protected by default.
    pub fn is_mutation(&self) -> bool {
        match self {
            Self::GetHandler(_) => false,
            Self::SetHandler(_, _) => true,
            Self::TextGeneration(controller_type) => !matches!(
                controller_type,
                ConfigTextGenerationSettingRelatedControllerType::GetContextManagementEnabled
                    | ConfigTextGenerationSettingRelatedControllerType::GetThinkingNoticeEnabled
                    | ConfigTextGenerationSettingRelatedControllerType::GetPrefixRequirementType
                    | ConfigTextGenerationSettingRelatedControllerType::GetAutoUsage
                    | ConfigTextGenerationSettingRelatedControllerType::GetPromptOverride
                    | ConfigTextGenerationSettingRelatedControllerType::GetTemperatureOverride
                    | ConfigTextGenerationSettingRelatedControllerType::GetSenderContextMode
            ),
            Self::SpeechToText(controller_type) => !matches!(
                controller_type,
                ConfigSpeechToTextSettingRelatedControllerType::GetFlowType
                    | ConfigSpeechToTextSettingRelatedControllerType::GetMsgTypeForNonThreadedOnlyTranscribedMessages
                    | ConfigSpeechToTextSettingRelatedControllerType::GetLanguage
            ),
            Self::TextToSpeech(controller_type) => !matches!(
                controller_type,
                ConfigTextToSpeechSettingRelatedControllerType::GetBotMessagesFlowType
                    | ConfigTextToSpeechSettingRelatedControllerType::GetUserMessagesFlowType
                    | ConfigTextToSpeechSettingRelatedControllerType::GetSpeedOverride
                    | ConfigTextToSpeechSettingRelatedControllerType::GetVoiceOverride
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::AgentPurpose;

    #[test]
    fn getters_are_not_mutations() {
        let getters = [
            ConfigSettingRelatedControllerType::GetHandler(AgentPurpose::TextGeneration),
            ConfigSettingRelatedControllerType::TextGeneration(
                ConfigTextGenerationSettingRelatedControllerType::GetAutoUsage,
            ),
            ConfigSettingRelatedControllerType::TextGeneration(
                ConfigTextGenerationSettingRelatedControllerType::GetPromptOverride,
            ),
            ConfigSettingRelatedControllerType::SpeechToText(
                ConfigSpeechToTextSettingRelatedControllerType::GetLanguage,
            ),
            ConfigSettingRelatedControllerType::TextToSpeech(
                ConfigTextToSpeechSettingRelatedControllerType::GetVoiceOverride,
            ),
        ];

        for getter in getters {
            assert!(!getter.is_mutation(), "{getter:?} should not be a mutation");
        }
    }

    #[test]
    fn setters_are_mutations() {
        let setters = [
            ConfigSettingRelatedControllerType::SetHandler(AgentPurpose::TextGeneration, None),
            ConfigSettingRelatedControllerType::TextGeneration(
                ConfigTextGenerationSettingRelatedControllerType::SetAutoUsage(None),
            ),
            ConfigSettingRelatedControllerType::TextGeneration(
                ConfigTextGenerationSettingRelatedControllerType::SetPromptOverride(Some(
                    "x".to_owned(),
                )),
            ),
            ConfigSettingRelatedControllerType::SpeechToText(
                ConfigSpeechToTextSettingRelatedControllerType::SetLanguage(None),
            ),
            ConfigSettingRelatedControllerType::TextToSpeech(
                ConfigTextToSpeechSettingRelatedControllerType::SetVoiceOverride(None),
            ),
        ];

        for setter in setters {
            assert!(setter.is_mutation(), "{setter:?} should be a mutation");
        }
    }
}
