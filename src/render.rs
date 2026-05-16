use crate::ast::DiagramKind;
use crate::model::{
    FamilyDocument, FamilyNode, FamilyNodeKind, ParticipantRole, TimelineDocument,
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

fn family_kind_label(kind: DiagramKind) -> &'static str {
    match kind {
        DiagramKind::Class => "class",
        DiagramKind::Object => "object",
        DiagramKind::UseCase => "usecase",
        DiagramKind::Gantt => "gantt",
        DiagramKind::Chronology => "chronology",
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
        FamilyNodeKind::Component => "component",
        FamilyNodeKind::Interface => "interface",
        FamilyNodeKind::Port => "port",
        FamilyNodeKind::Node => "node",
        FamilyNodeKind::Artifact => "artifact",
        FamilyNodeKind::Cloud => "cloud",
        FamilyNodeKind::Frame => "frame",
        FamilyNodeKind::Storage => "storage",
        FamilyNodeKind::Database => "database",
        FamilyNodeKind::Package => "package",
        FamilyNodeKind::Rectangle => "rectangle",
        FamilyNodeKind::Folder => "folder",
        FamilyNodeKind::File => "file",
        FamilyNodeKind::Card => "card",
        FamilyNodeKind::Actor => "actor",
        FamilyNodeKind::State => "state",
        FamilyNodeKind::StateInitial => "initial",
        FamilyNodeKind::StateFinal => "final",
        FamilyNodeKind::StateHistory => "history",
        FamilyNodeKind::ActivityStart => "start",
        FamilyNodeKind::ActivityStop => "stop",
        FamilyNodeKind::ActivityAction => "action",
        FamilyNodeKind::ActivityDecision => "decision",
        FamilyNodeKind::ActivityFork => "fork",
        FamilyNodeKind::ActivityForkEnd => "end fork",
        FamilyNodeKind::ActivityMerge => "merge",
        FamilyNodeKind::ActivityPartition => "partition",
        FamilyNodeKind::TimingConcise => "concise",
        FamilyNodeKind::TimingRobust => "robust",
        FamilyNodeKind::TimingClock => "clock",
        FamilyNodeKind::TimingBinary => "binary",
        FamilyNodeKind::TimingEvent => "event",
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

pub fn render_component_svg(doc: &FamilyDocument) -> String {
    render_box_grid_svg(doc, "component")
}

pub fn render_deployment_svg(doc: &FamilyDocument) -> String {
    render_box_grid_svg(doc, "deployment")
}

pub fn render_state_svg(doc: &FamilyDocument) -> String {
    render_box_grid_svg(doc, "state")
}

fn render_box_grid_svg(doc: &FamilyDocument, family: &str) -> String {
    let cols = 3i32;
    let cell_w = 200i32;
    let cell_h = 80i32;
    let margin_x = 40i32;
    let margin_y = 60i32;
    let gap = 40i32;
    let n = doc.nodes.len() as i32;
    let rows = (n + cols - 1) / cols;
    let title_lines = doc
        .title
        .as_deref()
        .map(|v| v.lines().count() as i32)
        .unwrap_or(0);
    let header_h = 40 + title_lines * 22;
    let width = (margin_x * 2) + (cols * cell_w) + ((cols - 1).max(0) * gap);
    let height = header_h + margin_y + (rows.max(1) * cell_h) + ((rows - 1).max(0) * gap) + 60;

    // Position lookup by name and alias.
    let mut positions: std::collections::BTreeMap<String, (i32, i32, i32, i32)> =
        std::collections::BTreeMap::new();

    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        width, height, width, height
    ));
    out.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");

    let mut y_cursor = 28;
    if let Some(title) = &doc.title {
        for line in title.lines() {
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"18\" font-weight=\"600\">{}</text>",
                margin_x,
                y_cursor,
                escape_text(line)
            ));
            y_cursor += 22;
        }
    }
    out.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" fill=\"#475569\">{} diagram</text>",
        margin_x,
        y_cursor + 2,
        family
    ));

    for (idx, node) in doc.nodes.iter().enumerate() {
        let col = (idx as i32) % cols;
        let row = (idx as i32) / cols;
        let x = margin_x + col * (cell_w + gap);
        let y = header_h + margin_y + row * (cell_h + gap);
        render_family_node_shape(&mut out, node, x, y, cell_w, cell_h);
        let id_name = node.name.clone();
        let id_alias = node.alias.clone();
        positions.insert(id_name, (x, y, cell_w, cell_h));
        if let Some(alias) = id_alias {
            positions.insert(alias, (x, y, cell_w, cell_h));
        }
    }

    // Draw relations as straight arrows between node center boundaries.
    for rel in &doc.relations {
        let from_box = positions.get(&rel.from);
        let to_box = positions.get(&rel.to);
        let (Some(&(fx, fy, fw, fh)), Some(&(tx, ty, tw, th))) = (from_box, to_box) else {
            continue;
        };
        let cx1 = fx + fw / 2;
        let cy1 = fy + fh / 2;
        let cx2 = tx + tw / 2;
        let cy2 = ty + th / 2;
        let (x1, y1) = clip_to_box_edge(cx1, cy1, cx2, cy2, fx, fy, fw, fh);
        let (x2, y2) = clip_to_box_edge(cx2, cy2, cx1, cy1, tx, ty, tw, th);
        let dashed = rel.arrow.contains("..") || rel.arrow.contains("--");
        let dash = if dashed && rel.arrow.contains("..") {
            " stroke-dasharray=\"4 4\""
        } else {
            ""
        };
        out.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#334155\" stroke-width=\"1.5\"{}/>",
            x1, y1, x2, y2, dash
        ));
        // arrowhead
        out.push_str(&arrowhead_svg(x1, y1, x2, y2, "#334155"));
        if let Some(label) = &rel.label {
            let mx = (x1 + x2) / 2;
            let my = (y1 + y2) / 2 - 6;
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"#1e293b\">{}</text>",
                mx,
                my,
                escape_text(label)
            ));
        }
    }

    out.push_str("</svg>");
    out
}

fn clip_to_box_edge(
    cx: i32,
    cy: i32,
    tx: i32,
    ty: i32,
    bx: i32,
    by: i32,
    bw: i32,
    bh: i32,
) -> (i32, i32) {
    let dx = (tx - cx) as f64;
    let dy = (ty - cy) as f64;
    if dx.abs() < 1e-6 && dy.abs() < 1e-6 {
        return (cx, cy);
    }
    let half_w = (bw as f64) / 2.0;
    let half_h = (bh as f64) / 2.0;
    let scale_x = if dx.abs() > 1e-6 {
        half_w / dx.abs()
    } else {
        f64::INFINITY
    };
    let scale_y = if dy.abs() > 1e-6 {
        half_h / dy.abs()
    } else {
        f64::INFINITY
    };
    let s = scale_x.min(scale_y);
    let ex = (cx as f64) + dx * s;
    let ey = (cy as f64) + dy * s;
    // Keep within box bounds
    let ex = ex.clamp(bx as f64, (bx + bw) as f64);
    let ey = ey.clamp(by as f64, (by + bh) as f64);
    (ex as i32, ey as i32)
}

fn arrowhead_svg(_x1: i32, _y1: i32, x2: i32, y2: i32, color: &str) -> String {
    // small triangle arrowhead pointing in the direction from (x1,y1) -> (x2,y2)
    let dx = (x2 - _x1) as f64;
    let dy = (y2 - _y1) as f64;
    let len = (dx * dx + dy * dy).sqrt().max(1.0);
    let ux = dx / len;
    let uy = dy / len;
    let size = 8.0;
    let bx = (x2 as f64) - ux * size;
    let by = (y2 as f64) - uy * size;
    let px = -uy;
    let py = ux;
    let lx = bx + px * (size / 2.0);
    let ly = by + py * (size / 2.0);
    let rx = bx - px * (size / 2.0);
    let ry = by - py * (size / 2.0);
    format!(
        "<polygon points=\"{},{} {},{} {},{}\" fill=\"{}\"/>",
        x2, y2, lx as i32, ly as i32, rx as i32, ry as i32, color
    )
}

fn render_family_node_shape(out: &mut String, node: &FamilyNode, x: i32, y: i32, w: i32, h: i32) {
    let cx = x + w / 2;
    let cy = y + h / 2;
    let display = node
        .label
        .clone()
        .unwrap_or_else(|| node.name.clone());
    let kind_label = family_node_label(node.kind);

    match node.kind {
        FamilyNodeKind::Interface => {
            // small circle interface
            let r = 18;
            out.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"#f1f5f9\" stroke=\"#0f172a\" stroke-width=\"1.5\"/>",
                cx, cy, r
            ));
        }
        FamilyNodeKind::Component => {
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"4\" ry=\"4\" fill=\"#fefce8\" stroke=\"#a16207\" stroke-width=\"1\"/>",
                x, y, w, h
            ));
            // component badges (two small rectangles on the left edge)
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"16\" height=\"8\" fill=\"#fefce8\" stroke=\"#a16207\" stroke-width=\"1\"/>",
                x - 4,
                y + 12
            ));
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"16\" height=\"8\" fill=\"#fefce8\" stroke=\"#a16207\" stroke-width=\"1\"/>",
                x - 4,
                y + h - 20
            ));
        }
        FamilyNodeKind::Node | FamilyNodeKind::Frame => {
            // 3D-ish prism: outer rect with offset shadow
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"#eef2ff\" stroke=\"#3730a3\" stroke-width=\"1\"/>",
                x + 6,
                y + 6,
                w,
                h
            ));
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"#ffffff\" stroke=\"#3730a3\" stroke-width=\"1.5\"/>",
                x, y, w, h
            ));
        }
        FamilyNodeKind::Cloud => {
            // cloud-ish: rounded with several arcs
            out.push_str(&format!(
                "<ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\" fill=\"#f0f9ff\" stroke=\"#0369a1\" stroke-width=\"1.5\"/>",
                cx,
                cy,
                w / 2 - 4,
                h / 2 - 4
            ));
        }
        FamilyNodeKind::Database => {
            // database cylinder
            out.push_str(&format!(
                "<ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"8\" fill=\"#ecfeff\" stroke=\"#0e7490\" stroke-width=\"1.5\"/>",
                cx,
                y + 10,
                w / 2 - 6
            ));
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"#ecfeff\" stroke=\"#0e7490\" stroke-width=\"1.5\"/>",
                x + 6,
                y + 10,
                w - 12,
                h - 20
            ));
            out.push_str(&format!(
                "<ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"8\" fill=\"#ecfeff\" stroke=\"#0e7490\" stroke-width=\"1.5\"/>",
                cx,
                y + h - 10,
                w / 2 - 6
            ));
        }
        FamilyNodeKind::Artifact | FamilyNodeKind::File => {
            // folded-corner rectangle
            out.push_str(&format!(
                "<polygon points=\"{},{} {},{} {},{} {},{} {},{}\" fill=\"#fff7ed\" stroke=\"#9a3412\" stroke-width=\"1.5\"/>",
                x,
                y,
                x + w - 18,
                y,
                x + w,
                y + 18,
                x + w,
                y + h,
                x,
                y + h
            ));
        }
        FamilyNodeKind::Folder | FamilyNodeKind::Package => {
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"60\" height=\"14\" fill=\"#fef3c7\" stroke=\"#92400e\" stroke-width=\"1\"/>",
                x, y
            ));
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"#fef3c7\" stroke=\"#92400e\" stroke-width=\"1\"/>",
                x,
                y + 14,
                w,
                h - 14
            ));
        }
        FamilyNodeKind::Storage => {
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"16\" ry=\"16\" fill=\"#fff1f2\" stroke=\"#9f1239\" stroke-width=\"1.5\"/>",
                x, y, w, h
            ));
        }
        FamilyNodeKind::Rectangle
        | FamilyNodeKind::Card
        | FamilyNodeKind::Actor
        | FamilyNodeKind::Port => {
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"6\" ry=\"6\" fill=\"#f8fafc\" stroke=\"#475569\" stroke-width=\"1\"/>",
                x, y, w, h
            ));
        }
        FamilyNodeKind::State => {
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"14\" ry=\"14\" fill=\"#ecfccb\" stroke=\"#3f6212\" stroke-width=\"1.5\"/>",
                x, y, w, h
            ));
        }
        FamilyNodeKind::StateInitial => {
            out.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"10\" fill=\"#0f172a\"/>",
                cx, cy
            ));
        }
        FamilyNodeKind::StateFinal => {
            out.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"12\" fill=\"#ffffff\" stroke=\"#0f172a\" stroke-width=\"1.5\"/>",
                cx, cy
            ));
            out.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"6\" fill=\"#0f172a\"/>",
                cx, cy
            ));
        }
        FamilyNodeKind::StateHistory => {
            out.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"14\" fill=\"#fef3c7\" stroke=\"#92400e\" stroke-width=\"1.5\"/>",
                cx, cy
            ));
        }
        FamilyNodeKind::Class | FamilyNodeKind::Object | FamilyNodeKind::UseCase => {
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"6\" ry=\"6\" fill=\"#f1f5f9\" stroke=\"#64748b\" stroke-width=\"1\"/>",
                x, y, w, h
            ));
        }
        _ => {
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"4\" ry=\"4\" fill=\"#f8fafc\" stroke=\"#64748b\" stroke-width=\"1\"/>",
                x, y, w, h
            ));
        }
    }

    // For interface/initial/final we render label below the marker.
    let (label_x, label_y) = match node.kind {
        FamilyNodeKind::Interface
        | FamilyNodeKind::StateInitial
        | FamilyNodeKind::StateFinal
        | FamilyNodeKind::StateHistory => (cx, cy + 28),
        _ => (cx, cy + 6),
    };
    out.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"13\" font-weight=\"600\">{}</text>",
        label_x,
        label_y,
        escape_text(&display)
    ));
    let kind_tag_y = match node.kind {
        FamilyNodeKind::Interface
        | FamilyNodeKind::StateInitial
        | FamilyNodeKind::StateFinal
        | FamilyNodeKind::StateHistory => label_y + 14,
        _ => y + 14,
    };
    out.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"10\" fill=\"#475569\">{}</text>",
        cx,
        kind_tag_y,
        kind_label
    ));
}

pub fn render_activity_svg(doc: &FamilyDocument) -> String {
    let width = 480;
    let step_h = 60;
    let n = doc.nodes.len() as i32;
    let title_lines = doc
        .title
        .as_deref()
        .map(|v| v.lines().count() as i32)
        .unwrap_or(0);
    let header_h = 40 + title_lines * 22;
    let height = header_h + n.max(1) * step_h + 60;

    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        width, height, width, height
    ));
    out.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");

    let mut y_cursor = 28;
    if let Some(title) = &doc.title {
        for line in title.lines() {
            out.push_str(&format!(
                "<text x=\"32\" y=\"{}\" font-family=\"monospace\" font-size=\"18\" font-weight=\"600\">{}</text>",
                y_cursor,
                escape_text(line)
            ));
            y_cursor += 22;
        }
    }
    out.push_str(&format!(
        "<text x=\"32\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" fill=\"#475569\">activity diagram</text>",
        y_cursor + 2
    ));

    let cx = width / 2;
    let box_w = 220;
    let mut last_y: Option<i32> = None;

    for (idx, node) in doc.nodes.iter().enumerate() {
        let y = header_h + (idx as i32) * step_h;
        let label = node.label.clone().unwrap_or_default();
        match node.kind {
            FamilyNodeKind::ActivityStart => {
                out.push_str(&format!(
                    "<circle cx=\"{}\" cy=\"{}\" r=\"12\" fill=\"#0f172a\"/>",
                    cx,
                    y + 20
                ));
            }
            FamilyNodeKind::ActivityStop => {
                out.push_str(&format!(
                    "<circle cx=\"{}\" cy=\"{}\" r=\"14\" fill=\"white\" stroke=\"#0f172a\" stroke-width=\"1.5\"/>",
                    cx,
                    y + 20
                ));
                out.push_str(&format!(
                    "<circle cx=\"{}\" cy=\"{}\" r=\"7\" fill=\"#0f172a\"/>",
                    cx,
                    y + 20
                ));
            }
            FamilyNodeKind::ActivityAction => {
                out.push_str(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"36\" rx=\"18\" ry=\"18\" fill=\"#ecfdf5\" stroke=\"#047857\" stroke-width=\"1.5\"/>",
                    cx - box_w / 2,
                    y + 4,
                    box_w
                ));
                out.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"12\">{}</text>",
                    cx,
                    y + 27,
                    escape_text(&label)
                ));
            }
            FamilyNodeKind::ActivityDecision => {
                // diamond
                let dx = 100;
                let dy = 22;
                out.push_str(&format!(
                    "<polygon points=\"{},{} {},{} {},{} {},{}\" fill=\"#fef9c3\" stroke=\"#a16207\" stroke-width=\"1.5\"/>",
                    cx,
                    y + 2,
                    cx + dx,
                    y + 2 + dy,
                    cx,
                    y + 2 + (dy * 2),
                    cx - dx,
                    y + 2 + dy
                ));
                out.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\">{}</text>",
                    cx,
                    y + 2 + dy + 4,
                    escape_text(&label)
                ));
            }
            FamilyNodeKind::ActivityFork | FamilyNodeKind::ActivityForkEnd => {
                out.push_str(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"8\" fill=\"#0f172a\"/>",
                    cx - box_w / 2,
                    y + 24,
                    box_w
                ));
            }
            FamilyNodeKind::ActivityMerge => {
                out.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"#475569\">{}</text>",
                    cx,
                    y + 28,
                    escape_text(&format!("(merge) {}", label))
                ));
            }
            FamilyNodeKind::ActivityPartition => {
                out.push_str(&format!(
                    "<rect x=\"24\" y=\"{}\" width=\"{}\" height=\"36\" rx=\"4\" ry=\"4\" fill=\"#f1f5f9\" stroke=\"#475569\" stroke-width=\"1\" stroke-dasharray=\"4 3\"/>",
                    y + 4,
                    width - 48
                ));
                out.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"12\" font-weight=\"600\" fill=\"#1e293b\">{}</text>",
                    cx,
                    y + 27,
                    escape_text(&format!("partition: {}", label))
                ));
            }
            _ => {
                out.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"12\">{}</text>",
                    cx,
                    y + 28,
                    escape_text(&label)
                ));
            }
        }
        // arrow from previous
        if let Some(prev_y) = last_y {
            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#0f172a\" stroke-width=\"1.5\"/>",
                cx,
                prev_y + 42,
                cx,
                y
            ));
            out.push_str(&format!(
                "<polygon points=\"{},{} {},{} {},{}\" fill=\"#0f172a\"/>",
                cx,
                y,
                cx - 4,
                y - 6,
                cx + 4,
                y - 6
            ));
        }
        last_y = Some(y);
    }

    out.push_str("</svg>");
    out
}

pub fn render_timing_svg(doc: &FamilyDocument) -> String {
    // Group nodes: signals (TimingConcise/Robust/Clock/Binary) form rows; TimingEvent are points.
    let signals: Vec<&FamilyNode> = doc
        .nodes
        .iter()
        .filter(|n| {
            matches!(
                n.kind,
                FamilyNodeKind::TimingConcise
                    | FamilyNodeKind::TimingRobust
                    | FamilyNodeKind::TimingClock
                    | FamilyNodeKind::TimingBinary
            )
        })
        .collect();
    let events: Vec<&FamilyNode> = doc
        .nodes
        .iter()
        .filter(|n| matches!(n.kind, FamilyNodeKind::TimingEvent))
        .collect();

    let mut times: Vec<&str> = events.iter().map(|e| e.name.as_str()).collect();
    times.sort();
    times.dedup();
    let n_times = times.len().max(2) as i32;
    let row_h = 56i32;
    let header_h = 60i32;
    let left_pad = 120i32;
    let right_pad = 40i32;
    let col_w = 80i32;
    let width = left_pad + right_pad + n_times * col_w;
    let height = header_h + (signals.len().max(1) as i32) * row_h + 60;

    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        width, height, width, height
    ));
    out.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");

    let mut y_cursor = 28;
    if let Some(title) = &doc.title {
        for line in title.lines() {
            out.push_str(&format!(
                "<text x=\"24\" y=\"{}\" font-family=\"monospace\" font-size=\"18\" font-weight=\"600\">{}</text>",
                y_cursor,
                escape_text(line)
            ));
            y_cursor += 22;
        }
    }
    out.push_str(&format!(
        "<text x=\"24\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" fill=\"#475569\">timing diagram</text>",
        y_cursor + 2
    ));

    // Time axis labels
    for (i, t) in times.iter().enumerate() {
        let x = left_pad + (i as i32) * col_w;
        out.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#cbd5e1\" stroke-width=\"1\" stroke-dasharray=\"3 3\"/>",
            x,
            header_h - 6,
            x,
            header_h + (signals.len().max(1) as i32) * row_h
        ));
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"#1e293b\">@{}</text>",
            x,
            header_h - 10,
            escape_text(t)
        ));
    }

    for (row_idx, signal) in signals.iter().enumerate() {
        let y = header_h + (row_idx as i32) * row_h;
        // Row label
        out.push_str(&format!(
            "<text x=\"24\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" font-weight=\"600\" fill=\"#0f172a\">{}</text>",
            y + row_h / 2,
            escape_text(&signal.name)
        ));
        out.push_str(&format!(
            "<text x=\"24\" y=\"{}\" font-family=\"monospace\" font-size=\"10\" fill=\"#475569\">{}</text>",
            y + row_h / 2 + 14,
            family_node_label(signal.kind)
        ));
        // Baseline
        out.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#94a3b8\" stroke-width=\"1\"/>",
            left_pad,
            y + row_h - 12,
            width - right_pad,
            y + row_h - 12
        ));

        // Transitions for this signal
        let mut last_x: Option<i32> = None;
        let mut last_state: Option<String> = None;
        for ev in events.iter().filter(|e| e.alias.as_deref() == Some(signal.name.as_str())) {
            let t_idx = times.iter().position(|t| *t == ev.name.as_str()).unwrap_or(0);
            let x = left_pad + (t_idx as i32) * col_w;
            let state = ev.members.first().cloned().unwrap_or_default();
            // vertical transition
            if let (Some(lx), Some(_ls)) = (last_x, last_state.as_ref()) {
                out.push_str(&format!(
                    "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#0f172a\" stroke-width=\"1.5\"/>",
                    lx,
                    y + row_h - 28,
                    x,
                    y + row_h - 28
                ));
            }
            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#0f172a\" stroke-width=\"1.5\"/>",
                x,
                y + row_h - 12,
                x,
                y + row_h - 28
            ));
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"#1e293b\">{}</text>",
                x,
                y + row_h - 32,
                escape_text(&state)
            ));
            last_x = Some(x);
            last_state = Some(state);
        }
    }

    out.push_str("</svg>");
    out
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
