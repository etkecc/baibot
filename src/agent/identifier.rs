use std::fmt;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum PublicIdentifier {
    Static(String),
    DynamicGlobal(String),
    DynamicRoomLocal(String),
}

impl PublicIdentifier {
    pub fn from_str(s: &str) -> Option<Self> {
        if let Some(rest) = s.strip_prefix("static/") {
            return Some(PublicIdentifier::Static(rest.to_string()));
        } else if let Some(rest) = s.strip_prefix("global/") {
            return Some(PublicIdentifier::DynamicGlobal(rest.to_string()));
        } else if let Some(rest) = s.strip_prefix("room-local/") {
            return Some(PublicIdentifier::DynamicRoomLocal(rest.to_string()));
        }
        None
    }

    pub fn as_string(&self) -> String {
        match self {
            PublicIdentifier::Static(s) => format!("static/{}", s),
            PublicIdentifier::DynamicGlobal(s) => format!("global/{}", s),
            PublicIdentifier::DynamicRoomLocal(s) => format!("room-local/{}", s),
        }
    }

    pub fn prefixless(&self) -> String {
        match self {
            PublicIdentifier::Static(s) => s.to_owned(),
            PublicIdentifier::DynamicGlobal(s) => s.to_owned(),
            PublicIdentifier::DynamicRoomLocal(s) => s.to_owned(),
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        let prefixless = self.prefixless();

        if prefixless.is_empty() {
            return Err("The agent ID must not be empty.".to_owned());
        }

        // We use a slash to separate the agent type from the agent ID.
        if prefixless.contains("/") {
            return Err("The agent ID must not contain the `/` character.".to_owned());
        }

        // Spaces are used for separating command arguments, etc.
        if prefixless.contains(" ") {
            return Err("The agent ID must not contain spaces.".to_owned());
        }

        Ok(())
    }
}

impl fmt::Display for PublicIdentifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_public_identifier_from_str() {
        assert_eq!(
            PublicIdentifier::from_str("static/abc"),
            Some(PublicIdentifier::Static("abc".to_string()))
        );
        assert_eq!(
            PublicIdentifier::from_str("global/abc"),
            Some(PublicIdentifier::DynamicGlobal("abc".to_string()))
        );
        assert_eq!(
            PublicIdentifier::from_str("room-local/abc"),
            Some(PublicIdentifier::DynamicRoomLocal("abc".to_string()))
        );
        assert_eq!(PublicIdentifier::from_str("abc"), None);
    }

    #[test]
    fn test_public_identifier_as_string() {
        assert_eq!(
            PublicIdentifier::Static("abc".to_string()).as_string(),
            "static/abc"
        );
        assert_eq!(
            PublicIdentifier::DynamicGlobal("abc".to_string()).as_string(),
            "global/abc"
        );
        assert_eq!(
            PublicIdentifier::DynamicRoomLocal("abc".to_string()).as_string(),
            "room-local/abc"
        );
    }
}
