use crate::ast::DiagramKind;
use crate::model::{
    FamilyDocument, FamilyNodeKind, LegendHAlign, LegendVAlign, ParticipantRole, ScaleSpec,
    TimelineDocument, VirtualEndpointKind,
};
use crate::scene::{ParticipantBox, Scene, StructureKind};

const MESSAGE_LABEL_LINE_GAP: i32 = 16;

pub fn render_svg(scene: &Scene) -> String {
    let mut out = String::new();

    // Compute output dimensions based on scale spec.
    let (svg_width, svg_height, viewbox) = compute_svg_dimensions(scene);

    // Determine if a drop-shadow filter is needed.
    let shadow_filter = if scene.style.shadowing {
        " filter=\"url(#shadow)\""
    } else {
        ""
    };
    let _ = shadow_filter; // used below per-element

    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"{}\">",
        svg_width, svg_height, viewbox
    ));

    // Embed drop-shadow filter when shadowing is enabled.
    if scene.style.shadowing {
        out.push_str(
            "<defs><filter id=\"shadow\" x=\"-10%\" y=\"-10%\" width=\"130%\" height=\"130%\">\
             <feDropShadow dx=\"3\" dy=\"3\" stdDeviation=\"2\" flood-color=\"#00000040\"/>\
             </filter></defs>",
        );
    }

    let bg_fill = scene
        .style
        .background_color
        .as_deref()
        .unwrap_or("white");
    out.push_str(&format!("<rect width=\"100%\" height=\"100%\" fill=\"{}\"/>", escape_text(bg_fill)));

    // Determine font family and size from style.
    let font_family = scene
        .style
        .default_font_name
        .as_deref()
        .unwrap_or("monospace");
    let font_size_px = scene.style.default_font_size.unwrap_or(12);
    let _ = (font_family, font_size_px); // used inline below

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
        let grx = (scene.style.round_corner / 2).max(2);
        out.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" ry=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
            g.x,
            g.y,
            g.width,
            g.height,
            grx,
            grx,
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
        let nrx = (scene.style.round_corner / 2).max(2);
        out.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" ry=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
            n.x, n.y, n.width, n.height, nrx, nrx, scene.style.note_background_color, scene.style.note_border_color
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

    // Render legend if present.
    if let Some(legend_text) = &scene.legend_text {
        render_legend(&mut out, legend_text, scene);
    }

    out.push_str("</svg>");
    out
}

fn compute_svg_dimensions(scene: &Scene) -> (String, String, String) {
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

fn render_legend(out: &mut String, text: &str, scene: &Scene) {
    let lines: Vec<&str> = text.lines().collect();
    let line_count = lines.len() as i32;
    let box_width = 200_i32;
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
            let rx = scene.style.round_corner;
            let shadow_attr = if scene.style.shadowing { " filter=\"url(#shadow)\"" } else { "" };
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
