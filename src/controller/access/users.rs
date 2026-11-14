use mxlink::MessageResponseType;

use crate::{
    Bot,
    entity::{MessageContext, globalconfig},
    strings,
};

pub async fn handle_get(bot: &Bot, message_context: &MessageContext) -> anyhow::Result<()> {
    let message = match &message_context.global_config().access.user_patterns {
        Some(patterns) => strings::access::users_now_match_patterns(patterns),
        None => strings::access::users_no_patterns(),
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

    globalconfig::update(bot.global_config_manager(), |global_config| {
        global_config.access.user_patterns = patterns.clone();
        Some(())
    })
    .await?;

    let message = match patterns {
        Some(patterns) => strings::access::users_now_match_patterns(patterns),
        None => strings::access::users_no_patterns(),
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
