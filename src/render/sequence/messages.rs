use crate::model::VirtualEndpointKind;
use crate::scene::MessageLine;
use crate::theme::{css3_color_to_hex, MessageAlign};

pub(super) fn sequence_message_line_style_attrs(m: &MessageLine) -> String {
    if m.style.color.is_none()
        && !m.style.hidden
        && !m.style.dashed
        && !m.style.dotted
        && m.style.thickness.is_none()
    {
        return String::new();
    }

    let mut classes = Vec::new();
    let mut styles = Vec::new();
    let renders_dashed = m.style.dashed || (!m.style.dotted && m.arrow.contains("--"));
    if m.style.color.is_some() {
        classes.push("sequence-message-line-colored");
        styles.push("color");
    }
    if m.style.dotted {
        classes.push("sequence-message-line-dotted");
        styles.push("dotted");
    } else if renders_dashed {
        classes.push("sequence-message-line-dashed");
        styles.push("dashed");
    }
    if m.style.hidden {
        classes.push("sequence-message-line-hidden");
        styles.push("hidden");
    }
    if m.style.thickness.is_some() {
        classes.push("sequence-message-line-thick");
        styles.push("thickness");
    }
    if classes.is_empty() {
        return String::new();
    }

    format!(
        " class=\"sequence-message-line {}\" data-sequence-message-style=\"{}\"",
        classes.join(" "),
        styles.join(" ")
    )
}

pub(super) fn sequence_message_label_anchor(
    x1: i32,
    x2: i32,
    align: MessageAlign,
) -> (i32, &'static str) {
    let left = x1.min(x2);
    let right = x1.max(x2);
    match align {
        MessageAlign::Left => (left + 8, "start"),
        MessageAlign::Center => (((x1 + x2) / 2) + 2, "middle"),
        MessageAlign::Right => (right - 8, "end"),
    }
}

pub(super) fn is_response_message_arrow(arrow: &str) -> bool {
    arrow.contains("--")
}

pub(super) fn render_sequence_arrow_heads(
    out: &mut String,
    m: &MessageLine,
    stroke_color: &str,
    fill_color: &str,
    stroke_width: f32,
    hidden: &str,
) {
    let head_stroke_width = m
        .style
        .thickness
        .map(f32::from)
        .unwrap_or(1.0)
        .clamp(1.0, 8.0);
    let raw_arrow = m.arrow.as_str();
    let arrow = raw_arrow.replace(['/', '\\'], "");
    let left_marker = arrow.chars().next().filter(|c| matches!(c, 'o' | 'x'));
    let right_marker = arrow.chars().last().filter(|c| matches!(c, 'o' | 'x'));
    let left_arrow = arrow.starts_with('<') || arrow.starts_with("<<");
    let left_slant = sequence_arrow_head_slant(raw_arrow, true);
    let right_slant = sequence_arrow_head_slant(raw_arrow, false);
    let right_arrow =
        (arrow.contains('>') || right_slant.is_some()) && !matches!(right_marker, Some('o' | 'x'));
    let open_head = arrow.contains(">>") || arrow.contains("<<");

    if left_arrow {
        render_arrow_head(
            out,
            ArrowHeadRender {
                point: (m.x1, m.route_y),
                from_to_x: (m.x2, m.x1),
                open: open_head,
                slant: left_slant,
                colors: (stroke_color, fill_color),
                stroke_width: head_stroke_width,
                hidden,
            },
        );
    }
    if right_arrow {
        render_arrow_head(
            out,
            ArrowHeadRender {
                point: (m.x2, m.route_y),
                from_to_x: (m.x1, m.x2),
                open: open_head,
                slant: right_slant,
                colors: (stroke_color, fill_color),
                stroke_width: head_stroke_width,
                hidden,
            },
        );
    }
    if let Some(marker) = left_marker {
        render_arrow_endpoint_marker(
            out,
            m.x1,
            m.route_y,
            marker,
            stroke_color,
            stroke_width,
            hidden,
        );
    }
    if let Some(marker) = right_marker {
        render_arrow_endpoint_marker(
            out,
            m.x2,
            m.route_y,
            marker,
            stroke_color,
            stroke_width,
            hidden,
        );
    }
}

struct ArrowHeadRender<'a> {
    point: (i32, i32),
    from_to_x: (i32, i32),
    open: bool,
    slant: Option<char>,
    colors: (&'a str, &'a str),
    stroke_width: f32,
    hidden: &'a str,
}

fn render_arrow_head(out: &mut String, head: ArrowHeadRender<'_>) {
    let (x, y) = head.point;
    let (from_x, to_x) = head.from_to_x;
    let (stroke_color, fill_color) = head.colors;
    let dir = if to_x >= from_x { 1 } else { -1 };
    let back = x - (dir * 8);
    if let Some(slant) = head.slant {
        let back_y = match slant {
            '/' => y + (dir * 5),
            '\\' => y - (dir * 5),
            _ => y,
        };
        let slant_name = if slant == '/' { "slash" } else { "backslash" };
        out.push_str(&format!(
            "<line class=\"sequence-arrow-head sequence-arrow-head-{}\" x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{}/>",
            slant_name,
            back,
            back_y,
            x,
            y,
            stroke_color,
            head.stroke_width,
            head.hidden
        ));
    } else if head.open {
        out.push_str(&format!(
            "<polyline points=\"{},{} {},{} {},{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"{}/>",
            back,
            y - 5,
            x,
            y,
            back,
            y + 5,
            stroke_color,
            head.stroke_width,
            head.hidden
        ));
    } else {
        out.push_str(&format!(
            "<polygon points=\"{},{} {},{} {},{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{}/>",
            x,
            y,
            back,
            y - 5,
            back,
            y + 5,
            fill_color,
            stroke_color,
            head.stroke_width,
            head.hidden
        ));
    }
}

fn sequence_arrow_head_slant(raw_arrow: &str, left: bool) -> Option<char> {
    let mut marker = None;
    let mut saw_head = false;
    for ch in raw_arrow.chars() {
        if matches!(ch, '/' | '\\') {
            marker = Some(ch);
            continue;
        }
        if left && ch == '<' {
            return marker;
        }
        if !left && ch == '>' {
            saw_head = true;
        }
        if saw_head && matches!(ch, '/' | '\\') {
            return Some(ch);
        }
    }
    if left {
        None
    } else {
        marker
    }
}

fn render_arrow_endpoint_marker(
    out: &mut String,
    x: i32,
    y: i32,
    marker: char,
    stroke_color: &str,
    stroke_width: f32,
    hidden: &str,
) {
    match marker {
        'o' => out.push_str(&format!(
            "<circle class=\"sequence-arrow-end sequence-arrow-end-circle\" data-sequence-arrow-end=\"circle\" cx=\"{}\" cy=\"{}\" r=\"4\" fill=\"white\" stroke=\"{}\" stroke-width=\"{}\"{}/>",
            x, y, stroke_color, stroke_width, hidden
        )),
        'x' => out.push_str(&format!(
            "<g class=\"sequence-arrow-end sequence-arrow-end-cross\" data-sequence-arrow-end=\"cross\" stroke=\"{}\" stroke-width=\"{}\"{}><line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\"/><line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\"/></g>",
            stroke_color,
            stroke_width,
            hidden,
            x - 4,
            y - 4,
            x + 4,
            y + 4,
            x - 4,
            y + 4,
            x + 4,
            y - 4
        )),
        _ => {}
    }
}

pub(super) fn normalize_message_color(value: &str) -> Option<&str> {
    let value = value.trim();
    if value.starts_with('#') {
        return Some(value);
    }
    css3_color_to_hex(value).or(Some(value))
}

pub(super) fn render_virtual_endpoint_marker(
    out: &mut String,
    x: i32,
    y: i32,
    kind: VirtualEndpointKind,
) {
    match kind {
        VirtualEndpointKind::Plain => {
            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#111\" stroke-width=\"1.5\"/>",
                x,
                y - 6,
                x,
                y + 6
            ));
        }
        VirtualEndpointKind::Circle => {
            out.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"4\" fill=\"white\" stroke=\"#111\" stroke-width=\"1.5\"/>",
                x, y
            ));
        }
        VirtualEndpointKind::Cross => {
            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#111\" stroke-width=\"1.5\"/>",
                x - 4,
                y - 4,
                x + 4,
                y + 4
            ));
            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#111\" stroke-width=\"1.5\"/>",
                x - 4,
                y + 4,
                x + 4,
                y - 4
            ));
        }
        VirtualEndpointKind::Filled => {
            out.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"4\" fill=\"#111\" stroke=\"#111\" stroke-width=\"1.5\"/>",
                x, y
            ));
        }
        VirtualEndpointKind::Short => {
            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#111\" stroke-width=\"1.5\" stroke-dasharray=\"3 2\"/>",
                x - 6,
                y,
                x + 6,
                y
            ));
        }
    }
}
