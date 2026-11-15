use std::ops::Deref;

use mxlink::MatrixLink;

use crate::{
    Bot,
    agent::AgentPurpose,
    authorization::Permission,
    entity::{MessageContext, MessagePayload},
};

mod text_to_speech;

pub async fn handle(
    bot: &Bot,
    matrix_link: MatrixLink,
    message_context: &MessageContext,
) -> anyhow::Result<()> {
    match &message_context.payload() {
        MessagePayload::Reaction {
            key,
            reacted_to_event_payload,
            reacted_to_event_id,
            reacted_to_event_sender_id,
        } => {
            if key == AgentPurpose::TextToSpeech.emoji() {
                if !message_context.sender_can(Permission::TextToSpeech) {
                    tracing::debug!(
                        "Ignoring text-to-speech reaction from a user without the text-to-speech permission"
                    );
                    return Ok(());
                }

                // Like conversation context, explicit requests may cover others' messages,
                // except those of authors denied the feature. The bot's own messages are always fine.
                if reacted_to_event_sender_id != bot.user_id()
                    && message_context
                        .authorizer()
                        .denies(reacted_to_event_sender_id, Permission::TextToSpeech)
                {
                    tracing::debug!(
                        "Ignoring text-to-speech reaction to a message whose author is denied text-to-speech"
                    );
                    return Ok(());
                }

                if let MessagePayload::Text(text_content) = reacted_to_event_payload.deref() {
                    return text_to_speech::handle(
                        bot,
                        matrix_link,
                        message_context,
                        reacted_to_event_id,
                        reacted_to_event_sender_id,
                        text_content,
                    )
                    .await;
                }

                tracing::debug!("Ignoring text-to-speech reaction to non-text message");
                return Ok(());
            }

            tracing::debug!("Ignoring unknown reaction");

            Ok(())
        }
        _ => Err(anyhow::anyhow!(
            "Reaction controller called with a non-reaction message"
        )),
    }
}
