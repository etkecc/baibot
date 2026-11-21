//! Running the tools the model calls while answering a message.

use mxlink::MessageResponseType;
use mxlink::matrix_sdk::Room;
use mxlink::matrix_sdk::ruma::{OwnedEventId, OwnedUserId};

use crate::Bot;
use crate::agent::{AgentPurpose, utils::get_effective_agent_for_purpose};
use crate::authorization::{Authorizer, room_power_levels};
use crate::entity::RoomConfigContext;
use crate::tools::registry::{self, PostedMessages, RoomCapabilities, Tool, ToolContext};
use crate::tools::{ToolCall, ToolExecutor, ToolResult};

/// Determines what the room's agents can do for tools.
pub async fn room_capabilities(
    bot: &Bot,
    room_config_context: &RoomConfigContext,
) -> RoomCapabilities {
    let image_agent = get_effective_agent_for_purpose(
        bot.agent_manager(),
        room_config_context,
        AgentPurpose::ImageGeneration,
    )
    .await;

    RoomCapabilities {
        image_generation: image_agent.is_ok(),
    }
}

const NOT_ALLOWED_MESSAGE: &str = "The person you are answering may no longer use this tool (access rules changed). Answer without it.";

const ACCESS_CHECK_FAILED_MESSAGE: &str =
    "Failed to check whether this tool may be used. Answer without it.";

/// Runs the tools offered for a reply, on behalf of the person being answered.
pub struct BotToolExecutor<'a> {
    bot: &'a Bot,
    room: Room,
    requester: OwnedUserId,
    trigger_event_id: OwnedEventId,
    response_type: MessageResponseType,
    tools: Vec<&'static dyn Tool>,
    posted_messages: PostedMessages,

    /// Receives a line for people about the call being run (see [`Tool::activity`]).
    activity: tokio::sync::watch::Sender<Option<String>>,
}

impl<'a> BotToolExecutor<'a> {
    pub fn new(
        bot: &'a Bot,
        room: Room,
        requester: OwnedUserId,
        trigger_event_id: OwnedEventId,
        response_type: MessageResponseType,
        tools: Vec<&'static dyn Tool>,
        activity: tokio::sync::watch::Sender<Option<String>>,
    ) -> Self {
        Self {
            bot,
            room,
            requester,
            trigger_event_id,
            response_type,
            tools,
            posted_messages: PostedMessages::default(),
            activity,
        }
    }

    /// The last message tools posted while answering, if any.
    pub fn last_posted_message(&self) -> Option<OwnedEventId> {
        self.posted_messages.last()
    }

    /// Builds an authorizer (and the room's configuration) from the current configuration,
    /// which may have changed since the reply started.
    async fn current_authorizer(&self) -> anyhow::Result<(Authorizer, RoomConfigContext)> {
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

        Ok((
            authorizer,
            RoomConfigContext::new(global_config, room_config),
        ))
    }
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

        let (authorizer, room_config_context) = match authorizer {
            Ok(result) => result,
            Err(err) => {
                tracing::warn!(tool = tool_id, ?err, "Failed to check tool access");
                return ToolResult::error(call, ACCESS_CHECK_FAILED_MESSAGE);
            }
        };

        // Tools are offered according to the rules when the reply started, which may have changed since.
        if !registry::may_run(&authorizer, &self.requester, tool) {
            tracing::info!(
                tool = tool_id,
                "Not running a tool which the requester may no longer use"
            );
            return ToolResult::error(call, NOT_ALLOWED_MESSAGE);
        }

        self.activity
            .send_replace(Some(tool.activity(&call.arguments)));

        let context = ToolContext {
            bot: self.bot.clone(),
            room: self.room.clone(),
            bot_user_id: self.bot.user_id().to_owned(),
            trigger_event_id: self.trigger_event_id.clone(),
            response_type: self.response_type.clone(),
            posted_messages: self.posted_messages.clone(),
            deadline,
            authorizer,
            room_config_context,
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
