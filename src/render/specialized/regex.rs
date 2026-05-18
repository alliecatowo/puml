use super::*;

pub fn render_regex_svg(document: &RegexDocument) -> String {
    // Auto-expand canvas width to prevent title / token row truncation (#514).
    let min_width = 760_i32;
    // Include the document title width in the canvas width calculation.
    let title_px = document
        .title
        .as_deref()
        .map(|t| (t.len() as i32) * 9 + 64)
        .unwrap_or(0);
    let max_row_px: i32 = document
        .patterns
        .iter()
        .map(|pat| {
            // Source regex string row width (shown as /source/)
            let source_px = (pat.source.len() as i32) * 7 + 64;
            // Token row: sum of token label widths
            let tokens_px: i32 = regex_tokens_to_labels(&pat.tokens)
                .iter()
                .map(|l| (l.len().max(1) as i32) * 8 + 18 + 8)
                .sum::<i32>()
                + 80;
            source_px.max(tokens_px)
        })
        .max()
        .unwrap_or(0);
    let width = min_width.max(title_px).max(max_row_px);
    let row_height = 80;
    let height = 80 + (document.patterns.len().max(1) as i32) * row_height;
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
        "<text x=\"24\" y=\"{y}\" font-family=\"monospace\" font-size=\"12\" fill=\"#475569\">Railroad diagram (regex)</text>"
    ));
    y += 18;
    if document.patterns.is_empty() {
        out.push_str(&format!(
            "<text x=\"24\" y=\"{y}\" font-family=\"monospace\" font-size=\"12\" fill=\"#94a3b8\">(empty)</text>"
        ));
    } else {
        for pat in &document.patterns {
            render_regex_row(&mut out, &pat.source, &pat.tokens, y, width, min_width);
            y += row_height;
        }
    }
    out.push_str("</svg>");
    out
}

fn render_regex_row(out: &mut String, source: &str, tokens: &[RegexToken], y: i32, width: i32, _min_width: i32) {
    out.push_str(&format!(
        "<text x=\"24\" y=\"{}\" font-family=\"monospace\" font-size=\"11\" fill=\"#334155\">/{}/</text>",
        y - 4,
        escape_text(source)
    ));
    let baseline = y + 26;
    out.push_str(&format!(
        "<line x1=\"24\" y1=\"{by}\" x2=\"{x2}\" y2=\"{by}\" stroke=\"#94a3b8\" stroke-width=\"1\"/>",
        by = baseline,
        x2 = width - 48
    ));
    let mut x = 40;
    out.push_str(&format!(
        "<circle cx=\"{x}\" cy=\"{by}\" r=\"5\" fill=\"#1e293b\"/>",
        x = x,
        by = baseline
    ));
    x += 18;
    let labels = regex_tokens_to_labels(tokens);
    for label in &labels {
        let box_w = (label.len().max(1) as i32) * 8 + 18;
        let box_w = box_w.min(width - x - 60);
        let (class_name, fill, stroke) = regex_label_style(label);
        out.push_str(&format!(
            "<rect class=\"regex-token {class_name}\" x=\"{x}\" y=\"{ry}\" width=\"{w}\" height=\"22\" rx=\"4\" ry=\"4\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"1\"/>",
            x = x,
            ry = baseline - 11,
            w = box_w
        ));
        out.push_str(&format!(
            "<text x=\"{tx}\" y=\"{ty}\" font-family=\"monospace\" font-size=\"11\" fill=\"#0c4a6e\">{}</text>",
            escape_text(label),
            tx = x + 6,
            ty = baseline + 4
        ));
        x += box_w + 8;
        // With auto-expanded canvas, only break at absolute canvas boundary.
        if x > width - 48 {
            break;
        }
    }
    out.push_str(&format!(
        "<circle cx=\"{x}\" cy=\"{by}\" r=\"5\" fill=\"#1e293b\"/>",
        x = (width - 36),
        by = baseline
    ));
}

fn regex_label_style(label: &str) -> (&'static str, &'static str, &'static str) {
    if label.contains("alt(") {
        ("regex-alt", "#fef3c7", "#d97706")
    } else if label.contains('{')
        || label.ends_with('?')
        || label.ends_with('*')
        || label.ends_with('+')
    {
        ("regex-repeat", "#dcfce7", "#16a34a")
    } else if label.starts_with('[') {
        ("regex-class", "#ede9fe", "#7c3aed")
    } else if label == "^" || label == "$" {
        ("regex-anchor", "#fee2e2", "#dc2626")
    } else {
        ("regex-literal", "#e0f2fe", "#0284c7")
    }
}

fn regex_tokens_to_labels(tokens: &[RegexToken]) -> Vec<String> {
    let mut out = Vec::new();
    for t in tokens {
        out.push(regex_token_label(t));
    }
    out
}

fn regex_token_label(token: &RegexToken) -> String {
    match token {
        RegexToken::Literal(s) => format!("'{}'", s),
        RegexToken::CharClass(s) => format!("[{}]", s),
        RegexToken::Group(inner) => format!("({})", regex_tokens_to_labels(inner).join(" ")),
        RegexToken::Alt(branches) => {
            let parts: Vec<String> = branches
                .iter()
                .map(|b| regex_tokens_to_labels(b).join(" "))
                .collect();
            format!("alt({})", parts.join("|"))
        }
        RegexToken::Repeat { inner, kind } => {
            let suffix = match kind {
                RepeatKind::ZeroOrOne => "?",
                RepeatKind::ZeroOrMore => "*",
                RepeatKind::OneOrMore => "+",
                RepeatKind::Exact(n) => return format!("{}{{{}}}", regex_token_label(inner), n),
                RepeatKind::Range { min, max } => {
                    return format!(
                        "{}{{{},{}}}",
                        regex_token_label(inner),
                        min.map(|n| n.to_string()).unwrap_or_default(),
                        max.map(|n| n.to_string()).unwrap_or_default()
                    );
                }
            };
            format!("{}{}", regex_token_label(inner), suffix)
        }
        RegexToken::Escape(c) => format!("\\{}", c),
        RegexToken::AnyChar => ".".to_string(),
        RegexToken::Anchor(s) => s.clone(),
        RegexToken::Unsupported(s) => format!("?{}?", s),
    }
}
