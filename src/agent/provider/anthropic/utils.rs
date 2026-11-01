use super::wire::{
    ContentBlock, ErrorResponse, ImageSource, Message, MessagesResponse, ResponseContentBlock, Role,
};

use crate::conversation::llm::{
    Author as LLMAuthor, Message as LLMMessage, MessageContent as LLMMessageContent,
};

pub(super) fn create_anthropic_messages(llm_messages: Vec<LLMMessage>) -> Vec<Message> {
    let mut messages = vec![];

    for message in llm_messages {
        let role = match message.author {
            LLMAuthor::User => Role::User,
            LLMAuthor::Assistant => Role::Assistant,
            LLMAuthor::Prompt => {
                continue;
            }
        };

        let content = match &message.content {
            LLMMessageContent::Text(text) => vec![ContentBlock::Text { text: text.clone() }],
            LLMMessageContent::Image(image_details) => {
                vec![ContentBlock::Image {
                    source: ImageSource::Base64 {
                        media_type: image_details.mime.to_string(),
                        data: crate::utils::base64::base64_encode(&image_details.data),
                    },
                }]
            }
            LLMMessageContent::File(file_details) => {
                tracing::warn!(
                    "The Anthropic provider does not support file/document content yet. This file message ({}) will be skipped.",
                    file_details.filename(),
                );
                continue;
            }
        };

        messages.push(Message { role, content });
    }

    messages
}

/// Joins the response's text blocks. Other blocks (e.g. `thinking`) are not shown.
pub(super) fn response_text(response: MessagesResponse) -> anyhow::Result<String> {
    let text_parts: Vec<String> = response
        .content
        .into_iter()
        .filter_map(|block| match block {
            ResponseContentBlock::Text { text } => Some(text),
            ResponseContentBlock::Other => None,
        })
        .collect();

    if !text_parts.is_empty() {
        return Ok(text_parts.join("\n\n"));
    }

    match response.stop_reason.as_deref() {
        Some("refusal") => Err(anyhow::anyhow!(
            "The model declined to respond to this request"
        )),
        Some("max_tokens") => Err(anyhow::anyhow!(
            "The model used up all response tokens (`max_response_tokens`) before writing a reply"
        )),
        _ => Err(anyhow::anyhow!(
            "No text content in response from the Anthropic create message API"
        )),
    }
}

/// Turns a non-2xx response body into a readable error message.
pub(super) fn error_message(status: reqwest::StatusCode, body: &str) -> String {
    match serde_json::from_str::<ErrorResponse>(body) {
        Ok(response) => format!(
            "Anthropic API error ({status}, {}): {}",
            response.error.kind, response.error.message
        ),
        Err(_) => format!("Anthropic API error ({status})"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(json: &str) -> MessagesResponse {
        serde_json::from_str(json).expect("valid response")
    }

    #[test]
    fn thinking_and_unknown_blocks_are_skipped() {
        let response = parse(
            r#"{
                "id": "msg_1",
                "type": "message",
                "role": "assistant",
                "content": [
                    {"type": "thinking", "thinking": "", "signature": "abc"},
                    {"type": "text", "text": "Hello"},
                    {"type": "server_tool_use", "id": "x", "name": "web_search", "input": {}},
                    {"type": "text", "text": "there", "citations": null}
                ],
                "stop_reason": "end_turn",
                "usage": {"input_tokens": 1, "output_tokens": 2}
            }"#,
        );

        assert_eq!(response_text(response).unwrap(), "Hello\n\nthere");
    }

    #[test]
    fn empty_response_reports_the_stop_reason() {
        let refusal = parse(r#"{"content": [], "stop_reason": "refusal"}"#);
        assert!(
            response_text(refusal)
                .unwrap_err()
                .to_string()
                .contains("declined")
        );

        let thinking_only = parse(
            r#"{"content": [{"type": "thinking", "thinking": ""}], "stop_reason": "max_tokens"}"#,
        );
        assert!(
            response_text(thinking_only)
                .unwrap_err()
                .to_string()
                .contains("max_response_tokens")
        );
    }

    #[test]
    fn error_message_includes_the_api_message() {
        let body = r#"{"type": "error", "error": {"type": "invalid_request_error", "message": "temperature is not supported"}}"#;
        assert_eq!(
            error_message(reqwest::StatusCode::BAD_REQUEST, body),
            "Anthropic API error (400 Bad Request, invalid_request_error): temperature is not supported"
        );

        assert_eq!(
            error_message(reqwest::StatusCode::BAD_GATEWAY, "<html>"),
            "Anthropic API error (502 Bad Gateway)"
        );
    }
}
