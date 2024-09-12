use std::ops::Deref;

use mxlink::MatrixLink;

use crate::{
    agent::AgentPurpose,
    entity::{MessageContext, MessagePayload},
    Bot,
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
