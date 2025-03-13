use std::sync::Arc;

use mxlink::MatrixLink;
use mxlink::matrix_sdk::ruma::OwnedEventId;

use crate::conversation::matrix::MatrixMessage;

use super::llm::{Conversation, Message, convert_matrix_message_to_llm_message};
use super::matrix::{
    MatrixMessageProcessingParams, RoomEventFetcher, get_matrix_messages_in_reply_chain,
    get_matrix_messages_in_thread, process_matrix_messages,
};

pub async fn create_llm_conversation_for_matrix_thread(
    matrix_link: MatrixLink,
    room: &mxlink::matrix_sdk::Room,
    thread_id: OwnedEventId,
    params: &MatrixMessageProcessingParams,
) -> Result<Conversation, mxlink::matrix_sdk::Error> {
    let messages = get_matrix_messages_in_thread(matrix_link, room, thread_id).await?;

    let llm_messages = filter_messages_and_convert_to_llm_messages(messages, params).await;

    Ok(Conversation {
        messages: llm_messages,
    })
}

pub async fn create_llm_conversation_for_matrix_reply_chain(
    event_fetcher: &Arc<RoomEventFetcher>,
    room: &mxlink::matrix_sdk::Room,
    event_id: OwnedEventId,
    params: &MatrixMessageProcessingParams,
) -> Result<Conversation, mxlink::matrix_sdk::Error> {
    let messages = get_matrix_messages_in_reply_chain(event_fetcher, room, event_id).await?;

    let llm_messages = filter_messages_and_convert_to_llm_messages(messages, params).await;

    Ok(Conversation {
        messages: llm_messages,
    })
}

async fn filter_messages_and_convert_to_llm_messages(
    messages: Vec<MatrixMessage>,
    params: &MatrixMessageProcessingParams,
) -> Vec<Message> {
    let messages_filtered = process_matrix_messages(&messages, params).await;

    let mut llm_messages: Vec<Message> = Vec::new();

    for matrix_message in messages_filtered {
        let Some(llm_message) =
            convert_matrix_message_to_llm_message(&matrix_message, &params.bot_user_id)
        else {
            continue;
        };

        llm_messages.push(llm_message);
    }

    llm_messages
}
