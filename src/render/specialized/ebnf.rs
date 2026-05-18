use super::*;

pub fn render_ebnf_svg(document: &EbnfDocument) -> String {
    // Auto-expand canvas width to fit the widest rule (#510): each token ~8px/char +
    // 8px gap, plus 120px for the rule name + margins.
    let min_width = 820_i32;
    let max_token_row_px: i32 = document
        .rules
        .iter()
        .map(|rule| {
            let labels = ebnf_tokens_to_labels(&rule.tokens);
            let row_px: i32 = labels
                .iter()
                .map(|l| (l.len() as i32 * 8).clamp(36, 400) + 8)
                .sum::<i32>()
                + 120;
            row_px
        })
        .max()
        .unwrap_or(0);
    let width = min_width.max(max_token_row_px + 48);
    let row_height = 90;
    // Extra bottom pad so the last-row terminal circles/ovals aren't clipped (#510).
    let bottom_pad = 32;
    let height = 80 + (document.rules.len().max(1) as i32) * row_height + bottom_pad;
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
        "<text x=\"24\" y=\"{y}\" font-family=\"monospace\" font-size=\"12\" fill=\"#475569\">EBNF railroad diagrams</text>"
    ));
    y += 18;
    if document.rules.is_empty() {
        out.push_str(&format!(
            "<text x=\"24\" y=\"{y}\" font-family=\"monospace\" font-size=\"12\" fill=\"#94a3b8\">(empty)</text>"
        ));
    } else {
        for rule in &document.rules {
            out.push_str(&format!(
                "<text x=\"24\" y=\"{ty}\" font-family=\"monospace\" font-size=\"13\" font-weight=\"600\" fill=\"#0f172a\">{} ::=</text>",
                escape_text(&rule.name),
                ty = y
            ));
            let baseline = y + 30;
            out.push_str(&format!(
                "<line x1=\"24\" y1=\"{by}\" x2=\"{x2}\" y2=\"{by}\" stroke=\"#94a3b8\" stroke-width=\"1\"/>",
                by = baseline,
                x2 = width - 24
            ));
            out.push_str(&format!(
                "<circle cx=\"40\" cy=\"{by}\" r=\"5\" fill=\"#1e293b\"/>",
                by = baseline
            ));
            let mut x = 60;
            let labels = ebnf_tokens_to_labels(&rule.tokens);
            for label in &labels {
                let box_w = ((label.len() as i32) * 8).clamp(36, width - x - 60);
                let (class_name, fill, stroke) = ebnf_label_style(label);
                out.push_str(&format!(
                    "<rect class=\"ebnf-token {class_name}\" x=\"{x}\" y=\"{ry}\" width=\"{w}\" height=\"22\" rx=\"4\" ry=\"4\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"1\"/>",
                    x = x,
                    ry = baseline - 11,
                    w = box_w
                ));
                out.push_str(&format!(
                    "<text x=\"{tx}\" y=\"{ty}\" font-family=\"monospace\" font-size=\"11\" fill=\"#1e293b\">{}</text>",
                    escape_text(label),
                    tx = x + 6,
                    ty = baseline + 4
                ));
                x += box_w + 8;
                // With auto-expanded canvas, only break when truly out of space.
                if x > width - 48 {
                    break;
                }
            }
            out.push_str(&format!(
                "<circle cx=\"{x}\" cy=\"{by}\" r=\"5\" fill=\"#1e293b\"/>",
                x = (width - 36),
                by = baseline
            ));
            y += row_height;
        }
    }
    out.push_str("</svg>");
    out
}

fn ebnf_tokens_to_labels(tokens: &[EbnfToken]) -> Vec<String> {
    tokens.iter().map(ebnf_token_label).collect()
}

fn ebnf_label_style(label: &str) -> (&'static str, &'static str, &'static str) {
    if label.starts_with('"') || label.starts_with('\'') {
        ("ebnf-terminal", "#fef3c7", "#d97706")
    } else if label.starts_with('[') {
        ("ebnf-optional", "#dcfce7", "#16a34a")
    } else if label.starts_with('{') {
        ("ebnf-repetition", "#ede9fe", "#7c3aed")
    } else if label.contains(" | ") {
        ("ebnf-alt", "#fee2e2", "#dc2626")
    } else if label.contains('{')
        || label.ends_with('?')
        || label.ends_with('*')
        || label.ends_with('+')
    {
        ("ebnf-repeat", "#e0f2fe", "#0284c7")
    } else {
        ("ebnf-nonterminal", "#e0e7ff", "#4f46e5")
    }
}

fn ebnf_token_label(token: &EbnfToken) -> String {
    match token {
        EbnfToken::Terminal(s) => format!("\"{}\"", s),
        EbnfToken::NonTerminal(s) => s.clone(),
        EbnfToken::Alt(branches) => {
            let parts: Vec<String> = branches
                .iter()
                .map(|b| ebnf_tokens_to_labels(b).join(" "))
                .collect();
            format!("({})", parts.join(" | "))
        }
        EbnfToken::Group(inner) => format!("({})", ebnf_tokens_to_labels(inner).join(" ")),
        EbnfToken::Optional(inner) => format!("[{}]", ebnf_tokens_to_labels(inner).join(" ")),
        EbnfToken::Repetition(inner) => format!("{{{}}}", ebnf_tokens_to_labels(inner).join(" ")),
        EbnfToken::Repeat { inner, kind } => {
            let suffix = match kind {
                RepeatKind::ZeroOrOne => "?",
                RepeatKind::ZeroOrMore => "*",
                RepeatKind::OneOrMore => "+",
                RepeatKind::Exact(n) => return format!("{}{{{}}}", ebnf_token_label(inner), n),
                RepeatKind::Range { min, max } => {
                    return format!(
                        "{}{{{},{}}}",
                        ebnf_token_label(inner),
                        min.map(|n| n.to_string()).unwrap_or_default(),
                        max.map(|n| n.to_string()).unwrap_or_default()
                    );
                }
            };
            format!("{}{}", ebnf_token_label(inner), suffix)
        }
        EbnfToken::Unsupported(s) => format!("?{}?", s),
    }
}
