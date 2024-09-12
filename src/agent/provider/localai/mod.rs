// LocalAI is based on OpenAI (async-openai), because it seems to be fully compatible.
// Moreover, openai_api_rust does not support speech-to-text, so if we wish to use this feature
// we need to stick to async-openai.

use super::openai_compat::Config;

pub fn default_config() -> Config {
    let mut config = Config {
        base_url: "http://my-localai-self-hosted-service:8080/v1".to_owned(),

        ..Default::default()
    };

    if let Some(ref mut config) = config.text_generation.as_mut() {
        config.model_id = "gpt-4".to_owned();
        config.max_context_tokens = 128_000;
        config.max_response_tokens = 4096;
    }

    if let Some(ref mut config) = config.text_to_speech.as_mut() {
        config.model_id = "tts-1".to_owned();
    }

    if let Some(ref mut config) = config.speech_to_text.as_mut() {
        config.model_id = "whisper-1".to_owned();
    }

    if let Some(ref mut config) = config.image_generation.as_mut() {
        config.model_id = "stablediffusion".to_owned();
    }

    config
}
