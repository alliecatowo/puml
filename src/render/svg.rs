use crate::creole::{render_creole_to_svg_tspans, tokenize_creole};

pub(crate) fn creole_text(
    x: i32,
    y: i32,
    extra_attrs: &str,
    label: &str,
    base_color: &str,
) -> String {
    let lines = tokenize_creole(label);
    let label_lower = label.to_ascii_lowercase();
    let has_markup = label.contains("**")
        || label.contains("//")
        || label.contains("\"\"")
        || label.contains("__")
        || label.contains("--")
        || label.contains("[[")
        || label_lower.contains("<color")
        || label_lower.contains("</color")
        || label_lower.contains("<size")
        || label_lower.contains("</size")
        || label_lower.contains("<font")
        || label_lower.contains("</font")
        || label_lower.contains("<b>")
        || label_lower.contains("</b>")
        || label_lower.contains("<i>")
        || label_lower.contains("</i>")
        || label_lower.contains("<u>")
        || label_lower.contains("</u>")
        || label.contains("<&");

    if !has_markup && lines.len() == 1 {
        // Fast path — no markup, single line: emit fill when the color is non-default
        // and extra_attrs does not already carry a fill (avoids duplicate attributes).
        let color_attr = if !base_color.is_empty()
            && base_color != "black"
            && base_color != "#000000"
            && base_color != "#000"
            && !extra_attrs.contains("fill=")
        {
            format!(" fill=\"{}\"", base_color)
        } else {
            String::new()
        };
        let attrs = if extra_attrs.is_empty() {
            color_attr
        } else {
            format!(" {}{}", extra_attrs, color_attr)
        };
        return format!(
            "<text x=\"{}\" y=\"{}\"{}>{}</text>",
            x,
            y,
            attrs,
            escape_text(label)
        );
    }

    let inner = render_creole_to_svg_tspans(&lines, x, base_color);
    let color_attr = if !base_color.is_empty()
        && base_color != "black"
        && base_color != "#000000"
        && base_color != "#000"
        && !extra_attrs.contains("fill=")
    {
        format!(" fill=\"{}\"", base_color)
    } else {
        String::new()
    };
    format!(
        "<text x=\"{}\" y=\"{}\"{}>{}</text>",
        x,
        y,
        if extra_attrs.is_empty() {
            color_attr
        } else {
            format!(" {}{}", extra_attrs, color_attr)
        },
        inner
    )
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
