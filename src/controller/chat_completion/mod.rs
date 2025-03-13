pub mod message_aggregator;
pub use message_aggregator::handle;

use mxlink::matrix_sdk::ruma::events::room::message::AudioMessageEventContent;
use mxlink::matrix_sdk::ruma::OwnedEventId;
use mxlink::matrix_sdk::ruma::events::room::message::AudioMessageEventContent;
use mxlink::{MatrixLink, MessageResponseType};

use tracing::Instrument;

use crate::agent::AgentInstance;
use crate::agent::AgentPurpose;
use crate::agent::ControllerTrait;
use crate::agent::provider::{
    SpeechToTextParams, TextGenerationParams, TextGenerationPromptVariables,
};
use crate::controller::utils::agent::get_effective_agent_for_purpose_or_complain;
use crate::conversation::matrix::MatrixMessageProcessingParams;
use crate::entity::MessagePayload;
use crate::repository::Response;
use crate::entity::roomconfig::{
    SpeechToTextFlowType, SpeechToTextMessageTypeForNonThreadedOnlyTranscribedMessages,
    TextToSpeechBotMessagesFlowType, TextToSpeechUserMessagesFlowType,
};
use crate::strings;
use crate::utils::text_to_speech::create_transcribed_message_text;
use crate::{
    Bot,
    conversation::{
        create_llm_conversation_for_matrix_reply_chain, create_llm_conversation_for_matrix_thread,
        matrix::create_list_of_bot_user_prefixes_to_strip,
    },
    entity::MessageContext,
};

#[derive(Debug, PartialEq, Clone)]
pub enum ChatCompletionControllerType {
    // Invoked via a command prefix (e.g. `!bai Hello!`)
    TextCommand,
    // Invoked via a mention (e.g. `@baibot Hello!`)
    TextMention,
    // Invoked via a direct message (e.g. `Hello!`)
    TextDirect,

    Audio,

    ThreadMention,
    ReplyMention,
}

struct TextToSpeechEligiblePayload {
    text: String,
    event_id: OwnedEventId,
}

enum TextToSpeechParams {
    Perform(TextToSpeechEligiblePayload, MessageResponseType),
    Offer(TextToSpeechEligiblePayload, MessageResponseType),
}

pub async fn handle_message(
    bot: &Bot,
    matrix_link: MatrixLink,
    message_context: &MessageContext,
    controller_type: &ChatCompletionControllerType,
) -> anyhow::Result<()> {
    let mut original_message_is_audio = false;

    let mut _typing_notice_guard: Option<mxlink::TypingNoticeGuard> = None;

    let speech_to_text_flow_type = message_context
        .room_config_context()
        .speech_to_text_flow_type();

    let mut speech_to_text_created_event_id: Option<OwnedEventId> = None;

    if let MessagePayload::Audio(audio_content) = &message_context.payload() {
        original_message_is_audio = true;

        let (response_type, msg_type) = match speech_to_text_flow_type {
            SpeechToTextFlowType::Ignore => {
                tracing::debug!("Intentionally ignoring audio message");
                return Ok(());
            }
            SpeechToTextFlowType::TranscribeAndGenerateText => {
                tracing::debug!("Will be transcribing and possibly generating text..");
                (
                    MessageResponseType::InThread(message_context.thread_info().clone()),
                    SpeechToTextMessageTypeForNonThreadedOnlyTranscribedMessages::Notice,
                )
            }
            SpeechToTextFlowType::OnlyTranscribe => {
                tracing::debug!("Will only be transcribing audio to text..");
                if message_context.thread_info().is_thread_root_only() {
                    let msg_type = message_context
                        .room_config_context()
                        .speech_to_text_msg_type_for_non_threaded_only_transcribed_messages();
                    (
                        MessageResponseType::Reply(
                            message_context.thread_info().root_event_id.clone(),
                        ),
                        msg_type,
                    )
                } else {
                    (
                        MessageResponseType::InThread(message_context.thread_info().clone()),
                        SpeechToTextMessageTypeForNonThreadedOnlyTranscribedMessages::Notice,
                    )
                }
            }
        };

        if _typing_notice_guard.is_none() {
            _typing_notice_guard = Some(bot.start_typing_notice(message_context.room()).await);
        }

        let Some(speech_to_text_created_event_id_result) = handle_stage_speech_to_text(
            bot,
            message_context,
            audio_content,
            response_type,
            msg_type,
        )
        .await
        else {
            return Ok(());
        };

        speech_to_text_created_event_id = Some(speech_to_text_created_event_id_result);

        if speech_to_text_flow_type == SpeechToTextFlowType::OnlyTranscribe {
            tracing::debug!(
                "Intentionally not continuing with text generation after transcription"
            );
            return Ok(());
        }

        // We've pushed a transcription to the room.
        // Let's proceed below where we potentially handle text-generation.
    }

    let text_to_speech_stage_params: Option<TextToSpeechParams>;

    if message_context
        .room_config_context()
        .should_auto_text_generate(original_message_is_audio)
    {
        if _typing_notice_guard.is_none() {
            _typing_notice_guard = Some(bot.start_typing_notice(message_context.room()).await);
        }

        let speech_to_text_created_event_id_reaction_event_id =
            if let Some(speech_to_text_created_event_id) = speech_to_text_created_event_id {
                let reaction_event_response = bot
                    .reacting()
                    .react_no_fail(
                        message_context.room(),
                        speech_to_text_created_event_id.clone(),
                        strings::PROGRESS_INDICATOR_EMOJI.to_owned(),
                    )
                    .await;

                reaction_event_response
                    .map(|reaction_event_response| reaction_event_response.event_id)
            } else {
                None
            };

        let response_type = match controller_type {
            // When we're triggered via a reply mention, we reply to the message that triggered us.
            ChatCompletionControllerType::ReplyMention => {
                MessageResponseType::Reply(message_context.thread_info().last_event_id.clone())
            }

            // In all other cases, we're dealing with a threaded conversation, so we reply in the thread.
            _ => MessageResponseType::InThread(message_context.thread_info().clone()),
        };

        let text_to_speech_eligible_payload = handle_stage_text_generation(
            bot,
            matrix_link.clone(),
            message_context,
            controller_type,
            response_type.clone(),
        )
        .await;

        if let Some(speech_to_text_created_event_id_reaction_event_id) =
            speech_to_text_created_event_id_reaction_event_id
        {
            bot.messaging()
                .redact_event_no_fail(
                    message_context.room(),
                    speech_to_text_created_event_id_reaction_event_id,
                    Some("Done".to_owned()),
                )
                .await;
        }

        // If no text was generated (due to some issue), there's no point in continuing.
        let Some(text_to_speech_eligible_payload) = text_to_speech_eligible_payload else {
            return Ok(());
        };

        text_to_speech_stage_params = match message_context
            .room_config_context()
            .text_to_speech_bot_messages_flow_type()
        {
            TextToSpeechBotMessagesFlowType::Never => None,
            TextToSpeechBotMessagesFlowType::OnDemandAlways => Some(TextToSpeechParams::Offer(
                text_to_speech_eligible_payload,
                response_type,
            )),
            TextToSpeechBotMessagesFlowType::OnDemandForVoice => {
                if original_message_is_audio {
                    Some(TextToSpeechParams::Offer(
                        text_to_speech_eligible_payload,
                        response_type,
                    ))
                } else {
                    None
                }
            }
            TextToSpeechBotMessagesFlowType::OnlyForVoice => {
                if original_message_is_audio {
                    Some(TextToSpeechParams::Perform(
                        text_to_speech_eligible_payload,
                        response_type,
                    ))
                } else {
                    None
                }
            }
            TextToSpeechBotMessagesFlowType::Always => Some(TextToSpeechParams::Perform(
                text_to_speech_eligible_payload,
                response_type,
            )),
        };
    } else {
        tracing::debug!("Not generating text due to auto-usage configuration");

        let response_type = MessageResponseType::Reply(message_context.event_id().clone());

        // If we got text from the user, perhaps it's eligible for text-to-speech.

        let MessagePayload::Text(text_payload) = &message_context.payload() else {
            // Audio message, or a notice or something else.
            // We don't wish to proceed with potential TTS for non-text messages.
            return Ok(());
        };

        let text_to_speech_eligible_payload = TextToSpeechEligiblePayload {
            text: text_payload.body.clone(),
            event_id: message_context.event_id().clone(),
        };

        text_to_speech_stage_params = match message_context
            .room_config_context()
            .text_to_speech_user_messages_flow_type()
        {
            TextToSpeechUserMessagesFlowType::Never => None,
            TextToSpeechUserMessagesFlowType::OnDemand => Some(TextToSpeechParams::Offer(
                text_to_speech_eligible_payload,
                response_type,
            )),
            TextToSpeechUserMessagesFlowType::Always => Some(TextToSpeechParams::Perform(
                text_to_speech_eligible_payload,
                response_type,
            )),
        };
    }

    // We're potentially dealing with some text in text_to_speech_eligible_payload - either coming directly from the user or generated by an agent.

    match text_to_speech_stage_params {
        Some(TextToSpeechParams::Perform(text_to_speech_eligible_payload, response_type)) => {
            if _typing_notice_guard.is_none() {
                _typing_notice_guard = Some(bot.start_typing_notice(message_context.room()).await);
            }

            let _tts_result = generate_and_send_tts_for_message(
                bot,
                matrix_link.clone(),
                message_context,
                response_type,
                text_to_speech_eligible_payload.event_id,
                &text_to_speech_eligible_payload.text,
            )
            .await;
        }
        Some(TextToSpeechParams::Offer(text_to_speech_eligible_payload, response_type)) => {
            send_tts_offer_for_message(
                bot,
                message_context,
                response_type,
                text_to_speech_eligible_payload.event_id,
            )
            .await;
        }
        None => {}
    }

    Ok(())
}

async fn handle_stage_speech_to_text(
    bot: &Bot,
    message_context: &MessageContext,
    audio_content: &AudioMessageEventContent,
    response_type: MessageResponseType,
    msg_type: SpeechToTextMessageTypeForNonThreadedOnlyTranscribedMessages,
) -> Option<OwnedEventId> {
    let agent = get_effective_agent_for_purpose_or_complain(
        bot,
        message_context,
        AgentPurpose::SpeechToText,
        response_type.clone(),
        true,
    )
    .await?;

    tracing::debug!(
        agent_id = agent.identifier().as_string(),
        "Handling speech-to-text",
    );

    let reaction_event_response = bot
        .reacting()
        .react_no_fail(
            message_context.room(),
            message_context.event_id().clone(),
            strings::PROGRESS_INDICATOR_EMOJI.to_owned(),
        )
        .await;

    let speech_to_text_created_event_id = handle_stage_speech_to_text_actual_transcribing(
        bot,
        message_context,
        &agent,
        audio_content,
        response_type.clone(),
        msg_type,
    )
    .await;

    if let Some(reaction_event_response) = reaction_event_response {
        let redaction_reason = if speech_to_text_created_event_id.is_ok() {
            strings::speech_to_text::redaction_reason_done()
        } else {
            strings::speech_to_text::redaction_reason_failed()
        };

        bot.messaging()
            .redact_event_no_fail(
                message_context.room(),
                reaction_event_response.event_id,
                Some(redaction_reason.to_owned()),
            )
            .await;
    }

    let speech_to_text_created_event_id = match speech_to_text_created_event_id {
        Ok(event_id) => event_id,
        Err(err) => {
            tracing::warn!(
                "Error in room {} while trying to transcribe via agent {}: {:?}",
                message_context.room_id(),
                agent.identifier(),
                err,
            );

            bot.messaging()
                .send_error_markdown_no_fail(
                    message_context.room(),
                    &strings::agent::error_while_serving_purpose(
                        agent.identifier(),
                        &AgentPurpose::SpeechToText,
                        &err,
                    ),
                    response_type,
                )
                .await;

            return None;
        }
    };

    Some(speech_to_text_created_event_id)
}

async fn handle_stage_text_generation(
    bot: &Bot,
    matrix_link: MatrixLink,
    message_context: &MessageContext,
    controller_type: &ChatCompletionControllerType,
    response_type: MessageResponseType,
) -> Option<TextToSpeechEligiblePayload> {
    let agent = get_effective_agent_for_purpose_or_complain(
        bot,
        message_context,
        AgentPurpose::TextGeneration,
        response_type.clone(),
        true,
    )
    .await?;

    // We only strip text from the first message if we're invoked via a command prefix.
    // Otherwise, we do bot-user mentions stripping on all messages below.
    let first_message_prefixes_to_strip = match controller_type {
        ChatCompletionControllerType::TextCommand => vec![bot.command_prefix().to_owned()],
        _ => vec![],
    };

    let bot_user_prefixes_to_strip = create_list_of_bot_user_prefixes_to_strip(
        bot.user_id(),
        message_context.bot_display_name(),
    );

    let allowed_users = match controller_type {
        // Regular chat completion only operates on messages from allowed users.
        ChatCompletionControllerType::TextCommand
        | ChatCompletionControllerType::TextMention
        | ChatCompletionControllerType::TextDirect
        | ChatCompletionControllerType::Audio => {
            Some(message_context.combined_admin_and_user_regexes())
        }

        // When we're triggered via an explicit mention (thread or reply), we wish to operate against the mention's whole context
        // (the whole thread or the whole reply chain upward of the message that triggered us).
        //
        // This is to allow admins and users to trigger text-generation for other users' messages.
        // When we're dragged into a conversation by a known (to us) user, we'd like to process all messages in the conversation,
        // not just those from allowed users.
        ChatCompletionControllerType::ThreadMention
        | ChatCompletionControllerType::ReplyMention => None,
    };

    let params = MatrixMessageProcessingParams::new(bot.user_id().to_owned(), allowed_users)
        .with_first_message_prefixes_to_strip(first_message_prefixes_to_strip)
        .with_bot_user_prefixes_to_strip(bot_user_prefixes_to_strip);

    let conversation = match controller_type {
        // When we're triggered via a reply mention, the context is the whole reply chain upward of the message that triggered us.
        ChatCompletionControllerType::ReplyMention => {
            create_llm_conversation_for_matrix_reply_chain(
                &bot.room_event_fetcher().clone(),
                message_context.room(),
                message_context.thread_info().last_event_id.clone(),
                &params,
            )
            .await
        }

        // Everything else is happening in a thread, so the context is the whole thread.
        _ => {
            create_llm_conversation_for_matrix_thread(
                matrix_link.clone(),
                message_context.room(),
                message_context.thread_info().root_event_id.clone(),
                &params,
            )
            .await
        }
    };

    let conversation = match conversation {
        Ok(conversation) => conversation,
        Err(err) => {
            tracing::warn!(?err, "Error while trying to create conversation");

            bot.messaging()
                .send_error_markdown_no_fail(
                    message_context.room(),
                    &strings::agent::error_while_serving_purpose(
                        agent.identifier(),
                        &AgentPurpose::TextGeneration,
                        &err,
                    ),
                    response_type,
                )
                .await;

            return None;
        }
    };

    tracing::debug!(
        agent_id = agent.identifier().as_string(),
        provider = format!("{}", agent.definition().provider.clone()),
        "Invoking LLM for text generation with conversation.."
    );

    let span = tracing::debug_span!(
        "text_generation",
        agent_id = agent.identifier().as_string(),
        provider = format!("{}", agent.definition().provider.clone()),
    );

    let start_time = std::time::Instant::now();

    let controller = agent.controller();

    let prompt_variables = TextGenerationPromptVariables::new(
        bot.name(),
        &controller
            .text_generation_model_id()
            .unwrap_or("unknown-model".to_owned()),
        chrono::Utc::now(),
        conversation.start_time(),
    );

    let params = TextGenerationParams {
        context_management_enabled: message_context
            .room_config_context()
            .text_generation_context_management_enabled(),

        prompt_override: message_context
            .room_config_context()
            .text_generation_prompt_override(),

        temperature_override: message_context
            .room_config_context()
            .text_generation_temperature_override(),

        prompt_variables,
    };

    let result = controller
        .generate_text(conversation, params)
        .instrument(span)
        .await;

    let duration = std::time::Instant::now().duration_since(start_time);

    tracing::debug!(
        agent_id = agent.identifier().as_string(),
        provider = format!("{}", agent.definition().provider.clone()),
        ?duration,
        "Done with LLM text generation"
    );

    let result = match result {
        Ok(result) => result,
        Err(err) => {
            tracing::warn!(
                "Error in room {} while trying to generate text via agent {}: {:?}",
                message_context.room_id(),
                agent.identifier(),
                err,
            );

            bot.messaging()
                .send_error_markdown_no_fail(
                    message_context.room(),
                    &strings::agent::error_while_serving_purpose(
                        agent.identifier(),
                        &AgentPurpose::TextGeneration,
                        &err,
                    ),
                    response_type,
                )
                .await;

            return None;
        }
    };

    let text = result.text.clone().trim().to_owned();
    if text.is_empty() {
        tracing::warn!(
            agent_id = agent.identifier().as_string(),
            "Agent returned empty text",
        );

        bot.messaging()
            .send_error_markdown_no_fail(
                message_context.room(),
                &strings::agent::empty_response_returned(agent.identifier()),
                response_type,
            )
            .await;

        return None;
    }

    let _ = bot.repository().store_response(
        Response {
            id: 0,
            bot_id: bot.bot_uniqe_id(),
            length: text.split_whitespace().count() as i64,
            stored_at: chrono::Utc::now().date_naive().to_string()
        }
    ).await;
    
        let send_message_response = bot
        .messaging()
        .send_text_markdown_no_fail(message_context.room(), text.clone(), response_type)
        .await?;

    Some(TextToSpeechEligiblePayload {
        text,
        event_id: send_message_response.event_id,
    })
}

async fn handle_stage_speech_to_text_actual_transcribing(
    bot: &Bot,
    message_context: &MessageContext,
    agent: &AgentInstance,
    audio_content: &AudioMessageEventContent,
    response_type: MessageResponseType,
    msg_type: SpeechToTextMessageTypeForNonThreadedOnlyTranscribedMessages,
) -> anyhow::Result<OwnedEventId> {
    let src = &audio_content.source;

    let media_request = mxlink::matrix_sdk::media::MediaRequestParameters {
        source: src.to_owned(),
        format: mxlink::matrix_sdk::media::MediaFormat::File,
    };

    let media = message_context
        .room()
        .client()
        .media()
        .get_media_content(&media_request, true)
        .await?;

    let span = tracing::debug_span!(
        "speech_to_text_generation",
        agent_id = agent.identifier().as_string()
    );

    let mime_type = audio_content
        .info
        .as_ref()
        .and_then(|info| info.mimetype.clone())
        .unwrap_or_else(|| "audio/ogg".to_string())
        .parse::<mxlink::mime::Mime>()
        .map_err(|err| anyhow::anyhow!("Invalid MIME type: {}", err))?;

    let params = SpeechToTextParams {
        language_override: message_context
            .room_config_context()
            .speech_to_text_language(),
    };

    let speech_to_text_result = agent
        .controller()
        .speech_to_text(&mime_type, media, params)
        .instrument(span)
        .await?;

    // Only use the `> 🦻 Transcribed text` format if we're posting in a thread.
    //
    // If we're dealing with a regular reply (which would be the case in "Transcribe-only mode" = speech-to-text/flow-type=only_transcribe),
    // we don't want to use the `> 🦻 Transcribed text` format for 2 reasons:
    //
    // 1. This kind of blockquote-formatting can be confused by clients for a fallback-for-rich-replies
    //    (see https://spec.matrix.org/v1.11/client-server-api/#fallbacks-for-rich-replies).
    //    It makes certain clients render our messages incorrectly.
    //
    // 2. Transcribe-only mode is typically used for memos. Sticking to a plain-text format
    //    allows people to copy-paste the text or forward it to another room more easily (without having to strip formatting, etc.)
    //
    // When sending a bare reply, we'd better annotate the message with a 🦻 reaction instead,
    // to make it clear to users that it's a transcription.
    let (transcribed_text, annotate_message_with_reaction) =
        if let MessageResponseType::InThread(_) = response_type {
            (
                create_transcribed_message_text(&speech_to_text_result.text),
                false,
            )
        } else {
            (speech_to_text_result.text, true)
        };

    let result = match msg_type {
        SpeechToTextMessageTypeForNonThreadedOnlyTranscribedMessages::Text => {
            bot.messaging()
                .send_text_markdown_no_fail(message_context.room(), transcribed_text, response_type)
                .await
        }
        SpeechToTextMessageTypeForNonThreadedOnlyTranscribedMessages::Notice => {
            bot.messaging()
                .send_notice_markdown_no_fail(
                    message_context.room(),
                    transcribed_text,
                    response_type,
                )
                .await
        }
    };

    let event_id = result
        .map(|result| result.event_id)
        .ok_or_else(|| anyhow::anyhow!("Failed to send transcribed text"))?;

    if annotate_message_with_reaction {
        bot.reacting()
            .react_no_fail(
                message_context.room(),
                event_id.clone(),
                AgentPurpose::SpeechToText.emoji().to_owned(),
            )
            .await;
    }

    Ok(event_id)
}

async fn send_tts_offer_for_message(
    bot: &Bot,
    message_context: &MessageContext,
    response_type: MessageResponseType,
    event_id: OwnedEventId,
) {
    // Offers may be enabled, but there's no guarantee that whatever agent is configured can actually do TTS.
    // So.. do not complain if there's no agent available. Just silently ignore it.
    let speech_agent = get_effective_agent_for_purpose_or_complain(
        bot,
        message_context,
        AgentPurpose::TextToSpeech,
        response_type,
        false,
    )
    .await;

    if speech_agent.is_some() {
        bot.reacting()
            .react_no_fail(
                message_context.room(),
                event_id,
                AgentPurpose::TextToSpeech.emoji().to_owned(),
            )
            .await;
    }
}

async fn generate_and_send_tts_for_message(
    bot: &Bot,
    matrix_link: MatrixLink,
    message_context: &MessageContext,
    response_type: MessageResponseType,
    event_id: OwnedEventId,
    text: &str,
) -> bool {
    let speech_agent = get_effective_agent_for_purpose_or_complain(
        bot,
        message_context,
        AgentPurpose::TextToSpeech,
        response_type.clone(),
        true,
    )
    .await;

    let Some(speech_agent) = speech_agent else {
        return false;
    };

    crate::controller::utils::text_to_speech::generate_and_send_tts_for_message(
        bot,
        matrix_link,
        message_context,
        response_type,
        &speech_agent,
        &event_id,
        text,
    )
    .await
}
