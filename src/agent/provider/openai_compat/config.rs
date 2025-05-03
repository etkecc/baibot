use serde::{Deserialize, Serialize};

use crate::agent::default_prompt;
use crate::agent::provider::openai::{
    ImageGenerationConfig as OpenAIImageGenerationConfig,
    SpeechToTextConfig as OpenAISpeechToTextConfig,
    TextGenerationConfig as OpenAITextGenerationConfig,
    TextToSpeechConfig as OpenAITextToSpeechConfig,
};

use crate::agent::provider::ConfigTrait;

use super::utils::convert_string_to_enum;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub base_url: String,

    pub api_key: Option<String>,

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
            base_url: "".to_owned(),
            api_key: Some("YOUR_API_KEY_HERE".to_owned()),
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
    pub max_context_tokens: u32,
}

impl Default for TextGenerationConfig {
    fn default() -> Self {
        Self {
            model_id: default_text_model_id(),
            prompt: Some(default_prompt().to_owned()),
            temperature: super::super::default_temperature(),
            max_response_tokens: Some(4096),
            max_context_tokens: 128_000,
        }
    }
}

impl TryInto<OpenAITextGenerationConfig> for TextGenerationConfig {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<OpenAITextGenerationConfig, Self::Error> {
        Ok(OpenAITextGenerationConfig {
            model_id: self.model_id,
            prompt: self.prompt,
            temperature: self.temperature,
            max_response_tokens: self.max_response_tokens,
            max_completion_tokens: None,
            max_context_tokens: self.max_context_tokens,
        })
    }
}

fn default_text_model_id() -> String {
    "some-model".to_owned()
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

impl TryInto<OpenAISpeechToTextConfig> for SpeechToTextConfig {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<OpenAISpeechToTextConfig, Self::Error> {
        Ok(OpenAISpeechToTextConfig {
            model_id: self.model_id,
        })
    }
}

fn default_speech_to_text_model_id() -> String {
    "whisper-1".to_owned()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextToSpeechConfig {
    #[serde(default = "default_text_to_speech_model_id")]
    pub model_id: String,

    #[serde(default = "default_text_to_speech_voice")]
    pub voice: String,

    #[serde(default = "default_text_to_speech_speed")]
    pub speed: f32,

    #[serde(default = "default_text_to_speech_response_format")]
    pub response_format: String,
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

impl TryInto<OpenAITextToSpeechConfig> for TextToSpeechConfig {
    type Error = String;

    fn try_into(self) -> Result<OpenAITextToSpeechConfig, Self::Error> {
        let model_id = convert_string_to_enum::<async_openai::types::SpeechModel>(&self.model_id)?;

        let voice = convert_string_to_enum::<async_openai::types::Voice>(&self.voice)?;

        let response_format = convert_string_to_enum::<async_openai::types::SpeechResponseFormat>(
            &self.response_format,
        )?;

        Ok(OpenAITextToSpeechConfig {
            model_id,
            voice,
            speed: self.speed,
            response_format,
        })
    }
}

fn default_text_to_speech_model_id() -> String {
    "tts-1".to_owned()
}

fn default_text_to_speech_voice() -> String {
    "onyx".to_owned()
}

fn default_text_to_speech_speed() -> f32 {
    1.0
}

fn default_text_to_speech_response_format() -> String {
    "opus".to_owned()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageGenerationConfig {
    pub model_id: String,

    #[serde(default = "default_image_style")]
    pub style: Option<String>,

    #[serde(default = "default_image_size")]
    pub size: Option<String>,

    #[serde(default = "default_image_quality")]
    pub quality: Option<String>,
}

impl Default for ImageGenerationConfig {
    fn default() -> Self {
        Self {
            model_id: "stablediffusion".to_owned(),
            style: default_image_style(),
            size: default_image_size(),
            quality: default_image_quality(),
        }
    }
}

impl TryInto<OpenAIImageGenerationConfig> for ImageGenerationConfig {
    type Error = String;

    fn try_into(self) -> Result<OpenAIImageGenerationConfig, Self::Error> {
        let size = if let Some(size) = &self.size {
            convert_string_to_enum::<async_openai::types::ImageSize>(size)?
        } else {
            async_openai::types::ImageSize::S1024x1024
        };

        let style = if let Some(style) = &self.style {
            Some(convert_string_to_enum::<async_openai::types::ImageStyle>(style)?)
        } else {
            None
        };

        let quality = if let Some(quality) = &self.quality {
            Some(convert_string_to_enum::<async_openai::types::ImageQuality>(quality)?)
        } else {
            None
        };

        Ok(OpenAIImageGenerationConfig {
            model_id: self.model_id,
            style,
            size,
            quality,
        })
    }
}

fn default_image_style() -> Option<String> {
    Some("vivid".to_owned())
}

fn default_image_size() -> Option<String> {
    Some("1024x1024".to_owned())
}

fn default_image_quality() -> Option<String> {
    Some("standard".to_owned())
}
