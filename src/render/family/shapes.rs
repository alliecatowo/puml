use super::*;

#[derive(Debug, Clone)]
pub(super) struct NodeLayout {
    pub(super) label_lines: Vec<String>,
    pub(super) width: i32,
    pub(super) height: i32,
    pub(super) x: i32,
    pub(super) y: i32,
}

pub(super) fn wrap_text(
    text: String,
    max_chars: usize,
    policy: crate::scene::TextOverflowPolicy,
) -> Vec<String> {
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

pub(super) fn render_tree_arrow(out: &mut String, x1: i32, y1: i32, x2: i32, y2: i32, color: &str) {
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

pub(super) fn wrap_line(line: &str, max_chars: usize) -> Vec<String> {
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

pub(super) fn chunk_text(text: &str, max_chars: usize) -> Vec<String> {
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

pub(super) fn ellipsize(text: String, max_chars: usize) -> String {
    if max_chars == 0 {
        return "...".to_string();
    }

    let count = text.chars().count();
    if count <= max_chars {
        return text;
    }

    if max_chars <= 3 {
        return "...".to_string();
    }

    text.chars().take(max_chars - 3).collect::<String>() + "..."
}

pub(super) fn render_family_node_shape(
    out: &mut String,
    node: &FamilyNode,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
) {
    let cx = x + w / 2;
    let cy = y + h / 2;
    let display = node.label.clone().unwrap_or_else(|| node.name.clone());
    let kind_label = family_node_label(node.kind);
    out.push_str(&format!(
        "<desc data-uml-id=\"{}\">{}</desc>",
        escape_text(&node.name),
        escape_text(&node.name)
    ));

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
            // 3D cube: top face (parallelogram) + right face + front face (fix #571)
            let offset = 12i32;
            // Top face
            out.push_str(&format!(
                "<polygon points=\"{},{} {},{} {},{} {},{}\" fill=\"#d4dff7\" stroke=\"#3730a3\" stroke-width=\"1\"/>",
                x, y,
                x + offset, y - offset,
                x + w + offset, y - offset,
                x + w, y
            ));
            // Right face
            out.push_str(&format!(
                "<polygon points=\"{},{} {},{} {},{} {},{}\" fill=\"#b8c8ef\" stroke=\"#3730a3\" stroke-width=\"1\"/>",
                x + w, y,
                x + w + offset, y - offset,
                x + w + offset, y + h - offset,
                x + w, y + h
            ));
            // Front face
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"#eef2ff\" stroke=\"#3730a3\" stroke-width=\"1.5\"/>",
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
            let fill = node.fill_color.as_deref().unwrap_or("#fef3c7");
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"60\" height=\"14\" fill=\"{}\" stroke=\"#92400e\" stroke-width=\"1\"/>",
                x, y, fill
            ));
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"#92400e\" stroke-width=\"1\"/>",
                x,
                y + 14,
                w,
                h - 14,
                fill
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
        FamilyNodeKind::Note => {
            render_note_card(out, x, y, w, h, &display);
            return;
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
    // Suppress the kind-tag for package/rectangle/folder container nodes — they already
    // show their label in a visual header/tab (fix #549).
    let is_package_container = matches!(
        node.kind,
        FamilyNodeKind::Package | FamilyNodeKind::Rectangle | FamilyNodeKind::Folder
    );
    if !is_package_container {
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"10\" fill=\"#475569\">{}</text>",
            cx,
            kind_tag_y,
            kind_label
        ));
    }
    render_node_stereotype_rows(out, node, cx, kind_tag_y + 13);
}

pub(super) fn render_node_stereotype_rows(
    out: &mut String,
    node: &FamilyNode,
    cx: i32,
    start_y: i32,
) {
    for (idx, member) in node
        .members
        .iter()
        .filter(|member| {
            let text = member.text.trim();
            text.starts_with("<<") && text.ends_with(">>")
        })
        .take(4)
        .enumerate()
    {
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"10\" fill=\"#64748b\">{}</text>",
            cx,
            start_y + idx as i32 * 12,
            escape_text(member.text.trim())
        ));
    }
}

pub(crate) fn render_note_card(out: &mut String, x: i32, y: i32, w: i32, h: i32, text: &str) {
    out.push_str(&format!(
        "<path d=\"M{x},{y} H{} L{} {} V{} H{x} Z\" fill=\"#fff8c4\" stroke=\"#8a6d00\" stroke-width=\"1.2\"/>",
        x + w - 16,
        x + w,
        y + 16,
        y + h
    ));
    out.push_str(&format!(
        "<path d=\"M{} {y} V{} H{}\" fill=\"none\" stroke=\"#8a6d00\" stroke-width=\"1\"/>",
        x + w - 16,
        y + 16,
        x + w
    ));
    let mut ty = y + 22;
    for line in text.lines().take(5) {
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" fill=\"#3b2f00\">{}</text>",
            x + 10,
            ty,
            escape_text(line)
        ));
        ty += 15;
    }
}

#[derive(Debug, Clone)]
pub(super) struct RenderGroupFrame {
    pub(super) kind: String,
    pub(super) label: Option<String>,
    pub(super) scope: String,
    pub(super) member_ids: Vec<String>,
    pub(super) depth: usize,
}

impl RenderGroupFrame {
    pub(super) fn display_label(&self) -> String {
        match self.label.as_deref() {
            Some(label) if !label.is_empty() => {
                // For boundary keywords like `rectangle` (used in usecase diagrams as
                // system-boundary frames, fix #553), the label alone is the display
                // name — the keyword is structural, not part of the visible text.
                if self.kind == "rectangle" {
                    label.to_string()
                } else {
                    format!("{} {}", self.kind, label)
                }
            }
            _ => self.kind.clone(),
        }
    }
}

pub(super) fn collect_render_group_frames(groups: &[FamilyGroup]) -> Vec<RenderGroupFrame> {
    let mut frames: std::collections::BTreeMap<String, RenderGroupFrame> =
        std::collections::BTreeMap::new();

    for group in groups {
        let explicit_scope = group
            .label
            .as_deref()
            .filter(|label| !label.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| group.kind.clone());
        if !group.member_ids.is_empty() {
            let scope = explicit_scope;
            let depth = scope.split("::").filter(|part| !part.is_empty()).count();
            let key = format!("{}\x1f{}", group.kind, scope);
            let entry = frames.entry(key).or_insert_with(|| RenderGroupFrame {
                kind: group.kind.clone(),
                label: group.label.clone(),
                scope: scope.clone(),
                member_ids: Vec::new(),
                depth: depth.saturating_sub(1),
            });
            entry.member_ids.extend(group.member_ids.iter().cloned());
        }

        for member_id in &group.member_ids {
            let node_id = member_id
                .split('\t')
                .next()
                .unwrap_or(member_id.as_str())
                .trim();
            if node_id.is_empty() {
                continue;
            }
            let parts = node_id
                .split("::")
                .filter(|part| !part.is_empty())
                .collect::<Vec<_>>();
            if parts.len() < 2 {
                continue;
            }
            for prefix_len in 1..parts.len() {
                let scope = parts[..prefix_len].join("::");
                let key = format!("{}\x1f{}", group.kind, scope);
                let label = parts.get(prefix_len - 1).map(|value| (*value).to_string());
                let entry = frames.entry(key).or_insert_with(|| RenderGroupFrame {
                    kind: group.kind.clone(),
                    label,
                    scope: scope.clone(),
                    member_ids: Vec::new(),
                    depth: prefix_len.saturating_sub(1),
                });
                entry.member_ids.push(node_id.to_string());
            }
        }
    }

    let mut frames = frames.into_values().collect::<Vec<_>>();
    for frame in &mut frames {
        frame.member_ids.sort();
        frame.member_ids.dedup();
    }
    frames.sort_by(|a, b| {
        (a.depth, a.scope.as_str(), a.kind.as_str()).cmp(&(
            b.depth,
            b.scope.as_str(),
            b.kind.as_str(),
        ))
    });
    frames
}

/// Styled variant of `render_family_node_shape` that applies `comp_style` for
/// Component/Interface nodes and falls back to the default for others.
pub(super) fn render_family_node_shape_styled(
    out: &mut String,
    node: &FamilyNode,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    comp_style: &ComponentStyle,
) {
    let cx = x + w / 2;
    let cy = y + h / 2;
    let display = node.label.clone().unwrap_or_else(|| node.name.clone());
    let kind_label = family_node_label(node.kind);
    out.push_str(&format!(
        "<desc data-uml-id=\"{}\">{}</desc>",
        escape_text(&node.name),
        escape_text(&node.name)
    ));

    match node.kind {
        FamilyNodeKind::Interface => {
            let r = 18;
            let fill = node
                .fill_color
                .as_deref()
                .unwrap_or(&comp_style.interface_color);
            out.push_str(&format!(
                "<circle class=\"uml-node uml-interface\" data-uml-kind=\"interface\" cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                cx, cy, r, fill, comp_style.border_color
            ));
        }
        FamilyNodeKind::Port => {
            let pw = 24;
            let ph = 24;
            let fill = node
                .fill_color
                .as_deref()
                .unwrap_or(&comp_style.interface_color);
            let port_dir = if node.members.iter().any(|m| m.text == "<<portin>>") {
                "in"
            } else if node.members.iter().any(|m| m.text == "<<portout>>") {
                "out"
            } else {
                ""
            };
            out.push_str(&format!(
                "<rect class=\"uml-node uml-port\" data-uml-kind=\"port\" data-uml-port-direction=\"{}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"2\" ry=\"2\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                escape_text(port_dir),
                cx - pw / 2,
                cy - ph / 2,
                pw,
                ph,
                fill,
                comp_style.border_color
            ));
        }
        FamilyNodeKind::Component => {
            let fill = node
                .fill_color
                .as_deref()
                .unwrap_or(&comp_style.background_color);
            out.push_str(&format!(
                "<rect class=\"uml-node uml-component\" data-uml-kind=\"component\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"4\" ry=\"4\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                x, y, w, h, fill, comp_style.border_color
            ));
            // component badges (two small rectangles on the left edge)
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"16\" height=\"8\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                x - 4, y + 12, fill, comp_style.border_color
            ));
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"16\" height=\"8\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                x - 4, y + h - 20, fill, comp_style.border_color
            ));
        }
        FamilyNodeKind::Node
        | FamilyNodeKind::Frame
        | FamilyNodeKind::Artifact
        | FamilyNodeKind::Cloud
        | FamilyNodeKind::Storage
        | FamilyNodeKind::Database
        | FamilyNodeKind::Package
        | FamilyNodeKind::Rectangle
        | FamilyNodeKind::Folder
        | FamilyNodeKind::File
        | FamilyNodeKind::Card
        | FamilyNodeKind::Actor => {
            let fill = node
                .fill_color
                .as_deref()
                .unwrap_or(&comp_style.background_color);
            match node.kind {
                // 3D cube for deployment nodes (fix #571)
                FamilyNodeKind::Node | FamilyNodeKind::Frame => {
                    let offset = 12i32; // 3D depth offset (right and up)
                                        // Top face: parallelogram from front-top edge to back-top edge (shifted right+up).
                                        // Points: front-top-left → back-top-left → back-top-right → front-top-right
                    out.push_str(&format!(
                        "<polygon points=\"{},{} {},{} {},{} {},{}\" \
                         fill=\"#d4dff7\" stroke=\"{}\" stroke-width=\"1\"/>",
                        x,
                        y, // front-top-left
                        x + offset,
                        y - offset, // back-top-left (up + right)
                        x + w + offset,
                        y - offset, // back-top-right
                        x + w,
                        y, // front-top-right
                        comp_style.border_color
                    ));
                    // Right face: parallelogram from front-right edge to back-right edge.
                    // Points: front-top-right → back-top-right → back-bottom-right → front-bottom-right
                    out.push_str(&format!(
                        "<polygon points=\"{},{} {},{} {},{} {},{}\" \
                         fill=\"#b8c8ef\" stroke=\"{}\" stroke-width=\"1\"/>",
                        x + w,
                        y, // front-top-right
                        x + w + offset,
                        y - offset, // back-top-right
                        x + w + offset,
                        y + h - offset, // back-bottom-right
                        x + w,
                        y + h, // front-bottom-right
                        comp_style.border_color
                    ));
                    // Front face (main visible face, drawn last so it sits on top)
                    out.push_str(&format!(
                        "<rect class=\"uml-node uml-deployment-shape\" data-uml-kind=\"{}\" \
                         x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" \
                         fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                        kind_label,
                        x,
                        y,
                        w,
                        h,
                        escape_text(fill),
                        comp_style.border_color
                    ));
                }
                FamilyNodeKind::Database | FamilyNodeKind::Storage => {
                    out.push_str(&format!(
                        "<path class=\"uml-node uml-deployment-shape\" data-uml-kind=\"{}\" d=\"M{x},{top} C{x},{top_minus} {right},{top_minus} {right},{top} L{right},{bottom} C{right},{bottom_plus} {x},{bottom_plus} {x},{bottom} Z\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                        kind_label,
                        escape_text(fill),
                        comp_style.border_color,
                        top = y + 10,
                        top_minus = y,
                        right = x + w,
                        bottom = y + h - 10,
                        bottom_plus = y + h
                    ));
                    out.push_str(&format!(
                        "<ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"10\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                        cx,
                        y + 10,
                        w / 2,
                        escape_text(fill),
                        comp_style.border_color
                    ));
                }
                FamilyNodeKind::Cloud => {
                    out.push_str(&format!(
                        "<path class=\"uml-node uml-deployment-shape\" data-uml-kind=\"cloud\" d=\"M{} {} C{} {}, {} {}, {} {} C{} {}, {} {}, {} {} L{} {} C{} {}, {} {}, {} {} Z\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                        x + 24, y + 56,
                        x + 4, y + 54, x + 4, y + 28, x + 30, y + 28,
                        x + 36, y + 8, x + 76, y + 8, x + 88, y + 26,
                        x + w - 22, y + 26,
                        x + w - 2, y + 28, x + w - 4, y + 56, x + w - 28, y + 56,
                        escape_text(fill),
                        comp_style.border_color
                    ));
                }
                FamilyNodeKind::Folder => {
                    out.push_str(&format!(
                        "<path class=\"uml-node uml-deployment-shape\" data-uml-kind=\"folder\" d=\"M{x},{y} H{} L{} {} H{} V{} H{x} Z\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                        x + 66,
                        x + 82,
                        y + 14,
                        x + w,
                        y + h,
                        escape_text(fill),
                        comp_style.border_color
                    ));
                }
                FamilyNodeKind::Artifact | FamilyNodeKind::File => {
                    out.push_str(&format!(
                        "<path class=\"uml-node uml-deployment-shape\" data-uml-kind=\"{}\" d=\"M{x},{y} H{} L{} {} V{} H{x} Z\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                        kind_label,
                        x + w - 18,
                        x + w,
                        y + 18,
                        y + h,
                        escape_text(fill),
                        comp_style.border_color
                    ));
                }
                _ => {
                    out.push_str(&format!(
                        "<rect class=\"uml-node uml-deployment-shape\" data-uml-kind=\"{}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"6\" ry=\"6\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                        kind_label, x, y, w, h, fill, comp_style.border_color
                    ));
                }
            }
        }
        _ => {
            // Delegate to the non-styled version for all other shapes
            render_family_node_shape(out, node, x, y, w, h);
            return;
        }
    }

    // Label
    let (label_x, label_y) = match node.kind {
        FamilyNodeKind::Interface | FamilyNodeKind::Port => (cx, cy + 28),
        _ => (cx, cy + 6),
    };
    out.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"13\" font-weight=\"600\" fill=\"{}\">{}</text>",
        label_x,
        label_y,
        escape_text(&comp_style.font_color),
        escape_text(&display)
    ));
    let kind_tag_y = match node.kind {
        FamilyNodeKind::Interface | FamilyNodeKind::Port => label_y + 14,
        _ => y + 14,
    };
    // For Component, show «component» guillemet stereotype instead of raw "component" (fix #525).
    // For Package and Rectangle container nodes, suppress the kind-tag entirely — these
    // shapes display their label in a tab/header already (fix #549).
    let is_package_container = matches!(
        node.kind,
        FamilyNodeKind::Package | FamilyNodeKind::Rectangle | FamilyNodeKind::Folder
    );
    if !is_package_container {
        let kind_tag_text: std::borrow::Cow<str> = match node.kind {
            FamilyNodeKind::Component => std::borrow::Cow::Borrowed("\u{ab}component\u{bb}"),
            FamilyNodeKind::Interface => std::borrow::Cow::Borrowed("\u{ab}interface\u{bb}"),
            _ => std::borrow::Cow::Borrowed(kind_label),
        };
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"10\" fill=\"{}\">{}</text>",
            cx, kind_tag_y, escape_text(&comp_style.font_color), escape_text(&kind_tag_text)
        ));
    }
    render_node_stereotype_rows(out, node, cx, kind_tag_y + 13);
}
