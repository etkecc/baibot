use crate::{Bot, entity::MessageContext};

use super::controller_type::{
    ConfigControllerType, ConfigSettingRelatedControllerType, SettingsStorageSource,
};

mod speech_to_text;
mod text_generation;
mod text_to_speech;

pub async fn dispatch_controller(
    handler: &ConfigControllerType,
    message_context: &MessageContext,
    bot: &Bot,
) -> anyhow::Result<()> {
    // Access checks are done centrally, before dispatching (see `ConfigControllerType::required_permission()`).

    match handler {
        ConfigControllerType::Help => super::help::handle(bot, message_context).await,
        ConfigControllerType::Status => super::status::handle(bot, message_context).await,
        ConfigControllerType::SettingsRelated(config_type, config_related_handler) => {
            dispatch_config_related_handler(
                config_type,
                config_related_handler,
                message_context,
                bot,
            )
            .await
        }
    }
}

async fn dispatch_config_related_handler(
    config_type: &SettingsStorageSource,
    handler: &ConfigSettingRelatedControllerType,
    message_context: &MessageContext,
    bot: &Bot,
) -> anyhow::Result<()> {
    let room_settings = match config_type {
        SettingsStorageSource::Room => &message_context.room_config().settings,
        SettingsStorageSource::Global => &message_context.global_config().fallback_room_settings,
    };

    match handler {
        ConfigSettingRelatedControllerType::GetHandler(purpose) => match config_type {
            SettingsStorageSource::Room => {
                super::room_config::handler::handle_get(bot, message_context, *purpose).await
            }
            SettingsStorageSource::Global => {
                super::global_config::handler::handle_get(bot, message_context, *purpose).await
            }
        },
        ConfigSettingRelatedControllerType::SetHandler(purpose, agent_identifier) => {
            match config_type {
                SettingsStorageSource::Room => {
                    super::room_config::handler::handle_set(
                        bot,
                        bot.room_config_manager(),
                        message_context,
                        *purpose,
                        agent_identifier,
                    )
                    .await
                }
                SettingsStorageSource::Global => {
                    super::global_config::handler::handle_set(
                        bot,
                        bot.global_config_manager(),
                        message_context,
                        *purpose,
                        agent_identifier,
                    )
                    .await
                }
            }
        }
        ConfigSettingRelatedControllerType::TextGeneration(controller_type) => {
            text_generation::dispatch(
                controller_type,
                message_context,
                bot,
                room_settings,
                config_type,
            )
            .await
        }
        ConfigSettingRelatedControllerType::SpeechToText(controller_type) => {
            speech_to_text::dispatch(
                controller_type,
                message_context,
                bot,
                room_settings,
                config_type,
            )
            .await
        }
        ConfigSettingRelatedControllerType::TextToSpeech(controller_type) => {
            text_to_speech::dispatch(
                controller_type,
                message_context,
                bot,
                room_settings,
                config_type,
            )
            .await
        }
    }
}
