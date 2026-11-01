//! Serde structs modeling the subset of the OpenAI API that OpenAI-compatible providers (Groq,
//! LocalAI, Ollama, OpenRouter, ...) implement and baibot uses: `/chat/completions`,
//! `/audio/transcriptions`, `/audio/speech` and `/images/generations`. Request types are
//! `Serialize`-only and response types are `Deserialize`-only, as in `venice/wire.rs`.
//!
//! Model IDs, voices, sizes, etc. are plain strings, since each provider has its own. Optional
//! fields are omitted rather than sent as `null`, as some providers reject unknown or `null` fields.
//! `/audio/speech` returns raw binary audio, so it has no response struct.

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct ChatCompletionRequest {
    pub model: String,

    pub messages: Vec<ChatMessage>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct ChatMessage {
    pub role: &'static str,

    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct ChatCompletionResponse {
    #[serde(default)]
    pub choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
pub struct ChatChoice {
    #[serde(default)]
    pub message: Option<ChatResponseMessage>,
}

#[derive(Debug, Deserialize)]
pub struct ChatResponseMessage {
    #[serde(default)]
    pub content: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TranscriptionResponse {
    #[serde(default)]
    pub text: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SpeechRequest {
    pub model: String,

    pub input: String,

    pub voice: String,

    pub speed: f32,

    pub response_format: String,
}

#[derive(Debug, Serialize)]
pub struct ImageGenerationRequest {
    pub model: String,

    pub prompt: String,

    pub n: u8,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality: Option<String>,

    pub response_format: &'static str,
}

#[derive(Debug, Deserialize)]
pub struct ImageGenerationResponse {
    #[serde(default)]
    pub data: Vec<ImageData>,
}

#[derive(Debug, Deserialize)]
pub struct ImageData {
    #[serde(default)]
    pub b64_json: Option<String>,

    #[serde(default)]
    pub revised_prompt: Option<String>,
}
