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
            // IE/ER entity: light warm tan to distinguish from plain class boxes
            Some("\u{ab}entity\u{bb}") => "#fde68a",
            // Exception: reddish header (PlantUML convention)
            Some("\u{ab}exception\u{bb}") => "#fecaca",
            // Metaclass, stereotype, circle: neutral slate header
            Some("\u{ab}metaclass\u{bb}")
            | Some("\u{ab}stereotype\u{bb}")
            | Some("\u{ab}circle\u{bb}") => "#e2e8f0",
            // ── Smart-default DDD / architectural stereotype header colours (#1285) ──
            Some("\u{ab}controller\u{bb}") => "#bfdbfe", // light blue
            Some("\u{ab}service\u{bb}") => "#bbf7d0",    // light green
            Some("\u{ab}repository\u{bb}") => "#fef3c7", // light tan
            Some("\u{ab}value\u{bb}") => "#e9d5ff",      // lavender (DDD value object)
            Some("\u{ab}aggregate\u{bb}") => "#ffffff",  // white (thick border)
            Some("\u{ab}factory\u{bb}") => "#fed7aa",    // salmon
            Some("\u{ab}datatype\u{bb}") => "#f1f5f9",   // white-gray (double border)
            Some("\u{ab}utility\u{bb}") => "#cbd5e1",    // gray (corner U mark)
            _ => effective_style.header_color.as_str(),
        },
        FamilyNodeKind::Object => "#fef3c7",
        FamilyNodeKind::Map => "#fef3c7",
        FamilyNodeKind::UseCase | FamilyNodeKind::BusinessUseCase => "#dcfce7",
        _ => "#f1f5f9",
    };
    // ── Smart-default shape dispatch for DDD / architectural stereotypes (#1285) ─
    // When the leading stereotype is one of the mapped DDD/arch types, delegate
    // to a specialised SVG shape renderer.  The skinparam cascade (header_color
    // from effective_style) continues to win because the node will already have
    // been styled before we reach here.
    if node.kind == FamilyNodeKind::Class {
        let dispatched = render_smart_default_shape(
            out,
            node,
            geometry,
            builtin_type_marker,
            header_fill,
            fill,
            stroke,
            stroke_width,
            font_family,
            font_color,
            title_font_size,
            namespace_separator,
            hide_stereotype,
        );
        if dispatched {
            return;
        }
    }

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
            // Position the name in the upper portion of the ellipse.
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

    // Corner radius from `skinparam roundcorner <N>`; default keeps the
    // historical visual of 4px when the skinparam is not specified.
    let rx = class_style.round_corner.unwrap_or(4);
    // `skinparam shadowing true` drops a soft shadow under the outer rect.
    // The header rect is intentionally unshadowed so the band does not
    // appear to "float" above the body.
    let shadow_attr = if class_style.shadowing {
        " filter=\"url(#shadow)\""
    } else {
        ""
    };
    // Outer rect
    out.push_str(&format!(
        "<rect x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" rx=\"{rx}\" ry=\"{rx}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{stroke_width}\"{stroke_dash}{shadow_attr}/>",
    ));
    // Header band — taller when we display stereotype labels (14px per label — fix #470, #551)
    let stereotype_extra = (header_stereotype_labels.len() as i32) * 14;
    let effective_header_h = header_h + stereotype_extra;
    out.push_str(&format!(
        "<rect x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{hh}\" rx=\"{rx}\" ry=\"{rx}\" fill=\"{header_fill}\" stroke=\"{stroke}\" stroke-width=\"{stroke_width}\"{stroke_dash}/>",
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

// ── Smart-default shape dispatch for DDD / architectural stereotypes (#1285) ──

/// Render a DDD / architectural stereotype node using its canonical shape.
///
/// Returns `true` when a specialised shape was rendered (caller must `return`).
/// Returns `false` for stereotypes that have only a header colour but no bespoke
/// geometry, allowing the caller to fall through to the standard rect renderer.
///
/// Shape mapping (const table — issue #1285):
/// | Stereotype      | Shape                          | Header colour |
/// |-----------------|--------------------------------|---------------|
/// | `<<controller>>`| hexagon (flat top/bottom)      | `#bfdbfe`     |
/// | `<<service>>`   | pill / rounded-rect tall       | `#bbf7d0`     |
/// | `<<repository>>`| cylinder                       | `#fef3c7`     |
/// | `<<value>>`     | hexagon (flat top/bottom)      | `#e9d5ff`     |
/// | `<<aggregate>>` | thick-border rounded rect      | `#ffffff`     |
/// | `<<factory>>`   | rounded rect + header band     | `#fed7aa`     |
/// | `<<datatype>>`  | double-border rectangle        | `#f1f5f9`     |
/// | `<<utility>>`   | rectangle + corner U mark      | `#cbd5e1`     |
///
/// Opt-out: `!pragma stereotype_smart_defaults off` reverts to vanilla (plain
/// class box).  The pragma is accepted by the family normaliser which passes it
/// through `ClassStyle`; that path is tracked in issue #1285 and currently
/// implemented as a TODO pending pragma infrastructure work.
///
/// TODO(#1285): thread `ClassStyle::stereotype_smart_defaults` bool through the
/// normaliser pragma handler and check it here before dispatching.
#[allow(clippy::too_many_arguments)]
fn render_smart_default_shape(
    out: &mut String,
    node: &crate::model::FamilyNode,
    geometry: ClassNodeGeometry,
    builtin_type_marker: Option<&'static str>,
    header_fill: &str,
    fill: &str,
    stroke: &str,
    stroke_width: f32,
    font_family: &str,
    font_color: &str,
    title_font_size: u32,
    namespace_separator: Option<&str>,
    hide_stereotype: bool,
) -> bool {
    let ClassNodeGeometry { x, y, w, h, .. } = geometry;
    let node_id = node.alias.as_deref().unwrap_or(&node.name);
    let display_name = class_node_display_name(node, namespace_separator);
    // Collect any additional user-defined stereotypes beyond the first built-in one,
    // so they still appear in the header even when a smart-default shape is rendered.
    let extra_user_labels: Vec<String> = if hide_stereotype {
        Vec::new()
    } else {
        let header_skip = count_header_stereotype_members(&node.members);
        // Skip the first member (the built-in type marker), collect any remaining
        // leading user stereotypes.
        node.members[..header_skip]
            .iter()
            .skip(1) // skip the primary builtin marker
            .filter_map(|m| {
                if is_user_stereotype(&m.text) {
                    let inner = m.text.trim_start_matches("<<").trim_end_matches(">>");
                    Some(format!("\u{ab}{inner}\u{bb}"))
                } else {
                    None
                }
            })
            .collect()
    };

    match builtin_type_marker {
        Some("\u{ab}controller\u{bb}") => {
            render_hexagon_node(
                out,
                node_id,
                &display_name,
                "\u{ab}controller\u{bb}",
                &extra_user_labels,
                "uml-stereotype-controller",
                x,
                y,
                w,
                h,
                header_fill,
                fill,
                stroke,
                stroke_width,
                font_family,
                font_color,
                title_font_size,
                hide_stereotype,
            );
            true
        }
        Some("\u{ab}service\u{bb}") => {
            render_pill_node(
                out,
                node_id,
                &display_name,
                "\u{ab}service\u{bb}",
                &extra_user_labels,
                "uml-stereotype-service",
                x,
                y,
                w,
                h,
                header_fill,
                fill,
                stroke,
                stroke_width,
                font_family,
                font_color,
                title_font_size,
                hide_stereotype,
            );
            true
        }
        Some("\u{ab}repository\u{bb}") => {
            render_cylinder_node(
                out,
                node_id,
                &display_name,
                "\u{ab}repository\u{bb}",
                &extra_user_labels,
                "uml-stereotype-repository",
                x,
                y,
                w,
                h,
                header_fill,
                fill,
                stroke,
                stroke_width,
                font_family,
                font_color,
                title_font_size,
                hide_stereotype,
            );
            true
        }
        Some("\u{ab}value\u{bb}") => {
            render_hexagon_node(
                out,
                node_id,
                &display_name,
                "\u{ab}value\u{bb}",
                &extra_user_labels,
                "uml-stereotype-value",
                x,
                y,
                w,
                h,
                header_fill,
                fill,
                stroke,
                stroke_width,
                font_family,
                font_color,
                title_font_size,
                hide_stereotype,
            );
            true
        }
        Some("\u{ab}aggregate\u{bb}") => {
            render_thick_rounded_rect_node(
                out,
                node_id,
                &display_name,
                "\u{ab}aggregate\u{bb}",
                &extra_user_labels,
                "uml-stereotype-aggregate",
                x,
                y,
                w,
                h,
                header_fill,
                fill,
                stroke,
                stroke_width,
                font_family,
                font_color,
                title_font_size,
                hide_stereotype,
            );
            true
        }
        Some("\u{ab}factory\u{bb}") => {
            // Factory uses the standard rect layout; we only need a distinctive
            // header band.  Fall through to the default renderer but signal
            // "not dispatched" so the caller handles it via the standard path.
            // The header_fill is already set to the salmon colour.
            false
        }
        Some("\u{ab}datatype\u{bb}") => {
            render_double_border_rect_node(
                out,
                node_id,
                &display_name,
                "\u{ab}datatype\u{bb}",
                &extra_user_labels,
                "uml-stereotype-datatype",
                x,
                y,
                w,
                h,
                header_fill,
                fill,
                stroke,
                stroke_width,
                font_family,
                font_color,
                title_font_size,
                hide_stereotype,
            );
            true
        }
        Some("\u{ab}utility\u{bb}") => {
            render_corner_u_rect_node(
                out,
                node_id,
                &display_name,
                "\u{ab}utility\u{bb}",
                &extra_user_labels,
                "uml-stereotype-utility",
                x,
                y,
                w,
                h,
                header_fill,
                fill,
                stroke,
                stroke_width,
                font_family,
                font_color,
                title_font_size,
                hide_stereotype,
            );
            true
        }
        _ => false,
    }
}

// ── Individual smart-default shape renderers ────────────────────────────────

/// Helper: render the guillemet stereotype label(s) above the node name, and then
/// the node name itself, centred within a pre-drawn shape.
///
/// `stereotype_label` is the canonical built-in label (e.g. `«controller»`).
/// `extra_labels` is a slice of any additional user-defined stereotype labels that
/// should also appear in the header (e.g. `«internal»` when the source had both
/// `<<controller>> <<internal>>`).
fn render_smart_shape_labels(
    out: &mut String,
    display_name: &str,
    stereotype_label: &str,
    extra_labels: &[String],
    cx: i32,
    label_y: i32,
    name_y: i32,
    font_family: &str,
    font_color: &str,
    title_font_size: u32,
    hide_stereotype: bool,
) {
    if !hide_stereotype {
        out.push_str(&format!(
            "<text class=\"uml-stereotype\" x=\"{cx}\" y=\"{label_y}\" text-anchor=\"middle\" font-family=\"{ff}\" font-size=\"10\" fill=\"{fc}\">{lbl}</text>",
            ff = escape_text(font_family),
            fc = escape_text(font_color),
            lbl = escape_text(stereotype_label),
        ));
        // Render any additional user-defined stereotype labels below the primary one.
        for (i, extra) in extra_labels.iter().enumerate() {
            let extra_y = label_y + (i as i32 + 1) * 12;
            out.push_str(&format!(
                "<text class=\"uml-stereotype\" x=\"{cx}\" y=\"{extra_y}\" text-anchor=\"middle\" font-family=\"{ff}\" font-size=\"10\" fill=\"{fc}\">{lbl}</text>",
                ff = escape_text(font_family),
                fc = escape_text(font_color),
                lbl = escape_text(extra),
            ));
        }
    }
    out.push_str(&format!(
        "<text class=\"uml-node-name\" x=\"{cx}\" y=\"{name_y}\" text-anchor=\"middle\" font-family=\"{ff}\" font-size=\"{fs}\" font-weight=\"600\" fill=\"{fc}\">{name}</text>",
        ff = escape_text(font_family),
        fs = title_font_size,
        fc = escape_text(font_color),
        name = escape_text(display_name),
    ));
}

/// Render a flat-top hexagon node (used by `<<controller>>` and `<<value>>`).
///
/// Geometry: the hexagon is inscribed in the node bounding box.  Flat-top
/// means the top and bottom edges are horizontal; the left/right sides are
/// angled at 60°.
///
/// ```text
///   ┌─────────┐   ← horizontal top edge
///  /           \
/// │             │
///  \           /
///   └─────────┘   ← horizontal bottom edge
/// ```
///
/// The header band is drawn as a filled polygon occupying the upper ~30% of
/// the hexagon (clipped by the same outline shape).
#[allow(clippy::too_many_arguments)]
fn render_hexagon_node(
    out: &mut String,
    node_id: &str,
    display_name: &str,
    stereotype_label: &str,
    extra_labels: &[String],
    css_class: &str,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    header_fill: &str,
    fill: &str,
    stroke: &str,
    stroke_width: f32,
    font_family: &str,
    font_color: &str,
    title_font_size: u32,
    hide_stereotype: bool,
) {
    // Flat-top hexagon: indent = w / 5 gives a ~72° angle (close enough to 60°).
    let indent = (w / 5).max(8);
    let cx = x + w / 2;
    // Outer hexagon points (clockwise from top-left)
    //   TL=(x+indent, y)  TR=(x+w-indent, y)
    //   R=(x+w, y+h/2)
    //   BR=(x+w-indent, y+h)  BL=(x+indent, y+h)
    //   L=(x, y+h/2)
    let points = format!(
        "{},{} {},{} {},{} {},{} {},{} {},{}",
        x + indent,
        y,
        x + w - indent,
        y,
        x + w,
        y + h / 2,
        x + w - indent,
        y + h,
        x + indent,
        y + h,
        x,
        y + h / 2,
    );
    // Body hexagon
    out.push_str(&format!(
        "<polygon class=\"uml-node {css_class}\" data-uml-kind=\"class\" data-uml-id=\"{id}\" points=\"{pts}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"/>",
        id = escape_text(node_id),
        pts = points,
        sw = stroke_width,
    ));
    // Header band: same shape clipped to upper ~30%.
    let header_h = (h * 3 / 10).max(22);
    let hy = y + header_h;
    // Determine where the angled sides cross at hy.
    // Left side: from (x+indent, y) to (x, y+h/2).  Parametric t = (hy-y)/(h/2 - 0).
    // But the left angled side only starts at y+0 (top of indent corner).
    // Simpler: at row `hy`, the hexagon's left edge = x + indent * (1 - 2*(hy-y)/h).max(0)
    // The left edge of the angled part: as we go from y to y+h/2, x goes from x+indent to x.
    let t_top = (hy - y) as f64 / (h as f64 / 2.0); // 0..1 from top to mid
    let left_x_at_hy = if t_top <= 1.0 {
        x as f64 + indent as f64 * (1.0 - t_top)
    } else {
        // below midpoint — left side goes back out
        let t_bot = t_top - 1.0;
        x as f64 + indent as f64 * t_bot
    };
    let right_x_at_hy = if t_top <= 1.0 {
        (x + w) as f64 - indent as f64 * (1.0 - t_top)
    } else {
        let t_bot = t_top - 1.0;
        (x + w) as f64 - indent as f64 * t_bot
    };
    let hx_l = left_x_at_hy.round() as i32;
    let hx_r = right_x_at_hy.round() as i32;
    // Header polygon: top two corners of the hexagon + the band cut
    let header_pts = format!(
        "{},{} {},{} {},{} {},{}",
        x + indent,
        y,
        x + w - indent,
        y,
        hx_r,
        hy,
        hx_l,
        hy,
    );
    out.push_str(&format!(
        "<polygon class=\"{css_class}-header\" points=\"{pts}\" fill=\"{hf}\" stroke=\"none\"/>",
        pts = header_pts,
        hf = header_fill,
    ));
    // Header bottom border line
    out.push_str(&format!(
        "<line x1=\"{hx_l}\" y1=\"{hy}\" x2=\"{hx_r}\" y2=\"{hy}\" stroke=\"{stroke}\" stroke-width=\"1\"/>",
    ));
    // Stereotype label + node name
    let label_y = y + 12;
    let name_y = y + header_h - 6;
    render_smart_shape_labels(
        out,
        display_name,
        stereotype_label,
        extra_labels,
        cx,
        label_y,
        name_y,
        font_family,
        font_color,
        title_font_size,
        hide_stereotype,
    );
}

/// Render a pill (heavily-rounded rectangle) node — used by `<<service>>`.
///
/// The pill is a rectangle where `rx = ry = h/2`, making the left and right
/// ends fully rounded.
#[allow(clippy::too_many_arguments)]
fn render_pill_node(
    out: &mut String,
    node_id: &str,
    display_name: &str,
    stereotype_label: &str,
    extra_labels: &[String],
    css_class: &str,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    header_fill: &str,
    fill: &str,
    stroke: &str,
    stroke_width: f32,
    font_family: &str,
    font_color: &str,
    title_font_size: u32,
    hide_stereotype: bool,
) {
    let r = h / 2; // full radius → pill shape
    let cx = x + w / 2;
    // Body pill
    out.push_str(&format!(
        "<rect class=\"uml-node {css_class}\" data-uml-kind=\"class\" data-uml-id=\"{id}\" x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" rx=\"{r}\" ry=\"{r}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"/>",
        id = escape_text(node_id),
        sw = stroke_width,
    ));
    // Header band: upper ~30% of the pill, clipped via a rect with the same rx.
    let header_h = (h * 3 / 10).max(22);
    // Draw header fill as a rect that is clipped by the pill outline.
    // We clip by drawing the header rect and the full pill body on top so the
    // corners are handled naturally by the underlying pill rect.
    out.push_str(&format!(
        "<rect class=\"{css_class}-header\" x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{header_h}\" rx=\"{r}\" ry=\"{r}\" fill=\"{hf}\" stroke=\"none\"/>",
        hf = header_fill,
    ));
    // Erase the bottom-rounded corners of the header rect (it has rounded bottom corners
    // that should be square at the divider line).  Cover with the body fill.
    let sq_y = y + header_h - r;
    if sq_y > y {
        out.push_str(&format!(
            "<rect x=\"{x}\" y=\"{sq_y}\" width=\"{w}\" height=\"{r}\" fill=\"{hf}\" stroke=\"none\"/>",
            hf = header_fill,
        ));
    }
    // Header separator line
    let hy = y + header_h;
    out.push_str(&format!(
        "<line x1=\"{x}\" y1=\"{hy}\" x2=\"{x2}\" y2=\"{hy}\" stroke=\"{stroke}\" stroke-width=\"1\"/>",
        x2 = x + w,
    ));
    // Stereotype label + node name
    let label_y = y + 12;
    let name_y = y + header_h - 6;
    render_smart_shape_labels(
        out,
        display_name,
        stereotype_label,
        extra_labels,
        cx,
        label_y,
        name_y,
        font_family,
        font_color,
        title_font_size,
        hide_stereotype,
    );
}

/// Render a cylinder node — used by `<<repository>>`.
///
/// A cylinder is drawn as:
///   - A rectangle for the body.
///   - An ellipse cap at the top.
///   - An ellipse cap at the bottom (body fill, so it merges).
///
/// The top ellipse acts as the header band.
#[allow(clippy::too_many_arguments)]
fn render_cylinder_node(
    out: &mut String,
    node_id: &str,
    display_name: &str,
    stereotype_label: &str,
    extra_labels: &[String],
    css_class: &str,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    header_fill: &str,
    fill: &str,
    stroke: &str,
    stroke_width: f32,
    font_family: &str,
    font_color: &str,
    title_font_size: u32,
    hide_stereotype: bool,
) {
    let ell_ry = (h / 8).max(6); // vertical radius of the ellipse caps
    let cx = x + w / 2;
    let cy_top = y + ell_ry;
    let cy_bot = y + h - ell_ry;
    let rx = w / 2;
    // Body rect (flush between the two ellipse caps)
    out.push_str(&format!(
        "<rect class=\"uml-node {css_class}\" data-uml-kind=\"class\" data-uml-id=\"{id}\" x=\"{x}\" y=\"{cy_top}\" width=\"{w}\" height=\"{body_h}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"/>",
        id = escape_text(node_id),
        body_h = cy_bot - cy_top,
        sw = stroke_width,
    ));
    // Bottom ellipse cap (body fill, no top-of-stroke so it blends)
    out.push_str(&format!(
        "<ellipse cx=\"{cx}\" cy=\"{cy_bot}\" rx=\"{rx}\" ry=\"{ell_ry}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"/>",
        sw = stroke_width,
    ));
    // Top ellipse cap = header fill
    out.push_str(&format!(
        "<ellipse class=\"{css_class}-header\" cx=\"{cx}\" cy=\"{cy_top}\" rx=\"{rx}\" ry=\"{ell_ry}\" fill=\"{hf}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"/>",
        hf = header_fill,
        sw = stroke_width,
    ));
    // Left / right body border lines (connecting the two ellipse caps)
    out.push_str(&format!(
        "<line x1=\"{x}\" y1=\"{cy_top}\" x2=\"{x}\" y2=\"{cy_bot}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"/>",
        sw = stroke_width,
    ));
    out.push_str(&format!(
        "<line x1=\"{x2}\" y1=\"{cy_top}\" x2=\"{x2}\" y2=\"{cy_bot}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"/>",
        x2 = x + w,
        sw = stroke_width,
    ));
    // Stereotype label + node name inside the top cap area
    let label_y = cy_top - 4;
    let name_y = cy_top + ell_ry + 14;
    render_smart_shape_labels(
        out,
        display_name,
        stereotype_label,
        extra_labels,
        cx,
        label_y,
        name_y,
        font_family,
        font_color,
        title_font_size,
        hide_stereotype,
    );
}

/// Render a thick-border rounded rect node — used by `<<aggregate>>`.
///
/// Identical to the standard class box but with `stroke-width` tripled to give
/// a visually "heavier" boundary (DDD aggregate root convention).
#[allow(clippy::too_many_arguments)]
fn render_thick_rounded_rect_node(
    out: &mut String,
    node_id: &str,
    display_name: &str,
    stereotype_label: &str,
    extra_labels: &[String],
    css_class: &str,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    header_fill: &str,
    fill: &str,
    stroke: &str,
    stroke_width: f32,
    font_family: &str,
    font_color: &str,
    title_font_size: u32,
    hide_stereotype: bool,
) {
    let rx = 4;
    let thick_sw = (stroke_width * 3.0).min(6.0);
    let cx = x + w / 2;
    // Body rect
    out.push_str(&format!(
        "<rect class=\"uml-node {css_class}\" data-uml-kind=\"class\" data-uml-id=\"{id}\" x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" rx=\"{rx}\" ry=\"{rx}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"/>",
        id = escape_text(node_id),
        sw = thick_sw,
    ));
    // Header band
    let header_h = 28_i32.max(h / 3);
    out.push_str(&format!(
        "<rect class=\"{css_class}-header\" x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{header_h}\" rx=\"{rx}\" ry=\"{rx}\" fill=\"{hf}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"/>",
        hf = header_fill,
        sw = thick_sw,
    ));
    // Square off the bottom corners of the header band
    let sq_y = y + header_h - rx;
    out.push_str(&format!(
        "<rect x=\"{x}\" y=\"{sq_y}\" width=\"{w}\" height=\"{rx}\" fill=\"{hf}\" stroke=\"none\"/>",
        hf = header_fill,
    ));
    // Header separator line
    let hy = y + header_h;
    out.push_str(&format!(
        "<line x1=\"{x}\" y1=\"{hy}\" x2=\"{x2}\" y2=\"{hy}\" stroke=\"{stroke}\" stroke-width=\"1\"/>",
        x2 = x + w,
    ));
    // Stereotype label + node name
    let label_y = y + 12;
    let name_y = y + header_h - 6;
    render_smart_shape_labels(
        out,
        display_name,
        stereotype_label,
        extra_labels,
        cx,
        label_y,
        name_y,
        font_family,
        font_color,
        title_font_size,
        hide_stereotype,
    );
}

/// Render a double-border rectangle node — used by `<<datatype>>`.
///
/// Two concentric rectangles:  the outer one is the standard border; the inner
/// one is inset by 3 px and uses the same stroke colour, creating a "double
/// frame" effect (UML datatype convention).
#[allow(clippy::too_many_arguments)]
fn render_double_border_rect_node(
    out: &mut String,
    node_id: &str,
    display_name: &str,
    stereotype_label: &str,
    extra_labels: &[String],
    css_class: &str,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    header_fill: &str,
    fill: &str,
    stroke: &str,
    stroke_width: f32,
    font_family: &str,
    font_color: &str,
    title_font_size: u32,
    hide_stereotype: bool,
) {
    let cx = x + w / 2;
    // Outer rect
    out.push_str(&format!(
        "<rect class=\"uml-node {css_class}\" data-uml-kind=\"class\" data-uml-id=\"{id}\" x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"/>",
        id = escape_text(node_id),
        sw = stroke_width,
    ));
    // Inner rect (inset by 3px on each side)
    let inset = 3_i32;
    out.push_str(&format!(
        "<rect class=\"{css_class}-inner\" x=\"{ix}\" y=\"{iy}\" width=\"{iw}\" height=\"{ih}\" fill=\"none\" stroke=\"{stroke}\" stroke-width=\"{sw}\"/>",
        ix = x + inset,
        iy = y + inset,
        iw = (w - inset * 2).max(1),
        ih = (h - inset * 2).max(1),
        sw = stroke_width,
    ));
    // Header band (within the outer rect)
    let header_h = 28_i32.max(h / 3);
    out.push_str(&format!(
        "<rect class=\"{css_class}-header\" x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{header_h}\" fill=\"{hf}\" stroke=\"none\"/>",
        hf = header_fill,
    ));
    // Header separator line
    let hy = y + header_h;
    out.push_str(&format!(
        "<line x1=\"{x}\" y1=\"{hy}\" x2=\"{x2}\" y2=\"{hy}\" stroke=\"{stroke}\" stroke-width=\"1\"/>",
        x2 = x + w,
    ));
    // Stereotype label + node name
    let label_y = y + 12;
    let name_y = y + header_h - 6;
    render_smart_shape_labels(
        out,
        display_name,
        stereotype_label,
        extra_labels,
        cx,
        label_y,
        name_y,
        font_family,
        font_color,
        title_font_size,
        hide_stereotype,
    );
}

/// Render a rectangle with a corner U mark — used by `<<utility>>`.
///
/// The corner mark is a small "U" glyph (drawn as a rounded-bottom rect path)
/// in the top-right corner to signal static/utility semantics.
#[allow(clippy::too_many_arguments)]
fn render_corner_u_rect_node(
    out: &mut String,
    node_id: &str,
    display_name: &str,
    stereotype_label: &str,
    extra_labels: &[String],
    css_class: &str,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    header_fill: &str,
    fill: &str,
    stroke: &str,
    stroke_width: f32,
    font_family: &str,
    font_color: &str,
    title_font_size: u32,
    hide_stereotype: bool,
) {
    let cx = x + w / 2;
    // Body rect
    out.push_str(&format!(
        "<rect class=\"uml-node {css_class}\" data-uml-kind=\"class\" data-uml-id=\"{id}\" x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{sw}\"/>",
        id = escape_text(node_id),
        sw = stroke_width,
    ));
    // Header band
    let header_h = 28_i32.max(h / 3);
    out.push_str(&format!(
        "<rect class=\"{css_class}-header\" x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{header_h}\" fill=\"{hf}\" stroke=\"none\"/>",
        hf = header_fill,
    ));
    // Header separator line
    let hy = y + header_h;
    out.push_str(&format!(
        "<line x1=\"{x}\" y1=\"{hy}\" x2=\"{x2}\" y2=\"{hy}\" stroke=\"{stroke}\" stroke-width=\"1\"/>",
        x2 = x + w,
    ));
    // Corner U mark: a small "U" in the top-right area of the header.
    // Drawn as a path: two vertical lines joined at the bottom by a semicircle.
    let u_w = 10_i32;
    let u_h = 10_i32;
    let u_x = x + w - u_w - 4;
    let u_y = y + 4;
    let u_rx = u_w / 2;
    out.push_str(&format!(
        "<path class=\"{css_class}-corner-u\" d=\"M {ux},{uy} L {ux},{uy2} A {rx},{rx} 0 0 0 {ux2},{uy2} L {ux2},{uy}\" fill=\"none\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>",
        ux = u_x,
        uy = u_y,
        uy2 = u_y + u_h,
        ux2 = u_x + u_w,
        rx = u_rx,
    ));
    // Stereotype label + node name
    let label_y = y + 12;
    let name_y = y + header_h - 6;
    render_smart_shape_labels(
        out,
        display_name,
        stereotype_label,
        extra_labels,
        cx,
        label_y,
        name_y,
        font_family,
        font_color,
        title_font_size,
        hide_stereotype,
    );
}
