use async_openai::types::responses::{
    EasyInputContent, EasyInputMessage, FunctionCallOutput, FunctionCallOutputItemParam,
    FunctionTool, ImageDetail, InputContent, InputFileArgs, InputImageContent, InputItem,
    InputParam, Item, MessagePhase, MessageType, OutputItem, OutputMessageContent, Role, Tool,
};

use crate::conversation::llm::{
    Author as LLMAuthor, Message as LLMMessage, MessageContent as LLMMessageContent,
};
use crate::tools::{ToolCall, ToolDefinition, ToolResult};
use crate::utils::base64::base64_encode;

pub fn convert_llm_messages_to_openai_response_input(
    conversation_messages: Vec<LLMMessage>,
) -> InputParam {
    let mut items = Vec::with_capacity(conversation_messages.len());

    for message in conversation_messages {
        let role = match message.author {
            LLMAuthor::Prompt => Role::System,
            LLMAuthor::Assistant => Role::Assistant,
            LLMAuthor::User => Role::User,
        };

        let content = match message.content {
            LLMMessageContent::Text(text) => EasyInputContent::Text(text),
            LLMMessageContent::Image(image_details) => {
                let image_url = format!(
                    "data:{};base64,{}",
                    image_details.mime,
                    base64_encode(&image_details.data)
                );

                EasyInputContent::ContentList(vec![InputContent::InputImage(InputImageContent {
                    image_url: Some(image_url),
                    detail: ImageDetail::Auto,
                    file_id: None,
                    prompt_cache_breakpoint: None,
                })])
            }
            LLMMessageContent::File(file_details) => {
                let file_data = format!(
                    "data:{};base64,{}",
                    file_details.mime,
                    base64_encode(&file_details.data)
                );

                let file_content = InputFileArgs::default()
                    .file_data(file_data)
                    .filename(file_details.filename())
                    .build()
                    .expect("Failed to build InputFileContent");

                EasyInputContent::ContentList(vec![InputContent::InputFile(file_content)])
            }
        };

        items.push(InputItem::EasyMessage(EasyInputMessage {
            r#type: MessageType::Message,
            role,
            content,
            phase: None,
        }));
    }

    InputParam::Items(items)
}

/// The input as a list of items (which the requests built here always are), to add to.
pub fn input_items(input: &mut InputParam) -> &mut Vec<InputItem> {
    if let InputParam::Text(text) = input {
        *input = InputParam::Items(vec![InputItem::EasyMessage(EasyInputMessage {
            r#type: MessageType::Message,
            role: Role::User,
            content: EasyInputContent::Text(std::mem::take(text)),
            phase: None,
        })]);
    }

    match input {
        InputParam::Items(items) => items,
        InputParam::Text(_) => unreachable!("text input was just turned into items"),
    }
}

/// Turns the functions offered to the model into function tools.
pub fn function_tools(definitions: &[ToolDefinition]) -> Vec<Tool> {
    definitions
        .iter()
        .map(|definition| {
            Tool::Function(FunctionTool {
                name: definition.name.clone(),
                parameters: Some(definition.parameters.clone()),
                // Strict mode needs every property to be required, which optional arguments (e.g. a limit) aren't.
                strict: Some(false),
                description: Some(definition.description.clone()),
                ..Default::default()
            })
        })
        .collect()
}

/// The function calls in the response output, in order.
pub fn function_calls(output: &[OutputItem]) -> Vec<ToolCall> {
    output
        .iter()
        .filter_map(|item| match item {
            OutputItem::FunctionCall(call) => Some(ToolCall {
                id: call.call_id.clone(),
                name: call.name.clone(),
                arguments: call.arguments.clone(),
            }),
            _ => None,
        })
        .collect()
}

/// The model's answer in the response output, if any: its final answer when messages are marked with phases,
/// or else its last message (earlier ones may be commentary, e.g. before a web search).
pub fn output_text(output: &[OutputItem]) -> Option<String> {
    let messages: Vec<_> = output
        .iter()
        .filter_map(|item| match item {
            OutputItem::Message(message) => {
                let text: String = message
                    .content
                    .iter()
                    .filter_map(|content| match content {
                        OutputMessageContent::OutputText(text_content) => {
                            Some(text_content.text.as_str())
                        }
                        _ => None,
                    })
                    .collect();

                (!text.is_empty()).then_some((message.phase.as_ref(), text))
            }
            _ => None,
        })
        .collect();

    let final_answer = messages
        .iter()
        .rev()
        .find(|(phase, _)| matches!(phase, Some(MessagePhase::FinalAnswer)));

    final_answer
        .or(messages.last())
        .map(|(_, text)| text.clone())
}

/// Turns the response output into input items, to send it back.
pub fn replay_items(output: Vec<OutputItem>) -> impl Iterator<Item = InputItem> {
    output.into_iter().map(|item| match item {
        // The input requires `outputs`, but it's left out when missing, so send an empty list instead.
        OutputItem::CodeInterpreterCall(mut call) => {
            call.outputs.get_or_insert_with(Vec::new);
            InputItem::from(OutputItem::CodeInterpreterCall(call))
        }
        item => InputItem::from(item),
    })
}

/// The input items which answer the function calls. Failures are marked as such, as there is no error flag.
pub fn function_call_outputs(results: Vec<ToolResult>) -> Vec<InputItem> {
    results
        .into_iter()
        .map(|result| {
            let output = if result.is_error {
                format!("Error: {}", result.content)
            } else {
                result.content
            };

            Item::FunctionCallOutput(FunctionCallOutputItemParam {
                call_id: Some(result.call_id),
                output: FunctionCallOutput::Text(output),
                id: None,
                status: None,
                name: None,
                namespace: None,
                caller: None,
            })
            .into()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conversation::llm::ImageDetails;
    use mxlink::matrix_sdk::ruma::{OwnedMxcUri, events::room::message::ImageMessageEventContent};
    use serde_json::json;

    #[test]
    fn image_input_keeps_data_url_without_enabling_prompt_cache_breakpoints() {
        let message = LLMMessage {
            author: LLMAuthor::User,
            sender_id: None,
            timestamp: chrono::Utc::now(),
            content: LLMMessageContent::Image(ImageDetails::new(
                ImageMessageEventContent::plain(
                    "cat.png".to_owned(),
                    OwnedMxcUri::from("mxc://example.com/cat"),
                ),
                mxlink::mime::IMAGE_PNG,
                vec![1, 2, 3],
            )),
        };

        let input = convert_llm_messages_to_openai_response_input(vec![message]);
        assert_eq!(
            serde_json::to_value(input).unwrap(),
            json!([{
                "type": "message",
                "role": "user",
                "content": [{
                    "type": "input_image",
                    "image_url": "data:image/png;base64,AQID",
                    "detail": "auto",
                }],
            }])
        );
    }

    /// Output shaped like the function calling example in OpenAI's docs, with reasoning and a web search.
    fn output() -> Vec<OutputItem> {
        serde_json::from_value(json!([
            {"type": "reasoning", "id": "rs_1", "summary": [], "encrypted_content": null},
            {"type": "web_search_call", "id": "ws_1", "status": "completed", "action": {"type": "search", "query": "x"}},
            {"type": "code_interpreter_call", "id": "ci_1", "status": "completed", "code": "print(1)", "container_id": "cntr_1", "outputs": null},
            {"type": "message", "id": "msg_1", "role": "assistant", "status": "completed", "content": [
                {"type": "output_text", "text": "Let me check the room.", "annotations": []}
            ]},
            {"type": "function_call", "id": "fc_1", "call_id": "call_1", "name": "room_history", "arguments": "{\"limit\":5}", "status": "completed"},
            {"type": "function_call", "id": "fc_2", "call_id": "call_2", "name": "room_history", "arguments": "{}", "status": "completed"}
        ]))
        .unwrap()
    }

    #[test]
    fn functions_become_non_strict_function_tools() {
        let tools = function_tools(&[ToolDefinition {
            name: "room_history".to_owned(),
            description: "Reads the room's recent messages.".to_owned(),
            parameters: json!({"type": "object", "properties": {"limit": {"type": "integer"}}}),
        }]);

        assert_eq!(
            serde_json::to_value(tools).unwrap(),
            json!([{
                "type": "function",
                "name": "room_history",
                "description": "Reads the room's recent messages.",
                "parameters": {"type": "object", "properties": {"limit": {"type": "integer"}}},
                "strict": false,
            }])
        );
    }

    #[test]
    fn function_calls_and_text_are_read_from_the_output() {
        let output = output();

        assert_eq!(
            function_calls(&output),
            [
                ToolCall {
                    id: "call_1".to_owned(),
                    name: "room_history".to_owned(),
                    arguments: r#"{"limit":5}"#.to_owned(),
                },
                ToolCall {
                    id: "call_2".to_owned(),
                    name: "room_history".to_owned(),
                    arguments: "{}".to_owned(),
                },
            ]
        );
        assert_eq!(
            output_text(&output).as_deref(),
            Some("Let me check the room.")
        );
    }

    fn message(text: &str, phase: Option<&str>) -> serde_json::Value {
        let mut message = json!({"type": "message", "id": "msg", "role": "assistant", "status": "completed", "content": [
            {"type": "output_text", "text": text, "annotations": []}
        ]});
        if let Some(phase) = phase {
            message["phase"] = json!(phase);
        }
        message
    }

    #[test]
    fn the_answer_is_the_final_answer_or_else_the_last_message() {
        let with_phases: Vec<OutputItem> = serde_json::from_value(json!([
            message("I'll verify that.", Some("commentary")),
            {"type": "web_search_call", "id": "ws_1", "status": "completed", "action": {"type": "search", "query": "x"}},
            message("It's 42.", Some("final_answer")),
            message("Anything else?", Some("commentary")),
        ]))
        .unwrap();
        assert_eq!(output_text(&with_phases).as_deref(), Some("It's 42."));

        let without_phases: Vec<OutputItem> = serde_json::from_value(json!([
            message("I'll verify that.", None),
            {"type": "web_search_call", "id": "ws_1", "status": "completed", "action": {"type": "search", "query": "x"}},
            message("It's 42.", None),
        ]))
        .unwrap();
        assert_eq!(output_text(&without_phases).as_deref(), Some("It's 42."));

        assert_eq!(output_text(&[]), None);
    }

    #[test]
    fn the_output_and_results_continue_the_input() {
        let mut input = InputParam::Text("Hi".to_owned());
        let items = input_items(&mut input);
        items.extend(replay_items(output()));

        let call = |id: &str| ToolCall {
            id: id.to_owned(),
            name: "room_history".to_owned(),
            arguments: "{}".to_owned(),
        };
        items.extend(function_call_outputs(vec![
            ToolResult::success(&call("call_1"), "Alice: hi".to_owned()),
            ToolResult::error(&call("call_2"), "Timed out"),
        ]));

        let input = serde_json::to_value(input).unwrap();
        let types: Vec<&str> = input
            .as_array()
            .unwrap()
            .iter()
            .map(|item| item["type"].as_str().unwrap())
            .collect();
        assert_eq!(
            types,
            [
                "message",
                "reasoning",
                "web_search_call",
                "code_interpreter_call",
                "message",
                "function_call",
                "function_call",
                "function_call_output",
                "function_call_output"
            ]
        );

        assert_eq!(input[1]["id"], "rs_1");
        assert_eq!(input[3]["outputs"], json!([]));
        assert_eq!(input[5]["call_id"], "call_1");
        assert_eq!(
            input[7],
            json!({"type": "function_call_output", "call_id": "call_1", "output": "Alice: hi"})
        );
        assert_eq!(
            input[8],
            json!({"type": "function_call_output", "call_id": "call_2", "output": "Error: Timed out"})
        );
    }
}
