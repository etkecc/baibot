use mxlink::matrix_sdk::ruma::OwnedUserId;

use super::globalconfig::GlobalConfig;
use super::roomconfig::RoomConfig;

use crate::entity::roomconfig::{
    SpeechToTextFlowType, SpeechToTextMessageTypeForNonThreadedOnlyTranscribedMessages,
    TextGenerationAutoUsage, TextGenerationPrefixRequirementType, TextToSpeechBotMessagesFlowType,
    TextToSpeechUserMessagesFlowType, defaults as roomconfig_defaults,
};

#[derive(Debug, Clone)]
pub struct RoomConfigContext {
    pub(crate) global_config: GlobalConfig,
    pub(crate) room_config: RoomConfig,
}

impl RoomConfigContext {
    pub fn new(global_config: GlobalConfig, room_config: RoomConfig) -> RoomConfigContext {
        Self {
            global_config,
            room_config,
        }
    }

    pub fn speech_to_text_flow_type(&self) -> SpeechToTextFlowType {
        self.room_config
            .settings
            .speech_to_text
            .flow_type
            .or({
                self.global_config
                    .fallback_room_settings
                    .speech_to_text
                    .flow_type
            })
            .unwrap_or(roomconfig_defaults::SPEECH_TO_TEXT_FLOW_TYPE)
    }

    pub fn speech_to_text_msg_type_for_non_threaded_only_transcribed_messages(
        &self,
    ) -> SpeechToTextMessageTypeForNonThreadedOnlyTranscribedMessages {
        self.room_config
            .settings
            .speech_to_text
            .msg_type_for_non_threaded_only_transcribed_messages
            .or({
                self.global_config
                    .fallback_room_settings
                    .speech_to_text
                    .msg_type_for_non_threaded_only_transcribed_messages
            })
            .unwrap_or(
                roomconfig_defaults::SPEECH_TO_TEXT_ONLY_TRANSCRIBE_NON_THREADED_MESSAGE_TYPE,
            )
    }

    pub fn speech_to_text_language(&self) -> Option<String> {
        self.room_config
            .settings
            .speech_to_text
            .language
            .clone()
            .or({
                self.global_config
                    .fallback_room_settings
                    .speech_to_text
                    .language
                    .clone()
            })
    }

    pub fn auto_text_generation_usage(&self) -> TextGenerationAutoUsage {
        self.room_config
            .settings
            .text_generation
            .auto_usage
            .or({
                self.global_config
                    .fallback_room_settings
                    .text_generation
                    .auto_usage
            })
            .unwrap_or(roomconfig_defaults::TEXT_GENERATION_AUTO_USAGE)
    }

    pub fn should_auto_text_generate(&self, original_message_is_audio: bool) -> bool {
        match self.auto_text_generation_usage() {
            TextGenerationAutoUsage::Never => false,
            TextGenerationAutoUsage::Always => true,
            TextGenerationAutoUsage::OnlyForVoice => original_message_is_audio,
            TextGenerationAutoUsage::OnlyForText => !original_message_is_audio,
        }
    }

    pub fn text_generation_prompt_override(&self) -> Option<String> {
        self.room_config
            .settings
            .text_generation
            .prompt_override
            .clone()
            .or_else(|| {
                self.global_config
                    .fallback_room_settings
                    .text_generation
                    .prompt_override
                    .clone()
            })
    }

    pub fn text_generation_temperature_override(&self) -> Option<f32> {
        self.room_config
            .settings
            .text_generation
            .temperature_override
            .or({
                self.global_config
                    .fallback_room_settings
                    .text_generation
                    .temperature_override
            })
    }

    pub fn text_generation_context_management_enabled(&self) -> bool {
        self.room_config
            .settings
            .text_generation
            .context_management_enabled
            .or({
                self.global_config
                    .fallback_room_settings
                    .text_generation
                    .context_management_enabled
            })
            .unwrap_or(false)
    }

    pub fn text_generation_prefix_requirement_type(&self) -> TextGenerationPrefixRequirementType {
        self.room_config
            .settings
            .text_generation
            .prefix_requirement_type
            .or({
                self.global_config
                    .fallback_room_settings
                    .text_generation
                    .prefix_requirement_type
            })
            .unwrap_or(roomconfig_defaults::TEXT_GENERATION_PREFIX_REQUIREMENT_TYPE)
    }

    pub fn text_to_speech_bot_messages_flow_type(&self) -> TextToSpeechBotMessagesFlowType {
        self.room_config
            .settings
            .text_to_speech
            .bot_msgs_flow_type
            .or({
                self.global_config
                    .fallback_room_settings
                    .text_to_speech
                    .bot_msgs_flow_type
            })
            .unwrap_or(roomconfig_defaults::TEXT_TO_SPEECH_BOT_MESSAGES_FLOW_TYPE)
    }

    pub fn text_to_speech_user_messages_flow_type(&self) -> TextToSpeechUserMessagesFlowType {
        self.room_config
            .settings
            .text_to_speech
            .user_msgs_flow_type
            .or({
                self.global_config
                    .fallback_room_settings
                    .text_to_speech
                    .user_msgs_flow_type
            })
            .unwrap_or(roomconfig_defaults::TEXT_TO_SPEECH_USER_MESSAGES_FLOW_TYPE)
    }

    pub fn text_to_speech_speed_override(&self) -> Option<f32> {
        self.room_config.settings.text_to_speech.speed_override.or({
            self.global_config
                .fallback_room_settings
                .text_to_speech
                .speed_override
        })
    }

    pub fn text_to_speech_voice_override(&self) -> Option<String> {
        self.room_config
            .settings
            .text_to_speech
            .voice_override
            .clone()
            .or_else(|| {
                self.global_config
                    .fallback_room_settings
                    .text_to_speech
                    .voice_override
                    .clone()
            })
    }

    pub fn is_user_allowed_room_local_agent_manager(
        &self,
        user_id: OwnedUserId,
    ) -> mxidwc::Result<bool> {
        match &self.global_config.access.room_local_agent_manager_patterns {
            None => Ok(false),
            Some(patterns) => {
                let allowed_regexes = mxidwc::parse_patterns_vector(patterns)?;

                Ok(mxidwc::match_user_id(user_id.as_str(), &allowed_regexes))
            }
        }
    }
}
