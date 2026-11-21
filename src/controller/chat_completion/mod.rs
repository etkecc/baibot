use mxlink::matrix_sdk::ruma::OwnedEventId;
use mxlink::matrix_sdk::ruma::events::room::message::AudioMessageEventContent;
use mxlink::{MatrixLink, MessageResponseType, ThreadInfo};

use tracing::Instrument;

use crate::agent::AgentInstance;
use crate::agent::AgentPurpose;
use crate::agent::ControllerTrait;
use crate::agent::provider::{
    SpeechToTextParams, TextGenerationParams, TextGenerationPromptVariables,
};
use crate::authorization::Permission;
use crate::controller::utils::agent::get_effective_agent_for_purpose_or_complain;
use crate::conversation::matrix::MatrixMessageProcessingParams;
use crate::entity::MessagePayload;
use crate::entity::roomconfig::{
    SpeechToTextFlowType, SpeechToTextMessageTypeForNonThreadedOnlyTranscribedMessages,
    TextGenerationReplyStyle, TextGenerationSenderContextMode, TextToSpeechBotMessagesFlowType,
    TextToSpeechUserMessagesFlowType,
};
use crate::strings;
use crate::utils::text_to_speech::create_transcribed_message_text;
use crate::{
    Bot,
    conversation::{
        create_llm_conversation_for_matrix_reply_chain, create_llm_conversation_for_matrix_thread,
        llm::{Author, Conversation, MessageContent},
        matrix::create_list_of_bot_user_prefixes_to_strip,
    },
    entity::MessageContext,
};

/// How long text generation must run before the first "thinking…" placeholder appears.
/// Fast responses (under this threshold) never get a placeholder.
const THINKING_NOTICE_FIRST_DELAY: std::time::Duration = std::time::Duration::from_secs(3);

/// How often the "thinking…" placeholder is refreshed once it has appeared.
const THINKING_NOTICE_INTERVAL: std::time::Duration = std::time::Duration::from_secs(10);

#[derive(Debug, PartialEq)]
pub enum ChatCompletionControllerType {
    // Invoked via a command prefix (e.g. `!bai Hello!`)
    TextCommand,
    // Invoked via a mention (e.g. `@baibot Hello!`)
    TextMention,
    // Invoked via a direct message (e.g. `Hello!`)
    TextDirect,

    Audio,

    Image,

    File,

    ThreadMention,
    ReplyMention,
}

impl ChatCompletionControllerType {
    /// Tells whether the user addressed the bot explicitly (command prefix or mention),
    /// as opposed to the bot reacting to a message on its own (depending on room settings).
    fn is_explicit_invocation(&self) -> bool {
        match self {
            Self::TextCommand | Self::TextMention | Self::ThreadMention | Self::ReplyMention => {
                true
            }
            Self::TextDirect | Self::Audio | Self::Image | Self::File => false,
        }
    }
}

struct TextToSpeechEligiblePayload {
    text: String,
    event_id: OwnedEventId,
}

enum TextToSpeechParams {
    Perform(TextToSpeechEligiblePayload, MessageResponseType),
    Offer(TextToSpeechEligiblePayload, MessageResponseType),
}

pub async fn handle(
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

        if !message_context.sender_can(Permission::SpeechToText) {
            tracing::debug!(
                "Ignoring audio message from a user without the speech-to-text permission"
            );
            return Ok(());
        }

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

    let should_generate_text = message_context
        .room_config_context()
        .should_auto_text_generate(original_message_is_audio);

    let sender_can_generate_text = message_context.sender_can(Permission::TextGeneration);

    if should_generate_text && !sender_can_generate_text {
        if controller_type.is_explicit_invocation() {
            bot.messaging()
                .send_error_markdown_no_fail(
                    message_context.room(),
                    &strings::access::missing_permission(
                        Permission::TextGeneration.emoji(),
                        Permission::TextGeneration.name(),
                    ),
                    MessageResponseType::Reply(message_context.event_id().clone()),
                )
                .await;

            return Ok(());
        }

        // Not falling back to the flow for rooms which don't generate text automatically,
        // which would offer text-to-speech for the user's own message instead.
        tracing::debug!("Not generating text for a user without the text-generation permission");
        return Ok(());
    }

    let text_to_speech_stage_params: Option<TextToSpeechParams> =
        if should_generate_text && sender_can_generate_text {
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

            let response_type = determine_text_generation_response_type(
                controller_type,
                message_context.thread_info(),
                message_context
                    .room_config_context()
                    .text_generation_reply_style(),
            );

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

            // In rooms preferring replies, the audio replies to the generated answer (not to the user's message),
            // so that the reply chain (and thus the conversation context) keeps the answer when someone replies to the audio.
            let response_type = match response_type {
                MessageResponseType::Reply(_)
                    if message_context
                        .room_config_context()
                        .text_generation_reply_style()
                        == TextGenerationReplyStyle::Reply =>
                {
                    MessageResponseType::Reply(text_to_speech_eligible_payload.event_id.clone())
                }
                other => other,
            };

            match message_context
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
            }
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

            match message_context
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
            }
        };

    // We're potentially dealing with some text in text_to_speech_eligible_payload - either coming directly from the user or generated by an agent.

    match text_to_speech_stage_params {
        Some(TextToSpeechParams::Perform(_, _))
            if !message_context.sender_can(Permission::TextToSpeech) =>
        {
            tracing::debug!(
                "Not performing text-to-speech for a user without the text-to-speech permission"
            );
        }
        // Offers are made even if the sender cannot use text-to-speech, because others in the room may.
        // Whoever reacts is checked then (see the reaction controller), which also skips messages
        // whose author is denied text-to-speech. So there's no point in offering it on such a user's own message.
        Some(TextToSpeechParams::Offer(text_to_speech_eligible_payload, _))
            if text_to_speech_eligible_payload.event_id == *message_context.event_id()
                && message_context
                    .authorizer()
                    .denies(message_context.sender_id(), Permission::TextToSpeech) =>
        {
            tracing::debug!(
                "Not offering text-to-speech on the message of a user denied text-to-speech"
            );
        }
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

/// Determines where the text-generation response goes.
fn determine_text_generation_response_type(
    controller_type: &ChatCompletionControllerType,
    thread_info: &ThreadInfo,
    reply_style: TextGenerationReplyStyle,
) -> MessageResponseType {
    match controller_type {
        // When we're triggered via a reply mention, we reply to the message that triggered us.
        ChatCompletionControllerType::ReplyMention => {
            MessageResponseType::Reply(thread_info.last_event_id.clone())
        }

        // Audio is excluded, because its transcription has already been posted in a thread.
        ChatCompletionControllerType::Audio => MessageResponseType::InThread(thread_info.clone()),

        // A top-level message in a room preferring replies gets an in-room reply instead of a new thread.
        // Messages in existing threads (e.g. started before the setting changed) are still answered in the thread.
        _ if reply_style == TextGenerationReplyStyle::Reply
            && thread_info.is_thread_root_only() =>
        {
            MessageResponseType::Reply(thread_info.root_event_id.clone())
        }

        // In all other cases, we're dealing with a threaded conversation, so we reply in the thread.
        _ => MessageResponseType::InThread(thread_info.clone()),
    }
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

    let sender_filter = match controller_type {
        // Regular chat completion only operates on messages from users allowed to use text generation.
        ChatCompletionControllerType::TextCommand
        | ChatCompletionControllerType::TextMention
        | ChatCompletionControllerType::TextDirect
        | ChatCompletionControllerType::Audio
        | ChatCompletionControllerType::Image
        | ChatCompletionControllerType::File => {
            message_context.context_sender_filter(Permission::TextGeneration, false)
        }

        // When we're triggered via an explicit mention (thread or reply), we wish to operate against the mention's whole context
        // (the whole thread or the whole reply chain upward of the message that triggered us).
        //
        // This is to allow users to trigger text-generation for other users' messages.
        // When we're dragged into a conversation by a user allowed to use text generation, we'd like to process all messages in the conversation,
        // except those of authors explicitly denied text generation.
        ChatCompletionControllerType::ThreadMention
        | ChatCompletionControllerType::ReplyMention => {
            message_context.context_sender_filter(Permission::TextGeneration, true)
        }
    };

    let params = MatrixMessageProcessingParams::new(bot.user_id().to_owned(), sender_filter)
        .with_first_message_prefixes_to_strip(first_message_prefixes_to_strip)
        .with_bot_user_prefixes_to_strip(bot_user_prefixes_to_strip);

    let conversation = match (controller_type, &response_type) {
        // When we're triggered via a reply mention, the context is the whole reply chain upward of the message that triggered us.
        // When we answer a new conversation with an in-room reply (see `TextGenerationReplyStyle::Reply`), the reply chain is just the triggering message.
        (ChatCompletionControllerType::ReplyMention, _) | (_, MessageResponseType::Reply(_)) => {
            create_llm_conversation_for_matrix_reply_chain(
                &matrix_link,
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
                &matrix_link,
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

    let conversation = inject_sender_context(
        conversation,
        message_context
            .room_config_context()
            .text_generation_sender_context_mode(),
    );

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

    // Cloned only when the thinking-notice is enabled; the original is moved into `params` below.
    let notice_prompt_variables = if message_context
        .room_config_context()
        .text_generation_thinking_notice_enabled()
    {
        Some(prompt_variables.clone())
    } else {
        None
    };

    let context_management_enabled = message_context
        .room_config_context()
        .text_generation_context_management_enabled();

    let executor = crate::tools::NoTools;
    let offered_tools = vec![];

    let (reserved_context_tokens, tool_limits) = crate::tools::tool_budget(
        context_management_enabled,
        controller.text_generation_max_context_tokens(),
    );

    let params = TextGenerationParams {
        context_management_enabled,

        prompt_override: message_context
            .room_config_context()
            .text_generation_prompt_override(),

        temperature_override: message_context
            .room_config_context()
            .text_generation_temperature_override(),

        prompt_variables,

        reserved_context_tokens: if offered_tools.is_empty() {
            0
        } else {
            reserved_context_tokens
        },
        tools: offered_tools,
        tool_instructions: None,
        continuation: None,
    };

    let generation =
        crate::tools::run_text_generation(controller, conversation, params, &executor, tool_limits)
            .instrument(span);

    // When the thinking-notice is enabled, race generation against a timer that posts and then
    // periodically edits a "thinking…" placeholder. `biased;` makes generation win a tie, and the
    // loop exits the instant generation resolves, so there is no detached task and no late edit can
    // ever clobber the real answer. `placeholder` is the event we must finalize in every exit path.
    let (result, placeholder) = if let Some(notice_prompt_variables) = notice_prompt_variables {
        tokio::pin!(generation);

        let mut placeholder: Option<OwnedEventId> = None;
        // Seed the flavor sequence per-generation so different turns don't all open on the same
        // line; the monotonic increment then guarantees consecutive notices differ.
        let mut notice_sequence: usize = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|since| since.subsec_nanos() as usize)
            .unwrap_or(0);
        let mut tick = tokio::time::interval_at(
            tokio::time::Instant::now() + THINKING_NOTICE_FIRST_DELAY,
            THINKING_NOTICE_INTERVAL,
        );
        // If an edit runs long, hold ~INTERVAL spacing rather than bursting the missed ticks.
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        let result = loop {
            tokio::select! {
                biased;
                generation_result = &mut generation => break generation_result,
                _ = tick.tick() => {
                    let message = notice_prompt_variables.format(
                        strings::thinking::pick_message(start_time.elapsed(), notice_sequence),
                    );
                    notice_sequence = notice_sequence.wrapping_add(1);

                    match &placeholder {
                        None => {
                            placeholder = bot
                                .messaging()
                                .send_text_markdown_no_fail_quietly(
                                    message_context.room(),
                                    message,
                                    response_type.clone(),
                                )
                                .await
                                .map(|response| response.event_id);
                        }
                        Some(event_id) => {
                            bot.messaging()
                                .edit_text_markdown_no_fail(
                                    message_context.room(),
                                    event_id,
                                    message,
                                )
                                .await;
                        }
                    }
                }
            }
        };

        (result, placeholder)
    } else {
        (generation.await, None)
    };

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

            let error_text = strings::agent::error_while_serving_purpose(
                agent.identifier(),
                &AgentPurpose::TextGeneration,
                &err,
            );

            finalize_thinking_notice_with_error(
                bot,
                message_context,
                placeholder.as_ref(),
                &error_text,
                response_type,
            )
            .await;

            return None;
        }
    };

    let text = result.trim().to_owned();
    if text.is_empty() {
        tracing::warn!(
            agent_id = agent.identifier().as_string(),
            "Agent returned empty text",
        );

        let empty_text = strings::agent::empty_response_returned(agent.identifier());

        finalize_thinking_notice_with_error(
            bot,
            message_context,
            placeholder.as_ref(),
            &empty_text,
            response_type,
        )
        .await;

        return None;
    }

    // Finalize the answer into a single message. With a placeholder, edit it in place (so the
    // "thinking…" message becomes the answer); the TTS payload then points at that same event.
    // If the edit fails, fall back to a fresh send so the real answer is never lost.
    let event_id = match &placeholder {
        Some(event_id)
            if bot
                .messaging()
                .edit_text_markdown_no_fail(message_context.room(), event_id, text.clone())
                .await
                .is_some() =>
        {
            event_id.clone()
        }
        _ => {
            bot.messaging()
                .send_text_markdown_no_fail(message_context.room(), text.clone(), response_type)
                .await?
                .event_id
        }
    };

    Some(TextToSpeechEligiblePayload { text, event_id })
}

/// Finalizes a thinking-notice placeholder (if one was posted) with error/notice text, so a
/// failed or empty generation never leaves an orphaned "thinking…" message behind. With no
/// placeholder, this is the original behavior: a fresh error notice.
async fn finalize_thinking_notice_with_error(
    bot: &Bot,
    message_context: &MessageContext,
    placeholder: Option<&OwnedEventId>,
    text: &str,
    response_type: MessageResponseType,
) {
    match placeholder {
        Some(event_id) => {
            bot.messaging()
                .edit_text_markdown_no_fail(
                    message_context.room(),
                    event_id,
                    crate::utils::status::create_error_message_text(text),
                )
                .await;
        }
        None => {
            bot.messaging()
                .send_error_markdown_no_fail(message_context.room(), text, response_type)
                .await;
        }
    }
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

fn inject_sender_context(
    conversation: Conversation,
    sender_context_mode: TextGenerationSenderContextMode,
) -> Conversation {
    if sender_context_mode == TextGenerationSenderContextMode::Disabled {
        return conversation;
    }

    let include_timestamp =
        sender_context_mode == TextGenerationSenderContextMode::MatrixUserIdAndTimestamp;

    let messages = conversation
        .messages
        .into_iter()
        .map(|mut message| {
            if message.author == Author::Prompt {
                return message;
            }

            let Some(sender_id) = &message.sender_id else {
                return message;
            };

            if let MessageContent::Text(ref mut text) = message.content {
                *text = if include_timestamp {
                    let timestamp = message.timestamp.format("%Y-%m-%dT%H:%M:%SZ");
                    format!("[sender={} sent_at={}] {}", sender_id, timestamp, text)
                } else {
                    format!("[sender={}] {}", sender_id, text)
                };
            }

            message
        })
        .collect();

    Conversation { messages }
}

#[cfg(test)]
mod response_type_tests {
    use super::{ChatCompletionControllerType, determine_text_generation_response_type};
    use crate::entity::roomconfig::TextGenerationReplyStyle;
    use mxlink::matrix_sdk::ruma::OwnedEventId;
    use mxlink::{MessageResponseType, ThreadInfo};

    fn event_id(id: &str) -> OwnedEventId {
        OwnedEventId::try_from(id).unwrap()
    }

    fn top_level() -> ThreadInfo {
        ThreadInfo::new(event_id("$root:example.com"), event_id("$root:example.com"))
    }

    fn in_thread() -> ThreadInfo {
        ThreadInfo::new(event_id("$root:example.com"), event_id("$last:example.com"))
    }

    fn is_reply_to(response_type: &MessageResponseType, expected: &str) -> bool {
        matches!(response_type, MessageResponseType::Reply(id) if id.as_str() == expected)
    }

    fn is_in_thread(response_type: &MessageResponseType) -> bool {
        matches!(response_type, MessageResponseType::InThread(info) if info.root_event_id.as_str() == "$root:example.com")
    }

    #[test]
    fn thread_style_answers_top_level_messages_in_a_thread() {
        for controller_type in [
            ChatCompletionControllerType::TextCommand,
            ChatCompletionControllerType::TextMention,
            ChatCompletionControllerType::TextDirect,
            ChatCompletionControllerType::Image,
            ChatCompletionControllerType::File,
            ChatCompletionControllerType::Audio,
        ] {
            let result = determine_text_generation_response_type(
                &controller_type,
                &top_level(),
                TextGenerationReplyStyle::Thread,
            );
            assert!(is_in_thread(&result), "{controller_type:?}");
        }
    }

    #[test]
    fn reply_style_answers_top_level_messages_with_a_reply() {
        for controller_type in [
            ChatCompletionControllerType::TextCommand,
            ChatCompletionControllerType::TextMention,
            ChatCompletionControllerType::TextDirect,
            ChatCompletionControllerType::Image,
            ChatCompletionControllerType::File,
        ] {
            let result = determine_text_generation_response_type(
                &controller_type,
                &top_level(),
                TextGenerationReplyStyle::Reply,
            );
            assert!(
                is_reply_to(&result, "$root:example.com"),
                "{controller_type:?}"
            );
        }
    }

    #[test]
    fn reply_style_keeps_audio_in_a_thread() {
        let result = determine_text_generation_response_type(
            &ChatCompletionControllerType::Audio,
            &top_level(),
            TextGenerationReplyStyle::Reply,
        );
        assert!(is_in_thread(&result));
    }

    #[test]
    fn reply_style_keeps_existing_threads() {
        for controller_type in [
            ChatCompletionControllerType::TextDirect,
            ChatCompletionControllerType::ThreadMention,
        ] {
            let result = determine_text_generation_response_type(
                &controller_type,
                &in_thread(),
                TextGenerationReplyStyle::Reply,
            );
            assert!(is_in_thread(&result), "{controller_type:?}");
        }
    }

    #[test]
    fn reply_mention_replies_to_the_triggering_message_in_both_styles() {
        for reply_style in TextGenerationReplyStyle::choices() {
            let result = determine_text_generation_response_type(
                &ChatCompletionControllerType::ReplyMention,
                &in_thread(),
                reply_style,
            );
            assert!(is_reply_to(&result, "$last:example.com"), "{reply_style}");
        }
    }
}

#[cfg(test)]
mod sender_context_tests {
    use super::inject_sender_context;
    use crate::conversation::llm::{Author, Conversation, ImageDetails, Message, MessageContent};
    use crate::entity::roomconfig::TextGenerationSenderContextMode;
    use chrono::{TimeZone, Utc};
    use mxlink::matrix_sdk::ruma::events::room::message::ImageMessageEventContent;
    use mxlink::matrix_sdk::ruma::{OwnedMxcUri, OwnedUserId};
    use mxlink::mime;

    #[test]
    fn test_inject_sender_context_prefixes_text_messages() {
        let timestamp = Utc.with_ymd_and_hms(2026, 3, 23, 14, 30, 0).unwrap();
        let user_id = OwnedUserId::try_from("@alice:example.com").unwrap();

        let conversation = Conversation {
            messages: vec![Message {
                author: Author::User,
                sender_id: Some(user_id),
                timestamp,
                content: MessageContent::Text("Hello bot".to_string()),
            }],
        };

        let result = inject_sender_context(
            conversation,
            TextGenerationSenderContextMode::MatrixUserIdAndTimestamp,
        );

        assert_eq!(result.messages.len(), 1);
        assert_eq!(
            result.messages[0].content,
            MessageContent::Text(
                "[sender=@alice:example.com sent_at=2026-03-23T14:30:00Z] Hello bot".to_string()
            )
        );
    }

    #[test]
    fn test_inject_sender_context_can_prefix_without_timestamp() {
        let timestamp = Utc.with_ymd_and_hms(2026, 3, 23, 14, 30, 0).unwrap();
        let user_id = OwnedUserId::try_from("@alice:example.com").unwrap();

        let conversation = Conversation {
            messages: vec![Message {
                author: Author::User,
                sender_id: Some(user_id),
                timestamp,
                content: MessageContent::Text("Hello bot".to_string()),
            }],
        };

        let result =
            inject_sender_context(conversation, TextGenerationSenderContextMode::MatrixUserId);

        assert_eq!(result.messages.len(), 1);
        assert_eq!(
            result.messages[0].content,
            MessageContent::Text("[sender=@alice:example.com] Hello bot".to_string())
        );
    }

    #[test]
    fn test_inject_sender_context_prefixes_assistant_messages() {
        let timestamp = Utc.with_ymd_and_hms(2026, 3, 23, 14, 30, 0).unwrap();
        let user_id = OwnedUserId::try_from("@baibot:example.com").unwrap();

        let conversation = Conversation {
            messages: vec![Message {
                author: Author::Assistant,
                sender_id: Some(user_id),
                timestamp,
                content: MessageContent::Text("Hello human".to_string()),
            }],
        };

        let result = inject_sender_context(
            conversation,
            TextGenerationSenderContextMode::MatrixUserIdAndTimestamp,
        );

        assert_eq!(result.messages.len(), 1);
        assert_eq!(
            result.messages[0].content,
            MessageContent::Text(
                "[sender=@baibot:example.com sent_at=2026-03-23T14:30:00Z] Hello human".to_string()
            )
        );
    }

    #[test]
    fn test_inject_sender_context_skips_prompt_messages() {
        let timestamp = Utc.with_ymd_and_hms(2026, 3, 23, 14, 30, 0).unwrap();

        let conversation = Conversation {
            messages: vec![Message {
                author: Author::Prompt,
                sender_id: None,
                timestamp,
                content: MessageContent::Text("You are a bot".to_string()),
            }],
        };

        let result =
            inject_sender_context(conversation, TextGenerationSenderContextMode::MatrixUserId);

        assert_eq!(
            result.messages[0].content,
            MessageContent::Text("You are a bot".to_string())
        );
    }

    #[test]
    fn test_inject_sender_context_skips_messages_without_sender_id() {
        let timestamp = Utc.with_ymd_and_hms(2026, 3, 23, 14, 30, 0).unwrap();

        let conversation = Conversation {
            messages: vec![Message {
                author: Author::User,
                sender_id: None,
                timestamp,
                content: MessageContent::Text("Transcribed text".to_string()),
            }],
        };

        let result = inject_sender_context(
            conversation,
            TextGenerationSenderContextMode::MatrixUserIdAndTimestamp,
        );

        assert_eq!(
            result.messages[0].content,
            MessageContent::Text("Transcribed text".to_string())
        );
    }

    #[test]
    fn test_inject_sender_context_leaves_non_text_content_unchanged() {
        let timestamp = Utc.with_ymd_and_hms(2026, 3, 23, 14, 30, 0).unwrap();
        let user_id = OwnedUserId::try_from("@alice:example.com").unwrap();
        let image_event_content = ImageMessageEventContent::plain(
            "image.png".to_string(),
            OwnedMxcUri::from("mxc://example.com/1234567890"),
        );

        let conversation = Conversation {
            messages: vec![Message {
                author: Author::User,
                sender_id: Some(user_id),
                timestamp,
                content: MessageContent::Image(ImageDetails::new(
                    image_event_content.clone(),
                    mime::IMAGE_PNG,
                    vec![],
                )),
            }],
        };

        let result = inject_sender_context(
            conversation,
            TextGenerationSenderContextMode::MatrixUserIdAndTimestamp,
        );

        assert_eq!(
            result.messages[0].content,
            MessageContent::Image(ImageDetails::new(
                image_event_content,
                mime::IMAGE_PNG,
                vec![]
            ))
        );
    }

    #[test]
    fn test_inject_sender_context_none_leaves_text_unchanged() {
        let timestamp = Utc.with_ymd_and_hms(2026, 3, 23, 14, 30, 0).unwrap();
        let user_id = OwnedUserId::try_from("@alice:example.com").unwrap();

        let conversation = Conversation {
            messages: vec![Message {
                author: Author::User,
                sender_id: Some(user_id),
                timestamp,
                content: MessageContent::Text("Hello bot".to_string()),
            }],
        };

        let result = inject_sender_context(conversation, TextGenerationSenderContextMode::Disabled);

        assert_eq!(
            result.messages[0].content,
            MessageContent::Text("Hello bot".to_string())
        );
    }
}
