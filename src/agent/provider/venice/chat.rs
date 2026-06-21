use crate::agent::AgentPurpose;
use crate::agent::provider::entity::{TextGenerationParams, TextGenerationResult};
use crate::conversation::llm::{
    Author as LLMAuthor, Conversation as LLMConversation, Message as LLMMessage,
    MessageContent as LLMMessageContent, shorten_messages_list_to_context_size,
};
use crate::strings;

use super::config::Config;
use super::utils::convert_llm_messages_to_venice;
use super::wire::{ChatCompletionRequest, ChatCompletionResponse};

pub async fn generate_text(
    config: &Config,
    http: &reqwest::Client,
    conversation: LLMConversation,
    params: TextGenerationParams,
) -> anyhow::Result<TextGenerationResult> {
    let Some(text_generation_config) = &config.text_generation else {
        return Err(anyhow::anyhow!(
            strings::agent::no_configuration_for_purpose_so_cannot_be_used(
                &AgentPurpose::TextGeneration
            ),
        ));
    };

    let prompt_text = params.prompt_variables.format(
        params
            .prompt_override
            .unwrap_or(text_generation_config.prompt.clone().unwrap_or_default())
            .trim(),
    );

    let prompt_message = if prompt_text.is_empty() {
        None
    } else {
        Some(LLMMessage {
            author: LLMAuthor::Prompt,
            sender_id: None,
            content: LLMMessageContent::Text(prompt_text),
            timestamp: chrono::Utc::now(),
        })
    };

    let mut conversation_messages = conversation.messages;

    if params.context_management_enabled {
        conversation_messages = shorten_messages_list_to_context_size(
            &text_generation_config.model_id,
            &prompt_message,
            conversation_messages,
            text_generation_config.max_response_tokens,
            text_generation_config.max_context_tokens,
        );
    }

    if let Some(prompt_message) = prompt_message {
        conversation_messages.insert(0, prompt_message);
    }

    let messages = convert_llm_messages_to_venice(conversation_messages);

    let temperature = params
        .temperature_override
        .unwrap_or(text_generation_config.temperature);

    let request = ChatCompletionRequest {
        model: text_generation_config.model_id.clone(),
        messages,
        temperature: Some(temperature),
        // Web search rides entirely inside `venice_parameters`; there is no `tools` array here.
        // `max_tokens` is deprecated on Venice in favor of `max_completion_tokens`.
        max_completion_tokens: text_generation_config.max_response_tokens,
        venice_parameters: text_generation_config.venice_parameters.clone(),
    };

    let url = format!(
        "{}/chat/completions",
        config.base_url.trim_end_matches('/')
    );

    tracing::trace!(
        model = text_generation_config.model_id,
        messages_count = request.messages.len(),
        "Sending Venice chat completion API request"
    );

    let response = http
        .post(&url)
        .bearer_auth(&config.api_key)
        .json(&request)
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() {
        // Log the body server-side for debugging (Venice explains a rejected strict body there),
        // but keep it OUT of the returned error: that error surfaces in the Matrix room, and the
        // body can carry account / rate-limit details that shouldn't reach room members.
        let body = response.text().await.unwrap_or_default();
        tracing::warn!(%status, body, "Venice chat completion request failed");
        return Err(anyhow::anyhow!(
            "Venice chat completion request failed with status {status}"
        ));
    }

    let response: ChatCompletionResponse = response.json().await?;

    let Some(choice) = response.choices.into_iter().next() else {
        return Err(anyhow::anyhow!(
            "No choices were returned from the Venice chat completion API"
        ));
    };

    let Some(content) = choice.message.content else {
        return Err(anyhow::anyhow!(
            "No message content was returned from the Venice chat completion API"
        ));
    };

    Ok(TextGenerationResult { text: content })
}
