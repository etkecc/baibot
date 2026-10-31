use super::wire::{ContentBlock, ErrorResponse, ImageSource, Message, ResponseContentBlock, Role};

use crate::conversation::llm::{
    Author as LLMAuthor, Message as LLMMessage, MessageContent as LLMMessageContent,
};
use crate::utils::text::render_sources_list;

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

/// Turns the response content blocks into the reply text, or an error if there is none.
///
/// Web search splits a sentence into several text blocks around its citations, so adjacent text
/// blocks are joined as-is, while text that follows other blocks (e.g. a tool call or result)
/// starts a new paragraph. Cited sources become inline `[n]` references with a `Sources:` list at
/// the end, as Anthropic requires citations to be shown to end users. Other blocks (e.g.
/// `thinking`) are not shown.
pub(super) fn response_text(
    content: &[serde_json::Value],
    stop_reason: Option<&str>,
) -> anyhow::Result<String> {
    let mut text = String::new();
    let mut sources: Vec<(String, String)> = vec![];
    let mut new_paragraph = false;

    for block in content {
        let ResponseContentBlock::Text {
            text: part,
            citations,
        } = ResponseContentBlock::from_value(block)
        else {
            new_paragraph = true;
            continue;
        };

        if new_paragraph && !text.is_empty() {
            text.push_str("\n\n");
        }
        new_paragraph = false;

        // References go before trailing whitespace, so they stay on the cited line.
        let cited = part.trim_end();
        text.push_str(cited);

        let mut refs = vec![];
        for citation in citations.into_iter().flatten() {
            let Some(url) = citation.url else {
                continue;
            };
            let n = match sources.iter().position(|(_, known)| *known == url) {
                Some(index) => index + 1,
                None => {
                    sources.push((citation.title.unwrap_or_default(), url));
                    sources.len()
                }
            };
            if !refs.contains(&n) {
                refs.push(n);
                text.push_str(&format!("[{n}]"));
            }
        }
        text.push_str(&part[cited.len()..]);
    }

    if !text.is_empty() {
        if !sources.is_empty() {
            text.push_str(&render_sources_list(
                sources
                    .iter()
                    .map(|(title, url)| (title.as_str(), url.as_str())),
            ));
        }
        return Ok(text);
    }

    match stop_reason {
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
    use serde_json::json;

    use super::super::wire::MessagesResponse;
    use super::*;

    fn parse(json: &str) -> MessagesResponse {
        serde_json::from_str(json).expect("valid response")
    }

    fn text_of(response: MessagesResponse) -> anyhow::Result<String> {
        response_text(&response.content, response.stop_reason.as_deref())
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

        assert_eq!(text_of(response).unwrap(), "Hello\n\nthere");
    }

    #[test]
    fn empty_response_reports_the_stop_reason() {
        let refusal = parse(r#"{"content": [], "stop_reason": "refusal"}"#);
        assert!(
            text_of(refusal)
                .unwrap_err()
                .to_string()
                .contains("declined")
        );

        let thinking_only = parse(
            r#"{"content": [{"type": "thinking", "thinking": ""}], "stop_reason": "max_tokens"}"#,
        );
        assert!(
            text_of(thinking_only)
                .unwrap_err()
                .to_string()
                .contains("max_response_tokens")
        );
    }

    #[test]
    fn web_search_reply_joins_split_text_and_lists_sources() {
        // Shaped like the web search example in Anthropic's docs.
        let content = json!([
            {"type": "text", "text": "I'll search for that."},
            {"type": "server_tool_use", "id": "srvtoolu_1", "name": "web_search", "input": {"query": "claude shannon birth date"}},
            {"type": "web_search_tool_result", "tool_use_id": "srvtoolu_1", "content": [
                {"type": "web_search_result", "url": "https://en.wikipedia.org/wiki/Claude_Shannon", "title": "Claude Shannon - Wikipedia", "encrypted_content": "abc", "page_age": null}
            ]},
            {"type": "text", "text": "Based on the search results, "},
            {"type": "text", "text": "Claude Shannon was born on April 30, 1916", "citations": [
                {"type": "web_search_result_location", "url": "https://en.wikipedia.org/wiki/Claude_Shannon", "title": "Claude Shannon - Wikipedia", "encrypted_index": "def", "cited_text": "Claude Elwood Shannon (April 30, 1916 – February 24, 2001)"},
                {"type": "web_search_result_location", "url": "https://en.wikipedia.org/wiki/Claude_Shannon", "title": "Claude Shannon - Wikipedia", "encrypted_index": "ghi", "cited_text": "born in Petoskey, Michigan"}
            ]},
            {"type": "text", "text": "."}
        ]);

        assert_eq!(
            response_text(content.as_array().unwrap(), Some("end_turn")).unwrap(),
            "I'll search for that.\n\nBased on the search results, Claude Shannon was born on April 30, 1916[1].\n\nSources:\n[1] [Claude Shannon - Wikipedia](https://en.wikipedia.org/wiki/Claude_Shannon)"
        );
    }

    #[test]
    fn references_stay_on_the_cited_line() {
        let content = json!([
            {"type": "text", "text": "- one\n", "citations": [
                {"type": "web_search_result_location", "url": "https://example.com/", "title": "Example"}
            ]},
            {"type": "text", "text": "- two"}
        ]);

        assert!(
            response_text(content.as_array().unwrap(), None)
                .unwrap()
                .starts_with("- one[1]\n- two")
        );
    }

    #[test]
    fn citations_without_a_url_are_not_listed() {
        let content = json!([
            {"type": "text", "text": "Fetched.", "citations": [
                {"type": "char_location", "document_title": "Page", "cited_text": "x", "start_char_index": 0, "end_char_index": 1}
            ]}
        ]);

        assert_eq!(
            response_text(content.as_array().unwrap(), None).unwrap(),
            "Fetched."
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
