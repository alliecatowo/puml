use super::{CreoleLine, CreoleSpan};
use crate::text_markup::{escape_svg_attr as escape_attr, escape_svg_text as escape_xml};

/// Render a single `CreoleLine` to SVG `<tspan>` elements.
///
/// `base_x` is the x coordinate of the text element.
/// `default_color` is used when no span-level color override is present.
/// Returns a string of concatenated `<tspan>` elements (no wrapper `<text>`).
pub fn render_creole_line_to_tspans(
    line: &CreoleLine,
    _base_x: i32,
    default_color: &str,
) -> String {
    if line.is_empty() {
        return String::new();
    }

    let mut out = String::new();
    for span in line {
        render_span(&mut out, span, default_color);
    }
    out
}

/// Render multi-line creole text into a sequence of `<tspan>` elements with
/// `dy="1.2em"` for subsequent lines. The first line sits at the caller's `y`
/// position; later lines advance by `dy`.
///
/// Returns a flat string of `<tspan>` elements. Pass this inside a `<text>` tag.
pub fn render_creole_to_svg_tspans(
    lines: &[CreoleLine],
    base_x: i32,
    default_color: &str,
) -> String {
    let mut out = String::new();
    for (i, line) in lines.iter().enumerate() {
        let dy_attr = if i == 0 {
            String::new()
        } else {
            " dy=\"1.2em\"".to_string()
        };
        let x_attr = format!(" x=\"{}\"", base_x);
        let inner = render_creole_line_to_tspans(line, base_x, default_color);
        out.push_str(&format!("<tspan{}{}>", x_attr, dy_attr));
        out.push_str(&inner);
        out.push_str("</tspan>");
    }
    out
}

fn render_span(out: &mut String, span: &CreoleSpan, default_color: &str) {
    if let Some(url) = &span.link {
        out.push_str(&format!(
            "<a xlink:href=\"{}\" xmlns:xlink=\"http://www.w3.org/1999/xlink\">",
            escape_attr(url)
        ));
        if let Some(tooltip) = &span.link_tooltip {
            out.push_str(&format!("<title>{}</title>", escape_xml(tooltip)));
        }
    }

    let mut style_parts = Vec::new();
    push_font_attrs(span, &mut style_parts);
    push_decoration_attrs(span, &mut style_parts);
    push_color_attrs(span, default_color, &mut style_parts);
    push_size_and_position_attrs(span, &mut style_parts);

    let attrs = if style_parts.is_empty() {
        String::new()
    } else {
        format!(" {}", style_parts.join(" "))
    };

    out.push_str(&format!(
        "<tspan{}>{}</tspan>",
        attrs,
        escape_xml(&span.text)
    ));

    if span.link.is_some() {
        out.push_str("</a>");
    }
}

fn push_font_attrs(span: &CreoleSpan, style_parts: &mut Vec<String>) {
    if span.bold {
        style_parts.push("font-weight=\"bold\"".to_string());
    }
    if span.italic {
        style_parts.push("font-style=\"italic\"".to_string());
    }
    if span.mono {
        style_parts.push("font-family=\"monospace\"".to_string());
    }
    if let Some(font) = &span.font {
        style_parts.push(format!("font-family=\"{}\"", escape_attr(font)));
    }
}

fn push_decoration_attrs(span: &CreoleSpan, style_parts: &mut Vec<String>) {
    let mut text_decorations = Vec::new();
    if span.underline || span.link.is_some() {
        text_decorations.push("underline");
    }
    if span.strike {
        text_decorations.push("line-through");
    }
    if span.wave {
        text_decorations.push("underline");
        style_parts.push("text-decoration-style=\"wavy\"".to_string());
    }
    if !text_decorations.is_empty() {
        style_parts.push(format!(
            "text-decoration=\"{}\"",
            text_decorations.join(" ")
        ));
    }
    if let Some(decoration_color) = &span.decoration_color {
        style_parts.push(format!(
            "text-decoration-color=\"{}\"",
            escape_attr(decoration_color)
        ));
    }
}

fn push_color_attrs(span: &CreoleSpan, default_color: &str, style_parts: &mut Vec<String>) {
    let color = if span.link.is_some() {
        "blue".to_string()
    } else if let Some(c) = &span.color {
        c.clone()
    } else {
        default_color.to_string()
    };
    if color != default_color || span.link.is_some() {
        style_parts.push(format!("fill=\"{}\"", escape_attr(&color)));
    }
}

fn push_size_and_position_attrs(span: &CreoleSpan, style_parts: &mut Vec<String>) {
    if let Some(size) = span.size {
        style_parts.push(format!("font-size=\"{}\"", size));
    }
    if let Some(background) = &span.background {
        style_parts.push(format!(
            "data-creole-back=\"{}\" style=\"background-color:{}\"",
            escape_attr(background),
            escape_attr(background)
        ));
    }
    if let Some(baseline_shift) = &span.baseline_shift {
        style_parts.push(format!(
            "baseline-shift=\"{}\"",
            escape_attr(baseline_shift)
        ));
        if span.size.is_none() {
            style_parts.push("font-size=\"80%\"".to_string());
        }
    }
}
