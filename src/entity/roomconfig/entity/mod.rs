use mxlink::helpers::account_data_config::RoomConfig as RoomConfigTrait;
use mxlink::helpers::account_data_config::RoomConfigCarrierContent as RoomConfigCarrierContentTrait;
use mxlink::matrix_sdk::ruma::events::macros::EventContent;
use mxlink::matrix_sdk::{Room, RoomMemberships};

use serde::{Deserialize, Serialize};

use crate::agent::AgentDefinition;

mod handler;
mod speech_to_text;
mod text_generation;
mod text_to_speech;

pub use handler::RoomSettingsHandler;
pub use speech_to_text::{
    SpeechToTextFlowType, SpeechToTextMessageTypeForNonThreadedOnlyTranscribedMessages,
};
pub use text_generation::{TextGenerationAutoUsage, TextGenerationPrefixRequirementType};
pub use text_to_speech::{TextToSpeechBotMessagesFlowType, TextToSpeechUserMessagesFlowType};

#[derive(Clone, Debug, Deserialize, Serialize, EventContent)]
#[ruma_event(type = "cc.etke.baibot.room_config", kind = RoomAccountData)]
pub struct RoomConfigCarrierContent {
    pub payload: String,
}

impl RoomConfigCarrierContentTrait for RoomConfigCarrierContent {
    fn payload(&self) -> &str {
        &self.payload
    }

    fn new(payload: String) -> Self {
        Self { payload }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct RoomConfig {
    pub settings: RoomSettings,

    pub agents: Vec<AgentDefinition>,
}

impl RoomConfigTrait for RoomConfig {}

impl RoomConfig {
    pub async fn with_room(mut self, room: Room) -> Self {
        tracing::trace!(
            "Determining room members count to decide on a suitable text-generation/prefix-requirement-type default"
        );

        let members = room.members(RoomMemberships::ACTIVE).await;

        let prefix_requirement_type = match members {
            Ok(members) => {
                let members_count = members.len();

                let prefix_requirement_type = if members.len() > 2 {
                    text_generation::TextGenerationPrefixRequirementType::CommandPrefix
                } else {
                    text_generation::TextGenerationPrefixRequirementType::No
                };

                tracing::info!(
                    ?members_count,
                    ?prefix_requirement_type,
                    "Determined text-generation/prefix-requirement-type based on room members count"
                );

                prefix_requirement_type
            }
            Err(err) => {
                tracing::error!(
                    ?err,
                    "Failed to get members of room - will default text-generation/prefix-requirement-type to No"
                );
                text_generation::TextGenerationPrefixRequirementType::No
            }
        };

        self.settings.text_generation.prefix_requirement_type = Some(prefix_requirement_type);

        self
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct RoomSettings {
    pub handler: handler::RoomSettingsHandler,

    #[serde(default)]
    pub text_generation: text_generation::RoomSettingsTextGeneration,

    #[serde(default)]
    pub speech_to_text: speech_to_text::RoomSettingsSpeechToText,

    #[serde(default)]
    pub text_to_speech: text_to_speech::RoomSettingsTextToSpeech,
}
