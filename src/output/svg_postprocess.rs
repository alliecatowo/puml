use crate::model::ScaleSpec;
use crate::{
    creole::{render_creole_to_svg_tspans, tokenize_creole},
    text_markup::escape_svg_text,
};

pub fn append_mainframe_svg(svg: &mut String, title: &str) {
    let Some(width) = svg_numeric_attr(svg, "width") else {
        return;
    };
    let Some(height) = svg_numeric_attr(svg, "height") else {
        return;
    };
    let Some(insert_at) = svg.rfind("</svg>") else {
        return;
    };
    if width <= 8 || height <= 8 {
        return;
    }

    const INSET: i32 = 4;
    const NOTCH_H: i32 = 20;
    const NOTCH_CUT: i32 = 6;
    let notch_w = ((title.chars().count() as i32 * 7) + 16).clamp(32, width - 2 * INSET);
    let stroke = "#1e293b";
    let fill = "#ffffff";
    let x = INSET;
    let y = INSET;
    let w = width - 2 * INSET;
    let h = height - 2 * INSET;

    let mut frame = format!(
        "<rect class=\"uml-mainframe\" x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" fill=\"none\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>"
    );
    frame.push_str(&format!(
        "<polygon class=\"uml-mainframe-title\" points=\"{},{} {},{} {},{} {},{} {},{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
        x,
        y,
        x + notch_w,
        y,
        x + notch_w,
        y + NOTCH_H - NOTCH_CUT,
        x + notch_w - NOTCH_CUT,
        y + NOTCH_H,
        x,
        y + NOTCH_H,
        fill,
        stroke
    ));
    if !title.is_empty() {
        frame.push_str(&svg_text(
            x + 8,
            y + 14,
            "font-family=\"monospace\" font-size=\"12\" font-weight=\"600\"",
            title,
            stroke,
        ));
    }
    svg.insert_str(insert_at, &frame);
}

pub fn append_optional_mainframe_svg(svg: &mut String, title: Option<&str>) {
    if let Some(title) = title {
        append_mainframe_svg(svg, title);
    }
}

fn svg_text(x: i32, y: i32, extra_attrs: &str, label: &str, base_color: &str) -> String {
    let lines = tokenize_creole(label);
    if !label_has_markup(label) && lines.len() == 1 {
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
            escape_svg_text(label)
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

fn label_has_markup(label: &str) -> bool {
    let label_lower = label.to_ascii_lowercase();
    label.contains("**")
        || label.contains("//")
        || label.contains("\"\"")
        || label.contains("__")
        || label.contains("--")
        || label.contains("~~")
        || label.contains('~')
        || label.contains("[[")
        || label_lower.contains("<color")
        || label_lower.contains("</color")
        || label_lower.contains("<size")
        || label_lower.contains("</size")
        || label_lower.contains("<font")
        || label_lower.contains("</font")
        || label_lower.contains("<back")
        || label_lower.contains("</back")
        || label_lower.contains("<code>")
        || label_lower.contains("</code>")
        || label_lower.contains("<plain>")
        || label_lower.contains("</plain>")
        || label_lower.contains("<b>")
        || label_lower.contains("</b>")
        || label_lower.contains("<i>")
        || label_lower.contains("</i>")
        || label_lower.contains("<u>")
        || label_lower.contains("<u:")
        || label_lower.contains("</u>")
        || label_lower.contains("<s>")
        || label_lower.contains("<s:")
        || label_lower.contains("</s>")
        || label_lower.contains("<w>")
        || label_lower.contains("<w:")
        || label_lower.contains("</w>")
        || label_lower.contains("<sub>")
        || label_lower.contains("</sub>")
        || label_lower.contains("<sup>")
        || label_lower.contains("</sup>")
        || label_lower.contains("<img:")
        || label_lower.contains("<br")
        || label_lower.contains("<strong>")
        || label_lower.contains("</strong>")
        || label_lower.contains("<em>")
        || label_lower.contains("</em>")
        || label_lower.contains("<del>")
        || label_lower.contains("</del>")
        || label_lower.contains("<strike>")
        || label_lower.contains("</strike>")
        || label_lower.contains("<tt>")
        || label_lower.contains("</tt>")
}

pub fn apply_scale_svg(svg: &mut String, scale: &ScaleSpec) {
    let Some(width) = svg_numeric_attr(svg, "width") else {
        return;
    };
    let Some(height) = svg_numeric_attr(svg, "height") else {
        return;
    };
    if width <= 0 || height <= 0 {
        return;
    }

    let (scaled_width, scaled_height) = scaled_svg_dimensions(width, height, scale);
    replace_svg_numeric_attr(svg, "width", scaled_width);
    replace_svg_numeric_attr(svg, "height", scaled_height);
}

fn scaled_svg_dimensions(width: i32, height: i32, scale: &ScaleSpec) -> (i32, i32) {
    let scaled = match scale {
        ScaleSpec::Factor(factor) => (
            (width as f64 * factor).round() as i32,
            (height as f64 * factor).round() as i32,
        ),
        ScaleSpec::Width(target_width) => {
            let factor = *target_width as f64 / width as f64;
            (
                *target_width as i32,
                (height as f64 * factor).round() as i32,
            )
        }
        ScaleSpec::Height(target_height) => {
            let factor = *target_height as f64 / height as f64;
            (
                (width as f64 * factor).round() as i32,
                *target_height as i32,
            )
        }
        ScaleSpec::Fixed { width, height } => (*width as i32, *height as i32),
        ScaleSpec::Max(max) => {
            let max = *max as f64;
            let larger = width.max(height) as f64;
            if larger <= max {
                (width, height)
            } else {
                let factor = max / larger;
                (
                    (width as f64 * factor).round() as i32,
                    (height as f64 * factor).round() as i32,
                )
            }
        }
        ScaleSpec::MaxWidth(max_width) => {
            if width <= *max_width as i32 {
                (width, height)
            } else {
                let factor = *max_width as f64 / width as f64;
                (*max_width as i32, (height as f64 * factor).round() as i32)
            }
        }
        ScaleSpec::MaxHeight(max_height) => {
            if height <= *max_height as i32 {
                (width, height)
            } else {
                let factor = *max_height as f64 / height as f64;
                ((width as f64 * factor).round() as i32, *max_height as i32)
            }
        }
        ScaleSpec::MaxFixed {
            width: max_width,
            height: max_height,
        } => {
            if width <= *max_width as i32 && height <= *max_height as i32 {
                (width, height)
            } else {
                let factor =
                    (*max_width as f64 / width as f64).min(*max_height as f64 / height as f64);
                (
                    (width as f64 * factor).round() as i32,
                    (height as f64 * factor).round() as i32,
                )
            }
        }
    };
    (scaled.0.max(1), scaled.1.max(1))
}

fn svg_numeric_value_attr(svg: &str, attr: &str) -> Option<f64> {
    let needle = format!("{attr}=\"");
    let start = svg.find(&needle)? + needle.len();
    let value = svg[start..].split('"').next()?;
    value.parse::<f64>().ok()
}

fn svg_numeric_attr(svg: &str, attr: &str) -> Option<i32> {
    svg_numeric_value_attr(svg, attr).map(|v| v.round() as i32)
}

fn replace_svg_numeric_attr(svg: &mut String, attr: &str, value: i32) {
    let needle = format!("{attr}=\"");
    let Some(start) = svg.find(&needle).map(|idx| idx + needle.len()) else {
        return;
    };
    let Some(end_offset) = svg[start..].find('"') else {
        return;
    };
    svg.replace_range(start..start + end_offset, &value.to_string());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scale_updates_serialized_svg_dimensions_without_touching_viewbox() {
        let mut svg =
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"100\" height=\"50\" viewBox=\"0 0 100 50\"></svg>"
                .to_string();

        apply_scale_svg(&mut svg, &ScaleSpec::Factor(2.0));

        assert!(svg.contains("width=\"200\""));
        assert!(svg.contains("height=\"100\""));
        assert!(svg.contains("viewBox=\"0 0 100 50\""));
    }

    #[test]
    fn mainframe_is_serialized_at_svg_boundary() {
        let mut svg =
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"120\" height=\"80\"></svg>"
                .to_string();

        append_mainframe_svg(&mut svg, "Main");

        assert!(svg.contains("class=\"uml-mainframe\""));
        assert!(svg.contains("class=\"uml-mainframe-title\""));
        assert!(svg.ends_with("</svg>"));
    }
}
