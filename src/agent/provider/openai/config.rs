use serde::{Deserialize, Serialize};

use super::OPENAI_IMAGE_MODEL_GPT_IMAGE_2_5_SUNBURST;
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

    #[serde(default)]
    pub temperature: Option<f32>,

    #[serde(default)]
    pub max_response_tokens: Option<u32>,

    #[serde(default)]
    pub max_completion_tokens: Option<u32>,

    #[serde(default)]
    pub max_context_tokens: u32,

    #[serde(default)]
    pub tools: ToolsConfig,
}

impl Default for TextGenerationConfig {
    fn default() -> Self {
        Self {
            model_id: default_text_model_id(),
            prompt: Some(default_prompt().to_owned()),
            temperature: None,
            max_response_tokens: None,
            max_completion_tokens: Some(128_000),
            max_context_tokens: 400_000,
            tools: ToolsConfig::default(),
        }
    }
}

fn default_text_model_id() -> String {
    "gpt-6.1-sol".to_owned()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolsConfig {
    #[serde(default)]
    pub web_search: bool,

    #[serde(default)]
    pub code_interpreter: bool,
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
    // OpenAI shuts down whisper-1 on 2027-02-26.
    "gpt-transcribe".to_owned()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextToSpeechConfig {
    /// A Realtime model (e.g. `gpt-realtime-2.1-mini`) or a speech-endpoint model (e.g. `tts-1-hd`).
    /// OpenAI shuts down all speech-endpoint models on 2027-01-06.
    #[serde(default = "default_text_to_speech_model_id")]
    pub model_id: String,

    /// When unset, a voice that the model supports: `cedar` for Realtime models, `onyx` otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub voice: Option<async_openai::types::audio::Voice>,

    #[serde(default = "default_text_to_speech_speed")]
    pub speed: f32,

    /// For Realtime models, baibot encodes the audio itself and supports `opus` and `wav` only.
    #[serde(default = "default_text_to_speech_response_format")]
    pub response_format: async_openai::types::audio::SpeechResponseFormat,
}

impl Default for TextToSpeechConfig {
    fn default() -> Self {
        Self {
            model_id: default_text_to_speech_model_id(),
            voice: Some(async_openai::types::audio::Voice::Cedar),
            speed: default_text_to_speech_speed(),
            response_format: default_text_to_speech_response_format(),
        }
    }
}

impl TextToSpeechConfig {
    pub fn effective_voice(&self) -> async_openai::types::audio::Voice {
        if let Some(voice) = &self.voice {
            return voice.clone();
        }

        if super::realtime_tts::is_realtime_model(&self.model_id) {
            async_openai::types::audio::Voice::Cedar
        } else {
            async_openai::types::audio::Voice::Onyx
        }
    }
}

fn default_text_to_speech_model_id() -> String {
    "gpt-realtime-2.1-mini".to_owned()
}

fn default_text_to_speech_speed() -> f32 {
    1.0
}

fn default_text_to_speech_response_format() -> async_openai::types::audio::SpeechResponseFormat {
    // The API defaults to mp3, but we prefer Opus because it's smaller.
    // Our clients should all have support for it.
    async_openai::types::audio::SpeechResponseFormat::Opus
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageGenerationConfig {
    pub model_id: String,

    #[serde(default = "default_image_size")]
    pub size: Option<async_openai::types::images::ImageSize>,

    #[serde(default = "default_image_quality")]
    pub quality: Option<async_openai::types::images::ImageQuality>,
}

impl Default for ImageGenerationConfig {
    fn default() -> Self {
        Self {
            model_id: OPENAI_IMAGE_MODEL_GPT_IMAGE_2_5_SUNBURST.to_owned(),
            size: default_image_size(),
            quality: default_image_quality(),
        }
    }
}

impl ImageGenerationConfig {
    pub fn model_id_as_openai_image_model(
        &self,
    ) -> Result<async_openai::types::images::ImageModel, String> {
        // Use the SDK's model names, including its fallback for custom providers.
        serde_json::from_value(serde_json::Value::String(self.model_id.clone()))
            .map_err(|err| err.to_string())
    }
}

fn default_image_size() -> Option<async_openai::types::images::ImageSize> {
    None
}

fn default_image_quality() -> Option<async_openai::types::images::ImageQuality> {
    None
}

#[cfg(test)]
mod tests {
    use async_openai::types::audio::Voice;

    use super::{ImageGenerationConfig, TextGenerationConfig, TextToSpeechConfig};

    #[test]
    fn new_agents_use_sol_without_temperature_and_explicit_values_still_load() {
        let defaults = TextGenerationConfig::default();
        assert_eq!(defaults.model_id, "gpt-6.1-sol");
        assert_eq!(defaults.temperature, None);

        let missing: TextGenerationConfig = serde_yaml_ng::from_str("{}\n").unwrap();
        assert_eq!(missing.model_id, "gpt-6.1-sol");
        assert_eq!(missing.temperature, None);

        let existing: TextGenerationConfig =
            serde_yaml_ng::from_str("model_id: gpt-5.4\ntemperature: 1.0\n").unwrap();
        assert_eq!(existing.model_id, "gpt-5.4");
        assert_eq!(existing.temperature, Some(1.0));
    }

    #[test]
    fn image_configs_with_the_removed_style_still_load() {
        let existing: ImageGenerationConfig =
            serde_yaml_ng::from_str("model_id: gpt-image-2.5-sunburst\nstyle: vivid\n").unwrap();
        assert_eq!(existing.model_id, "gpt-image-2.5-sunburst");
    }

    fn parse(yaml: &str) -> TextToSpeechConfig {
        serde_yaml_ng::from_str(yaml).unwrap()
    }

    #[test]
    fn text_to_speech_defaults_to_realtime() {
        let config = parse("{}");
        assert_eq!(config.model_id, "gpt-realtime-2.1-mini");
        assert_eq!(config.effective_voice(), Voice::Cedar);
    }

    #[test]
    fn legacy_text_to_speech_configs_keep_a_supported_voice() {
        let config = parse("model_id: tts-1-hd");
        assert_eq!(config.effective_voice(), Voice::Onyx);

        let config = parse("model_id: tts-1-hd\nvoice: nova");
        assert_eq!(config.effective_voice(), Voice::Nova);
    }

    #[test]
    fn text_to_speech_voice_accepts_names_and_custom_voices() {
        let config = parse("voice: some-new-voice");
        assert_eq!(
            config.effective_voice(),
            Voice::Other("some-new-voice".to_owned())
        );

        let config = parse("voice:\n  id: voice_1234");
        assert_eq!(
            serde_json::to_value(config.effective_voice()).unwrap(),
            serde_json::json!({"id": "voice_1234"})
        );
    }
}
