use anthropic::types::{
    ContentBlock, ImageSource, Message, MessagesRequest, MessagesRequestBuilder, Role,
};

use crate::conversation::llm::{
    Author as LLMAuthor, Message as LLMMessage, MessageContent as LLMMessageContent,
};
use crate::utils::text::render_sources_list;

pub(super) fn create_anthropic_message_request(llm_messages: Vec<LLMMessage>) -> MessagesRequest {
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
            LLMMessageContent::Text(text) => vec![ContentBlock::Text {
                text: text.clone(),
                citations: None,
            }],
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
                    "The Anthropic provider's library does not support file/document content. This file message ({}) will be skipped.",
                    file_details.filename(),
                );
                continue;
            }
        };

        let message = Message { role, content };

        messages.push(message);
    }

    MessagesRequestBuilder::default()
        .messages(messages)
        .stream(false)
        .build()
        .expect("Failed to build messages request")
}

/// Turns the response content blocks into the reply text.
///
/// Web search splits a sentence into several text blocks around its citations, so adjacent text
/// blocks are joined as-is, while text that follows a tool call or result starts a new paragraph.
/// Cited sources become inline `[n]` references with a `Sources:` list at the end, as Anthropic
/// requires citations to be shown to end users.
pub(super) fn response_content_to_text(content: &[ContentBlock]) -> String {
    let mut text = String::new();
    let mut sources: Vec<(&str, &str)> = vec![];
    let mut new_paragraph = false;

    for block in content {
        match block {
            ContentBlock::Text {
                text: part,
                citations,
            } => {
                if new_paragraph && !text.is_empty() {
                    text.push_str("\n\n");
                }
                new_paragraph = false;
                // References go before trailing whitespace, so they stay on the cited line.
                let cited = part.trim_end();
                text.push_str(cited);

                let mut refs = vec![];
                for citation in citations.iter().flatten() {
                    let Some(url) = citation["url"].as_str() else {
                        continue;
                    };
                    let n = match sources.iter().position(|(_, known)| *known == url) {
                        Some(index) => index + 1,
                        None => {
                            sources.push((citation["title"].as_str().unwrap_or_default(), url));
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
            ContentBlock::Image { .. } => {
                if !text.is_empty() {
                    text.push_str("\n\n");
                }
                text.push_str("The model responded with an image");
                new_paragraph = true;
            }
            ContentBlock::Other(_) => new_paragraph = true,
        }
    }

    if !sources.is_empty() {
        text.push_str(&render_sources_list(sources));
    }

    text
}

#[cfg(test)]
mod tests {
    use anthropic::types::ContentBlock;
    use serde_json::json;

    use super::response_content_to_text;

    #[test]
    fn web_search_reply_joins_split_text_and_lists_sources() {
        // Shaped like the web search example in Anthropic's docs.
        let content: Vec<ContentBlock> = serde_json::from_value(json!([
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
        ]))
        .unwrap();

        assert_eq!(
            response_content_to_text(&content),
            "I'll search for that.\n\nBased on the search results, Claude Shannon was born on April 30, 1916[1].\n\nSources:\n[1] [Claude Shannon - Wikipedia](https://en.wikipedia.org/wiki/Claude_Shannon)"
        );
    }

    #[test]
    fn references_stay_on_the_cited_line() {
        let content: Vec<ContentBlock> = serde_json::from_value(json!([
            {"type": "text", "text": "- one\n", "citations": [
                {"type": "web_search_result_location", "url": "https://example.com/", "title": "Example"}
            ]},
            {"type": "text", "text": "- two"}
        ]))
        .unwrap();

        assert!(response_content_to_text(&content).starts_with("- one[1]\n- two"));
    }

    #[test]
    fn plain_reply_is_unchanged() {
        let content: Vec<ContentBlock> = serde_json::from_value(json!([
            {"type": "thinking", "thinking": "", "signature": "sig"},
            {"type": "text", "text": "Hello!"}
        ]))
        .unwrap();

        assert_eq!(response_content_to_text(&content), "Hello!");
    }
}
