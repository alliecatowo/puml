// ─── Shared SVG utilities ─────────────────────────────────────────────────────

pub(super) fn escape_xml(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    out
}

pub(super) fn svg_header(width: i32, height: i32) -> String {
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        width, height, width, height
    )
}

pub(super) fn svg_white_bg() -> &'static str {
    "<rect width=\"100%\" height=\"100%\" fill=\"white\"/>"
}

/// Strip the @start.../@end... wrapper and return body lines and optional title.
pub(super) fn strip_block<'a>(
    source: &'a str,
    start_tag: &str,
    end_tag: &str,
) -> (&'a str, Option<String>) {
    let mut lines = source.lines();
    let first_line = lines.next().unwrap_or("").trim();
    // consume @startXXX line, possibly with a title
    let tag_lower = first_line.to_ascii_lowercase();
    let rest_after_tag = if tag_lower.starts_with(start_tag) {
        first_line[start_tag.len()..].trim()
    } else {
        first_line
    };
    let title: Option<String> = if rest_after_tag.starts_with('"') {
        Some(rest_after_tag.trim_matches('"').to_string())
    } else if !rest_after_tag.is_empty() {
        Some(rest_after_tag.to_string())
    } else {
        None
    };

    // The body is everything between the @start and @end lines
    let body_start = first_line.len() + 1; // skip first line + newline
    let body_end = if let Some(pos) = source.to_ascii_lowercase().rfind(end_tag) {
        // go back to find start of that line
        let before = &source[..pos];
        before.rfind('\n').map(|i| i + 1).unwrap_or(0)
    } else {
        source.len()
    };

    let body = source
        .get(body_start.min(source.len())..body_end.min(source.len()))
        .unwrap_or("");
    (body, title)
}
