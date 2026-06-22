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
            base_url: "https://api.venice.ai/api/v1".to_owned(),
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
    pub max_context_tokens: u32,

    /// Sampling and reasoning knobs that live at the top level of Venice's `/chat/completions`
    /// body, not inside the `venice_parameters` bag. Venice silently ignores a top-level knob
    /// placed in the bag, so these sit here as siblings and map straight to top-level wire fields
    /// in `chat.rs`. Each is omitted from the request when unset.
    #[serde(default)]
    pub top_p: Option<f32>,

    #[serde(default)]
    pub frequency_penalty: Option<f32>,

    #[serde(default)]
    pub presence_penalty: Option<f32>,

    #[serde(default)]
    pub repetition_penalty: Option<f32>,

    #[serde(default)]
    pub reasoning_effort: Option<String>,

    /// Prompt-cache retention window (`default`, `extended`, or `24h`). This carries a named
    /// default rather than a bare `#[serde(default)]` (which would yield `None`), so a config that
    /// omits the key still ships `24h` and keeps caching on. Caching is the per-deployment cost
    /// lever, so the omitted-key case must not silently disable it. The value here must agree with
    /// the `Default` impl below.
    #[serde(default = "default_prompt_cache_retention")]
    pub prompt_cache_retention: Option<String>,

    /// When set, the model's `reasoning_content` (its thinking) is appended to the reply. Off by
    /// default to match today's `strip_thinking_response: true` behavior, so existing deployments
    /// see no change.
    #[serde(default)]
    pub show_reasoning: bool,

    /// Venice-specific request knobs, serialized 1:1 into the `venice_parameters` bag on the
    /// wire. Any unset field is omitted, so Venice applies its own server-side default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub venice_parameters: Option<VeniceParameters>,
}

impl Default for TextGenerationConfig {
    fn default() -> Self {
        Self {
            model_id: default_text_model_id(),
            prompt: Some(default_prompt().to_owned()),
            temperature: super::super::default_temperature(),
            // Reserved output budget: sent as the response cap AND subtracted from the context
            // window when trimming history. Mirrors the openai_compat sibling's default.
            max_response_tokens: Some(4096),
            // Matches Venice's own `availableContextTokens` (131072) and the non-OpenAI sibling
            // providers (ollama/localai/mistral all default to 128_000).
            max_context_tokens: 128_000,
            // Sampling knobs stay None so Venice applies its own server-side default. Caching is
            // the one exception: retention defaults to 24h here so a programmatic default caches
            // out of the box, agreeing with the `#[serde(default = ...)]` on the field.
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            repetition_penalty: None,
            reasoning_effort: None,
            prompt_cache_retention: default_prompt_cache_retention(),
            show_reasoning: false,
            // A usable starting point, not an everything-set dump: only these three are sent;
            // every other knob stays None so Venice applies its own default (omitting != false).
            venice_parameters: Some(VeniceParameters {
                enable_web_search: Some(WebSearchMode::Auto),
                strip_thinking_response: Some(true),
                enable_e2ee: Some(false),
                ..Default::default()
            }),
        }
    }
}

fn default_text_model_id() -> String {
    "kimi-k2-5".to_owned()
}

/// Defaults prompt-cache retention to 24h so caching is on unless a config explicitly opts out.
/// A bare `#[serde(default)]` would deserialize an omitted key to `None`, which disables caching;
/// this keeps the cost lever engaged for configs that never mention it.
fn default_prompt_cache_retention() -> Option<String> {
    Some("24h".to_owned())
}

/// The full `venice_parameters` knob set, mirroring Venice's `ChatCompletionRequest`
/// schema field-for-field. Every field is optional with `skip_serializing_if`, so the
/// request never carries a knob the user didn't set (the body is `additionalProperties: false`,
/// and an unset knob simply omits rather than sending `null`).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VeniceParameters {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enable_web_search: Option<WebSearchMode>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enable_web_citations: Option<bool>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enable_web_scraping: Option<bool>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub include_venice_system_prompt: Option<bool>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub include_search_results_in_stream: Option<bool>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub return_search_results_as_documents: Option<bool>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enable_x_search: Option<bool>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enable_e2ee: Option<bool>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub character_slug: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strip_thinking_response: Option<bool>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disable_thinking: Option<bool>,

    /// Response verbosity (`low`, `medium`, `high`). Venice accepts this both top-level and inside
    /// the bag; it lives here so the top-level config stays lean, and Venice reads it from the bag.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verbosity: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WebSearchMode {
    Auto,
    On,
    Off,
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
    "nvidia/parakeet-tdt-0.6b-v3".to_owned()
}

/// `/audio/speech` (`CreateSpeechRequestSchema`) request knobs. Only `model_id` is required on
/// the wire; everything else is optional with `skip_serializing_if` so an unset knob is omitted
/// rather than sent as `null` (the body is `additionalProperties: false`). `voice` is a free
/// `Option<String>`, not a closed enum: Venice's voice set spans dozens of model-specific names
/// plus arbitrary cloned-voice handles (`vv_<id>`), so an enum would reject valid handles.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextToSpeechConfig {
    #[serde(default = "default_text_to_speech_model_id")]
    pub model_id: String,

    #[serde(
        default = "default_text_to_speech_voice",
        skip_serializing_if = "Option::is_none"
    )]
    pub voice: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speed: Option<f32>,

    #[serde(
        default = "default_text_to_speech_response_format",
        skip_serializing_if = "Option::is_none"
    )]
    pub response_format: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
}

impl Default for TextToSpeechConfig {
    fn default() -> Self {
        Self {
            model_id: default_text_to_speech_model_id(),
            voice: default_text_to_speech_voice(),
            speed: None,
            response_format: default_text_to_speech_response_format(),
            prompt: None,
            temperature: None,
            top_p: None,
        }
    }
}

fn default_text_to_speech_model_id() -> String {
    "tts-kokoro".to_owned()
}

fn default_text_to_speech_voice() -> Option<String> {
    Some("af_sky".to_owned())
}

fn default_text_to_speech_response_format() -> Option<String> {
    Some("mp3".to_owned())
}

/// `/image/generate` (`GenerateImageRequest`) request knobs, mirroring Venice's schema
/// field-for-field. Only `model_id` is required; every other knob is optional with
/// `skip_serializing_if` so unset knobs are omitted (the body is `additionalProperties: false`).
/// The full knob set is deliberate: the native `/image/generate` endpoint is the flagship's
/// reason to exist over the knob-dropping OpenAI-compat path, so the knobs ARE the feature.
/// The deprecated `inpaint` knob is intentionally absent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageGenerationConfig {
    #[serde(default = "default_image_generation_model_id")]
    pub model_id: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub negative_prompt: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cfg_scale: Option<f32>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub steps: Option<u32>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style_preset: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seed: Option<i64>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub safe_mode: Option<bool>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hide_watermark: Option<bool>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aspect_ratio: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolution: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quality: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lora_strength: Option<u32>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embed_exif_metadata: Option<bool>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enable_web_search: Option<bool>,

    /// Image-edit settings, nested here because baibot has a single `ImageGeneration` purpose
    /// and edit shares its config gate. The gen and edit model sets are disjoint, so edit
    /// carries its own model field.
    #[serde(default)]
    pub edit: ImageEditSettings,
}

impl Default for ImageGenerationConfig {
    fn default() -> Self {
        Self {
            model_id: default_image_generation_model_id(),
            negative_prompt: None,
            cfg_scale: None,
            steps: None,
            style_preset: None,
            seed: None,
            safe_mode: None,
            hide_watermark: None,
            format: None,
            width: None,
            height: None,
            aspect_ratio: None,
            resolution: None,
            quality: None,
            lora_strength: None,
            embed_exif_metadata: None,
            enable_web_search: None,
            edit: ImageEditSettings::default(),
        }
    }
}

fn default_image_generation_model_id() -> String {
    "chroma".to_owned()
}

/// `/image/edit` (`EditImageRequest`) request knobs, mirroring Venice's schema. The source image
/// and prompt are supplied per-call (not config), so only the model and the output-shaping knobs
/// live here. Each knob is optional with `skip_serializing_if`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageEditSettings {
    #[serde(default = "default_image_edit_model_id")]
    pub model_id: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_format: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aspect_ratio: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolution: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub safe_mode: Option<bool>,
}

impl Default for ImageEditSettings {
    fn default() -> Self {
        Self {
            model_id: default_image_edit_model_id(),
            output_format: None,
            aspect_ratio: None,
            resolution: None,
            safe_mode: None,
        }
    }
}

fn default_image_edit_model_id() -> String {
    "firered-image-edit".to_owned()
}
