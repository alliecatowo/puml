use crate::creole::{decode_unicode_escapes, render_creole_to_svg_tspans, tokenize_creole};

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

/// Canonical actor stick-figure renderer used across all diagram families.
///
/// Proportions (canonical, issue #715):
///   head  r = 6   (12 px diameter)
///   body  14 px   (neck bottom to hip)
///   arms  20 px wide centred on cx, at shoulder (neck bottom + 4)
///   legs  16 px spread (each leg goes ±8 px from hip)
///
/// `cx`, `cy` are the **centre** of the figure. The full figure spans roughly
/// 44 px in height: from `cy - 21` (top of head) to `cy + 23` (feet).
/// `stroke` is the SVG stroke colour string (e.g. `"#334155"`).
pub(crate) fn render_actor_stick_figure(out: &mut String, cx: i32, cy: i32, stroke: &str) {
    // Head: centre at (cx, cy - 15) -> top of figure is cy - 21
    let head_cy = cy - 15;
    out.push_str(&format!(
        "<circle cx=\"{cx}\" cy=\"{head_cy}\" r=\"6\" fill=\"none\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>"
    ));
    // Body: from neck (head_cy + 6) to hip (head_cy + 20)
    let neck_y = head_cy + 6;
    let hip_y = head_cy + 20;
    out.push_str(&format!(
        "<line x1=\"{cx}\" y1=\"{neck_y}\" x2=\"{cx}\" y2=\"{hip_y}\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>"
    ));
    // Arms: centred on body at shoulder (neck_y + 4), spanning cx±10
    let arm_y = neck_y + 4;
    let arm_x1 = cx - 10;
    let arm_x2 = cx + 10;
    out.push_str(&format!(
        "<line x1=\"{arm_x1}\" y1=\"{arm_y}\" x2=\"{arm_x2}\" y2=\"{arm_y}\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>"
    ));
    // Legs: from hip, spread cx±8
    let leg_x_left = cx - 8;
    let leg_x_right = cx + 8;
    let leg_end_y = hip_y + 16;
    out.push_str(&format!(
        "<line x1=\"{cx}\" y1=\"{hip_y}\" x2=\"{leg_x_left}\" y2=\"{leg_end_y}\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>"
    ));
    out.push_str(&format!(
        "<line x1=\"{cx}\" y1=\"{hip_y}\" x2=\"{leg_x_right}\" y2=\"{leg_end_y}\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>"
    ));
}

pub(crate) fn escape_text(input: &str) -> String {
    let decoded = decode_unicode_escapes(input);
    let mut escaped = String::with_capacity(decoded.len());
    for ch in decoded.chars() {
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
