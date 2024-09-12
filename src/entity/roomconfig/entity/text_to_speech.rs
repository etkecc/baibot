use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct RoomSettingsTextToSpeech {
    pub bot_msgs_flow_type: Option<TextToSpeechBotMessagesFlowType>,

    pub user_msgs_flow_type: Option<TextToSpeechUserMessagesFlowType>,

    pub speed_override: Option<f32>,

    pub voice_override: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
pub enum TextToSpeechBotMessagesFlowType {
    /// Never do text-to-speech for bot messages automatically and don't offer it
    #[serde(rename = "never")]
    Never,

    /// Never do text-to-speech for bot messages automatically, but offer it via an emoji reaction for all messages
    #[serde(rename = "on_demand_always")]
    OnDemandAlways,

    /// Never do text-to-speech for bot messages automatically, but offer it via an emoji reaction if the user message that prompted the bot message was audio (voice)
    #[serde(rename = "on_demand_for_voice")]
    OnDemandForVoice,

    /// Convert all bot text messages to audio (voice) automatically if the user message that prompted the bot message was audio (voice)
    #[serde(rename = "only_for_voice")]
    OnlyForVoice,

    /// Convert all bot text messages to audio (voice) automatically
    #[serde(rename = "always")]
    Always,
}

impl TextToSpeechBotMessagesFlowType {
    pub fn choices() -> Vec<Self> {
        vec![
            Self::Never,
            Self::OnDemandAlways,
            Self::OnDemandForVoice,
            Self::Always,
            Self::OnlyForVoice,
        ]
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "never" => Some(Self::Never),
            "on_demand_always" => Some(Self::OnDemandAlways),
            "on_demand_for_voice" => Some(Self::OnDemandForVoice),
            "only_for_voice" => Some(Self::OnlyForVoice),
            "always" => Some(Self::Always),
            _ => None,
        }
    }
}

impl std::fmt::Display for TextToSpeechBotMessagesFlowType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TextToSpeechBotMessagesFlowType::Never => write!(f, "never"),
            TextToSpeechBotMessagesFlowType::OnDemandAlways => write!(f, "on_demand_always"),
            TextToSpeechBotMessagesFlowType::OnDemandForVoice => write!(f, "on_demand_for_voice"),
            TextToSpeechBotMessagesFlowType::Always => write!(f, "always"),
            TextToSpeechBotMessagesFlowType::OnlyForVoice => write!(f, "only_for_voice"),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
pub enum TextToSpeechUserMessagesFlowType {
    /// Never do text-to-speech for user messages automatically and don't offer it
    #[serde(rename = "never")]
    Never,

    /// Never do text-to-speech for user messages automatically, but offer it via an emoji reaction
    #[serde(rename = "on_demand")]
    OnDemand,

    /// Convert all user text messages to audio (voice) automatically
    #[serde(rename = "always")]
    Always,
}

impl TextToSpeechUserMessagesFlowType {
    pub fn choices() -> Vec<Self> {
        vec![Self::Never, Self::OnDemand, Self::Always]
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "never" => Some(Self::Never),
            "on_demand" => Some(Self::OnDemand),
            "always" => Some(Self::Always),
            _ => None,
        }
    }
}

impl std::fmt::Display for TextToSpeechUserMessagesFlowType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TextToSpeechUserMessagesFlowType::Never => write!(f, "never"),
            TextToSpeechUserMessagesFlowType::OnDemand => write!(f, "on_demand"),
            TextToSpeechUserMessagesFlowType::Always => write!(f, "always"),
        }
    }
}
