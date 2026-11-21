//! What access rules and help need to know about the tools the bot knows.

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

/// Returns the known tools, sorted by id.
pub fn known() -> impl Iterator<Item = &'static ToolInfo> {
    super::registry::tools().map(|tool| tool.info())
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::known;
    use crate::tools::registry::function_name;

    #[test]
    fn known_tools_have_unique_sorted_builtin_ids() {
        let tools: Vec<_> = known().collect();

        for pair in tools.windows(2) {
            assert!(pair[0].id < pair[1].id, "{} / {}", pair[0].id, pair[1].id);
        }

        for tool in &tools {
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

        // Function names are derived from ids, so they must not collide either.
        let function_names: HashSet<_> = tools.iter().map(|tool| function_name(tool.id)).collect();
        assert_eq!(function_names.len(), tools.len());
    }
}
