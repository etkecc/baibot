//! Tools which create images with the room's image-generation agent and post them in the room.

mod generate;

pub use generate::GenerateImageTool;

use std::future::Future;
use std::time::Duration;

use crate::agent::{AgentInstance, AgentPurpose, utils::get_effective_agent_for_purpose};
use crate::utils::mime::get_file_extension;

use super::registry::ToolContext;

/// Creating an image may take a while (high quality images take a minute or more with some models).
///
/// Split into [`GENERATION_TIMEOUT`] and [`POSTING_TIMEOUT`] (with some time left for checking access),
/// so that the model learns which part didn't finish.
const TIMEOUT: Duration = Duration::from_secs(180);

const GENERATION_TIMEOUT: Duration = Duration::from_secs(145);

const POSTING_TIMEOUT: Duration = Duration::from_secs(25);

/// How long before the call's deadline a step ends, so that its result still gets through.
const STEP_MARGIN: Duration = Duration::from_secs(1);

/// When a step must end: after its own time limit, or `reserve` before the call's deadline, whichever comes first.
fn step_deadline(
    own: tokio::time::Instant,
    call_deadline: tokio::time::Instant,
    reserve: Duration,
) -> tokio::time::Instant {
    own.min(call_deadline.checked_sub(reserve).unwrap_or(call_deadline))
}

const GENERATION_TIMED_OUT: &str =
    "Creating the image took too long, so it was given up. Nothing was posted.";

/// Nothing was posted (the image wasn't uploaded, or not completely).
const UPLOAD_FAILED: &str =
    "The image was created, but uploading it failed, so nothing was posted.";

/// Matrix may have accepted the message before the error.
const POSTING_FAILED: &str = "The image was created, but posting it in the room failed: it may or may not be there. Don't create it again unless the person asks.";

/// Matrix may have accepted the message before the time ran out.
const POSTING_TIMED_OUT: &str = "The image was created, but posting it in the room wasn't confirmed in time: it may or may not be there. Don't create it again unless the person asks.";

/// The longest prompt the model may write (some image models accept no more than about 4000 characters).
const MAX_PROMPT_CHARS: usize = 4000;

/// Finds the room's image-generation agent (as configured now).
async fn image_agent(context: &ToolContext) -> Result<AgentInstance, String> {
    get_effective_agent_for_purpose(
        context.bot.agent_manager(),
        &context.room_config_context,
        AgentPurpose::ImageGeneration,
    )
    .await
    .map(|info| info.instance)
    .map_err(|err| {
        tracing::debug!(?err, "No image-generation agent for an image tool");
        "No agent for image generation is set up in this room anymore.".to_owned()
    })
}

/// Runs an image provider call, with a time limit. Provider errors are logged, not passed on.
///
/// Their details (e.g. the provider's response, or request URLs) stay out of the conversation,
/// which goes to the text-generation provider and may be repeated in the room.
async fn generate<T>(
    context: &ToolContext,
    agent: &AgentInstance,
    action: &str,
    future: impl Future<Output = anyhow::Result<T>>,
) -> Result<T, String> {
    // Leaving time for posting, even when checking access took a while.
    let deadline = step_deadline(
        tokio::time::Instant::now() + GENERATION_TIMEOUT,
        context.deadline,
        POSTING_TIMEOUT + STEP_MARGIN,
    );

    match tokio::time::timeout_at(deadline, future).await {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(err)) => {
            tracing::warn!(
                agent_id = agent.identifier().as_string(),
                ?err,
                "Image provider call failed for a tool call ({action})"
            );
            Err(failure_message(&format!("{err:#}")))
        }
        Err(_) => {
            tracing::warn!(
                agent_id = agent.identifier().as_string(),
                "Image provider call timed out for a tool call ({action})"
            );
            Err(GENERATION_TIMED_OUT.to_owned())
        }
    }
}

/// Says why an image provider call failed, without the details.
fn failure_message(error: &str) -> String {
    let error = error.to_lowercase();

    let refused = [
        "safety",
        "content policy",
        "content_policy",
        "moderation",
        "inappropriate",
    ]
    .iter()
    .any(|word| error.contains(word));

    if refused {
        "The image provider refused the request (it may go against its content rules). Nothing was posted.".to_owned()
    } else {
        "The image provider returned an error (the details are in the bot's logs). Nothing was posted.".to_owned()
    }
}

/// Uploads an image and posts it where the bot's answer goes, with a time limit.
async fn post_image(
    context: &ToolContext,
    mime_type: &mxlink::mime::Mime,
    bytes: Vec<u8>,
    file_name_stem: &str,
) -> Result<(), String> {
    // Ending before the call's deadline, so that the model learns which step didn't finish.
    let deadline = step_deadline(
        tokio::time::Instant::now() + POSTING_TIMEOUT,
        context.deadline,
        STEP_MARGIN,
    );

    let matrix_link = context.bot.matrix_link();
    let file_name = format!("{file_name_stem}.{}", get_file_extension(mime_type));

    let mut event_content = match tokio::time::timeout_at(
        deadline,
        matrix_link.media().upload_and_prepare_event_content(
            &context.room,
            mime_type,
            bytes,
            &file_name,
        ),
    )
    .await
    {
        Ok(Ok(event_content)) => event_content,
        Ok(Err(err)) => {
            tracing::warn!(?err, "Failed to upload an image created by a tool");
            return Err(UPLOAD_FAILED.to_owned());
        }
        Err(_) => {
            tracing::warn!("Uploading an image created by a tool timed out");
            return Err(UPLOAD_FAILED.to_owned());
        }
    };

    let response = match tokio::time::timeout_at(
        deadline,
        matrix_link.messaging().send_event(
            &context.room,
            &mut event_content,
            context.response_type_for_new_message(),
        ),
    )
    .await
    {
        Ok(Ok(response)) => response,
        Ok(Err(err)) => {
            tracing::warn!(?err, "Failed to post an image created by a tool");
            // The homeserver may have accepted it before the error (e.g. a lost response).
            return Err(POSTING_FAILED.to_owned());
        }
        Err(_) => {
            tracing::warn!("Posting an image created by a tool timed out");
            return Err(POSTING_TIMED_OUT.to_owned());
        }
    };

    context.posted_messages.push(response.event_id);

    Ok(())
}

/// Reads the `prompt` argument: what to draw (or change).
fn parse_prompt(arguments: &str) -> Result<String, String> {
    #[derive(serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Arguments {
        prompt: String,
    }

    let arguments: Arguments = serde_json::from_str(if arguments.trim().is_empty() {
        "{}"
    } else {
        arguments
    })
    .map_err(|err| format!("Invalid arguments: {err}"))?;

    let prompt = arguments.prompt.trim();

    if prompt.is_empty() {
        return Err("The prompt is empty.".to_owned());
    }

    if prompt.chars().count() > MAX_PROMPT_CHARS {
        return Err(format!(
            "The prompt is too long (at most {MAX_PROMPT_CHARS} characters)."
        ));
    }

    Ok(prompt.to_owned())
}

/// The result for the model once an image was posted.
///
/// `revised_prompt` is the prompt the image model actually used, if it changed it (shortened to fit `max_bytes`).
fn posted_result(
    what: &str,
    prompt: &str,
    revised_prompt: Option<&str>,
    max_bytes: usize,
) -> String {
    let mut text = format!(
        "{what} and posted it in the room, where everyone can see it. You can't see it yourself: don't describe it as if you could, and don't link to it."
    );

    if let Some(revised_prompt) = revised_prompt.map(str::trim)
        && !revised_prompt.is_empty()
        && revised_prompt != prompt
    {
        let intro = " The image model revised the prompt to: ";
        let room = max_bytes.saturating_sub(text.len() + intro.len());

        if room > 0 {
            text.push_str(intro);
            text.push_str(truncate(revised_prompt, room));
        }
    }

    text
}

/// Shortens text to at most `max_bytes` bytes, at a character boundary.
fn truncate(text: &str, max_bytes: usize) -> &str {
    if text.len() <= max_bytes {
        return text;
    }

    let mut end = max_bytes;
    while !text.is_char_boundary(end) {
        end -= 1;
    }

    &text[..end]
}

#[cfg(test)]
mod tests {
    use super::{
        GENERATION_TIMEOUT, MAX_PROMPT_CHARS, POSTING_TIMEOUT, TIMEOUT, failure_message,
        parse_prompt, posted_result,
    };

    #[test]
    fn the_steps_fit_in_the_time_limit() {
        // Leaving time for checking access before the call.
        assert!(
            GENERATION_TIMEOUT + POSTING_TIMEOUT + std::time::Duration::from_secs(5) <= TIMEOUT
        );
    }

    #[test]
    fn steps_end_before_the_call_deadline() {
        use std::time::Duration;

        use super::step_deadline;

        let now = tokio::time::Instant::now();
        let call_deadline = now + Duration::from_secs(100);

        // Its own limit comes first.
        assert_eq!(
            step_deadline(
                now + Duration::from_secs(10),
                call_deadline,
                Duration::from_secs(5)
            ),
            now + Duration::from_secs(10)
        );

        // Leaving the reserve before the call's deadline.
        assert_eq!(
            step_deadline(
                now + Duration::from_secs(150),
                call_deadline,
                Duration::from_secs(26)
            ),
            now + Duration::from_secs(74)
        );
    }

    #[test]
    fn failures_are_told_without_details() {
        let refused = failure_message(
            "OpenAI error: Your request was rejected as a result of our safety system. https://api.example.com/v1/images?key=x",
        );
        assert!(refused.starts_with("The image provider refused"));
        assert!(!refused.contains("example.com"));

        let other =
            failure_message("error sending request for url (https://api.example.com/v1/images)");
        assert!(other.starts_with("The image provider returned an error"));
        assert!(!other.contains("example.com"));
    }

    #[test]
    fn prompts_are_required_trimmed_and_capped() {
        assert_eq!(
            parse_prompt(r#"{"prompt": " a blue cat "}"#),
            Ok("a blue cat".to_owned())
        );

        assert!(parse_prompt("").unwrap_err().contains("missing field"));
        assert!(parse_prompt("not json").is_err());
        assert_eq!(
            parse_prompt(r#"{"prompt": "  "}"#),
            Err("The prompt is empty.".to_owned())
        );
        assert!(
            parse_prompt(r#"{"prompt": "a cat", "size": "large"}"#)
                .unwrap_err()
                .contains("unknown field")
        );

        let long = format!(r#"{{"prompt": "{}"}}"#, "é".repeat(MAX_PROMPT_CHARS + 1));
        assert!(parse_prompt(&long).unwrap_err().contains("too long"));

        let longest = format!(r#"{{"prompt": "{}"}}"#, "é".repeat(MAX_PROMPT_CHARS));
        assert!(parse_prompt(&longest).is_ok());
    }

    #[test]
    fn results_mention_revised_prompts_within_the_budget() {
        let plain = posted_result("Created the image", "a cat", None, 10_000);
        assert!(plain.starts_with("Created the image and posted it in the room"));
        assert!(!plain.contains("revised"));

        // The same prompt isn't repeated.
        assert_eq!(
            posted_result("Created the image", "a cat", Some(" a cat "), 10_000),
            plain
        );

        let revised = posted_result("Created the image", "a cat", Some("A fluffy cat"), 10_000);
        assert!(revised.ends_with(" The image model revised the prompt to: A fluffy cat"));

        // Shortened to fit, at a character boundary.
        let budget = plain.len() + " The image model revised the prompt to: ".len() + 3;
        let shortened = posted_result("Created the image", "a cat", Some("éééé"), budget);
        assert!(shortened.ends_with("to: é"));
        assert!(shortened.len() <= budget);

        // Without room for it, it's left out.
        assert_eq!(
            posted_result("Created the image", "a cat", Some("A fluffy cat"), 10),
            plain
        );
    }
}
