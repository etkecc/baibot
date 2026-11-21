use std::collections::HashSet;
use std::sync::{OnceLock, RwLock};

use super::wire::{
    ChatMessage, ChatResponseMessage, FunctionCall, FunctionDefinition, Tool,
    ToolCall as WireToolCall,
};

use crate::conversation::llm::{
    Author as LLMAuthor, Message as LLMMessage, MessageContent as LLMMessageContent,
};
use crate::tools::{ToolCall, ToolDefinition, ToolResult};

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
        LLMMessageContent::Text(text) => Some(ChatMessage::text(role, text)),
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

/// Turns the functions offered to the model into tools.
pub(super) fn tools(definitions: &[ToolDefinition]) -> Vec<Tool> {
    definitions
        .iter()
        .map(|definition| Tool {
            kind: "function",
            function: FunctionDefinition {
                name: definition.name.clone(),
                description: definition.description.clone(),
                parameters: definition.parameters.clone(),
            },
        })
        .collect()
}

/// Reads the tool calls of a response message, along with the assistant message to send back with their results.
///
/// Calls without an id (or with one used by an earlier call) get a new one, and arguments sent as an object are
/// turned into JSON text, so the message sent back and the results always match the calls.
pub(super) fn tool_calls(message: ChatResponseMessage) -> (Vec<ToolCall>, ChatMessage) {
    // Sent back as received, so chunks such as Mistral's `thinking` ones stay in the transcript.
    let content = match &message.content {
        serde_json::Value::String(text) if text.is_empty() => None,
        serde_json::Value::String(_) | serde_json::Value::Array(_) => Some(message.content.clone()),
        _ => None,
    };

    let mut calls: Vec<ToolCall> = vec![];
    let mut wire_calls = vec![];

    for (index, call) in message.tool_calls.into_iter().flatten().enumerate() {
        let is_used = |id: &str| calls.iter().any(|call| call.id == id);

        let id = match call.id {
            Some(id) if !id.is_empty() && !is_used(&id) => id,
            _ => {
                let mut id = format!("call_{index}");
                while is_used(&id) {
                    id.push('_');
                }
                id
            }
        };

        let arguments = match call.function.arguments {
            serde_json::Value::String(arguments) => arguments,
            serde_json::Value::Null => "{}".to_owned(),
            arguments => arguments.to_string(),
        };

        wire_calls.push(WireToolCall {
            id: id.clone(),
            kind: "function",
            function: FunctionCall {
                name: call.function.name.clone(),
                arguments: arguments.clone(),
            },
        });

        calls.push(ToolCall {
            id,
            name: call.function.name,
            arguments,
        });
    }

    let assistant_message = ChatMessage {
        role: "assistant",
        content,
        tool_calls: wire_calls,
        tool_call_id: None,
        reasoning_content: message.reasoning_content.filter(|value| !value.is_null()),
        reasoning_details: message.reasoning_details.filter(|value| !value.is_null()),
    };

    (calls, assistant_message)
}

/// The `tool` messages which answer the calls. Failures are marked as such, as there is no error flag.
pub(super) fn tool_result_messages(results: Vec<ToolResult>) -> Vec<ChatMessage> {
    results
        .into_iter()
        .map(|result| ChatMessage {
            role: "tool",
            content: Some(serde_json::Value::String(if result.is_error {
                format!("Error: {}", result.content)
            } else {
                result.content
            })),
            tool_calls: vec![],
            tool_call_id: Some(result.call_id),
            reasoning_content: None,
            reasoning_details: None,
        })
        .collect()
}

/// Models (per base URL) which rejected a request with tools, so they get none from then on (until a restart).
fn models_without_tools() -> &'static RwLock<HashSet<(String, String)>> {
    static MODELS: OnceLock<RwLock<HashSet<(String, String)>>> = OnceLock::new();
    MODELS.get_or_init(|| RwLock::new(HashSet::new()))
}

pub(super) fn model_rejects_tools(base_url: &str, model_id: &str) -> bool {
    models_without_tools()
        .read()
        .is_ok_and(|models| models.contains(&(base_url.to_owned(), model_id.to_owned())))
}

pub(super) fn remember_model_rejects_tools(base_url: &str, model_id: &str) {
    if let Ok(mut models) = models_without_tools().write() {
        models.insert((base_url.to_owned(), model_id.to_owned()));
    }
}

/// Whether a failed request with tools looks rejected because the model doesn't support them, e.g.
/// Ollama's "... does not support tools" or Mistral's "Function calling is not enabled for this model".
///
/// Other errors about tools (e.g. Groq's "Invalid tool call generated" for a malformed call) don't count.
pub(super) fn is_tools_rejection(status: reqwest::StatusCode, body: &str) -> bool {
    if !status.is_client_error()
        || status == reqwest::StatusCode::UNAUTHORIZED
        || status == reqwest::StatusCode::FORBIDDEN
        || status == reqwest::StatusCode::TOO_MANY_REQUESTS
    {
        return false;
    }

    // Only the error message counts, as the rest may echo what the model generated (e.g. Groq's `failed_generation`).
    let Some(message) = error_message_from_body(body) else {
        return false;
    };
    let body = message.to_lowercase();

    let about_tools = body.contains("tool") || body.contains("function calling");
    let unsupported = ["support", "not enabled", "not available"]
        .iter()
        .any(|phrase| body.contains(phrase));

    about_tools && unsupported
}

/// Turns a non-2xx response into an error, including the provider's error message when there is one.
pub(super) async fn error_from_response(api: &str, response: reqwest::Response) -> anyhow::Error {
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    error_from_body(api, status, &body)
}

/// Turns a non-2xx response's status and body into an error (see [`error_from_response`]).
pub(super) fn error_from_body(api: &str, status: reqwest::StatusCode, body: &str) -> anyhow::Error {
    tracing::warn!(%status, body, "OpenAI-compat {api} request failed");

    match error_message_from_body(body) {
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

    fn response_message(json: serde_json::Value) -> ChatResponseMessage {
        serde_json::from_value(json).unwrap()
    }

    #[test]
    fn functions_become_tools() {
        let tools = tools(&[ToolDefinition {
            name: "room_history".to_owned(),
            description: "Reads the room's recent messages.".to_owned(),
            parameters: serde_json::json!({"type": "object", "properties": {}}),
        }]);

        assert_eq!(
            serde_json::to_value(tools).unwrap(),
            serde_json::json!([{"type": "function", "function": {
                "name": "room_history",
                "description": "Reads the room's recent messages.",
                "parameters": {"type": "object", "properties": {}}
            }}])
        );
    }

    #[test]
    fn tool_calls_are_sent_back_as_they_were_read() {
        // OpenAI's shape, plus a provider which leaves out the id and sends the arguments as an object.
        let message = response_message(serde_json::json!({
            "role": "assistant",
            "content": null,
            "tool_calls": [
                {"id": "call_abc", "type": "function", "function": {"name": "room_history", "arguments": "{\"limit\":5}"}},
                {"type": "function", "function": {"name": "room_history", "arguments": {"limit": 3}}}
            ]
        }));

        let (calls, assistant_message) = tool_calls(message);

        assert_eq!(
            calls,
            [
                ToolCall {
                    id: "call_abc".to_owned(),
                    name: "room_history".to_owned(),
                    arguments: r#"{"limit":5}"#.to_owned(),
                },
                ToolCall {
                    id: "call_1".to_owned(),
                    name: "room_history".to_owned(),
                    arguments: r#"{"limit":3}"#.to_owned(),
                },
            ]
        );

        assert_eq!(
            serde_json::to_value(assistant_message).unwrap(),
            serde_json::json!({"role": "assistant", "tool_calls": [
                {"id": "call_abc", "type": "function", "function": {"name": "room_history", "arguments": "{\"limit\":5}"}},
                {"id": "call_1", "type": "function", "function": {"name": "room_history", "arguments": "{\"limit\":3}"}}
            ]})
        );
    }

    #[test]
    fn call_ids_are_made_unique() {
        let message = response_message(serde_json::json!({
            "role": "assistant",
            "tool_calls": [
                {"id": "call_1", "function": {"name": "a", "arguments": "{}"}},
                {"function": {"name": "b", "arguments": "{}"}},
                {"id": "call_1", "function": {"name": "c", "arguments": "{}"}}
            ]
        }));

        let (calls, _) = tool_calls(message);
        let ids: Vec<&str> = calls.iter().map(|call| call.id.as_str()).collect();
        assert_eq!(ids, ["call_1", "call_1_", "call_2"]);
    }

    #[test]
    fn reasoning_is_sent_back_with_the_calls() {
        // DeepSeek's thinking mode and OpenRouter's reasoning details.
        let message = response_message(serde_json::json!({
            "role": "assistant",
            "content": "",
            "reasoning_content": "The user wants the room's history.",
            "reasoning_details": [{"type": "reasoning.encrypted", "data": "abc"}],
            "tool_calls": [{"id": "call_1", "type": "function", "function": {"name": "a", "arguments": "{}"}}]
        }));

        let (_, assistant_message) = tool_calls(message);
        let assistant_message = serde_json::to_value(assistant_message).unwrap();

        assert_eq!(
            assistant_message["reasoning_content"],
            "The user wants the room's history."
        );
        assert_eq!(
            assistant_message["reasoning_details"],
            serde_json::json!([{"type": "reasoning.encrypted", "data": "abc"}])
        );
        assert!(assistant_message.get("content").is_none());
    }

    #[test]
    fn text_is_read_from_chunks() {
        // Shaped like Mistral's reasoning models.
        let message = response_message(serde_json::json!({
            "role": "assistant",
            "content": [
                {"type": "thinking", "thinking": [{"type": "text", "text": "Hmm."}]},
                {"type": "text", "text": "Let me check."}
            ],
            "tool_calls": [{"id": "call_1", "type": "function", "function": {"name": "a", "arguments": "{}"}}]
        }));

        assert_eq!(message.text().as_deref(), Some("Let me check."));

        let (calls, assistant_message) = tool_calls(message);
        assert_eq!(calls.len(), 1);

        // Sent back as received, thinking included.
        assert_eq!(
            assistant_message.content.unwrap()[0]["type"],
            serde_json::json!("thinking")
        );
    }

    #[test]
    fn null_tool_calls_mean_none() {
        let message = response_message(
            serde_json::json!({"role": "assistant", "content": "Hi", "tool_calls": null}),
        );

        let (calls, _) = tool_calls(message);
        assert!(calls.is_empty());
    }

    #[test]
    fn tool_results_become_tool_messages() {
        let call = |id: &str| ToolCall {
            id: id.to_owned(),
            name: "room_history".to_owned(),
            arguments: "{}".to_owned(),
        };

        let messages = tool_result_messages(vec![
            ToolResult::success(&call("call_1"), "Alice: hi".to_owned()),
            ToolResult::error(&call("call_2"), "Timed out"),
        ]);

        assert_eq!(
            serde_json::to_value(messages).unwrap(),
            serde_json::json!([
                {"role": "tool", "content": "Alice: hi", "tool_call_id": "call_1"},
                {"role": "tool", "content": "Error: Timed out", "tool_call_id": "call_2"}
            ])
        );
    }

    #[test]
    fn tool_rejections_are_recognized() {
        let ollama = r#"{"error":{"message":"registry.ollama.ai/library/gemma3:1b does not support tools","type":"api_error","param":null,"code":null}}"#;
        assert!(is_tools_rejection(reqwest::StatusCode::BAD_REQUEST, ollama));

        let mistral = r#"{"object":"error","message":"Function calling is not enabled for this model","type":"invalid_request_error"}"#;
        assert!(is_tools_rejection(
            reqwest::StatusCode::BAD_REQUEST,
            mistral
        ));

        // A malformed call, which says nothing about whether the model supports tools.
        let groq = r#"{"error":{"message":"Invalid tool call generated","type":"invalid_request_error","code":"tool_use_failed","failed_generation":"{\"name\": \"room_history\", \"arguments\": {\"query\": \"tool support\"}}"}}"#;
        assert!(!is_tools_rejection(reqwest::StatusCode::BAD_REQUEST, groq));

        // Not about tools, or not the request's fault.
        assert!(!is_tools_rejection(
            reqwest::StatusCode::BAD_REQUEST,
            r#"{"error": {"message": "model not found"}}"#
        ));
        assert!(!is_tools_rejection(
            reqwest::StatusCode::INTERNAL_SERVER_ERROR,
            ollama
        ));
        assert!(!is_tools_rejection(
            reqwest::StatusCode::TOO_MANY_REQUESTS,
            r#"{"error": {"message": "too many tool requests"}}"#
        ));
    }

    #[test]
    fn models_which_reject_tools_are_remembered_per_base_url() {
        let base_url = "http://tools-test.invalid/v1";
        assert!(!model_rejects_tools(base_url, "small-model"));

        remember_model_rejects_tools(base_url, "small-model");

        assert!(model_rejects_tools(base_url, "small-model"));
        assert!(!model_rejects_tools(base_url, "other-model"));
        assert!(!model_rejects_tools(
            "http://other.invalid/v1",
            "small-model"
        ));
    }
}
