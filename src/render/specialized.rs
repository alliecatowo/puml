use super::*;

pub fn render_nwdiag_svg(document: &NwdiagDocument) -> String {
    let width = 760;
    let net_rows: i32 = document
        .networks
        .iter()
        .map(|n| 1 + n.nodes.len() as i32)
        .sum();
    let group_rows: i32 = document
        .groups
        .iter()
        .map(|g| 1 + g.nodes.len() as i32)
        .sum();
    let height = 80
        + (net_rows + group_rows).max(1) * 24
        + ((document.networks.len() + document.groups.len()) as i32) * 14;
    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        width, height, width, height
    ));
    out.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    let mut y = 28;
    out.push_str(&format!(
        "<text x=\"24\" y=\"{}\" font-family=\"monospace\" font-size=\"18\" font-weight=\"600\">{}</text>",
        y,
        escape_text(document.title.as_deref().unwrap_or("Network diagram"))
    ));
    y += 24;
    if document.networks.is_empty() {
        out.push_str(&format!(
            "<text x=\"24\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" fill=\"#475569\">(no networks)</text>",
            y
        ));
    } else {
        for net in &document.networks {
            // Swimlane header
            let net_fill = net.color.as_deref().unwrap_or("#e0f2fe");
            let net_style = net.style.as_deref().unwrap_or("solid");
            let net_dash = if net_style.eq_ignore_ascii_case("dashed") {
                " stroke-dasharray=\"5 3\""
            } else {
                ""
            };
            out.push_str(&format!(
                "<rect class=\"nwdiag-network\" data-nwdiag-style=\"{}\" data-nwdiag-shape=\"{}\" x=\"24\" y=\"{}\" width=\"712\" height=\"22\" fill=\"{}\" stroke=\"#0284c7\" stroke-width=\"1\"{} />",
                escape_text(net_style),
                escape_text(net.shape.as_deref().unwrap_or("swimlane")),
                y,
                escape_text(net_fill),
                net_dash
            ));
            let net_name = net.label.as_deref().unwrap_or(&net.name);
            let label = match &net.address {
                Some(a) => format!("network {} ({})", net_name, a),
                None => format!("network {}", net_name),
            };
            out.push_str(&format!(
                "<text x=\"32\" y=\"{}\" font-family=\"monospace\" font-size=\"13\" font-weight=\"600\" fill=\"#0c4a6e\">{}</text>",
                y + 16,
                escape_text(&label)
            ));
            y += 26;
            for node in &net.nodes {
                let node_fill = node.color.as_deref().unwrap_or("white");
                let shape = node.shape.as_deref().unwrap_or("box");
                let style = node.style.as_deref().unwrap_or("solid");
                let dashed = if style.eq_ignore_ascii_case("dashed") {
                    " stroke-dasharray=\"5 3\""
                } else {
                    ""
                };
                let node_width = node
                    .width
                    .and_then(|w| i32::try_from(w).ok())
                    .unwrap_or(680)
                    .clamp(120, 680);
                let radius = if shape.eq_ignore_ascii_case("roundedbox")
                    || shape.eq_ignore_ascii_case("cloud")
                {
                    10
                } else {
                    3
                };
                out.push_str(&format!(
                    "<rect class=\"nwdiag-node\" data-nwdiag-name=\"{}\" data-nwdiag-addresses=\"{}\" data-nwdiag-shape=\"{}\" data-nwdiag-style=\"{}\" x=\"56\" y=\"{}\" width=\"{}\" height=\"20\" rx=\"{}\" ry=\"{}\" fill=\"{}\" stroke=\"#0284c7\" stroke-width=\"1\"{}/>",
                    escape_text(&node.name),
                    escape_text(&node.addresses.join(", ")),
                    escape_text(shape),
                    escape_text(style),
                    y,
                    node_width,
                    radius,
                    radius,
                    escape_text(node_fill),
                    dashed
                ));
                let display = node.label.as_deref().unwrap_or(&node.name);
                let lbl = if node.addresses.is_empty() {
                    display.to_string()
                } else {
                    format!("{} [{}]", display, node.addresses.join(", "))
                };
                out.push_str(&format!(
                    "<text x=\"66\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" fill=\"#0f172a\">{}</text>",
                    y + 14,
                    escape_text(&lbl)
                ));
                y += 24;
            }
            y += 10;
        }
        for group in &document.groups {
            let fill = group.color.as_deref().unwrap_or("#fef3c7");
            let style = group.style.as_deref().unwrap_or("solid");
            let dashed = if style.eq_ignore_ascii_case("dashed") {
                " stroke-dasharray=\"5 3\""
            } else {
                ""
            };
            out.push_str(&format!(
                "<rect class=\"nwdiag-group\" data-nwdiag-style=\"{}\" data-nwdiag-shape=\"{}\" x=\"24\" y=\"{}\" width=\"712\" height=\"22\" fill=\"{}\" stroke=\"#d97706\" stroke-width=\"1\"{} />",
                escape_text(style),
                escape_text(group.shape.as_deref().unwrap_or("box")),
                y,
                escape_text(fill),
                dashed
            ));
            let group_label = group.label.as_deref().unwrap_or(&group.name);
            out.push_str(&format!(
                "<text x=\"32\" y=\"{}\" font-family=\"monospace\" font-size=\"13\" font-weight=\"600\" fill=\"#78350f\">group {}</text>",
                y + 16,
                escape_text(group_label)
            ));
            y += 26;
            for node in &group.nodes {
                out.push_str(&format!(
                    "<rect class=\"nwdiag-group-member\" x=\"56\" y=\"{}\" width=\"680\" height=\"20\" rx=\"3\" ry=\"3\" fill=\"#fff7ed\" stroke=\"#f59e0b\" stroke-width=\"1\"/>",
                    y
                ));
                out.push_str(&format!(
                    "<text x=\"66\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" fill=\"#0f172a\">{}</text>",
                    y + 14,
                    escape_text(node)
                ));
                y += 24;
            }
            y += 10;
        }
    }
    out.push_str("</svg>");
    out
}

pub fn render_archimate_svg(document: &ArchimateDocument) -> String {
    let width = 760;
    let layers = [
        "strategy",
        "business",
        "application",
        "technology",
        "motivation",
        "junction",
    ];
    let lane_height = 80;
    let height = 80 + (layers.len() as i32) * lane_height;
    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        width, height, width, height
    ));
    out.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    render_archimate_relation_marker_defs(&mut out, "#475569");
    let mut y = 28;
    out.push_str(&format!(
        "<text x=\"24\" y=\"{}\" font-family=\"monospace\" font-size=\"18\" font-weight=\"600\">{}</text>",
        y,
        escape_text(document.title.as_deref().unwrap_or("Archimate"))
    ));
    y += 16;
    let mut element_bounds: BTreeMap<String, (i32, i32, i32, i32)> = BTreeMap::new();
    let mut element_markup = String::new();
    for layer in layers.iter() {
        let layer_y = y;
        let bg = match *layer {
            "strategy" => "#fee2e2",
            "business" => "#fef3c7",
            "application" => "#dbeafe",
            "technology" => "#dcfce7",
            "motivation" => "#ede9fe",
            "junction" => "#f1f5f9",
            _ => "#f1f5f9",
        };
        out.push_str(&format!(
            "<rect x=\"24\" y=\"{}\" width=\"712\" height=\"{}\" fill=\"{}\" stroke=\"#94a3b8\" stroke-width=\"1\"/>",
            layer_y, lane_height, bg
        ));
        out.push_str(&format!(
            "<text x=\"32\" y=\"{}\" font-family=\"monospace\" font-size=\"11\" fill=\"#475569\">{}</text>",
            layer_y + 14,
            escape_text(layer)
        ));
        let mut x = 100;
        for elem in document.elements.iter().filter(|e| e.layer == *layer) {
            let fill = elem.fill.as_deref().unwrap_or("white");
            let stroke = elem.stroke.as_deref().unwrap_or("#334155");
            let elem_y = layer_y + 22;
            render_archimate_element_shape(
                &mut element_markup,
                ArchimateElementRender {
                    layer: &elem.layer,
                    kind: &elem.kind,
                    alias: elem.alias.as_deref().unwrap_or(""),
                    x,
                    y: elem_y,
                    w: 140,
                    h: 40,
                    fill,
                    stroke,
                },
            );
            element_bounds.insert(elem.name.clone(), (x, elem_y, 140, 40));
            if let Some(alias) = &elem.alias {
                element_bounds.insert(alias.clone(), (x, elem_y, 140, 40));
            }
            element_markup.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" fill=\"#0f172a\">{}</text>",
                x + 8,
                layer_y + 46,
                escape_text(&elem.name)
            ));
            x += 150;
            if x + 140 > 736 {
                break;
            }
        }
        y += lane_height;
    }
    for rel in &document.relations {
        let Some(&from) = element_bounds.get(&rel.from) else {
            continue;
        };
        let Some(&to) = element_bounds.get(&rel.to) else {
            continue;
        };
        let (x1, y1, x2, y2) = compute_edge_anchors_tuple(from, to);
        let relation_style = archimate_relation_style(rel.kind.as_str(), rel.style.as_deref());
        out.push_str(&format!(
            "<line class=\"archimate-relation-edge\" data-archimate-kind=\"{}\" data-archimate-direction=\"{}\" data-archimate-style=\"{}\" x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{}{}{} />",
            escape_text(&rel.kind),
            escape_text(rel.direction.as_deref().unwrap_or("")),
            escape_text(rel.style.as_deref().unwrap_or("")),
            x1,
            y1,
            x2,
            y2,
            escape_text(relation_style.color),
            relation_style.stroke_width,
            relation_style.dash,
            relation_style.marker_start,
            relation_style.marker_end
        ));
        if let Some(label) = rel.label.as_deref().filter(|label| !label.is_empty()) {
            out.push_str(&format!(
                "<text class=\"archimate-relation-label\" data-archimate-kind=\"{}\" x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\" fill=\"#1e293b\">{}</text>",
                escape_text(&rel.kind),
                (x1 + x2) / 2 + 6,
                (y1 + y2) / 2 - 4,
                escape_text(label)
            ));
        }
    }
    out.push_str(&element_markup);
    out.push_str("</svg>");
    out
}

struct ArchimateRelationStyle<'a> {
    color: &'a str,
    stroke_width: f64,
    dash: &'static str,
    marker_start: &'static str,
    marker_end: &'static str,
}

struct ArchimateElementRender<'a> {
    layer: &'a str,
    kind: &'a str,
    alias: &'a str,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    fill: &'a str,
    stroke: &'a str,
}

fn archimate_relation_style<'a>(
    kind: &str,
    inline_style: Option<&'a str>,
) -> ArchimateRelationStyle<'a> {
    let lower_style = inline_style.unwrap_or("").to_ascii_lowercase();
    let color = inline_style
        .filter(|style| style.starts_with('#') || style.starts_with('$'))
        .unwrap_or("#475569");
    let bold = lower_style.contains("bold");
    let dashed = lower_style.contains("dashed")
        || matches!(
            kind,
            "access" | "flow" | "influence" | "realization" | "used_by"
        );
    let marker_start = match kind {
        "aggregation" => " marker-start=\"url(#arrow-diamond-open)\"",
        "composition" => " marker-start=\"url(#arrow-diamond-filled)\"",
        "assignment" => " marker-start=\"url(#archimate-assignment)\"",
        _ => "",
    };
    let marker_end = match kind {
        "association" => "",
        "realization" | "specialization" => " marker-end=\"url(#arrow-triangle)\"",
        _ => " marker-end=\"url(#arrow-open)\"",
    };
    ArchimateRelationStyle {
        color,
        stroke_width: if bold { 2.5 } else { 1.5 },
        dash: if dashed {
            " stroke-dasharray=\"5 3\""
        } else {
            ""
        },
        marker_start,
        marker_end,
    }
}

fn render_archimate_element_shape(out: &mut String, element: ArchimateElementRender<'_>) {
    let ArchimateElementRender {
        layer,
        kind,
        alias,
        x,
        y,
        w,
        h,
        fill,
        stroke,
    } = element;
    out.push_str(&format!(
        "<g class=\"archimate-element\" data-archimate-layer=\"{}\" data-archimate-kind=\"{}\" data-archimate-alias=\"{}\">",
        escape_text(layer),
        escape_text(kind),
        escape_text(alias)
    ));
    match archimate_shape_for(layer, kind) {
        "junction" => {
            out.push_str(&format!(
                "<circle class=\"archimate-junction\" cx=\"{}\" cy=\"{}\" r=\"10\" fill=\"#334155\"/>",
                x + w / 2,
                y + h / 2
            ));
        }
        "component" => {
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"4\" ry=\"4\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                x,
                y,
                w,
                h,
                escape_text(fill),
                escape_text(stroke)
            ));
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"15\" height=\"8\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                x - 4,
                y + 10,
                escape_text(fill),
                escape_text(stroke)
            ));
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"15\" height=\"8\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                x - 4,
                y + h - 18,
                escape_text(fill),
                escape_text(stroke)
            ));
        }
        "service" | "process" => {
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"18\" ry=\"18\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                x,
                y,
                w,
                h,
                escape_text(fill),
                escape_text(stroke)
            ));
        }
        "node" => {
            out.push_str(&format!(
                "<path d=\"M{x},{front_y} H{front_right} L{right},{top} V{back_bottom} L{front_right},{bottom} H{x} Z\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                escape_text(fill),
                escape_text(stroke),
                front_y = y + 8,
                front_right = x + w - 12,
                right = x + w,
                top = y,
                back_bottom = y + h - 8,
                bottom = y + h
            ));
            out.push_str(&format!(
                "<path d=\"M{} {} V{} M{} {} L{} {}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1\"/>",
                x + w - 12,
                y + 8,
                y + h,
                x + w - 12,
                y + 8,
                x + w,
                y,
                escape_text(stroke)
            ));
        }
        "data-object" => {
            out.push_str(&format!(
                "<path d=\"M{x},{y} H{} L{} {} V{} H{x} Z\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                x + w - 18,
                x + w,
                y + 18,
                y + h,
                escape_text(fill),
                escape_text(stroke)
            ));
            out.push_str(&format!(
                "<path d=\"M{} {y} V{} H{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1\"/>",
                x + w - 18,
                y + 18,
                x + w,
                escape_text(stroke)
            ));
        }
        "motivation" => {
            out.push_str(&format!(
                "<polygon points=\"{},{} {},{} {},{} {},{} {},{} {},{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                x + 14,
                y,
                x + w - 14,
                y,
                x + w,
                y + h / 2,
                x + w - 14,
                y + h,
                x + 14,
                y + h,
                x,
                y + h / 2,
                escape_text(fill),
                escape_text(stroke)
            ));
        }
        "strategy" => {
            out.push_str(&format!(
                "<path d=\"M{x},{y} H{} L{} {} H{x} Z\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                x + w - 18,
                x + w,
                y + h,
                escape_text(fill),
                escape_text(stroke)
            ));
        }
        _ => {
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"6\" ry=\"6\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                x,
                y,
                w,
                h,
                escape_text(fill),
                escape_text(stroke)
            ));
        }
    }
    out.push_str("</g>");
}

fn archimate_shape_for(layer: &str, kind: &str) -> &'static str {
    let lower = kind.to_ascii_lowercase();
    if layer == "junction" || lower.starts_with("and") || lower.starts_with("or") {
        "junction"
    } else if lower.contains("component") {
        "component"
    } else if lower.contains("service") {
        "service"
    } else if lower.contains("process") || lower.contains("function") || lower.contains("event") {
        "process"
    } else if lower.contains("node")
        || lower.contains("device")
        || lower.contains("system-software")
    {
        "node"
    } else if lower.contains("data-object") || lower.contains("artifact") {
        "data-object"
    } else if layer == "motivation" {
        "motivation"
    } else if layer == "strategy" {
        "strategy"
    } else {
        "box"
    }
}

fn render_archimate_relation_marker_defs(out: &mut String, arrow_stroke: &str) {
    render_relation_marker_defs(out, arrow_stroke);
    out.push_str(&format!(
        "<defs><marker id=\"archimate-assignment\" viewBox=\"0 0 10 10\" refX=\"1\" refY=\"5\" markerWidth=\"8\" markerHeight=\"8\" orient=\"auto-start-reverse\"><circle cx=\"5\" cy=\"5\" r=\"3\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/></marker></defs>",
        escape_text(arrow_stroke),
        escape_text(arrow_stroke)
    ));
}

pub fn render_regex_svg(document: &RegexDocument) -> String {
    let width = 760;
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
            render_regex_row(&mut out, &pat.source, &pat.tokens, y, width);
            y += row_height;
        }
    }
    out.push_str("</svg>");
    out
}

fn render_regex_row(out: &mut String, source: &str, tokens: &[RegexToken], y: i32, width: i32) {
    out.push_str(&format!(
        "<text x=\"24\" y=\"{}\" font-family=\"monospace\" font-size=\"11\" fill=\"#334155\">/{}/</text>",
        y - 4,
        escape_text(source)
    ));
    let baseline = y + 26;
    out.push_str(&format!(
        "<line x1=\"24\" y1=\"{by}\" x2=\"{x2}\" y2=\"{by}\" stroke=\"#94a3b8\" stroke-width=\"1\"/>",
        by = baseline,
        x2 = width - 24
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
        if x > width - 80 {
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

pub fn render_ebnf_svg(document: &EbnfDocument) -> String {
    let width = 820;
    let row_height = 90;
    let height = 80 + (document.rules.len().max(1) as i32) * row_height;
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
                if x > width - 80 {
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

pub fn render_math_svg(document: &MathDocument) -> String {
    let start = document
        .title
        .as_ref()
        .map(|title| format!("@startmath \"{}\"", title.replace('"', "\\\"")))
        .unwrap_or_else(|| "@startmath".to_string());
    let source = format!("{start}\n{}\n@endmath", document.body);
    if let Some(Ok(svg)) = crate::specialized::try_render_specialized(&source) {
        return svg;
    }

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

pub fn render_ditaa_svg(document: &DitaaDocument) -> String {
    let start = document
        .title
        .as_ref()
        .map(|title| format!("@startditaa \"{}\"", title.replace('"', "\\\"")))
        .unwrap_or_else(|| "@startditaa".to_string());
    let source = format!("{start}\n{}\n@endditaa", document.body);
    if let Some(Ok(svg)) = crate::specialized::try_render_specialized(&source) {
        return svg;
    }

    let width = 820;
    let lines: Vec<&str> = document.body.lines().collect();
    let line_count = lines.len().max(1) as i32;
    let height = 120 + line_count * 18;
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
        "<text x=\"24\" y=\"{y}\" font-family=\"monospace\" font-size=\"12\" fill=\"#475569\">ditaa (ASCII art frame)</text>"
    ));
    y += 16;
    let box_y = y;
    let box_h = (line_count * 18) + 24;
    out.push_str(&format!(
        "<rect x=\"24\" y=\"{by}\" width=\"{bw}\" height=\"{bh}\" rx=\"4\" ry=\"4\" fill=\"#fdf6e3\" stroke=\"#b58900\" stroke-width=\"1\"/>",
        by = box_y,
        bw = width - 48,
        bh = box_h
    ));
    let mut ty = box_y + 20;
    for line in lines {
        out.push_str(&format!(
            "<text x=\"36\" y=\"{ty}\" font-family=\"monospace\" font-size=\"12\" fill=\"#073642\" xml:space=\"preserve\">{}</text>",
            escape_text(line)
        ));
        ty += 18;
    }
    out.push_str("</svg>");
    out
}

pub fn render_sdl_svg(document: &SdlDocument) -> String {
    let state_count = document.states.len().max(1) as i32;
    let cols = state_count.clamp(1, 2);
    let col_w = 260;
    let row_h = 96;
    let margin_x = 40;
    let header_h = if document.title.is_some() { 64 } else { 40 };
    let rows = (state_count + cols - 1) / cols;
    let width = margin_x * 2 + cols * col_w;
    let height = header_h + rows * row_h + 48;
    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\">",
        w = width,
        h = height
    ));
    out.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    out.push_str(
        "<defs><marker id=\"sdl-arrow\" markerWidth=\"10\" markerHeight=\"10\" refX=\"8\" refY=\"3\" orient=\"auto\"><path d=\"M0,0 L0,6 L9,3 z\" fill=\"#334155\"/></marker></defs>",
    );
    let mut y = 28;
    if let Some(title) = &document.title {
        out.push_str(&format!(
            "<text x=\"24\" y=\"{y}\" font-family=\"monospace\" font-size=\"16\" font-weight=\"600\" fill=\"#0f172a\">{}</text>",
            escape_text(title)
        ));
        y += 24;
    }
    out.push_str(&format!(
        "<text x=\"24\" y=\"{y}\" font-family=\"monospace\" font-size=\"12\" fill=\"#475569\">SDL diagram</text>"
    ));
    let grid_top = header_h;
    let mut positions: BTreeMap<&str, SdlNodeBox> = BTreeMap::new();
    for (idx, state) in document.states.iter().enumerate() {
        let col = (idx as i32) % cols;
        let row = (idx as i32) / cols;
        let node = sdl_node_box(
            margin_x + col * col_w + (col_w - SDL_NODE_W) / 2,
            grid_top + row * row_h + 12,
            state.kind,
        );
        positions.insert(&state.name, node);
    }

    for tr in &document.transitions {
        let Some(from) = positions.get(tr.from.as_str()) else {
            continue;
        };
        let Some(to) = positions.get(tr.to.as_str()) else {
            continue;
        };
        render_sdl_transition(&mut out, tr, *from, *to);
    }

    for state in &document.states {
        if let Some(node) = positions.get(state.name.as_str()) {
            render_sdl_node(&mut out, state, *node);
        }
    }
    out.push_str("</svg>");
    out
}

const SDL_NODE_W: i32 = 168;
const SDL_NODE_H: i32 = 48;

#[derive(Debug, Clone, Copy)]
struct SdlNodeBox {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

fn sdl_node_box(x: i32, y: i32, kind: SdlStateKind) -> SdlNodeBox {
    match kind {
        SdlStateKind::Start | SdlStateKind::Stop => SdlNodeBox {
            x: x + 44,
            y,
            w: 80,
            h: 56,
        },
        SdlStateKind::Decision => SdlNodeBox {
            x: x + 12,
            y: y - 8,
            w: 144,
            h: 72,
        },
        SdlStateKind::Input | SdlStateKind::Output | SdlStateKind::State => SdlNodeBox {
            x,
            y,
            w: SDL_NODE_W,
            h: SDL_NODE_H,
        },
    }
}

fn render_sdl_transition(
    out: &mut String,
    tr: &crate::model::SdlTransition,
    from: SdlNodeBox,
    to: SdlNodeBox,
) {
    let (x1, y1, x2, y2) = sdl_transition_endpoints(from, to);
    if from.x == to.x && from.y == to.y {
        let cx = from.x + from.w;
        let cy = from.y + from.h / 2;
        out.push_str(&format!(
            "<path class=\"sdl-transition\" data-sdl-from=\"{}\" data-sdl-to=\"{}\" d=\"M {cx} {cy} C {} {}, {} {}, {cx} {}\" fill=\"none\" stroke=\"#334155\" stroke-width=\"1.5\" marker-end=\"url(#sdl-arrow)\"/>",
            escape_text(&tr.from),
            escape_text(&tr.to),
            cx + 46,
            cy - 24,
            cx + 46,
            cy + 34,
            cy + 10,
        ));
    } else {
        out.push_str(&format!(
            "<line class=\"sdl-transition\" data-sdl-from=\"{}\" data-sdl-to=\"{}\" x1=\"{x1}\" y1=\"{y1}\" x2=\"{x2}\" y2=\"{y2}\" stroke=\"#334155\" stroke-width=\"1.5\" marker-end=\"url(#sdl-arrow)\"/>",
            escape_text(&tr.from),
            escape_text(&tr.to),
        ));
    }
    if let Some(label) = &tr.signal {
        let lx = (x1 + x2) / 2;
        let ly = (y1 + y2) / 2 - 8;
        out.push_str(&format!(
            "<text class=\"sdl-transition-label\" x=\"{lx}\" y=\"{ly}\" font-family=\"monospace\" font-size=\"11\" text-anchor=\"middle\" fill=\"#475569\">{}</text>",
            escape_text(label)
        ));
    }
}

fn sdl_transition_endpoints(from: SdlNodeBox, to: SdlNodeBox) -> (i32, i32, i32, i32) {
    let fcx = from.x + from.w / 2;
    let fcy = from.y + from.h / 2;
    let tcx = to.x + to.w / 2;
    let tcy = to.y + to.h / 2;
    let dx = tcx - fcx;
    let dy = tcy - fcy;
    if dx.abs() >= dy.abs() {
        if dx >= 0 {
            (from.x + from.w, fcy, to.x, tcy)
        } else {
            (from.x, fcy, to.x + to.w, tcy)
        }
    } else if dy >= 0 {
        (fcx, from.y + from.h, tcx, to.y)
    } else {
        (fcx, from.y, tcx, to.y + to.h)
    }
}

fn render_sdl_node(out: &mut String, state: &crate::model::SdlState, node: SdlNodeBox) {
    let kind = sdl_state_kind_label(state.kind);
    out.push_str(&format!(
        "<g class=\"sdl-node sdl-{kind}\" data-sdl-kind=\"{kind}\" data-sdl-name=\"{}\">",
        escape_text(&state.name)
    ));
    match state.kind {
        SdlStateKind::Start => {
            let cx = node.x + node.w / 2;
            let cy = node.y + 18;
            out.push_str(&format!(
                "<circle cx=\"{cx}\" cy=\"{cy}\" r=\"13\" fill=\"#111827\"/>"
            ));
            render_sdl_label(out, &state.name, cx, node.y + 50, "#111827");
        }
        SdlStateKind::Stop => {
            let cx = node.x + node.w / 2;
            let cy = node.y + 18;
            out.push_str(&format!(
                "<circle cx=\"{cx}\" cy=\"{cy}\" r=\"15\" fill=\"none\" stroke=\"#111827\" stroke-width=\"2\"/><circle cx=\"{cx}\" cy=\"{cy}\" r=\"9\" fill=\"#111827\"/>"
            ));
            render_sdl_label(out, &state.name, cx, node.y + 50, "#111827");
        }
        SdlStateKind::Decision => {
            let cx = node.x + node.w / 2;
            let cy = node.y + node.h / 2;
            out.push_str(&format!(
                "<polygon points=\"{cx},{} {},{cy} {cx},{} {},{cy}\" fill=\"#fef3c7\" stroke=\"#b45309\" stroke-width=\"1.5\"/>",
                node.y,
                node.x + node.w,
                node.y + node.h,
                node.x,
            ));
            render_sdl_label(out, &state.name, cx, cy + 4, "#78350f");
        }
        SdlStateKind::Input => {
            let slant = 16;
            out.push_str(&format!(
                "<polygon points=\"{},{} {},{} {},{} {},{}\" fill=\"#e0f2fe\" stroke=\"#0284c7\" stroke-width=\"1.5\"/>",
                node.x + slant,
                node.y,
                node.x + node.w,
                node.y,
                node.x + node.w - slant,
                node.y + node.h,
                node.x,
                node.y + node.h,
            ));
            render_sdl_label(
                out,
                &state.name,
                node.x + node.w / 2,
                node.y + node.h / 2 + 4,
                "#075985",
            );
        }
        SdlStateKind::Output => {
            let slant = 16;
            out.push_str(&format!(
                "<polygon points=\"{},{} {},{} {},{} {},{}\" fill=\"#dcfce7\" stroke=\"#16a34a\" stroke-width=\"1.5\"/>",
                node.x,
                node.y,
                node.x + node.w - slant,
                node.y,
                node.x + node.w,
                node.y + node.h,
                node.x + slant,
                node.y + node.h,
            ));
            render_sdl_label(
                out,
                &state.name,
                node.x + node.w / 2,
                node.y + node.h / 2 + 4,
                "#166534",
            );
        }
        SdlStateKind::State => {
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"8\" ry=\"8\" fill=\"#e0e7ff\" stroke=\"#4f46e5\" stroke-width=\"1.5\"/>",
                node.x, node.y, node.w, node.h
            ));
            render_sdl_label(
                out,
                &state.name,
                node.x + node.w / 2,
                node.y + node.h / 2 + 4,
                "#312e81",
            );
        }
    }
    out.push_str("</g>");
}

fn render_sdl_label(out: &mut String, text: &str, x: i32, y: i32, fill: &str) {
    out.push_str(&format!(
        "<text x=\"{x}\" y=\"{y}\" font-family=\"monospace\" font-size=\"12\" font-weight=\"600\" text-anchor=\"middle\" fill=\"{fill}\">{}</text>",
        escape_text(text)
    ));
}

fn sdl_state_kind_label(kind: SdlStateKind) -> &'static str {
    match kind {
        SdlStateKind::Start => "start",
        SdlStateKind::Input => "input",
        SdlStateKind::Output => "output",
        SdlStateKind::Decision => "decision",
        SdlStateKind::Stop => "stop",
        SdlStateKind::State => "state",
    }
}

pub fn render_chart_svg(document: &ChartDocument) -> String {
    let width = 780;
    let height = 420;
    let style = &document.style;
    let series = effective_chart_series(document);
    let categories = effective_chart_categories(document, &series);
    let type_name = chart_subtype_name(document.subtype);
    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\" data-chart-type=\"{type_name}\" data-chart-horizontal=\"{}\" data-chart-stacked=\"{}\">",
        document.horizontal,
        document.stacked,
        w = width,
        h = height
    ));
    out.push_str(&format!(
        "<rect width=\"100%\" height=\"100%\" fill=\"{}\"/>",
        escape_text(&style.background_color)
    ));
    out.push_str(&format!(
        "<metadata data-chart-style=\"{} {} {} {} {} {} {} {}\"/>",
        escape_text(&style.background_color),
        escape_text(&style.axis_color),
        escape_text(&style.grid_color),
        escape_text(&style.series_color),
        escape_text(&style.bar_color),
        escape_text(&style.line_color),
        escape_text(&style.pie_border_color),
        escape_text(&style.font_color)
    ));
    let mut y = 28;
    if let Some(title) = &document.title {
        out.push_str(&format!(
            "<text x=\"24\" y=\"{y}\" font-family=\"monospace\" font-size=\"16\" font-weight=\"600\">{}</text>",
            escape_text(title)
        ));
        y += 22;
    }
    out.push_str(&format!(
        "<text x=\"24\" y=\"{y}\" font-family=\"monospace\" font-size=\"12\" fill=\"#475569\">{}</text>",
        type_name
    ));
    if !document.palette.is_empty() {
        out.push_str(&format!(
            "<metadata data-chart-palette=\"{}\"/>",
            escape_text(&document.palette.join(" "))
        ));
    }
    if !series.is_empty() {
        let names = series
            .iter()
            .map(|item| item.name.as_str())
            .collect::<Vec<_>>()
            .join("|");
        out.push_str(&format!(
            "<metadata data-chart-series=\"{}\"/>",
            escape_text(&names)
        ));
    }
    out.push_str(&format!(
        "<metadata data-chart-label-mode=\"{}\"/>",
        chart_label_mode_name(document.label_mode)
    ));
    let legend_visible = chart_legend_visible(document, &series);
    let legend_left = legend_visible && document.legend.h_align == crate::model::LegendHAlign::Left;
    let legend_right =
        legend_visible && document.legend.h_align == crate::model::LegendHAlign::Right;
    let legend_bottom =
        legend_visible && document.legend.v_align == crate::model::LegendVAlign::Bottom;
    let plot_top =
        y + if legend_visible && document.legend.v_align == crate::model::LegendVAlign::Top {
            54
        } else {
            16
        };
    let plot_bottom = height - if legend_bottom { 122 } else { 74 };
    let plot_left = if legend_left { 218 } else { 78 };
    let plot_right = width - if legend_right { 178 } else { 40 };
    let plot = ChartPlotArea {
        left: plot_left,
        top: plot_top,
        right: plot_right,
        bottom: plot_bottom,
    };
    match document.subtype {
        ChartSubtype::Bar if document.horizontal => {
            render_chart_horizontal_bars(&mut out, document, &series, &categories, plot, style)
        }
        ChartSubtype::Bar => {
            render_chart_bars(&mut out, document, &series, &categories, plot, style)
        }
        ChartSubtype::Line => {
            render_chart_line(&mut out, document, &series, &categories, plot, style)
        }
        ChartSubtype::Pie => {
            let points = effective_chart_points(document, &series, &categories);
            render_chart_pie(
                document,
                &mut out,
                &points,
                width / 2,
                (plot_top + plot_bottom) / 2,
                style,
            )
        }
    }
    render_chart_annotations(&mut out, document, plot);
    render_chart_caption(&mut out, document, width, height);
    render_chart_legend(&mut out, document, &series, plot);
    out.push_str("</svg>");
    out
}

const CHART_PALETTE: &[&str] = &[
    "#1d4ed8", "#16a34a", "#d97706", "#7c3aed", "#0891b2", "#dc2626", "#0f172a", "#facc15",
];

#[derive(Clone, Copy)]
struct ChartPlotArea {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

fn render_chart_bars(
    out: &mut String,
    document: &ChartDocument,
    series: &[crate::model::ChartSeries],
    categories: &[String],
    plot: ChartPlotArea,
    style: &crate::theme::ChartStyle,
) {
    render_chart_axes(out, document, categories, plot, style);
    if series.is_empty() || categories.is_empty() {
        return;
    }
    let (min_value, max_value) = chart_value_range(document, series);
    let count = categories.len() as i32;
    let avail = (plot.right - plot.left).max(20);
    let band = (avail / count).max(10);
    let group_count = if document.stacked {
        1
    } else {
        series.len().max(1) as i32
    };
    let bar_w = ((band - 8) / group_count).max(4);
    for (cat_idx, category) in categories.iter().enumerate() {
        let band_x = plot.left + (cat_idx as i32) * band;
        let mut stack_pos = 0.0_f64;
        let mut stack_neg = 0.0_f64;
        for (series_idx, item) in series.iter().enumerate() {
            let value = item.values.get(cat_idx).copied().unwrap_or(0.0);
            let bx = band_x
                + 4
                + if document.stacked {
                    0
                } else {
                    (series_idx as i32) * bar_w
                };
            let (from, to) = if document.stacked {
                if value >= 0.0 {
                    let from = stack_pos;
                    stack_pos += value;
                    (from, stack_pos)
                } else {
                    let from = stack_neg;
                    stack_neg += value;
                    (from, stack_neg)
                }
            } else {
                (0.0, value)
            };
            let y1 = chart_y_for_value(from, min_value, max_value, plot);
            let y2 = chart_y_for_value(to, min_value, max_value, plot);
            let by = y1.min(y2);
            let bh = (y1 - y2).abs().max(1);
            let color = chart_series_color(document, item, series_idx, style.bar_color.as_str());
            out.push_str(&format!(
                "<rect x=\"{bx}\" y=\"{by}\" width=\"{bw}\" height=\"{bh}\" fill=\"{color}\" stroke=\"{}\" stroke-width=\"0.5\"/>",
                escape_text(&style.axis_color),
                bx = bx,
                by = by,
                bw = bar_w,
                bh = bh,
                color = escape_text(&color)
            ));
            out.push_str(&format!(
                "<text class=\"chart-value-label\" data-chart-label-mode=\"{}\" x=\"{tx}\" y=\"{ty}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"10\" fill=\"#334155\">{}</text>",
                chart_label_mode_name(document.label_mode),
                format_chart_value(value),
                tx = bx + bar_w / 2,
                ty = if value >= 0.0 { by - 4 } else { by + bh + 12 }
            ));
        }
        out.push_str(&format!(
            "<text x=\"{tx}\" y=\"{ty}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"{}\">{}</text>",
            escape_text(&style.font_color),
            escape_text(category),
            tx = band_x + band / 2,
            ty = plot.bottom + 16
        ));
    }
}

fn render_chart_line(
    out: &mut String,
    document: &ChartDocument,
    series: &[crate::model::ChartSeries],
    categories: &[String],
    plot: ChartPlotArea,
    style: &crate::theme::ChartStyle,
) {
    render_chart_axes(out, document, categories, plot, style);
    if series.is_empty() || categories.is_empty() {
        return;
    }
    let (min_value, max_value) = chart_value_range(document, series);
    let count = categories.len() as i32;
    let step = ((plot.right - plot.left) as f64) / ((count.max(2) - 1) as f64).max(1.0);
    for (series_idx, item) in series.iter().enumerate() {
        let color = chart_series_color(document, item, series_idx, style.line_color.as_str());
        let mut points = String::new();
        for (idx, category) in categories.iter().enumerate() {
            let value = item.values.get(idx).copied().unwrap_or(0.0);
            let px = plot.left + ((idx as f64) * step) as i32;
            let py = chart_y_for_value(value, min_value, max_value, plot);
            if !points.is_empty() {
                points.push(' ');
            }
            points.push_str(&format!("{px},{py}"));
            out.push_str(&format!(
                "<circle class=\"chart-point\" data-chart-value=\"{}\" cx=\"{px}\" cy=\"{py}\" r=\"3\" fill=\"{}\"/>",
                format_chart_value(value),
                escape_text(&color)
            ));
            if !matches!(document.label_mode, ChartLabelMode::None) {
                out.push_str(&format!(
                    "<text class=\"chart-value-label\" data-chart-label-mode=\"{}\" x=\"{tx}\" y=\"{ty}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"10\" fill=\"#334155\">{}</text>",
                    chart_label_mode_name(document.label_mode),
                    format_chart_value(value),
                    tx = px,
                    ty = py - 7
                ));
            }
            if series_idx == 0 {
                out.push_str(&format!(
                    "<text x=\"{tx}\" y=\"{ty}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"{}\">{}</text>",
                    escape_text(&style.font_color),
                    escape_text(category),
                    tx = px,
                    ty = plot.bottom + 16
                ));
            }
        }
        out.push_str(&format!(
            "<polyline points=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.5\"/>",
            points,
            escape_text(&color)
        ));
    }
}

fn render_chart_horizontal_bars(
    out: &mut String,
    document: &ChartDocument,
    series: &[crate::model::ChartSeries],
    categories: &[String],
    plot: ChartPlotArea,
    style: &crate::theme::ChartStyle,
) {
    render_chart_axes(out, document, categories, plot, style);
    if series.is_empty() || categories.is_empty() {
        return;
    }
    let (min_value, max_value) = chart_value_range(document, series);
    let count = categories.len() as i32;
    let avail = (plot.bottom - plot.top).max(20);
    let band = (avail / count).max(10);
    let group_count = if document.stacked {
        1
    } else {
        series.len().max(1) as i32
    };
    let bar_h = ((band - 8) / group_count).max(4);
    for (cat_idx, category) in categories.iter().enumerate() {
        let band_y = plot.top + (cat_idx as i32) * band;
        let mut stack_pos = 0.0_f64;
        let mut stack_neg = 0.0_f64;
        for (series_idx, item) in series.iter().enumerate() {
            let value = item.values.get(cat_idx).copied().unwrap_or(0.0);
            let (from, to) = if document.stacked {
                if value >= 0.0 {
                    let from = stack_pos;
                    stack_pos += value;
                    (from, stack_pos)
                } else {
                    let from = stack_neg;
                    stack_neg += value;
                    (from, stack_neg)
                }
            } else {
                (0.0, value)
            };
            let x1 = chart_x_for_value(from, min_value, max_value, plot);
            let x2 = chart_x_for_value(to, min_value, max_value, plot);
            let bx = x1.min(x2);
            let bw = (x1 - x2).abs().max(1);
            let by = band_y
                + 4
                + if document.stacked {
                    0
                } else {
                    (series_idx as i32) * bar_h
                };
            let color = chart_series_color(document, item, series_idx, style.bar_color.as_str());
            out.push_str(&format!(
                "<rect x=\"{bx}\" y=\"{by}\" width=\"{bw}\" height=\"{bh}\" fill=\"{color}\" stroke=\"{}\" stroke-width=\"0.5\"/>",
                escape_text(&style.axis_color),
                bx = bx,
                by = by,
                bw = bw,
                bh = bar_h,
                color = escape_text(&color)
            ));
        }
        out.push_str(&format!(
            "<text x=\"{tx}\" y=\"{ty}\" text-anchor=\"end\" font-family=\"monospace\" font-size=\"11\" fill=\"{}\">{}</text>",
            escape_text(&style.font_color),
            escape_text(category),
            tx = plot.left - 8,
            ty = band_y + band / 2 + 4
        ));
    }
}

fn render_chart_pie(
    document: &ChartDocument,
    out: &mut String,
    data: &[crate::model::ChartPoint],
    cx: i32,
    cy: i32,
    style: &crate::theme::ChartStyle,
) {
    let radius = 120_i32;
    let total: f64 = data.iter().map(|p| p.value.max(0.0)).sum();
    if total <= 0.0 {
        out.push_str(&format!(
            "<circle cx=\"{cx}\" cy=\"{cy}\" r=\"{r}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
            style.grid_color,
            style.pie_border_color,
            cx = cx,
            cy = cy,
            r = radius
        ));
        return;
    }
    let mut acc = 0.0_f64;
    // Deterministic angle accumulation using f64.
    for (idx, point) in data.iter().enumerate() {
        let v = point.value.max(0.0);
        let start = acc / total * std::f64::consts::TAU;
        acc += v;
        let end = acc / total * std::f64::consts::TAU;
        let x1 = cx as f64 + (radius as f64) * start.cos();
        let y1 = cy as f64 + (radius as f64) * start.sin();
        let x2 = cx as f64 + (radius as f64) * end.cos();
        let y2 = cy as f64 + (radius as f64) * end.sin();
        let large = if (end - start) > std::f64::consts::PI {
            1
        } else {
            0
        };
        let color = chart_slice_color(document, point, idx, style.series_color.as_str());
        out.push_str(&format!(
            "<path class=\"chart-pie-slice\" data-chart-slice=\"{}\" data-chart-value=\"{}\" data-chart-percent=\"{}\" d=\"M {cx} {cy} L {x1:.2} {y1:.2} A {r} {r} 0 {large} 1 {x2:.2} {y2:.2} Z\" fill=\"{}\" stroke=\"{}\" stroke-width=\"0.5\"/>",
            escape_text(&point.label),
            format_chart_value(point.value),
            format_chart_percent(v, total),
            escape_text(&color),
            escape_text(&style.pie_border_color),
            cx = cx,
            cy = cy,
            r = radius,
            x1 = x1,
            y1 = y1,
            x2 = x2,
            y2 = y2,
            large = large
        ));
        let mid = (start + end) / 2.0;
        if !matches!(document.label_mode, ChartLabelMode::None) {
            let label_radius = if matches!(document.label_mode, ChartLabelMode::Outside) {
                1.23
            } else {
                0.6
            };
            let lx = cx as f64 + ((radius as f64) * label_radius) * mid.cos();
            let ly = cy as f64 + ((radius as f64) * label_radius) * mid.sin();
            let label_text = chart_pie_label_text(document.label_mode, point, v, total);
            if matches!(document.label_mode, ChartLabelMode::Outside) {
                let c1x = cx as f64 + ((radius as f64) * 0.82) * mid.cos();
                let c1y = cy as f64 + ((radius as f64) * 0.82) * mid.sin();
                let c2x = cx as f64 + ((radius as f64) * 1.08) * mid.cos();
                let c2y = cy as f64 + ((radius as f64) * 1.08) * mid.sin();
                out.push_str(&format!(
                    "<line class=\"chart-pie-callout\" data-chart-slice-callout=\"{}\" x1=\"{c1x:.0}\" y1=\"{c1y:.0}\" x2=\"{c2x:.0}\" y2=\"{c2y:.0}\" stroke=\"{}\" stroke-width=\"0.75\"/>",
                    escape_text(&point.label),
                    escape_text(&style.axis_color),
                ));
            }
            out.push_str(&format!(
                "<text class=\"chart-pie-label\" data-chart-label-mode=\"{}\" data-chart-slice-label=\"{}\" x=\"{lx:.0}\" y=\"{ly:.0}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"{}\">{}</text>",
                chart_label_mode_name(document.label_mode),
                escape_text(&point.label),
                escape_text(&style.font_color),
                escape_text(&label_text),
                lx = lx,
                ly = ly
            ));
        }
    }
}

fn render_chart_axes(
    out: &mut String,
    document: &ChartDocument,
    categories: &[String],
    plot: ChartPlotArea,
    style: &crate::theme::ChartStyle,
) {
    let h_axis = document.h_axis.as_ref();
    let v_axis = document.v_axis.as_ref();
    let h_axis_color = chart_axis_color(h_axis, &style.axis_color);
    let v_axis_color = chart_axis_color(v_axis, &style.axis_color);
    let h_label_color = chart_axis_label_color(h_axis, &style.font_color);
    let v_label_color = chart_axis_label_color(v_axis, &style.font_color);
    let h_grid_color = chart_axis_grid_color(h_axis, &style.grid_color);
    let v_grid_color = chart_axis_grid_color(v_axis, &style.grid_color);
    out.push_str(&format!(
        "<line x1=\"{l}\" y1=\"{b}\" x2=\"{r}\" y2=\"{b}\" stroke=\"{}\" stroke-width=\"1\"/>",
        escape_text(h_axis_color),
        l = plot.left,
        r = plot.right,
        b = plot.bottom
    ));
    out.push_str(&format!(
        "<line x1=\"{l}\" y1=\"{t}\" x2=\"{l}\" y2=\"{b}\" stroke=\"{}\" stroke-width=\"1\"/>",
        escape_text(v_axis_color),
        l = plot.left,
        t = plot.top,
        b = plot.bottom
    ));
    let series = effective_chart_series(document);
    let (min_value, max_value) = chart_value_range(document, &series);
    let ticks = chart_axis_ticks(document, min_value, max_value);
    for value in ticks {
        let y = chart_y_for_value(value, min_value, max_value, plot);
        out.push_str(&format!(
            "<line class=\"chart-axis-grid chart-axis-grid-v\" x1=\"{l}\" y1=\"{y}\" x2=\"{r}\" y2=\"{y}\" stroke=\"{}\" stroke-width=\"0.5\"/>",
            escape_text(v_grid_color),
            l = plot.left,
            r = plot.right
        ));
        out.push_str(&format!(
            "<text class=\"chart-axis-tick chart-axis-tick-v\" data-chart-axis-tick=\"{}\" x=\"{x}\" y=\"{ty}\" text-anchor=\"end\" font-family=\"monospace\" font-size=\"10\" fill=\"{}\">{}</text>",
            format_chart_value(value),
            escape_text(v_label_color),
            format_chart_value(value),
            x = plot.left - 8,
            ty = y + 4
        ));
    }
    out.push_str(&format!(
        "<metadata data-chart-axis-v-range=\"{}..{}\"/>",
        format_chart_value(min_value),
        format_chart_value(max_value)
    ));
    render_chart_axis_metadata(out, "h", h_axis);
    render_chart_axis_metadata(out, "v", v_axis);
    if min_value <= 0.0 && max_value >= 0.0 {
        if document.horizontal {
            let x = chart_x_for_value(0.0, min_value, max_value, plot);
            out.push_str(&format!(
                "<line class=\"chart-zero-axis\" x1=\"{x}\" y1=\"{t}\" x2=\"{x}\" y2=\"{b}\" stroke=\"{}\" stroke-width=\"1.25\"/>",
                escape_text(v_axis_color),
                t = plot.top,
                b = plot.bottom
            ));
        } else {
            let y = chart_y_for_value(0.0, min_value, max_value, plot);
            out.push_str(&format!(
                "<line class=\"chart-zero-axis\" x1=\"{l}\" y1=\"{y}\" x2=\"{r}\" y2=\"{y}\" stroke=\"{}\" stroke-width=\"1.25\"/>",
                escape_text(v_axis_color),
                l = plot.left,
                r = plot.right
            ));
        }
    }
    if let Some(axis) = &document.h_axis {
        if let Some(label) = &axis.label {
            out.push_str(&format!(
                "<text x=\"{x}\" y=\"{y}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"12\" fill=\"{}\">{}</text>",
                escape_text(h_label_color),
                escape_text(label),
                x = (plot.left + plot.right) / 2,
                y = plot.bottom + 42
            ));
        }
    }
    if let Some(axis) = &document.v_axis {
        if let Some(label) = &axis.label {
            out.push_str(&format!(
                "<text x=\"18\" y=\"{y}\" transform=\"rotate(-90 18 {y})\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"12\" fill=\"{}\">{}</text>",
                escape_text(v_label_color),
                escape_text(label),
                y = (plot.top + plot.bottom) / 2
            ));
        }
    }
    if document.horizontal {
        return;
    }
    if categories.len() > 1 {
        let step =
            ((plot.right - plot.left) as f64) / ((categories.len() as i32 - 1).max(1) as f64);
        for idx in 0..categories.len() {
            let x = plot.left + ((idx as f64) * step) as i32;
            out.push_str(&format!(
                "<line class=\"chart-axis-grid chart-axis-grid-h\" x1=\"{x}\" y1=\"{t}\" x2=\"{x}\" y2=\"{b}\" stroke=\"{}\" stroke-width=\"0.5\"/>",
                escape_text(h_grid_color),
                t = plot.top,
                b = plot.bottom
            ));
        }
    }
}

fn render_chart_legend(
    out: &mut String,
    document: &ChartDocument,
    series: &[crate::model::ChartSeries],
    plot: ChartPlotArea,
) {
    if !chart_legend_visible(document, series) {
        return;
    }
    let pie_points;
    let legend_items: Vec<ChartLegendItem<'_>> = if document.subtype == ChartSubtype::Pie {
        let categories = effective_chart_categories(document, series);
        pie_points = effective_chart_points(document, series, &categories);
        pie_points
            .iter()
            .enumerate()
            .map(|(idx, point)| ChartLegendItem {
                name: point.label.as_str(),
                color: chart_slice_color(document, point, idx, "#1d4ed8"),
            })
            .collect()
    } else {
        series
            .iter()
            .enumerate()
            .map(|(idx, item)| ChartLegendItem {
                name: item.name.as_str(),
                color: chart_series_color(document, item, idx, "#1d4ed8"),
            })
            .collect()
    };
    if legend_items.is_empty() {
        return;
    }
    let x = match document.legend.h_align {
        crate::model::LegendHAlign::Left => 24,
        crate::model::LegendHAlign::Center => ((plot.left + plot.right) / 2) - 66,
        crate::model::LegendHAlign::Right => plot.right + 20,
    };
    let y = match document.legend.v_align {
        crate::model::LegendVAlign::Top => (plot.top - 44).max(44),
        crate::model::LegendVAlign::Bottom => plot.bottom + 46,
    };
    let width = 132;
    let height = 18 + (legend_items.len() as i32) * 18;
    let background = document
        .legend
        .background_color
        .as_deref()
        .unwrap_or("#ffffff");
    let border = document.legend.border_color.as_deref().unwrap_or("#cbd5e1");
    let text_color = document.legend.text_color.as_deref().unwrap_or("#0f172a");
    out.push_str(&format!(
        "<g class=\"chart-legend\" data-chart-legend=\"{}\" data-chart-legend-h=\"{}\" data-chart-legend-v=\"{}\"><rect x=\"{x}\" y=\"{y}\" width=\"{width}\" height=\"{height}\" rx=\"4\" fill=\"{}\" stroke=\"{}\"/>",
        chart_legend_position(document),
        chart_legend_h_name(document.legend.h_align),
        chart_legend_v_name(document.legend.v_align),
        escape_text(background),
        escape_text(border)
    ));
    for (idx, item) in legend_items.iter().enumerate() {
        let cy = y + 18 + (idx as i32) * 18;
        out.push_str(&format!(
            "<rect class=\"chart-legend-swatch\" x=\"{x1}\" y=\"{y1}\" width=\"10\" height=\"10\" fill=\"{}\"/><text class=\"chart-legend-label\" x=\"{tx}\" y=\"{ty}\" font-family=\"monospace\" font-size=\"11\" fill=\"{}\">{}</text>",
            escape_text(&item.color),
            escape_text(text_color),
            escape_text(item.name),
            x1 = x + 8,
            y1 = cy - 9,
            tx = x + 24,
            ty = cy
        ));
    }
    out.push_str("</g>");
}

struct ChartLegendItem<'a> {
    name: &'a str,
    color: String,
}

fn render_chart_annotations(out: &mut String, document: &ChartDocument, plot: ChartPlotArea) {
    if document.annotations.is_empty() {
        return;
    }
    let mut y = plot.top + 8;
    for annotation in &document.annotations {
        out.push_str(&format!(
            "<g data-chart-annotation=\"{}\"><rect x=\"{x}\" y=\"{y}\" width=\"190\" height=\"24\" rx=\"5\" ry=\"5\" fill=\"#fff7ed\" stroke=\"#f97316\" stroke-width=\"1\"/><text x=\"{tx}\" y=\"{ty}\" font-family=\"monospace\" font-size=\"10\" fill=\"#7c2d12\">{}: {}</text></g>",
            escape_text(&annotation.target),
            escape_text(&annotation.target),
            escape_text(&annotation.text),
            x = plot.right - 196,
            y = y,
            tx = plot.right - 186,
            ty = y + 16
        ));
        y += 30;
    }
}

fn render_chart_caption(out: &mut String, document: &ChartDocument, width: i32, height: i32) {
    if let Some(caption) = &document.caption {
        out.push_str(&format!(
            "<text data-chart-caption=\"true\" x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"#475569\">{}</text>",
            width / 2,
            height - 18,
            escape_text(caption)
        ));
    }
}

fn effective_chart_series(document: &ChartDocument) -> Vec<crate::model::ChartSeries> {
    if !document.series.is_empty() {
        return document.series.clone();
    }
    if document.data.is_empty() {
        return Vec::new();
    }
    vec![crate::model::ChartSeries {
        name: "Value".to_string(),
        values: document.data.iter().map(|p| p.value).collect(),
        color: None,
    }]
}

fn effective_chart_categories(
    document: &ChartDocument,
    series: &[crate::model::ChartSeries],
) -> Vec<String> {
    if let Some(axis) = &document.h_axis {
        if !axis.categories.is_empty() {
            return axis.categories.clone();
        }
    }
    if !document.data.is_empty() {
        return document.data.iter().map(|p| p.label.clone()).collect();
    }
    let count = series.iter().map(|s| s.values.len()).max().unwrap_or(0);
    (1..=count).map(|idx| idx.to_string()).collect()
}

fn effective_chart_points(
    document: &ChartDocument,
    series: &[crate::model::ChartSeries],
    categories: &[String],
) -> Vec<crate::model::ChartPoint> {
    if !document.data.is_empty() {
        return document.data.clone();
    }
    let Some(first_series) = series.first() else {
        return Vec::new();
    };
    first_series
        .values
        .iter()
        .enumerate()
        .map(|(idx, value)| crate::model::ChartPoint {
            label: categories
                .get(idx)
                .cloned()
                .unwrap_or_else(|| (idx + 1).to_string()),
            value: *value,
            color: first_series.color.clone(),
        })
        .collect()
}

fn chart_value_range(document: &ChartDocument, series: &[crate::model::ChartSeries]) -> (f64, f64) {
    let axis_min = document.v_axis.as_ref().and_then(|axis| axis.min);
    let axis_max = document.v_axis.as_ref().and_then(|axis| axis.max);
    let (computed_min, computed_max) = if document.stacked {
        let categories = series.iter().map(|s| s.values.len()).max().unwrap_or(0);
        let mut min_value = 0.0_f64;
        let mut max_value = 0.0_f64;
        for idx in 0..categories {
            let mut positive = 0.0_f64;
            let mut negative = 0.0_f64;
            for value in series
                .iter()
                .map(|s| s.values.get(idx).copied().unwrap_or(0.0))
            {
                if value >= 0.0 {
                    positive += value;
                } else {
                    negative += value;
                }
            }
            min_value = min_value.min(negative);
            max_value = max_value.max(positive);
        }
        (min_value, max_value)
    } else {
        let mut values = series
            .iter()
            .flat_map(|s| s.values.iter().copied())
            .peekable();
        if values.peek().is_none() {
            (0.0, 1.0)
        } else {
            values.fold((0.0_f64, 0.0_f64), |(min_value, max_value), value| {
                (min_value.min(value), max_value.max(value))
            })
        }
    };
    let min = axis_min.unwrap_or(computed_min.min(0.0));
    let max = axis_max
        .unwrap_or(computed_max.max(0.0).max(1.0))
        .max(min + 1.0);
    (min, max)
}

fn chart_y_for_value(value: f64, min_value: f64, max_value: f64, plot: ChartPlotArea) -> i32 {
    let value = value.clamp(min_value, max_value);
    let ratio = (value - min_value) / (max_value - min_value);
    plot.bottom - (ratio * ((plot.bottom - plot.top) as f64)) as i32
}

fn chart_x_for_value(value: f64, min_value: f64, max_value: f64, plot: ChartPlotArea) -> i32 {
    let value = value.clamp(min_value, max_value);
    let ratio = (value - min_value) / (max_value - min_value);
    plot.left + (ratio * ((plot.right - plot.left) as f64)) as i32
}

fn chart_axis_ticks(document: &ChartDocument, min_value: f64, max_value: f64) -> Vec<f64> {
    let Some(step) = document
        .v_axis
        .as_ref()
        .and_then(|axis| axis.tick_step)
        .filter(|step| *step > 0.0)
    else {
        return (0..=4)
            .map(|tick| min_value + ((max_value - min_value) * (tick as f64) / 4.0))
            .collect();
    };
    let mut ticks = Vec::new();
    let mut value = (min_value / step).ceil() * step;
    while value <= max_value + 1e-9 && ticks.len() < 64 {
        ticks.push(value);
        value += step;
    }
    if ticks
        .first()
        .is_none_or(|first| (*first - min_value).abs() > 1e-9)
    {
        ticks.insert(0, min_value);
    }
    if ticks
        .last()
        .is_none_or(|last| (*last - max_value).abs() > 1e-9)
    {
        ticks.push(max_value);
    }
    ticks
}

fn chart_axis_color<'a>(axis: Option<&'a crate::model::ChartAxis>, fallback: &'a str) -> &'a str {
    axis.and_then(|axis| axis.color.as_deref())
        .unwrap_or(fallback)
}

fn chart_axis_label_color<'a>(
    axis: Option<&'a crate::model::ChartAxis>,
    fallback: &'a str,
) -> &'a str {
    axis.and_then(|axis| axis.label_color.as_deref())
        .unwrap_or(fallback)
}

fn chart_axis_grid_color<'a>(
    axis: Option<&'a crate::model::ChartAxis>,
    fallback: &'a str,
) -> &'a str {
    axis.and_then(|axis| axis.grid_color.as_deref())
        .unwrap_or(fallback)
}

fn render_chart_axis_metadata(
    out: &mut String,
    name: &str,
    axis: Option<&crate::model::ChartAxis>,
) {
    let Some(axis) = axis else {
        return;
    };
    if let Some(color) = &axis.color {
        out.push_str(&format!(
            "<metadata data-chart-axis-{name}-color=\"{}\"/>",
            escape_text(color)
        ));
    }
    if let Some(color) = &axis.label_color {
        out.push_str(&format!(
            "<metadata data-chart-axis-{name}-text=\"{}\"/>",
            escape_text(color)
        ));
    }
    if let Some(color) = &axis.grid_color {
        out.push_str(&format!(
            "<metadata data-chart-axis-{name}-grid=\"{}\"/>",
            escape_text(color)
        ));
    }
}

fn chart_series_color(
    document: &ChartDocument,
    series: &crate::model::ChartSeries,
    idx: usize,
    first_fallback: &str,
) -> String {
    series.color.clone().unwrap_or_else(|| {
        if let Some(color) = document.palette.get(idx) {
            return color.clone();
        }
        if idx == 0 {
            first_fallback.to_string()
        } else {
            CHART_PALETTE[idx % CHART_PALETTE.len()].to_string()
        }
    })
}

fn chart_slice_color(
    document: &ChartDocument,
    point: &crate::model::ChartPoint,
    idx: usize,
    first_fallback: &str,
) -> String {
    point.color.clone().unwrap_or_else(|| {
        document.palette.get(idx).cloned().unwrap_or_else(|| {
            if idx == 0 {
                first_fallback.to_string()
            } else {
                CHART_PALETTE[idx % CHART_PALETTE.len()].to_string()
            }
        })
    })
}

fn chart_legend_visible(document: &ChartDocument, series: &[crate::model::ChartSeries]) -> bool {
    document.legend.visible
        || (!document.legend.explicit
            && (series.len() > 1
                || (document.subtype == ChartSubtype::Pie && document.data.len() > 1)))
}

fn chart_legend_position(document: &ChartDocument) -> &'static str {
    match (document.legend.v_align, document.legend.h_align) {
        (crate::model::LegendVAlign::Top, crate::model::LegendHAlign::Left) => "top-left",
        (crate::model::LegendVAlign::Top, crate::model::LegendHAlign::Center) => "top",
        (crate::model::LegendVAlign::Top, crate::model::LegendHAlign::Right) => "right",
        (crate::model::LegendVAlign::Bottom, crate::model::LegendHAlign::Left) => "bottom-left",
        (crate::model::LegendVAlign::Bottom, crate::model::LegendHAlign::Center) => "bottom",
        (crate::model::LegendVAlign::Bottom, crate::model::LegendHAlign::Right) => "bottom-right",
    }
}

fn chart_subtype_name(subtype: ChartSubtype) -> &'static str {
    match subtype {
        ChartSubtype::Bar => "bar",
        ChartSubtype::Line => "line",
        ChartSubtype::Pie => "pie",
    }
}

fn format_chart_value(v: f64) -> String {
    if (v - v.round()).abs() < 1e-9 {
        format!("{}", v as i64)
    } else {
        format!("{:.2}", v)
    }
}

fn format_chart_percent(value: f64, total: f64) -> String {
    if total <= 0.0 {
        return "0%".to_string();
    }
    let pct = value.max(0.0) / total * 100.0;
    if (pct - pct.round()).abs() < 1e-9 {
        format!("{}%", pct as i64)
    } else {
        format!("{pct:.1}%")
    }
}

fn chart_label_mode_name(mode: ChartLabelMode) -> &'static str {
    match mode {
        ChartLabelMode::Auto => "auto",
        ChartLabelMode::Inside => "inside",
        ChartLabelMode::Outside => "outside",
        ChartLabelMode::None => "none",
        ChartLabelMode::Value => "value",
        ChartLabelMode::Percent => "percent",
    }
}

fn chart_pie_label_text(
    mode: ChartLabelMode,
    point: &crate::model::ChartPoint,
    value: f64,
    total: f64,
) -> String {
    match mode {
        ChartLabelMode::Value => format!("{} {}", point.label, format_chart_value(point.value)),
        ChartLabelMode::Percent => {
            format!("{} {}", point.label, format_chart_percent(value, total))
        }
        ChartLabelMode::None => String::new(),
        ChartLabelMode::Auto | ChartLabelMode::Inside | ChartLabelMode::Outside => {
            format!("{} {}", point.label, format_chart_percent(value, total))
        }
    }
}

fn chart_legend_h_name(value: LegendHAlign) -> &'static str {
    match value {
        LegendHAlign::Left => "left",
        LegendHAlign::Center => "center",
        LegendHAlign::Right => "right",
    }
}

fn chart_legend_v_name(value: LegendVAlign) -> &'static str {
    match value {
        LegendVAlign::Top => "top",
        LegendVAlign::Bottom => "bottom",
    }
}
