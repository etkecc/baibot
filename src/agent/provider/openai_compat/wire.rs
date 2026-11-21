//! Serde structs modeling the subset of the OpenAI API that OpenAI-compatible providers (Groq,
//! LocalAI, Ollama, OpenRouter, ...) implement and baibot uses: `/chat/completions`,
//! `/audio/transcriptions`, `/audio/speech` and `/images/generations`. Request types are
//! `Serialize`-only and response types are `Deserialize`-only, as in `venice/wire.rs`.
//!
//! Model IDs, voices, sizes, etc. are plain strings, since each provider has its own. Optional
//! fields are omitted rather than sent as `null`, as some providers reject unknown or `null` fields.
//! `/audio/speech` returns raw binary audio, so it has no response struct.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize)]
pub struct ChatCompletionRequest {
    pub model: String,

    pub messages: Vec<ChatMessage>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,

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
    pub role: &'static str,

    /// Text, or for an assistant message which calls tools, the content as received (possibly a list of chunks).
    /// Only missing for an assistant message which just calls tools.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<serde_json::Value>,

    /// The calls of an assistant message.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<ToolCall>,

    /// The call a `tool` message is the result of.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,

    /// The reasoning of an assistant message which calls tools, sent back as received,
    /// as some providers require (DeepSeek's thinking mode) or recommend (OpenRouter).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_content: Option<serde_json::Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_details: Option<serde_json::Value>,
}

impl ChatMessage {
    pub fn text(role: &'static str, content: String) -> Self {
        Self {
            role,
            content: Some(serde_json::Value::String(content)),
            tool_calls: vec![],
            tool_call_id: None,
            reasoning_content: None,
            reasoning_details: None,
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

#[derive(Debug, Deserialize)]
pub struct ChatCompletionResponse {
    #[serde(default)]
    pub choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
pub struct ChatChoice {
    #[serde(default)]
    pub message: Option<ChatResponseMessage>,

    #[serde(default)]
    pub finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ChatResponseMessage {
    /// Usually text, but some models send a list of chunks (e.g. Mistral's reasoning models: `thinking` and `text`).
    #[serde(default)]
    pub content: serde_json::Value,

    #[serde(default)]
    pub reasoning_content: Option<serde_json::Value>,

    #[serde(default)]
    pub reasoning_details: Option<serde_json::Value>,

    /// Some providers send `null` instead of leaving it out.
    #[serde(default)]
    pub tool_calls: Option<Vec<ResponseToolCall>>,
}

/// A tool call in a response. Providers differ a little, so the id is optional
/// and the arguments may be a JSON text (as OpenAI sends them) or an object.
#[derive(Debug, Deserialize)]
pub struct ResponseToolCall {
    #[serde(default)]
    pub id: Option<String>,

    pub function: ResponseFunctionCall,
}

impl ChatResponseMessage {
    /// The text of the message, if any. Of a list of chunks, only the `text` ones count.
    pub fn text(&self) -> Option<String> {
        match &self.content {
            serde_json::Value::String(text) => Some(text.clone()),
            serde_json::Value::Array(chunks) => {
                let text: String = chunks
                    .iter()
                    .filter(|chunk| chunk["type"] == "text")
                    .filter_map(|chunk| chunk["text"].as_str())
                    .collect();

                (!text.is_empty()).then_some(text)
            }
            _ => None,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ResponseFunctionCall {
    pub name: String,

    #[serde(default)]
    pub arguments: serde_json::Value,
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
