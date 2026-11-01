//! Serde structs modeling the subset of Anthropic's Messages API (`POST /v1/messages`) that baibot
//! uses. Request types are `Serialize`-only and response types are `Deserialize`-only, as in
//! `venice/wire.rs`.
//!
//! Response content blocks other than `text` (`thinking`, `redacted_thinking`, server tool blocks,
//! ...) deserialize into `ResponseContentBlock::Other`, so a new block type never fails the whole
//! response.

use serde::{Deserialize, Serialize};

/// The API version sent in the `anthropic-version` header.
///
/// `2026-10-21` rejects `temperature` on every model, so moving to it is a separate change.
pub const API_VERSION: &str = "2023-06-01";

#[derive(Debug, Serialize)]
pub struct MessagesRequest {
    pub model: String,

    pub max_tokens: u32,

    pub messages: Vec<Message>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
}

#[derive(Debug, Serialize)]
pub struct Message {
    pub role: Role,

    pub content: Vec<ContentBlock>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    User,
    Assistant,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    Text { text: String },
    Image { source: ImageSource },
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ImageSource {
    Base64 { media_type: String, data: String },
}

#[derive(Debug, Deserialize)]
pub struct MessagesResponse {
    pub content: Vec<ResponseContentBlock>,

    #[serde(default)]
    pub stop_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResponseContentBlock {
    Text {
        text: String,
    },

    #[serde(other)]
    Other,
}

/// The body of a non-2xx response: `{"type": "error", "error": {"type": "...", "message": "..."}}`.
#[derive(Debug, Deserialize)]
pub struct ErrorResponse {
    pub error: ErrorDetails,
}

#[derive(Debug, Deserialize)]
pub struct ErrorDetails {
    #[serde(rename = "type")]
    pub kind: String,

    pub message: String,
}
