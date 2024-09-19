use crate::strings;
use crate::{entity::MessageContext, Bot};
use mxlink::MessageResponseType;

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
    // Anyone can access Help and Status.
    // Settings-related access checks are done in dispatch_config_related_handler().

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
    if let SettingsStorageSource::Global = config_type {
        if !message_context.sender_can_manage_global_config() {
            bot.messaging()
                .send_error_markdown_no_fail(
                    message_context.room(),
                    strings::global_config::no_permissions_to_administrate(),
                    MessageResponseType::Reply(message_context.thread_info().root_event_id.clone()),
                )
                .await;
            return Ok(());
        }
    };

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
