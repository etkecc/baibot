use std::{
    collections::HashMap,
    sync::{Arc},
    thread,
    time::{self, Duration, Instant},
};

use tokio::sync::Mutex;

use mxlink::MatrixLink;
use serde::Deserialize;

use super::handle_message;

use crate::{entity::MessageContext, Bot};

use super::ChatCompletionControllerType;

#[derive(Debug, Deserialize)]
pub struct ConfigChatCompletionAggregator {
    pub message_expiration_seconds: u16,
    pub message_polling_interval_seconds: u16,
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
        bot:  Bot,
        matrix_link: MatrixLink,
        message_context:  MessageContext,
        controller_type:  ChatCompletionControllerType,
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
}

impl MessageAggregator {
    pub fn new() -> Self {
        Self {
            messages: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    async fn handle(&self, k: String, param: Param) {
        let mut messages = self.messages.lock().await;        
        
        messages.insert(k, param);
    }

    pub async fn listen(&self) {
        loop {
            let now = time::Instant::now();
            let ten_seconds_ago = now - Duration::from_secs(10);

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

            thread::sleep(Duration::from_secs(2));
        }
    }

    async fn send_to_chat_completion_controller(&self, p: &Param) {
        // TODO
        let _ = handle_message(&p.bot, p.matrix_link.clone(), &p.message_context, &p.controller_type).await;
    }
}

pub async fn handle(
    bot: &Bot,
    matrix_link: MatrixLink,
    message_context: &MessageContext,
    controller_type: &ChatCompletionControllerType,
) -> anyhow::Result<()> {
    let k = message_context.room_id().to_string();
        
    let param = Param::new(bot.clone(), matrix_link.clone(), message_context.clone(), controller_type.clone());
    
    //TODO
    let _ = bot.chat_completion_message_aggregator().handle(k, param).await;

    Ok(())
}
