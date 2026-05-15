use crate::scene::Scene;

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
        out.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"4\" ry=\"4\" fill=\"#f6f6f6\" stroke=\"#111\" stroke-width=\"1\"/>",
            p.x, p.y, p.width, p.height
        ));
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"13\">{}</text>",
            p.x + p.width / 2,
            p.y + 21,
            escape_text(&p.display)
        ));
    }

    for l in &scene.lifelines {
        out.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#555\" stroke-width=\"1\" stroke-dasharray=\"6 4\"/>",
            l.x, l.y1, l.x, l.y2
        ));
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
