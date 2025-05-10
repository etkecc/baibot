use mxlink::MessageResponseType;

use crate::{Bot, entity::MessageContext, strings};

use super::ControllerType;

pub async fn dispatch_controller(
    controller_type: &ControllerType,
    message_context: &MessageContext,
    bot: &Bot,
) {
    let result = match controller_type {
        ControllerType::Access(controller_type) => {
            super::access::dispatch_controller(controller_type, message_context, bot).await
        }
        ControllerType::Agent(controller_type) => {
            super::agent::dispatch_controller(controller_type, message_context, bot).await
        }
        ControllerType::Config(controller_type) => {
            super::cfg::dispatch_controller(controller_type, message_context, bot).await
        }
        ControllerType::Help => super::help::handle(bot, message_context).await,
        ControllerType::Unknown => {
            bot.messaging()
                .send_error_markdown_no_fail(
                    message_context.room(),
                    &strings::error::unknown_command_see_help(bot.command_prefix()),
                    MessageResponseType::Reply(message_context.thread_info().root_event_id.clone()),
                )
                .await;

            Ok(())
        }
        ControllerType::ProviderHelp => super::provider::handle_help(message_context, bot).await,
        ControllerType::UsageHelp => super::usage::handle_help(message_context, bot).await,
        ControllerType::ChatCompletion(controller_type) => {
            super::chat_completion::handle(
                bot,
                bot.matrix_link().clone(),
                message_context,
                controller_type,
            )
            .await
        }
        ControllerType::Error(message) => {
            bot.messaging()
                .send_error_markdown_no_fail(
                    message_context.room(),
                    message,
                    MessageResponseType::Reply(message_context.thread_info().root_event_id.clone()),
                )
                .await;

            Ok(())
        }
        ControllerType::ErrorInThread(message, thread_info) => {
            bot.messaging()
                .send_error_markdown_no_fail(
                    message_context.room(),
                    message,
                    MessageResponseType::InThread(thread_info.clone()),
                )
                .await;

            Ok(())
        }
        ControllerType::Ignore => {
            tracing::trace!("Ignoring text message");
            Ok(())
        }
        ControllerType::ImageGeneration(prompt) => {
            super::image::generation::handle_image(
                bot,
                bot.matrix_link().clone(),
                message_context,
                prompt,
            )
            .await
        }
        ControllerType::ImageEdit(prompt) => {
            super::image::edit::handle(
                bot,
                bot.matrix_link().clone(),
                message_context,
                prompt,
            )
            .await
        }
        ControllerType::StickerGeneration(prompt) => {
            super::image::generation::handle_sticker(
                bot,
                bot.matrix_link().clone(),
                message_context,
                prompt,
            )
            .await
        }
    };

    if let Err(e) = result {
        tracing::error!(
            "Error handling message {} from sender {} in room {}: {:?}",
            message_context.event_id(),
            message_context.sender_id(),
            message_context.room_id(),
            e,
        );

        bot.messaging()
            .send_error_markdown_no_fail(
                message_context.room(),
                strings::error::error_while_processing_message(),
                MessageResponseType::Reply(message_context.thread_info().root_event_id.clone()),
            )
            .await;
    }
}
