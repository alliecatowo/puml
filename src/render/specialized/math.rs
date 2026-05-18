use super::*;

pub fn render_math_svg(document: &MathDocument) -> String {
    match crate::specialized::render_math_from_parts(&document.body, document.title.as_deref()) {
        Ok(svg) => svg,
        Err(_) => render_math_fallback(document),
    }
}

fn render_math_fallback(document: &MathDocument) -> String {
    let width = 760;
    let lines: Vec<&str> = document.body.lines().collect();
    let line_count = lines.len().max(1) as i32;
    let height = 120 + line_count * 22;
    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\">",
        w = width,
        h = height
    ));
    out.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    let mut y = 32;
    if let Some(title) = &document.title {
        out.push_str(&format!(
            "<text x=\"24\" y=\"{y}\" font-family=\"monospace\" font-size=\"16\" font-weight=\"600\">{}</text>",
            escape_text(title)
        ));
        y += 24;
    }
    out.push_str(&format!(
        "<text x=\"24\" y=\"{y}\" font-family=\"monospace\" font-size=\"12\" fill=\"#475569\">math (LaTeX-like)</text>"
    ));
    y += 16;
    let box_y = y;
    let box_h = (line_count * 22) + 24;
    out.push_str(&format!(
        "<rect x=\"24\" y=\"{by}\" width=\"{bw}\" height=\"{bh}\" rx=\"6\" ry=\"6\" fill=\"#f8fafc\" stroke=\"#cbd5e1\" stroke-width=\"1\"/>",
        by = box_y,
        bw = width - 48,
        bh = box_h
    ));
    let mut ty = box_y + 24;
    for line in lines {
        out.push_str(&format!(
            "<text x=\"40\" y=\"{ty}\" font-family=\"monospace\" font-size=\"13\" fill=\"#0f172a\">{}</text>",
            escape_text(line)
        ));
        ty += 22;
    }
    out.push_str("</svg>");
    out
}
