use std::time::Duration;

use tracing::Instrument;

use crate::agent::ControllerTrait;
use crate::agent::provider::ImageGenerationParams;
use crate::authorization::Permission;
use crate::tools::catalog::ToolInfo;
use crate::tools::registry::{Capability, Tool, ToolCallFuture, ToolContext};

use super::{TIMEOUT, generate, image_agent, parse_prompt, post_image, posted_result};

pub static INFO: ToolInfo = ToolInfo {
    id: "builtin.image.generate",
    emoji: "🖌️",
    summary: "create images",
    default_enabled: true,
};

const DESCRIPTION: &str = "Creates an image from a description and posts it in this chat room, along with your answer. Use it when the person asks you to draw, create or generate a picture. The image model doesn't see the conversation: write a complete, detailed description of the image (subject, style, composition, colors), including details from earlier messages that matter. Each call creates one image and may cost money: only call it again when the person asks for more images. After it succeeds, answer briefly (the person sees the image).";

/// The tool which creates images.
pub struct GenerateImageTool;

impl Tool for GenerateImageTool {
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
                    "description": "A complete, detailed description of the image to create.",
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
        Some(Capability::ImageGeneration)
    }

    fn timeout(&self) -> Duration {
        TIMEOUT
    }

    fn activity(&self, _arguments: &str) -> String {
        "🖌️ Creating an image".to_owned()
    }

    fn call<'a>(
        &'a self,
        context: &'a ToolContext,
        arguments: &'a str,
        max_output_bytes: usize,
    ) -> ToolCallFuture<'a> {
        Box::pin(async move {
            let prompt = parse_prompt(arguments)?;

            let agent = image_agent(context).await?;

            let span = tracing::debug_span!(
                "image_generation_tool",
                agent_id = agent.identifier().as_string()
            );

            let response = generate(
                context,
                &agent,
                "creating an image",
                agent
                    .controller()
                    .generate_image(&prompt, ImageGenerationParams::default())
                    .instrument(span),
            )
            .await?;

            post_image(
                context,
                &response.mime_type,
                response.bytes,
                "generated-image",
            )
            .await?;

            Ok(posted_result(
                "Created the image",
                &prompt,
                response.revised_prompt.as_deref(),
                max_output_bytes,
            ))
        })
    }
}
