// At the time of testing, Ollama can be powered by `openai`, but we use `openai_compat` for better reliability
// in the event of future updates to `async-openai`.

use super::openai_compat::Config;

pub fn default_config() -> Config {
    let mut config = Config {
        base_url: "http://my-ollama-self-hosted-service:11434/v1".to_owned(),

        text_to_speech: None,
        image_generation: None,
        speech_to_text: None,

        ..Default::default()
    };

    if let Some(ref mut config) = config.text_generation.as_mut() {
        config.model_id = "gemma2:2b".to_owned();
        config.max_context_tokens = 128_000;
        config.max_response_tokens = Some(4096);
    }

    config
}
