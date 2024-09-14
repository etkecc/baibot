use mxlink::{MatrixLink, MessageResponseType};

use tracing::Instrument;

use crate::agent::provider::ImageGenerationParams;
use crate::agent::AgentPurpose;
use crate::agent::ControllerTrait;
use crate::controller::utils::agent::get_effective_agent_for_purpose_or_complain;
use crate::conversation::create_llm_conversation_for_matrix_thread;
use crate::conversation::matrix::MatrixMessageProcessingParams;
use crate::strings;
use crate::{entity::MessageContext, Bot};

// We may make this configurable (per room, etc.) in the future, but for now it's hardcoded.
const STICKER_SIZE: &str = "256x256";

pub async fn handle_image(
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

    let params = MatrixMessageProcessingParams::new(
        bot.user_id().as_str().to_owned(),
        message_context.combined_admin_and_user_regexes(),
    );

    let conversation = create_llm_conversation_for_matrix_thread(
        matrix_link.clone(),
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

    let span = tracing::debug_span!(
        "image_generation",
        agent_id = agent.identifier().as_string()
    );

    let response = agent
        .controller()
        .generate_image(&prompt, ImageGenerationParams::default())
        .instrument(span)
        .await?;

    let actual_prompt = response.revised_prompt.as_deref().unwrap_or(&prompt);

    if *actual_prompt.trim() != *prompt.trim() {
        bot.messaging()
            .send_notice_markdown_no_fail(
                message_context.room(),
                strings::image_generation::revised_prompt(actual_prompt),
                response_type.clone(),
            )
            .await;
    }

    let attachment_body_text = format!("Generated image based on: {}", actual_prompt);

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

    if conversation.messages.len() == 1 {
        // If this is the beginning of the thread, send helpful instructions
        bot.messaging()
            .send_notice_markdown_no_fail(
                message_context.room(),
                strings::image_generation::guide_how_to_proceed(),
                response_type.clone(),
            )
            .await;
    }

    Ok(())
}

pub async fn handle_sticker(
    bot: &Bot,
    matrix_link: MatrixLink,
    message_context: &MessageContext,
    original_prompt: &str,
) -> anyhow::Result<()> {
    // Stickers are always sent directly to the room - no threading.
    let response_type =
        MessageResponseType::Reply(message_context.thread_info().root_event_id.clone());

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

    let _typing_notice_guard = bot.start_typing_notice(message_context.room()).await;

    let span = tracing::debug_span!(
        "sticker_generation",
        agent_id = agent.identifier().as_string()
    );

    let params = ImageGenerationParams::default()
        .with_size_override(Some(STICKER_SIZE.to_owned()))
        .with_cheaper_model_switching_allowed(true)
        .with_cheaper_quality_switching_allowed(true);

    let response = agent
        .controller()
        .generate_image(original_prompt, params)
        .instrument(span)
        .await?;

    let attachment_body_text = format!("Generated sticker image based on: {}", original_prompt);

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
        .send_event(message_context.room(), &mut event_content, response_type)
        .await?;

    Ok(())
}
