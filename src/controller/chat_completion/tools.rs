//! Running the tools the model calls while answering a message.

use mxlink::matrix_sdk::Room;
use mxlink::matrix_sdk::ruma::{OwnedEventId, OwnedUserId, UserId};

use crate::Bot;
use crate::authorization::{Authorizer, Permission, room_power_levels};
use crate::tools::catalog::ToolInfo;
use crate::tools::registry::{self, Tool, ToolContext};
use crate::tools::{ToolCall, ToolExecutor, ToolResult};

const NOT_ALLOWED_MESSAGE: &str = "The person you are answering may no longer use this tool (access rules changed). Answer without it.";

const ACCESS_CHECK_FAILED_MESSAGE: &str =
    "Failed to check whether this tool may be used. Answer without it.";

/// Runs the tools offered for a reply, on behalf of the person being answered.
pub struct BotToolExecutor<'a> {
    bot: &'a Bot,
    room: Room,
    requester: OwnedUserId,
    trigger_event_id: OwnedEventId,
    tools: Vec<&'static dyn Tool>,

    /// Receives a line for people about the call being run (see [`Tool::activity`]).
    activity: tokio::sync::watch::Sender<Option<String>>,
}

impl<'a> BotToolExecutor<'a> {
    pub fn new(
        bot: &'a Bot,
        room: Room,
        requester: OwnedUserId,
        trigger_event_id: OwnedEventId,
        tools: Vec<&'static dyn Tool>,
        activity: tokio::sync::watch::Sender<Option<String>>,
    ) -> Self {
        Self {
            bot,
            room,
            requester,
            trigger_event_id,
            tools,
            activity,
        }
    }

    /// Builds an authorizer from the current configuration, which may have changed since the reply started.
    async fn current_authorizer(&self) -> anyhow::Result<Authorizer> {
        let global_config = self.bot.global_config().await?;

        let room_config = self
            .bot
            .room_config_manager()
            .lock()
            .await
            .get_or_create_for_room(&self.room)
            .await?;

        let power_levels = room_power_levels(&self.room).await;
        let known_power_levels = power_levels.is_some();

        let authorizer = Authorizer::new(self.bot.admin_pattern_regexes(), &global_config)
            .for_room(&room_config.access.rules, power_levels);

        // Without power levels, rules with a power-level condition don't match, which would skip such denies.
        if !known_power_levels && authorizer.has_power_level_rules() {
            anyhow::bail!("The room's power levels are unknown");
        }

        Ok(authorizer)
    }
}

/// Tells whether a tool may run for someone, according to the current access rules.
///
/// Tools are offered according to the rules when the reply started, which may have changed since.
fn may_run(authorizer: &Authorizer, user_id: &UserId, tool: &ToolInfo) -> bool {
    authorizer.allows(user_id, Permission::TextGeneration) && authorizer.allows_tool(user_id, tool)
}

impl ToolExecutor for BotToolExecutor<'_> {
    async fn execute(&self, call: &ToolCall, max_output_bytes: usize) -> ToolResult {
        // The loop only passes calls of offered tools, but be strict anyway.
        let Some(tool) = self
            .tools
            .iter()
            .copied()
            .find(|tool| registry::function_name(tool.info().id) == call.name)
        else {
            return ToolResult::error(call, format!("Unknown tool: {}", call.name));
        };

        let tool_id = tool.info().id;

        // The tool's time limit also covers checking access (which may wait for configuration updates).
        let timeout = tool.timeout();
        let deadline = tokio::time::Instant::now() + timeout;
        let timed_out = || {
            tracing::warn!(tool = tool_id, ?timeout, "Tool call timed out");
            ToolResult::error(
                call,
                format!(
                    "The tool did not finish within {} seconds. Answer with the information you already have.",
                    timeout.as_secs()
                ),
            )
        };

        let Ok(authorizer) = tokio::time::timeout_at(deadline, self.current_authorizer()).await
        else {
            return timed_out();
        };

        let authorizer = match authorizer {
            Ok(authorizer) => authorizer,
            Err(err) => {
                tracing::warn!(tool = tool_id, ?err, "Failed to check tool access");
                return ToolResult::error(call, ACCESS_CHECK_FAILED_MESSAGE);
            }
        };

        if !may_run(&authorizer, &self.requester, tool.info()) {
            tracing::info!(
                tool = tool_id,
                "Not running a tool which the requester may no longer use"
            );
            return ToolResult::error(call, NOT_ALLOWED_MESSAGE);
        }

        self.activity
            .send_replace(Some(tool.activity(&call.arguments)));

        let context = ToolContext {
            room: self.room.clone(),
            bot_user_id: self.bot.user_id().to_owned(),
            trigger_event_id: self.trigger_event_id.clone(),
            authorizer,
        };

        let outcome = tokio::time::timeout_at(
            deadline,
            tool.call(&context, &call.arguments, max_output_bytes),
        )
        .await;

        // The model works on the result now.
        self.activity.send_replace(None);

        match outcome {
            Ok(Ok(output)) => ToolResult::success(call, output),
            Ok(Err(message)) => ToolResult::error(call, message),
            Err(_) => timed_out(),
        }
    }
}

#[cfg(test)]
mod tests {
    use mxlink::matrix_sdk::ruma::UserId;

    use super::may_run;
    use crate::authorization::{AccessRule, Authorizer, RuleEffect};
    use crate::entity::globalconfig::GlobalConfig;
    use crate::tools::registry;

    fn authorizer(rules: &[(RuleEffect, &str)]) -> Authorizer {
        let mut global_config = GlobalConfig::new(vec![]);
        global_config.access.rules = Some(
            rules
                .iter()
                .enumerate()
                .map(|(index, (effect, permission))| AccessRule {
                    id: format!("r{index}"),
                    effect: effect.clone(),
                    permissions: vec![permission.to_string()],
                    users: vec!["@*:example.com".to_owned()],
                    min_power_level: None,
                })
                .collect(),
        );

        Authorizer::new(&[], &global_config)
    }

    #[test]
    fn rules_changed_after_offering_stop_the_tool() {
        let alice = UserId::parse("@alice:example.com").unwrap();
        let tool = registry::tools().next().unwrap().info();

        // When the reply started.
        let before = authorizer(&[(RuleEffect::Allow, "use")]);
        assert_eq!(registry::offer(&before, &alice).offered.len(), 1);
        assert!(may_run(&before, &alice, tool));

        // A deny added while the model was working.
        let tool_denied = authorizer(&[
            (RuleEffect::Allow, "use"),
            (RuleEffect::Deny, "tool:builtin.room.history"),
        ]);
        assert!(!may_run(&tool_denied, &alice, tool));

        // No longer allowed to chat at all.
        let chat_denied = authorizer(&[
            (RuleEffect::Allow, "use"),
            (RuleEffect::Deny, "text-generation"),
        ]);
        assert!(!may_run(&chat_denied, &alice, tool));
    }
}
