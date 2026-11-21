//! The tools the bot knows, by id, for access rules and the tool registry.

/// The id of the tool which reads the room's recent messages.
pub const ROOM_HISTORY: &str = "builtin.room.history";

/// What access rules and help need to know about a tool.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ToolInfo {
    /// The stable id, used in `tool:ID` rule permissions.
    ///
    /// Dot-separated names, each made of lowercase letters, digits and `-`. Rules can match a namespace as a whole
    /// (`tool:builtin.image.*`). Built-in tools are `builtin.NAMESPACE.NAME` (e.g. `builtin.room.history`), so
    /// `tool:builtin.*` covers all of them. Tools defined by administrators later get namespaces of their own (e.g.
    /// `holmes.investigate`), outside of `builtin`.
    pub id: &'static str,

    /// The emoji shown next to the tool, for people.
    pub emoji: &'static str,

    /// A short description, for people.
    pub summary: &'static str,

    /// Whether people may use the tool unless a rule denies it.
    /// Otherwise, a rule needs to allow it.
    pub default_enabled: bool,
}

/// The built-in tools.
pub const BUILT_IN: &[ToolInfo] = &[ToolInfo {
    id: ROOM_HISTORY,
    emoji: "📜",
    summary: "read the room's recent messages",
    default_enabled: true,
}];

/// Returns the known tools, sorted by id.
pub fn known() -> impl Iterator<Item = &'static ToolInfo> {
    BUILT_IN.iter()
}

#[cfg(test)]
mod tests {
    use super::BUILT_IN;

    #[test]
    fn built_in_tools_have_unique_sorted_builtin_ids() {
        for pair in BUILT_IN.windows(2) {
            assert!(pair[0].id < pair[1].id, "{} / {}", pair[0].id, pair[1].id);
        }

        for tool in BUILT_IN {
            // `builtin.namespace.name`, so that `tool:builtin.*` rules cover all built-in tools and
            // `tool:builtin.namespace.*` rules a group of related ones. All parts are plain names (not a wildcard like
            // `builtin.room.*`), made of lowercase letters, digits and `-`.
            let parts: Vec<&str> = tool.id.split('.').collect();
            assert!(
                parts.len() == 3
                    && parts[0] == "builtin"
                    && parts.iter().all(|part| {
                        !part.is_empty()
                            && part
                                .chars()
                                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
                    }),
                "{}",
                tool.id
            );
        }
    }
}
