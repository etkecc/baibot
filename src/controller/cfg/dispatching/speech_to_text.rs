use crate::entity::roomconfig::{RoomSettings, SpeechToTextFlowType};
use crate::{entity::MessageContext, Bot};

use super::super::controller_type::{
    ConfigSpeechToTextSettingRelatedControllerType, SettingsStorageSource,
};

use super::super::common::generic_setting::handle_get as setting_get;

use super::super::global_config::generic_setting::handle_set as global_setting_set;

use super::super::room_config::generic_setting::handle_set as room_setting_set;

pub(super) async fn dispatch(
    handler: &ConfigSpeechToTextSettingRelatedControllerType,
    message_context: &MessageContext,
    bot: &Bot,
    room_settings: &RoomSettings,
    config_type: &SettingsStorageSource,
) -> anyhow::Result<()> {
    match handler {
        ConfigSpeechToTextSettingRelatedControllerType::GetFlowType => {
            let value = &room_settings.speech_to_text.flow_type;
            setting_get::<SpeechToTextFlowType>(bot, message_context, value).await
        }
        ConfigSpeechToTextSettingRelatedControllerType::SetFlowType(value) => {
            let value = value.to_owned();

            let setter_callback = Box::new(move |room_settings: &mut RoomSettings| {
                room_settings.speech_to_text.flow_type = value;
            });

            match config_type {
                SettingsStorageSource::Room => {
                    room_setting_set::<SpeechToTextFlowType>(
                        bot,
                        message_context,
                        &value,
                        setter_callback,
                    )
                    .await
                }
                SettingsStorageSource::Global => {
                    global_setting_set::<SpeechToTextFlowType>(
                        bot,
                        message_context,
                        &value,
                        setter_callback,
                    )
                    .await
                }
            }
        }

        ConfigSpeechToTextSettingRelatedControllerType::GetLanguage => {
            let value = &room_settings.speech_to_text.language;
            setting_get::<String>(bot, message_context, value).await
        }
        ConfigSpeechToTextSettingRelatedControllerType::SetLanguage(value) => {
            let value = value.to_owned();

            let value_setter = value.clone();
            let setter_callback = Box::new(move |room_settings: &mut RoomSettings| {
                room_settings.speech_to_text.language = value_setter;
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
