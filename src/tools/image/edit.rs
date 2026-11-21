use std::time::Duration;

use mxlink::matrix_sdk::ruma::events::room::message::MessageType;
use mxlink::matrix_sdk::ruma::events::{
    AnySyncMessageLikeEvent, AnySyncTimelineEvent, SyncMessageLikeEvent,
};
use mxlink::matrix_sdk::ruma::{OwnedEventId, OwnedUserId, UserId};
use tracing::Instrument;

use crate::agent::ControllerTrait;
use crate::agent::provider::{ImageEditParams, ImageSource};
use crate::authorization::Permission;
use crate::conversation::llm::ImageDetails;
use crate::conversation::matrix::{
    MatrixMessageContent, conversation_message_content,
    convert_matrix_native_event_to_matrix_message,
};
use crate::tools::catalog::ToolInfo;
use crate::tools::registry::{Capability, Tool, ToolCallFuture, ToolContext};

use super::{
    TIMEOUT, generate, image_agent, parse_arguments, post_image, posted_result, validate_prompt,
};

pub static INFO: ToolInfo = ToolInfo {
    id: super::EDIT_IMAGE_ID,
    emoji: "🎨",
    summary: "edit images",
    default_enabled: true,
};

/// The most images a call may work with.
const MAX_IMAGES: usize = 4;

const DESCRIPTION: &str = "Edits images and posts the result in this chat room, along with your answer. Use it when the person asks to change, restyle or combine existing images: ones in this conversation (like an image they posted or replied to, or one you created) or, by event ID, other images in this room (e.g. found with the room history). Without `image_event_ids`, it edits the most recent image in the conversation (or combines the last `conversation_images` images). The image model doesn't see the conversation: describe the result completely (e.g. \"make the cat blue, keep everything else the same\"). Each call may cost money: only call it again when the person asks for more images. After it succeeds, answer briefly (the person sees the image).";

/// An image from the conversation the bot is answering.
#[derive(Clone)]
pub struct ConversationImage {
    pub sender_id: Option<OwnedUserId>,
    pub image: ImageDetails,
}

/// The tool which edits images.
pub struct EditImageTool;

impl Tool for EditImageTool {
    fn info(&self) -> &'static ToolInfo {
        &INFO
    }

    fn description(&self) -> &'static str {
        DESCRIPTION
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "prompt": {
                    "type": "string",
                    "description": "A complete description of the result (what to change, and what to keep).",
                },
                "image_event_ids": {
                    "type": "array",
                    "items": {"type": "string"},
                    "minItems": 1,
                    "maxItems": MAX_IMAGES,
                    "description": "Event IDs of images in this room to edit (e.g. from the room history). Leave out to use images from the conversation.",
                },
                "conversation_images": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": MAX_IMAGES,
                    "description": "How many of the conversation's most recent images to use (default 1). Only without `image_event_ids`.",
                },
            },
            "required": ["prompt"],
            "additionalProperties": false,
        })
    }

    fn required_permission(&self) -> Option<Permission> {
        Some(Permission::ImageGeneration)
    }

    fn required_capability(&self) -> Option<Capability> {
        Some(Capability::ImageEditing)
    }

    fn timeout(&self) -> Duration {
        TIMEOUT
    }

    fn activity(&self, arguments: &str) -> String {
        activity(arguments)
    }

    fn call<'a>(
        &'a self,
        context: &'a ToolContext,
        arguments: &'a str,
        max_output_bytes: usize,
    ) -> ToolCallFuture<'a> {
        Box::pin(async move {
            let request = EditRequest::parse(arguments)?;

            let agent = image_agent(context).await?;

            let max_images = agent.controller().image_edit_max_images();
            if max_images == 0 {
                return Err("This room's image agent can't edit images.".to_owned());
            }
            if request.sources.count() > max_images {
                return Err(format!(
                    "This room's image agent can edit at most {max_images} image(s) at once."
                ));
            }

            // Images from people denied image generation are not sent to the image provider.
            let may_use = |sender_id: &UserId| {
                sender_id == context.bot_user_id
                    || !context
                        .authorizer
                        .denies(sender_id, Permission::ImageGeneration)
            };

            let images: Vec<ImageSource> = match &request.sources {
                Sources::Conversation(count) => {
                    select_conversation_images(&context.conversation_images(), *count, may_use)?
                        .into_iter()
                        .map(ImageSource::from)
                        .collect()
                }
                Sources::Events(event_ids) => {
                    let mut images = Vec::with_capacity(event_ids.len());
                    for event_id in event_ids {
                        images.push(fetch_image(context, event_id, may_use).await?);
                    }
                    images
                }
            };

            let span =
                tracing::debug_span!("image_edit_tool", agent_id = agent.identifier().as_string());

            let response = generate(
                context,
                &agent,
                "editing an image",
                agent
                    .controller()
                    .create_image_edit(&request.prompt, images, ImageEditParams::default())
                    .instrument(span),
            )
            .await?;

            post_image(
                context,
                &response.mime_type,
                response.bytes,
                "generated-image-edit",
            )
            .await?;

            Ok(posted_result(
                "Edited the image",
                &request.prompt,
                None,
                max_output_bytes,
            ))
        })
    }
}

/// Which images to edit.
#[derive(Debug, PartialEq)]
enum Sources {
    /// The conversation's most recent images (this many).
    Conversation(usize),

    /// Images in the room, by event ID.
    Events(Vec<OwnedEventId>),
}

impl Sources {
    fn count(&self) -> usize {
        match self {
            Sources::Conversation(count) => *count,
            Sources::Events(event_ids) => event_ids.len(),
        }
    }
}

#[derive(Debug, PartialEq)]
struct EditRequest {
    prompt: String,
    sources: Sources,
}

impl EditRequest {
    fn parse(arguments: &str) -> Result<Self, String> {
        #[derive(serde::Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Arguments {
            prompt: String,
            #[serde(default)]
            image_event_ids: Option<Vec<String>>,
            #[serde(default)]
            conversation_images: Option<usize>,
        }

        let arguments: Arguments = parse_arguments(arguments)?;

        let prompt = validate_prompt(&arguments.prompt)?;

        let sources = match (arguments.image_event_ids, arguments.conversation_images) {
            (Some(_), Some(_)) => {
                return Err(
                    "Pass either `image_event_ids` or `conversation_images`, not both.".to_owned(),
                );
            }
            (Some(event_ids), None) => {
                if event_ids.is_empty() || event_ids.len() > MAX_IMAGES {
                    return Err(format!(
                        "`image_event_ids` takes 1 to {MAX_IMAGES} event IDs."
                    ));
                }

                let mut parsed: Vec<OwnedEventId> = Vec::with_capacity(event_ids.len());
                for event_id in event_ids {
                    let event_id = OwnedEventId::try_from(event_id.trim())
                        .map_err(|_| format!("Invalid event ID: {event_id}"))?;

                    if !parsed.contains(&event_id) {
                        parsed.push(event_id);
                    }
                }

                Sources::Events(parsed)
            }
            (None, count) => {
                let count = count.unwrap_or(1);
                if count == 0 || count > MAX_IMAGES {
                    return Err(format!(
                        "`conversation_images` must be between 1 and {MAX_IMAGES}."
                    ));
                }

                Sources::Conversation(count)
            }
        };

        Ok(Self { prompt, sources })
    }
}

/// Picks the conversation's last `count` images, oldest first.
///
/// When one of them may not be used, that's an error (rather than taking an older image instead,
/// which the person didn't mean).
fn select_conversation_images(
    images: &[ConversationImage],
    count: usize,
    may_use: impl Fn(&UserId) -> bool,
) -> Result<Vec<ImageDetails>, String> {
    if images.is_empty() {
        return Err("There is no image in this conversation to edit. Images elsewhere in the room can be used by event ID.".to_owned());
    }

    if images.len() < count {
        return Err(format!(
            "The conversation has only {} image(s).",
            images.len()
        ));
    }

    let selected = &images[images.len() - count..];

    if selected.iter().any(|image| {
        image
            .sender_id
            .as_deref()
            .is_some_and(|sender_id| !may_use(sender_id))
    }) {
        return Err(
            "An image to edit was posted by someone denied image generation, so it can't be used."
                .to_owned(),
        );
    }

    Ok(selected.iter().map(|image| image.image.clone()).collect())
}

/// Fetches an image posted in the room.
async fn fetch_image(
    context: &ToolContext,
    event_id: &OwnedEventId,
    may_use: impl Fn(&UserId) -> bool,
) -> Result<ImageSource, String> {
    let not_found = || format!("No image with event ID {event_id} was found in this room.");

    // Fetched anew (not from the bot's cache), so that a deleted image isn't used.
    let event = context.room.event(event_id, None).await.map_err(|err| {
        tracing::debug!(?err, "Failed to fetch an image for a tool call");
        not_found()
    })?;

    let Ok(AnySyncTimelineEvent::MessageLike(event)) = event.raw().deserialize() else {
        return Err(not_found());
    };

    let AnySyncMessageLikeEvent::RoomMessage(SyncMessageLikeEvent::Original(original)) = &event
    else {
        // Deleted (redacted), not decrypted, or not a message.
        return Err(not_found());
    };

    // Checked before downloading anything, on the same content the conversion below uses (with edits applied).
    let is_image = conversation_message_content(original)
        .is_some_and(|(content, _, _)| matches!(content.msgtype, MessageType::Image(_)));
    if !is_image {
        return Err(format!("The message {event_id} is not an image."));
    }

    if !may_use(event.sender()) {
        return Err(format!(
            "The image {event_id} was posted by someone denied image generation, so it can't be used."
        ));
    }

    let message = convert_matrix_native_event_to_matrix_message(context.bot.matrix_link(), &event)
        .await
        .map_err(|err| {
            tracing::warn!(?err, "Failed to download an image for a tool call");
            format!("Downloading the image {event_id} failed.")
        })?;

    match message.map(|message| message.content) {
        Some(MatrixMessageContent::Image(content, mime_type, bytes)) => {
            let filename = content.filename.clone().unwrap_or(content.body);
            Ok(ImageSource::new(filename, bytes, mime_type))
        }
        _ => Err(format!("The message {event_id} is not an image.")),
    }
}

/// Says what a call does, without repeating text the model wrote.
fn activity(arguments: &str) -> String {
    let count = EditRequest::parse(arguments)
        .map(|request| request.sources.count())
        .unwrap_or(1);

    if count == 1 {
        "🎨 Editing an image".to_owned()
    } else {
        format!("🎨 Combining {count} images")
    }
}

#[cfg(test)]
mod tests;
