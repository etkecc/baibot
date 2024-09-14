use mxlink::matrix_sdk::{
    ruma::{
        events::{
            room::message::Relation, AnyMessageLikeEvent, AnySyncTimelineEvent, AnyTimelineEvent,
            MessageLikeEvent,
        },
        OwnedEventId, OwnedUserId,
    },
    Room,
};

use mxlink::CallbackError;
use mxlink::ThreadInfo;

use tracing::Instrument;

use crate::entity::{MessageContext, MessagePayload, RoomConfigContext, TriggerEventInfo};

#[derive(Clone)]
pub struct Reacting {
    bot: super::Bot,
}

impl Reacting {
    pub fn new(bot: super::Bot) -> Self {
        Self { bot }
    }

    pub async fn react_no_fail(
        &self,
        room: &Room,
        target_event_id: OwnedEventId,
        reaction_key: String,
    ) -> Option<mxlink::matrix_sdk::ruma::api::client::message::send_message_event::v3::Response>
    {
        let result = self
            .bot
            .matrix_link()
            .reacting()
            .react(room, target_event_id.clone(), reaction_key)
            .await;

        match result {
            Ok(result) => Some(result),
            Err(err) => {
                tracing::error!(
                    "Failed to send reaction to {} in room {:?}: {:?}",
                    target_event_id,
                    room.room_id(),
                    err
                );
                None
            }
        }
    }

    pub(super) async fn attach_event_handlers(&self) {
        let matrix_link_reacting = self.bot.matrix_link().reacting();

        let this = self.clone();
        matrix_link_reacting.on_actionable_reaction(
            |event, room, reaction_event_content| async move {
                this.on_actionable_reaction(event, room, reaction_event_content)
                    .await
            },
        );
    }

    #[tracing::instrument(name = "bot_on_actionable_reaction", skip_all, fields(room_id = room.room_id().as_str(), event_id = event.event_id().as_str()))]
    async fn on_actionable_reaction(
        &self,
        event: AnySyncTimelineEvent,
        room: Room,
        reaction_event_content: mxlink::matrix_sdk::ruma::events::reaction::ReactionEventContent,
    ) -> Result<(), CallbackError> {
        if self
            .bot
            .is_caught_up(event.origin_server_ts())
            .await
            .map_err(|e| {
                CallbackError::Unknown(
                    format!("Failed to determine catch-up state: {:?}", e).into(),
                )
            })?
        {
            tracing::debug!(
                event_origin_server_ts = format!("{:?}", event.origin_server_ts()),
                "Ignoring old reaction event",
            );

            return Ok(());
        }

        tracing::info!("Handling reaction");

        let global_config = self
            .bot
            .global_config()
            .await
            .map_err(|err| CallbackError::Unknown(err.into()))?;

        tracing::trace!(?global_config, "Global config");

        let trigger_event_sender_is_admin =
            mxidwc::match_user_id(event.sender().as_str(), self.bot.admin_pattern_regexes());

        let trigger_event_sender_is_allowed_user = match &global_config.access.user_patterns {
            Some(user_patterns) => {
                let allowed_user_regexes = mxidwc::parse_patterns_vector(user_patterns)
                    .map_err(|err| CallbackError::Unknown(err.into()))?;

                mxidwc::match_user_id(event.sender().as_str(), &allowed_user_regexes)
            }
            None => false,
        };

        if !trigger_event_sender_is_admin && !trigger_event_sender_is_allowed_user {
            tracing::debug!("Ignoring reaction from non-admin/non-allowed user");
            return Ok(());
        }

        let reacted_to_event_id = &reaction_event_content.relates_to.event_id;

        let reacted_to_event = self
            .bot
            .room_event_fetcher()
            .fetch_event_in_room(reacted_to_event_id, &room)
            .await;

        let reacted_to_event = match reacted_to_event {
            Ok(value) => value,
            Err(err) => {
                tracing::error!(
                    ?reacted_to_event_id,
                    ?err,
                    "Failed to fetch reacted-to event",
                );
                return Ok(());
            }
        };

        let reacted_to_event_any_timeline_event = match reacted_to_event.event.deserialize() {
            Ok(value) => value,
            Err(err) => {
                tracing::error!(
                    ?reacted_to_event_id,
                    ?err,
                    "Failed to deserialize reacted-to event event",
                );
                return Ok(());
            }
        };

        let reacted_to_event_sender_id: OwnedUserId =
            reacted_to_event_any_timeline_event.sender().to_owned();

        let AnyTimelineEvent::MessageLike(reacted_to_event_message_like) =
            reacted_to_event_any_timeline_event
        else {
            tracing::debug!(
                ?reacted_to_event_id,
                "Ignoring non-MessageLike reacted-to event",
            );
            return Ok(());
        };

        let AnyMessageLikeEvent::RoomMessage(reacted_to_event_room_message) =
            reacted_to_event_message_like
        else {
            tracing::debug!(
                ?reacted_to_event_id,
                "Ignoring non-RoomMessage reacted-to event",
            );
            return Ok(());
        };

        let MessageLikeEvent::Original(reacted_to_event_room_message_original) =
            reacted_to_event_room_message
        else {
            tracing::debug!(?reacted_to_event_id, "Ignoring redacted reacted-to event",);
            return Ok(());
        };

        let reacted_to_event_payload: Result<MessagePayload, String> =
            reacted_to_event_room_message_original
                .content
                .msgtype
                .clone()
                .try_into();
        let Ok(reacted_to_event_payload) = reacted_to_event_payload else {
            tracing::debug!(
                msg_type = reacted_to_event_room_message_original.content.msgtype(),
                "Ignoring reaction to message of unknown type",
            );
            return Ok(());
        };

        let thread_root_event_id = match reacted_to_event_room_message_original.content.relates_to {
            Some(relation) => {
                if let Relation::Thread(thread_id) = relation {
                    thread_id.event_id.clone()
                } else {
                    reacted_to_event_id.clone()
                }
            }
            None => reacted_to_event_id.clone(),
        };

        let thread_info = ThreadInfo::new(thread_root_event_id, reacted_to_event_id.clone());

        let room_config = self
            .bot
            .room_config_manager()
            .lock()
            .await
            .get_or_create_for_room(&room)
            .await
            .map_err(|err| CallbackError::Unknown(err.into()))?;

        tracing::trace!(?room_config, "Room config");

        let room_config_context =
            RoomConfigContext::new(global_config.clone(), room_config.clone());

        let trigger_event_info = TriggerEventInfo::new(
            event.event_id().to_owned(),
            event.sender().to_owned(),
            MessagePayload::Reaction {
                key: reaction_event_content.relates_to.key,
                reacted_to_event_payload: Box::new(reacted_to_event_payload),
                reacted_to_event_id: reaction_event_content.relates_to.event_id.clone(),
                reacted_to_event_sender_id,
            },
            trigger_event_sender_is_admin,
        );

        let message_context = MessageContext::new(
            room,
            room_config_context,
            self.bot.admin_pattern_regexes().clone(),
            trigger_event_info,
            thread_info,
        );

        tracing::info!("Handling reaction via reaction controller");

        let event_span = tracing::error_span!("reaction_controller");

        crate::controller::reaction::handle(
            &self.bot,
            self.bot.matrix_link().clone(),
            &message_context,
        )
        .instrument(event_span)
        .await
        .map_err(|err| CallbackError::Unknown(err.into()))?;

        self.bot.catch_up(event.origin_server_ts()).await;

        Ok(())
    }
}
