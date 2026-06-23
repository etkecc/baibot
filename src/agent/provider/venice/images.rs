use crate::agent::AgentPurpose;
use crate::agent::provider::entity::{ImageEditResult, ImageGenerationResult, ImageSource};
use crate::agent::provider::{ImageEditParams, ImageGenerationParams};
use crate::strings;
use crate::utils::base64::{base64_decode, base64_encode};

use super::config::Config;
use super::wire::{EditImageRequest, GenerateImageRequest, GenerateImageResponse};

/// Generate an image via Venice's native `/image/generate` endpoint.
///
/// This is the base64-in-JSON path: we pin `return_binary: false` so Venice answers with a JSON
/// envelope (`GenerateImageResponse`) carrying the image as a base64 string, which we decode. The
/// sibling `create_image_edit` is the *other* response shape (raw binary); the two must not be
/// crossed. `params` is advisory only; the Venice config drives the request.
pub async fn generate_image(
    config: &Config,
    http: &reqwest::Client,
    prompt: &str,
    _params: ImageGenerationParams,
) -> anyhow::Result<ImageGenerationResult> {
    let Some(image_generation_config) = &config.image_generation else {
        return Err(anyhow::anyhow!(
            strings::agent::no_configuration_for_purpose_so_cannot_be_used(
                &AgentPurpose::ImageGeneration
            ),
        ));
    };

    let request = GenerateImageRequest {
        model: image_generation_config.model_id.clone(),
        prompt: prompt.to_owned(),
        // Pinned: baibot wants exactly one image, returned as base64-in-JSON so `GenerateImageResponse`
        // can decode it. Flipping `return_binary` would make Venice answer with raw binary and break
        // the JSON decode below, so neither knob is configurable.
        return_binary: false,
        variants: 1,
        negative_prompt: image_generation_config.negative_prompt.clone(),
        cfg_scale: image_generation_config.cfg_scale,
        steps: image_generation_config.steps,
        style_preset: image_generation_config.style_preset.clone(),
        seed: image_generation_config.seed,
        safe_mode: image_generation_config.safe_mode,
        hide_watermark: image_generation_config.hide_watermark,
        format: image_generation_config.format.clone(),
        width: image_generation_config.width,
        height: image_generation_config.height,
        aspect_ratio: image_generation_config.aspect_ratio.clone(),
        resolution: image_generation_config.resolution.clone(),
        quality: image_generation_config.quality.clone(),
        lora_strength: image_generation_config.lora_strength,
        embed_exif_metadata: image_generation_config.embed_exif_metadata,
        enable_web_search: image_generation_config.enable_web_search,
    };

    let url = format!("{}/image/generate", config.base_url.trim_end_matches('/'));

    // The prompt is user content; keep it out of logs (mirrors the STT/TTS paths).
    tracing::trace!(
        model_id = image_generation_config.model_id,
        "Sending Venice image generation API request"
    );

    let response = http
        .post(&url)
        .bearer_auth(&config.api_key)
        .json(&request)
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() {
        // Body to the server log only, not into the returned error (which reaches the Matrix room).
        let body = response.text().await.unwrap_or_default();
        tracing::warn!(%status, body, "Venice image generation request failed");
        return Err(anyhow::anyhow!(
            "Venice image generation request failed with status {status}"
        ));
    }

    let response: GenerateImageResponse = response.json().await?;

    tracing::trace!(request_id = ?response.id, "Venice image generation succeeded");

    let Some(image_base64) = response.images.into_iter().next() else {
        return Err(anyhow::anyhow!(
            "The Venice image generation API returned no images"
        ));
    };

    // Swallow the decode error's detail (it can echo input bytes/offsets); the returned error
    // reaches the Matrix room, so it stays generic while the real cause goes to the server log.
    let bytes = base64_decode(&image_base64).map_err(|decode_err| {
        tracing::warn!(%decode_err, "Venice image generation returned undecodable base64");
        anyhow::anyhow!("Venice image generation returned invalid base64 image data")
    })?;

    Ok(ImageGenerationResult {
        bytes,
        mime_type: image_format_to_mime_type(image_generation_config.format.as_deref()),
        revised_prompt: None,
    })
}

/// Edit an image via Venice's native `/image/edit` endpoint.
///
/// This is the raw-binary path: the request is JSON carrying the source image as a base64 string
/// (Venice's `image` field is `anyOf` upload/base64/URL; we send base64, no multipart), and the
/// response body IS the edited image bytes (no JSON envelope). `params` is advisory only.
pub async fn create_image_edit(
    config: &Config,
    http: &reqwest::Client,
    prompt: &str,
    images: Vec<ImageSource>,
    _params: ImageEditParams,
) -> anyhow::Result<ImageEditResult> {
    let Some(image_generation_config) = &config.image_generation else {
        return Err(anyhow::anyhow!(
            strings::agent::no_configuration_for_purpose_so_cannot_be_used(
                &AgentPurpose::ImageGeneration
            ),
        ));
    };

    let edit_config = &image_generation_config.edit;

    let Some(source) = images.into_iter().next() else {
        return Err(anyhow::anyhow!("No image sources provided"));
    };

    let request = EditImageRequest {
        model: edit_config.model_id.clone(),
        prompt: prompt.to_owned(),
        image: base64_encode(&source.bytes),
        output_format: edit_config.output_format.clone(),
        aspect_ratio: edit_config.aspect_ratio.clone(),
        resolution: edit_config.resolution.clone(),
        safe_mode: edit_config.safe_mode,
    };

    let url = format!("{}/image/edit", config.base_url.trim_end_matches('/'));

    // The prompt is user content; keep it out of logs (mirrors the STT/TTS paths).
    tracing::trace!(
        model_id = edit_config.model_id,
        "Sending Venice image edit API request"
    );

    let response = http
        .post(&url)
        .bearer_auth(&config.api_key)
        .json(&request)
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() {
        // Body to the server log only, not into the returned error (which reaches the Matrix room).
        let body = response.text().await.unwrap_or_default();
        tracing::warn!(%status, body, "Venice image edit request failed");
        return Err(anyhow::anyhow!(
            "Venice image edit request failed with status {status}"
        ));
    }

    // The edit endpoint answers with raw binary image bytes, so read the body directly instead of
    // parsing JSON. The actual format comes from the response Content-Type header; fall back to the
    // configured `output_format` when the header is missing or unparseable.
    let mime_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<mxlink::mime::Mime>().ok())
        .unwrap_or_else(|| image_format_to_mime_type(edit_config.output_format.as_deref()));

    let bytes = response.bytes().await?.to_vec();

    Ok(ImageEditResult { bytes, mime_type })
}

/// Map a Venice image `format`/`output_format` value (`jpeg`/`png`/`webp`) to its MIME type.
/// Venice defaults to `webp` when the format is unset, so an absent value maps to `image/webp`.
fn image_format_to_mime_type(format: Option<&str>) -> mxlink::mime::Mime {
    match format.unwrap_or("webp") {
        "jpeg" | "jpg" => mxlink::mime::IMAGE_JPEG,
        "png" => mxlink::mime::IMAGE_PNG,
        // No mxlink::mime constant for webp; parse it, falling back to PNG on any surprise value.
        _ => "image/webp".parse().unwrap_or(mxlink::mime::IMAGE_PNG),
    }
}
