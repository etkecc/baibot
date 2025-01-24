use mxlink::matrix_sdk::ruma::OwnedEventId;
use mxlink::{MatrixLink, MessageResponseType};

use tracing::Instrument;

use crate::controller::utils::mime::get_file_extension;
use crate::{
    agent::{provider::TextToSpeechParams, AgentInstance, AgentPurpose, ControllerTrait},
    entity::MessageContext,
    strings, Bot,
};

pub async fn generate_and_send_tts_for_message(
    bot: &Bot,
    matrix_link: MatrixLink,
    message_context: &MessageContext,
    response_type: MessageResponseType,
    speech_agent: &AgentInstance,
    text_message_event_id: &OwnedEventId,
    text_content: &str,
) -> bool {
    let reaction_event_response = bot
        .reacting()
        .react_no_fail(
            message_context.room(),
            text_message_event_id.clone(),
            strings::PROGRESS_INDICATOR_EMOJI.to_owned(),
        )
        .await;

    let result = do_generate_and_send_tts_for_message(
        bot,
        matrix_link,
        message_context,
        response_type,
        speech_agent,
        text_content,
    )
    .await;

    if let Some(reaction_event_response) = reaction_event_response {
        let redaction_reason = if result {
            strings::text_to_speech::redaction_reason_done()
        } else {
            strings::text_to_speech::redaction_reason_failed()
        };

        bot.messaging()
            .redact_event_no_fail(
                message_context.room(),
                reaction_event_response.event_id,
                Some(redaction_reason.to_owned()),
            )
            .await;
    }

    result
}

async fn do_generate_and_send_tts_for_message(
    bot: &Bot,
    matrix_link: MatrixLink,
    message_context: &MessageContext,
    response_type: MessageResponseType,
    speech_agent: &AgentInstance,
    text_content: &str,
) -> bool {
    let params = TextToSpeechParams {
        speed_override: message_context
            .room_config_context()
            .text_to_speech_speed_override(),

        voice_override: message_context
            .room_config_context()
            .text_to_speech_voice_override(),
    };

    let text_content = if let Some(text_content) = text_content.strip_prefix(bot.command_prefix()) {
        text_content.trim()
    } else {
        text_content
    };

    let span = tracing::debug_span!(
        "text_to_speech_generation",
        agent_id = speech_agent.identifier().as_string()
    );

    let text_to_speech_result = speech_agent
        .controller()
        .text_to_speech(text_content, params)
        .instrument(span)
        .await;

    let text_to_speech_result = match text_to_speech_result {
        Ok(text_to_speech_result) => text_to_speech_result,
        Err(err) => {
            tracing::warn!(
                "Error in room {} while trying to generate TTS via agent {}: {:?}",
                message_context.room_id(),
                speech_agent.identifier(),
                err,
            );

            bot.messaging()
                .send_error_markdown_no_fail(
                    message_context.room(),
                    &strings::agent::error_while_serving_purpose(
                        speech_agent.identifier(),
                        &AgentPurpose::SpeechToText,
                        &err,
                    ),
                    response_type,
                )
                .await;

            return false;
        }
    };

    let attachment_body_text = format!("generated-speech.{}", get_file_extension(&text_to_speech_result.mime_type));

    let event_content = matrix_link
        .media()
        .upload_and_prepare_event_content(
            message_context.room(),
            &text_to_speech_result.mime_type,
            text_to_speech_result.bytes,
            &attachment_body_text,
        )
        .await;

    let mut event_content = match event_content {
        Ok(event_content) => event_content,
        Err(err) => {
            tracing::error!(
                ?err,
                "Error in room {} while trying to upload TTS via agent {}",
                message_context.room_id(),
                speech_agent.identifier(),
            );

            return false;
        }
    };

    let result = matrix_link
        .messaging()
        .send_event(
            message_context.room(),
            &mut event_content,
            response_type.clone(),
        )
        .await;

    let Err(err) = result else {
        return true;
    };

    tracing::error!(
        ?err,
        "Error in room {} while trying to send TTS payload",
        message_context.room_id(),
    );

    false
}
