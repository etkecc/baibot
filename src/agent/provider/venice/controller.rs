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
    MessageContent as LLMMessageContent,
};

use super::super::ControllerTrait;
use super::config::Config;
use super::recovery::UnsupportedFieldsCache;

#[derive(Debug, Clone)]
pub struct Controller {
    config: Config,
    http: reqwest::Client,
    // Per-model record of chat fields this Venice deployment has rejected as unsupported, learned at
    // runtime. `Arc`-backed inside, so the `Clone` derive shares one cache across all clones of an
    // agent's controller.
    unsupported_fields: UnsupportedFieldsCache,
}

impl Controller {
    pub fn new(config: Config) -> Self {
        // Image generation and text-to-speech can run long, so give the client a generous timeout
        // instead of reqwest's default (none). `build` only fails on TLS/system init; fall back to
        // the infallible `Client::new()` so this constructor stays infallible.
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            config,
            http,
            unsupported_fields: UnsupportedFieldsCache::default(),
        }
    }
}

impl ControllerTrait for Controller {
    async fn ping(&self) -> anyhow::Result<PingResult> {
        if !self.supports_purpose(AgentPurpose::TextGeneration) {
            return Ok(PingResult::Inconclusive);
        }

        // Mirror the openai/openai_compat ping: a real "Hello!" round-trip exercises the strict
        // /chat/completions body and auth, so a successful ping proves text generation works.
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
        super::chat::generate_text(
            &self.config,
            &self.http,
            &self.unsupported_fields,
            conversation,
            params,
        )
        .await
    }

    async fn speech_to_text(
        &self,
        mime_type: &mxlink::mime::Mime,
        media: Vec<u8>,
        params: SpeechToTextParams,
    ) -> anyhow::Result<SpeechToTextResult> {
        super::audio::speech_to_text(&self.config, &self.http, mime_type, media, params).await
    }

    async fn generate_image(
        &self,
        prompt: &str,
        params: ImageGenerationParams,
    ) -> anyhow::Result<ImageGenerationResult> {
        super::images::generate_image(&self.config, &self.http, prompt, params).await
    }

    async fn create_image_edit(
        &self,
        prompt: &str,
        images: Vec<ImageSource>,
        params: ImageEditParams,
    ) -> anyhow::Result<ImageEditResult> {
        super::images::create_image_edit(&self.config, &self.http, prompt, images, params).await
    }

    async fn text_to_speech(
        &self,
        input: &str,
        params: TextToSpeechParams,
    ) -> anyhow::Result<TextToSpeechResult> {
        super::audio::text_to_speech(&self.config, &self.http, input, params).await
    }

    fn supports_purpose(&self, purpose: AgentPurpose) -> bool {
        match purpose {
            AgentPurpose::TextGeneration => self.config.text_generation.is_some(),
            AgentPurpose::SpeechToText => self.config.speech_to_text.is_some(),
            AgentPurpose::TextToSpeech => self.config.text_to_speech.is_some(),
            AgentPurpose::ImageGeneration => self.config.image_generation.is_some(),
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
            .and_then(|config| config.voice.clone())
    }

    fn text_to_speech_speed(&self) -> Option<f32> {
        self.config
            .text_to_speech
            .as_ref()
            .and_then(|config| config.speed)
    }
}
