use mxlink::{MatrixLink, MessageResponseType};

use tracing::Instrument;

use crate::agent::provider::ImageSource;
use crate::agent::AgentPurpose;
use crate::agent::ControllerTrait;
use crate::agent::provider::ImageEditParams;
use crate::controller::utils::agent::get_effective_agent_for_purpose_or_complain;
use crate::utils::mime::get_file_extension;
use crate::conversation::create_llm_conversation_for_matrix_thread;
use crate::conversation::matrix::MatrixMessageProcessingParams;
use crate::strings;
use crate::{Bot, entity::MessageContext};

pub async fn handle(
    bot: &Bot,
    matrix_link: MatrixLink,
    message_context: &MessageContext,
    original_prompt: &str,
) -> anyhow::Result<()> {
    let response_type = MessageResponseType::InThread(message_context.thread_info().clone());

    let Some(agent) = get_effective_agent_for_purpose_or_complain(
        bot,
        message_context,
        AgentPurpose::ImageGeneration,
        response_type.clone(),
        true,
    )
    .await
    else {
        return Ok(());
    };

    if message_context.thread_info().is_thread_root_only() {
        return send_guide(bot, message_context).await;
    }

    let _typing_notice_guard = bot.start_typing_notice(message_context.room()).await;

    let params = MatrixMessageProcessingParams::new(
        bot.user_id().to_owned(),
        Some(message_context.combined_admin_and_user_regexes()),
    );

    let conversation = create_llm_conversation_for_matrix_thread(
        &matrix_link,
        message_context.room(),
        message_context.thread_info().root_event_id.clone(),
        &params,
    )
    .await?;

    let prompt = if conversation.messages.len() >= 2 {
        // Skip the first message, which contains the original prompt (which we already have)
        let other_messages = conversation.messages.iter().skip(1).cloned().collect();

        super::prompt::build(original_prompt, other_messages)
    } else {
        original_prompt.to_owned()
    };

    let got_go_signal = conversation.messages.iter().any(|message| {
        if let crate::conversation::llm::MessageContent::Text(text) = &message.content {
            text.to_lowercase() == "go"
        } else {
            false
        }
    });

    let image_sources: Vec<ImageSource> = conversation.messages.iter().filter_map(|message| {
        if let crate::conversation::llm::MessageContent::Image(image_content) = &message.content {
            Some(image_content.into())
        } else {
            None
        }
    }).collect();

    if !got_go_signal || image_sources.is_empty() {
        // We don't send the guide again here to avoid being annoying.
        return Ok(());
    }

    let span = tracing::debug_span!(
        "image_edit",
        agent_id = agent.identifier().as_string()
    );

    let result = agent
        .controller()
        .create_image_edit(&prompt, image_sources, ImageEditParams::default())
        .instrument(span)
        .await;

    let response = match result {
        Ok(response) => response,
        Err(err) => {
            tracing::warn!(
                "Error in room {} while trying to generate image edit via agent {}: {:?}",
                message_context.room_id(),
                agent.identifier(),
                err,
            );

            bot.messaging()
                .send_error_markdown_no_fail(
                    message_context.room(),
                    &strings::agent::error_while_serving_purpose(
                        agent.identifier(),
                        &AgentPurpose::ImageGeneration,
                        &err,
                    ),
                    response_type,
                )
                .await;

            return Ok(());
        }
    };

    let attachment_body_text = format!(
        "generated-image-edit.{}",
        get_file_extension(&response.mime_type)
    );

    let mut event_content = matrix_link
        .media()
        .upload_and_prepare_event_content(
            message_context.room(),
            &response.mime_type,
            response.bytes,
            &attachment_body_text,
        )
        .await
        .map_err(|e| anyhow::anyhow!("Failed to upload and prepare event: {}", e))?;

    matrix_link
        .messaging()
        .send_event(
            message_context.room(),
            &mut event_content,
            response_type.clone(),
        )
        .await?;

    Ok(())
}

async fn send_guide(
    bot: &Bot,
    message_context: &MessageContext,
) -> anyhow::Result<()> {
    bot.messaging()
        .send_text_markdown_no_fail(
            message_context.room(),
            strings::image_edit::guide_how_to_proceed(),
            MessageResponseType::InThread(message_context.thread_info().clone()),
        )
        .await;

    Ok(())
}
