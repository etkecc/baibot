use std::fmt::Debug;
use std::str::FromStr;
use std::sync::Arc;

use anthropic_rs::completion::message::ContentType;
use anthropic_rs::{
    client::Client as AnthropicClient, config::Config as AnthropicConfig,
    models::claude::ClaudeModel,
};

use super::super::ControllerTrait;
use crate::agent::provider::entity::{
    ImageGenerationResult, PingResult, TextGenerationParams, TextGenerationResult,
    TextToSpeechParams, TextToSpeechResult,
};
use crate::agent::provider::{ImageGenerationParams, SpeechToTextParams, SpeechToTextResult};
use crate::agent::AgentPurpose;
use crate::conversation::llm::{
    shorten_messages_list_to_context_size, Author as LLMAuthor, Conversation as LLMConversation,
    Message as LLMMessage,
};
use crate::strings;

use super::config::Config;

struct ControllerInner {
    client: AnthropicClient,
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
        let anthropic_config =
            AnthropicConfig::new(config.api_key.clone()).with_base_url(config.base_url.clone());

        let client = match AnthropicClient::new(anthropic_config) {
            Ok(client) => client,
            Err(err) => {
                return Err(anyhow::anyhow!(
                    "Failed to create Anthropic client: {}",
                    err.to_string()
                ));
            }
        };

        Ok(Self {
            config,
            inner: Arc::new(ControllerInner { client }),
        })
    }
}

impl ControllerTrait for Controller {
    async fn ping(&self) -> anyhow::Result<PingResult> {
        if !self.supports_purpose(AgentPurpose::TextGeneration) {
            return Ok(PingResult::Inconclusive);
        }

        let messages = vec![LLMMessage {
            author: LLMAuthor::User,
            message_text: "Hello!".to_string(),
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

        let prompt_text = params
            .prompt_override
            .unwrap_or(self.text_generation_prompt().unwrap_or("".to_owned()))
            .trim()
            .to_owned();

        let prompt_message = if prompt_text.is_empty() {
            None
        } else {
            Some(LLMMessage {
                author: LLMAuthor::Prompt,
                message_text: prompt_text,
            })
        };

        let mut conversation_messages = conversation.messages;

        if params.context_management_enabled {
            tracing::trace!("Shortening messages list to context size");

            conversation_messages = shorten_messages_list_to_context_size(
                &text_generation_config.model_id,
                &prompt_message,
                conversation_messages,
                text_generation_config.max_response_tokens,
                text_generation_config.max_context_tokens,
            );

            tracing::trace!("Finished shortening messages list to context size");
        };

        let messages_count = conversation_messages.len();

        let mut request = super::utils::create_anthropic_message_request(conversation_messages);

        let model = match ClaudeModel::from_str(&text_generation_config.model_id) {
            Ok(model) => model,
            Err(err) => {
                tracing::debug!(?err, "Failed to parse model ID");

                return Err(anyhow::anyhow!(
                    "Failed to parse model ID: {}",
                    &text_generation_config.model_id
                ));
            }
        };

        let temperature = params
            .temperature_override
            .unwrap_or(text_generation_config.temperature);

        if let Some(prompt_message) = prompt_message {
            request.system = Some(prompt_message.message_text);
        }

        request.model = model;
        request.temperature = Some(temperature);
        request.max_tokens = text_generation_config.max_response_tokens;

        if let Ok(request_as_json) = serde_json::to_string(&request) {
            tracing::trace!(
                model = format!("{:?}", request.model),
                ?messages_count,
                request = request_as_json,
                "Sending Anthropic create message API request"
            );
        }

        let response = self.inner.client.create_message(request).await?;

        tracing::trace!(?response, "Got response from Anthropic create message API");

        // response.content usually contains a single element, but we support handling multiple to account for all possibilities
        let mut text_parts = vec![];
        for content in response.content {
            let content_type = content.content_type;

            match content_type {
                ContentType::Text => {
                    text_parts.push(content.text);
                } // There are no other content types to handle yet, but there may be in the future
            }
        }

        if text_parts.is_empty() {
            return Err(anyhow::anyhow!(
                "No text content in response from the Anthropic create message API"
            ));
        }

        Ok(TextGenerationResult {
            text: text_parts.join("\n\n"),
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

    fn text_generation_prompt(&self) -> Option<String> {
        let Some(text_generation_config) = &self.config.text_generation else {
            return None;
        };

        text_generation_config.prompt.clone()
    }

    fn text_generation_temperature(&self) -> Option<f32> {
        let Some(text_generation_config) = &self.config.text_generation else {
            return None;
        };

        Some(text_generation_config.temperature)
    }

    fn text_to_speech_voice(&self) -> Option<String> {
        None
    }

    fn text_to_speech_speed(&self) -> Option<f32> {
        None
    }
}
