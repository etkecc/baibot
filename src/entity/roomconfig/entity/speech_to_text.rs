use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct RoomSettingsSpeechToText {
    pub flow_type: Option<SpeechToTextFlowType>,

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
