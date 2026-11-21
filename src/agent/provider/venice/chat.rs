use crate::agent::provider::entity::{PreparedConversation, prepare_conversation};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::OnceLock;

use regex::Regex;

use crate::agent::AgentPurpose;
use crate::agent::provider::entity::{TextGenerationParams, TextGenerationResult};
use crate::conversation::llm::{Conversation as LLMConversation, TokenEstimate};
use crate::strings;
use crate::utils::text::render_sources_list;

use crate::tools::ProviderState;

use super::config::{Config, TextGenerationConfig, WebSearchMode};
use super::utils::{
    convert_llm_messages_to_venice, has_tool_calls, tool_calls, tool_result_messages, tools,
};
use super::wire::{ChatCompletionRequest, ChatCompletionResponse, WebSearchCitation};

pub async fn generate_text(
    config: &Config,
    http: &reqwest::Client,
    unsupported: &super::recovery::UnsupportedFieldsCache,
    conversation: LLMConversation,
    mut params: TextGenerationParams,
) -> anyhow::Result<TextGenerationResult> {
    let Some(text_generation_config) = &config.text_generation else {
        return Err(anyhow::anyhow!(
            strings::agent::no_configuration_for_purpose_so_cannot_be_used(
                &AgentPurpose::TextGeneration
            ),
        ));
    };

    let model_id = text_generation_config.model_id.clone();

    let known_unsupported = unsupported.known_for(&model_id);

    if !text_generation_config.bot_tools || known_unsupported.contains(super::recovery::TOOLS_FIELD)
    {
        params.tools.clear();
    }

    let first_round = params.continuation.is_none();

    // Kept for building the request again without tools, if the model rejects them.
    let mut retry_conversation =
        (first_round && !params.tools.is_empty()).then(|| conversation.clone());

    let (mut request, earlier_citations) = match params.continuation.take() {
        // The request so far already has everything else (system prompt, tools, the conversation, the model's tool
        // calls and Venice's web search results), so only the results are added. Web search stays on, though Venice
        // doesn't seem to search again in continued turns.
        Some(continuation) => {
            let Continuation {
                mut request,
                citations,
            } = continuation.state.into_inner()?;
            request
                .messages
                .extend(tool_result_messages(continuation.results));
            (request, citations)
        }
        None => (
            create_request(text_generation_config, conversation, &params)?,
            vec![],
        ),
    };

    let url = format!("{}/chat/completions", config.base_url.trim_end_matches('/'));

    // Proactively drop fields this model has already rejected earlier in this process, so a known
    // mismatch costs zero wasted round-trips after the first discovery. Venice's body is
    // `additionalProperties: false`, so sending a known-unsupported field would 400 again.
    for field in known_unsupported {
        super::recovery::strip_droppable_field(&mut request, &field);
    }

    // Set when the model rejected the tools, until the request works without them.
    let mut tools_dropped = false;

    // Send with bounded auto-recovery. When Venice 400s because a model does not support an optional
    // knob, it names the field (`field: '...'`); if that field is one we may safely drop, we strip
    // it, remember the rejection for this model, and retry. The loop is bounded: each retry clears a
    // distinct droppable field (strip returns false once it is gone), so after at most
    // `DROPPABLE_FIELDS.len()` retries (plus one for the tools) the request either succeeds or surfaces the error.
    let response = loop {
        if first_round {
            tracing::trace!(
                model = model_id,
                messages_count = request.messages.len(),
                tools_count = request.tools.len(),
                "Sending Venice chat completion API request"
            );
        } else {
            tracing::trace!(
                model = model_id,
                "Continuing a Venice chat completion with tool results"
            );
        }

        let response = http
            .post(&url)
            .bearer_auth(&config.api_key)
            .json(&request)
            .send()
            .await?;

        let status = response.status();
        if status.is_success() {
            break response;
        }

        // Always log the body server-side: Venice explains a rejected strict body there.
        let body = response.text().await.unwrap_or_default();
        tracing::warn!(%status, body, "Venice chat completion request failed");

        // Recover from a strict-body 400 over an unsupported optional knob: strip the named field
        // and retry. Only fields in `DROPPABLE_FIELDS` are eligible, so a meaning-bearing knob (a
        // sampling parameter) is never silently dropped; that case falls through to surface below.
        if status == reqwest::StatusCode::BAD_REQUEST
            && let Some(field) = super::recovery::parse_rejected_field(&body)
            && super::recovery::strip_droppable_field(&mut request, &field)
        {
            unsupported.record(&model_id, &field);
            tracing::info!(
                model = model_id,
                field,
                "Venice rejected an unsupported field; dropping it and retrying"
            );
            continue;
        }

        // Function calling is model-specific, so a model may reject the tools. It is answered without them then.
        if status == reqwest::StatusCode::BAD_REQUEST
            && first_round
            && !request.tools.is_empty()
            && super::recovery::is_tools_rejection(&body)
        {
            tracing::warn!(
                model = model_id,
                "The model rejected a request with tools - retrying without them"
            );
            without_tools(
                &mut request,
                &mut params,
                text_generation_config,
                retry_conversation.take().unwrap_or_default(),
            )?;

            // Fields this model rejected so far are dropped from the new request too.
            for field in unsupported.known_for(&model_id) {
                super::recovery::strip_droppable_field(&mut request, &field);
            }

            tools_dropped = true;
            continue;
        }

        // A 413 almost always means an attached file pushed the request past Venice's size limit.
        if status == reqwest::StatusCode::PAYLOAD_TOO_LARGE {
            return Err(anyhow::anyhow!(
                "The request was too large for Venice, most likely an attached file over the 25MB limit."
            ));
        }

        // A 400 is a complaint about the request baibot built, so the body is safe and useful to
        // surface: it tells the operator (e.g. at agent-create time) exactly which field or value
        // Venice rejected, instead of an opaque status. Other statuses keep the body OUT of the
        // returned error, since it can carry account / rate-limit details that shouldn't reach the
        // room.
        if status == reqwest::StatusCode::BAD_REQUEST {
            return Err(anyhow::anyhow!(
                "Venice rejected the request (400 Bad Request): {}",
                super::recovery::extract_error_message(&body)
            ));
        }

        return Err(anyhow::anyhow!(
            "Venice chat completion request failed with status {status}"
        ));
    };

    let response: ChatCompletionResponse = response.json().await?;

    let (result, answered) =
        read_response(response, request, earlier_citations, text_generation_config)?;

    // Only when it works without them (an answer, not an error or nothing), it's really about the tools.
    if tools_dropped && answered {
        tracing::warn!(model = model_id, "Not offering tools to this model anymore");
        unsupported.record(&model_id, super::recovery::TOOLS_FIELD);
    }

    Ok(result)
}

/// What a turn with tool calls continues with: the request so far, and the citations of the web search done in an
/// earlier round (Venice reports them only in the response of the round which searched).
#[derive(Clone)]
struct Continuation {
    request: ChatCompletionRequest,
    citations: Vec<WebSearchCitation>,
}

/// Turns a response into an answer, or into tool calls to continue `request` with. `earlier_citations` are those of
/// the turn's earlier rounds, used when this response has none of its own.
/// Also tells whether the model answered (wrote something besides thinking, before sources and reasoning are added).
fn read_response(
    response: ChatCompletionResponse,
    mut request: ChatCompletionRequest,
    earlier_citations: Vec<WebSearchCitation>,
    text_generation_config: &TextGenerationConfig,
) -> anyhow::Result<(TextGenerationResult, bool)> {
    let own_citations = response
        .venice_parameters
        .map(|vp| vp.web_search_citations)
        .unwrap_or_default();

    let citations = if own_citations.is_empty() {
        earlier_citations
    } else {
        own_citations.clone()
    };

    let Some(choice) = response.choices.into_iter().next() else {
        return Err(anyhow::anyhow!(
            "No choices were returned from the Venice chat completion API"
        ));
    };

    // Only needed when Venice was asked not to strip the thinking, which is when tools are offered (see
    // `create_request`), so that literal tags in ordinary replies stay alone.
    let strip_thinking =
        wants_thinking_stripped(text_generation_config) && !request.tools.is_empty();

    // Venice's finish reason for tool calls is `tool_calls`, but whether there are any is what counts.
    if has_tool_calls(&choice.message, &request.tools) {
        // A call cut off by the response limit can't run, and the text before it isn't an answer.
        if choice.finish_reason.as_deref() == Some("length") {
            return Err(anyhow::anyhow!(
                "The model used up all response tokens (`max_response_tokens`) while calling a tool"
            ));
        }

        let text = choice
            .message
            .content
            .clone()
            .map(|text| {
                if strip_thinking {
                    strip_thinking_blocks(&text)
                } else {
                    text
                }
            })
            .filter(|text| !text.trim().is_empty());

        // Besides the text, the content may carry the thinking (stripped from the text only), which goes back too.
        let text_bytes = text.as_deref().map_or(0, str::len);

        // It's the reply if the model may not call more tools, so it gets its sources too. Only those of this round's
        // search, as the text counts against the tool budget, and earlier sources would be counted again each round.
        let text = text.map(|text| render_with_citations(text, &own_citations));

        // Sent back as received (thinking included), as models may need it to continue.
        // Venice's own calls (web search results) are sent back with their results, which aren't counted, like those of
        // other providers' hosted tools.
        let (calls, assistant_message, venice_results) = tool_calls(choice.message, &request.tools);
        let reasoning_bytes = crate::tools::json_bytes(&assistant_message.content)
            .saturating_sub(text_bytes)
            + crate::tools::json_bytes(&assistant_message.reasoning_content)
            + crate::tools::json_bytes(&assistant_message.reasoning_details)
            + crate::tools::json_bytes(&assistant_message.thought_signature);
        request.messages.push(assistant_message);
        request.messages.extend(venice_results);

        return Ok((
            TextGenerationResult::ToolCalls {
                text,
                calls,
                reasoning_bytes,
                continuation: ProviderState::new(Continuation { request, citations }),
            },
            false,
        ));
    }

    let reasoning = choice.message.reasoning_text();

    let Some(mut content) = choice.message.content else {
        return Err(anyhow::anyhow!(
            "No message content was returned from the Venice chat completion API"
        ));
    };

    if strip_thinking {
        content = strip_thinking_blocks(&content);
    }

    // Thinking alone is no answer, however it's shown.
    let answered = !strip_thinking_blocks(&content).trim().is_empty();

    let text = render_with_citations(content, &citations);
    let text = append_reasoning(text, reasoning, text_generation_config.show_reasoning);

    Ok((TextGenerationResult::text(text), answered))
}

/// The model ID suffix (e.g. `some-model:strip_thinking_response=true`) which turns stripping the thinking on or off.
/// It takes precedence over the `venice_parameters` field.
const STRIP_THINKING_SUFFIX: &str = "strip_thinking_response";

/// Whether the agent asks Venice to strip the model's thinking from responses (`strip_thinking_response`, as a model ID
/// suffix or in `venice_parameters`).
fn wants_thinking_stripped(text_generation_config: &TextGenerationConfig) -> bool {
    model_id_strips_thinking(&text_generation_config.model_id).unwrap_or_else(|| {
        text_generation_config
            .venice_parameters
            .as_ref()
            .is_some_and(|vp| vp.strip_thinking_response == Some(true))
    })
}

/// What a model ID suffix says about stripping the thinking, if anything.
fn model_id_strips_thinking(model_id: &str) -> Option<bool> {
    let (_, suffixes) = model_id.split_once(':')?;

    suffixes
        .split('&')
        .filter_map(|suffix| suffix.split_once('='))
        .find(|(name, _)| *name == STRIP_THINKING_SUFFIX)
        .and_then(|(_, value)| value.parse().ok())
}

/// Asks Venice to keep the thinking in responses (see `create_request`).
fn keep_thinking(request: &mut ChatCompletionRequest) {
    if let Some(vp) = request.venice_parameters.as_mut()
        && vp.strip_thinking_response == Some(true)
    {
        vp.strip_thinking_response = Some(false);
    }

    if model_id_strips_thinking(&request.model) == Some(true)
        && let Some((model, suffixes)) = request.model.split_once(':')
    {
        let suffixes: Vec<String> = suffixes
            .split('&')
            .map(|suffix| match suffix.split_once('=') {
                Some((STRIP_THINKING_SUFFIX, _)) => format!("{STRIP_THINKING_SUFFIX}=false"),
                _ => suffix.to_owned(),
            })
            .collect();

        request.model = format!("{model}:{}", suffixes.join("&"));
    }
}

/// Removes the thinking (a `<think>...</think>` block) from the start of a response's text, as Venice's
/// `strip_thinking_response` would. An unclosed block (cut off by the response limit) goes entirely.
pub(super) fn strip_thinking_blocks(text: &str) -> String {
    static THINK: OnceLock<Regex> = OnceLock::new();
    let think = THINK.get_or_init(|| {
        Regex::new(r"(?s)\A\s*<think>.*?(?:</think>|\z)").expect("thinking block regex is valid")
    });

    match think.find(text) {
        Some(found) => text[found.end()..].trim_start().to_owned(),
        None => text.to_owned(),
    }
}

/// Builds the request for the first round of a turn.
pub(super) fn create_request(
    text_generation_config: &TextGenerationConfig,
    conversation: LLMConversation,
    params: &TextGenerationParams,
) -> anyhow::Result<ChatCompletionRequest> {
    // Trimmed with a copy of the parameters, as the tools may be dropped (see `prepare_conversation`).
    let mut params = params.clone();
    let PreparedConversation {
        prompt_text,
        prompt_message,
        messages: mut conversation_messages,
    } = prepare_conversation(
        &mut params,
        text_generation_config.prompt.clone(),
        conversation.messages,
        TokenEstimate::Approximate,
        text_generation_config.max_response_tokens.unwrap_or(0),
        text_generation_config.max_context_tokens,
    );
    let params = &params;

    let prompt_cache_key = prompt_cache_key(&prompt_text, params);

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

    let mut request = ChatCompletionRequest {
        model: text_generation_config.model_id.clone(),
        messages,
        temperature: Some(temperature),
        // Web search rides entirely inside `venice_parameters`; `tools` only has the bot's tools.
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
        tools: tools(&params.tools),
    };

    // `strip_thinking_response` also strips `reasoning_content`, which models may need to get back to continue after
    // tool calls (Venice: "pass them back verbatim ... especially in tool loops"). So when tools are offered, the
    // thinking is kept in the transcript and stripped from the answer by the bot instead (see `read_response`).
    if !request.tools.is_empty() {
        keep_thinking(&mut request);
    }

    Ok(request)
}

/// Prompt-cache routing key. Hash ONLY conversation-stable inputs: the rendered system prompt
/// and the conversation start time. Folding in anything per-turn (message content, the current
/// time, the message count) would mint a fresh key every turn, miss the cache every lookup, and
/// pay full price plus the hashing cost. The start time is rendered explicitly here so the key
/// stays stable even when the user's prompt template never mentions the time variable; an
/// unknown start time renders "unknown" and simply keys on the prompt alone.
fn prompt_cache_key(prompt_text: &str, params: &TextGenerationParams) -> String {
    let conversation_start_time = params
        .prompt_variables
        .format("{{ baibot_conversation_start_time_utc }}");

    derive_prompt_cache_key(prompt_text, &conversation_start_time)
}

/// Builds a first-round request again without tools (also leaving out their instructions from the system prompt,
/// the context kept free for them, and what was turned on for them, like keeping the thinking).
pub(super) fn without_tools(
    request: &mut ChatCompletionRequest,
    params: &mut TextGenerationParams,
    text_generation_config: &TextGenerationConfig,
    conversation: LLMConversation,
) -> anyhow::Result<()> {
    params.tools.clear();
    *request = create_request(text_generation_config, conversation, params)?;
    Ok(())
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
///
/// The thinking renders as a Matrix-native collapsible `<details>` block: folded by default, one
/// click to expand, so it stays out of the way of the answer instead of dumping a wall of reasoning
/// inline. This survives the send path: the reply goes through markdown (`send_text_markdown`),
/// whose pulldown-cmark pass writes raw HTML verbatim rather than escaping it, and ruma's HTML
/// sanitizer allow-lists `<details>`/`<summary>`. Clients that do not render `<details>` degrade to
/// showing the summary and reasoning inline, so nothing is lost there either.
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
            // The blank lines around the trimmed reasoning keep it a separate markdown block from
            // the surrounding `<details>`/`</details>` HTML blocks, so the reasoning itself still
            // renders as markdown (lists, code, emphasis) inside the collapsible.
            let reasoning = reasoning.trim();
            format!(
                "{text}\n\n<details><summary>💭 Reasoning</summary>\n\n{reasoning}\n\n</details>"
            )
        }
        _ => text,
    }
}

/// Rewrites Venice's inline `^n^` citation superscripts into readable `[n]` references and appends
/// a `Sources:` list of markdown links, one per citation in order. Returns the content unchanged
/// when web search returned no citations, so non-search replies are never touched.
pub(super) fn render_with_citations(content: String, citations: &[WebSearchCitation]) -> String {
    if citations.is_empty() {
        return content;
    }

    let mut text = rewrite_citation_superscripts(&content);
    text.push_str(&render_sources_list(
        citations
            .iter()
            .map(|citation| (citation.title.as_str(), citation.url.as_str())),
    ));
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
