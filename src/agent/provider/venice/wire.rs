//! Serde structs modeling Venice's `/chat/completions`, `/audio/transcriptions`,
//! `/audio/speech`, `/image/generate`, and `/image/edit` wire shapes. Request types are
//! `Serialize`-only (we build them, Venice never sends them back); response types are
//! `Deserialize`-only. Keeping the split means the untagged request content enum is never on a
//! deserialize path, so a surprise response shape can't fail to match it.
//!
//! Field names match Venice's schema 1:1 (so the config's `model_id` becomes `model` here). Every
//! request body is `additionalProperties: false`, so optional knobs carry `skip_serializing_if`
//! to omit rather than send `null`. `/audio/speech` and `/image/edit` return raw binary (no
//! response struct); only `/image/generate` returns JSON (`GenerateImageResponse`).

use serde::{Deserialize, Serialize};

use super::config::VeniceParameters;

#[derive(Debug, Serialize)]
pub struct ChatCompletionRequest {
    pub model: String,

    pub messages: Vec<ChatMessage>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_completion_tokens: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub venice_parameters: Option<VeniceParameters>,
}

#[derive(Debug, Serialize)]
pub struct ChatMessage {
    pub role: String,

    pub content: MessageContent,
}

/// A message body is either a bare string or a list of content parts. Venice accepts both; we
/// send the parts form only when a message carries an image (baibot keeps text and images in
/// separate messages, so a parts list only ever holds images in v1).
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Parts(Vec<ContentPart>),
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentPart {
    ImageUrl { image_url: ImageUrl },
}

#[derive(Debug, Serialize)]
pub struct ImageUrl {
    /// A `data:<mime>;base64,<data>` URI for inline images.
    pub url: String,
}

/// Standard OpenAI-shaped chat completion response. We only read `choices[0].message.content`;
/// when web search is on, Venice inlines citations as `^n^` superscripts in that content and we
/// pass it through untouched.
#[derive(Debug, Deserialize)]
pub struct ChatCompletionResponse {
    pub choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
pub struct ChatChoice {
    pub message: ResponseMessage,
}

#[derive(Debug, Deserialize)]
pub struct ResponseMessage {
    #[serde(default)]
    pub content: Option<String>,
}

/// `/audio/transcriptions` response. We read `text`; the optional `duration`/`timestamps` the
/// API can return are not used in v1.
#[derive(Debug, Deserialize)]
pub struct TranscriptionResponse {
    pub text: String,
}

/// `/audio/speech` (`CreateSpeechRequestSchema`) request. `input` and `model` are always sent;
/// the rest are omitted when unset. The response is raw binary audio, so there is no response
/// struct.
#[derive(Debug, Serialize)]
pub struct SpeechRequest {
    pub model: String,

    pub input: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed: Option<f32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
}

/// `/image/generate` (`GenerateImageRequest`) request. `return_binary` is pinned `false` and
/// `variants` to `1` by the builder: baibot wants exactly one image returned as base64-in-JSON,
/// which `GenerateImageResponse` then decodes. Flipping `return_binary` would make Venice answer
/// with raw binary and break that JSON decode, so it is not configurable.
#[derive(Debug, Serialize)]
pub struct GenerateImageRequest {
    pub model: String,

    pub prompt: String,

    pub return_binary: bool,

    pub variants: u32,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub negative_prompt: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub cfg_scale: Option<f32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub steps: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub style_preset: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub safe_mode: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub hide_watermark: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub aspect_ratio: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolution: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub lora_strength: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub embed_exif_metadata: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_web_search: Option<bool>,
}

/// `/image/generate` response when `return_binary` is false: a JSON envelope carrying the images
/// as base64 strings. We read `images[0]`; `request`/`timing` and other fields are ignored. `id`
/// is telemetry only (logged, never used for correctness), so it is optional: a response that
/// carries usable `images` must not fail to deserialize just because the telemetry field drifted.
#[derive(Debug, Deserialize)]
pub struct GenerateImageResponse {
    #[serde(default)]
    pub id: Option<String>,
    pub images: Vec<String>,
}

/// `/image/edit` (`EditImageRequest`) request. The source `image` is a base64-encoded string
/// (Venice's `image` field is `anyOf` upload/base64/URL; we send base64-in-JSON, no multipart).
/// The response is raw binary, so there is no response struct.
#[derive(Debug, Serialize)]
pub struct EditImageRequest {
    pub model: String,

    pub prompt: String,

    /// Base64-encoded source image bytes.
    pub image: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_format: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub aspect_ratio: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolution: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub safe_mode: Option<bool>,
}
