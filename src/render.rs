use crate::ast::DiagramKind;
use crate::model::{
    FamilyDocument, FamilyNodeKind, FamilyOrientation, ParticipantRole, TimelineDocument,
    VirtualEndpointKind,
};
use crate::scene::{ParticipantBox, Scene, StructureKind};

const MESSAGE_LABEL_LINE_GAP: i32 = 16;

pub fn render_svg(scene: &Scene) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        scene.width, scene.height, scene.width, scene.height
    ));
    out.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");

    if let Some(title) = &scene.title {
        for (idx, line) in title.lines.iter().enumerate() {
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"18\" font-weight=\"600\">{}</text>",
                title.x,
                title.y + (idx as i32 * 24),
                escape_text(line)
            ));
        }
    }

    for p in &scene.participants {
        render_participant_box(&mut out, p, scene);
    }

    for l in &scene.lifelines {
        out.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1\" stroke-dasharray=\"6 4\"/>",
            l.x, l.y1, l.x, l.y2, scene.style.lifeline_border_color
        ));
    }

    for g in &scene.groups {
        out.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"3\" ry=\"3\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
            g.x,
            g.y,
            g.width,
            g.height,
            if g.kind.eq_ignore_ascii_case("ref") {
                "#eef6ff"
            } else {
                scene.style.group_background_color.as_str()
            },
            scene.style.group_border_color
        ));

        if let Some(label) = &g.label {
            let header = label.lines().next().unwrap_or("");
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" font-weight=\"600\">{}</text>",
                g.x + 8,
                g.y + 16,
                escape_text(format!("{} {}", g.kind, header).trim())
            ));
            if g.kind.eq_ignore_ascii_case("ref") {
                let mut y = g.y + 32;
                for line in label.lines().skip(1) {
                    out.push_str(&format!(
                        "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"12\">{}</text>",
                        g.x + 8,
                        y,
                        escape_text(line)
                    ));
                    y += 16;
                }
            }
        }

        for sep in &g.separators {
            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1\" stroke-dasharray=\"5 4\"/>",
                g.x,
                sep.y,
                g.x + g.width,
                sep.y,
                scene.style.group_border_color
            ));
            if let Some(label) = &sep.label {
                out.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\" fill=\"#333\">{}</text>",
                    g.x + 8,
                    sep.y - 6,
                    escape_text(label)
                ));
            }
        }
    }

    for m in &scene.messages {
        let stroke_dash = if m.arrow.contains("--") {
            " stroke-dasharray=\"6 4\""
        } else {
            ""
        };
        out.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"{}/>",
            m.x1, m.y, m.x2, m.y, scene.style.arrow_color, stroke_dash
        ));
        let arrow_size = 6;
        if m.x2 >= m.x1 {
            out.push_str(&format!(
                "<polygon points=\"{},{} {},{} {},{}\" fill=\"{}\"/>",
                m.x2,
                m.y,
                m.x2 - arrow_size,
                m.y - 4,
                m.x2 - arrow_size,
                m.y + 4,
                scene.style.arrow_color
            ));
        } else {
            out.push_str(&format!(
                "<polygon points=\"{},{} {},{} {},{}\" fill=\"{}\"/>",
                m.x2,
                m.y,
                m.x2 + arrow_size,
                m.y - 4,
                m.x2 + arrow_size,
                m.y + 4,
                scene.style.arrow_color
            ));
        }

        if let Some(virtual_ep) = m.from_virtual {
            render_virtual_endpoint_marker(&mut out, m.x1, m.y, virtual_ep.kind);
        }
        if let Some(virtual_ep) = m.to_virtual {
            render_virtual_endpoint_marker(&mut out, m.x2, m.y, virtual_ep.kind);
        }

        if !m.label_lines.is_empty() {
            let tx = ((m.x1 + m.x2) / 2) + 2;
            let start_y = m.y - 8 - (((m.label_lines.len() as i32) - 1) * MESSAGE_LABEL_LINE_GAP);
            for (idx, line) in m.label_lines.iter().enumerate() {
                out.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"12\">{}</text>",
                    tx,
                    start_y + (idx as i32 * MESSAGE_LABEL_LINE_GAP),
                    escape_text(line)
                ));
            }
        } else if let Some(label) = &m.label {
            let tx = ((m.x1 + m.x2) / 2) + 2;
            let ty = m.y - 8;
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"12\">{}</text>",
                tx,
                ty,
                escape_text(label)
            ));
        }
    }

    for n in &scene.notes {
        out.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"3\" ry=\"3\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
            n.x, n.y, n.width, n.height, scene.style.note_background_color, scene.style.note_border_color
        ));

        let mut text_y = n.y + 20;
        for line in n.text.lines() {
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"12\">{}</text>",
                n.x + 8,
                text_y,
                escape_text(line)
            ));
            text_y += 16;
        }
    }

    for s in &scene.structures {
        match s.kind {
            StructureKind::Delay => {
                out.push_str(&format!(
                    "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#777\" stroke-width=\"1\" stroke-dasharray=\"3 7\"/>",
                    s.x1, s.y, s.x2, s.y
                ));
                if let Some(label) = &s.label {
                    out.push_str(&format!(
                        "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"#444\">{}</text>",
                        (s.x1 + s.x2) / 2,
                        s.y - 6,
                        escape_text(label)
                    ));
                }
            }
            StructureKind::Divider => {
                out.push_str(&format!(
                    "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#666\" stroke-width=\"1\" stroke-dasharray=\"8 5\"/>",
                    s.x1, s.y, s.x2, s.y
                ));
                if let Some(label) = &s.label {
                    out.push_str(&format!(
                        "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"#333\">{}</text>",
                        (s.x1 + s.x2) / 2,
                        s.y - 6,
                        escape_text(label)
                    ));
                }
            }
            StructureKind::Separator => {
                out.push_str(&format!(
                    "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#222\" stroke-width=\"1.5\"/>",
                    s.x1, s.y, s.x2, s.y
                ));
                let label = if let Some(label) = &s.label {
                    format!("== {} ==", label)
                } else {
                    "== ==".to_string()
                };
                out.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" font-weight=\"600\" fill=\"#222\">{}</text>",
                    (s.x1 + s.x2) / 2,
                    s.y - 6,
                    escape_text(&label)
                ));
            }
            StructureKind::Spacer => {}
        }
    }

    for p in &scene.footboxes {
        render_participant_box(&mut out, p, scene);
    }

    out.push_str("</svg>");
    out
}

pub fn render_family_stub_svg(document: &FamilyDocument) -> String {
    let width = 760;
    let mut y = 28;
    let title_lines = document
        .title
        .as_deref()
        .map(|v| v.lines().count() as i32)
        .unwrap_or(0);
    let body_rows = document.nodes.len().max(1) as i32;
    let member_rows = document
        .nodes
        .iter()
        .map(|n| n.members.len() as i32)
        .sum::<i32>();
    let relation_rows = document.relations.len() as i32;
    let height =
        140 + (body_rows * 42) + (member_rows * 16) + (relation_rows * 20) + (title_lines * 24);

    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        width, height, width, height
    ));
    out.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");

    if let Some(title) = &document.title {
        for line in title.lines() {
            out.push_str(&format!(
                "<text x=\"24\" y=\"{}\" font-family=\"monospace\" font-size=\"18\" font-weight=\"600\">{}</text>",
                y,
                escape_text(line)
            ));
            y += 24;
        }
    }

    out.push_str(&format!(
        "<text x=\"24\" y=\"{}\" font-family=\"monospace\" font-size=\"13\" fill=\"#333\">Bootstrap stub for {} diagrams</text>",
        y,
        family_kind_label(document.kind)
    ));
    y += 16;

    out.push_str(&format!(
        "<rect x=\"24\" y=\"{}\" width=\"712\" height=\"{}\" rx=\"6\" ry=\"6\" fill=\"#f8fafc\" stroke=\"#94a3b8\" stroke-width=\"1\"/>",
        y,
        32 + (body_rows * 42) + (member_rows * 16)
    ));
    y += 24;

    if document.nodes.is_empty() {
        out.push_str(&format!(
            "<text x=\"40\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" fill=\"#475569\">No declarations parsed.</text>",
            y
        ));
        y += 30;
    } else {
        for node in &document.nodes {
            out.push_str(&format!(
                "<rect x=\"40\" y=\"{}\" width=\"680\" height=\"30\" rx=\"4\" ry=\"4\" fill=\"white\" stroke=\"#cbd5e1\" stroke-width=\"1\"/>",
                y - 14
            ));
            let alias = node
                .alias
                .as_deref()
                .map(|v| format!(" as {v}"))
                .unwrap_or_default();
            out.push_str(&format!(
                "<text x=\"52\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" fill=\"#0f172a\">{} {}{}</text>",
                y + 6,
                family_node_label(node.kind),
                escape_text(&node.name),
                escape_text(&alias)
            ));
            y += 22;
            for member in &node.members {
                out.push_str(&format!(
                    "<text x=\"66\" y=\"{}\" font-family=\"monospace\" font-size=\"11\" fill=\"#334155\">{}</text>",
                    y + 6,
                    escape_text(member)
                ));
                y += 16;
            }
            y += 20;
        }
    }

    if !document.relations.is_empty() {
        out.push_str(&format!(
            "<text x=\"24\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" font-weight=\"600\" fill=\"#334155\">Relations</text>",
            y + 6
        ));
        y += 24;
        for relation in &document.relations {
            let label = relation
                .label
                .as_deref()
                .map(|v| format!(" : {v}"))
                .unwrap_or_default();
            out.push_str(&format!(
                "<text x=\"40\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" fill=\"#1e293b\">{} {} {}{}</text>",
                y,
                escape_text(&relation.from),
                escape_text(&relation.arrow),
                escape_text(&relation.to),
                escape_text(&label)
            ));
            y += 20;
        }
    }

    out.push_str("</svg>");
    out
}

pub fn render_family_tree_svg(document: &FamilyDocument) -> String {
    const MARGIN: i32 = 24;
    const CHAR_WIDTH: i32 = 7;
    const TOPLINE_FONT_SIZE: i32 = 13;
    const NODE_FONT_SIZE: i32 = 12;
    const NODE_MIN_WIDTH: i32 = 220;
    const NODE_MAX_WIDTH: i32 = 360;
    const NODE_PADDING_X: i32 = 12;
    const NODE_PADDING_Y: i32 = 12;
    const MIN_SPACING_X: i32 = 80;
    const MIN_SPACING_Y: i32 = 48;
    const MAX_LINE_CHARS: usize = 24;

    let mut out = String::new();
    let title_lines = document
        .title
        .as_deref()
        .map(|v| v.lines().collect::<Vec<_>>())
        .unwrap_or_default();

    let mut layouts = Vec::with_capacity(document.nodes.len());
    for node in &document.nodes {
        let raw_label = node
            .alias
            .as_ref()
            .map_or_else(|| node.name.clone(), |alias| format!("{} as {}", node.name, alias));
        let lines = wrap_text(raw_label, MAX_LINE_CHARS, document.text_overflow_policy);
        let width_chars = lines
            .iter()
            .map(|line| line.chars().count() as i32)
            .max()
            .unwrap_or(1);
        let width = (width_chars * CHAR_WIDTH + (NODE_PADDING_X * 2)).clamp(NODE_MIN_WIDTH, NODE_MAX_WIDTH);
        let height = (lines.len() as i32 * 18) + (NODE_PADDING_Y * 2);
        layouts.push(NodeLayout {
            label_lines: lines,
            width,
            height,
            x: 0,
            y: 0,
        });
    }

    let mut levels = Vec::<Vec<usize>>::new();
    let mut max_depth = 0usize;
    for (idx, node) in document.nodes.iter().enumerate() {
        let depth = node.depth;
        if depth > max_depth {
            max_depth = depth;
        }
        if levels.len() <= depth {
            levels.resize_with(depth + 1, Vec::new);
        }
        levels[depth].push(idx);
    }

    let mut depth_slot = vec![0usize; document.nodes.len()];
    for level_nodes in &levels {
        for (slot, idx) in level_nodes.iter().copied().enumerate() {
            depth_slot[idx] = slot;
        }
    }

    let max_node_width = layouts
        .iter()
        .map(|layout| layout.width)
        .max()
        .unwrap_or(NODE_MIN_WIDTH);
    let max_node_height = layouts
        .iter()
        .map(|layout| layout.height)
        .max()
        .unwrap_or(58);

    let x_step = max_node_width + MIN_SPACING_X;
    let y_step = max_node_height + MIN_SPACING_Y;

    let mut y_offsets = vec![0i32; levels.len()];
    for i in 1..levels.len() {
        let prev = y_offsets[i - 1] + y_step;
        y_offsets[i] = prev;
    }

    let vertical = matches!(
        document.orientation,
        FamilyOrientation::TopToBottom | FamilyOrientation::BottomToTop
    );

    let mut height_offset = MARGIN;
    if !title_lines.is_empty() {
        height_offset += (title_lines.len() as i32) * 24;
        height_offset += 12;
    }

    for (depth, level_nodes) in levels.iter().enumerate() {
        for &node_idx in level_nodes {
            let slot = depth_slot[node_idx] as i32;
            let display_depth = match document.orientation {
                FamilyOrientation::TopToBottom => depth,
                FamilyOrientation::BottomToTop => max_depth.saturating_sub(depth),
                FamilyOrientation::LeftToRight => depth,
                FamilyOrientation::RightToLeft => max_depth.saturating_sub(depth),
            };

            if vertical {
                layouts[node_idx].x = MARGIN + (slot * x_step);
                layouts[node_idx].y = height_offset + (display_depth as i32 * y_step);
            } else {
                layouts[node_idx].x = MARGIN + (display_depth as i32 * x_step);
                layouts[node_idx].y = MARGIN + (slot * y_step);
            }
        }
    }

    let mut max_x = MARGIN;
    let mut max_y = height_offset;
    for layout in &layouts {
        max_x = max_x.max(layout.x + layout.width);
        max_y = max_y.max(layout.y + layout.height);
    }
    if !title_lines.is_empty() {
        max_y = max_y.max(height_offset);
    }

    let width = (max_x + MARGIN).max(760);
    let height = (max_y + MARGIN).max(180);

    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        width, height, width, height
    ));
    out.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");

    let mut y_cursor = MARGIN;
    if !title_lines.is_empty() {
        for line in &title_lines {
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"18\" font-weight=\"600\">{}</text>",
                MARGIN,
                y_cursor,
                escape_text(line)
            ));
            y_cursor += 24;
        }
        y_cursor += 12;
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" fill=\"#334155\">orientation: {}</text>",
            MARGIN,
            y_cursor,
            document.orientation.as_str()
        ));
        y_cursor += 20;
    } else {
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" fill=\"#334155\">orientation: {}</text>",
            MARGIN,
            y_cursor,
            document.orientation.as_str()
        ));
        y_cursor += 20;
    }

    for (idx, layout) in layouts.iter().enumerate() {
        out.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"6\" ry=\"6\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
            layout.x,
            layout.y,
            layout.width,
            layout.height,
            document.style.participant_background_color,
            document.style.participant_border_color
        ));

        for (line_idx, line) in layout.label_lines.iter().enumerate() {
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"{}\" fill=\"#0f172a\">{}</text>",
                layout.x + NODE_PADDING_X,
                layout.y + NODE_PADDING_Y + (line_idx as i32 * 18),
                NODE_FONT_SIZE,
                escape_text(line)
            ));
        }
    }

    for relation in &document.relations {
        let from_idx = document
            .nodes
            .iter()
            .position(|node| node.name == relation.from)
            .or_else(|| {
                document
                    .nodes
                    .iter()
                    .position(|node| node.alias.as_deref() == Some(relation.from.as_str()))
            });
        let to_idx = document
            .nodes
            .iter()
            .position(|node| node.name == relation.to)
            .or_else(|| {
                document
                    .nodes
                    .iter()
                    .position(|node| node.alias.as_deref() == Some(relation.to.as_str()))
            });

        if let (Some(from), Some(to)) = (from_idx, to_idx) {
            let from_layout = &layouts[from];
            let to_layout = &layouts[to];
            let (x1, y1, x2, y2) = match document.orientation {
                FamilyOrientation::TopToBottom => (
                    from_layout.x + from_layout.width / 2,
                    from_layout.y + from_layout.height,
                    to_layout.x + to_layout.width / 2,
                    to_layout.y,
                ),
                FamilyOrientation::BottomToTop => (
                    from_layout.x + from_layout.width / 2,
                    from_layout.y,
                    to_layout.x + to_layout.width / 2,
                    to_layout.y + to_layout.height,
                ),
                FamilyOrientation::LeftToRight => (
                    from_layout.x + from_layout.width,
                    from_layout.y + from_layout.height / 2,
                    to_layout.x,
                    to_layout.y + to_layout.height / 2,
                ),
                FamilyOrientation::RightToLeft => (
                    from_layout.x,
                    from_layout.y + from_layout.height / 2,
                    to_layout.x + to_layout.width,
                    to_layout.y + to_layout.height / 2,
                ),
            };

            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                x1, y1, x2, y2, document.style.arrow_color
            ));
            render_tree_arrow(&mut out, x1, y1, x2, y2, &document.style.arrow_color);

            if let Some(label) = &relation.label {
                let label_lines = wrap_text(label.clone(), 18, document.text_overflow_policy);
                let label_x = ((x1 + x2) / 2).max(4);
                let label_y = ((y1 + y2) / 2).min(height - 8);
                for (line_idx, line) in label_lines.iter().enumerate() {
                    out.push_str(&format!(
                        "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\">{}</text>",
                        label_x,
                        label_y + (line_idx as i32 * 12),
                        escape_text(line)
                    ));
                }
            }
        }
    }

    let relation_count = if document.relations.is_empty() {
        "relationships: 0".to_string()
    } else {
        format!("relationships: {}", document.relations.len())
    };
    out.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\" fill=\"#475569\">{}</text>",
        MARGIN,
        height - 12,
        relation_count
    ));

    out.push_str("</svg>");
    out
}

fn family_kind_label(kind: DiagramKind) -> &'static str {
    match kind {
        DiagramKind::Class => "class",
        DiagramKind::Object => "object",
        DiagramKind::UseCase => "usecase",
        DiagramKind::Gantt => "gantt",
        DiagramKind::Chronology => "chronology",
        DiagramKind::Salt => "salt",
        DiagramKind::Component => "component",
        DiagramKind::Deployment => "deployment",
        DiagramKind::State => "state",
        DiagramKind::Activity => "activity",
        DiagramKind::Timing => "timing",
        DiagramKind::MindMap => "mindmap",
        DiagramKind::Wbs => "wbs",
        DiagramKind::Sequence => "sequence",
        DiagramKind::Unknown => "unknown",
    }
}

fn family_node_label(kind: FamilyNodeKind) -> &'static str {
    match kind {
        FamilyNodeKind::Class => "class",
        FamilyNodeKind::Object => "object",
        FamilyNodeKind::UseCase => "usecase",
        FamilyNodeKind::Salt => "widget",
        FamilyNodeKind::MindMap => "mindmap",
        FamilyNodeKind::Wbs => "wbs",
    }
}

pub fn render_timeline_stub_svg(document: &TimelineDocument) -> String {
    let width = 760;
    let event_rows =
        (document.tasks.len() + document.milestones.len() + document.constraints.len()) as i32;
    let chronology_rows = document.chronology_events.len() as i32;
    let height = 180 + (event_rows * 20) + (chronology_rows * 20);
    let mut y = 32;
    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        width, height, width, height
    ));
    out.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    out.push_str(&format!(
        "<text x=\"24\" y=\"{}\" font-family=\"monospace\" font-size=\"18\" font-weight=\"600\">Baseline {} model</text>",
        y,
        match document.kind {
            DiagramKind::Gantt => "gantt",
            DiagramKind::Chronology => "chronology",
            _ => "timeline",
        }
    ));
    y += 26;
    out.push_str(&format!(
        "<text x=\"24\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" fill=\"#334155\">Render parity for this family is out-of-scope in this slice.</text>",
        y
    ));
    y += 28;
    for task in &document.tasks {
        out.push_str(&format!(
            "<text x=\"24\" y=\"{}\" font-family=\"monospace\" font-size=\"12\">task: {}</text>",
            y,
            escape_text(&task.name)
        ));
        y += 20;
    }
    for milestone in &document.milestones {
        out.push_str(&format!(
            "<text x=\"24\" y=\"{}\" font-family=\"monospace\" font-size=\"12\">milestone: {}</text>",
            y,
            escape_text(&milestone.name)
        ));
        y += 20;
    }
    for constraint in &document.constraints {
        out.push_str(&format!(
            "<text x=\"24\" y=\"{}\" font-family=\"monospace\" font-size=\"12\">constraint: {} {} {}</text>",
            y,
            escape_text(&constraint.subject),
            escape_text(&constraint.kind),
            escape_text(&constraint.target)
        ));
        y += 20;
    }
    for evt in &document.chronology_events {
        out.push_str(&format!(
            "<text x=\"24\" y=\"{}\" font-family=\"monospace\" font-size=\"12\">event: {} happens on {}</text>",
            y,
            escape_text(&evt.subject),
            escape_text(&evt.when)
        ));
        y += 20;
    }
    out.push_str("</svg>");
    out
}

#[derive(Debug, Clone)]
struct NodeLayout {
    label_lines: Vec<String>,
    width: i32,
    height: i32,
    x: i32,
    y: i32,
}

fn wrap_text(text: String, max_chars: usize, policy: crate::scene::TextOverflowPolicy) -> Vec<String> {
    match policy {
        crate::scene::TextOverflowPolicy::EllipsisSingleLine => {
            let one_line = text.replace('\n', " ");
            vec![ellipsize(one_line, max_chars)]
        }
        crate::scene::TextOverflowPolicy::WrapAndGrow => text
            .lines()
            .flat_map(|line| wrap_line(line, max_chars))
            .collect::<Vec<_>>(),
    }
}

fn render_tree_arrow(out: &mut String, x1: i32, y1: i32, x2: i32, y2: i32, color: &str) {
    let size = 6;
    if x2 >= x1 && y1 == y2 {
        out.push_str(&format!(
            "<polygon points=\"{},{} {},{} {},{}\" fill=\"{}\"/>",
            x2,
            y2,
            x2 - size,
            y2 - size,
            x2 - size,
            y2 + size,
            color
        ));
        return;
    }

    if x1 == x2 && y2 >= y1 {
        out.push_str(&format!(
            "<polygon points=\"{},{} {},{} {},{}\" fill=\"{}\"/>",
            x2,
            y2,
            x2 - size,
            y2 - size,
            x2 + size,
            y2 - size,
            color
        ));
        return;
    }

    if x2 >= x1 {
        out.push_str(&format!(
            "<polygon points=\"{},{} {},{} {},{}\" fill=\"{}\"/>",
            x2,
            y2,
            x2 - size,
            y2 - size,
            x2 - size,
            y2 + size,
            color
        ));
        return;
    }

    if x1 > x2 {
        out.push_str(&format!(
            "<polygon points=\"{},{} {},{} {},{}\" fill=\"{}\"/>",
            x2,
            y2,
            x2 + size,
            y2 - size,
            x2 + size,
            y2 + size,
            color
        ));
    }
}

fn wrap_line(line: &str, max_chars: usize) -> Vec<String> {
    if line.is_empty() {
        return vec![String::new()];
    }
    let words = line.split_whitespace().collect::<Vec<_>>();
    if words.is_empty() {
        return vec![String::new()];
    }

    let mut lines = Vec::new();
    let mut current = String::new();
    for word in words {
        let word_len = word.chars().count();
        if current.is_empty() {
            if word_len <= max_chars {
                current.push_str(word);
            } else {
                for chunk in chunk_text(word, max_chars) {
                    lines.push(chunk);
                }
            }
            continue;
        }

        let next_len = current.chars().count() + 1 + word_len;
        if next_len <= max_chars {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(current);
            if word_len <= max_chars {
                current = word.to_string();
            } else {
                let mut chunks = chunk_text(word, max_chars);
                let tail = chunks.pop().unwrap_or_default();
                lines.extend(chunks);
                current = tail;
            }
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

fn chunk_text(text: &str, max_chars: usize) -> Vec<String> {
    if max_chars == 0 {
        return vec![text.to_string()];
    }
    let mut out = Vec::new();
    let mut current = String::new();
    for ch in text.chars() {
        current.push(ch);
        if current.chars().count() >= max_chars {
            out.push(current);
            current = String::new();
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
    if out.is_empty() {
        vec![String::new()]
    } else {
        out
    }
}

fn ellipsize(text: String, max_chars: usize) -> String {
    if max_chars == 0 {
        return "...".to_string();
    }

    let mut chars = text.chars();
    let count = text.chars().count();
    if count <= max_chars {
        return text;
    }

    if max_chars <= 3 {
        return "...".to_string();
    }

    text.chars().take(max_chars - 3).collect::<String>() + "..."
}

fn render_virtual_endpoint_marker(out: &mut String, x: i32, y: i32, kind: VirtualEndpointKind) {
    match kind {
        VirtualEndpointKind::Plain => {
            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#111\" stroke-width=\"1.5\"/>",
                x, y - 6, x, y + 6
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

fn escape_text(input: &str) -> String {
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

fn render_participant_box(out: &mut String, participant: &ParticipantBox, scene: &Scene) {
    let x = participant.x;
    let y = participant.y;
    let width = participant.width;
    let height = participant.height;
    let display_lines = &participant.display_lines;
    let cx = x + (width / 2);

    match participant.role {
        ParticipantRole::Participant => {
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"4\" ry=\"4\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                x,
                y,
                width,
                height,
                scene.style.participant_background_color,
                scene.style.participant_border_color
            ));
        }
        ParticipantRole::Actor => {
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"8\" ry=\"8\" fill=\"#fff3e0\" stroke=\"#8a5a00\" stroke-width=\"1\"/>",
                x, y, width, height
            ));
            out.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"4\" fill=\"none\" stroke=\"#8a5a00\" stroke-width=\"1\"/>",
                x + 12,
                y + 10
            ));
            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#8a5a00\" stroke-width=\"1\"/>",
                x + 12,
                y + 14,
                x + 12,
                y + 22
            ));
            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#8a5a00\" stroke-width=\"1\"/>",
                x + 8,
                y + 18,
                x + 16,
                y + 18
            ));
            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#8a5a00\" stroke-width=\"1\"/>",
                x + 12,
                y + 22,
                x + 8,
                y + 28
            ));
            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#8a5a00\" stroke-width=\"1\"/>",
                x + 12,
                y + 22,
                x + 16,
                y + 28
            ));
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
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"4\" ry=\"4\" fill=\"#fff0f0\" stroke=\"#8a3030\" stroke-width=\"1\"/>",
                x, y, width, height
            ));
            for i in [8, 14, 20] {
                out.push_str(&format!(
                    "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#8a3030\" stroke-width=\"1\"/>",
                    x + 8,
                    y + i,
                    x + width - 8,
                    y + i
                ));
            }
        }
    }

    for (idx, line) in display_lines.iter().enumerate() {
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"13\">{}</text>",
            cx,
            y + 21 + (idx as i32 * 16),
            escape_text(line)
        ));
    }
}
