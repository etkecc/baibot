use crate::entity::roomconfig::{
    RoomSettings, TextGenerationAutoUsage, TextGenerationPrefixRequirementType,
};
use crate::{entity::MessageContext, Bot};

use super::super::controller_type::{
    ConfigTextGenerationSettingRelatedControllerType, SettingsStorageSource,
};

use super::super::common::generic_setting::handle_get as setting_get;

use super::super::global_config::generic_setting::handle_set as global_setting_set;

use super::super::room_config::generic_setting::handle_set as room_setting_set;

pub(super) async fn dispatch(
    handler: &ConfigTextGenerationSettingRelatedControllerType,
    message_context: &MessageContext,
    bot: &Bot,
    room_settings: &RoomSettings,
    config_type: &SettingsStorageSource,
) -> anyhow::Result<()> {
    match handler {
        ConfigTextGenerationSettingRelatedControllerType::GetContextManagementEnabled => {
            let value = &room_settings.text_generation.context_management_enabled;
            setting_get::<bool>(bot, message_context, value).await
        }
        ConfigTextGenerationSettingRelatedControllerType::SetContextManagementEnabled(value) => {
            let value = value.to_owned();

            let setter_callback = Box::new(move |room_settings: &mut RoomSettings| {
                room_settings.text_generation.context_management_enabled = value;
            });

            match config_type {
                SettingsStorageSource::Room => {
                    room_setting_set::<bool>(bot, message_context, &value, setter_callback).await
                }
                SettingsStorageSource::Global => {
                    global_setting_set::<bool>(bot, message_context, &value, setter_callback).await
                }
            }
        }

        ConfigTextGenerationSettingRelatedControllerType::GetPrefixRequirementType => {
            let value = &room_settings.text_generation.prefix_requirement_type;
            setting_get::<TextGenerationPrefixRequirementType>(bot, message_context, value).await
        }
        ConfigTextGenerationSettingRelatedControllerType::SetPrefixRequirementType(value) => {
            let value = value.to_owned();

            let setter_callback = Box::new(move |room_settings: &mut RoomSettings| {
                room_settings.text_generation.prefix_requirement_type = value;
            });

            match config_type {
                SettingsStorageSource::Room => {
                    room_setting_set::<TextGenerationPrefixRequirementType>(
                        bot,
                        message_context,
                        &value,
                        setter_callback,
                    )
                    .await
                }
                SettingsStorageSource::Global => {
                    global_setting_set::<TextGenerationPrefixRequirementType>(
                        bot,
                        message_context,
                        &value,
                        setter_callback,
                    )
                    .await
                }
            }
        }

        ConfigTextGenerationSettingRelatedControllerType::GetAutoUsage => {
            let value = &room_settings.text_generation.auto_usage;
            setting_get::<TextGenerationAutoUsage>(bot, message_context, value).await
        }
        ConfigTextGenerationSettingRelatedControllerType::SetAutoUsage(value) => {
            let value = value.to_owned();

            let setter_callback = Box::new(move |room_settings: &mut RoomSettings| {
                room_settings.text_generation.auto_usage = value;
            });

            match config_type {
                SettingsStorageSource::Room => {
                    room_setting_set::<TextGenerationAutoUsage>(
                        bot,
                        message_context,
                        &value,
                        setter_callback,
                    )
                    .await
                }
                SettingsStorageSource::Global => {
                    global_setting_set::<TextGenerationAutoUsage>(
                        bot,
                        message_context,
                        &value,
                        setter_callback,
                    )
                    .await
                }
            }
        }

        ConfigTextGenerationSettingRelatedControllerType::GetPromptOverride => {
            let value = &room_settings.text_generation.prompt_override;
            setting_get::<String>(bot, message_context, value).await
        }
        ConfigTextGenerationSettingRelatedControllerType::SetPromptOverride(value) => {
            let value = value.to_owned();

            let value_setter = value.clone();
            let setter_callback = Box::new(move |room_settings: &mut RoomSettings| {
                room_settings.text_generation.prompt_override = value_setter;
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

        ConfigTextGenerationSettingRelatedControllerType::GetTemperatureOverride => {
            let value = &room_settings.text_generation.temperature_override;
            setting_get::<f32>(bot, message_context, value).await
        }
        ConfigTextGenerationSettingRelatedControllerType::SetTemperatureOverride(value) => {
            let value = value.to_owned();

            let setter_callback = Box::new(move |room_settings: &mut RoomSettings| {
                room_settings.text_generation.temperature_override = value;
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
    }
}
