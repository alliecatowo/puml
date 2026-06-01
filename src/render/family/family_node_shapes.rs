use crate::model::{FamilyNode, FamilyNodeKind};
use crate::render::svg::{creole_text, escape_text};

use super::class_members::{is_family_style_member, is_user_stereotype, parse_spot_member};
use super::tree::render_centered_multiline_text;

pub(super) fn render_actor_awesome_figure(out: &mut String, cx: i32, cy: i32, stroke: &str) {
    let head_cy = cy - 15;
    out.push_str(&format!(
        "<circle class=\"uml-actor-glyph\" data-uml-actor-style=\"awesome\" cx=\"{cx}\" cy=\"{head_cy}\" r=\"7\" fill=\"{stroke}\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>"
    ));
    let shoulder_y = head_cy + 11;
    let body_top = head_cy + 16;
    let body_bottom = head_cy + 37;
    out.push_str(&format!(
        "<path class=\"uml-actor-glyph\" data-uml-actor-style=\"awesome\" d=\"M{} {} Q{} {} {} {} L{} {} Q{} {} {} {} L{} {} Q{} {} {} {} Z\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
        cx - 15,
        body_bottom,
        cx - 13,
        shoulder_y,
        cx,
        body_top,
        cx + 15,
        body_bottom,
        cx + 8,
        body_bottom + 4,
        cx,
        body_bottom + 4,
        cx - 8,
        body_bottom + 4,
        cx - 15,
        body_bottom,
        cx - 15,
        body_bottom,
        stroke,
        stroke
    ));
}

pub(super) fn render_actor_hollow_figure(out: &mut String, cx: i32, cy: i32, stroke: &str) {
    let head_cy = cy - 15;
    out.push_str(&format!(
        "<circle class=\"uml-actor-glyph\" data-uml-actor-style=\"hollow\" cx=\"{cx}\" cy=\"{head_cy}\" r=\"7\" fill=\"none\" stroke=\"{stroke}\" stroke-width=\"1.8\"/>"
    ));
    let shoulder_y = head_cy + 11;
    let body_bottom = head_cy + 38;
    out.push_str(&format!(
        "<path class=\"uml-actor-glyph\" data-uml-actor-style=\"hollow\" d=\"M{} {} Q{} {} {} {} Q{} {} {} {} Q{} {} {} {}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.8\" stroke-linecap=\"round\" stroke-linejoin=\"round\"/>",
        cx - 16,
        body_bottom,
        cx - 13,
        shoulder_y,
        cx,
        shoulder_y,
        cx + 13,
        shoulder_y,
        cx + 16,
        body_bottom,
        cx,
        body_bottom + 6,
        cx - 16,
        body_bottom,
        stroke
    ));
}

pub(super) fn render_family_node_shape(
    out: &mut String,
    node: &FamilyNode,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    hide_stereotype: bool,
) {
    let cx = x + w / 2;
    let cy = y + h / 2;
    let display = node.label.clone().unwrap_or_else(|| node.name.clone());
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
    let label_last_y =
        render_centered_multiline_text(out, label_x, label_y, 13, "600", None, &display);
    let kind_tag_y = match node.kind {
        FamilyNodeKind::Interface
        | FamilyNodeKind::StateInitial
        | FamilyNodeKind::StateFinal
        | FamilyNodeKind::StateHistory => label_last_y + 14,
        _ => y + 14,
    };
    // PlantUML parity (#1347): suppress the implicit kind-tag caption entirely.
    // PlantUML never emits these decorative kind keywords; the shape itself is
    // sufficient signal of the kind. Subsumes #549 (Package/Rectangle/Folder).
    // User-supplied `<<stereotype>>` members are still rendered below via
    // `render_node_stereotype_rows`.
    //
    // #1465: place user stereotypes ABOVE the name label (label_y - 14) so
    // they never overlap the name text.  For Interface/StateInitial/StateFinal/
    // StateHistory the kind_tag_y is already above label_y; keep the old
    // formula for those kinds.
    if !hide_stereotype {
        let stereo_y = match node.kind {
            FamilyNodeKind::Interface
            | FamilyNodeKind::StateInitial
            | FamilyNodeKind::StateFinal
            | FamilyNodeKind::StateHistory => kind_tag_y + 13,
            _ => label_y - 14,
        };
        render_node_stereotype_rows(out, node, cx, stereo_y);
    }
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
            text.starts_with("<<")
                && text.ends_with(">>")
                && !matches!(text, "<<portin>>" | "<<portout>>")
                // Spot stereotype encoding (#1398) is rendered as a badge, not a text row.
                && parse_spot_member(text).is_none()
        })
        .take(4)
        .enumerate()
    {
        let text = member.text.trim();
        if let Some(sprite_label) = stereotype_sprite_label(text) {
            out.push_str(&creole_text(
                cx - 10,
                start_y + idx as i32 * 14,
                "font-family=\"monospace\" font-size=\"10\" fill=\"#64748b\"",
                &sprite_label,
                "#64748b",
            ));
            continue;
        }
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"10\" fill=\"#64748b\">{}</text>",
            cx,
            start_y + idx as i32 * 12,
            escape_text(text)
        ));
    }
}

fn stereotype_sprite_label(text: &str) -> Option<String> {
    let inner = text.strip_prefix("<<")?.strip_suffix(">>")?.trim();
    inner
        .strip_prefix('$')
        .filter(|name| !name.is_empty())
        .map(|name| format!("<${name}>"))
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

/// Render a UseCase or BusinessUseCase node as an ellipse / rounded-rect (#578, #1349).
///
/// Call sites in `class_node_render` call this instead of inlining the block, so that
/// `class_node_render.rs` stays within the 600-LOC file-size guardrail.
#[allow(clippy::too_many_arguments)]
pub(super) fn render_usecase_node(
    out: &mut String,
    node: &FamilyNode,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    fill: &str,
    stroke: &str,
    stroke_width: f32,
    stroke_dash: &str,
    font_family: &str,
    font_color: &str,
    member_color: &str,
    title_font_size: u32,
    member_font_size: u32,
    hide_stereotype: bool,
) {
    let cx = x + w / 2;
    let cy = y + h / 2;
    let rx = w / 2;
    let ry = h / 2;
    if matches!(node.kind, FamilyNodeKind::BusinessUseCase) {
        out.push_str(&format!(
            "<rect class=\"uml-business-usecase\" data-uml-kind=\"business-usecase\" x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" rx=\"18\" ry=\"18\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{stroke_width}\"{stroke_dash}/>",
        ));
    } else {
        out.push_str(&format!(
            "<ellipse cx=\"{cx}\" cy=\"{cy}\" rx=\"{rx}\" ry=\"{ry}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{stroke_width}\"{stroke_dash}/>",
        ));
    }
    // Resolve display name: namespace-qualified nodes (e.g. "Package::MP") encode
    // the human-readable label as members[0] when the parser embeds `as DisplayName`
    // inside a group. Detect this by checking that members[0] is plain text (not a
    // UML modifier line) and use it as the displayed label (fix #578).
    let (uc_display_name, uc_member_skip): (&str, usize) = if node.name.contains("::") {
        let first_member_is_label = node.members.first().is_some_and(|m| {
            let t = m.text.trim();
            !t.is_empty()
                && !t.starts_with("<<")
                && !t.starts_with('+')
                && !t.starts_with('-')
                && !t.starts_with('#')
                && !t.starts_with('~')
                && !t.starts_with('{')
                && !t.starts_with('\x1f')
                && !t.contains(':')
                && !t.contains('(')
        });
        if first_member_is_label {
            (node.members[0].text.trim(), 1)
        } else {
            let short = node.name.rsplit("::").next().unwrap_or(&node.name);
            (short, 0)
        }
    } else {
        (node.name.as_str(), 0)
    };
    // Collect extension point names (encoded as `\x1fuc:ext-point:NAME` members).
    // These are rendered as a horizontal divider + list inside the oval.
    let ext_points: Vec<&str> = node
        .members
        .iter()
        .filter_map(|m| m.text.strip_prefix("\x1fuc:ext-point:"))
        .collect();
    let has_ext_points = !ext_points.is_empty()
        || node
            .members
            .iter()
            .any(|m| m.text == "\x1fuc:ext-points-header");

    // Name centered — the alias is the internal id only; do NOT display it (fix #478).
    // When extension points are present, shift the name upward so the divider
    // and point list fit inside the oval below it.
    let name_ty = if has_ext_points {
        cy - (ry / 3).max(8)
    } else {
        cy + 4
    };
    out.push_str(&format!(
        "<text x=\"{cx}\" y=\"{name_ty}\" text-anchor=\"middle\" font-family=\"{}\" font-size=\"{}\" font-weight=\"600\" fill=\"{}\">{name}</text>",
        escape_text(font_family),
        title_font_size,
        escape_text(font_color),
        name = escape_text(uc_display_name)
    ));

    // Render extension-points section inside the ellipse.
    if has_ext_points {
        // Dividing line across the interior of the oval at ~40% from top.
        let div_y = cy - (ry / 6).max(4);
        // Half-chord width at div_y: w_chord = rx * sqrt(1 - ((div_y-cy)/ry)^2)
        let dy_frac = (div_y - cy) as f64 / ry as f64;
        let chord_half = (rx as f64 * (1.0 - dy_frac * dy_frac).max(0.0).sqrt()) as i32;
        let line_x1 = cx - chord_half + 4;
        let line_x2 = cx + chord_half - 4;
        out.push_str(&format!(
            "<line class=\"uml-usecase-ext-divider\" x1=\"{line_x1}\" y1=\"{div_y}\" x2=\"{line_x2}\" y2=\"{div_y}\" stroke=\"{stroke}\" stroke-width=\"1\"/>",
        ));
        // Extension point names listed below the divider.
        let mut ep_y = div_y + 13;
        for ep_name in &ext_points {
            out.push_str(&format!(
                "<text class=\"uml-usecase-ext-point\" x=\"{cx}\" y=\"{ep_y}\" text-anchor=\"middle\" font-family=\"{}\" font-size=\"9\" fill=\"{}\">{txt}</text>",
                escape_text(font_family),
                escape_text(member_color),
                txt = escape_text(ep_name)
            ));
            ep_y += 12;
        }
    }

    // Members rendered below the ellipse (rare for usecases), skipping display-label slot.
    // Skip internal uc: members — those are rendered inside the oval above.
    let mut my = y + h + 14;
    for member in node.members.iter().skip(uc_member_skip) {
        let text = member.text.trim();
        if is_family_style_member(text)
            || text.starts_with("\x1fuc:")
            || parse_spot_member(text).is_some()
            || (hide_stereotype && is_user_stereotype(text))
        {
            continue;
        }
        out.push_str(&format!(
            "<text x=\"{tx}\" y=\"{my}\" text-anchor=\"middle\" font-family=\"{}\" font-size=\"{}\" fill=\"{mc}\">{m}</text>",
            escape_text(font_family),
            member_font_size,
            tx = x + w / 2,
            mc = member_color,
            m = escape_text(text)
        ));
        my += 14;
    }
}
