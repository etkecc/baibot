use crate::agent::AgentPurpose;
use crate::agent::provider::entity::{TextToSpeechParams, TextToSpeechResult};
use crate::agent::provider::{SpeechToTextParams, SpeechToTextResult};
use crate::strings;

use super::config::Config;
use super::wire::{SpeechRequest, TranscriptionResponse};

pub async fn speech_to_text(
    config: &Config,
    http: &reqwest::Client,
    mime_type: &mxlink::mime::Mime,
    media: Vec<u8>,
    params: SpeechToTextParams,
) -> anyhow::Result<SpeechToTextResult> {
    let Some(speech_to_text_config) = &config.speech_to_text else {
        return Err(anyhow::anyhow!(
            strings::agent::no_configuration_for_purpose_so_cannot_be_used(
                &AgentPurpose::SpeechToText
            ),
        ));
    };

    // Unlike the openai_compat path (which writes the audio to a temp file because its library
    // can't take bytes), reqwest's multipart takes the bytes directly.
    let part = reqwest::multipart::Part::bytes(media)
        .file_name("audio")
        .mime_str(mime_type.as_ref())?;

    let mut form = reqwest::multipart::Form::new()
        .part("file", part)
        .text("model", speech_to_text_config.model_id.clone())
        .text("response_format", "json");

    if let Some(language) = &params.language_override {
        form = form.text("language", language.clone());
    }

    let url = format!(
        "{}/audio/transcriptions",
        config.base_url.trim_end_matches('/')
    );

    tracing::trace!(
        model_id = speech_to_text_config.model_id,
        language = ?params.language_override,
        "Sending Venice audio transcription API request"
    );

    let response = http
        .post(&url)
        .bearer_auth(&config.api_key)
        .multipart(form)
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() {
        // Body to the server log only, not into the returned error (which reaches the Matrix room).
        let body = response.text().await.unwrap_or_default();
        tracing::warn!(%status, body, "Venice audio transcription request failed");
        return Err(anyhow::anyhow!(
            "Venice audio transcription request failed with status {status}"
        ));
    }

    let response: TranscriptionResponse = response.json().await?;

    Ok(SpeechToTextResult {
        text: response.text,
    })
}

pub async fn text_to_speech(
    config: &Config,
    http: &reqwest::Client,
    input: &str,
    params: TextToSpeechParams,
) -> anyhow::Result<TextToSpeechResult> {
    let Some(text_to_speech_config) = &config.text_to_speech else {
        return Err(anyhow::anyhow!(
            strings::agent::no_configuration_for_purpose_so_cannot_be_used(
                &AgentPurpose::TextToSpeech
            ),
        ));
    };

    // Per-call overrides win over the configured defaults.
    let voice = params
        .voice_override
        .or_else(|| text_to_speech_config.voice.clone());
    let speed = params.speed_override.or(text_to_speech_config.speed);

    let response_format = text_to_speech_config.response_format.clone();
    let mime_type = response_format_to_mime_type(response_format.as_deref());

    let request = SpeechRequest {
        model: text_to_speech_config.model_id.clone(),
        input: input.to_owned(),
        voice,
        speed,
        response_format,
        prompt: text_to_speech_config.prompt.clone(),
        temperature: text_to_speech_config.temperature,
        top_p: text_to_speech_config.top_p,
    };

    let url = format!("{}/audio/speech", config.base_url.trim_end_matches('/'));

    tracing::trace!(
        model_id = text_to_speech_config.model_id,
        voice = ?request.voice,
        "Sending Venice text-to-speech API request"
    );

    let response = http
        .post(&url)
        .bearer_auth(&config.api_key)
        .json(&request)
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() {
        // Body to the server log only, not into the returned error (which reaches the Matrix room).
        let body = response.text().await.unwrap_or_default();
        tracing::warn!(%status, body, "Venice text-to-speech request failed");
        return Err(anyhow::anyhow!(
            "Venice text-to-speech request failed with status {status}"
        ));
    }

    // The speech endpoint answers with raw binary audio; read the body directly.
    let bytes = response.bytes().await?.to_vec();

    Ok(TextToSpeechResult { bytes, mime_type })
}

/// Map a Venice TTS `response_format` to its MIME type. Defaults to `audio/mpeg` (the
/// IANA-registered MP3 type, RFC 3003) when the format is unset, matching Venice's own `mp3`
/// default. This deliberately uses `audio/mpeg` rather than the `audio/mp3` alias the openai
/// provider emits; baibot's downstream audio-filename mapping treats both as `.mp3`.
fn response_format_to_mime_type(response_format: Option<&str>) -> mxlink::mime::Mime {
    let raw = match response_format.unwrap_or("mp3") {
        "mp3" => "audio/mpeg",
        "opus" => "audio/ogg",
        "aac" => "audio/aac",
        "flac" => "audio/flac",
        "wav" => "audio/wav",
        "pcm" => "audio/L8",
        _ => "audio/mpeg",
    };

    raw.parse()
        .unwrap_or(mxlink::mime::APPLICATION_OCTET_STREAM)
}
