use crate::creole::{render_creole_to_svg_tspans, tokenize_creole};

pub(crate) fn creole_text(
    x: i32,
    y: i32,
    extra_attrs: &str,
    label: &str,
    base_color: &str,
) -> String {
    let lines = tokenize_creole(label);
    let has_markup = label.contains("**")
        || label.contains("//")
        || label.contains("\"\"")
        || label.contains("__")
        || label.contains("--")
        || label.contains("[[")
        || label.contains("<color")
        || label.contains("<size")
        || label.contains("<b>")
        || label.contains("<B>")
        || label.contains("<i>")
        || label.contains("<I>")
        || label.contains("<u>")
        || label.contains("<U>")
        || label.contains("<&");

    if !has_markup && lines.len() == 1 {
        // Fast path — no markup, no multi-line: keep old behavior.
        return format!(
            "<text x=\"{}\" y=\"{}\"{}>{}",
            x,
            y,
            if extra_attrs.is_empty() {
                String::new()
            } else {
                format!(" {}", extra_attrs)
            },
            escape_text(label)
        ) + "</text>";
    }

    let inner = render_creole_to_svg_tspans(&lines, x, base_color);
    format!(
        "<text x=\"{}\" y=\"{}\"{}>{}",
        x,
        y,
        if extra_attrs.is_empty() {
            String::new()
        } else {
            format!(" {}", extra_attrs)
        },
        inner
    ) + "</text>"
}

pub(crate) fn escape_text(input: &str) -> String {
    let mut escaped = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            _ => escaped.push(ch),
        }
    }
    escaped
}
