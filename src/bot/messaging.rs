use mxlink::matrix_sdk::{
    ruma::{
        api::client::receipt::create_receipt::v3::ReceiptType,
        events::room::message::OriginalSyncRoomMessageEvent, OwnedEventId,
    },
    Room,
};

use mxlink::{CallbackError, MessageResponseType};

use tracing::Instrument;

use crate::{
    conversation::matrix::determine_thread_context_for_room_event,
    entity::{MessageContext, MessagePayload, RoomConfigContext, TriggerEventInfo},
};

#[derive(Clone)]
pub struct Messaging {
    bot: super::Bot,
}

impl Messaging {
    pub fn new(bot: super::Bot) -> Self {
        Self { bot }
    }

    pub async fn send_text_markdown_no_fail(
        &self,
        room: &Room,
        message: String,
        response_type: MessageResponseType,
    ) -> Option<mxlink::matrix_sdk::ruma::api::client::message::send_message_event::v3::Response>
    {
        let result = self
            .bot
            .matrix_link()
            .messaging()
            .send_text_markdown(room, message, response_type)
            .await;

        match result {
            Ok(result) => Some(result),
            Err(err) => {
                tracing::error!(
                    room_id = format!("{:?}", room.room_id()),
                    ?err,
                    "Failed to send text message to room",
                );
                None
            }
        }
    }

    pub async fn send_notice_markdown_no_fail(
        &self,
        room: &Room,
        message: String,
        response_type: MessageResponseType,
    ) -> Option<mxlink::matrix_sdk::ruma::api::client::message::send_message_event::v3::Response>
    {
        let result = self
            .bot
            .matrix_link()
            .messaging()
            .send_notice_markdown(room, message, response_type)
            .await;

        match result {
            Ok(result) => Some(result),
            Err(err) => {
                tracing::error!(
                    room_id = format!("{:?}", room.room_id()),
                    ?err,
                    "Failed to send notice message to room",
                );
                None
            }
        }
    }

    pub async fn send_tooltip_markdown_no_fail(
        &self,
        room: &Room,
        message: &str,
        response_type: MessageResponseType,
    ) -> Option<mxlink::matrix_sdk::ruma::api::client::message::send_message_event::v3::Response>
    {
        self.send_notice_markdown_no_fail(
            room,
            crate::utils::status::create_tooltip_message_text(message),
            response_type,
        )
        .await
    }

    pub async fn send_success_markdown_no_fail(
        &self,
        room: &Room,
        message: &str,
        response_type: MessageResponseType,
    ) -> Option<mxlink::matrix_sdk::ruma::api::client::message::send_message_event::v3::Response>
    {
        self.send_notice_markdown_no_fail(
            room,
            crate::utils::status::create_success_message_text(message),
            response_type,
        )
        .await
    }

    pub async fn send_error_markdown_no_fail(
        &self,
        room: &Room,
        err: &str,
        response_type: MessageResponseType,
    ) -> Option<mxlink::matrix_sdk::ruma::api::client::message::send_message_event::v3::Response>
    {
        self.send_notice_markdown_no_fail(
            room,
            crate::utils::status::create_error_message_text(err),
            response_type,
        )
        .await
    }

    pub async fn redact_event_no_fail(
        &self,
        room: &Room,
        target_event_id: OwnedEventId,
        reason: Option<String>,
    ) -> Option<mxlink::matrix_sdk::ruma::api::client::redact::redact_event::v3::Response> {
        let result = self
            .bot
            .matrix_link()
            .messaging()
            .redact_event(room, target_event_id.clone(), reason)
            .await;

        match result {
            Ok(result) => Some(result),
            Err(err) => {
                tracing::error!(
                    room_id = format!("{:?}", room.room_id()),
                    ?target_event_id,
                    ?err,
                    "Failed to send redaction to room",
                );

                None
            }
        }
    }

    pub(super) async fn attach_event_handlers(&self) {
        let matrix_link_messaging = self.bot.matrix_link().messaging();

        let this = self.clone();
        matrix_link_messaging.on_actionable_room_message(|event, room| async move {
            this.on_actionable_message(event, room).await
        });
    }

    #[tracing::instrument(name = "bot_on_actionable_message", skip_all, fields(room_id = room.room_id().as_str(), event_id = event.event_id.as_str()))]
    async fn on_actionable_message(
        &self,
        event: OriginalSyncRoomMessageEvent,
        room: Room,
    ) -> Result<(), CallbackError> {
        if self
            .bot
            .is_caught_up(event.origin_server_ts)
            .await
            .map_err(|e| {
                CallbackError::Unknown(
                    format!("Failed to determine catch-up state: {:?}", e).into(),
                )
            })?
        {
            tracing::debug!(
                event_origin_server_ts = format!("{:?}", event.origin_server_ts),
                "Ignoring old message event",
            );

            return Ok(());
        }

        tracing::info!("Processing message");

        let global_config = self
            .bot
            .global_config()
            .await
            .map_err(|err| CallbackError::Unknown(err.into()))?;

        tracing::trace!(?global_config, "Global config");

        let room_config = self
            .bot
            .room_config_manager()
            .lock()
            .await
            .get_or_create_for_room(&room)
            .await
            .map_err(|err| CallbackError::Unknown(err.into()))?;

        tracing::trace!(?room_config, "Room config");

        let trigger_event_sender_is_admin = mxidwc::match_user_id(
            event.sender.clone().as_str(),
            self.bot.admin_pattern_regexes(),
        );

        let trigger_event_sender_is_allowed_user = match &global_config.access.user_patterns {
            Some(user_patterns) => {
                let allowed_user_regexes = mxidwc::parse_patterns_vector(user_patterns)
                    .map_err(|err| CallbackError::Unknown(err.into()))?;

                mxidwc::match_user_id(event.sender.clone().as_str(), &allowed_user_regexes)
            }
            None => false,
        };

        if !trigger_event_sender_is_admin && !trigger_event_sender_is_allowed_user {
            tracing::debug!("Ignoring message from non-admin/non-allowed user");
            return Ok(());
        }

        let payload: Result<MessagePayload, String> = event.content.msgtype.clone().try_into();
        let payload = match payload {
            Ok(payload) => payload,
            Err(err) => {
                tracing::debug!(
                    msg_type = event.content.msgtype(),
                    ?err,
                    "Ignoring message not supported by us",
                );
                return Ok(());
            }
        };

        let thread_context = determine_thread_context_for_room_event(
            self.bot.user_id(),
            &room,
            &event,
            &payload,
            &self.bot.room_event_fetcher(),
        )
        .await;

        let thread_context = match thread_context {
            Ok(value) => value,
            Err(err) => {
                tracing::error!(?err, "Failed to determine thread context for event");
                return Ok(());
            }
        };

        let Some(thread_context) = thread_context else {
            tracing::debug!("Ignoring message with unknown thread context (likely not a threaded message or a top-level message)");
            return Ok(());
        };

        let room_config_context =
            RoomConfigContext::new(global_config.clone(), room_config.clone());

        let trigger_event_info = TriggerEventInfo::new(
            event.event_id.clone(),
            event.sender.clone(),
            payload,
            trigger_event_sender_is_admin,
        );

        let message_context = MessageContext::new(
            room.clone(),
            room_config_context,
            self.bot.admin_pattern_regexes().clone(),
            trigger_event_info,
            thread_context.info.clone(),
        );

        let bot_display_name = self
            .bot
            .room_display_name_fetcher()
            .own_display_name_in_room(message_context.room())
            .await;

        let bot_display_name = match bot_display_name {
            Ok(value) => value,
            Err(err) => {
                tracing::warn!(
                    ?err,
                    "Failed to fetch bot display name. Proceeding without it"
                );
                None
            }
        };

        // The first event in the thread determines which handler processes the current event.
        let controller_type = crate::controller::determine_controller(
            self.bot.command_prefix(),
            &thread_context.first_message,
            &message_context,
            self.bot.user_id(),
            &bot_display_name,
        );

        tracing::info!(?controller_type, "Determined controller");

        let _ = room
            .send_single_receipt(
                ReceiptType::Read,
                thread_context.info.clone().into(),
                event.event_id.clone(),
            )
            .await;

        let start_time = std::time::Instant::now();

        let event_span = tracing::error_span!("message_controller", ?controller_type);

        crate::controller::dispatch_controller(&controller_type, &message_context, &self.bot)
            .instrument(event_span)
            .await;

        let duration = std::time::Instant::now().duration_since(start_time);

        tracing::debug!(?duration, "Controller finished");

        self.bot.catch_up(event.origin_server_ts).await;

        return Ok(());
    }
}
