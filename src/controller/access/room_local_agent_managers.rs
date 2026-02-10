use mxlink::MessageResponseType;

use crate::{Bot, entity::MessageContext, strings};

pub async fn handle_get(bot: &Bot, message_context: &MessageContext) -> anyhow::Result<()> {
    let message = match &message_context
        .global_config()
        .access
        .room_local_agent_manager_patterns
    {
        Some(patterns) => strings::access::room_local_agent_managers_now_match_patterns(patterns),
        None => strings::access::room_local_agent_managers_no_patterns(),
    };

    bot.messaging()
        .send_text_markdown_no_fail(
            message_context.room(),
            message,
            MessageResponseType::Reply(message_context.thread_info().root_event_id.clone()),
        )
        .await;

    Ok(())
}

pub async fn handle_set(
    bot: &Bot,
    message_context: &MessageContext,
    patterns: &Option<Vec<String>>,
) -> anyhow::Result<()> {
    if let Some(patterns) = patterns
        && let Err(err) = mxidwc::parse_patterns_vector(patterns)
    {
        bot.messaging()
            .send_error_markdown_no_fail(
                message_context.room(),
                &strings::access::failed_to_parse_patterns(&err.to_string()),
                MessageResponseType::Reply(message_context.thread_info().root_event_id.clone()),
            )
            .await;

        return Ok(());
    }

    let mut global_config_manager_guard = bot.global_config_manager().lock().await;

    let mut global_config = global_config_manager_guard.get_or_create().await?;

    global_config.access.room_local_agent_manager_patterns = patterns.clone();

    global_config_manager_guard.persist(&global_config).await?;

    let message = match patterns {
        Some(patterns) => strings::access::room_local_agent_managers_now_match_patterns(patterns),
        None => strings::access::room_local_agent_managers_no_patterns(),
    };

    bot.messaging()
        .send_success_markdown_no_fail(
            message_context.room(),
            &message,
            MessageResponseType::Reply(message_context.thread_info().root_event_id.clone()),
        )
        .await;

    Ok(())
}
