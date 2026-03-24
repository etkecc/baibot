use async_openai::types::responses::{
    EasyInputContent, EasyInputMessage, ImageDetail, InputContent, InputFileArgs,
    InputImageContent, InputItem, InputParam, MessageType, Role,
};

use crate::conversation::llm::{
    Author as LLMAuthor, Message as LLMMessage, MessageContent as LLMMessageContent,
};
use crate::utils::base64::base64_encode;

pub fn convert_llm_messages_to_openai_response_input(
    conversation_messages: Vec<LLMMessage>,
) -> InputParam {
    let mut items = Vec::with_capacity(conversation_messages.len());

    for message in conversation_messages {
        let role = match message.author {
            LLMAuthor::Prompt => Role::System,
            LLMAuthor::Assistant => Role::Assistant,
            LLMAuthor::User => Role::User,
        };

        let content = match message.content {
            LLMMessageContent::Text(text) => EasyInputContent::Text(text),
            LLMMessageContent::Image(image_details) => {
                let image_url = format!(
                    "data:{};base64,{}",
                    image_details.mime,
                    base64_encode(&image_details.data)
                );

                EasyInputContent::ContentList(vec![InputContent::InputImage(InputImageContent {
                    image_url: Some(image_url),
                    detail: ImageDetail::Auto,
                    file_id: None,
                })])
            }
            LLMMessageContent::File(file_details) => {
                let file_data = format!(
                    "data:{};base64,{}",
                    file_details.mime,
                    base64_encode(&file_details.data)
                );

                let file_content = InputFileArgs::default()
                    .file_data(file_data)
                    .filename(file_details.filename())
                    .build()
                    .expect("Failed to build InputFileContent");

                EasyInputContent::ContentList(vec![InputContent::InputFile(file_content)])
            }
        };

        items.push(InputItem::EasyMessage(EasyInputMessage {
            r#type: MessageType::Message,
            role,
            content,
            phase: None,
        }));
    }

    InputParam::Items(items)
}
