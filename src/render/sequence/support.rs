use super::*;

pub(super) fn render_participant_group_box(
    out: &mut String,
    group: &crate::scene::GroupBox,
    scene: &Scene,
) {
    let fill = group
        .color
        .as_deref()
        .and_then(normalize_message_color)
        .unwrap_or(scene.style.group_background_color.as_str());
    out.push_str(&format!(
        "<rect class=\"sequence-participant-group\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"4\" ry=\"4\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
        group.x,
        group.y,
        group.width,
        group.height,
        fill,
        scene.style.group_border_color
    ));
    if let Some(label) = &group.label {
        out.push_str(&creole_text(
            group.x + 8,
            group.y + 16,
            "font-family=\"monospace\" font-size=\"12\" font-weight=\"600\"",
            label,
            "#333",
        ));
    }
}

pub(super) fn render_sequence_metadata_label(
    out: &mut String,
    label: &crate::scene::Label,
    class_name: &str,
    attrs: &str,
    color: &str,
    line_gap: i32,
) {
    out.push_str(&format!("<g class=\"{}\">", escape_text(class_name)));
    for (idx, line) in label.lines.iter().enumerate() {
        let y = label.y + (idx as i32 * line_gap);
        let semantic_attrs =
            sequence_label_attrs("diagram", class_name, label.x, y, line, 12, false);
        out.push_str(&creole_text(
            label.x,
            y,
            &format!("class=\"puml-label\" {semantic_attrs} {attrs}"),
            line,
            color,
        ));
    }
    out.push_str("</g>");
}

pub(super) fn sequence_message_line_style_attrs(m: &crate::scene::MessageLine) -> String {
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

pub(super) fn geometry_bbox(x: i32, y: i32, w: i32, h: i32) -> SceneRect {
    SceneRect::new(x as f64, y as f64, w as f64, h as f64)
}

pub(super) fn sequence_label_attrs(
    owner: &str,
    label_kind: &str,
    x: i32,
    y: i32,
    text: &str,
    font_size: i32,
    middle_anchor: bool,
) -> String {
    let bbox = estimate_text_bbox(x as f64, y as f64, text, font_size as f64, middle_anchor);
    super::super::puml_label_attrs(owner, label_kind, bbox)
}

pub(super) fn sequence_message_id(m: &crate::scene::MessageLine) -> String {
    format!("message:{}:{}:{}", m.from_id, m.to_id, m.route_y)
}

pub(super) fn sequence_note_id(idx: usize, note: &crate::scene::NoteBox) -> String {
    match &note.target_id {
        Some(target) => format!("note:{idx}:{target}"),
        None => format!("note:{idx}"),
    }
}

pub(super) fn render_sequence_message_edge_hook(
    out: &mut String,
    m: &crate::scene::MessageLine,
    is_self_loop: bool,
) {
    let edge_id = sequence_message_id(m);
    let puml_attrs =
        super::super::puml_edge_attrs(&edge_id, "sequence", "message", &m.from_id, &m.to_id);
    let (x1, y1, x2, y2) = if is_self_loop {
        (m.x1, m.route_y, m.x1, m.route_y + 32)
    } else {
        (m.x1, m.route_y, m.x2, m.route_y)
    };
    out.push_str(&format!(
        "<line class=\"puml-edge\" data-sequence-from=\"{}\" data-sequence-to=\"{}\" {} x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"none\" pointer-events=\"none\"/>",
        escape_text(&m.from_id),
        escape_text(&m.to_id),
        puml_attrs,
        x1,
        y1,
        x2,
        y2
    ));
}

pub(super) fn is_response_message_arrow(arrow: &str) -> bool {
    arrow.contains("--")
}

pub(super) fn render_sequence_arrow_heads(
    out: &mut String,
    m: &crate::scene::MessageLine,
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

pub(super) struct ArrowHeadRender<'a> {
    point: (i32, i32),
    from_to_x: (i32, i32),
    open: bool,
    slant: Option<char>,
    colors: (&'a str, &'a str),
    stroke_width: f32,
    hidden: &'a str,
}

pub(super) fn render_arrow_head(out: &mut String, head: ArrowHeadRender<'_>) {
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

pub(super) fn sequence_arrow_head_slant(raw_arrow: &str, left: bool) -> Option<char> {
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

pub(super) fn render_arrow_endpoint_marker(
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

pub(super) fn compute_svg_dimensions(scene: &Scene) -> (String, String, String) {
    let w = scene.width;
    let h = scene.height;
    let viewbox = format!("0 0 {} {}", w, h);
    match &scene.scale {
        None => (w.to_string(), h.to_string(), viewbox),
        Some(ScaleSpec::Factor(f)) => {
            let sw = (w as f64 * f).round() as i32;
            let sh = (h as f64 * f).round() as i32;
            (sw.to_string(), sh.to_string(), viewbox)
        }
        Some(ScaleSpec::Fixed {
            width: fw,
            height: fh,
        }) => (fw.to_string(), fh.to_string(), viewbox),
        Some(ScaleSpec::Max(max)) => {
            let max = *max as f64;
            let larger = (w.max(h)) as f64;
            if larger <= max {
                (w.to_string(), h.to_string(), viewbox)
            } else {
                let factor = max / larger;
                let sw = (w as f64 * factor).round() as i32;
                let sh = (h as f64 * factor).round() as i32;
                (sw.to_string(), sh.to_string(), viewbox)
            }
        }
    }
}

pub(super) fn render_legend(out: &mut String, text: &str, scene: &Scene) {
    let lines: Vec<&str> = text.lines().collect();
    let line_count = lines.len().max(1) as i32;
    let max_line_width = lines
        .iter()
        .map(|line| (line.chars().count() as i32) * 7)
        .max()
        .unwrap_or(0);
    let box_width = (max_line_width + 16).max(200);
    let box_height = 24 + line_count * 16;
    let margin = 10_i32;

    let x = match scene.legend_halign {
        LegendHAlign::Left => margin,
        LegendHAlign::Center => (scene.width - box_width) / 2,
        LegendHAlign::Right => scene.width - box_width - margin,
    };
    let y = match scene.legend_valign {
        LegendVAlign::Top => margin,
        LegendVAlign::Bottom => scene.height - box_height - margin,
    };

    out.push_str(&format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"4\" ry=\"4\" fill=\"#fffff0\" stroke=\"#aaa\" stroke-width=\"1\" opacity=\"0.9\"/>",
        x, y, box_width, box_height
    ));

    let mut ty = y + 16;
    for line in &lines {
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\" fill=\"#333\">{}</text>",
            x + 8,
            ty,
            escape_text(line)
        ));
        ty += 16;
    }
}

pub(super) fn render_sequence_note_shape(
    out: &mut String,
    note: &NoteBox,
    scene: &Scene,
    note_id: &str,
) {
    let x = note.x;
    let y = note.y;
    let width = note.width;
    let height = note.height;
    let fill = &scene.style.note_background_color;
    let stroke = &scene.style.note_border_color;
    let puml_attrs = super::super::puml_node_attrs(
        note_id,
        "sequence",
        "note",
        geometry_bbox(x, y, width, height),
    );
    match note.kind {
        NoteKind::Folded => {
            let fold = 14.min(width / 4).min(height / 3).max(8);
            out.push_str(&format!(
                "<path class=\"sequence-note puml-node\" {} d=\"M{x},{y} H{} L{} {} V{} H{x} Z\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                puml_attrs,
                x + width - fold,
                x + width,
                y + fold,
                y + height,
                fill,
                stroke
            ));
            out.push_str(&format!(
                "<path d=\"M{} {y} V{} H{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1\"/>",
                x + width - fold,
                y + fold,
                x + width,
                stroke
            ));
        }
        NoteKind::Hexagonal => {
            let cut = 16.min(width / 5).max(8);
            out.push_str(&format!(
                "<polygon class=\"sequence-note puml-node\" {} points=\"{},{} {},{} {},{} {},{} {},{} {},{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                puml_attrs,
                x + cut,
                y,
                x + width - cut,
                y,
                x + width,
                y + height / 2,
                x + width - cut,
                y + height,
                x + cut,
                y + height,
                x,
                y + height / 2,
                fill,
                stroke
            ));
        }
        NoteKind::Rectangle => {
            out.push_str(&format!(
                "<rect class=\"sequence-note puml-node\" {} x=\"{x}\" y=\"{y}\" width=\"{width}\" height=\"{height}\" rx=\"0\" ry=\"0\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                puml_attrs,
                fill,
                stroke
            ));
        }
    }
}

pub(super) fn render_participant_box(
    out: &mut String,
    participant: &ParticipantBox,
    scene: &Scene,
) {
    let x = participant.x;
    let y = participant.y;
    let width = participant.width;
    let height = participant.height;
    let display_lines = &participant.display_lines;
    let cx = x + (width / 2);
    let puml_attrs = super::super::puml_node_attrs(
        &participant.id,
        "sequence",
        "participant",
        geometry_bbox(x, y, width, height),
    );
    out.push_str(&format!(
        "<g class=\"sequence-participant puml-node\" data-participant=\"{}\" {}>",
        escape_text(&participant.id),
        puml_attrs
    ));

    match participant.role {
        ParticipantRole::Participant => {
            let rx = scene.style.round_corner;
            let shadow_attr = if scene.style.shadowing {
                " filter=\"url(#shadow)\""
            } else {
                ""
            };
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" ry=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"{shadow_attr}/>",
                x,
                y,
                width,
                height,
                rx,
                rx,
                scene.style.participant_background_color,
                scene.style.participant_border_color,
                shadow_attr = shadow_attr
            ));
        }
        ParticipantRole::Actor => {
            // Canonical actor stick-figure (issue #715). The rounded rect provides the
            // coloured background; the canonical helper renders the figure centred in the
            // left icon area (cx = x+12) with cy = y+16 so proportions match family.rs.
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"8\" ry=\"8\" fill=\"#fff3e0\" stroke=\"#8a5a00\" stroke-width=\"1\"/>",
                x, y, width, height
            ));
            render_actor_stick_figure(out, x + 12, y + 16, "#8a5a00");
        }
        ParticipantRole::Boundary => {
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"#eef8ff\" stroke=\"#1b5e8a\" stroke-width=\"1\" stroke-dasharray=\"5 3\"/>",
                x, y, width, height
            ));
            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#1b5e8a\" stroke-width=\"1\"/>",
                x + 6,
                y + 4,
                x + 6,
                y + height - 4
            ));
            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#1b5e8a\" stroke-width=\"1\"/>",
                x + width - 6,
                y + 4,
                x + width - 6,
                y + height - 4
            ));
        }
        ParticipantRole::Control => {
            out.push_str(&format!(
                "<polygon points=\"{},{} {},{} {},{} {},{} {},{} {},{}\" fill=\"#edf7ed\" stroke=\"#2d6a2d\" stroke-width=\"1\"/>",
                x + 10,
                y,
                x + width - 10,
                y,
                x + width,
                y + height / 2,
                x + width - 10,
                y + height,
                x + 10,
                y + height,
                x,
                y + height / 2
            ));
            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#2d6a2d\" stroke-width=\"1\"/>",
                x + 10,
                y + height / 2,
                x + width - 10,
                y + height / 2
            ));
        }
        ParticipantRole::Entity => {
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"3\" ry=\"3\" fill=\"#f4f0ff\" stroke=\"#4e3a8f\" stroke-width=\"1\"/>",
                x, y, width, height
            ));
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"2\" ry=\"2\" fill=\"none\" stroke=\"#4e3a8f\" stroke-width=\"1\"/>",
                x + 4,
                y + 4,
                width - 8,
                height - 8
            ));
            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#4e3a8f\" stroke-width=\"1\"/>",
                x + 6,
                y + 12,
                x + width - 6,
                y + 12
            ));
        }
        ParticipantRole::Database => {
            out.push_str(&format!(
                "<ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"6\" fill=\"#e9f5ff\" stroke=\"#1b5e8a\" stroke-width=\"1\"/>",
                cx,
                y + 6,
                (width / 2) - 2
            ));
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"#e9f5ff\" stroke=\"#1b5e8a\" stroke-width=\"1\"/>",
                x + 2,
                y + 6,
                width - 4,
                height - 12
            ));
            out.push_str(&format!(
                "<ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"6\" fill=\"#e9f5ff\" stroke=\"#1b5e8a\" stroke-width=\"1\"/>",
                cx,
                y + height - 6,
                (width / 2) - 2
            ));
        }
        ParticipantRole::Collections => {
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"3\" ry=\"3\" fill=\"#fff9e8\" stroke=\"#8a6d1b\" stroke-width=\"1\"/>",
                x, y + 4, width, height - 4
            ));
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"2\" ry=\"2\" fill=\"#fff9e8\" stroke=\"#8a6d1b\" stroke-width=\"1\"/>",
                x + 8, y, 24, 8
            ));
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"2\" ry=\"2\" fill=\"#fff9e8\" stroke=\"#8a6d1b\" stroke-width=\"1\"/>",
                x + 14, y + 2, 24, 8
            ));
        }
        ParticipantRole::Queue => {
            // Render queue as a horizontal cylinder (pipe) icon with neutral blue palette,
            // consistent with other shaped participants (Database, Boundary, etc.).
            // The horizontal stripes suggest a FIFO queue visually.
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"4\" ry=\"4\" fill=\"#e9f5ff\" stroke=\"#1b5e8a\" stroke-width=\"1\"/>",
                x, y, width, height
            ));
            // Left ellipse cap — suggests pipe/cylinder opening
            out.push_str(&format!(
                "<ellipse cx=\"{}\" cy=\"{}\" rx=\"6\" ry=\"{}\" fill=\"#d0eaff\" stroke=\"#1b5e8a\" stroke-width=\"1\"/>",
                x + 10,
                y + height / 2,
                (height / 2) - 4
            ));
            // Right ellipse cap — other end of cylinder
            out.push_str(&format!(
                "<ellipse cx=\"{}\" cy=\"{}\" rx=\"6\" ry=\"{}\" fill=\"#d0eaff\" stroke=\"#1b5e8a\" stroke-width=\"1\"/>",
                x + width - 10,
                y + height / 2,
                (height / 2) - 4
            ));
        }
    }

    let participant_font_color = scene.style.participant_font_color_resolved();
    for (idx, line) in display_lines.iter().enumerate() {
        let label_y = y + 21 + (idx as i32 * 16);
        let attrs = sequence_label_attrs(
            &participant.id,
            "participant-label",
            cx,
            label_y,
            line,
            13,
            true,
        );
        out.push_str(&creole_text(
            cx,
            label_y,
            &format!(
                "class=\"puml-label\" {attrs} text-anchor=\"middle\" font-family=\"monospace\" font-size=\"13\""
            ),
            line,
            participant_font_color,
        ));
    }
    out.push_str("</g>");
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
    }
}
