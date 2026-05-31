use crate::ast::MemberModifier;
use crate::model::FamilyNodeKind;
use crate::render::svg::{creole_text, escape_text, render_actor_stick_figure};
use crate::theme::{effective_class_node_style, ActorStyle, ClassStyle, StyleMode, StyleSource};

use super::c4_nodes::{is_c4_kind, render_c4_node};
use super::class_layout::class_node_display_name;
use super::class_members::{
    builtin_type_stereotype_label, class_node_visibility_symbol, count_header_stereotype_members,
    emit_visibility_glyph, is_family_style_member, is_user_stereotype, member_modifier_name,
    parse_member_divider, parse_member_modifiers, parse_spot_member, parse_visibility_member,
    render_map_rows, uml_visibility_name, MapRenderCtx,
};
use super::class_smart_shapes::{ddd_smart_header_color, render_smart_default_shape};
use super::class_types::ClassNodeGeometry;
use super::cloud_icons::{find_cloud_stereotype, render_cloud_icon_box};
use super::family_node_shapes::{
    render_actor_awesome_figure, render_actor_hollow_figure, render_note_card, render_usecase_node,
};

pub(super) fn render_class_node(
    out: &mut String,
    node: &crate::model::FamilyNode,
    geometry: ClassNodeGeometry,
    class_style: &ClassStyle,
    namespace_separator: Option<&str>,
    hide_stereotype: bool,
    hide_circle: bool,
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
    // Determine the header fill colour — inspect the leading type-marker member so
    // that enum / annotation / interface / abstract classes get a distinct header (#769).
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
            // DDD/arch stereotypes (#1285): user override wins; else smart default colour.
            m if ddd_smart_header_color(m).is_some() => {
                if matches!(
                    effective_style.header_color.source(),
                    StyleSource::Stereotype | StyleSource::StyleBlock
                ) {
                    effective_style.header_color.as_str()
                } else {
                    ddd_smart_header_color(m).unwrap()
                }
            }
            _ => effective_style.header_color.as_str(),
        },
        FamilyNodeKind::Object => {
            // PlantUML mode: neutral gray header (no yellow chrome).
            if class_style.style_mode == StyleMode::Plantuml {
                "#e2e8f0"
            } else {
                "#fef3c7"
            }
        }
        FamilyNodeKind::Map => "#fef3c7",
        FamilyNodeKind::UseCase | FamilyNodeKind::BusinessUseCase => "#dcfce7",
        _ => "#f1f5f9",
    };
    // DDD/arch stereotype dispatch (#1285): delegate to specialised SVG shape renderer.
    if node.kind == FamilyNodeKind::Class
        && render_smart_default_shape(
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
        )
    {
        return;
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
            // Stick-figure proportions shared with sequence renderer via render_actor_stick_figure (#715).
            ActorStyle::Stick => render_actor_stick_figure(out, cx, fig_cy, stroke),
            ActorStyle::Awesome => render_actor_awesome_figure(out, cx, fig_cy, stroke),
            ActorStyle::Hollow => render_actor_hollow_figure(out, cx, fig_cy, stroke),
        }
        let name_y = match class_style.actor_style {
            ActorStyle::Stick => fig_cy + 27, // feet end at fig_cy+23; 4px gap
            ActorStyle::Awesome | ActorStyle::Hollow => fig_cy + 42, // bulkier silhouette
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
            // Always suppress spot stereotype encoding (badge is separate, not a text member).
            if parse_spot_member(text).is_some() {
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
        render_usecase_node(
            out,
            node,
            x,
            y,
            w,
            h,
            fill,
            stroke,
            stroke_width,
            stroke_dash,
            font_family,
            font_color,
            member_color,
            title_font_size,
            member_font_size,
            hide_stereotype,
        );
        return;
    }

    // Collect leading header stereotype labels (built-in + user-defined + spot — fix #470, #551);
    // rendered as guillemet labels in the header, not as ordinary member rows.
    // Each entry is `(label, is_sprite_ref)`: sprite stereotypes (`<<$foo>>`) are
    // rendered via creole_text so the inline-sprite path draws the actual pixel icon;
    // all other stereotypes are rendered as guillemet «…» plain text.
    // Spot stereotypes (#1398) also contribute a label row when they carry a non-empty label.
    let header_skip = count_header_stereotype_members(&node.members);
    let mut header_stereotype_labels: Vec<(String, bool)> = Vec::new();
    // Spot badge override: first <<spot:…>> member wins over the kind-default badge.
    // We extract it now so the badge section below can use it.
    let spot_override: Option<(char, String)> = node.members[..header_skip]
        .iter()
        .find_map(|m| parse_spot_member(&m.text).map(|(l, c, _)| (l, c)));
    if !hide_stereotype {
        for m in &node.members[..header_skip] {
            if let Some(builtin) = builtin_type_stereotype_label(&m.text) {
                header_stereotype_labels.push((builtin.to_string(), false));
            } else if let Some((_letter, _color, label)) = parse_spot_member(&m.text) {
                // Spot stereotype: render label text as «Label» when non-empty.
                if !label.is_empty() {
                    header_stereotype_labels.push((format!("\u{ab}{label}\u{bb}"), false));
                }
            } else if is_user_stereotype(&m.text) {
                let inner = m.text.trim_start_matches("<<").trim_end_matches(">>");
                if inner.starts_with('$') {
                    // Sprite stereotype: store as `<$name>` for creole_text to render
                    // as an actual sprite icon (#1401 — inline sprite definition support).
                    header_stereotype_labels.push((format!("<{inner}>"), true));
                } else {
                    // Convert <<foo>> → «foo»
                    header_stereotype_labels.push((format!("\u{ab}{inner}\u{bb}"), false));
                }
            }
        }
    }
    // Members to display: skip all header stereotype members
    let display_members = &node.members[header_skip..];

    // Corner radius from `skinparam roundcorner <N>`; 4px default keeps historical visual.
    let rx = class_style.round_corner.unwrap_or(4);
    // `skinparam shadowing true` drops a soft shadow; header rect is intentionally unshadowed.
    let shadow_attr = if class_style.shadowing {
        " filter=\"url(#shadow)\""
    } else {
        ""
    };
    // Outer rect — carries data-uml-id for test and tooling identification
    let node_id = escape_text(node.alias.as_deref().unwrap_or(&node.name));
    out.push_str(&format!(
        "<rect class=\"uml-node uml-class\" data-uml-id=\"{node_id}\" x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" rx=\"{rx}\" ry=\"{rx}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{stroke_width}\"{stroke_dash}{shadow_attr}/>",
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

    // Render each stereotype label above the class name in the header (fix #470, #551).
    // Sprite stereotypes (`is_sprite == true`) go through creole_text so the inline-sprite
    // path draws the actual pixel icon; all other labels are plain guillemet text.
    for (i, (label, is_sprite)) in header_stereotype_labels.iter().enumerate() {
        let ty = y + 13 + (i as i32) * 14;
        let tx = x + w / 2;
        if *is_sprite {
            out.push_str(&creole_text(
                tx - 8,
                ty,
                &format!(
                    "font-family=\"{}\" font-size=\"10\" fill=\"{}\"",
                    escape_text(font_family),
                    escape_text(font_color)
                ),
                label,
                font_color,
            ));
        } else {
            out.push_str(&format!(
                "<text x=\"{tx}\" y=\"{ty}\" text-anchor=\"middle\" font-family=\"{ff}\" font-size=\"10\" fill=\"{fc}\">{lbl}</text>",
                ff = escape_text(font_family),
                fc = escape_text(font_color),
                lbl = escape_text(label)
            ));
        }
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

    // ── Class-type badge (#1350, #1398) ──────────────────────────────────────
    // PlantUML renders a small coloured circle with a letter in the header-left
    // area of every class box to visually indicate the node kind.
    // Badge letter (kind-default, overridden by explicit spot stereotype — #1398):
    //   FamilyNodeKind::Class + <<abstract>>  → 'A' (green circle)
    //   FamilyNodeKind::Class + <<interface>> → 'I' (blue circle)
    //   FamilyNodeKind::Class + <<enum>>      → 'E' (yellow circle)
    //   FamilyNodeKind::Class (plain)         → 'C' (green circle)
    //   FamilyNodeKind::Object                → 'O' (amber circle)
    // Interface/Enum/Abstract are routed through FamilyNodeKind::Class with a
    // leading builtin-type stereotype marker — checked via builtin_type_marker.
    //
    // Spot stereotypes `<<(L,#color) Label>>` (encoded as <<spot:L:#color:Label>>)
    // override the kind-default badge letter AND colour (#1398).  Spot badges are
    // shown in BOTH PUML and PlantUML style modes (PlantUML itself renders them).
    let kind_default_badge: Option<(&str, &str, &str)> = match node.kind {
        FamilyNodeKind::Class => {
            let (letter, fill, stroke_c) = match builtin_type_marker {
                Some("\u{ab}abstract\u{bb}") => ("A", "#A2D5A2", "#2E7D32"),
                Some("\u{ab}interface\u{bb}") => ("I", "#90CAF9", "#1565C0"),
                Some("\u{ab}enumeration\u{bb}") => ("E", "#FFF176", "#F9A825"),
                Some("\u{ab}annotation\u{bb}") => ("@", "#FFCC80", "#E65100"),
                // DDD/arch and other stereotype flavours still get the green C.
                _ => ("C", "#A2D5A2", "#2E7D32"),
            };
            Some((letter, fill, stroke_c))
        }
        FamilyNodeKind::Object => Some(("O", "#FFD54F", "#F57F17")),
        _ => None,
    };
    // Suppress kind-default badges when `hide circle` is active or in PlantUML style
    // mode.  Spot badges always show when not hidden via `hide circle` (PlantUML parity).
    let show_kind_badge = !hide_circle && class_style.style_mode == StyleMode::Puml;
    let show_spot_badge = !hide_circle;

    if let Some((spot_letter, ref spot_color)) = spot_override {
        // Spot badge: user-specified letter + color.  White letter text over the spot color.
        if show_spot_badge {
            let badge_r = 8_i32;
            let badge_cx = x + badge_r + 4;
            let badge_cy = name_ty - 4;
            let letter_str = spot_letter.to_string();
            out.push_str(&format!(
                "<circle class=\"uml-class-badge uml-spot-badge\" cx=\"{badge_cx}\" cy=\"{badge_cy}\" r=\"{badge_r}\" fill=\"{spot_color}\" stroke=\"{spot_color}\" stroke-width=\"1\"/>",
            ));
            out.push_str(&format!(
                "<text class=\"uml-class-badge-letter uml-spot-badge-letter\" x=\"{badge_cx}\" y=\"{ty}\" text-anchor=\"middle\" font-family=\"{ff}\" font-size=\"9\" font-weight=\"700\" fill=\"#ffffff\">{letter_str}</text>",
                ty = badge_cy + 3,
                ff = escape_text(font_family),
            ));
        }
    } else if show_kind_badge {
        if let Some((badge_letter, badge_fill, badge_stroke)) = kind_default_badge {
            let badge_r = 8_i32;
            let badge_cx = x + badge_r + 4; // 4 px from the left inner edge
            let badge_cy = name_ty - 4; // vertically centre on the name baseline
            out.push_str(&format!(
                "<circle class=\"uml-class-badge\" cx=\"{badge_cx}\" cy=\"{badge_cy}\" r=\"{badge_r}\" fill=\"{badge_fill}\" stroke=\"{badge_stroke}\" stroke-width=\"1\"/>",
            ));
            out.push_str(&format!(
                "<text class=\"uml-class-badge-letter\" x=\"{badge_cx}\" y=\"{ty}\" text-anchor=\"middle\" font-family=\"{ff}\" font-size=\"9\" font-weight=\"700\" fill=\"{badge_stroke}\">{badge_letter}</text>",
                ty = badge_cy + 3, // +3 px to visually centre letter inside circle
                ff = escape_text(font_family),
            ));
        }
    }

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

    // Members — split by `--` / `..` dividers (fix #468); auto-insert divider between
    // last attribute and first operation when no explicit divider exists (fix #468).
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
        }
        // Detect explicit divider tokens (`--` / `..` / titled separators — fix #468)
        if let Some(div_title) = parse_member_divider(raw_text) {
            let div_y = my - 8;
            out.push_str(&format!(
                "<line x1=\"{x}\" y1=\"{div_y}\" x2=\"{x2}\" y2=\"{div_y}\" stroke=\"{stroke}\" stroke-width=\"1\"/>",
                x2 = x + w
            ));
            if let Some(title) = div_title {
                let title_escaped = crate::render::svg::escape_text(title);
                let cx = x + w / 2;
                let title_y = my - 10;
                out.push_str(&format!(
                    "<text x=\"{cx}\" y=\"{title_y}\" text-anchor=\"middle\" font-family=\"{ff}\" font-size=\"9\" fill=\"{stroke}\" font-style=\"italic\">{title_escaped}</text>",
                    ff = escape_text(font_family),
                ));
                my += 4;
            }
            continue;
        }
        // Skip blank display lines
        if raw_text.is_empty() {
            my += 16;
            continue;
        }
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
        // In PlantUML mode, suppress UML 2.x SVG glyphs and fall back to ASCII
        // prefixes (+/-/#/~) — this matches PlantUML's default behaviour.
        let render_visibility_icons =
            class_style.attribute_icons && class_style.style_mode == StyleMode::Puml;
        // If no explicit visibility color, fall back to member_color from style.
        let effective_color = if vis_sym.is_some() && render_visibility_icons {
            vis_color
        } else {
            member_color
        };
        // When rendering glyphs, strip the ASCII prefix from display text.
        // When icons are off, keep the prefix (legacy ASCII behaviour).
        let display_text = if vis_sym.is_some() && render_visibility_icons {
            text_after_mod.to_string()
        } else if vis_sym.is_some() {
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

        // Emit UML 2.x visibility glyph (SVG shape) before member text (#1349).
        // LAYOUT INVARIANT: always reserve the same glyph indent (14 px) when a
        // visibility symbol is present — even in PlantUML mode where the glyph is
        // suppressed.  This keeps text-x byte-identical across style modes so that
        // only paint differs (the circle/diamond/triangle SVG shape), not position.
        let glyph_x_offset = if vis_sym.is_some() {
            if render_visibility_icons {
                let gx = x + 10; // left margin
                let gy = my - 5; // vertically centred on the text baseline (~5px above)
                let is_method = matches!(member.modifier, Some(MemberModifier::Method))
                    || text_after_mod.contains('(');
                emit_visibility_glyph(out, vis_sym, vis_color, gx, gy, is_method)
            } else {
                // No glyph emitted, but reserve the same 14-px indent so layout
                // coordinates match PUML mode exactly.
                14
            }
        } else {
            0
        };

        if let Some(required_text) = display_text.strip_prefix('*') {
            out.push_str(&format!(
                "<text class=\"uml-member uml-ie-member\" data-uml-ie-mandatory=\"true\"{visibility_attr}{modifier_attr} x=\"{tx}\" y=\"{my}\" font-family=\"{ff}\" font-size=\"{fs}\" fill=\"{vc}\"{sa}>\
                 <tspan font-weight=\"700\">*</tspan><tspan dx=\"4\">{m}</tspan></text>",
                ff = escape_text(font_family),
                fs = member_font_size,
                tx = x + 10 + glyph_x_offset,
                vc = effective_color,
                sa = style_attrs,
                m = escape_text(required_text.trim_start())
            ));
        } else {
            if display_text.contains("<$") {
                out.push_str(&creole_text(
                    x + 10 + glyph_x_offset,
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
                    tx = x + 10 + glyph_x_offset,
                    vc = effective_color,
                    sa = style_attrs,
                    m = escape_text(&display_text)
                ));
            }
        }
        my += 16;
    }
}
