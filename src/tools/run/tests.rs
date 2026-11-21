use std::sync::Mutex;

use super::*;
use crate::agent::AgentPurpose;
use crate::agent::provider::entity::{
    ImageEditParams, ImageEditResult, ImageGenerationParams, ImageGenerationResult, ImageSource,
    PingResult, SpeechToTextParams, SpeechToTextResult, TextToSpeechParams, TextToSpeechResult,
};
use crate::conversation::llm::{Author, Message, MessageContent};
use crate::tools::{ProviderState, ToolDefinition};

/// What the fake provider saw in one request.
#[derive(Debug)]
struct Request {
    messages_count: usize,
    results: Option<Vec<ToolResult>>,
    state: Option<u32>,
}

/// A provider which replies with scripted results, numbering its states by round.
struct FakeController {
    script: Mutex<Vec<TextGenerationResult>>,
    requests: Mutex<Vec<Request>>,
}

impl FakeController {
    fn new(mut script: Vec<TextGenerationResult>) -> Self {
        script.reverse();
        Self {
            script: Mutex::new(script),
            requests: Mutex::new(vec![]),
        }
    }

    fn requests(&self) -> std::sync::MutexGuard<'_, Vec<Request>> {
        self.requests.lock().unwrap()
    }
}

fn calls(round: u32, names: &[&str]) -> TextGenerationResult {
    TextGenerationResult::ToolCalls {
        text: None,
        calls: names
            .iter()
            .enumerate()
            .map(|(index, name)| ToolCall {
                id: format!("call-{round}-{index}"),
                name: name.to_string(),
                arguments: "{}".to_owned(),
            })
            .collect(),
        continuation: ProviderState::new(round),
    }
}

fn answer(text: &str) -> TextGenerationResult {
    TextGenerationResult::text(text.to_owned())
}

impl ControllerTrait for FakeController {
    fn supports_purpose(&self, _purpose: AgentPurpose) -> bool {
        true
    }

    async fn ping(&self) -> anyhow::Result<PingResult> {
        Ok(PingResult::Successful)
    }

    fn text_generation_model_id(&self) -> Option<String> {
        None
    }

    fn text_generation_prompt(&self) -> Option<String> {
        None
    }

    fn text_generation_temperature(&self) -> Option<f32> {
        None
    }

    fn text_generation_max_context_tokens(&self) -> Option<u32> {
        None
    }

    fn text_to_speech_voice(&self) -> Option<String> {
        None
    }

    fn text_to_speech_speed(&self) -> Option<f32> {
        None
    }

    async fn generate_text(
        &self,
        conversation: Conversation,
        params: TextGenerationParams,
    ) -> anyhow::Result<TextGenerationResult> {
        self.requests.lock().unwrap().push(Request {
            messages_count: conversation.messages.len(),
            state: params
                .continuation
                .as_ref()
                .map(|continuation| continuation.state.clone().into_inner::<u32>().unwrap()),
            results: params.continuation.map(|continuation| continuation.results),
        });

        self.script
            .lock()
            .unwrap()
            .pop()
            .ok_or_else(|| anyhow::anyhow!("script exhausted"))
    }

    async fn speech_to_text(
        &self,
        _mime_type: &mxlink::mime::Mime,
        _media: Vec<u8>,
        _params: SpeechToTextParams,
    ) -> anyhow::Result<SpeechToTextResult> {
        unimplemented!()
    }

    async fn generate_image(
        &self,
        _prompt: &str,
        _params: ImageGenerationParams,
    ) -> anyhow::Result<ImageGenerationResult> {
        unimplemented!()
    }

    async fn create_image_edit(
        &self,
        _prompt: &str,
        _images: Vec<ImageSource>,
        _params: ImageEditParams,
    ) -> anyhow::Result<ImageEditResult> {
        unimplemented!()
    }

    async fn text_to_speech(
        &self,
        _text: &str,
        _params: TextToSpeechParams,
    ) -> anyhow::Result<TextToSpeechResult> {
        unimplemented!()
    }
}

/// Answers each call with `output`, recording the calls and the budgets it was given.
struct FakeExecutor {
    output: String,
    executed: Mutex<Vec<(String, usize)>>,
}

impl FakeExecutor {
    fn new(output: &str) -> Self {
        Self {
            output: output.to_owned(),
            executed: Mutex::new(vec![]),
        }
    }

    fn executed(&self) -> Vec<(String, usize)> {
        self.executed.lock().unwrap().clone()
    }
}

impl ToolExecutor for FakeExecutor {
    async fn execute(&self, call: &ToolCall, max_output_bytes: usize) -> ToolResult {
        self.executed
            .lock()
            .unwrap()
            .push((call.id.clone(), max_output_bytes));

        // A wrong id must not reach the provider.
        let mut result = ToolResult::success(call, self.output.clone());
        result.call_id = "wrong".to_owned();
        result
    }
}

fn conversation() -> Conversation {
    Conversation {
        messages: vec![Message {
            author: Author::User,
            sender_id: None,
            timestamp: chrono::Utc::now(),
            content: MessageContent::Text("Hello".to_owned()),
        }],
    }
}

fn params_with_tools() -> TextGenerationParams {
    TextGenerationParams {
        tools: ["room_history", "a", "b", "c", "d", "e", "f", "g"]
            .into_iter()
            .map(|name| ToolDefinition {
                name: name.to_owned(),
                description: "Does something".to_owned(),
                parameters: serde_json::json!({"type": "object", "properties": {}}),
            })
            .collect(),
        ..Default::default()
    }
}

async fn run(
    controller: &FakeController,
    executor: &FakeExecutor,
    limits: ToolLimits,
) -> anyhow::Result<String> {
    run_text_generation(
        controller,
        conversation(),
        params_with_tools(),
        executor,
        limits,
    )
    .await
}

#[tokio::test]
async fn a_direct_answer_makes_a_single_request() {
    let controller = FakeController::new(vec![answer("Hi")]);
    let executor = FakeExecutor::new("unused");

    assert_eq!(
        run(&controller, &executor, ToolLimits::default())
            .await
            .unwrap(),
        "Hi"
    );

    let requests = controller.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].messages_count, 1);
    assert!(requests[0].results.is_none());
    assert!(executor.executed().is_empty());
}

#[tokio::test]
async fn every_call_gets_a_result_with_its_id_and_the_turn_continues_from_the_provider_state() {
    let controller = FakeController::new(vec![
        calls(1, &["room_history", "room_history"]),
        answer("Done"),
    ]);
    let executor = FakeExecutor::new("output");

    assert_eq!(
        run(&controller, &executor, ToolLimits::default())
            .await
            .unwrap(),
        "Done"
    );

    let requests = controller.requests();
    assert_eq!(requests.len(), 2);

    // The conversation is only sent with the first request.
    assert_eq!(requests[1].messages_count, 0);
    assert_eq!(requests[1].state, Some(1));

    let results = requests[1].results.as_ref().unwrap();
    assert_eq!(
        results
            .iter()
            .map(|result| result.call_id.as_str())
            .collect::<Vec<_>>(),
        ["call-1-0", "call-1-1"]
    );
    assert!(results.iter().all(|result| !result.is_error));
}

#[tokio::test]
async fn calls_after_the_round_limit_get_an_error_and_one_more_round() {
    let controller = FakeController::new(vec![
        calls(1, &["room_history"]),
        calls(2, &["room_history"]),
        calls(3, &["room_history"]),
        calls(4, &["room_history"]),
        answer("Done"),
    ]);
    let executor = FakeExecutor::new("output");

    assert_eq!(
        run(&controller, &executor, ToolLimits::default())
            .await
            .unwrap(),
        "Done"
    );

    assert_eq!(executor.executed().len(), 3);

    let requests = controller.requests();
    let last_results = requests[4].results.as_ref().unwrap();
    assert!(last_results[0].is_error);
    assert_eq!(last_results[0].content, LIMIT_REACHED_MESSAGE);
}

#[tokio::test]
async fn calls_after_the_call_limit_get_an_error() {
    let controller = FakeController::new(vec![
        calls(1, &["a", "b", "c", "d"]),
        calls(2, &["e", "f", "g"]),
        answer("Done"),
    ]);
    let executor = FakeExecutor::new("output");

    run(&controller, &executor, ToolLimits::default())
        .await
        .unwrap();

    assert_eq!(executor.executed().len(), 6);

    let requests = controller.requests();
    let results = requests[2].results.as_ref().unwrap();
    assert_eq!(
        results
            .iter()
            .map(|result| result.is_error)
            .collect::<Vec<_>>(),
        [false, false, true]
    );
}

#[tokio::test]
async fn calling_tools_after_the_limit_ends_with_the_text_written_so_far() {
    let mut script = vec![];
    for round in 1..=4 {
        script.push(calls(round, &["room_history"]));
    }
    script.push(TextGenerationResult::ToolCalls {
        text: Some("Partial answer".to_owned()),
        calls: vec![ToolCall {
            id: "x".to_owned(),
            name: "room_history".to_owned(),
            arguments: "{}".to_owned(),
        }],
        continuation: ProviderState::new(5_u32),
    });

    let controller = FakeController::new(script);
    let executor = FakeExecutor::new("output");

    assert_eq!(
        run(&controller, &executor, ToolLimits::default())
            .await
            .unwrap(),
        "Partial answer"
    );
}

#[tokio::test]
async fn calling_tools_after_the_limit_without_text_is_an_error() {
    let mut script = vec![];
    for round in 1..=5 {
        script.push(calls(round, &["room_history"]));
    }

    let controller = FakeController::new(script);
    let executor = FakeExecutor::new("output");

    assert!(
        run(&controller, &executor, ToolLimits::default())
            .await
            .is_err()
    );
    assert_eq!(controller.requests().len(), 5);
}

#[tokio::test]
async fn output_is_kept_within_the_budget_for_the_reply() {
    let controller = FakeController::new(vec![calls(1, &["a", "b", "c"]), answer("Done")]);
    let executor = FakeExecutor::new(&"x".repeat(100));

    run(
        &controller,
        &executor,
        ToolLimits::with_max_output_bytes(200),
    )
    .await
    .unwrap();

    // The calls themselves take 57 bytes (id twice, name and `{}` arguments, 3 times).
    // The second call only gets what the first one left, and the third one nothing.
    assert_eq!(
        executor.executed(),
        [("call-1-0".to_owned(), 143), ("call-1-1".to_owned(), 43)]
    );

    let requests = controller.requests();
    let results = requests[1].results.as_ref().unwrap();
    assert_eq!(results[0].content.len(), 100);
    assert_eq!(results[1].content.len(), 43);
    assert!(results[2].is_error);
    assert_eq!(results[2].content, OUTPUT_BUDGET_USED_UP_MESSAGE);

    // Only the fixed error result goes over the budget.
    let total: usize = results.iter().map(|result| result.content.len()).sum();
    assert_eq!(total, 200 - 57 + OUTPUT_BUDGET_USED_UP_MESSAGE.len());
}

#[tokio::test]
async fn calls_of_tools_which_were_not_offered_get_an_error_without_running() {
    let controller =
        FakeController::new(vec![calls(1, &["room_history", "made_up"]), answer("Done")]);
    let executor = FakeExecutor::new("output");

    run(&controller, &executor, ToolLimits::default())
        .await
        .unwrap();

    assert_eq!(executor.executed().len(), 1);
    assert_eq!(executor.executed()[0].0, "call-1-0");

    let requests = controller.requests();
    let results = requests[1].results.as_ref().unwrap();
    assert!(!results[0].is_error);
    assert!(results[1].is_error);
    assert_eq!(results[1].call_id, "call-1-1");
    assert_eq!(results[1].content, UNKNOWN_TOOL_MESSAGE);
}

#[tokio::test]
async fn calls_of_unknown_tools_count_towards_the_limits() {
    let mut script = vec![];
    for round in 1..=5 {
        script.push(calls(round, &["made_up"]));
    }

    let controller = FakeController::new(script);
    let executor = FakeExecutor::new("output");

    assert!(
        run(&controller, &executor, ToolLimits::default())
            .await
            .is_err()
    );
    assert!(executor.executed().is_empty());

    let requests = controller.requests();
    assert_eq!(requests.len(), 5);
    assert_eq!(
        requests[4].results.as_ref().unwrap()[0].content,
        LIMIT_REACHED_MESSAGE
    );
}

#[tokio::test]
async fn a_huge_batch_of_calls_ends_the_turn_without_running_any() {
    let names = vec!["room_history"; MAX_CALLS_PER_BATCH + 1];
    let mut huge_batch = calls(1, &names);
    if let TextGenerationResult::ToolCalls { text, .. } = &mut huge_batch {
        *text = Some("Partial answer".to_owned());
    }

    let controller = FakeController::new(vec![huge_batch]);
    let executor = FakeExecutor::new("output");

    assert_eq!(
        run(&controller, &executor, ToolLimits::default())
            .await
            .unwrap(),
        "Partial answer"
    );
    assert!(executor.executed().is_empty());
}

#[tokio::test]
async fn tool_calls_without_offered_tools_end_the_turn() {
    let controller = FakeController::new(vec![calls(1, &["room_history"])]);
    let executor = FakeExecutor::new("output");

    let result = run_text_generation(
        &controller,
        conversation(),
        TextGenerationParams::default(),
        &executor,
        ToolLimits::default(),
    )
    .await;

    assert!(result.is_err());
    assert!(executor.executed().is_empty());
}

#[test]
fn truncation_keeps_whole_characters_and_says_so() {
    let mut text = "ж".repeat(200);
    truncate_to(&mut text, 301);
    assert!(text.len() <= 301);
    assert!(text.ends_with(TRUNCATION_NOTE));

    let mut short = "short".to_owned();
    truncate_to(&mut short, 100);
    assert_eq!(short, "short");

    let mut tiny_budget = "abcdef".to_owned();
    truncate_to(&mut tiny_budget, 3);
    assert_eq!(tiny_budget, "abc");
}

#[test]
fn the_budget_follows_the_context_size_when_context_management_is_on() {
    let (reserved, limits) = tool_budget(true, Some(8_000));
    assert_eq!(reserved, 2_000);
    assert_eq!(limits.max_output_bytes, 6_000);

    let (reserved, limits) = tool_budget(true, Some(1_000_000));
    assert_eq!(reserved, 16 * 1024);
    assert_eq!(limits.max_output_bytes, 48 * 1024);

    let (reserved, limits) = tool_budget(false, Some(8_000));
    assert_eq!(reserved, 0);
    assert_eq!(limits.max_output_bytes, MAX_OUTPUT_BYTES_PER_REPLY);
}

#[tokio::test]
async fn calls_of_unknown_tools_use_up_the_call_limit() {
    let mut names = vec!["made_up"; 6];
    names.push("room_history");

    let controller = FakeController::new(vec![calls(1, &names), answer("Done")]);
    let executor = FakeExecutor::new("output");

    run(&controller, &executor, ToolLimits::default())
        .await
        .unwrap();

    assert!(executor.executed().is_empty());

    let requests = controller.requests();
    let results = requests[1].results.as_ref().unwrap();
    assert_eq!(results[6].content, LIMIT_REACHED_MESSAGE);
}

#[tokio::test]
async fn calls_larger_than_the_budget_end_the_turn_without_running() {
    let controller = FakeController::new(vec![TextGenerationResult::ToolCalls {
        text: None,
        calls: vec![ToolCall {
            id: "call".to_owned(),
            name: "room_history".to_owned(),
            arguments: format!("{{\"x\": \"{}\"}}", "y".repeat(300)),
        }],
        continuation: ProviderState::new(1_u32),
    }]);
    let executor = FakeExecutor::new("output");

    assert!(
        run(
            &controller,
            &executor,
            ToolLimits::with_max_output_bytes(200)
        )
        .await
        .is_err()
    );
    assert!(executor.executed().is_empty());
    assert_eq!(controller.requests().len(), 1);
}

#[tokio::test]
async fn call_ids_count_twice_towards_the_budget() {
    // Fits once (about 100 + 14 bytes), but not twice.
    let controller = FakeController::new(vec![TextGenerationResult::ToolCalls {
        text: None,
        calls: vec![ToolCall {
            id: "x".repeat(100),
            name: "room_history".to_owned(),
            arguments: "{}".to_owned(),
        }],
        continuation: ProviderState::new(1_u32),
    }]);
    let executor = FakeExecutor::new("output");

    assert!(
        run(
            &controller,
            &executor,
            ToolLimits::with_max_output_bytes(200)
        )
        .await
        .is_err()
    );
    assert!(executor.executed().is_empty());
}
