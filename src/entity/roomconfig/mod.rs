pub mod defaults;
mod entity;

use mxlink::helpers::account_data_config::RoomConfigManager as AccountDataRoomConfigManager;

pub use entity::{RoomConfig, RoomConfigCarrierContent, RoomSettings, RoomSettingsHandler};
pub use entity::{
    SpeechToTextFlowType, SpeechToTextMessageTypeForNonThreadedOnlyTranscribedMessages,
    TextGenerationAutoUsage, TextGenerationPrefixRequirementType, TextToSpeechBotMessagesFlowType,
    TextToSpeechUserMessagesFlowType,
};

pub type RoomConfigurationManager =
    AccountDataRoomConfigManager<RoomConfig, RoomConfigCarrierContent>;
