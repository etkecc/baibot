use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct RoomSettingsSpeechToText {
    pub flow_type: Option<SpeechToTextFlowType>,

    /// Controls how the transcribed message is posted when dealing with:
    /// - messages that only get transcribed (and do not trigger text-generation).
    ///   See `flow_type` and `SpeechToTextFlowType::OnlyTranscribe` for more details.
    /// - incoming voice messages that are not part of a thread.
    ///   For messages that are part of a thread, we need to reply within the thread in a way (with a notice message)
    ///   that won't confuse the bot later, so we have no choice but to use a notice message.
    ///
    /// Text-generation may happen either as a direct result of the incoming voice message or as part of a threaded conversation.
    /// Transcribed messages should not be attributed to the bot for the purposes of text-generation,
    /// so any time there's a chance of text-generation happening, we should use `SpeechToTextMessageTypeForNonThreadedOnlyTranscribedMessages::Notice`
    /// and optionally prefix the message with `> 🦻`.
    pub msg_type_for_non_threaded_only_transcribed_messages:
        Option<SpeechToTextMessageTypeForNonThreadedOnlyTranscribedMessages>,

    /// The language of the input audio.
    /// Supplying the input language in [ISO-639-1](https://en.wikipedia.org/wiki/List_of_ISO_639-1_codes) format will improve accuracy and latency.
    pub language: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
pub enum SpeechToTextFlowType {
    /// Voice messages are to be ignored.
    #[serde(rename = "ignore")]
    Ignore,

    /// Voice messages are to trigger text-generation.
    /// This may potentially trigger speech-to-text, but that's not what we care about here.
    #[serde(rename = "transcribe_and_generate_text")]
    TranscribeAndGenerateText,

    // Voices messages are to trigger transcription.
    #[serde(rename = "only_transcribe")]
    OnlyTranscribe,
}

impl SpeechToTextFlowType {
    pub fn choices() -> Vec<Self> {
        vec![
            Self::Ignore,
            Self::TranscribeAndGenerateText,
            Self::OnlyTranscribe,
        ]
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "ignore" => Some(Self::Ignore),
            "transcribe_and_generate_text" => Some(Self::TranscribeAndGenerateText),
            "only_transcribe" => Some(Self::OnlyTranscribe),
            _ => None,
        }
    }
}

impl std::fmt::Display for SpeechToTextFlowType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SpeechToTextFlowType::Ignore => {
                write!(f, "ignore")
            }
            SpeechToTextFlowType::TranscribeAndGenerateText => {
                write!(f, "transcribe_and_generate_text")
            }
            SpeechToTextFlowType::OnlyTranscribe => write!(f, "only_transcribe"),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
pub enum SpeechToTextMessageTypeForNonThreadedOnlyTranscribedMessages {
    /// Send the transcribed message text as a regular message
    #[serde(rename = "text")]
    Text,

    /// Send the transcribed message text as a notice message
    #[serde(rename = "notice")]
    Notice,
}

impl SpeechToTextMessageTypeForNonThreadedOnlyTranscribedMessages {
    pub fn choices() -> Vec<Self> {
        vec![Self::Text, Self::Notice]
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "text" => Some(Self::Text),
            "notice" => Some(Self::Notice),
            _ => None,
        }
    }
}

impl std::fmt::Display for SpeechToTextMessageTypeForNonThreadedOnlyTranscribedMessages {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SpeechToTextMessageTypeForNonThreadedOnlyTranscribedMessages::Text => write!(f, "text"),
            SpeechToTextMessageTypeForNonThreadedOnlyTranscribedMessages::Notice => {
                write!(f, "notice")
            }
        }
    }
}
