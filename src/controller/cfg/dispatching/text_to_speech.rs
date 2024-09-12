use crate::entity::roomconfig::{
    RoomSettings, TextToSpeechBotMessagesFlowType, TextToSpeechUserMessagesFlowType,
};
use crate::{entity::MessageContext, Bot};

use super::super::controller_type::{
    ConfigTextToSpeechSettingRelatedControllerType, SettingsStorageSource,
};

use super::super::common::generic_setting::handle_get as setting_get;

use super::super::global_config::generic_setting::handle_set as global_setting_set;

use super::super::room_config::generic_setting::handle_set as room_setting_set;

pub(super) async fn dispatch(
    handler: &ConfigTextToSpeechSettingRelatedControllerType,
    message_context: &MessageContext,
    bot: &Bot,
    room_settings: &RoomSettings,
    config_type: &SettingsStorageSource,
) -> anyhow::Result<()> {
    match handler {
        ConfigTextToSpeechSettingRelatedControllerType::GetBotMessagesFlowType => {
            let value = &room_settings.text_to_speech.bot_msgs_flow_type;
            setting_get::<TextToSpeechBotMessagesFlowType>(bot, message_context, value).await
        }
        ConfigTextToSpeechSettingRelatedControllerType::SetBotMessagesFlowType(value) => {
            let value = value.to_owned();

            let setter_callback = Box::new(move |room_settings: &mut RoomSettings| {
                room_settings.text_to_speech.bot_msgs_flow_type = value;
            });

            match config_type {
                SettingsStorageSource::Room => {
                    room_setting_set::<TextToSpeechBotMessagesFlowType>(
                        bot,
                        message_context,
                        &value,
                        setter_callback,
                    )
                    .await
                }
                SettingsStorageSource::Global => {
                    global_setting_set::<TextToSpeechBotMessagesFlowType>(
                        bot,
                        message_context,
                        &value,
                        setter_callback,
                    )
                    .await
                }
            }
        }

        ConfigTextToSpeechSettingRelatedControllerType::GetUserMessagesFlowType => {
            let value = &room_settings.text_to_speech.user_msgs_flow_type;
            setting_get::<TextToSpeechUserMessagesFlowType>(bot, message_context, value).await
        }
        ConfigTextToSpeechSettingRelatedControllerType::SetUserMessagesFlowType(value) => {
            let value = value.to_owned();

            let setter_callback = Box::new(move |room_settings: &mut RoomSettings| {
                room_settings.text_to_speech.user_msgs_flow_type = value;
            });

            match config_type {
                SettingsStorageSource::Room => {
                    room_setting_set::<TextToSpeechUserMessagesFlowType>(
                        bot,
                        message_context,
                        &value,
                        setter_callback,
                    )
                    .await
                }
                SettingsStorageSource::Global => {
                    global_setting_set::<TextToSpeechUserMessagesFlowType>(
                        bot,
                        message_context,
                        &value,
                        setter_callback,
                    )
                    .await
                }
            }
        }

        ConfigTextToSpeechSettingRelatedControllerType::GetSpeedOverride => {
            let value = &room_settings.text_to_speech.speed_override;
            setting_get::<f32>(bot, message_context, value).await
        }
        ConfigTextToSpeechSettingRelatedControllerType::SetSpeedOverride(value) => {
            let value = value.to_owned();

            let setter_callback = Box::new(move |room_settings: &mut RoomSettings| {
                room_settings.text_to_speech.speed_override = value;
            });

            match config_type {
                SettingsStorageSource::Room => {
                    room_setting_set::<f32>(bot, message_context, &value, setter_callback).await
                }
                SettingsStorageSource::Global => {
                    global_setting_set::<f32>(bot, message_context, &value, setter_callback).await
                }
            }
        }

        ConfigTextToSpeechSettingRelatedControllerType::GetVoiceOverride => {
            let value = &room_settings.text_to_speech.voice_override;
            setting_get::<String>(bot, message_context, value).await
        }
        ConfigTextToSpeechSettingRelatedControllerType::SetVoiceOverride(value) => {
            let value = value.to_owned();

            let value_setter = value.clone();
            let setter_callback = Box::new(move |room_settings: &mut RoomSettings| {
                room_settings.text_to_speech.voice_override = value_setter;
            });

            match config_type {
                SettingsStorageSource::Room => {
                    room_setting_set::<String>(bot, message_context, &value, setter_callback).await
                }
                SettingsStorageSource::Global => {
                    global_setting_set::<String>(bot, message_context, &value, setter_callback)
                        .await
                }
            }
        }
    }
}
