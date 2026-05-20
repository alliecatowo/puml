use super::*;

pub(super) struct ClassNodeGeometry {
    pub(super) x: i32,
    pub(super) y: i32,
    pub(super) w: i32,
    pub(super) h: i32,
    pub(super) header_h: i32,
}

/// Return the recognised kind-stereotype label for a type-marker member
/// (e.g. `"<<enum>>"` → `Some("«enumeration»")`).  Only the built-in
/// keyword markers produced by the parser qualify; user-defined stereotypes
/// like `"<<controller>>"` are NOT covered here (they are handled separately).
pub(super) fn builtin_type_stereotype_label(text: &str) -> Option<&'static str> {
    match text {
        "<<enum>>" => Some("\u{ab}enumeration\u{bb}"),
        "<<interface>>" => Some("\u{ab}interface\u{bb}"),
        "<<abstract>>" | "<<abstract class>>" => Some("\u{ab}abstract\u{bb}"),
        "<<annotation>>" => Some("\u{ab}annotation\u{bb}"),
        "<<protocol>>" => Some("\u{ab}protocol\u{bb}"),
        "<<struct>>" => Some("\u{ab}struct\u{bb}"),
        _ => None,
    }
}

/// Return true if `text` is an arbitrary user-defined stereotype marker
/// (any `<<…>>` value that is NOT one of the built-in type keywords).
pub(super) fn is_user_stereotype(text: &str) -> bool {
    text.starts_with("<<") && text.ends_with(">>") && builtin_type_stereotype_label(text).is_none()
}

/// Count how many leading members of `members` are header stereotypes that
/// should be rendered in the class-box header rather than as member rows.
/// This includes the optional built-in type marker (first position) plus any
/// consecutive user-defined stereotype markers that immediately follow it.
pub(super) fn count_header_stereotype_members(members: &[crate::ast::ClassMember]) -> usize {
    let mut skip = 0;
    // First member may be a built-in type marker (e.g. <<enum>>).
    if members
        .first()
        .is_some_and(|m| builtin_type_stereotype_label(&m.text).is_some())
    {
        skip += 1;
    }
    // Any consecutive user-defined <<…>> members directly after the type marker
    // (or at the start if there was no type marker) are also header stereotypes.
    while skip < members.len() && is_user_stereotype(&members[skip].text) {
        skip += 1;
    }
    skip
}

pub(super) fn render_class_node(
    out: &mut String,
    node: &crate::model::FamilyNode,
    family: &str,
    semantic_id: &str,
    geometry: ClassNodeGeometry,
    class_style: &ClassStyle,
    namespace_separator: Option<&str>,
) {
    let ClassNodeGeometry {
        x,
        y,
        w,
        h,
        header_h,
    } = geometry;

    // ── C4 node rendering ─────────────────────────────────────────────────────
    if is_c4_kind(node.kind) {
        out.push_str(&semantic_node_rect(
            semantic_id,
            family,
            family_node_label(node.kind),
            x,
            y,
            w,
            h,
        ));
        render_c4_node(out, node, x, y, w, h);
        return;
    }

    if node.kind == FamilyNodeKind::Note {
        out.push_str(&semantic_node_rect(semantic_id, family, "note", x, y, w, h));
        render_note_card(out, x, y, w, h, node.label.as_deref().unwrap_or(&node.name));
        return;
    }

    let fill = node
        .fill_color
        .as_deref()
        .unwrap_or(&class_style.background_color);
    let stroke = &class_style.border_color;
    let font_family = class_style.font_name.as_deref().unwrap_or("monospace");
    let title_font_size = class_style.font_size.unwrap_or(13);
    let title_font_size_i32 = title_font_size as i32;
    let member_font_size = title_font_size.saturating_sub(2).max(9);
    // Determine the header fill colour.  For classes we also inspect the
    // leading type-marker member so that enum / annotation / interface / abstract
    // classes each get a visually distinct header (fix #769).
    let builtin_type_marker = node
        .members
        .first()
        .and_then(|m| builtin_type_stereotype_label(&m.text));
    let header_fill = match node.kind {
        FamilyNodeKind::Class => match builtin_type_marker {
            Some("\u{ab}enumeration\u{bb}") => "#ffffcc", // lemon — PlantUML enum convention
            Some("\u{ab}annotation\u{bb}") => "#fff0cc",  // warm amber for @annotation
            Some("\u{ab}interface\u{bb}") => "#dae8fc",   // light blue for interface
            Some("\u{ab}abstract\u{bb}") => "#f0e6ff",    // light lavender for abstract
            _ => class_style.header_color.as_str(),
        },
        FamilyNodeKind::Object => "#fef3c7",
        FamilyNodeKind::UseCase => "#dcfce7",
        _ => "#f1f5f9",
    };

    if matches!(node.kind, FamilyNodeKind::Actor) {
        out.push_str(&semantic_node_rect(
            semantic_id,
            family,
            "actor",
            x,
            y,
            w,
            h,
        ));
        // Canonical stick-figure rendering for actors (issue #715).
        // Proportions are shared with the sequence renderer via render_actor_stick_figure.
        // The figure centre cy is placed at y + 21 so the head top sits at y + 0.
        let cx = x + w / 2;
        let fig_cy = y + 21; // centre of figure; head top = fig_cy - 21
        render_actor_stick_figure(out, cx, fig_cy, stroke);
        // Name below the figure: feet end at fig_cy + 23, add 4 px gap.
        let name_y = fig_cy + 27;
        out.push_str(&format!(
            "<text class=\"puml-label\" {} x=\"{cx}\" y=\"{name_y}\" text-anchor=\"middle\" font-family=\"{}\" font-size=\"{}\" font-weight=\"600\" fill=\"{}\">{name}</text>",
            text_semantic_attrs(semantic_id, "node-label", cx, name_y, &node.name, title_font_size_i32, true),
            escape_text(font_family),
            title_font_size,
            escape_text(&class_style.font_color),
            name = escape_text(&node.name)
        ));
        // Stereotype / extra members below name
        let mut member_y = name_y + 14;
        for member in &node.members {
            let text = member.text.trim();
            if text.is_empty() {
                continue;
            }
            out.push_str(&format!(
                "<text x=\"{cx}\" y=\"{member_y}\" text-anchor=\"middle\" font-family=\"{}\" font-size=\"11\" fill=\"#334155\">{}</text>",
                escape_text(font_family),
                escape_text(text)
            ));
            member_y += 14;
        }
        return;
    }

    if matches!(node.kind, FamilyNodeKind::UseCase) {
        // Ellipse rendering for use cases
        let cx = x + w / 2;
        let cy = y + h / 2;
        let rx = w / 2;
        let ry = h / 2;
        let node_attrs = crate::render::puml_node_attrs(
            semantic_id,
            family,
            "usecase",
            geometry_bbox(x, y, w, h),
        );
        out.push_str(&format!(
            "<ellipse class=\"uml-node puml-node\" data-uml-id=\"{}\" data-uml-kind=\"usecase\" {} cx=\"{cx}\" cy=\"{cy}\" rx=\"{rx}\" ry=\"{ry}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>",
            escape_text(semantic_id),
            node_attrs,
        ));
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
        // Name centered — the alias is the internal id only; do NOT display it (fix #478)
        out.push_str(&format!(
            "<text class=\"puml-label\" {} x=\"{cx}\" y=\"{ty}\" text-anchor=\"middle\" font-family=\"{}\" font-size=\"{}\" font-weight=\"600\" fill=\"{}\">{name}</text>",
            text_semantic_attrs(semantic_id, "node-label", cx, cy + 4, uc_display_name, title_font_size_i32, true),
            escape_text(font_family),
            title_font_size,
            escape_text(&class_style.font_color),
            ty = cy + 4,
            name = escape_text(uc_display_name)
        ));
        // Members rendered below the ellipse (rare for usecases), skipping display-label slot
        let mut my = y + h + 14;
        for member in node.members.iter().skip(uc_member_skip) {
            out.push_str(&format!(
                "<text x=\"{tx}\" y=\"{my}\" text-anchor=\"middle\" font-family=\"{}\" font-size=\"{}\" fill=\"{mc}\">{m}</text>",
                escape_text(font_family),
                member_font_size,
                tx = x + w / 2,
                mc = class_style.member_color,
                m = escape_text(&member.text)
            ));
            my += 14;
        }
        return;
    }

    // Collect all leading header stereotype labels (built-in type markers + user-defined
    // <<…>> markers — fix #470 for built-in types, fix #551 for user stereotypes).
    // These are rendered as guillemet labels in the header, NOT as ordinary member rows.
    let header_skip = count_header_stereotype_members(&node.members);
    // Build the list of guillemet labels to show in the header (top → bottom).
    let mut header_stereotype_labels: Vec<String> = Vec::new();
    for m in &node.members[..header_skip] {
        if let Some(builtin) = builtin_type_stereotype_label(&m.text) {
            header_stereotype_labels.push(builtin.to_string());
        } else if is_user_stereotype(&m.text) {
            // Convert <<foo>> → «foo»
            let inner = m.text.trim_start_matches("<<").trim_end_matches(">>");
            header_stereotype_labels.push(format!("\u{ab}{inner}\u{bb}"));
        }
    }
    // Members to display: skip all header stereotype members
    let display_members = &node.members[header_skip..];

    // Outer rect
    let node_attrs = crate::render::puml_node_attrs(
        semantic_id,
        family,
        family_node_label(node.kind),
        geometry_bbox(x, y, w, h),
    );
    out.push_str(&format!(
        "<rect class=\"uml-node puml-node\" data-uml-id=\"{}\" data-uml-kind=\"{}\" {} x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" rx=\"4\" ry=\"4\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>",
        escape_text(semantic_id),
        escape_text(family_node_label(node.kind)),
        node_attrs,
    ));
    // Header band — taller when we display stereotype labels (14px per label — fix #470, #551)
    let stereotype_extra = (header_stereotype_labels.len() as i32) * 14;
    let effective_header_h = header_h + stereotype_extra;
    out.push_str(&format!(
        "<rect x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{hh}\" rx=\"4\" ry=\"4\" fill=\"{header_fill}\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>",
        hh = effective_header_h
    ));
    // Header separator line
    out.push_str(&format!(
        "<line x1=\"{x}\" y1=\"{ly}\" x2=\"{x2}\" y2=\"{ly}\" stroke=\"{stroke}\" stroke-width=\"1\"/>",
        ly = y + effective_header_h,
        x2 = x + w
    ));

    // Render each stereotype label above the class name in the header (fix #470, #551)
    for (i, label) in header_stereotype_labels.iter().enumerate() {
        out.push_str(&format!(
            "<text x=\"{tx}\" y=\"{ty}\" text-anchor=\"middle\" font-family=\"{ff}\" font-size=\"10\" fill=\"{fc}\">{lbl}</text>",
            tx = x + w / 2,
            ty = y + 13 + (i as i32) * 14,
            ff = escape_text(font_family),
            fc = escape_text(&class_style.font_color),
            lbl = escape_text(label)
        ));
    }

    // Header text: class name (fix #486 — Object shows `Name : Type` underlined)
    let display_name = namespace_separator
        .filter(|sep| !sep.is_empty())
        .map(|sep| node.name.replace("::", sep))
        .unwrap_or_else(|| node.name.clone());
    // For objects: if the name contains " : " it's already in `name : Type` form;
    // otherwise we show just the name.  Either way we underline per UML.
    let header_text = display_name.clone();
    // Underline for objects (PlantUML convention — fix #486)
    let text_decoration = if matches!(node.kind, FamilyNodeKind::Object) {
        " text-decoration=\"underline\" text-decoration-thickness=\"1\""
    } else {
        ""
    };
    // Italic name for abstract classes and interfaces (fix #767 — PlantUML UML convention)
    let is_abstract_node = matches!(
        builtin_type_marker,
        Some("\u{ab}abstract\u{bb}") | Some("\u{ab}interface\u{bb}")
    );
    let name_font_style = if is_abstract_node {
        " font-style=\"italic\""
    } else {
        ""
    };
    let name_ty = y + effective_header_h - 9;
    out.push_str(&format!(
        "<text class=\"puml-label\" {} x=\"{tx}\" y=\"{ty}\" text-anchor=\"middle\" font-family=\"{ff}\" font-size=\"{fs}\" font-weight=\"600\" fill=\"{fc}\"{td}{fi}>{txt}</text>",
        text_semantic_attrs(semantic_id, "node-label", x + w / 2, name_ty, &header_text, title_font_size_i32, true),
        ff = escape_text(font_family),
        fs = title_font_size,
        fc = escape_text(&class_style.font_color),
        tx = x + w / 2,
        ty = name_ty,
        td = text_decoration,
        fi = name_font_style,
        txt = escape_text(&header_text)
    ));

    // Members — split by `--` / `..` divider tokens to draw compartment lines (fix #468).
    // We also auto-insert a divider between the last attribute and the first operation
    // when there is no explicit divider in the source (fix #468 second compartment).
    //
    // Pre-scan: detect whether there are both attributes and operations in display_members
    // so we know to auto-insert a divider at the transition boundary.
    let has_explicit_divider = display_members
        .iter()
        .any(|m| m.text.trim() == "--" || m.text.trim() == "..");
    let auto_divider = if !has_explicit_divider {
        // Determine the index of the first operation (text containing '(') after at least one attribute.
        let mut first_op_idx: Option<usize> = None;
        let mut seen_attr = false;
        for (i, m) in display_members.iter().enumerate() {
            let t = m.text.trim();
            if t == "--" || t == ".." || t.is_empty() {
                continue;
            }
            // Strip visibility prefix before checking for '('
            let (_vis, _col, rest) = parse_visibility_member(t);
            if rest.contains('(') {
                if seen_attr {
                    first_op_idx = Some(i);
                }
                break;
            } else {
                seen_attr = true;
            }
        }
        first_op_idx
    } else {
        None
    };

    let mut my = y + effective_header_h + 16;
    let mut section_started = false; // tracks if we've seen at least one non-divider member
    for (midx, member) in display_members.iter().enumerate() {
        let raw_text = member.text.trim();
        // Auto-insert divider before the first operation when no explicit divider exists (fix #468)
        if auto_divider == Some(midx) {
            let div_y = my - 8;
            out.push_str(&format!(
                "<line x1=\"{x}\" y1=\"{div_y}\" x2=\"{x2}\" y2=\"{div_y}\" stroke=\"{stroke}\" stroke-width=\"1\"/>",
                x2 = x + w
            ));
            section_started = false;
        }
        // Detect explicit divider tokens (`--` or `..` compartment separator)
        if raw_text == "--" || raw_text == ".." {
            // Draw a horizontal divider line (fix #468)
            let div_y = my - 8;
            out.push_str(&format!(
                "<line x1=\"{x}\" y1=\"{div_y}\" x2=\"{x2}\" y2=\"{div_y}\" stroke=\"{stroke}\" stroke-width=\"1\"/>",
                x2 = x + w
            ));
            section_started = false;
            continue;
        }
        // Skip blank display lines
        if raw_text.is_empty() {
            my += 16;
            continue;
        }
        let _ = section_started;
        section_started = true;
        let (vis_sym, vis_color, rest_after_vis) = parse_visibility_member(raw_text);
        let (base_style, text_after_mod) = parse_member_modifiers(rest_after_vis);
        let mut style_attrs = String::from(base_style);
        match &member.modifier {
            Some(MemberModifier::Abstract) | Some(MemberModifier::Field) => {
                if !style_attrs.contains("font-style") {
                    style_attrs.push_str(" font-style=\"italic\"");
                }
            }
            Some(MemberModifier::Static) => {
                if !style_attrs.contains("text-decoration") {
                    style_attrs.push_str(" text-decoration=\"underline\"");
                }
            }
            Some(MemberModifier::Method) | None => {
                // Interface members are implicitly abstract — render in italic (fix #767)
                if is_abstract_node && !style_attrs.contains("font-style") {
                    style_attrs.push_str(" font-style=\"italic\"");
                }
            }
        }
        // If no explicit visibility color, fall back to member_color from style
        let effective_color = if vis_sym.is_some() {
            vis_color
        } else {
            class_style.member_color.as_str()
        };
        // Reconstruct display text: keep visibility prefix + remaining text
        let display_text = if vis_sym.is_some() {
            format!("{}{}", vis_sym.unwrap_or(""), text_after_mod)
        } else {
            text_after_mod.to_string()
        };
        let visibility_attr = vis_sym
            .map(uml_visibility_name)
            .map(|name| format!(" data-uml-visibility=\"{name}\""))
            .unwrap_or_default();
        let modifier_attr = member_modifier_name(member.modifier.as_ref())
            .map(|name| format!(" data-uml-modifier=\"{name}\""))
            .unwrap_or_default();
        out.push_str(&format!(
            "<text class=\"uml-member\"{visibility_attr}{modifier_attr} x=\"{tx}\" y=\"{my}\" font-family=\"{ff}\" font-size=\"{fs}\" fill=\"{vc}\"{sa}>{m}</text>",
            ff = escape_text(font_family),
            fs = member_font_size,
            tx = x + 10,
            vc = effective_color,
            sa = style_attrs,
            m = escape_text(&display_text)
        ));
        my += 16;
    }
}

/// Ensure C4 and Actor nodes have enough minimum height to render their visual elements.
pub(super) fn c4_node_height(kind: FamilyNodeKind, computed: i32) -> i32 {
    match kind {
        // Person nodes need space for stick figure (44px) + body rect (≥50px)
        FamilyNodeKind::C4Person | FamilyNodeKind::C4PersonExt => computed.max(94),
        // All other C4 nodes need at least 60px for the label + type label
        k if is_c4_kind(k) => computed.max(60),
        // Usecase actor: stick figure (≈46px) + name label (≈18px) = 64px minimum
        FamilyNodeKind::Actor => computed.max(64),
        _ => computed,
    }
}

/// Returns true if the kind belongs to the C4 family.
pub(super) fn is_c4_kind(kind: FamilyNodeKind) -> bool {
    matches!(
        kind,
        FamilyNodeKind::C4Person
            | FamilyNodeKind::C4PersonExt
            | FamilyNodeKind::C4System
            | FamilyNodeKind::C4SystemExt
            | FamilyNodeKind::C4SystemDb
            | FamilyNodeKind::C4SystemQueue
            | FamilyNodeKind::C4Container
            | FamilyNodeKind::C4ContainerExt
            | FamilyNodeKind::C4ContainerDb
            | FamilyNodeKind::C4ContainerQueue
            | FamilyNodeKind::C4Component
            | FamilyNodeKind::C4ComponentExt
            | FamilyNodeKind::C4ComponentDb
            | FamilyNodeKind::C4ComponentQueue
            | FamilyNodeKind::C4Boundary
    )
}

/// Render a C4 architecture node with proper visual style.
///
/// Color conventions (following C4-PlantUML):
///   Person / Person_Ext   — person shape (stick figure above rounded rect)
///   System / *Ext         — saturated blue / gray rounded rect
///   Container             — blue rect with `[Container]` sub-label
///   Component             — lighter blue
///   *Db                   — cylinder (database icon)
///   *Queue                — open-ended cylinder
///   Boundary              — dashed rounded border
pub(super) fn render_c4_node(
    out: &mut String,
    node: &crate::model::FamilyNode,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
) {
    let cx = x + w / 2;
    let is_person = matches!(
        node.kind,
        FamilyNodeKind::C4Person | FamilyNodeKind::C4PersonExt
    );
    let is_db = matches!(
        node.kind,
        FamilyNodeKind::C4SystemDb | FamilyNodeKind::C4ContainerDb | FamilyNodeKind::C4ComponentDb
    );
    let is_queue = matches!(
        node.kind,
        FamilyNodeKind::C4SystemQueue
            | FamilyNodeKind::C4ContainerQueue
            | FamilyNodeKind::C4ComponentQueue
    );
    let is_boundary = matches!(node.kind, FamilyNodeKind::C4Boundary);
    let is_ext = matches!(
        node.kind,
        FamilyNodeKind::C4PersonExt
            | FamilyNodeKind::C4SystemExt
            | FamilyNodeKind::C4ContainerExt
            | FamilyNodeKind::C4ComponentExt
    );

    // Color palette
    let (fill, stroke, text_color) = if is_boundary {
        ("none", "#444444", "#444444")
    } else if is_ext {
        ("#8a8a8a", "#6b6b6b", "#ffffff")
    } else if matches!(
        node.kind,
        FamilyNodeKind::C4Component
            | FamilyNodeKind::C4ComponentDb
            | FamilyNodeKind::C4ComponentQueue
    ) {
        ("#85bbf0", "#5d82a8", "#000000")
    } else if matches!(
        node.kind,
        FamilyNodeKind::C4Container
            | FamilyNodeKind::C4ContainerDb
            | FamilyNodeKind::C4ContainerQueue
    ) {
        ("#438dd5", "#2e6da0", "#ffffff")
    } else {
        // Person, System, SystemDb, SystemQueue
        ("#1168bd", "#0d4f8f", "#ffffff")
    };

    let body_y = if is_person { y + 44 } else { y };
    let body_h = if is_person { h - 44 } else { h };
    let _ = body_h;

    // Boundary: just a dashed rounded rect
    if is_boundary {
        out.push_str(&format!(
            "<rect x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" rx=\"12\" ry=\"12\" \
             fill=\"none\" stroke=\"{stroke}\" stroke-width=\"2\" stroke-dasharray=\"8 4\"/>",
        ));
        out.push_str(&format!(
            "<text x=\"{cx}\" y=\"{ty}\" text-anchor=\"middle\" font-family=\"monospace\" \
             font-size=\"13\" font-weight=\"600\" fill=\"{stroke}\">{name}</text>",
            ty = y + 18,
            name = escape_text(&node.name)
        ));
        return;
    }

    // Person: stick figure above a rounded rect
    if is_person {
        // Draw figure above body
        let head_cx = cx;
        let head_cy = y + 10;
        // Head circle
        out.push_str(&format!(
            "<circle cx=\"{head_cx}\" cy=\"{head_cy}\" r=\"9\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>",
        ));
        // Body line
        out.push_str(&format!(
            "<line x1=\"{head_cx}\" y1=\"{by}\" x2=\"{head_cx}\" y2=\"{body_line_end}\" stroke=\"{stroke}\" stroke-width=\"2\"/>",
            by = head_cy + 9,
            body_line_end = head_cy + 22
        ));
        // Arms
        out.push_str(&format!(
            "<line x1=\"{ax1}\" y1=\"{ay}\" x2=\"{ax2}\" y2=\"{ay}\" stroke=\"{stroke}\" stroke-width=\"2\"/>",
            ax1 = head_cx - 12,
            ay = head_cy + 16,
            ax2 = head_cx + 12
        ));
        // Legs
        out.push_str(&format!(
            "<line x1=\"{head_cx}\" y1=\"{ly}\" x2=\"{lx2}\" y2=\"{ley}\" stroke=\"{stroke}\" stroke-width=\"2\"/>",
            ly = head_cy + 22,
            lx2 = head_cx - 10,
            ley = head_cy + 34
        ));
        out.push_str(&format!(
            "<line x1=\"{head_cx}\" y1=\"{ly}\" x2=\"{lx2}\" y2=\"{ley}\" stroke=\"{stroke}\" stroke-width=\"2\"/>",
            ly = head_cy + 22,
            lx2 = head_cx + 10,
            ley = head_cy + 34
        ));
    }

    // Database / cylinder shape
    if is_db {
        let ell_ry = 8i32;
        let rect_y = body_y + ell_ry;
        let rect_h = h - ell_ry * 2;
        // cylinder body
        out.push_str(&format!(
            "<rect x=\"{x}\" y=\"{rect_y}\" width=\"{w}\" height=\"{rect_h}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>",
        ));
        // top ellipse
        out.push_str(&format!(
            "<ellipse cx=\"{cx}\" cy=\"{rect_y}\" rx=\"{rx}\" ry=\"{ell_ry}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>",
            rx = w / 2
        ));
        // bottom ellipse
        out.push_str(&format!(
            "<ellipse cx=\"{cx}\" cy=\"{bot}\" rx=\"{rx}\" ry=\"{ell_ry}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>",
            bot = rect_y + rect_h,
            rx = w / 2
        ));
        // label
        out.push_str(&format!(
            "<text x=\"{cx}\" y=\"{ty}\" text-anchor=\"middle\" font-family=\"monospace\" \
             font-size=\"13\" font-weight=\"600\" fill=\"{text_color}\">{name}</text>",
            ty = rect_y + rect_h / 2 + 4,
            name = escape_text(&node.name)
        ));
        c4_sublabel(out, cx, rect_y + rect_h / 2 + 18, node, text_color);
        return;
    }

    // Queue: open-ended cylinder
    if is_queue {
        let ell_ry = 8i32;
        let rect_x = x + ell_ry;
        let rect_w = w - ell_ry * 2;
        let cy_mid = body_y + h / 2;
        // left open end (half-ellipse)
        out.push_str(&format!(
            "<path d=\"M{rect_x},{top} A{ell_ry},{ell_ry} 0 0 0 {rect_x},{bot}\" \
             fill=\"none\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>",
            top = body_y,
            bot = body_y + h
        ));
        // right closed end
        out.push_str(&format!(
            "<ellipse cx=\"{rx_cx}\" cy=\"{cy_mid}\" rx=\"{ell_ry}\" ry=\"{ry}\" \
             fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>",
            rx_cx = rect_x + rect_w,
            ry = h / 2
        ));
        // body rect
        out.push_str(&format!(
            "<rect x=\"{rect_x}\" y=\"{body_y}\" width=\"{rect_w}\" height=\"{h}\" \
             fill=\"{fill}\" stroke=\"none\"/>",
        ));
        // top/bottom lines
        out.push_str(&format!(
            "<line x1=\"{rect_x}\" y1=\"{top}\" x2=\"{rx_end}\" y2=\"{top}\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>",
            top = body_y,
            rx_end = rect_x + rect_w
        ));
        out.push_str(&format!(
            "<line x1=\"{rect_x}\" y1=\"{bot}\" x2=\"{rx_end}\" y2=\"{bot}\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>",
            bot = body_y + h,
            rx_end = rect_x + rect_w
        ));
        out.push_str(&format!(
            "<text x=\"{cx}\" y=\"{ty}\" text-anchor=\"middle\" font-family=\"monospace\" \
             font-size=\"13\" font-weight=\"600\" fill=\"{text_color}\">{name}</text>",
            ty = cy_mid + 4,
            name = escape_text(&node.name)
        ));
        c4_sublabel(out, cx, cy_mid + 18, node, text_color);
        return;
    }

    // Standard rounded rect (Person body, System, Container, Component)
    out.push_str(&format!(
        "<rect x=\"{x}\" y=\"{body_y}\" width=\"{w}\" height=\"{rect_h}\" rx=\"8\" ry=\"8\" \
         fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>",
        rect_h = h - (if is_person { 44 } else { 0 })
    ));

    // Type label line (e.g. "[Person]", "[System]", "[Container]")
    let type_label = c4_type_label(node.kind);
    let name_y = body_y + (if is_person { 24 } else { h / 2 - 4 });
    out.push_str(&format!(
        "<text x=\"{cx}\" y=\"{name_y}\" text-anchor=\"middle\" font-family=\"monospace\" \
         font-size=\"13\" font-weight=\"600\" fill=\"{text_color}\">{name}</text>",
        name = escape_text(&node.name)
    ));
    // Sub-label: [Type]
    out.push_str(&format!(
        "<text x=\"{cx}\" y=\"{sub_y}\" text-anchor=\"middle\" font-family=\"monospace\" \
         font-size=\"10\" fill=\"{text_color}\">{type_label}</text>",
        sub_y = name_y + 14
    ));
    // Description (from members[0] if any, shown as italic)
    if let Some(desc) = node.members.first() {
        out.push_str(&format!(
            "<text x=\"{cx}\" y=\"{desc_y}\" text-anchor=\"middle\" font-family=\"monospace\" \
             font-size=\"9\" font-style=\"italic\" fill=\"{text_color}\">{desc}</text>",
            desc_y = name_y + 26,
            desc = escape_text(&desc.text)
        ));
    }
}

/// Return the `[Type]` sub-label for a C4 kind.
pub(super) fn c4_type_label(kind: FamilyNodeKind) -> &'static str {
    match kind {
        FamilyNodeKind::C4Person => "[Person]",
        FamilyNodeKind::C4PersonExt => "[Person, ext]",
        FamilyNodeKind::C4System => "[System]",
        FamilyNodeKind::C4SystemExt => "[System, ext]",
        FamilyNodeKind::C4SystemDb => "[Database]",
        FamilyNodeKind::C4SystemQueue => "[Queue]",
        FamilyNodeKind::C4Container => "[Container]",
        FamilyNodeKind::C4ContainerExt => "[Container, ext]",
        FamilyNodeKind::C4ContainerDb => "[Database]",
        FamilyNodeKind::C4ContainerQueue => "[Queue]",
        FamilyNodeKind::C4Component => "[Component]",
        FamilyNodeKind::C4ComponentExt => "[Component, ext]",
        FamilyNodeKind::C4ComponentDb => "[Database]",
        FamilyNodeKind::C4ComponentQueue => "[Queue]",
        FamilyNodeKind::C4Boundary => "[Boundary]",
        _ => "",
    }
}

/// Render a small italic sub-label beneath the main name for C4 nodes.
pub(super) fn c4_sublabel(
    out: &mut String,
    cx: i32,
    y: i32,
    node: &crate::model::FamilyNode,
    color: &str,
) {
    let type_label = c4_type_label(node.kind);
    out.push_str(&format!(
        "<text x=\"{cx}\" y=\"{y}\" text-anchor=\"middle\" font-family=\"monospace\" \
         font-size=\"10\" fill=\"{color}\">{type_label}</text>",
    ));
    if let Some(desc) = node.members.first() {
        out.push_str(&format!(
            "<text x=\"{cx}\" y=\"{dy}\" text-anchor=\"middle\" font-family=\"monospace\" \
             font-size=\"9\" font-style=\"italic\" fill=\"{color}\">{desc}</text>",
            dy = y + 12,
            desc = escape_text(&desc.text)
        ));
    }
}
