use chrono::{DateTime, Utc};
use std::collections::HashMap;

pub struct TextGenerationPromptVariables {
    map: HashMap<String, String>,
}

impl Default for TextGenerationPromptVariables {
    fn default() -> Self {
        let now = Utc::now();
        Self::new("unnamed", "unknown-model", now, Some(now))
    }
}

impl TextGenerationPromptVariables {
    pub fn new(
        bot_name: &str,
        model_id: &str,
        now_time: DateTime<Utc>,
        conversation_start_time: Option<DateTime<Utc>>,
    ) -> Self {
        let mut map = HashMap::new();

        map.insert("baibot_name".to_string(), bot_name.to_string());
        map.insert("baibot_model_id".to_string(), model_id.to_string());
        map.insert("baibot_now_utc".to_string(), format_utc_time(now_time));

        let baibot_conversation_start_time_utc = match conversation_start_time {
            Some(conversation_start_time) => format_utc_time(conversation_start_time),
            None => "unknown".to_string(),
        };

        map.insert(
            "baibot_conversation_start_time_utc".to_string(),
            baibot_conversation_start_time_utc,
        );

        Self { map }
    }

    pub fn format(&self, text: &str) -> String {
        let mut formatted_text = text.to_string();

        for (key, value) in &self.map {
            let placeholder = format!("{{{{ {} }}}}", key);
            formatted_text = formatted_text.replace(&placeholder, value);
        }

        formatted_text
    }
}

fn format_utc_time(time: DateTime<Utc>) -> String {
    time.format("%Y-%m-%d (%A), %H:%M:%S UTC").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Timelike};

    #[test]
    fn test_new() {
        // Intentionally injecting some sub-seconds to ensure formatting would ignore them.
        let now_utc = Utc
            .with_ymd_and_hms(2024, 9, 20, 18, 34, 15)
            .unwrap()
            .with_nanosecond(250000000)
            .unwrap();

        let conversation_start_time_utc = Utc
            .with_ymd_and_hms(2024, 9, 19, 18, 34, 15)
            .unwrap()
            .with_nanosecond(250000000)
            .unwrap();

        let variables = TextGenerationPromptVariables::new(
            "baibot",
            "gpt-4o",
            now_utc,
            Some(conversation_start_time_utc),
        );

        assert_eq!(
            variables.map.get("baibot_name"),
            Some(&"baibot".to_string())
        );
        assert_eq!(
            variables.map.get("baibot_model_id"),
            Some(&"gpt-4o".to_string())
        );
        assert_eq!(
            variables.map.get("baibot_now_utc"),
            Some(&format_utc_time(now_utc))
        );
        assert_eq!(
            variables.map.get("baibot_conversation_start_time_utc"),
            Some(&format_utc_time(conversation_start_time_utc))
        );

        let prompt = "Hello, I'm {{ baibot_name }} using {{ baibot_model_id }}. The date/time now is {{ baibot_now_utc }} and this conversation started at {{ baibot_conversation_start_time_utc }}.";
        let expected = "Hello, I'm baibot using gpt-4o. The date/time now is 2024-09-20 (Friday), 18:34:15 UTC and this conversation started at 2024-09-19 (Thursday), 18:34:15 UTC.";

        assert_eq!(variables.format(prompt), expected);
    }
}
