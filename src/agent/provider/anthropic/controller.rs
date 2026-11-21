use std::fmt::Debug;
use std::sync::Arc;

use super::super::ControllerTrait;
use crate::agent::AgentPurpose;
use crate::agent::provider::entity::{
    ImageEditResult, ImageGenerationResult, ImageSource, PingResult, TextGenerationParams,
    TextGenerationResult, TextToSpeechParams, TextToSpeechResult,
};
use crate::agent::provider::{
    ImageEditParams, ImageGenerationParams, SpeechToTextParams, SpeechToTextResult,
};
use crate::conversation::llm::{
    Author as LLMAuthor, Conversation as LLMConversation, Message as LLMMessage,
    MessageContent as LLMMessageContent, TokenEstimate, shorten_messages_list_to_context_size,
};
use crate::strings;
use crate::tools::ProviderState;

use super::config::{Config, TextGenerationConfig};
use super::wire::{
    API_VERSION, ContentBlock, Message, MessagesRequest, MessagesResponse, Role, Tool,
};

/// How many times a rate-limited (429) or overloaded (529) request is retried.
const MAX_RETRIES: u32 = 3;

/// The longest we wait before a retry, even if `retry-after` asks for more.
const MAX_RETRY_DELAY: std::time::Duration = std::time::Duration::from_secs(60);

/// How many times a paused turn (`pause_turn`) is continued before replying with what we have.
const MAX_PAUSE_TURN_CONTINUATIONS: usize = 4;

/// How many times each server tool may be used per request, which keeps the cost of a single reply
/// bounded (a paused turn continues in a new request, with its own allowance).
const SERVER_TOOL_MAX_USES: u32 = 5;

struct ControllerInner {
    http: reqwest::Client,
}

#[derive(Clone)]
pub struct Controller {
    config: Config,
    inner: Arc<ControllerInner>,
}

impl Debug for Controller {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Controller")
            .field("config", &self.config)
            .finish()
    }
}

impl Controller {
    pub fn new(config: Config) -> anyhow::Result<Self> {
        if !config.base_url.ends_with("/v1") {
            return Err(anyhow::anyhow!("base_url must end with '/v1'"));
        }

        // Responses with thinking can take a while, so use a generous timeout instead of
        // reqwest's default (none).
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(600))
            .build()?;

        Ok(Self {
            config,
            inner: Arc::new(ControllerInner { http }),
        })
    }

    async fn create_message(&self, request: &MessagesRequest) -> anyhow::Result<MessagesResponse> {
        let url = format!("{}/messages", self.config.base_url);

        let mut attempt = 0;
        loop {
            let response = self
                .inner
                .http
                .post(&url)
                .bearer_auth(&self.config.api_key)
                .header("anthropic-version", API_VERSION)
                .json(request)
                .send()
                .await?;

            let status = response.status();
            if status.is_success() {
                return Ok(response.json().await?);
            }

            let retry_after = response
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.parse::<u64>().ok())
                .map(std::time::Duration::from_secs);

            let body = response.text().await.unwrap_or_default();
            tracing::warn!(%status, body, "Anthropic create message request failed");

            let retryable =
                status == reqwest::StatusCode::TOO_MANY_REQUESTS || status.as_u16() == 529;
            if retryable && attempt < MAX_RETRIES {
                attempt += 1;

                let delay = retry_after
                    .unwrap_or(std::time::Duration::from_secs(2u64.pow(attempt)))
                    .min(MAX_RETRY_DELAY);

                tracing::info!(?delay, attempt, "Retrying Anthropic create message request");
                tokio::time::sleep(delay).await;
                continue;
            }

            return Err(anyhow::anyhow!(super::utils::error_message(status, &body)));
        }
    }
}

impl ControllerTrait for Controller {
    async fn ping(&self) -> anyhow::Result<PingResult> {
        if !self.supports_purpose(AgentPurpose::TextGeneration) {
            return Ok(PingResult::Inconclusive);
        }

        let messages = vec![LLMMessage {
            author: LLMAuthor::User,
            sender_id: None,
            content: LLMMessageContent::Text("Hello!".to_string()),
            timestamp: chrono::Utc::now(),
        }];

        let conversation = LLMConversation { messages };

        self.generate_text(conversation, TextGenerationParams::default())
            .await?;

        Ok(PingResult::Successful)
    }

    async fn generate_text(
        &self,
        conversation: LLMConversation,
        mut params: TextGenerationParams,
    ) -> anyhow::Result<TextGenerationResult> {
        let Some(text_generation_config) = &self.config.text_generation else {
            return Err(anyhow::anyhow!(
                strings::agent::no_configuration_for_purpose_so_cannot_be_used(
                    &AgentPurpose::TextGeneration
                ),
            ));
        };

        if !text_generation_config.bot_tools {
            params.tools.clear();
        }

        let mut request = match params.continuation.take() {
            // The request so far already has everything else (system prompt, tools, the conversation and
            // the model's tool calls), so only the results are added.
            Some(continuation) => {
                let mut request: MessagesRequest = continuation.state.into_inner()?;
                request
                    .messages
                    .push(super::utils::tool_results_message(continuation.results));

                tracing::trace!(
                    model = format!("{:?}", request.model),
                    results_count = request.messages.last().map_or(0, |m| m.content.len()),
                    "Continuing an Anthropic turn with tool results"
                );

                request
            }
            None => create_request(text_generation_config, conversation, &params),
        };

        let mut content = vec![];
        let mut stop_reason;
        let mut continuation = 0;
        loop {
            let response = self.create_message(&request).await?;

            tracing::trace!(?response, "Got response from Anthropic create message API");

            content.extend(response.content);
            stop_reason = response.stop_reason;

            // Server tools (e.g. web search) may run in a container, which the rest of the turn reuses.
            if let Some(container) = response.container {
                request.container = Some(container.id);
            }

            if stop_reason.as_deref() != Some("pause_turn")
                || continuation == MAX_PAUSE_TURN_CONTINUATIONS
            {
                break;
            }
            continuation += 1;

            // Server tools (e.g. many web searches) hit the API's per-request limit. Sending the
            // partial turn back as-is lets the model continue it.
            if continuation > 1 {
                request.messages.pop();
            }
            request.messages.push(assistant_message(&content));

            tracing::debug!(continuation, "Continuing a paused Anthropic turn");
        }

        let calls = if stop_reason.as_deref() == Some("tool_use") && !params.tools.is_empty() {
            super::utils::tool_calls(&content)
        } else {
            vec![]
        };

        if calls.is_empty() {
            return Ok(TextGenerationResult::text(super::utils::response_text(
                &content,
                stop_reason.as_deref(),
            )?));
        }

        // The whole turn (replacing the partial one sent back when it paused) is sent back along with the results.
        if continuation > 0 {
            request.messages.pop();
        }
        request.messages.push(assistant_message(&content));

        Ok(TextGenerationResult::ToolCalls {
            text: super::utils::response_text(&content, stop_reason.as_deref()).ok(),
            calls,
            continuation: ProviderState::new(request),
        })
    }

    async fn speech_to_text(
        &self,
        _mime_type: &mxlink::mime::Mime,
        _media: Vec<u8>,
        _params: SpeechToTextParams,
    ) -> anyhow::Result<SpeechToTextResult> {
        Err(anyhow::anyhow!("Speech-to-Text not supported"))
    }

    async fn generate_image(
        &self,
        _prompt: &str,
        _params: ImageGenerationParams,
    ) -> anyhow::Result<ImageGenerationResult> {
        Err(anyhow::anyhow!("Image generation not supported"))
    }

    async fn create_image_edit(
        &self,
        _prompt: &str,
        _images: Vec<ImageSource>,
        _params: ImageEditParams,
    ) -> anyhow::Result<ImageEditResult> {
        Err(anyhow::anyhow!("Image editing is not supported"))
    }

    async fn text_to_speech(
        &self,
        _input: &str,
        _params: TextToSpeechParams,
    ) -> anyhow::Result<TextToSpeechResult> {
        Err(anyhow::anyhow!("Speech generation not supported"))
    }

    fn supports_purpose(&self, purpose: AgentPurpose) -> bool {
        match purpose {
            AgentPurpose::TextGeneration => self.config.text_generation.is_some(),
            AgentPurpose::SpeechToText => false,
            AgentPurpose::TextToSpeech => false,
            AgentPurpose::ImageGeneration => false,
            AgentPurpose::CatchAll => true,
        }
    }

    fn text_generation_model_id(&self) -> Option<String> {
        self.config
            .text_generation
            .as_ref()
            .map(|config| config.model_id.to_owned())
    }

    fn text_generation_prompt(&self) -> Option<String> {
        self.config
            .text_generation
            .as_ref()
            .and_then(|config| config.prompt.clone())
    }

    fn text_generation_max_context_tokens(&self) -> Option<u32> {
        self.config
            .text_generation
            .as_ref()
            .map(|config| config.max_context_tokens)
    }

    fn text_generation_temperature(&self) -> Option<f32> {
        None
    }

    fn text_to_speech_voice(&self) -> Option<String> {
        None
    }

    fn text_to_speech_speed(&self) -> Option<f32> {
        None
    }
}

/// Builds the request for the first round of a turn.
fn create_request(
    config: &TextGenerationConfig,
    conversation: LLMConversation,
    params: &TextGenerationParams,
) -> MessagesRequest {
    let prompt_text = params.system_prompt(config.prompt.clone());

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
        tracing::trace!("Shortening messages list to context size");

        conversation_messages = shorten_messages_list_to_context_size(
            TokenEstimate::Approximate,
            &prompt_message,
            conversation_messages,
            Some(config.max_response_tokens + params.reserved_context_tokens),
            config.max_context_tokens,
        );

        tracing::trace!("Finished shortening messages list to context size");
    };

    let messages_count = conversation_messages.len();

    // The Messages API no longer accepts a temperature, so a room's override can't apply.
    if params.temperature_override.is_some() {
        tracing::debug!("Ignoring the temperature override, which Anthropic does not support");
    }

    let system = prompt_message.and_then(|message| match message.content {
        LLMMessageContent::Text(text) => Some(text),
        _ => None,
    });

    // The `_20260209` versions filter results with code execution before they reach the
    // context. They need Claude Opus 4.6 / Sonnet 4.6 or later.
    let mut tools = vec![];
    if config.tools.web_search {
        tools.push(Tool::Server {
            kind: "web_search_20260209",
            name: "web_search",
            max_uses: SERVER_TOOL_MAX_USES,
        });
    }
    if config.tools.web_fetch {
        tools.push(Tool::Server {
            kind: "web_fetch_20260209",
            name: "web_fetch",
            max_uses: SERVER_TOOL_MAX_USES,
        });
    }
    tools.extend(super::utils::client_tools(&params.tools));

    let request = MessagesRequest {
        model: config.model_id.clone(),
        max_tokens: config.max_response_tokens,
        messages: super::utils::create_anthropic_messages(conversation_messages),
        system,
        tools,
        container: None,
    };

    if let Ok(request_as_json) = serde_json::to_string(&request) {
        tracing::trace!(
            model = format!("{:?}", request.model),
            ?messages_count,
            request = request_as_json,
            "Sending Anthropic create message API request"
        );
    }

    request
}

/// The model's (partial) turn, sent back as-is.
fn assistant_message(content: &[serde_json::Value]) -> Message {
    Message {
        role: Role::Assistant,
        content: content.iter().cloned().map(ContentBlock::Raw).collect(),
    }
}
