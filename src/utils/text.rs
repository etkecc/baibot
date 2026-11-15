pub fn block_quote(text: &str) -> String {
    text.lines()
        .map(|line| format!("> {}", line))
        .collect::<Vec<String>>()
        .join("\n")
}

pub fn block_unquote(text: &str) -> String {
    text.lines()
        .map(|line| {
            if let Some(stripped) = line.strip_prefix("> ") {
                stripped.to_string()
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<String>>()
        .join("\n")
}

/// Renders a `Sources:` block of numbered markdown links (`[1] [title](url)`, ...), for appending
/// to a reply whose inline `[n]` references point at them.
///
/// Titles and URLs come from web pages, so they are attacker-influenced. The title is escaped so
/// it cannot break out of the markdown link label, and the URL is used as a link target only when
/// it is a clean `http(s)` URL with no markdown-breaking characters; otherwise the source renders
/// as plain text. This stops a hostile page title or URL from injecting a spoofed clickable link
/// into the room.
pub fn render_sources_list<'a>(sources: impl IntoIterator<Item = (&'a str, &'a str)>) -> String {
    let mut list = String::from("\n\nSources:");
    for (index, (title, url)) in sources.into_iter().enumerate() {
        let n = index + 1;
        let title = escape_markdown_link_text(title);
        match sanitize_link_url(url) {
            // A source that arrived with no title still renders as a usable link by showing the
            // URL as the link text, rather than an empty `[]( )` label.
            Some(url) if title.is_empty() => list.push_str(&format!("\n[{n}] [{url}]({url})")),
            Some(url) => list.push_str(&format!("\n[{n}] [{title}]({url})")),
            None if !title.is_empty() => list.push_str(&format!("\n[{n}] {title}")),
            None => list.push_str(&format!("\n[{n}] (source unavailable)")),
        }
    }
    list
}

/// Escapes the characters that would let citation title text break out of a markdown link label,
/// and folds newlines to spaces so a multi-line title cannot inject extra markdown structure.
fn escape_markdown_link_text(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('[', "\\[")
        .replace(']', "\\]")
        .replace(['\r', '\n'], " ")
}

/// Returns the URL as a markdown link target only when it is a clean `http(s)` URL with no
/// characters that would break the `(...)` destination or smuggle a different scheme. Parentheses
/// (common in Wikipedia URLs) are percent-encoded instead, which keeps the same target. Anything
/// else returns `None`, so the caller renders the citation as plain text instead of a link.
fn sanitize_link_url(url: &str) -> Option<String> {
    let url = url.trim();
    let is_http = url.starts_with("https://") || url.starts_with("http://");
    let is_clean = !url.contains(['<', '>', ' ', '\t', '\r', '\n']);

    if is_http && is_clean {
        Some(url.replace('(', "%28").replace(')', "%29"))
    } else {
        None
    }
}

/// Escapes characters with a meaning in Markdown (and flattens newlines), so that user-provided text cannot change the formatting of a message.
pub fn escape_markdown(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());

    for c in text.chars() {
        match c {
            '\n' | '\r' => escaped.push(' '),
            '\\' | '`' | '*' | '_' | '[' | ']' | '(' | ')' | '<' | '>' | '#' | '~' | '|' | '!' => {
                escaped.push('\\');
                escaped.push(c);
            }
            _ => escaped.push(c),
        }
    }

    escaped
}

/// Formats user-provided text as inline code, so it cannot change the formatting of a message.
///
/// Text which cannot be safely put in a code span (containing backticks or newlines) is escaped instead.
pub fn inline_code(text: &str) -> String {
    if text.contains(['`', '\n', '\r']) {
        escape_markdown(text)
    } else {
        format!("`{text}`")
    }
}

#[cfg(test)]
mod tests {
    use super::render_sources_list;

    #[test]
    fn parentheses_in_source_urls_are_encoded_not_dropped() {
        let list = render_sources_list([(
            "Deno (software)",
            "https://en.wikipedia.org/wiki/Deno_(software)",
        )]);

        assert_eq!(
            list,
            "\n\nSources:\n[1] [Deno (software)](https://en.wikipedia.org/wiki/Deno_%28software%29)"
        );
    }

    #[test]
    fn inline_code() {
        assert_eq!(super::inline_code("@*:example.com"), "`@*:example.com`");
        assert_eq!(super::inline_code("a`b"), "a\\`b");
    }
}
