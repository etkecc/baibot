#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AgentPurpose {
    CatchAll,
    ImageGeneration,
    TextGeneration,
    TextToSpeech,
    SpeechToText,
}

impl AgentPurpose {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "catch-all" => Some(Self::CatchAll),
            "image-generation" => Some(Self::ImageGeneration),
            "text-generation" => Some(Self::TextGeneration),
            "text-to-speech" => Some(Self::TextToSpeech),
            "speech-to-text" => Some(Self::SpeechToText),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::CatchAll => "catch-all",
            Self::ImageGeneration => "image-generation",
            Self::TextGeneration => "text-generation",
            Self::TextToSpeech => "text-to-speech",
            Self::SpeechToText => "speech-to-text",
        }
    }

    pub fn choices() -> Vec<&'static Self> {
        vec![
            &Self::TextGeneration,
            &Self::SpeechToText,
            &Self::TextToSpeech,
            &Self::ImageGeneration,
            &Self::CatchAll,
        ]
    }

    pub fn emoji(&self) -> &'static str {
        match self {
            Self::CatchAll => "❓",
            Self::TextGeneration => "💬",
            Self::SpeechToText => "🦻",
            Self::TextToSpeech => "🗣️",
            Self::ImageGeneration => "🖌️",
        }
    }

    pub fn heading(&self) -> &'static str {
        match self {
            Self::CatchAll => "Catch-All",
            Self::TextGeneration => "Text Generation",
            Self::SpeechToText => "Speech-to-Text",
            Self::TextToSpeech => "Text-to-Speech",
            Self::ImageGeneration => "Image Generation",
        }
    }
}

impl std::fmt::Display for AgentPurpose {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
