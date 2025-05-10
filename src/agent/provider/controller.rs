use crate::{agent::AgentPurpose, conversation::llm::Conversation};

use super::{
    ImageGenerationParams, ImageEditParams, SpeechToTextParams, SpeechToTextResult,
    entity::{
        ImageGenerationResult, ImageEditResult, ImageSource, PingResult, TextGenerationParams,
        TextGenerationResult, TextToSpeechParams, TextToSpeechResult,
    },
};

pub trait ControllerTrait {
    fn supports_purpose(&self, purpose: AgentPurpose) -> bool;

    fn ping(&self) -> impl std::future::Future<Output = anyhow::Result<PingResult>> + Send;

    fn text_generation_model_id(&self) -> Option<String>;

    fn text_generation_prompt(&self) -> Option<String>;

    fn text_generation_temperature(&self) -> Option<f32>;

    fn text_to_speech_voice(&self) -> Option<String>;

    fn text_to_speech_speed(&self) -> Option<f32>;

    fn generate_text(
        &self,
        conversation: Conversation,
        params: TextGenerationParams,
    ) -> impl std::future::Future<Output = anyhow::Result<TextGenerationResult>> + Send;

    fn speech_to_text(
        &self,
        mime_type: &mxlink::mime::Mime,
        media: Vec<u8>,
        params: SpeechToTextParams,
    ) -> impl std::future::Future<Output = anyhow::Result<SpeechToTextResult>> + Send;

    fn generate_image(
        &self,
        prompt: &str,
        params: ImageGenerationParams,
    ) -> impl std::future::Future<Output = anyhow::Result<ImageGenerationResult>> + Send;

    fn create_image_edit(
        &self,
        prompt: &str,
        images: Vec<ImageSource>,
        params: ImageEditParams,
    ) -> impl std::future::Future<Output = anyhow::Result<ImageEditResult>> + Send;

    fn text_to_speech(
        &self,
        text: &str,
        params: TextToSpeechParams,
    ) -> impl std::future::Future<Output = anyhow::Result<TextToSpeechResult>> + Send;
}

#[derive(Debug, Clone)]
pub enum ControllerType {
    OpenAI(Box<super::openai::Controller>),
    OpenAICompat(Box<super::openai_compat::Controller>),
    Anthropic(Box<super::anthropic::Controller>),
}

impl ControllerTrait for ControllerType {
    fn supports_purpose(&self, purpose: AgentPurpose) -> bool {
        match &self {
            ControllerType::OpenAI(controller) => controller.supports_purpose(purpose),
            ControllerType::OpenAICompat(controller) => controller.supports_purpose(purpose),
            ControllerType::Anthropic(controller) => controller.supports_purpose(purpose),
        }
    }

    fn text_generation_model_id(&self) -> Option<String> {
        match &self {
            ControllerType::OpenAI(controller) => controller.text_generation_model_id(),
            ControllerType::OpenAICompat(controller) => controller.text_generation_model_id(),
            ControllerType::Anthropic(controller) => controller.text_generation_model_id(),
        }
    }

    fn text_generation_prompt(&self) -> Option<String> {
        match &self {
            ControllerType::OpenAI(controller) => controller.text_generation_prompt(),
            ControllerType::OpenAICompat(controller) => controller.text_generation_prompt(),
            ControllerType::Anthropic(controller) => controller.text_generation_prompt(),
        }
    }

    fn text_to_speech_voice(&self) -> Option<String> {
        match &self {
            ControllerType::OpenAI(controller) => controller.text_to_speech_voice(),
            ControllerType::OpenAICompat(controller) => controller.text_to_speech_voice(),
            ControllerType::Anthropic(controller) => controller.text_to_speech_voice(),
        }
    }

    fn text_to_speech_speed(&self) -> Option<f32> {
        match &self {
            ControllerType::OpenAI(controller) => controller.text_to_speech_speed(),
            ControllerType::OpenAICompat(controller) => controller.text_to_speech_speed(),
            ControllerType::Anthropic(controller) => controller.text_to_speech_speed(),
        }
    }

    fn text_generation_temperature(&self) -> Option<f32> {
        match &self {
            ControllerType::OpenAI(controller) => controller.text_generation_temperature(),
            ControllerType::OpenAICompat(controller) => controller.text_generation_temperature(),
            ControllerType::Anthropic(controller) => controller.text_generation_temperature(),
        }
    }

    async fn ping(&self) -> anyhow::Result<PingResult> {
        match &self {
            ControllerType::OpenAI(controller) => controller.ping().await,
            ControllerType::OpenAICompat(controller) => controller.ping().await,
            ControllerType::Anthropic(controller) => controller.ping().await,
        }
    }

    async fn generate_text(
        &self,
        conversation: Conversation,
        params: TextGenerationParams,
    ) -> anyhow::Result<TextGenerationResult> {
        match &self {
            ControllerType::OpenAI(controller) => {
                controller.generate_text(conversation, params).await
            }
            ControllerType::OpenAICompat(controller) => {
                controller.generate_text(conversation, params).await
            }
            ControllerType::Anthropic(controller) => {
                controller.generate_text(conversation, params).await
            }
        }
    }

    async fn speech_to_text(
        &self,
        mime_type: &mxlink::mime::Mime,
        media: Vec<u8>,
        params: SpeechToTextParams,
    ) -> anyhow::Result<SpeechToTextResult> {
        match &self {
            ControllerType::OpenAI(controller) => {
                controller.speech_to_text(mime_type, media, params).await
            }
            ControllerType::OpenAICompat(controller) => {
                controller.speech_to_text(mime_type, media, params).await
            }
            ControllerType::Anthropic(controller) => {
                controller.speech_to_text(mime_type, media, params).await
            }
        }
    }

    async fn generate_image(
        &self,
        prompt: &str,
        params: ImageGenerationParams,
    ) -> anyhow::Result<ImageGenerationResult> {
        match &self {
            ControllerType::OpenAI(controller) => controller.generate_image(prompt, params).await,
            ControllerType::OpenAICompat(controller) => {
                controller.generate_image(prompt, params).await
            }
            ControllerType::Anthropic(controller) => {
                controller.generate_image(prompt, params).await
            }
        }
    }

    async fn create_image_edit(
        &self,
        prompt: &str,
        images: Vec<ImageSource>,
        params: ImageEditParams,
    ) -> anyhow::Result<ImageEditResult> {
        match &self {
            ControllerType::OpenAI(controller) => controller.create_image_edit(prompt, images, params).await,
            ControllerType::OpenAICompat(controller) => {
                controller.create_image_edit(prompt, images, params).await
            }
            ControllerType::Anthropic(controller) => {
                controller.create_image_edit(prompt, images, params).await
            }
        }
    }

    async fn text_to_speech(
        &self,
        text: &str,
        params: TextToSpeechParams,
    ) -> anyhow::Result<TextToSpeechResult> {
        match &self {
            ControllerType::OpenAI(controller) => controller.text_to_speech(text, params).await,
            ControllerType::OpenAICompat(controller) => {
                controller.text_to_speech(text, params).await
            }
            ControllerType::Anthropic(controller) => controller.text_to_speech(text, params).await,
        }
    }
}
