const SMALLEST_IMAGE_SIZE: &str = "256x256";

use super::super::ControllerTrait;
use crate::utils::base64::base64_decode;
use crate::{
    agent::provider::{
        ImageEditParams, ImageGenerationParams, ImageSource, SpeechToTextParams,
        SpeechToTextResult,
        entity::{TextGenerationParams, TextGenerationResult},
    },
    conversation::llm::{
        Author as LLMAuthor, Conversation as LLMConversation, Message as LLMMessage,
        MessageContent as LLMMessageContent, TokenEstimate, shorten_messages_list_to_context_size,
    },
};
use crate::{
    agent::{
        AgentPurpose,
        provider::entity::{
            ImageEditResult, ImageGenerationResult, PingResult, TextToSpeechParams,
            TextToSpeechResult,
        },
    },
    strings,
};

use super::Config;
use super::utils::error_from_response;
use super::wire::{
    ChatCompletionRequest, ChatCompletionResponse, ImageGenerationRequest, ImageGenerationResponse,
    SpeechRequest, TranscriptionResponse,
};

#[derive(Debug, Clone)]
pub struct Controller {
    config: Config,
    http: reqwest::Client,
}

impl Controller {
    pub fn new(config: Config) -> Self {
        // Local models (e.g. LocalAI, Ollama) and image generation can be slow, so use a generous
        // timeout instead of reqwest's default (none). `build` only fails on TLS/system init.
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(600))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self { config, http }
    }

    fn url(&self, path: &str) -> String {
        format!("{}/{path}", self.config.base_url.trim_end_matches('/'))
    }

    fn post(&self, path: &str) -> reqwest::RequestBuilder {
        let request = self.http.post(self.url(path));

        // Some providers (e.g. a local Ollama) need no API key.
        match &self.config.api_key {
            Some(api_key) if !api_key.is_empty() => request.bearer_auth(api_key),
            _ => request,
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

        let mut conversation_messages = conversation.messages;

        if params.context_management_enabled {
            tracing::trace!("Shortening messages list to context size");

            conversation_messages = shorten_messages_list_to_context_size(
                TokenEstimate::Approximate,
                &prompt_message,
                conversation_messages,
                text_generation_config.max_response_tokens,
                text_generation_config.max_context_tokens,
            );

            tracing::trace!("Finished shortening messages list to context size");
        };

        if let Some(prompt_message) = prompt_message {
            conversation_messages.insert(0, prompt_message);
        }

        let openai_conversation_messages =
            super::utils::convert_llm_messages_to_openai_messages(conversation_messages);

        let messages_count = openai_conversation_messages.len();

        let temperature = params
            .temperature_override
            .unwrap_or(text_generation_config.temperature);

        let request = ChatCompletionRequest {
            model: text_generation_config.model_id.clone(),
            messages: openai_conversation_messages,
            temperature: Some(temperature),
            max_tokens: text_generation_config.max_response_tokens,
        };

        if let Ok(request_as_json) = serde_json::to_string(&request) {
            tracing::trace!(
                model = format!("{:?}", request.model),
                ?messages_count,
                request = request_as_json,
                "Sending OpenAI-compat chat completion API request"
            );
        }

        let response = self.post("chat/completions").json(&request).send().await?;
        if !response.status().is_success() {
            return Err(error_from_response("chat completion", response).await);
        }
        let response: ChatCompletionResponse = response.json().await?;

        tracing::trace!(
            ?response,
            "Got response from the OpenAI-compat chat completion API"
        );

        // We only request 1 result, so there should only be 1 choice.
        if let Some(choice) = response.choices.into_iter().next() {
            let Some(message) = choice.message else {
                return Err(anyhow::anyhow!(
                    "No response message in choice was returned from the OpenAI-compat chat completion API"
                ));
            };

            let Some(text) = message.content else {
                return Err(anyhow::anyhow!(
                    "No text content in the response message from the OpenAI-compat chat completion API"
                ));
            };

            return Ok(TextGenerationResult { text });
        }

        Err(anyhow::anyhow!(
            "No response messages choices were returned from the OpenAI-compat chat completion API"
        ))
    }

    async fn speech_to_text(
        &self,
        mime_type: &mxlink::mime::Mime,
        media: Vec<u8>,
        params: SpeechToTextParams,
    ) -> anyhow::Result<SpeechToTextResult> {
        let Some(speech_to_text_config) = &self.config.speech_to_text else {
            return Err(anyhow::anyhow!(
                strings::agent::no_configuration_for_purpose_so_cannot_be_used(
                    &AgentPurpose::SpeechToText
                ),
            ));
        };

        let part = reqwest::multipart::Part::bytes(media)
            .file_name(super::utils::audio_file_name(mime_type))
            .mime_str(mime_type.as_ref())?;

        let mut form = reqwest::multipart::Form::new()
            .part("file", part)
            .text("model", speech_to_text_config.model_id.clone());

        let language = params.language_override.clone();
        if let Some(language) = &language {
            form = form.text("language", language.clone());
        }

        tracing::trace!(
            model_id = speech_to_text_config.model_id,
            ?language,
            "Sending OpenAI-compat speech-to-text API request"
        );

        let response = self
            .post("audio/transcriptions")
            .multipart(form)
            .send()
            .await?;
        if !response.status().is_success() {
            return Err(error_from_response("audio transcription", response).await);
        }
        let response: TranscriptionResponse = response.json().await?;

        tracing::trace!(
            ?response,
            "Got response from the OpenAI-compat audio transcription API"
        );

        let Some(text) = response.text else {
            return Err(anyhow::anyhow!(
                "No response text was returned from the OpenAI-compat audio transcription API"
            ));
        };

        Ok(SpeechToTextResult { text })
    }

    async fn generate_image(
        &self,
        prompt: &str,
        params: ImageGenerationParams,
    ) -> anyhow::Result<ImageGenerationResult> {
        let Some(image_generation_config) = &self.config.image_generation else {
            return Err(anyhow::anyhow!(
                strings::agent::no_configuration_for_purpose_so_cannot_be_used(
                    &AgentPurpose::ImageGeneration
                ),
            ));
        };

        // It seems like some OpenAI-compatible providers (e.g. LocalAI with StableDiffusion) skip some requirements
        // when they span multiple lines.
        let prompt = prompt.replace("\n", " ");

        let size: Option<String> = if params.smallest_size_possible {
            Some(SMALLEST_IMAGE_SIZE.to_owned())
        } else {
            image_generation_config.size.clone()
        };

        let request = ImageGenerationRequest {
            model: image_generation_config.model_id.to_owned(),
            prompt: prompt.to_owned(),
            n: 1,
            size,
            style: image_generation_config.style.clone(),
            quality: image_generation_config.quality.clone(),
            response_format: "b64_json",
        };

        tracing::trace!(
            ?prompt,
            model = format!("{:?}", request.model),
            size = format!("{:?}", request.size),
            style = format!("{:?}", request.style),
            quality = format!("{:?}", request.quality),
            "Sending OpenAI-compat image generation API request"
        );

        let response = self
            .post("images/generations")
            .json(&request)
            .send()
            .await?;
        if !response.status().is_success() {
            return Err(error_from_response("image generation", response).await);
        }
        let response: ImageGenerationResponse = response.json().await?;

        if let Some(image) = response.data.into_iter().next() {
            let Some(b64_json) = &image.b64_json else {
                return Err(anyhow::anyhow!(
                    "The OpenAI-compat image generation API returned no b64_json image data"
                ));
            };

            let bytes = base64_decode(b64_json)?;

            return Ok(ImageGenerationResult {
                bytes,
                mime_type: mxlink::mime::IMAGE_PNG,
                revised_prompt: image.revised_prompt,
            });
        }

        Err(anyhow::anyhow!(
            "The OpenAI-compat image generation API returned no images"
        ))
    }

    async fn create_image_edit(
        &self,
        _prompt: &str,
        _images: Vec<ImageSource>,
        _params: ImageEditParams,
    ) -> anyhow::Result<ImageEditResult> {
        Err(anyhow::anyhow!(
            "The OpenAI image edit API is not supported by the OpenAI-compat provider"
        ))
    }

    async fn text_to_speech(
        &self,
        input: &str,
        params: TextToSpeechParams,
    ) -> anyhow::Result<TextToSpeechResult> {
        let Some(text_to_speech_config) = &self.config.text_to_speech else {
            return Err(anyhow::anyhow!(
                strings::agent::no_configuration_for_purpose_so_cannot_be_used(
                    &AgentPurpose::TextToSpeech
                ),
            ));
        };

        let request = SpeechRequest {
            model: text_to_speech_config.model_id.clone(),
            input: input.to_owned(),
            voice: params
                .voice_override
                .unwrap_or_else(|| text_to_speech_config.voice.clone()),
            speed: params.speed_override.unwrap_or(text_to_speech_config.speed),
            response_format: text_to_speech_config.response_format.clone(),
        };

        tracing::trace!(
            model = request.model,
            voice = request.voice,
            speed = request.speed,
            "Sending OpenAI-compat text-to-speech API request"
        );

        let response = self.post("audio/speech").json(&request).send().await?;
        if !response.status().is_success() {
            return Err(error_from_response("text-to-speech", response).await);
        }

        Ok(TextToSpeechResult {
            bytes: response.bytes().await?.to_vec(),
            mime_type: super::utils::speech_response_format_to_mime_type(&request.response_format),
        })
    }

    fn supports_purpose(&self, purpose: AgentPurpose) -> bool {
        match purpose {
            AgentPurpose::ImageGeneration => self.config.image_generation.is_some(),
            AgentPurpose::TextGeneration => self.config.text_generation.is_some(),
            AgentPurpose::SpeechToText => self.config.speech_to_text.is_some(),
            AgentPurpose::TextToSpeech => self.config.text_to_speech.is_some(),
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
        self.config
            .text_to_speech
            .as_ref()
            .map(|config| config.voice.clone())
    }

    fn text_to_speech_speed(&self) -> Option<f32> {
        let Some(text_to_speech_config) = &self.config.text_to_speech else {
            return None;
        };

        Some(text_to_speech_config.speed)
    }
}
