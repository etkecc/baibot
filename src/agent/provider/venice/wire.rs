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

#[derive(Clone, Debug, Serialize)]
pub struct ChatCompletionRequest {
    pub model: String,

    pub messages: Vec<ChatMessage>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_completion_tokens: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub repetition_penalty: Option<f32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_cache_key: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_cache_retention: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub venice_parameters: Option<VeniceParameters>,

    /// Functions the model may call (the bot's tools). Venice's own web search is not one of them:
    /// it is turned on via `venice_parameters`.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<Tool>,
}

/// A function the model may call (`{"type": "function", "function": {...}}`).
#[derive(Clone, Debug, Serialize)]
pub struct Tool {
    #[serde(rename = "type")]
    pub kind: &'static str,

    pub function: FunctionDefinition,
}

#[derive(Clone, Debug, Serialize)]
pub struct FunctionDefinition {
    pub name: String,

    pub description: String,

    pub parameters: serde_json::Value,
}

#[derive(Clone, Debug, Serialize)]
pub struct ChatMessage {
    pub role: String,

    /// Only missing for an assistant message which just calls tools.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<MessageContent>,

    /// The calls of an assistant message.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<ToolCall>,

    /// The call a `tool` message is the result of.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,

    /// The reasoning of an assistant message which calls tools, sent back as received, as some
    /// models require it to continue (e.g. DeepSeek's thinking mode, Gemini's thought signatures).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_content: Option<serde_json::Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_details: Option<serde_json::Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub thought_signature: Option<serde_json::Value>,
}

impl ChatMessage {
    pub fn new(role: &str, content: MessageContent) -> Self {
        Self {
            role: role.to_owned(),
            content: Some(content),
            tool_calls: vec![],
            tool_call_id: None,
            reasoning_content: None,
            reasoning_details: None,
            thought_signature: None,
        }
    }
}

/// A tool call of an assistant message, as sent back.
#[derive(Clone, Debug, Serialize)]
pub struct ToolCall {
    pub id: String,

    #[serde(rename = "type")]
    pub kind: &'static str,

    pub function: FunctionCall,
}

#[derive(Clone, Debug, Serialize)]
pub struct FunctionCall {
    pub name: String,

    /// A JSON text.
    pub arguments: String,
}

/// A message body is either a bare string or a list of content parts. Venice accepts both; we
/// send the parts form when a message carries an image or a file (baibot keeps text, images, and
/// files in separate messages, so a parts list holds a single image part or file part).
#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Parts(Vec<ContentPart>),
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentPart {
    ImageUrl { image_url: ImageUrl },
    File { file: FilePart },
}

#[derive(Clone, Debug, Serialize)]
pub struct ImageUrl {
    /// A `data:<mime>;base64,<data>` URI for inline images.
    pub url: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct FilePart {
    /// A `data:<mime>;base64,<data>` URI carrying the file bytes inline.
    pub file_data: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
}

/// Standard OpenAI-shaped chat completion response. We read `choices[0].message.content` and,
/// when web search is on, the structured `venice_parameters.web_search_citations` (requested via
/// `return_search_results_as_documents`) to rewrite the inline `^n^` superscripts into readable
/// `[n]` references plus a `Sources:` block. `reasoning_content` carries the model's thinking when
/// the model exposes it; it is appended only when `show_reasoning` is set.
#[derive(Debug, Deserialize)]
pub struct ChatCompletionResponse {
    pub choices: Vec<ChatChoice>,

    #[serde(default)]
    pub venice_parameters: Option<ResponseVeniceParameters>,
}

#[derive(Debug, Deserialize)]
pub struct ChatChoice {
    pub message: ResponseMessage,

    #[serde(default)]
    pub finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ResponseMessage {
    #[serde(default)]
    pub content: Option<String>,

    /// Kept as received (usually text), so it can be sent back as is with tool calls.
    #[serde(default)]
    pub reasoning_content: Option<serde_json::Value>,

    #[serde(default)]
    pub reasoning_details: Option<serde_json::Value>,

    #[serde(default)]
    pub thought_signature: Option<serde_json::Value>,

    /// Calls of the bot's tools, but possibly also Venice's own (a `web_search` call carrying web search results). The
    /// schema leaves the items open, so they are read leniently (see [`Self::tool_calls`]) and an odd one can't make an
    /// ordinary reply unreadable.
    #[serde(default)]
    pub tool_calls: Option<serde_json::Value>,
}

impl ResponseMessage {
    /// The reasoning as text, for showing it (see `show_reasoning`).
    pub fn reasoning_text(&self) -> Option<String> {
        self.reasoning_content
            .as_ref()
            .and_then(|value| value.as_str())
            .map(str::to_owned)
    }

    /// The tool calls which could be read. Items of another shape are skipped.
    pub fn tool_calls(&self) -> Vec<ResponseToolCall> {
        let Some(serde_json::Value::Array(items)) = &self.tool_calls else {
            return vec![];
        };

        items
            .iter()
            .filter_map(|item| serde_json::from_value(item.clone()).ok())
            .collect()
    }
}

#[derive(Debug, Deserialize)]
pub struct ResponseToolCall {
    #[serde(default)]
    pub id: Option<String>,

    #[serde(default)]
    pub function: Option<ResponseFunctionCall>,
}

#[derive(Debug, Deserialize)]
pub struct ResponseFunctionCall {
    #[serde(default)]
    pub name: Option<String>,

    /// Usually a JSON text, but possibly an object.
    #[serde(default)]
    pub arguments: serde_json::Value,
}

/// The `venice_parameters` envelope on a chat-completion *response*, distinct from the request-side
/// `VeniceParameters` bag. Only the citation list is read; other response-side fields are ignored.
#[derive(Debug, Deserialize, Default)]
pub struct ResponseVeniceParameters {
    #[serde(default)]
    pub web_search_citations: Vec<WebSearchCitation>,
}

/// Only the `title` and `url` are read (for rendering the `Sources:` block). Venice also returns
/// `content` and `date` per citation; serde drops them, the same way the response structs above
/// ignore the response fields baibot does not use. Both fields default to empty so a single
/// citation that arrives without one (schema drift on scraped results) degrades gracefully in the
/// rendered list instead of failing the whole response deserialization.
#[derive(Clone, Debug, Deserialize)]
pub struct WebSearchCitation {
    #[serde(default)]
    pub title: String,

    #[serde(default)]
    pub url: String,
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
