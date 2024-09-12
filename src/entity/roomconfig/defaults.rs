use super::SpeechToTextFlowType;
use super::{TextGenerationAutoUsage, TextGenerationPrefixRequirementType};
use super::{TextToSpeechBotMessagesFlowType, TextToSpeechUserMessagesFlowType};

pub const TEXT_GENERATION_PREFIX_REQUIREMENT_TYPE: TextGenerationPrefixRequirementType =
    TextGenerationPrefixRequirementType::No;

pub const TEXT_GENERATION_AUTO_USAGE: TextGenerationAutoUsage = TextGenerationAutoUsage::Always;

pub const TEXT_TO_SPEECH_BOT_MESSAGES_FLOW_TYPE: TextToSpeechBotMessagesFlowType =
    TextToSpeechBotMessagesFlowType::OnDemandForVoice;

pub const TEXT_TO_SPEECH_USER_MESSAGES_FLOW_TYPE: TextToSpeechUserMessagesFlowType =
    TextToSpeechUserMessagesFlowType::OnDemand;

pub const SPEECH_TO_TEXT_FLOW_TYPE: SpeechToTextFlowType =
    SpeechToTextFlowType::TranscribeAndGenerateText;
