use mxlink::matrix_sdk::ruma::OwnedEventId;
use mxlink::MatrixLink;

use super::llm::{convert_matrix_message_to_llm_message, Conversation, Message};
use super::matrix::{
    get_matrix_messages_in_thread, process_matrix_messages_in_thread, MatrixMessageProcessingParams,
};

pub async fn create_llm_conversation_for_matrix_thread(
    matrix_link: MatrixLink,
    room: &mxlink::matrix_sdk::Room,
    thread_id: OwnedEventId,
    params: &MatrixMessageProcessingParams,
) -> Result<Conversation, mxlink::matrix_sdk::Error> {
    let messages = get_matrix_messages_in_thread(matrix_link, room, thread_id).await?;

    let messages_filtered = process_matrix_messages_in_thread(&messages, params).await;

    let mut llm_messages: Vec<Message> = Vec::new();

    for matrix_message in messages_filtered {
        let Some(llm_message) =
            convert_matrix_message_to_llm_message(&matrix_message, &params.bot_user_id)
        else {
            continue;
        };

        llm_messages.push(llm_message);
    }

    Ok(Conversation {
        messages: llm_messages,
    })
}
