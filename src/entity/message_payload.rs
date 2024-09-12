use mxlink::matrix_sdk::ruma::events::room::message::{
    AudioMessageEventContent, MessageType, TextMessageEventContent,
};
use mxlink::matrix_sdk::ruma::{OwnedEventId, OwnedUserId};

use mxlink::ThreadInfo;

/// MessagePayload is like matrix-sdk's MessageType, but represents only message types that the bot deals with and payloads are massaged a bit.
#[derive(Debug, Clone)]
pub enum MessagePayload {
    Text(TextMessageEventContent),

    Audio(AudioMessageEventContent),

    Reaction {
        key: String,
        reacted_to_event_payload: Box<Self>,
        reacted_to_event_id: OwnedEventId,
        reacted_to_event_sender_id: OwnedUserId,
    },

    /// Represents an encrypted message
    Encrypted(ThreadInfo),
}

impl TryInto<MessagePayload> for MessageType {
    type Error = String;

    fn try_into(self) -> Result<MessagePayload, Self::Error> {
        let payload = match self {
            MessageType::Text(text_content) => MessagePayload::Text(text_content),
            MessageType::Audio(audio_content) => {
                // We can consider inspecting `audio_content.voice.is_some()` and ignoring audio which is not a voice message.
                //
                // However, at the time of this writing (2024-09-10), certain popular clients (Element iOS) send voice messages
                // as regular audio messages, without voice annotation as per MSC3245.
                // For this reason, we handle all audio.
                MessagePayload::Audio(audio_content)
            }
            other => {
                return Err(format!("Unsupported message type: {:?}", other));
            }
        };

        Ok(payload)
    }
}
