// Groq is based on openai_compat, because it's not fully compatible with async-openai.

use super::openai_compat::Config;

pub fn default_config() -> Config {
    let mut config = Config {
        base_url: "https://api.groq.com/openai/v1".to_owned(),

        text_to_speech: None,
        image_generation: None,

        ..Default::default()
    };

    if let Some(ref mut config) = config.text_generation.as_mut() {
        config.model_id = "llama3-70b-8192".to_owned();
        config.max_context_tokens = 131_072;
        config.max_response_tokens = 4096;
    }

    if let Some(ref mut config) = config.speech_to_text.as_mut() {
        config.model_id = "whisper-large-v3".to_owned();
    }

    config
}
