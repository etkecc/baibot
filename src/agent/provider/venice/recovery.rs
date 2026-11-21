//! Auto-recovery for Venice's strict request bodies.
//!
//! Venice's `/chat/completions` body is `additionalProperties: false`, so a model that does not
//! support an optional knob rejects the whole request with a 400 instead of ignoring the field.
//! Some knobs are documented as model-specific ("for supported models") and are pure
//! optimization/tuning hints: dropping them changes nothing about the answer, only loses the
//! optimization. When such a field is the reason for a 400, we strip it and retry, then remember
//! the rejection per model so later requests skip the field (and the wasted round-trip) entirely.
//!
//! Universal sampling knobs (`temperature`, `top_p`, the penalties, `max_completion_tokens`) are
//! deliberately NOT recoverable here: dropping one silently changes the model's output, so a model
//! that rejects one is a real configuration problem the operator must see, not paper over.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, OnceLock, RwLock};

use regex::Regex;

use super::wire::ChatCompletionRequest;

/// Top-level chat-completion fields baibot may drop to recover from a 400. Each is documented by
/// Venice as model-specific or as a routing hint, and is meaning-preserving to omit (Venice falls
/// back to its server-side default):
/// - `prompt_cache_retention` — "extends retention ... for supported models" (cache TTL only)
/// - `reasoning_effort` — "control reasoning effort level for supported models"
/// - `prompt_cache_key` — cache-routing hint; dropping it only forfeits a cache-hit optimization
///
/// Recovery operates on TOP-LEVEL request fields only. Sub-fields inside the `venice_parameters`
/// bag (`disable_thinking`, `enable_e2ee`, `character_slug`, …) are intentionally absent: a model
/// that rejects one surfaces a clear 400 to the operator rather than being auto-stripped. Adding a
/// new bag field does not extend recovery to it; only a name listed here is droppable.
pub(super) const DROPPABLE_FIELDS: &[&str] = &[
    "prompt_cache_retention",
    "prompt_cache_key",
    "reasoning_effort",
];

/// Per-model record of fields a Venice model has rejected as unsupported, learned at runtime from
/// 400 responses. The `Arc` is shared across `Controller` clones, so a rejection learned once is
/// seen by every clone of the same agent. The cache is process-lived only: a restart re-learns on
/// the first request to each model, which costs one extra round-trip and nothing else, so there is
/// no persistence to keep in sync with config changes.
#[derive(Debug, Clone, Default)]
pub(super) struct UnsupportedFieldsCache {
    inner: Arc<RwLock<HashMap<String, HashSet<String>>>>,
}

/// Process-global registry of per-deployment caches, keyed by Venice base URL. A `Controller` is
/// rebuilt from its config on every message for dynamic (global/room-local) agents (see
/// `agent::manager::available_room_agents_by_room_config_context`), so a cache stored on the
/// controller would be reset each message and the "process-lived" intent above would never hold.
/// The registry lets a rebuilt controller re-attach to the same cache. Keyed by base URL because
/// "which fields a model rejects" is a property of the deployment+model, not of the agent instance:
/// agents on the same Venice deployment share learnings, while a distinct deployment keeps its own,
/// so a model name that supports a field on one deployment is never proactively stripped against
/// another.
fn registry() -> &'static RwLock<HashMap<String, UnsupportedFieldsCache>> {
    static REGISTRY: OnceLock<RwLock<HashMap<String, UnsupportedFieldsCache>>> = OnceLock::new();
    REGISTRY.get_or_init(|| RwLock::new(HashMap::new()))
}

impl UnsupportedFieldsCache {
    /// The process-global cache for `base_url`, created on first use and shared by every controller
    /// for that deployment thereafter. This is what makes a learned rejection survive the
    /// per-message controller rebuild. A poisoned registry lock degrades to an isolated cache:
    /// correctness holds, only cross-controller sharing is lost for that call.
    pub(super) fn shared_for(base_url: &str) -> Self {
        if let Ok(map) = registry().read()
            && let Some(cache) = map.get(base_url)
        {
            return cache.clone();
        }

        match registry().write() {
            Ok(mut map) => map.entry(base_url.to_owned()).or_default().clone(),
            Err(_) => Self::default(),
        }
    }

    /// Fields already known unsupported for `model_id`. Returns an empty set on an unknown model or
    /// a poisoned lock, so a cache failure degrades to "strip nothing proactively" rather than
    /// breaking the request path.
    pub(super) fn known_for(&self, model_id: &str) -> HashSet<String> {
        self.inner
            .read()
            .ok()
            .and_then(|map| map.get(model_id).cloned())
            .unwrap_or_default()
    }

    /// Records that `model_id` rejected `field`. A poisoned lock is ignored: failing to memoize only
    /// means the next request re-discovers the rejection, never a wrong result.
    pub(super) fn record(&self, model_id: &str, field: &str) {
        if let Ok(mut map) = self.inner.write() {
            map.entry(model_id.to_owned())
                .or_default()
                .insert(field.to_owned());
        }
    }
}

/// Parses the offending field name out of a Venice 400 body. A field the schema does not allow is
/// reported as `... field: 'prompt_cache_retention', value: '...'`, so this matches the `field: '..'`
/// marker wherever it sits in the message. Returns `None` when the body carries no such marker (a
/// different 400 class, e.g. a missing required field), so the caller surfaces that error instead.
///
/// Returns only the FIRST `field: '..'` match by design. If Venice ever names several rejected
/// fields in one body, the retry loop strips this one, retries, and rediscovers the next on the
/// following 400 — bounded and correct. Do not switch to `captures_iter` to "batch" them without
/// re-checking the loop's per-field termination bound in `chat.rs`.
pub(super) fn parse_rejected_field(body: &str) -> Option<String> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re =
        RE.get_or_init(|| Regex::new(r"field: '([^']+)'").expect("rejected-field regex is valid"));
    re.captures(body)
        .map(|caps| caps[1].to_owned())
        .filter(|field| !field.is_empty())
}

/// Clears `field` from the request when it is one baibot may safely drop and it is currently set.
/// Returns `true` only when a value was actually removed, which is what bounds the retry loop: once
/// a field is `None`, a repeat rejection for the same name returns `false` and the caller stops
/// instead of retrying forever. A field outside [`DROPPABLE_FIELDS`] always returns `false`, so a
/// meaning-bearing knob is never silently dropped.
pub(super) fn strip_droppable_field(request: &mut ChatCompletionRequest, field: &str) -> bool {
    if !DROPPABLE_FIELDS.contains(&field) {
        return false;
    }
    match field {
        "prompt_cache_retention" => request.prompt_cache_retention.take().is_some(),
        "prompt_cache_key" => request.prompt_cache_key.take().is_some(),
        "reasoning_effort" => request.reasoning_effort.take().is_some(),
        _ => false,
    }
}

/// The name under which [`UnsupportedFieldsCache`] remembers a model which rejected a request with tools. Unlike the
/// droppable fields, tools are taken out of the request (and their instructions out of the system prompt) by the caller,
/// and only remembered once the request works without them.
pub(super) const TOOLS_FIELD: &str = "tools";

/// Whether a 400 for a request with tools looks like the model not supporting them. Function calling is model-specific
/// on Venice (`supportsFunctionCalling`), and requests with `tools` for other models are rejected before inference,
/// with the reason in `issues[]` (e.g. `{"error": "Invalid request parameters", "issues": [...]}`). An older form names
/// the field (`field: 'tools'`).
///
/// Only the error message and the issues count, not the rest of the body. Other complaints about the tools (e.g. a
/// schema keyword Venice doesn't take) don't count, as the model may well support tools.
pub(super) fn is_tools_rejection(body: &str) -> bool {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(body.trim()) else {
        return false;
    };

    // The older form: the whole field is rejected, said in the top-level message.
    if let Some(message) = value["error"].as_str()
        && !message.to_lowercase().contains("schema")
        && parse_rejected_field(message).as_deref() == Some(TOOLS_FIELD)
    {
        return true;
    }

    // The top-level message, and each issue's message with the field it's about (if said).
    let mut complaints: Vec<(String, Option<String>)> = [
        &value["error"]["message"],
        &value["error"],
        &value["message"],
    ]
    .into_iter()
    .filter_map(|value| value.as_str())
    .take(1)
    .map(|message| (message.to_owned(), None))
    .collect();

    if let Some(issues) = value["issues"].as_array() {
        for issue in issues {
            let message = issue
                .as_str()
                .or_else(|| issue["message"].as_str())
                .unwrap_or_default();

            let field = match &issue["path"] {
                // An issue deeper inside a field (e.g. `["tools", 0, "function", "parameters"]`) is about its content
                // (e.g. a schema keyword), not about the model supporting the field.
                serde_json::Value::Array(path) if path.len() > 1 => continue,
                serde_json::Value::Array(path) => path
                    .first()
                    .and_then(|first| first.as_str())
                    .map(str::to_owned),
                serde_json::Value::String(path) if path.contains(['.', '[']) => continue,
                serde_json::Value::String(path) => Some(path.clone()),
                _ => issue["field"].as_str().map(str::to_owned),
            };

            complaints.push((message.to_owned(), field));
        }
    }

    complaints.into_iter().any(|(message, field)| {
        let message = message.to_lowercase();

        // About the tools' schemas, not about the model.
        if message.contains("schema") {
            return false;
        }

        let about_tools = message.contains("tool")
            || message.contains("function calling")
            || field.is_some_and(|field| {
                matches!(
                    field.as_str(),
                    "tools" | "tool_choice" | "parallel_tool_calls"
                )
            });

        let unsupported = [
            "not supported",
            "does not support",
            "doesn't support",
            "not enabled",
            "not available",
        ]
        .iter()
        .any(|phrase| message.contains(phrase));

        about_tools && unsupported
    })
}

/// Pulls a human-readable message out of a Venice error body for surfacing in the room. Venice's
/// usual envelope is `{"error": "..."}`; some OpenAI-compatible paths nest `{"error": {"message":
/// "..."}}`. Falls back to the trimmed raw body (length-capped so a large body cannot flood the
/// room) and finally to a fixed string for an empty body, so the caller always has something to
/// show.
pub(super) fn extract_error_message(body: &str) -> String {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return "no response body".to_owned();
    }

    if let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) {
        if let Some(msg) = value.get("error").and_then(|e| e.as_str()) {
            return msg.to_owned();
        }
        if let Some(msg) = value
            .get("error")
            .and_then(|e| e.get("message"))
            .and_then(|m| m.as_str())
        {
            return msg.to_owned();
        }
    }

    const MAX: usize = 500;
    if trimmed.chars().count() > MAX {
        trimmed.chars().take(MAX).collect::<String>() + "…"
    } else {
        trimmed.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn full_request() -> ChatCompletionRequest {
        ChatCompletionRequest {
            model: "venice-uncensored".to_owned(),
            messages: vec![],
            temperature: Some(0.7),
            max_completion_tokens: Some(1024),
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            repetition_penalty: None,
            reasoning_effort: Some("high".to_owned()),
            prompt_cache_key: Some("cafef00d".to_owned()),
            prompt_cache_retention: Some("24h".to_owned()),
            venice_parameters: None,
            tools: vec![],
        }
    }

    #[test]
    fn parses_the_rejected_field_from_a_real_venice_body() {
        let body = r#"{"error":"Extra inputs are not permitted, field: 'prompt_cache_retention', value: 'default'","request_id":"qM_DmKSXKF07wRxmQJ-hc"}"#;
        assert_eq!(
            parse_rejected_field(body).as_deref(),
            Some("prompt_cache_retention")
        );
    }

    #[test]
    fn returns_no_field_when_the_body_has_no_field_marker() {
        // A different 400 class (e.g. a genuinely malformed request) carries no `field: '..'`
        // marker, so there is nothing to strip and the caller must surface the error instead.
        assert_eq!(parse_rejected_field(r#"{"error":"Invalid request"}"#), None);
        assert_eq!(parse_rejected_field(""), None);
    }

    #[test]
    fn strips_a_droppable_field_once_then_reports_no_progress() {
        let mut request = full_request();

        // First strip clears the field and reports progress, so the caller retries.
        assert!(strip_droppable_field(
            &mut request,
            "prompt_cache_retention"
        ));
        assert!(request.prompt_cache_retention.is_none());

        // A repeat rejection for the same (now absent) field reports no progress: this is what
        // stops the retry loop instead of spinning forever.
        assert!(!strip_droppable_field(
            &mut request,
            "prompt_cache_retention"
        ));
    }

    #[test]
    fn refuses_to_strip_a_meaning_bearing_field() {
        let mut request = full_request();

        // `temperature` is universal and changes the output; a rejection for it must surface, never
        // be silently dropped. The whole droppable set is the only thing strip will touch.
        assert!(!strip_droppable_field(&mut request, "temperature"));
        assert_eq!(request.temperature, Some(0.7));

        assert!(!strip_droppable_field(
            &mut request,
            "max_completion_tokens"
        ));
        assert_eq!(request.max_completion_tokens, Some(1024));

        for field in DROPPABLE_FIELDS {
            assert!(
                strip_droppable_field(&mut full_request(), field),
                "every advertised droppable field must actually be strippable: {field}"
            );
        }
    }

    #[test]
    fn cache_records_per_model_and_isolates_models() {
        let cache = UnsupportedFieldsCache::default();
        assert!(cache.known_for("venice-uncensored").is_empty());

        cache.record("venice-uncensored", "prompt_cache_retention");
        cache.record("venice-uncensored", "reasoning_effort");

        let known = cache.known_for("venice-uncensored");
        assert!(known.contains("prompt_cache_retention"));
        assert!(known.contains("reasoning_effort"));

        // A rejection learned for one model must not leak to another.
        assert!(cache.known_for("kimi-k2-5").is_empty());
    }

    #[test]
    fn shared_for_survives_controller_rebuild_and_isolates_deployments() {
        // Distinct, test-only base URLs so the process-global registry can't collide with another
        // test running in parallel.
        let url_a = "https://shared-for-test-a.invalid/api/v1";
        let url_b = "https://shared-for-test-b.invalid/api/v1";

        // A controller rebuilt for the same deployment (a fresh `shared_for` call, as happens per
        // message for dynamic agents) re-attaches to the SAME cache, so an earlier learning holds.
        let first = UnsupportedFieldsCache::shared_for(url_a);
        first.record("model-x", "reasoning_effort");

        let rebuilt = UnsupportedFieldsCache::shared_for(url_a);
        assert!(
            rebuilt.known_for("model-x").contains("reasoning_effort"),
            "a rebuilt controller for the same deployment must see the earlier rejection"
        );

        // A different deployment keeps its own learnings: a model name that rejects a field on one
        // Venice must not silence/strip it on another.
        let other_deployment = UnsupportedFieldsCache::shared_for(url_b);
        assert!(
            other_deployment.known_for("model-x").is_empty(),
            "rejections must not leak across deployments"
        );
    }

    #[test]
    fn tool_rejections_are_recognized() {
        assert!(is_tools_rejection(
            r#"{"error":"Extra inputs are not permitted, field: 'tools', value: '[...]'"}"#
        ));
        assert!(is_tools_rejection(
            r#"{"error":"Model venice-uncensored does not support function calling"}"#
        ));
        assert!(is_tools_rejection(
            r#"{"error":{"message":"Tools are not supported for this model"}}"#
        ));

        // Capability gates: a generic error, with the reason in the issues.
        assert!(is_tools_rejection(
            r#"{"error":"Invalid request parameters","issues":[{"code":"custom","path":["tools"],"message":"tools is not supported by this model"}]}"#
        ));
        assert!(is_tools_rejection(
            r#"{"error":"Invalid request parameters","issues":[{"path":["parallel_tool_calls"],"message":"This model does not support it"}]}"#
        ));
        assert!(is_tools_rejection(
            r#"{"error":"Invalid request parameters","issues":["Function calling is not supported by this model"]}"#
        ));
    }

    #[test]
    fn other_complaints_are_no_tool_rejections() {
        // About tools, but not about support for them.
        assert!(!is_tools_rejection(
            r#"{"error":"Invalid tool call: missing tool_call_id"}"#
        ));
        assert!(!is_tools_rejection(
            r#"{"error":"Invalid request parameters","issues":[{"path":["tools",0,"function","parameters"],"message":"Unsupported JSON Schema keyword"}]}"#
        ));
        assert!(!is_tools_rejection(
            r#"{"error":"Invalid request parameters","issues":[{"path":["tools",0,"function","parameters"],"message":"This JSON Schema keyword is not supported"}]}"#
        ));
        assert!(!is_tools_rejection(
            r#"{"error":"Invalid request parameters","issues":[{"path":"tools.0.function.parameters","message":"Tool parameters: keyword not supported"}]}"#
        ));
        assert!(!is_tools_rejection(
            r#"{"error":"Unsupported JSON Schema keyword in tools[0]"}"#
        ));
        assert!(!is_tools_rejection(
            r#"{"error":"The JSON schema keyword in tools[0] is not supported"}"#
        ));
        assert!(!is_tools_rejection(
            r#"{"error":"Invalid schema, field: 'tools'","issues":[{"path":["tools",0,"function","parameters"],"message":"Invalid"}]}"#
        ));
        assert!(!is_tools_rejection(
            r#"{"error":"Invalid request parameters","issues":[{"path":["tools",0],"message":"Bad item, field: 'tools'"}]}"#
        ));
        // Another field.
        assert!(!is_tools_rejection(
            r#"{"error":"Extra inputs are not permitted, field: 'reasoning_effort', value: 'high'"}"#
        ));
        assert!(!is_tools_rejection(
            r#"{"error":"Invalid request parameters","issues":[{"path":["seed"],"message":"seed is not supported by this model"}]}"#
        ));
        // Not a JSON error message.
        assert!(!is_tools_rejection("tools are not supported"));
    }

    #[test]
    fn extracts_a_human_message_from_error_envelopes() {
        assert_eq!(
            extract_error_message(r#"{"error":"Extra inputs are not permitted","request_id":"x"}"#),
            "Extra inputs are not permitted"
        );

        // OpenAI-style nested envelope.
        assert_eq!(
            extract_error_message(r#"{"error":{"message":"context length exceeded"}}"#),
            "context length exceeded"
        );

        // Unknown shape falls back to the raw body; empty falls back to a fixed string.
        assert_eq!(
            extract_error_message("plain text failure"),
            "plain text failure"
        );
        assert_eq!(extract_error_message("   "), "no response body");
    }
}
