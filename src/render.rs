use crate::model::ParticipantRole;
use crate::scene::{Scene, StructureKind};

pub fn render_svg(scene: &Scene) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        scene.width, scene.height, scene.width, scene.height
    ));
    out.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");

    if let Some(title) = &scene.title {
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"18\" font-weight=\"600\">{}</text>",
            title.x,
            title.y,
            escape_text(&title.text)
        ));
    }

    for p in &scene.participants {
        render_participant_box(&mut out, p.x, p.y, p.width, p.height, p.role, &p.display);
    }

    for l in &scene.lifelines {
        out.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#555\" stroke-width=\"1\" stroke-dasharray=\"6 4\"/>",
            l.x, l.y1, l.x, l.y2
        ));
    }

    for g in &scene.groups {
        let fill = if g.kind.eq_ignore_ascii_case("ref") {
            "#eef6ff"
        } else {
            "#fafafa"
        };
        out.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"3\" ry=\"3\" fill=\"{}\" stroke=\"#666\" stroke-width=\"1\"/>",
            g.x, g.y, g.width, g.height, fill
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
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#666\" stroke-width=\"1\" stroke-dasharray=\"5 4\"/>",
                g.x,
                sep.y,
                g.x + g.width,
                sep.y
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
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#111\" stroke-width=\"1.5\"{}/>",
            m.x1, m.y, m.x2, m.y, stroke_dash
        ));
        let arrow_size = 6;
        if m.x2 >= m.x1 {
            out.push_str(&format!(
                "<polygon points=\"{},{} {},{} {},{}\" fill=\"#111\"/>",
                m.x2,
                m.y,
                m.x2 - arrow_size,
                m.y - 4,
                m.x2 - arrow_size,
                m.y + 4
            ));
        } else {
            out.push_str(&format!(
                "<polygon points=\"{},{} {},{} {},{}\" fill=\"#111\"/>",
                m.x2,
                m.y,
                m.x2 + arrow_size,
                m.y - 4,
                m.x2 + arrow_size,
                m.y + 4
            ));
        }

        if let Some(label) = &m.label {
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
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"3\" ry=\"3\" fill=\"#fff8c4\" stroke=\"#111\" stroke-width=\"1\"/>",
            n.x, n.y, n.width, n.height
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
        render_participant_box(&mut out, p.x, p.y, p.width, p.height, p.role, &p.display);
    }

    out.push_str("</svg>");
    out
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

fn render_participant_box(
    out: &mut String,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    role: ParticipantRole,
    display: &str,
) {
    let cx = x + (width / 2);
    let text_y = y + 21;

    match role {
        ParticipantRole::Participant => {
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"4\" ry=\"4\" fill=\"#f6f6f6\" stroke=\"#111\" stroke-width=\"1\"/>",
                x, y, width, height
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
        }
        ParticipantRole::Boundary => {
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"#eef8ff\" stroke=\"#1b5e8a\" stroke-width=\"1\" stroke-dasharray=\"5 3\"/>",
                x, y, width, height
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

    out.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"13\">{}</text>",
        cx,
        text_y,
        escape_text(display)
    ));
}
