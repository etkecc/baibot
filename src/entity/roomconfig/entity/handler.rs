use serde::{Deserialize, Serialize};

use crate::agent::AgentPurpose;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct RoomSettingsHandler {
    /// The agent used for any of the tasks which do not have a dedicated agent for them
    catch_all: Option<String>,

    /// The agent used for text generation
    text_generation: Option<String>,

    /// The agent used for transcribing audio (voice) to text
    speech_to_text: Option<String>,

    /// The agent used for converting text to audio (voice)
    text_to_speech: Option<String>,

    /// The agent used for generating images
    image_generation: Option<String>,
}

impl RoomSettingsHandler {
    pub fn get_by_purpose(&self, purpose: AgentPurpose) -> Option<String> {
        match purpose {
            AgentPurpose::CatchAll => self.catch_all.clone(),
            AgentPurpose::TextGeneration => self.text_generation.clone(),
            AgentPurpose::SpeechToText => self.speech_to_text.clone(),
            AgentPurpose::TextToSpeech => self.text_to_speech.clone(),
            AgentPurpose::ImageGeneration => self.image_generation.clone(),
        }
    }

    pub fn get_by_purpose_with_catch_all_fallback(&self, purpose: AgentPurpose) -> Option<String> {
        match self.get_by_purpose(purpose) {
            Some(agent_id) => Some(agent_id),
            None => self.catch_all.clone(),
        }
    }

    pub fn set_by_purpose(&mut self, purpose: AgentPurpose, agent_id: Option<String>) {
        match purpose {
            AgentPurpose::CatchAll => {
                self.catch_all = agent_id;
            }
            AgentPurpose::TextGeneration => {
                self.text_generation = agent_id;
            }
            AgentPurpose::SpeechToText => {
                self.speech_to_text = agent_id;
            }
            AgentPurpose::TextToSpeech => {
                self.text_to_speech = agent_id;
            }
            AgentPurpose::ImageGeneration => {
                self.image_generation = agent_id;
            }
        };
    }
}
