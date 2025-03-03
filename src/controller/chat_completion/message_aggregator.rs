use std::{
    collections::HashMap,
    sync::Arc,
    thread,
    time::{self, Duration, Instant},
};

use matrix_sdk::ruma::events::room::message::TextMessageEventContent;
use tokio::sync::Mutex;

use mxlink::MatrixLink;
use serde::Deserialize;

use super::handle_message;

use crate::{
    entity::{MessageContext, MessagePayload}, repository::Answer, Bot
};

use super::ChatCompletionControllerType;

#[derive(Debug, Deserialize, Clone)]
pub struct ConfigChatCompletionAggregator {
    pub message_expiration_seconds: u64,
    pub message_polling_interval_seconds: u64,
}

struct Param {
    received_timestamp: time::Instant,
    bot: Bot,
    matrix_link: MatrixLink,
    message_context: MessageContext,
    controller_type: ChatCompletionControllerType,
}

impl Param {
    pub fn new(
        bot: Bot,
        matrix_link: MatrixLink,
        message_context: MessageContext,
        controller_type: ChatCompletionControllerType,
    ) -> Self {
        let received_timestamp = Instant::now();

        Self {
            bot,
            matrix_link,
            message_context,
            controller_type,
            received_timestamp,
        }
    }
}

pub struct MessageAggregator {
    messages: Arc<Mutex<HashMap<String, Param>>>,
    config: ConfigChatCompletionAggregator,
}

impl MessageAggregator {
    pub fn new(config: ConfigChatCompletionAggregator) -> Self {
        Self {
            messages: Arc::new(Mutex::new(HashMap::new())),
            config: config,
        }
    }

    async fn handle(&self, k: String, param: Param) {
        let mut messages = self.messages.lock().await;
        let existans = messages.get_mut(&k);

        match existans {
            None => {
                messages.insert(k, param);
            }
            Some(p) => {
                if let MessagePayload::Text(t) = p.message_context.payload() {
                    let mut payload = t.body.clone();

                    if let MessagePayload::Text(pt) = param.message_context.payload() {
                        payload.push('\n');
                        payload.push_str(&pt.body);
                    }

                    p.message_context.set_payload(MessagePayload::Text(
                        TextMessageEventContent::plain(payload),
                    ));

                    p.received_timestamp = Instant::now();
                }
            }
        };
    }

    pub async fn listen(&self) {
        loop {
            let now = time::Instant::now();
            let ten_seconds_ago = now - Duration::from_secs(self.config.message_expiration_seconds);

            // to unlock mutex
            {
                let mut messages = self.messages.lock().await;
                let mut to_remove = Vec::new();

                for (k, p) in messages.iter() {
                    if p.received_timestamp < ten_seconds_ago {
                        to_remove.push(k.clone());
                        self.send_to_chat_completion_controller(p).await;
                    }
                }

                for k in to_remove {
                    messages.remove(&k);
                }
            }

            thread::sleep(Duration::from_secs(
                self.config.message_polling_interval_seconds,
            ));
        }
    }

    async fn send_to_chat_completion_controller(&self, p: &Param) {
        if let MessagePayload::Text(t) = p.message_context.payload() {
            
            let _ = p.bot.repository().store_answer(
                Answer{
                    id: 0,
                    bot_id: p.bot.bot_uniqe_id(),
                    length: t.body.split_whitespace().count() as i64,
                    stored_at: chrono::Utc::now().date_naive().to_string()
                }
            ).await;
        }

        let _ = handle_message(
            &p.bot,
            p.matrix_link.clone(),
            &p.message_context,
            &p.controller_type,
        )
        .await;
    }
}

pub async fn handle(
    bot: &Bot,
    matrix_link: MatrixLink,
    message_context: &MessageContext,
    controller_type: &ChatCompletionControllerType,
) -> anyhow::Result<()> {
    let k = message_context.room_id().to_string();

    let param = Param::new(
        bot.clone(),
        matrix_link.clone(),
        message_context.clone(),
        controller_type.clone(),
    );

    let _ = bot
        .chat_completion_message_aggregator()
        .handle(k, param)
        .await;

    Ok(())
}
