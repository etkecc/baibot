mod determination;
pub mod edit;
pub mod generation;
mod prompt;

pub use determination::determine_controller;

use mxlink::matrix_sdk::ruma::OwnedUserId;

use crate::{Bot, authorization::Permission, entity::MessageContext};

/// Tells if the thread was started (with the original prompt) by the bot or by someone allowed to
/// generate images. Continuing other threads would generate images from their starter's prompt.
///
/// The thread root is looked up by its event id, since thread messages are ordered by timestamp
/// and the root need not come first.
async fn thread_starter_may_generate(
    bot: &Bot,
    message_context: &MessageContext,
) -> anyhow::Result<bool> {
    let root_event = bot
        .room_event_fetcher()
        .fetch_event_in_room(
            &message_context.thread_info().root_event_id,
            message_context.room(),
        )
        .await?;

    let sender: OwnedUserId = root_event
        .raw()
        .get_field("sender")?
        .ok_or_else(|| anyhow::anyhow!("The thread root has no sender"))?;

    Ok(sender == *bot.user_id()
        || message_context
            .authorizer()
            .allows(&sender, Permission::ImageGeneration))
}
