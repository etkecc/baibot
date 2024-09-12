use crate::{
    agent::{AgentPurpose, PublicIdentifier},
    entity::roomconfig::{
        SpeechToTextFlowType, TextGenerationAutoUsage, TextGenerationPrefixRequirementType,
        TextToSpeechBotMessagesFlowType, TextToSpeechUserMessagesFlowType,
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

    GetPrefixRequirementType,
    SetPrefixRequirementType(Option<TextGenerationPrefixRequirementType>),

    GetAutoUsage,
    SetAutoUsage(Option<TextGenerationAutoUsage>),

    GetPromptOverride,
    SetPromptOverride(Option<String>),

    GetTemperatureOverride,
    SetTemperatureOverride(Option<f32>),
}

#[derive(Debug, PartialEq)]
pub enum ConfigSpeechToTextSettingRelatedControllerType {
    GetFlowType,
    SetFlowType(Option<SpeechToTextFlowType>),

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
