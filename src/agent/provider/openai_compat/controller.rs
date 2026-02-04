use etke_openai_api_rust::audio::{AudioApi, AudioBody};
use etke_openai_api_rust::chat::{ChatApi, ChatBody};
use etke_openai_api_rust::images::{ImagesApi, ImagesBody};
use etke_openai_api_rust::{Auth, Message, OpenAI};

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
        MessageContent as LLMMessageContent, shorten_messages_list_to_context_size,
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

#[derive(Debug, Clone)]
pub struct Controller {
    config: Config,
    client: OpenAI,
}

impl Controller {
    pub fn new(config: Config) -> Self {
        let api_key = config.api_key.clone().unwrap_or("".to_owned());

        let auth = Auth::new(&api_key);

        // The library we use chokes if there's no trailing slash
        let base_url = if config.base_url.ends_with("/") {
            config.base_url.clone()
        } else {
            format!("{}/", config.base_url)
        };

        let client = OpenAI::new(auth, &base_url);

        Self { config, client }
    }
}

impl ControllerTrait for Controller {
    async fn ping(&self) -> anyhow::Result<PingResult> {
        if !self.supports_purpose(AgentPurpose::TextGeneration) {
            return Ok(PingResult::Inconclusive);
        }

        let messages = vec![LLMMessage {
            author: LLMAuthor::User,
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
                content: LLMMessageContent::Text(prompt_text),
                timestamp: chrono::Utc::now(),
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

        if let Some(prompt_message) = prompt_message {
            conversation_messages.insert(0, prompt_message);
        }

        let openai_conversation_messages: Vec<Message> =
            super::utils::convert_llm_messages_to_openai_messages(conversation_messages);

        let messages_count = openai_conversation_messages.len();

        let temperature = params
            .temperature_override
            .unwrap_or(text_generation_config.temperature);

        let max_tokens = text_generation_config
            .max_response_tokens
            .map(|max_response_tokens| {
                max_response_tokens
                    .try_into()
                    .expect("Failed converting max_response_tokens from u32 to i32")
            });

        let request = ChatBody {
            model: text_generation_config.model_id.clone(),
            max_tokens,
            temperature: Some(temperature),
            top_p: None,
            n: Some(1),
            stream: Some(false),
            stop: None,
            presence_penalty: None,
            frequency_penalty: None,
            logit_bias: None,
            user: None,
            messages: openai_conversation_messages,
        };

        if let Ok(request_as_json) = serde_json::to_string(&request) {
            tracing::trace!(
                model = format!("{:?}", request.model),
                ?messages_count,
                request = request_as_json,
                "Sending OpenAI-compat chat completion API request"
            );
        }

        // This library is not async-aware, so we need to use `spawn_blocking` to run the request on a separate thread.
        let client = self.client.clone();
        let response =
            tokio::task::spawn_blocking(move || client.chat_completion_create(&request)).await?;

        let response = match response {
            Ok(response) => response,
            Err(err) => {
                return Err(anyhow::anyhow!(
                    "Failed to get response from the OpenAI-compat chat completion API: {:?}",
                    err
                ));
            }
        };

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

            return Ok(TextGenerationResult {
                text: message.content,
            });
        }

        Err(anyhow::anyhow!(
            "No response messages choices were returned from the OpenAI-compat chat completion API"
        ))
    }

    async fn speech_to_text(
        &self,
        _mime_type: &mxlink::mime::Mime,
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

        // This library does not support passing the audio data as a byte slice, so we need to write it to a temporary file :/
        //
        // This temporary file will get auto-deleted when the variable goes out of scope.
        let temp_file = tokio::task::spawn_blocking(move || {
            let mut temp_file = match tempfile::NamedTempFile::new() {
                Ok(file) => file,
                Err(e) => return Err(e),
            };

            match std::io::Write::write_all(&mut temp_file, &media) {
                Ok(_) => (),
                Err(e) => return Err(e),
            }

            Ok(temp_file)
        })
        .await??;

        let file_path = temp_file
            .path()
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Failed to get temporary file path"))?;

        let language = params.language_override.clone();

        let request = AudioBody {
            file: std::fs::File::open(file_path)?,
            model: speech_to_text_config.model_id.to_owned(),
            prompt: None,
            response_format: None,
            temperature: None,
            language: language.clone(),
        };

        tracing::trace!(
            model_id = speech_to_text_config.model_id,
            ?language,
            "Sending OpenAI-compat speech-to-text API request"
        );

        // This library is not async-aware, so we need to use `spawn_blocking` to run the request on a separate thread.
        let client = self.client.clone();
        let response =
            tokio::task::spawn_blocking(move || client.audio_transcription_create(request)).await?;

        let response = match response {
            Ok(response) => response,
            Err(err) => {
                return Err(anyhow::anyhow!(
                    "Failed to get response from the OpenAI-compat audio transcription API: {:?}",
                    err
                ));
            }
        };

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

        let request = ImagesBody {
            model: Some(image_generation_config.model_id.to_owned()),
            prompt: prompt.to_owned(),
            n: Some(1),
            quality: image_generation_config.quality.clone(),
            size,
            style: image_generation_config.style.clone(),
            response_format: Some("b64_json".to_string()),
            user: None,
        };

        tracing::trace!(
            ?prompt,
            model = format!("{:?}", request.model),
            size = format!("{:?}", request.size),
            style = format!("{:?}", request.style),
            quality = format!("{:?}", request.quality),
            "Sending OpenAI-compat image generation API request"
        );

        // This library is not async-aware, so we need to use `spawn_blocking` to run the request on a separate thread.
        let client = self.client.clone();
        let response = tokio::task::spawn_blocking(move || client.image_create(&request)).await?;

        let response = match response {
            Ok(response) => response,
            Err(err) => {
                return Err(anyhow::anyhow!(
                    "Failed to get response from the OpenAI-compat image creation API: {:?}",
                    err
                ));
            }
        };

        let Some(data) = response.data else {
            return Err(anyhow::anyhow!(
                "The OpenAI-compat image generationAPI returned no image data"
            ));
        };

        if let Some(image) = data.into_iter().next() {
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
            "The OpenAI image generation API returned no images"
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
        // openai_api_rust does not support text-to-speech, so our only bet is to do it via async-openai and hope it works.
        // At the time of testing (2024-09-09), providers like LocalAI can be used for text-to-speech via async-openai.
        //
        // So.. below we try to convert our Config struct to the Config struct from the openai module
        // and invoke the openai controller.

        // Quick check to make sure doing work below is worth it
        let Some(_text_to_speech_config) = &self.config.text_to_speech else {
            return Err(anyhow::anyhow!(
                strings::agent::no_configuration_for_purpose_so_cannot_be_used(
                    &AgentPurpose::TextToSpeech
                ),
            ));
        };

        tracing::debug!("Converting OpenAI-compact config to OpenAI config..");

        let openai_config = super::utils::convert_config_to_openai_config_lossy(&self.config);

        let Some(_text_to_speech_config) = &openai_config.text_to_speech else {
            return Err(anyhow::anyhow!(
                strings::agent::no_configuration_for_purpose_after_conversion_so_cannot_be_used(
                    &AgentPurpose::TextToSpeech
                ),
            ));
        };

        let openai_controller = super::super::openai::Controller::new(openai_config);

        tracing::error!("Invoking text-to-speech via the OpenAI controller..");

        openai_controller.text_to_speech(input, params).await
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
        let Some(text_to_speech_config) = &self.config.text_to_speech else {
            return None;
        };

        // A hacky way to turn this enum to a string
        let voice_as_string = serde_json::to_string(&text_to_speech_config.voice).ok()?;
        Some(voice_as_string.replace("\"", ""))
    }

    fn text_to_speech_speed(&self) -> Option<f32> {
        let Some(text_to_speech_config) = &self.config.text_to_speech else {
            return None;
        };

        Some(text_to_speech_config.speed)
    }
}
