pub mod anthropic;
mod config;
mod controller;
mod entity;
pub(super) mod groq;
pub mod localai;
pub(super) mod mistral;
pub mod ollama;
pub mod openai;
pub mod openai_compat;
pub(super) mod openrouter;
pub(super) mod togetherai;

fn default_temperature() -> f32 {
    1.0
}

pub use controller::{ControllerTrait, ControllerType};

pub use config::ConfigTrait;

pub use entity::{
    AgentProvider, AgentProviderInfo, ImageEditParams, ImageGenerationParams, ImageSource,
    PingResult, SpeechToTextParams, SpeechToTextResult, TextGenerationParams,
    TextGenerationPromptVariables, TextToSpeechParams,
};
