use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct RoomSettingsTextGeneration {
    /// Controls whether initial text messages require a prefix to trigger text generation.
    /// This could have been a bool, using an enum allows us to add more options (e.g. CustomPrefix) in the future.
    /// Even if set to "required", prefixless-triggering could still happen via an initial voice message (see auto_usage).
    pub prefix_requirement_type: Option<TextGenerationPrefixRequirementType>,

    /// Controls whether text generation is automatically triggered (depending on message type).
    pub auto_usage: Option<TextGenerationAutoUsage>,

    /// Controls whether conversation context management is enabled.
    /// When enabled, the bot will automatically tokenize messages and try to shorten the message context intelligently.
    pub context_management_enabled: Option<bool>,

    /// Allows customizing the system prompt that the agent would use
    pub prompt_override: Option<String>,

    /// Allows customizing the temperature that the agent would use
    pub temperature_override: Option<f32>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
pub enum TextGenerationPrefixRequirementType {
    /// Text Generation is to be triggered for any text message
    #[serde(rename = "no")]
    No,

    /// Text Generation is to be triggered only for messages that are prefixed with the command prefix
    #[serde(rename = "command_prefix")]
    CommandPrefix,
}

impl TextGenerationPrefixRequirementType {
    pub fn choices() -> Vec<Self> {
        vec![Self::No, Self::CommandPrefix]
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "no" => Some(Self::No),
            "command_prefix" => Some(Self::CommandPrefix),
            _ => None,
        }
    }
}

impl std::fmt::Display for TextGenerationPrefixRequirementType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TextGenerationPrefixRequirementType::No => write!(f, "no"),
            TextGenerationPrefixRequirementType::CommandPrefix => {
                write!(f, "command_prefix")
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
pub enum TextGenerationAutoUsage {
    /// Text Generation is to never be performed
    #[serde(rename = "never")]
    Never,

    /// Text Generation is to always be performed
    #[serde(rename = "always")]
    Always,

    /// Text Generation is to be performed when the original message was sent as audio (voice).
    /// The voice message would be transcribed to text (subject to other configuration)
    /// and text generation would be triggered.
    #[serde(rename = "only_for_voice")]
    OnlyForVoice,

    /// Text Generation is to be performed when the original message was sent as text
    #[serde(rename = "only_for_text")]
    OnlyForText,
}

impl TextGenerationAutoUsage {
    pub fn choices() -> Vec<Self> {
        vec![
            Self::Never,
            Self::Always,
            Self::OnlyForVoice,
            Self::OnlyForText,
        ]
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "never" => Some(Self::Never),
            "always" => Some(Self::Always),
            "only_for_voice" => Some(Self::OnlyForVoice),
            "only_for_text" => Some(Self::OnlyForText),
            _ => None,
        }
    }
}

impl std::fmt::Display for TextGenerationAutoUsage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TextGenerationAutoUsage::Never => write!(f, "never"),
            TextGenerationAutoUsage::Always => write!(f, "always"),
            TextGenerationAutoUsage::OnlyForVoice => write!(f, "only_for_voice"),
            TextGenerationAutoUsage::OnlyForText => write!(f, "only_for_text"),
        }
    }
}
