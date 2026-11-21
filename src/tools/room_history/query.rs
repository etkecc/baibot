use chrono::{DateTime, Utc};
use mxlink::matrix_sdk::ruma::{EventId, OwnedEventId};
use serde::Deserialize;

/// Messages returned when the model doesn't say how many.
pub const DEFAULT_LIMIT: usize = 10;

/// The most messages returned for a single read.
pub const MAX_LIMIT: usize = 200;

/// The most sender names in a single read.
const MAX_SENDERS: usize = 20;

/// What the model asks for.
#[derive(Clone, Debug, PartialEq)]
pub struct HistoryQuery {
    /// How many messages to return (at most).
    pub limit: usize,

    /// Only messages sent at or after this time.
    pub since: Option<DateTime<Utc>>,

    /// Only messages sent at or before this time.
    pub until: Option<DateTime<Utc>>,

    /// Only messages from these people (names or Matrix user IDs, resolved against the room's members).
    pub senders: Option<Vec<String>>,

    /// Only messages before this event (for reading further back than an earlier call),
    /// instead of before the message being answered.
    pub before: Option<OwnedEventId>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Arguments {
    #[serde(default)]
    limit: Option<i64>,

    #[serde(default)]
    since: Option<String>,

    #[serde(default)]
    until: Option<String>,

    #[serde(default)]
    senders: Option<Vec<String>>,

    #[serde(default)]
    before: Option<String>,
}

impl HistoryQuery {
    /// Parses the arguments of a tool call (a JSON text), returning an error message for the model if invalid.
    pub fn parse(arguments: &str) -> Result<Self, String> {
        // Some models send an empty text for calls without arguments.
        let arguments = if arguments.trim().is_empty() {
            "{}"
        } else {
            arguments
        };

        let arguments: Arguments =
            serde_json::from_str(arguments).map_err(|err| format!("Invalid arguments: {err}"))?;

        let limit = match arguments.limit {
            None => DEFAULT_LIMIT,
            Some(limit) if limit < 1 => {
                return Err("Invalid arguments: `limit` must be at least 1.".to_owned());
            }
            Some(limit) => usize::try_from(limit).unwrap_or(MAX_LIMIT).min(MAX_LIMIT),
        };

        let since = parse_time("since", arguments.since)?;
        let until = parse_time("until", arguments.until)?;

        if let (Some(since), Some(until)) = (since, until)
            && since > until
        {
            return Err("Invalid arguments: `since` must not be after `until`.".to_owned());
        }

        let senders = match arguments.senders {
            None => None,
            Some(senders) => {
                let senders: Vec<String> = senders
                    .into_iter()
                    .map(|sender| sender.trim().to_owned())
                    .filter(|sender| !sender.is_empty())
                    .collect();

                if senders.len() > MAX_SENDERS {
                    return Err(format!(
                        "Invalid arguments: at most {MAX_SENDERS} `senders` are supported."
                    ));
                }

                // An empty list means no filter, like leaving it out.
                (!senders.is_empty()).then_some(senders)
            }
        };

        let before = match arguments.before.filter(|before| !before.trim().is_empty()) {
            None => None,
            Some(before) => Some(EventId::parse(before.trim()).map_err(|_| {
                "Invalid arguments: `before` must be an event ID from an earlier result (like $abc123).".to_owned()
            })?),
        };

        Ok(Self {
            limit,
            since,
            until,
            senders,
            before,
        })
    }
}

fn parse_time(name: &str, value: Option<String>) -> Result<Option<DateTime<Utc>>, String> {
    let Some(value) = value.filter(|value| !value.trim().is_empty()) else {
        return Ok(None);
    };

    DateTime::parse_from_rfc3339(value.trim())
        .map(|time| Some(time.with_timezone(&Utc)))
        .map_err(|_| {
            format!(
                "Invalid arguments: `{name}` must be an RFC 3339 timestamp (like 2026-01-31T14:00:00Z)."
            )
        })
}

/// The JSON schema of the tool's arguments.
pub fn parameters_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "limit": {
                "type": "integer",
                "minimum": 1,
                "maximum": MAX_LIMIT,
                "description": format!("How many messages to return at most (newest first are kept). Defaults to {DEFAULT_LIMIT}. Ask for more only when needed."),
            },
            "since": {
                "type": "string",
                "description": "Only messages sent at or after this time (RFC 3339, like 2026-01-31T14:00:00Z).",
            },
            "until": {
                "type": "string",
                "description": "Only messages sent at or before this time (RFC 3339).",
            },
            "before": {
                "type": "string",
                "description": "Only messages before this event ID. To read further back than an earlier call, pass the event ID it suggests.",
            },
            "senders": {
                "type": "array",
                "items": {"type": "string"},
                "description": "Only messages from these people: Matrix user IDs (like @alice:example.com) or display names.",
            },
        },
        "additionalProperties": false,
    })
}
