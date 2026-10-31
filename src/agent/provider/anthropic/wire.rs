//! Serde structs modeling the subset of Anthropic's Messages API (`POST /v1/messages`) that baibot
//! uses. Request types are `Serialize`-only and response types are `Deserialize`-only, as in
//! `venice/wire.rs`.
//!
//! Response content blocks are kept as raw JSON, so a paused turn (`pause_turn`) can be sent back
//! unchanged. They are read through `ResponseContentBlock`, where blocks other than `text`
//! (`thinking`, server tool blocks, ...) become `Other`, so a new block type never fails the whole
//! response.

use serde::{Deserialize, Serialize};

/// The API version sent in the `anthropic-version` header.
///
/// This version rejects `temperature`, `top_p` and `top_k` on every model, so none are sent.
pub const API_VERSION: &str = "2026-10-21";

#[derive(Debug, Serialize)]
pub struct MessagesRequest {
    pub model: String,

    pub max_tokens: u32,

    pub messages: Vec<Message>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<Tool>,

    /// The code execution container to reuse when continuing a paused turn. Web search and web
    /// fetch run their result filtering in one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container: Option<String>,
}

/// A server tool, which Anthropic runs on its side (e.g. `web_search_20260209`).
#[derive(Debug, Serialize)]
pub struct Tool {
    #[serde(rename = "type")]
    pub kind: &'static str,

    pub name: &'static str,
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
    Text {
        text: String,
    },
    Image {
        source: ImageSource,
    },

    /// A response block sent back unchanged, to continue a paused turn.
    #[serde(untagged)]
    Raw(serde_json::Value),
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ImageSource {
    Base64 { media_type: String, data: String },
}

#[derive(Debug, Deserialize)]
pub struct MessagesResponse {
    pub content: Vec<serde_json::Value>,

    #[serde(default)]
    pub stop_reason: Option<String>,

    #[serde(default)]
    pub container: Option<Container>,
}

#[derive(Debug, Deserialize)]
pub struct Container {
    pub id: String,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResponseContentBlock {
    Text {
        text: String,

        #[serde(default)]
        citations: Option<Vec<Citation>>,
    },

    #[serde(other)]
    Other,
}

impl ResponseContentBlock {
    pub fn from_value(value: &serde_json::Value) -> Self {
        Self::deserialize(value).unwrap_or(Self::Other)
    }
}

/// A citation on a `text` block. Web search results (`web_search_result_location`) carry a `url`
/// and `title`; other citation types have no `url` and are not listed as sources.
#[derive(Debug, Deserialize)]
pub struct Citation {
    #[serde(default)]
    pub url: Option<String>,

    #[serde(default)]
    pub title: Option<String>,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_paused_turn_is_sent_back_unchanged() {
        let block = serde_json::json!({"type": "server_tool_use", "id": "srvtoolu_1", "name": "web_search", "input": {"query": "x"}});

        let message = Message {
            role: Role::Assistant,
            content: vec![ContentBlock::Raw(block.clone())],
        };

        assert_eq!(
            serde_json::to_value(&message).unwrap(),
            serde_json::json!({"role": "assistant", "content": [block]})
        );
    }
}
