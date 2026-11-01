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

use super::config::Config;
use super::wire::{API_VERSION, MessagesRequest, MessagesResponse};

/// How many times a rate-limited (429) or overloaded (529) request is retried.
const MAX_RETRIES: u32 = 3;

/// The longest we wait before a retry, even if `retry-after` asks for more.
const MAX_RETRY_DELAY: std::time::Duration = std::time::Duration::from_secs(60);

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
                .header("x-api-key", &self.config.api_key)
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
        params: TextGenerationParams,
    ) -> anyhow::Result<TextGenerationResult> {
        let Some(text_generation_config) = &self.config.text_generation else {
            return Err(anyhow::anyhow!(
                strings::agent::no_configuration_for_purpose_so_cannot_be_used(
                    &AgentPurpose::TextGeneration
                ),
            ));
        };

        let prompt_text = params.prompt_variables.format(
            params
                .prompt_override
                .unwrap_or(self.text_generation_prompt().unwrap_or("".to_owned()))
                .trim(),
        );

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

        // Avoid the situation where multiple user or assistant messages are sent consecutively,
        // to avoid errors like:
        // > API error: Error response: error Api error: invalid_request_error messages: roles must alternate between "user" and "assistant", but found multiple "user" roles in a row
        // as reported here: https://github.com/etkecc/baibot/issues/13
        //
        // As https://docs.anthropic.com/en/api/messages says:
        // > Our models are trained to operate on alternating user and assistant conversational turns.
        let conversation = conversation.combine_consecutive_messages();

        let mut conversation_messages = conversation.messages;

        if params.context_management_enabled {
            tracing::trace!("Shortening messages list to context size");

            conversation_messages = shorten_messages_list_to_context_size(
                TokenEstimate::Approximate,
                &prompt_message,
                conversation_messages,
                Some(text_generation_config.max_response_tokens),
                text_generation_config.max_context_tokens,
            );

            tracing::trace!("Finished shortening messages list to context size");
        };

        let messages_count = conversation_messages.len();

        let temperature = params
            .temperature_override
            .unwrap_or(text_generation_config.temperature);

        let system = prompt_message.and_then(|message| match message.content {
            LLMMessageContent::Text(text) => Some(text),
            _ => None,
        });

        let request = MessagesRequest {
            model: text_generation_config.model_id.clone(),
            max_tokens: text_generation_config.max_response_tokens,
            messages: super::utils::create_anthropic_messages(conversation_messages),
            system,
            temperature: Some(temperature),
        };

        if let Ok(request_as_json) = serde_json::to_string(&request) {
            tracing::trace!(
                model = format!("{:?}", request.model),
                ?messages_count,
                request = request_as_json,
                "Sending Anthropic create message API request"
            );
        }

        let response = self.create_message(&request).await?;

        tracing::trace!(?response, "Got response from Anthropic create message API");

        Ok(TextGenerationResult {
            text: super::utils::response_text(response)?,
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

    fn text_generation_temperature(&self) -> Option<f32> {
        self.config
            .text_generation
            .as_ref()
            .map(|config| config.temperature)
    }

    fn text_to_speech_voice(&self) -> Option<String> {
        None
    }

    fn text_to_speech_speed(&self) -> Option<f32> {
        None
    }
}
