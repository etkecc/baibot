use crate::conversation::llm::{
    Author as LLMAuthor, Message as LLMMessage, MessageContent as LLMMessageContent,
};
use crate::utils::base64::base64_encode;

use crate::tools::{ToolCall, ToolDefinition, ToolResult};

use super::wire::{
    ChatMessage, ContentPart, FilePart, FunctionCall, FunctionDefinition, ImageUrl, MessageContent,
    ResponseMessage, Tool, ToolCall as WireToolCall,
};

/// Venice's documented file-input ceiling is 25MB on the decoded bytes (swagger `file_data`).
/// We check it here so an oversized file gets a clear message instead of an opaque 413 from the
/// API; the 413 status branch in `chat.rs` is the backstop if a file slips past this guard.
const MAX_FILE_BYTES: usize = 25 * 1024 * 1024;

pub fn convert_llm_messages_to_venice(
    messages: Vec<LLMMessage>,
) -> anyhow::Result<Vec<ChatMessage>> {
    let mut venice_messages: Vec<ChatMessage> = Vec::with_capacity(messages.len());

    for message in messages {
        venice_messages.push(convert_llm_message_to_venice(message)?);
    }

    Ok(venice_messages)
}

fn convert_llm_message_to_venice(message: LLMMessage) -> anyhow::Result<ChatMessage> {
    let role = match message.author {
        LLMAuthor::Prompt => "system",
        LLMAuthor::Assistant => "assistant",
        LLMAuthor::User => "user",
    };

    match message.content {
        LLMMessageContent::Text(text) => Ok(ChatMessage::new(role, MessageContent::Text(text))),
        LLMMessageContent::Image(image_details) => {
            // Inline the image as a base64 data URI, the same shape the OpenAI vision content
            // part uses. This is the gap the openai_compat provider can't fill (it drops images).
            let data_uri = format!(
                "data:{};base64,{}",
                image_details.mime,
                base64_encode(&image_details.data)
            );

            Ok(ChatMessage::new(
                role,
                MessageContent::Parts(vec![ContentPart::ImageUrl {
                    image_url: ImageUrl { url: data_uri },
                }]),
            ))
        }
        LLMMessageContent::File(file_details) => {
            // Inline the file as a base64 data URI in a `file` content part. This is the input
            // type the openai_compat provider drops; baibot already extracts the bytes upstream.
            // The message reaches the room, so it carries no user-controlled filename: a crafted
            // name could otherwise inject markdown (a spoofed link) into the bot's reply.
            if file_details.data.len() > MAX_FILE_BYTES {
                return Err(anyhow::anyhow!(
                    "The attached file is too large for Venice (the limit is 25MB)."
                ));
            }

            let data_uri = format!(
                "data:{};base64,{}",
                file_details.mime,
                base64_encode(&file_details.data)
            );

            Ok(ChatMessage::new(
                role,
                MessageContent::Parts(vec![ContentPart::File {
                    file: FilePart {
                        file_data: data_uri,
                        filename: Some(file_details.filename()),
                    },
                }]),
            ))
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

/// Whether a call in a response is one of Venice's own rather than a call of one of the offered tools. Those are not run.
/// With `return_search_results_as_documents`, Venice adds a call with the web search results (documented as
/// `{"id": "web_search_call", "function": {"name": "web_search", ...}}`). Such calls are sent back with their arguments
/// as the result (see [`tool_calls`]), as Venice searches only in the first round of a turn.
///
/// Calls of other unknown names count as the model's, so that it is told they failed.
fn is_venice_call(id: Option<&str>, name: &str, offered: &[Tool]) -> bool {
    if id == Some("web_search_call") || name.is_empty() {
        return true;
    }

    let is_offered = offered.iter().any(|tool| tool.function.name == name);

    !is_offered && (name.starts_with("venice_") || matches!(name, "web_search" | "x_search"))
}

/// The calls of a response message which aren't Venice's own (see [`is_venice_call`]), with their function's name.
fn model_calls(
    message: &ResponseMessage,
    offered: &[Tool],
) -> Vec<(Option<String>, String, serde_json::Value)> {
    calls_where(message, |id, name| !is_venice_call(id, name, offered))
}

/// Venice's own calls of a response message (see [`is_venice_call`]) which carry web search results, to be sent back:
/// those named like the documented ones, whose arguments hold the documents (`{"documents": [...]}`). Others (e.g. made
/// up by the model, like a search query, or merely using Venice's id) are dropped.
fn search_results_calls(
    message: &ResponseMessage,
    offered: &[Tool],
) -> Vec<(Option<String>, String, serde_json::Value)> {
    calls_where(message, |id, name| {
        is_venice_call(id, name, offered)
            && matches!(name, "web_search" | "venice_web_search_documents")
    })
    .into_iter()
    .filter(|(_, _, arguments)| holds_documents(arguments))
    .collect()
}

/// Whether call arguments (JSON text or an object) are web search results: an object with a `documents` list.
fn holds_documents(arguments: &serde_json::Value) -> bool {
    let parsed;
    let arguments = match arguments {
        serde_json::Value::String(text) => {
            let Ok(value) = serde_json::from_str::<serde_json::Value>(text) else {
                return false;
            };
            parsed = value;
            &parsed
        }
        arguments => arguments,
    };

    arguments
        .get("documents")
        .is_some_and(serde_json::Value::is_array)
}

fn calls_where(
    message: &ResponseMessage,
    keep: impl Fn(Option<&str>, &str) -> bool,
) -> Vec<(Option<String>, String, serde_json::Value)> {
    message
        .tool_calls()
        .into_iter()
        .filter_map(|call| {
            let function = call.function?;
            let name = function.name.unwrap_or_default();

            keep(call.id.as_deref(), &name).then_some((call.id, name, function.arguments))
        })
        .collect()
}

/// Arguments as JSON text, as calls are sent back with them that way.
fn arguments_text(arguments: serde_json::Value) -> String {
    match arguments {
        serde_json::Value::String(arguments) => arguments,
        serde_json::Value::Null => "{}".to_owned(),
        arguments => arguments.to_string(),
    }
}

/// Whether a response message calls any of the bot's tools (see [`tool_calls`]). Never when no tools were offered.
pub(super) fn has_tool_calls(message: &ResponseMessage, offered: &[Tool]) -> bool {
    !offered.is_empty() && !model_calls(message, offered).is_empty()
}

/// Reads the calls of the offered tools from a response message, along with the assistant message to send back with
/// their results and the `tool` messages answering Venice's own calls in it. Returns no calls when there are none
/// (Venice's own calls don't count).
///
/// Venice's web search results arrive as a call of its own, and it doesn't search again when the turn continues with
/// the results of the bot's tools. So that the model keeps them, such calls are sent back too (listed first, as their
/// results are sent first), answered with their arguments (the results). Venice's other calls aren't sent back.
///
/// Calls without an id (or with one used by an earlier call) get a new one, and arguments sent as an object are turned
/// into JSON text, so the message sent back and the results always match the calls.
pub(super) fn tool_calls(
    message: ResponseMessage,
    offered: &[Tool],
) -> (Vec<ToolCall>, ChatMessage, Vec<ChatMessage>) {
    let mut calls: Vec<ToolCall> = vec![];
    let mut wire_calls: Vec<WireToolCall> = vec![];

    let venice_calls = search_results_calls(&message, offered);

    for (index, (id, name, arguments)) in model_calls(&message, offered).into_iter().enumerate() {
        let is_used = |id: &str| calls.iter().any(|call| call.id == id);

        let id = match id {
            Some(id) if !id.is_empty() && !is_used(&id) => id,
            _ => {
                let mut id = format!("call_{index}");
                while is_used(&id) {
                    id.push('_');
                }
                id
            }
        };

        let arguments = arguments_text(arguments);

        wire_calls.push(WireToolCall {
            id: id.clone(),
            kind: "function",
            function: FunctionCall {
                name: name.clone(),
                arguments: arguments.clone(),
            },
        });

        calls.push(ToolCall {
            id,
            name,
            arguments,
        });
    }

    let mut venice_results = vec![];
    let mut venice_wire_calls: Vec<WireToolCall> = vec![];

    for (id, name, arguments) in venice_calls {
        let is_used = |id: &str| {
            wire_calls
                .iter()
                .chain(&venice_wire_calls)
                .any(|call| call.id == id)
        };

        let mut id = id
            .filter(|id| !id.is_empty())
            .unwrap_or_else(|| "web_search_call".to_owned());
        while is_used(&id) {
            id.push('_');
        }

        let arguments = arguments_text(arguments);

        venice_results.push(ChatMessage {
            tool_call_id: Some(id.clone()),
            ..ChatMessage::new("tool", MessageContent::Text(arguments.clone()))
        });

        venice_wire_calls.push(WireToolCall {
            id,
            kind: "function",
            function: FunctionCall { name, arguments },
        });
    }

    // Venice's calls come first, as their results are sent right after the message, before those of the bot's tools.
    // Results are then in the order of the calls, which matters to models whose chat templates drop the call ids.
    venice_wire_calls.append(&mut wire_calls);
    let wire_calls = venice_wire_calls;

    let assistant_message = ChatMessage {
        role: "assistant".to_owned(),
        content: message
            .content
            .filter(|text| !text.is_empty())
            .map(MessageContent::Text),
        tool_calls: wire_calls,
        tool_call_id: None,
        reasoning_content: message.reasoning_content.filter(|value| !value.is_null()),
        reasoning_details: message.reasoning_details.filter(|value| !value.is_null()),
        thought_signature: message.thought_signature.filter(|value| !value.is_null()),
    };

    (calls, assistant_message, venice_results)
}

/// The `tool` messages which answer the calls. Failures are marked as such, as there is no error flag.
pub(super) fn tool_result_messages(results: Vec<ToolResult>) -> Vec<ChatMessage> {
    results
        .into_iter()
        .map(|result| {
            let content = if result.is_error {
                format!("Error: {}", result.content)
            } else {
                result.content
            };

            ChatMessage {
                tool_call_id: Some(result.call_id),
                ..ChatMessage::new("tool", MessageContent::Text(content))
            }
        })
        .collect()
}
