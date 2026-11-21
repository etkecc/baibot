use crate::agent::provider::entity::{PreparedConversation, prepare_conversation};
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
        MessageContent as LLMMessageContent, TokenEstimate,
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
use super::config::TextGenerationConfig;
use super::utils::{error_from_body, error_from_response};
use super::wire::{
    ChatCompletionRequest, ChatCompletionResponse, ImageGenerationRequest, ImageGenerationResponse,
    SpeechRequest, TranscriptionResponse,
};
use crate::tools::ProviderState;

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
        mut params: TextGenerationParams,
    ) -> anyhow::Result<TextGenerationResult> {
        let Some(text_generation_config) = &self.config.text_generation else {
            return Err(anyhow::anyhow!(
                strings::agent::no_configuration_for_purpose_so_cannot_be_used(
                    &AgentPurpose::TextGeneration
                ),
            ));
        };

        let base_url = &self.config.base_url;
        let model_id = &text_generation_config.model_id;

        if !text_generation_config.bot_tools
            || super::utils::model_rejects_tools(base_url, model_id)
        {
            params.tools.clear();
        }

        let first_round = params.continuation.is_none();

        // Kept for building the request again without tools, if the model rejects them.
        let mut retry_conversation =
            (first_round && !params.tools.is_empty()).then(|| conversation.clone());

        let mut request = match params.continuation.take() {
            // The request so far already has everything else (system prompt, tools, the conversation and
            // the model's tool calls), so only the results are added.
            Some(continuation) => {
                let mut request: ChatCompletionRequest = continuation.state.into_inner()?;
                request
                    .messages
                    .extend(super::utils::tool_result_messages(continuation.results));

                tracing::trace!(
                    model = format!("{:?}", request.model),
                    "Continuing an OpenAI-compat chat completion with tool results"
                );

                request
            }
            None => create_request(text_generation_config, conversation, &params),
        };

        let mut response = self.post("chat/completions").json(&request).send().await?;

        // Many models behind OpenAI-compatible APIs don't support tools, and some providers reject requests
        // offering them. Such models get none from then on.
        if !response.status().is_success() && first_round && !request.tools.is_empty() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();

            if !super::utils::is_tools_rejection(status, &body) {
                return Err(error_from_body("chat completion", status, &body));
            }

            tracing::warn!(
                %status,
                body,
                model_id,
                "The model rejected a request with tools - retrying without them"
            );

            without_tools(
                &mut request,
                &mut params,
                text_generation_config,
                retry_conversation.take().unwrap_or_default(),
            );
            response = self.post("chat/completions").json(&request).send().await?;

            // Only when it works without them, it's really about the tools.
            if response.status().is_success() {
                tracing::warn!(model_id, "Not offering tools to this model anymore");
                super::utils::remember_model_rejects_tools(base_url, model_id);
            }
        }

        if !response.status().is_success() {
            return Err(error_from_response("chat completion", response).await);
        }
        let response: ChatCompletionResponse = response.json().await?;

        tracing::trace!(
            ?response,
            "Got response from the OpenAI-compat chat completion API"
        );

        // We only request 1 result, so there should only be 1 choice.
        let Some(choice) = response.choices.into_iter().next() else {
            return Err(anyhow::anyhow!(
                "No response messages choices were returned from the OpenAI-compat chat completion API"
            ));
        };

        let Some(message) = choice.message else {
            return Err(anyhow::anyhow!(
                "No response message in choice was returned from the OpenAI-compat chat completion API"
            ));
        };

        // Providers differ in the finish reason they give for tool calls (e.g. `tool_calls` or `stop`), so it isn't relied on.
        let has_tool_calls = message
            .tool_calls
            .as_ref()
            .is_some_and(|calls| !calls.is_empty());

        if has_tool_calls && !request.tools.is_empty() {
            // A call cut off by the response limit can't run, and the text before it isn't an answer.
            if choice.finish_reason.as_deref() == Some("length") {
                return Err(anyhow::anyhow!(
                    "The model used up all response tokens (`max_response_tokens`) while calling a tool"
                ));
            }

            let text = message.text().filter(|text| !text.trim().is_empty());

            let (calls, assistant_message) = super::utils::tool_calls(message);
            // Besides the text, the content may carry thinking (e.g. Mistral's chunks), which goes back too.
            let reasoning_bytes = crate::tools::json_bytes(&assistant_message.content)
                .saturating_sub(text.as_deref().map_or(0, str::len))
                + crate::tools::json_bytes(&assistant_message.reasoning_content)
                + crate::tools::json_bytes(&assistant_message.reasoning_details);
            request.messages.push(assistant_message);

            return Ok(TextGenerationResult::ToolCalls {
                text,
                calls,
                reasoning_bytes,
                continuation: ProviderState::new(request),
            });
        }

        let Some(text) = message.text() else {
            return Err(anyhow::anyhow!(
                "No text content in the response message from the OpenAI-compat chat completion API"
            ));
        };

        Ok(TextGenerationResult::text(text))
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

    fn text_generation_max_context_tokens(&self) -> Option<u32> {
        self.config
            .text_generation
            .as_ref()
            .map(|config| config.max_context_tokens)
    }

    fn text_generation_bot_tools_enabled(&self) -> bool {
        self.config
            .text_generation
            .as_ref()
            .is_some_and(|config| config.bot_tools)
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

/// Builds the request for the first round of a turn.
fn create_request(
    text_generation_config: &TextGenerationConfig,
    conversation: LLMConversation,
    params: &TextGenerationParams,
) -> ChatCompletionRequest {
    // Trimmed with a copy of the parameters, as the tools may be dropped (see `prepare_conversation`).
    let mut params = params.clone();
    let PreparedConversation {
        prompt_text: _,
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
        tools: super::utils::tools(&params.tools),
    };

    if let Ok(request_as_json) = serde_json::to_string(&request) {
        tracing::trace!(
            model = format!("{:?}", request.model),
            ?messages_count,
            request = request_as_json,
            "Sending OpenAI-compat chat completion API request"
        );
    }

    request
}

/// Builds a first-round request again without tools (also leaving out their instructions from the system prompt,
/// and the context kept free for them).
fn without_tools(
    request: &mut ChatCompletionRequest,
    params: &mut TextGenerationParams,
    text_generation_config: &TextGenerationConfig,
    conversation: LLMConversation,
) {
    params.tools.clear();
    *request = create_request(text_generation_config, conversation, params);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::ToolDefinition;

    fn params(tool_instructions: &str) -> TextGenerationParams {
        TextGenerationParams {
            tools: vec![ToolDefinition {
                name: "room_history".to_owned(),
                description: "Reads the room's recent messages.".to_owned(),
                parameters: serde_json::json!({"type": "object", "properties": {}}),
            }],
            tool_instructions: Some(tool_instructions.to_owned()),
            ..Default::default()
        }
    }

    fn conversation() -> LLMConversation {
        LLMConversation {
            messages: vec![LLMMessage {
                author: LLMAuthor::User,
                sender_id: None,
                content: LLMMessageContent::Text("Hi".to_owned()),
                timestamp: chrono::Utc::now(),
            }],
        }
    }

    #[test]
    fn without_tools_the_tool_instructions_go_too() {
        let config = TextGenerationConfig {
            prompt: Some("Be brief.".to_owned()),
            ..Default::default()
        };
        let mut params = params("Use the tools.");

        let mut request = create_request(&config, conversation(), &params);
        assert_eq!(request.tools.len(), 1);
        assert_eq!(
            request.messages[0].content,
            Some(serde_json::json!("Be brief.\n\nUse the tools."))
        );

        without_tools(&mut request, &mut params, &config, conversation());

        assert!(request.tools.is_empty());
        assert!(params.tools.is_empty());
        assert_eq!(
            request.messages[0].content,
            Some(serde_json::json!("Be brief."))
        );
        assert_eq!(request.messages.len(), 2);
    }

    #[test]
    fn without_tools_a_system_prompt_of_only_tool_instructions_goes() {
        let config = TextGenerationConfig {
            prompt: None,
            ..Default::default()
        };
        let mut params = params("Use the tools.");

        let mut request = create_request(&config, conversation(), &params);
        assert_eq!(request.messages[0].role, "system");

        without_tools(&mut request, &mut params, &config, conversation());

        assert_eq!(request.messages.len(), 1);
        assert_eq!(request.messages[0].role, "user");
    }

    #[test]
    fn without_tools_the_context_kept_for_them_goes_too() {
        let config = TextGenerationConfig {
            prompt: None,
            max_response_tokens: Some(100),
            max_context_tokens: 1000,
            ..Default::default()
        };

        // Two messages of about 200 tokens each: only one fits next to the reservation.
        let message = |text: &str| LLMMessage {
            author: LLMAuthor::User,
            sender_id: None,
            content: LLMMessageContent::Text(text.repeat(800)),
            timestamp: chrono::Utc::now(),
        };
        let conversation = || LLMConversation {
            messages: vec![message("a"), message("b")],
        };

        let mut params = TextGenerationParams {
            context_management_enabled: true,
            reserved_context_tokens: 600,
            ..params("Use the tools.")
        };

        let user_messages = |request: &ChatCompletionRequest| {
            request
                .messages
                .iter()
                .filter(|message| message.role == "user")
                .count()
        };

        let mut request = create_request(&config, conversation(), &params);
        assert_eq!(request.tools.len(), 1);
        assert_eq!(user_messages(&request), 1);

        without_tools(&mut request, &mut params, &config, conversation());
        assert!(request.tools.is_empty());
        assert_eq!(user_messages(&request), 2);
    }
}
