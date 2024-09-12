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
