use super::wire::ChatMessage;

use crate::conversation::llm::{
    Author as LLMAuthor, Message as LLMMessage, MessageContent as LLMMessageContent,
};

pub fn convert_llm_messages_to_openai_messages(
    conversation_messages: Vec<LLMMessage>,
) -> Vec<ChatMessage> {
    let mut openai_conversation_messages: Vec<ChatMessage> =
        Vec::with_capacity(conversation_messages.len());

    for message in conversation_messages {
        let openai_message = convert_llm_message_to_openai_message(message);
        if let Some(openai_message) = openai_message {
            openai_conversation_messages.push(openai_message);
        }
    }

    openai_conversation_messages
}

fn convert_llm_message_to_openai_message(llm_message: LLMMessage) -> Option<ChatMessage> {
    let role = match llm_message.author {
        LLMAuthor::Prompt => "system",
        LLMAuthor::Assistant => "assistant",
        LLMAuthor::User => "user",
    };

    match llm_message.content {
        LLMMessageContent::Text(text) => Some(ChatMessage {
            role,
            content: text,
        }),
        LLMMessageContent::Image(_image_details) => {
            tracing::warn!(
                "The OpenAI-compat provider does not support image content. This image message will be skipped."
            );
            None
        }
        LLMMessageContent::File(_file_details) => {
            tracing::warn!(
                "The OpenAI-compat provider does not support file content. This file message will be skipped."
            );
            None
        }
    }
}

/// Turns a non-2xx response into an error, including the provider's error message when there is one.
pub(super) async fn error_from_response(api: &str, response: reqwest::Response) -> anyhow::Error {
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    tracing::warn!(%status, body, "OpenAI-compat {api} request failed");

    match error_message_from_body(&body) {
        Some(message) => {
            anyhow::anyhow!("The OpenAI-compat {api} API request failed ({status}): {message}")
        }
        None => anyhow::anyhow!("The OpenAI-compat {api} API request failed ({status})"),
    }
}

/// Finds the error message in the error body shapes that OpenAI-compatible providers use:
/// `{"error": {"message": "..."}}` (OpenAI, Groq, ...), `{"error": "..."}` (Ollama),
/// `{"message": "..."}` (Mistral) and `{"detail": "..."}`.
fn error_message_from_body(body: &str) -> Option<String> {
    let body: serde_json::Value = serde_json::from_str(body).ok()?;

    [
        &body["error"]["message"],
        &body["error"],
        &body["message"],
        &body["detail"],
    ]
    .into_iter()
    .find_map(|value| value.as_str())
    .map(str::to_owned)
}

/// Picks an upload file name for audio of the given type, as transcription APIs (e.g. Groq's)
/// may check the extension.
pub(super) fn audio_file_name(mime_type: &mxlink::mime::Mime) -> String {
    let extension = match mime_type.essence_str() {
        "audio/flac" | "audio/x-flac" => "flac",
        "audio/x-m4a" | "audio/m4a" | "audio/mp4" => "m4a",
        "audio/mp3" | "audio/mpeg" => "mp3",
        "audio/wav" | "audio/x-wav" | "audio/wave" => "wav",
        "audio/webm" => "webm",
        // Matrix voice messages are Ogg Opus, so that's the fallback too.
        _ => "ogg",
    };

    format!("audio.{extension}")
}

/// Maps a text-to-speech `response_format` to its MIME type, the same way as the openai provider.
pub(super) fn speech_response_format_to_mime_type(response_format: &str) -> mxlink::mime::Mime {
    let content_type = match response_format {
        "wav" => "audio/wav",
        "opus" => "audio/ogg",
        "aac" => "audio/aac",
        "flac" => "audio/flac",
        "pcm" => "audio/L8",
        _ => "audio/mp3",
    };

    content_type
        .parse()
        .unwrap_or(mxlink::mime::APPLICATION_OCTET_STREAM)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_keep_their_roles() {
        let message = |author, content| LLMMessage {
            author,
            sender_id: None,
            content,
            timestamp: chrono::Utc::now(),
        };

        let messages = convert_llm_messages_to_openai_messages(vec![
            message(
                LLMAuthor::Prompt,
                LLMMessageContent::Text("Be brief".to_owned()),
            ),
            message(LLMAuthor::User, LLMMessageContent::Text("Hi".to_owned())),
            message(
                LLMAuthor::Assistant,
                LLMMessageContent::Text("Hello".to_owned()),
            ),
        ]);

        assert_eq!(
            serde_json::to_value(&messages).unwrap(),
            serde_json::json!([
                {"role": "system", "content": "Be brief"},
                {"role": "user", "content": "Hi"},
                {"role": "assistant", "content": "Hello"}
            ])
        );
    }

    #[test]
    fn error_messages_are_found_in_each_providers_shape() {
        for body in [
            r#"{"error": {"message": "Unknown model", "type": "invalid_request_error"}}"#,
            r#"{"error": "Unknown model"}"#,
            r#"{"object": "error", "message": "Unknown model", "code": "unknown_model"}"#,
            r#"{"detail": "Unknown model"}"#,
        ] {
            assert_eq!(
                error_message_from_body(body).as_deref(),
                Some("Unknown model"),
                "{body}"
            );
        }

        assert_eq!(error_message_from_body("<html>"), None);
        assert_eq!(error_message_from_body(r#"{"error": {"code": 1}}"#), None);
    }

    #[test]
    fn audio_file_names_follow_the_mime_type() {
        let name = |mime: &str| audio_file_name(&mime.parse().unwrap());

        assert_eq!(name("audio/ogg; codecs=opus"), "audio.ogg");
        assert_eq!(name("audio/webm"), "audio.webm");
        assert_eq!(name("audio/x-wav"), "audio.wav");
        assert_eq!(name("audio/mpeg"), "audio.mp3");
        assert_eq!(name("application/octet-stream"), "audio.ogg");
    }

    #[test]
    fn speech_formats_map_to_mime_types() {
        assert_eq!(speech_response_format_to_mime_type("opus"), "audio/ogg");
        assert_eq!(speech_response_format_to_mime_type("mp3"), "audio/mp3");
        assert_eq!(speech_response_format_to_mime_type("unknown"), "audio/mp3");
    }
}
