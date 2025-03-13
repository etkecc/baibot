use serde::{Deserialize, Serialize};

use crate::agent::{default_prompt, provider::ConfigTrait};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub base_url: String,

    pub api_key: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_generation: Option<TextGenerationConfig>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub speech_to_text: Option<SpeechToTextConfig>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_to_speech: Option<TextToSpeechConfig>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_generation: Option<ImageGenerationConfig>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            base_url: "https://api.openai.com/v1".to_owned(),
            api_key: "YOUR_API_KEY_HERE".to_owned(),
            text_generation: Some(TextGenerationConfig::default()),
            speech_to_text: Some(SpeechToTextConfig::default()),
            text_to_speech: Some(TextToSpeechConfig::default()),
            image_generation: Some(ImageGenerationConfig::default()),
        }
    }
}

impl ConfigTrait for Config {
    fn validate(&self) -> Result<(), String> {
        if self.base_url.is_empty() {
            return Err("The base URL must not be empty.".to_owned());
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextGenerationConfig {
    #[serde(default = "default_text_model_id")]
    pub model_id: String,

    #[serde(default)]
    pub prompt: Option<String>,

    #[serde(default = "super::super::default_temperature")]
    pub temperature: f32,

    #[serde(default)]
    pub max_response_tokens: Option<u32>,

    #[serde(default)]
    pub max_completion_tokens: Option<u32>,

    #[serde(default)]
    pub max_context_tokens: u32,
}

impl Default for TextGenerationConfig {
    fn default() -> Self {
        Self {
            model_id: default_text_model_id(),
            prompt: Some(default_prompt().to_owned()),
            temperature: super::super::default_temperature(),
            max_response_tokens: Some(16_384),
            max_completion_tokens: None,
            max_context_tokens: 128_000,
        }
    }
}

fn default_text_model_id() -> String {
    "gpt-4o".to_owned()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeechToTextConfig {
    #[serde(default = "default_speech_to_text_model_id")]
    pub model_id: String,
}

impl Default for SpeechToTextConfig {
    fn default() -> Self {
        Self {
            model_id: default_speech_to_text_model_id(),
        }
    }
}

fn default_speech_to_text_model_id() -> String {
    "whisper-1".to_owned()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextToSpeechConfig {
    #[serde(default = "default_text_to_speech_model_id")]
    pub model_id: async_openai::types::SpeechModel,

    #[serde(default = "default_text_to_speech_voice")]
    pub voice: async_openai::types::Voice,

    #[serde(default = "default_text_to_speech_speed")]
    pub speed: f32,

    #[serde(default = "default_text_to_speech_response_format")]
    pub response_format: async_openai::types::SpeechResponseFormat,
}

impl Default for TextToSpeechConfig {
    fn default() -> Self {
        Self {
            model_id: default_text_to_speech_model_id(),
            voice: default_text_to_speech_voice(),
            speed: default_text_to_speech_speed(),
            response_format: default_text_to_speech_response_format(),
        }
    }
}

fn default_text_to_speech_model_id() -> async_openai::types::SpeechModel {
    async_openai::types::SpeechModel::Tts1Hd
}

fn default_text_to_speech_voice() -> async_openai::types::Voice {
    async_openai::types::Voice::Onyx
}

fn default_text_to_speech_speed() -> f32 {
    1.0
}

fn default_text_to_speech_response_format() -> async_openai::types::SpeechResponseFormat {
    // The API defaults to mp3, but we prefer Opus because it's smaller.
    // Our clients should all have support for it.
    async_openai::types::SpeechResponseFormat::Opus
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageGenerationConfig {
    pub model_id: String,

    #[serde(default = "default_image_style")]
    pub style: async_openai::types::ImageStyle,

    #[serde(default = "default_image_size")]
    pub size: async_openai::types::ImageSize,

    #[serde(default = "default_image_quality")]
    pub quality: async_openai::types::ImageQuality,
}

impl Default for ImageGenerationConfig {
    fn default() -> Self {
        Self {
            model_id: "dall-e-3".to_owned(),
            style: default_image_style(),
            size: default_image_size(),
            quality: default_image_quality(),
        }
    }
}

impl ImageGenerationConfig {
    pub fn model_id_as_openai_image_model(
        &self,
    ) -> Result<async_openai::types::ImageModel, String> {
        match self.model_id.as_str() {
            "dall-e-2" => Ok(async_openai::types::ImageModel::DallE2),
            "dall-e-3" => Ok(async_openai::types::ImageModel::DallE3),
            other => Ok(async_openai::types::ImageModel::Other(other.to_owned())),
        }
    }
}

fn default_image_style() -> async_openai::types::ImageStyle {
    async_openai::types::ImageStyle::Vivid
}

fn default_image_size() -> async_openai::types::ImageSize {
    async_openai::types::ImageSize::S1024x1024
}

fn default_image_quality() -> async_openai::types::ImageQuality {
    async_openai::types::ImageQuality::Standard
}
