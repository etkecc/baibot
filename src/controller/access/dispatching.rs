use crate::{Bot, entity::MessageContext};

use super::AccessControllerType;

pub async fn dispatch_controller(
    handler: &AccessControllerType,
    message_context: &MessageContext,
    bot: &Bot,
) -> anyhow::Result<()> {
    match handler {
        AccessControllerType::Help => super::help::handle(bot, message_context).await,
        AccessControllerType::List => super::rules::handle_list(bot, message_context).await,
        AccessControllerType::AddRule(scope, rule) => {
            super::rules::handle_add(bot, message_context, *scope, rule).await
        }
        AccessControllerType::RemoveRule(scope, id) => {
            super::rules::handle_remove(bot, message_context, *scope, id).await
        }
        AccessControllerType::Explain(user_id) => {
            super::rules::handle_explain(bot, message_context, user_id).await
        }
    }
}
