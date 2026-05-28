use crate::ast::MemberModifier;
use crate::model::FamilyNodeKind;
use crate::render::svg::{creole_text, escape_text, render_actor_stick_figure};
use crate::theme::{effective_class_node_style, ActorStyle, ClassStyle};

use super::c4_nodes::{is_c4_kind, render_c4_node};
use super::class_layout::class_node_display_name;
use super::class_members::{
    builtin_type_stereotype_label, class_node_visibility_symbol, count_header_stereotype_members,
    is_family_style_member, is_user_stereotype, member_modifier_name, parse_member_divider,
    parse_member_modifiers, parse_visibility_member, render_map_rows, uml_visibility_name,
    MapRenderCtx,
};
use super::class_types::ClassNodeGeometry;
use super::cloud_icons::{find_cloud_stereotype, render_cloud_icon_box};
use super::family_node_shapes::{
    render_actor_awesome_figure, render_actor_hollow_figure, render_note_card,
};

pub(super) fn render_class_node(
    out: &mut String,
    node: &crate::model::FamilyNode,
    geometry: ClassNodeGeometry,
    class_style: &ClassStyle,
    namespace_separator: Option<&str>,
    hide_stereotype: bool,
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
        render_c4_node(out, node, x, y, w, h);
        return;
    }

    if node.kind == FamilyNodeKind::Note {
        render_note_card(out, x, y, w, h, node.label.as_deref().unwrap_or(&node.name));
        return;
    }

    // ── Cloud icon node rendering (AWS / Azure / GCP / tupadr3) ───────────────
    // Object nodes produced by cloud-library stubs carry stereotypes like
    // <<aws-EC2>>, <<azure-vm>>, <<gcp-BigQuery>>, <<fa-cloud>>.  Render them
    // as visually distinct, provider-coloured icon boxes instead of identical
    // generic stub boxes (P10 initiative — Refs #1258).
    if node.kind == FamilyNodeKind::Object {
        if let Some(cloud_icon) = find_cloud_stereotype(&node.members) {
            let display = node.label.clone().unwrap_or_else(|| node.name.clone());
            let node_id = node.alias.as_deref().unwrap_or(&node.name);
            render_cloud_icon_box(out, &cloud_icon, &display, x, y, w, h, header_h, node_id);
            return;
        }
    }

    let effective_style = effective_class_node_style(class_style, node);
    let fill = effective_style.fill.as_str();
    let stroke = effective_style.stroke.as_str();
    let font_color = effective_style.font_color.as_str();
    let member_color = effective_style.member_color.as_str();
    let stroke_dash = if effective_style.border_dashed {
        " stroke-dasharray=\"5 3\""
    } else {
        ""
    };
    let stroke_width = effective_style.stroke_width;
    let font_family = effective_style.font_family.as_str();
    let title_font_size = effective_style.title_font_size;
    let member_font_size = effective_style.member_font_size;
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
            _ => effective_style.header_color.as_str(),
        },
        FamilyNodeKind::Object => "#fef3c7",
        FamilyNodeKind::Map => "#fef3c7",
        FamilyNodeKind::UseCase | FamilyNodeKind::BusinessUseCase => "#dcfce7",
        _ => "#f1f5f9",
    };

    if matches!(node.kind, FamilyNodeKind::Diamond) {
        let cx = x + w / 2;
        let cy = y + h / 2;
        let r = (w.min(h) / 2).saturating_sub(3).max(12);
        out.push_str(&format!(
            "<polygon class=\"uml-node uml-diamond\" data-uml-kind=\"diamond\" data-uml-id=\"{}\" points=\"{},{} {},{} {},{} {},{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
            escape_text(node.alias.as_deref().unwrap_or(&node.name)),
            cx,
            cy - r,
            cx + r,
            cy,
            cx,
            cy + r,
            cx - r,
            cy,
            fill,
            stroke
        ));
        return;
    }

    if matches!(
        node.kind,
        FamilyNodeKind::Actor | FamilyNodeKind::BusinessActor
    ) {
        let cx = x + w / 2;
        let fig_cy = y + 21;
        if matches!(node.kind, FamilyNodeKind::BusinessActor) {
            out.push_str(&format!(
                "<rect class=\"uml-business-actor\" data-uml-kind=\"business-actor\" x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" rx=\"10\" ry=\"10\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{}/>",
                fill,
                stroke,
                stroke_width,
                stroke_dash
            ));
        }
        match class_style.actor_style {
            ActorStyle::Stick => {
                // Canonical stick-figure rendering for actors (issue #715).
                // Proportions are shared with the sequence renderer via render_actor_stick_figure.
                render_actor_stick_figure(out, cx, fig_cy, stroke);
            }
            ActorStyle::Awesome => render_actor_awesome_figure(out, cx, fig_cy, stroke),
            ActorStyle::Hollow => render_actor_hollow_figure(out, cx, fig_cy, stroke),
        }
        let name_y = match class_style.actor_style {
            // Stick-figure feet end at fig_cy + 23; keep the historical 4px gap.
            ActorStyle::Stick => fig_cy + 27,
            // The alternative PlantUML actor glyphs are bulkier silhouettes.
            ActorStyle::Awesome | ActorStyle::Hollow => fig_cy + 42,
        };
        out.push_str(&format!(
            "<text x=\"{cx}\" y=\"{name_y}\" text-anchor=\"middle\" font-family=\"{}\" font-size=\"{}\" font-weight=\"600\" fill=\"{}\">{name}</text>",
            escape_text(font_family),
            title_font_size,
            escape_text(font_color),
            name = escape_text(&node.name)
        ));
        // Stereotype / extra members below name
        let mut member_y = name_y + 14;
        for member in &node.members {
            let text = member.text.trim();
            if text.is_empty() || is_family_style_member(text) {
                continue;
            }
            if hide_stereotype && is_user_stereotype(text) {
                continue;
            }
            out.push_str(&format!(
                "<text x=\"{cx}\" y=\"{member_y}\" text-anchor=\"middle\" font-family=\"{}\" font-size=\"11\" fill=\"{}\">{}</text>",
                escape_text(font_family),
                escape_text(member_color),
                escape_text(text)
            ));
            member_y += 14;
        }
        return;
    }

    if matches!(
        node.kind,
        FamilyNodeKind::UseCase | FamilyNodeKind::BusinessUseCase
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
        // Name centered — the alias is the internal id only; do NOT display it (fix #478)
        out.push_str(&format!(
            "<text x=\"{cx}\" y=\"{ty}\" text-anchor=\"middle\" font-family=\"{}\" font-size=\"{}\" font-weight=\"600\" fill=\"{}\">{name}</text>",
            escape_text(font_family),
            title_font_size,
            escape_text(font_color),
            ty = cy + 4,
            name = escape_text(uc_display_name)
        ));
        // Members rendered below the ellipse (rare for usecases), skipping display-label slot
        let mut my = y + h + 14;
        for member in node.members.iter().skip(uc_member_skip) {
            let text = member.text.trim();
            if is_family_style_member(text) || (hide_stereotype && is_user_stereotype(text)) {
                continue;
            }
            out.push_str(&format!(
                "<text x=\"{tx}\" y=\"{my}\" text-anchor=\"middle\" font-family=\"{}\" font-size=\"{}\" fill=\"{mc}\">{m}</text>",
                escape_text(font_family),
                member_font_size,
                tx = x + w / 2,
                mc = member_color,
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
    if !hide_stereotype {
        for m in &node.members[..header_skip] {
            if let Some(builtin) = builtin_type_stereotype_label(&m.text) {
                header_stereotype_labels.push(builtin.to_string());
            } else if is_user_stereotype(&m.text) {
                // Convert <<foo>> → «foo»
                let inner = m.text.trim_start_matches("<<").trim_end_matches(">>");
                header_stereotype_labels.push(format!("\u{ab}{inner}\u{bb}"));
            }
        }
    }
    // Members to display: skip all header stereotype members
    let display_members = &node.members[header_skip..];

    // Outer rect
    out.push_str(&format!(
        "<rect x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" rx=\"4\" ry=\"4\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{stroke_width}\"{stroke_dash}/>",
    ));
    // Header band — taller when we display stereotype labels (14px per label — fix #470, #551)
    let stereotype_extra = (header_stereotype_labels.len() as i32) * 14;
    let effective_header_h = header_h + stereotype_extra;
    out.push_str(&format!(
        "<rect x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{hh}\" rx=\"4\" ry=\"4\" fill=\"{header_fill}\" stroke=\"{stroke}\" stroke-width=\"{stroke_width}\"{stroke_dash}/>",
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
            fc = escape_text(font_color),
            lbl = escape_text(label)
        ));
    }

    // Header text: class name or object instance label (`name : Type`).
    let header_text = class_node_display_name(node, namespace_separator);
    let class_visibility = class_node_visibility_symbol(node);
    let header_text = class_visibility
        .map(|symbol| format!("{symbol}{header_text}"))
        .unwrap_or(header_text);
    let class_visibility_attr = class_visibility
        .map(uml_visibility_name)
        .map(|name| format!(" data-uml-class-visibility=\"{name}\""))
        .unwrap_or_default();
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
        "<text x=\"{tx}\" y=\"{ty}\" text-anchor=\"middle\" font-family=\"{ff}\" font-size=\"{fs}\" font-weight=\"600\" fill=\"{fc}\"{td}{fi}{cv}>{txt}</text>",
        ff = escape_text(font_family),
        fs = title_font_size,
        fc = escape_text(font_color),
        tx = x + w / 2,
        ty = name_ty,
        td = text_decoration,
        fi = name_font_style,
        cv = class_visibility_attr,
        txt = escape_text(&header_text)
    ));

    if matches!(node.kind, FamilyNodeKind::Map) {
        render_map_rows(
            out,
            node,
            x,
            y,
            w,
            effective_header_h,
            &MapRenderCtx {
                font_family,
                member_font_size,
                member_color,
                stroke,
            },
        );
        return;
    }

    // Members — split by `--` / `..` divider tokens to draw compartment lines (fix #468).
    // We also auto-insert a divider between the last attribute and the first operation
    // when there is no explicit divider in the source (fix #468 second compartment).
    //
    // Pre-scan: detect whether there are both attributes and operations in display_members
    // so we know to auto-insert a divider at the transition boundary.
    let has_explicit_divider = display_members
        .iter()
        .any(|m| parse_member_divider(m.text.trim()).is_some());
    let auto_divider = if !has_explicit_divider {
        // Determine the index of the first operation (text containing '(') after at least one attribute.
        let mut first_op_idx: Option<usize> = None;
        let mut seen_attr = false;
        for (i, m) in display_members.iter().enumerate() {
            let t = m.text.trim();
            if parse_member_divider(t).is_some() || t.is_empty() {
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
        if is_family_style_member(raw_text) {
            continue;
        }
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
        // PlantUML 3.8: titled separators `-- Section --`, `== Title ==`, `__ sub __`, `.. note ..`
        if let Some(div_title) = parse_member_divider(raw_text) {
            // Draw a horizontal divider line (fix #468)
            let div_y = my - 8;
            out.push_str(&format!(
                "<line x1=\"{x}\" y1=\"{div_y}\" x2=\"{x2}\" y2=\"{div_y}\" stroke=\"{stroke}\" stroke-width=\"1\"/>",
                x2 = x + w
            ));
            // If the separator has a title, render it centered above the divider
            if let Some(title) = div_title {
                let title_escaped = crate::render::svg::escape_text(title);
                let cx = x + w / 2;
                let title_y = my - 10;
                out.push_str(&format!(
                    "<text x=\"{cx}\" y=\"{title_y}\" text-anchor=\"middle\" font-family=\"{ff}\" font-size=\"9\" fill=\"{stroke}\" font-style=\"italic\">{title_escaped}</text>",
                    ff = escape_text(font_family),
                ));
                my += 4; // extra vertical space for the title
            }
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
        let render_visibility_icons = class_style.attribute_icons;
        // If no explicit visibility color, fall back to member_color from style.
        let effective_color = if vis_sym.is_some() && render_visibility_icons {
            vis_color
        } else {
            member_color
        };
        // Reconstruct display text: keep visibility prefix + remaining text
        let display_text = if vis_sym.is_some() {
            format!("{}{}", vis_sym.unwrap_or(""), text_after_mod)
        } else {
            text_after_mod.to_string()
        };
        let visibility_attr = if render_visibility_icons {
            vis_sym
                .map(uml_visibility_name)
                .map(|name| format!(" data-uml-visibility=\"{name}\""))
                .unwrap_or_default()
        } else {
            String::new()
        };
        let modifier_attr = member_modifier_name(member.modifier.as_ref())
            .map(|name| format!(" data-uml-modifier=\"{name}\""))
            .unwrap_or_default();
        if let Some(required_text) = display_text.strip_prefix('*') {
            out.push_str(&format!(
                "<text class=\"uml-member uml-ie-member\" data-uml-ie-mandatory=\"true\"{visibility_attr}{modifier_attr} x=\"{tx}\" y=\"{my}\" font-family=\"{ff}\" font-size=\"{fs}\" fill=\"{vc}\"{sa}>\
                 <tspan font-weight=\"700\">*</tspan><tspan dx=\"4\">{m}</tspan></text>",
                ff = escape_text(font_family),
                fs = member_font_size,
                tx = x + 10,
                vc = effective_color,
                sa = style_attrs,
                m = escape_text(required_text.trim_start())
            ));
        } else {
            if display_text.contains("<$") {
                out.push_str(&creole_text(
                    x + 10,
                    my,
                    &format!(
                        "class=\"uml-member\"{visibility_attr}{modifier_attr} font-family=\"{}\" font-size=\"{}\" fill=\"{}\"{}",
                        escape_text(font_family),
                        member_font_size,
                        effective_color,
                        style_attrs
                    ),
                    &display_text,
                    effective_color,
                ));
            } else {
                out.push_str(&format!(
                    "<text class=\"uml-member\"{visibility_attr}{modifier_attr} x=\"{tx}\" y=\"{my}\" font-family=\"{ff}\" font-size=\"{fs}\" fill=\"{vc}\"{sa}>{m}</text>",
                    ff = escape_text(font_family),
                    fs = member_font_size,
                    tx = x + 10,
                    vc = effective_color,
                    sa = style_attrs,
                    m = escape_text(&display_text)
                ));
            }
        }
        my += 16;
    }
}
