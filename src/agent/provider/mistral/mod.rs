// Mistral is based on openai_compat, because it's not fully compatible with async-openai.

use super::openai_compat::Config;

pub fn default_config() -> Config {
    let mut config = Config {
        base_url: "https://api.mistral.ai/v1".to_owned(),

        speech_to_text: None,
        text_to_speech: None,
        image_generation: None,

        ..Default::default()
    };

    if let Some(ref mut config) = config.text_generation.as_mut() {
        config.model_id = "mistral-large-latest".to_owned();
        config.max_context_tokens = 128_000;
    }

    config
}
