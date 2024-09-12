use mxlink::{MatrixLink, MessageResponseType};

use mxlink::matrix_sdk::ruma::{
    events::room::message::TextMessageEventContent, OwnedEventId, OwnedUserId,
};

use crate::entity::roomconfig::{
    TextToSpeechBotMessagesFlowType, TextToSpeechUserMessagesFlowType,
};

use crate::{
    agent::AgentPurpose, controller::utils::agent::get_effective_agent_for_purpose_or_complain,
    entity::MessageContext, Bot,
};

pub(super) async fn handle(
    bot: &Bot,
    matrix_link: MatrixLink,
    message_context: &MessageContext,
    reacted_to_event_id: &OwnedEventId,
    reacted_to_event_sender_id: &OwnedUserId,
    text_content: &TextMessageEventContent,
) -> anyhow::Result<()> {
    // If we're in a thread, we're likely dealing with a bot message, so we should start in the thread.
    // Otherwise, we're likely operating in "TTS user messages" mode, so we should reply to the reacted-to message and avoid threads.
    let response_type = if message_context.thread_info().is_thread_root_only() {
        MessageResponseType::Reply(reacted_to_event_id.clone())
    } else {
        MessageResponseType::InThread(message_context.thread_info().clone())
    };

    if !is_allowed_to_tts_for_event(
        message_context,
        reacted_to_event_sender_id,
        matrix_link.user_id(),
    ) {
        tracing::debug!("Ignoring request for on-demand text-to-speech (via reaction) due to room configuration");
        return Ok(());
    }

    let speech_agent = get_effective_agent_for_purpose_or_complain(
        bot,
        message_context,
        AgentPurpose::TextToSpeech,
        response_type.clone(),
        true,
    )
    .await;

    let Some(speech_agent) = speech_agent else {
        // We've already complained about this in get_effective_agent_or_complain
        return Ok(());
    };

    crate::controller::utils::text_to_speech::generate_and_send_tts_for_message(
        bot,
        matrix_link,
        message_context,
        response_type,
        &speech_agent,
        reacted_to_event_id,
        &text_content.body,
    )
    .await;

    Ok(())
}

fn is_allowed_to_tts_for_event(
    message_context: &MessageContext,
    sender_id: &OwnedUserId,
    bot_user_id: &OwnedUserId,
) -> bool {
    // Whether we're allowed depends on who the original message sender is (the bot or some user).
    //
    // The user may be an allowed bot user or someone else.
    // Regardless, we've been invoked by an allowed user, so if the user wants TTS for a foreign message, we should allow it.

    if *sender_id == *bot_user_id {
        match message_context
            .room_config_context()
            .text_to_speech_bot_messages_flow_type()
        {
            TextToSpeechBotMessagesFlowType::Never => false,
            TextToSpeechBotMessagesFlowType::OnDemandAlways => true,
            TextToSpeechBotMessagesFlowType::OnDemandForVoice => true,
            TextToSpeechBotMessagesFlowType::OnlyForVoice => true,
            TextToSpeechBotMessagesFlowType::Always => true,
        }
    } else {
        match message_context
            .room_config_context()
            .text_to_speech_user_messages_flow_type()
        {
            TextToSpeechUserMessagesFlowType::Never => false,
            TextToSpeechUserMessagesFlowType::OnDemand => true,
            TextToSpeechUserMessagesFlowType::Always => true,
        }
    }
}
