use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::OnceLock;

use regex::Regex;

use crate::agent::AgentPurpose;
use crate::agent::provider::entity::{TextGenerationParams, TextGenerationResult};
use crate::conversation::llm::{
    Author as LLMAuthor, Conversation as LLMConversation, Message as LLMMessage,
    MessageContent as LLMMessageContent, shorten_messages_list_to_context_size,
};
use crate::strings;

use super::config::{Config, WebSearchMode};
use super::utils::convert_llm_messages_to_venice;
use super::wire::{ChatCompletionRequest, ChatCompletionResponse, WebSearchCitation};

pub async fn generate_text(
    config: &Config,
    http: &reqwest::Client,
    conversation: LLMConversation,
    params: TextGenerationParams,
) -> anyhow::Result<TextGenerationResult> {
    let Some(text_generation_config) = &config.text_generation else {
        return Err(anyhow::anyhow!(
            strings::agent::no_configuration_for_purpose_so_cannot_be_used(
                &AgentPurpose::TextGeneration
            ),
        ));
    };

    let prompt_text = params.prompt_variables.format(
        params
            .prompt_override
            .unwrap_or(text_generation_config.prompt.clone().unwrap_or_default())
            .trim(),
    );

    // Prompt-cache routing key. Hash ONLY conversation-stable inputs: the rendered system prompt
    // and the conversation start time. Folding in anything per-turn (message content, the current
    // time, the message count) would mint a fresh key every turn, miss the cache every lookup, and
    // pay full price plus the hashing cost. The start time is rendered explicitly here so the key
    // stays stable even when the user's prompt template never mentions the time variable; an
    // unknown start time renders "unknown" and simply keys on the prompt alone.
    let conversation_start_time = params
        .prompt_variables
        .format("{{ baibot_conversation_start_time_utc }}");
    let prompt_cache_key = derive_prompt_cache_key(&prompt_text, &conversation_start_time);

    let prompt_message = if prompt_text.is_empty() {
        None
    } else {
        Some(LLMMessage {
            author: LLMAuthor::Prompt,
            sender_id: None,
            content: LLMMessageContent::Text(prompt_text),
            timestamp: chrono::Utc::now(),
        })
    };

    let mut conversation_messages = conversation.messages;

    if params.context_management_enabled {
        conversation_messages = shorten_messages_list_to_context_size(
            &text_generation_config.model_id,
            &prompt_message,
            conversation_messages,
            text_generation_config.max_response_tokens,
            text_generation_config.max_context_tokens,
        );
    }

    if let Some(prompt_message) = prompt_message {
        conversation_messages.insert(0, prompt_message);
    }

    let messages = convert_llm_messages_to_venice(conversation_messages)?;

    let temperature = params
        .temperature_override
        .unwrap_or(text_generation_config.temperature);

    // When web search is active, ask Venice to return structured search results so we can render
    // readable citations from them. Respect an explicit user choice and only fill the flag when
    // the user left it unset.
    let venice_parameters = text_generation_config
        .venice_parameters
        .clone()
        .map(|mut vp| {
            let web_search_active = matches!(
                vp.enable_web_search,
                Some(WebSearchMode::On | WebSearchMode::Auto)
            );
            if web_search_active && vp.return_search_results_as_documents.is_none() {
                vp.return_search_results_as_documents = Some(true);
            }
            vp
        });

    let request = ChatCompletionRequest {
        model: text_generation_config.model_id.clone(),
        messages,
        temperature: Some(temperature),
        // Web search rides entirely inside `venice_parameters`; there is no `tools` array here.
        // `max_tokens` is deprecated on Venice in favor of `max_completion_tokens`.
        max_completion_tokens: text_generation_config.max_response_tokens,
        top_p: text_generation_config.top_p,
        frequency_penalty: text_generation_config.frequency_penalty,
        presence_penalty: text_generation_config.presence_penalty,
        repetition_penalty: text_generation_config.repetition_penalty,
        reasoning_effort: text_generation_config.reasoning_effort.clone(),
        prompt_cache_key: Some(prompt_cache_key),
        prompt_cache_retention: text_generation_config.prompt_cache_retention.clone(),
        venice_parameters,
    };

    let url = format!(
        "{}/chat/completions",
        config.base_url.trim_end_matches('/')
    );

    tracing::trace!(
        model = text_generation_config.model_id,
        messages_count = request.messages.len(),
        "Sending Venice chat completion API request"
    );

    let response = http
        .post(&url)
        .bearer_auth(&config.api_key)
        .json(&request)
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() {
        // Log the body server-side for debugging (Venice explains a rejected strict body there),
        // but keep it OUT of the returned error: that error surfaces in the Matrix room, and the
        // body can carry account / rate-limit details that shouldn't reach room members.
        let body = response.text().await.unwrap_or_default();
        tracing::warn!(%status, body, "Venice chat completion request failed");

        // A 413 almost always means an attached file pushed the request past Venice's size limit.
        // Surface a clear, actionable message rather than the opaque status; the raw body still
        // stays out of the room for the reason above.
        if status == reqwest::StatusCode::PAYLOAD_TOO_LARGE {
            return Err(anyhow::anyhow!(
                "The request was too large for Venice, most likely an attached file over the 25MB limit."
            ));
        }

        return Err(anyhow::anyhow!(
            "Venice chat completion request failed with status {status}"
        ));
    }

    let response: ChatCompletionResponse = response.json().await?;

    let citations = response
        .venice_parameters
        .map(|vp| vp.web_search_citations)
        .unwrap_or_default();

    let Some(choice) = response.choices.into_iter().next() else {
        return Err(anyhow::anyhow!(
            "No choices were returned from the Venice chat completion API"
        ));
    };

    let Some(content) = choice.message.content else {
        return Err(anyhow::anyhow!(
            "No message content was returned from the Venice chat completion API"
        ));
    };

    let text = render_with_citations(content, &citations);
    let text = append_reasoning(
        text,
        choice.message.reasoning_content,
        text_generation_config.show_reasoning,
    );

    Ok(TextGenerationResult { text })
}

/// Builds the prompt-cache routing key from conversation-stable inputs. `DefaultHasher::new()` is a
/// fixed-seed SipHasher (keys 0,0), so it is deterministic across processes and restarts: identical
/// inputs always produce the same key, which is what lets a restarted bot keep hitting the warm
/// cache. The algorithm is not guaranteed stable across Rust std versions, so a rebuild on a new
/// toolchain can shift every key once, a one-time cache warm-up with no correctness effect.
pub(super) fn derive_prompt_cache_key(prompt_text: &str, conversation_start_time: &str) -> String {
    let mut hasher = DefaultHasher::new();
    prompt_text.hash(&mut hasher);
    conversation_start_time.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// Appends the model's thinking to the reply only when the deployment opts in via `show_reasoning`.
/// `reasoning_content` is a field separate from the answer `content` (it is unaffected by
/// `strip_thinking_response`, which only strips inline `<think>` blocks from `content`), so reading
/// it here is independent of that knob. Default-off matches today's behavior: thinking never reaches
/// a room that did not ask for it.
pub(super) fn append_reasoning(
    text: String,
    reasoning_content: Option<String>,
    show_reasoning: bool,
) -> String {
    if !show_reasoning {
        return text;
    }

    match reasoning_content {
        Some(reasoning) if !reasoning.trim().is_empty() => {
            format!("{text}\n\n---\n\n*Reasoning:*\n\n{reasoning}")
        }
        _ => text,
    }
}

/// Rewrites Venice's inline `^n^` citation superscripts into readable `[n]` references and appends
/// a `Sources:` list of markdown links, one per citation in order. Returns the content unchanged
/// when web search returned no citations, so non-search replies are never touched.
///
/// Citation `title` and `url` come from scraped web pages, so they are attacker-influenced. The
/// title is escaped so it cannot break out of the markdown link label, and the URL is used as a
/// link target only when it is a clean `http(s)` URL with no markdown-breaking characters;
/// otherwise the citation renders as plain text. This stops a hostile page title or URL from
/// injecting a spoofed clickable link into the room.
pub(super) fn render_with_citations(content: String, citations: &[WebSearchCitation]) -> String {
    if citations.is_empty() {
        return content;
    }

    let mut text = rewrite_citation_superscripts(&content);

    let mut sources = String::from("\n\nSources:");
    for (index, citation) in citations.iter().enumerate() {
        let n = index + 1;
        let title = escape_markdown_link_text(&citation.title);
        match sanitize_link_url(&citation.url) {
            // A citation that arrived with no title still renders as a usable link by showing the
            // URL as the link text, rather than an empty `[]( )` label.
            Some(url) if title.is_empty() => sources.push_str(&format!("\n[{n}] [{url}]({url})")),
            Some(url) => sources.push_str(&format!("\n[{n}] [{title}]({url})")),
            None if !title.is_empty() => sources.push_str(&format!("\n[{n}] {title}")),
            None => sources.push_str(&format!("\n[{n}] (source unavailable)")),
        }
    }

    text.push_str(&sources);
    text
}

/// Venice marks web-search citations with superscript runs in the reply text: a single `^1^`, a
/// comma list `^1,2^`, or a caret-chained run `^2^3^10^` where consecutive citations share a
/// caret. The whole run has to be matched at once: a per-citation pattern (string or regex)
/// consumes the shared caret on the first match and orphans the rest (`^2^3^` would leave `3^`).
/// So this matches each full run and expands it to one `[n]` per citation (`^2^3^` -> `[2][3]`).
fn rewrite_citation_superscripts(content: &str) -> String {
    static RUN: OnceLock<Regex> = OnceLock::new();
    let run = RUN.get_or_init(|| {
        Regex::new(r"\^\d+(?:[,^]\d+)*\^").expect("citation superscript regex is valid")
    });

    run.replace_all(content, |caps: &regex::Captures| {
        caps[0]
            .split(['^', ','])
            .filter(|piece| !piece.is_empty())
            .map(|n| format!("[{n}]"))
            .collect::<String>()
    })
    .into_owned()
}

/// Escapes the characters that would let citation title text break out of a markdown link label,
/// and folds newlines to spaces so a multi-line title cannot inject extra markdown structure.
fn escape_markdown_link_text(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('[', "\\[")
        .replace(']', "\\]")
        .replace(['\r', '\n'], " ")
}

/// Returns the URL as a markdown link target only when it is a clean `http(s)` URL with no
/// characters that would break the `(...)` destination or smuggle a different scheme. Anything else
/// returns `None`, so the caller renders the citation as plain text instead of a link.
fn sanitize_link_url(url: &str) -> Option<String> {
    let url = url.trim();
    let is_http = url.starts_with("https://") || url.starts_with("http://");
    let is_clean = !url.contains(['(', ')', '<', '>', ' ', '\t', '\r', '\n']);

    if is_http && is_clean {
        Some(url.to_owned())
    } else {
        None
    }
}
