#!/usr/bin/env python3
"""Apply Chapter 4 object-diagram parity edits (idempotent)."""
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def patch(path: str, old: str, new: str) -> None:
    p = ROOT / path
    text = p.read_text()
    if old not in text:
        if new.split("\n", 1)[0] in text:
            return
        print(f"skip patch in {path}: {old[:50]!r}...")
        return
    p.write_text(text.replace(old, new, 1))


def main() -> None:
    patch(
        "src/model.rs",
        "    Class,\n    Object,\n    UseCase,",
        "    Class,\n    Object,\n    /// Object-diagram associative array (`map Name { key => value }`).\n    Map,\n    /// Object-diagram n-ary association hub (`diamond NAME`).\n    Diamond,\n    UseCase,",
    )
    patch(
        "src/parser/core.rs",
        "            if let Some((kind, end_idx)) = parse_family_declaration(&lines, i, line)? {",
        "            if let Some((kind, end_idx)) =\n                parse_family_declaration(&lines, i, line, detected_kind)?\n            {",
    )
    patch(
        "src/normalize/family.rs",
        """                let (clean_alias, c4_kind) = sequence::extract_c4_stereotype(decl.alias);
                let resolved_kind = c4_kind.unwrap_or(FamilyNodeKind::Object);
                let mut members = decl.members;""",
        """                let (clean_alias, c4_kind) = sequence::extract_c4_stereotype(decl.alias);
                let mut members = decl.members;
                let resolved_kind = if members
                    .first()
                    .is_some_and(|m| m.text == "\\x1fkind:map" || m.text == "<<map>>")
                {
                    let _ = members.remove(0);
                    FamilyNodeKind::Map
                } else if members.first().is_some_and(|m| m.text == "\\x1fkind:diamond") {
                    let _ = members.remove(0);
                    FamilyNodeKind::Diamond
                } else {
                    c4_kind.unwrap_or(FamilyNodeKind::Object)
                };""",
    )

    decl_old = """fn parse_family_declaration(
    lines: &[(&str, Span)],
    start: usize,
    line: &str,
) -> Result<Option<(StatementKind, usize)>, Diagnostic> {
    for (keyword, marker) in [
        ("abstract class", Some("<<abstract class>>")),
        ("exception", Some("<<exception>>")),
        ("metaclass", Some("<<metaclass>>")),
        ("stereotype", Some("<<stereotype>>")),
        ("interface", Some("<<interface>>")),
        ("enum", Some("<<enum>>")),
        ("annotation", Some("<<annotation>>")),
        ("protocol", Some("<<protocol>>")),
        ("struct", Some("<<struct>>")),
        ("circle", Some("<<circle>>")),
        ("diamond", Some("<<diamond>>")),
        ("abstract", Some("<<abstract>>")),
        ("class", None),
    ] {"""

    decl_new = """fn parse_family_declaration(
    lines: &[(&str, Span)],
    start: usize,
    line: &str,
    active_family: Option<DiagramKind>,
) -> Result<Option<(StatementKind, usize)>, Diagnostic> {
    if !matches!(active_family, Some(DiagramKind::Class)) {
        if let Some(decl) = parse_named_family_decl(line, "diamond") {
            let FamilyDeclParts {
                name,
                alias,
                has_block,
                stereotypes,
                fill_color,
                ..
            } = decl;
            let mut members = if has_block {
                parse_family_decl_members(lines, start, "diamond", &name)?
            } else {
                Vec::new()
            };
            members.insert(
                0,
                ClassMember {
                    text: "\\x1fkind:diamond".to_string(),
                    modifier: None,
                },
            );
            for stereotype in stereotypes.iter().rev() {
                members.insert(
                    0,
                    ClassMember {
                        text: format!("<<{stereotype}>>"),
                        modifier: None,
                    },
                );
            }
            append_inline_fill_member(&mut members, fill_color);
            return Ok(Some((
                StatementKind::ObjectDecl(ObjectDecl {
                    name,
                    alias,
                    members,
                }),
                if has_block {
                    find_family_decl_end(lines, start)
                } else {
                    start
                },
            )));
        }
    }

    let mut class_keywords: Vec<(&str, Option<&str>)> = vec![
        ("abstract class", Some("<<abstract class>>")),
        ("exception", Some("<<exception>>")),
        ("metaclass", Some("<<metaclass>>")),
        ("stereotype", Some("<<stereotype>>")),
        ("interface", Some("<<interface>>")),
        ("enum", Some("<<enum>>")),
        ("annotation", Some("<<annotation>>")),
        ("protocol", Some("<<protocol>>")),
        ("struct", Some("<<struct>>")),
        ("circle", Some("<<circle>>")),
        ("abstract", Some("<<abstract>>")),
        ("class", None),
    ];
    if matches!(active_family, Some(DiagramKind::Class)) {
        class_keywords.insert(10, ("diamond", Some("<<diamond>>")));
    }
    for (keyword, marker) in class_keywords {"""

    patch("src/parser/family.rs", decl_old, decl_new)
    patch(
        "src/parser/family.rs",
        '    for (keyword, marker) in [("map", Some("<<map>>")), ("object", None)] {',
        '    for (keyword, marker) in [("map", Some("\\x1fkind:map")), ("object", None)] {',
    )
    patch(
        "src/parser/family.rs",
        """    let mut members = Vec::new();
    for (raw, _) in lines.iter().take(end_idx).skip(start + 1) {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            members.push(parse_class_member(trimmed));
        }
    }
    Ok(members)
}

/// Parse a single member line, extracting any `{field}`, `{method}`, `{abstract}`,""",
        """    let mut members = Vec::new();
    for (raw, _) in lines.iter().take(end_idx).skip(start + 1) {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            let member = if keyword == "map" {
                parse_map_member(trimmed)
            } else {
                parse_class_member(trimmed)
            };
            members.push(member);
        }
    }
    Ok(members)
}

fn parse_map_member(raw: &str) -> ClassMember {
    if let Some((key, value)) = split_map_arrow_row(raw) {
        return ClassMember {
            text: format!("\\x1fmap:{key}\\x1f{value}"),
            modifier: None,
        };
    }
    parse_class_member(raw)
}

fn split_map_arrow_row(raw: &str) -> Option<(String, String)> {
    let mut in_quote = false;
    let mut arrow_idx = None;
    for (idx, ch) in raw.char_indices() {
        if ch == '"' {
            in_quote = !in_quote;
            continue;
        }
        if in_quote {
            continue;
        }
        if raw[idx..].starts_with("=>") {
            arrow_idx = Some(idx);
            break;
        }
    }
    let idx = arrow_idx?;
    let key = raw[..idx].trim();
    let value = raw[idx + 2..].trim();
    if key.is_empty() || value.is_empty() {
        return None;
    }
    Some((key.to_string(), value.to_string()))
}

/// Parse a single member line, extracting any `{field}`, `{method}`, `{abstract}`,""",
    )

    render_helpers = '''
        FamilyNodeKind::C4Boundary => "boundary",
    }
}

const MAP_ROW_HEIGHT: i32 = 18;
const MAP_COL_PAD: i32 = 10;

fn map_entry_parts(text: &str) -> Option<(&str, &str)> {
    let rest = text.strip_prefix("\\x1fmap:")?;
    rest.split_once('\\x1f')
}

fn count_map_display_rows(members: &[crate::ast::ClassMember]) -> usize {
    members
        .iter()
        .filter(|m| map_entry_parts(&m.text).is_some())
        .count()
}

fn map_row_index(members: &[crate::ast::ClassMember], row_key: &str) -> Option<usize> {
    members.iter().position(|m| {
        map_entry_parts(&m.text)
            .is_some_and(|(key, _)| key == row_key || key.eq_ignore_ascii_case(row_key))
    })
}

fn map_row_center_y(box_y: i32, header_h: i32, row_idx: usize) -> i32 {
    box_y + header_h + 16 + (row_idx as i32) * MAP_ROW_HEIGHT + MAP_ROW_HEIGHT / 2
}

fn find_family_node<'a>(
    nodes: &'a [crate::model::FamilyNode],
    key: &str,
) -> Option<&'a crate::model::FamilyNode> {
    nodes
        .iter()
        .find(|n| n.alias.as_deref() == Some(key) || n.name == key)
}

fn apply_map_qualified_anchor(
    full_endpoint: &str,
    nodes: &[crate::model::FamilyNode],
    node_box: ClassNodeBox,
    anchor_x: i32,
    anchor_y: i32,
) -> (i32, i32) {
    let Some((owner, row_key)) = full_endpoint.rsplit_once("::") else {
        return (anchor_x, anchor_y);
    };
    let Some(node) = find_family_node(nodes, owner) else {
        return (anchor_x, anchor_y);
    };
    if node.kind != FamilyNodeKind::Map {
        return (anchor_x, anchor_y);
    }
    let Some(row_idx) = map_row_index(&node.members, row_key) else {
        return (anchor_x, anchor_y);
    };
    (
        anchor_x,
        map_row_center_y(node_box.y, node_box.header_h, row_idx),
    )
}

struct ClassNodeGeometry {'''

    patch(
        "src/render/family.rs",
        """        FamilyNodeKind::C4Boundary => "boundary",
    }
}

struct ClassNodeGeometry {""",
        render_helpers,
    )
    patch(
        "src/render/family.rs",
        """        FamilyNodeKind::C4Boundary => "boundary",
    }
}

const MAP_ROW_HEIGHT""",
        """        FamilyNodeKind::C4Boundary => "boundary",
        FamilyNodeKind::Map => "map",
        FamilyNodeKind::Diamond => "diamond",
    }
}

const MAP_ROW_HEIGHT""",
    )

    # fix duplicate if helpers inserted without labels
    patch(
        "src/render/family.rs",
        """        FamilyNodeKind::C4Boundary => "boundary",
    }
}

const MAP_ROW_HEIGHT: i32 = 18;""",
        """        FamilyNodeKind::C4Boundary => "boundary",
        FamilyNodeKind::Map => "map",
        FamilyNodeKind::Diamond => "diamond",
    }
}

const MAP_ROW_HEIGHT: i32 = 18;""",
    )

    patch(
        "src/render/family.rs",
        """        let (x1, y1, x2, y2) = if relation.direction.is_some() {
            compute_edge_anchors_for_direction(
                (from.x, from.y, from.w, from.h),
                (to.x, to.y, to.w, to.h),
                relation.direction.as_deref(),
            )
        } else {
            pick_port((from.x, from.y, from.w, from.h), (to.x, to.y, to.w, to.h))
        };

        let lat_offset = ctx.parallel_offset.get(&rel_idx).copied().unwrap_or(0);""",
        """        let (mut x1, mut y1, mut x2, mut y2) = if relation.direction.is_some() {
            compute_edge_anchors_for_direction(
                (from.x, from.y, from.w, from.h),
                (to.x, to.y, to.w, to.h),
                relation.direction.as_deref(),
            )
        } else {
            pick_port((from.x, from.y, from.w, from.h), (to.x, to.y, to.w, to.h))
        };
        (x1, y1) = apply_map_qualified_anchor(&from_name, ctx.nodes, *from, x1, y1);
        (x2, y2) = apply_map_qualified_anchor(&to_name, ctx.nodes, *to, x2, y2);

        let lat_offset = ctx.parallel_offset.get(&rel_idx).copied().unwrap_or(0);""",
    )
    patch(
        "src/render/family.rs",
        """        let member_px = document
            .nodes
            .iter()
            .flat_map(|n| n.members.iter())
            .map(|m| m.text.chars().count() as i32 * 7 + 24)
            .max()
            .unwrap_or(0);
        name_px.max(member_px).clamp(160, 600)""",
        """        let member_px = document
            .nodes
            .iter()
            .map(|n| {
                if n.kind == FamilyNodeKind::Map {
                    n.members
                        .iter()
                        .filter_map(|m| map_entry_parts(&m.text))
                        .map(|(key, value)| {
                            (key.chars().count() + value.chars().count()) as i32 * 7 + 48
                        })
                        .max()
                        .unwrap_or(0)
                } else {
                    n.members
                        .iter()
                        .map(|m| m.text.chars().count() as i32 * 7 + 24)
                        .max()
                        .unwrap_or(0)
                }
            })
            .max()
            .unwrap_or(0);
        name_px.max(member_px).clamp(160, 600)""",
    )
    patch(
        "src/render/family.rs",
        """            } else if display_member_count == 0 {
                empty_member_pad
            } else {
                (display_member_count as i32) * member_line_height + 2 * member_padding
            };
            let h = c4_node_height(node.kind, header_height + stereotype_extra_h + body_h);""",
        """            } else if node.kind == FamilyNodeKind::Map {
                let rows = count_map_display_rows(&node.members);
                if rows == 0 {
                    empty_member_pad
                } else {
                    (rows as i32) * MAP_ROW_HEIGHT + 2 * member_padding
                }
            } else if node.kind == FamilyNodeKind::Diamond {
                48
            } else if display_member_count == 0 {
                empty_member_pad
            } else {
                (display_member_count as i32) * member_line_height + 2 * member_padding
            };
            let h = c4_node_height(node.kind, header_height + stereotype_extra_h + body_h);""",
    )
    patch(
        "src/render/family.rs",
        """    if node.kind == FamilyNodeKind::Note {
        render_note_card(out, x, y, w, h, node.label.as_deref().unwrap_or(&node.name));
        return;
    }

    let scoped_style =""",
        """    if node.kind == FamilyNodeKind::Note {
        render_note_card(out, x, y, w, h, node.label.as_deref().unwrap_or(&node.name));
        return;
    }

    if node.kind == FamilyNodeKind::Diamond {
        render_diamond_hub_node(out, node, x, y, w, h, class_style);
        return;
    }

    if node.kind == FamilyNodeKind::Map {
        render_map_table_node(out, node, geometry, class_style, namespace_separator);
        return;
    }

    let scoped_style =""",
    )
    patch(
        "src/render/family.rs",
        """        FamilyNodeKind::Object => "#fef3c7",
        FamilyNodeKind::UseCase => "#dcfce7",
        _ => "#f1f5f9",""",
        """        FamilyNodeKind::Object => "#fef3c7",
        FamilyNodeKind::Map => "#fef3c7",
        FamilyNodeKind::UseCase => "#dcfce7",
        FamilyNodeKind::Diamond => "#f8fafc",
        _ => "#f1f5f9",""",
    )
    patch(
        "src/render/family.rs",
        """    let text_decoration = if matches!(node.kind, FamilyNodeKind::Object) {
        " text-decoration=\"underline\" text-decoration-thickness=\"1\"""",
        """    let text_decoration = if matches!(node.kind, FamilyNodeKind::Object | FamilyNodeKind::Map) {
        " text-decoration=\"underline\" text-decoration-thickness=\"1\"""",
    )
    patch(
        "src/render/family.rs",
        """        FamilyNodeKind::Actor | FamilyNodeKind::Person => computed.max(64),
        _ => computed,
    }
}

/// Returns true if the kind belongs to the C4 family.
fn is_c4_kind(kind: FamilyNodeKind) -> bool {""",
        """        FamilyNodeKind::Actor | FamilyNodeKind::Person => computed.max(64),
        FamilyNodeKind::Diamond => computed.max(56),
        FamilyNodeKind::Map => computed.max(48),
        _ => computed,
    }
}

fn render_diamond_hub_node(
    out: &mut String,
    node: &crate::model::FamilyNode,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    class_style: &ClassStyle,
) {
    let cx = x + w / 2;
    let cy = y + h / 2;
    let half_w = (w / 2).saturating_sub(4).max(16);
    let half_h = (h / 2).saturating_sub(4).max(16);
    let stroke = &class_style.border_color;
    let fill = class_style.background_color.as_str();
    out.push_str(&format!(
        "<polygon class=\\"uml-diamond-hub\\" data-uml-kind=\\"diamond\\" points=\\"{cx},{top} {right},{cy} {cx},{bottom} {left},{cy}\\" fill=\\"{fill}\\" stroke=\\"{stroke}\\" stroke-width=\\"1.5\\"/>",
        top = cy - half_h,
        bottom = cy + half_h,
        right = cx + half_w,
        left = cx - half_w,
        fill = escape_text(fill),
        stroke = escape_text(stroke),
    ));
    let label = node.alias.as_deref().unwrap_or(&node.name);
    out.push_str(&format!(
        "<text x=\"{cx}\" y=\"{ty}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" font-weight=\"600\" fill=\"{fc}\">{lbl}</text>",
        ty = cy + 4,
        fc = escape_text(&class_style.font_color),
        lbl = escape_text(label),
    ));
}

fn render_map_table_node(
    out: &mut String,
    node: &crate::model::FamilyNode,
    geometry: ClassNodeGeometry,
    class_style: &ClassStyle,
    namespace_separator: Option<&str>,
) {
    let ClassNodeGeometry { x, y, w, h, header_h } = geometry;
    let stroke = &class_style.border_color;
    let fill = class_style.background_color.as_str();
    let header_fill = "#fef3c7";
    let font_family = class_style.font_name.as_deref().unwrap_or("monospace");
    let title_font_size = class_style.font_size.unwrap_or(13);
    let member_font_size = title_font_size.saturating_sub(2).max(9);
    let font_color = &class_style.font_color;
    let member_color = class_style.member_color.as_str();

    out.push_str(&format!(
        "<rect class=\"uml-map\" data-uml-kind=\"map\" x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" rx=\"4\" ry=\"4\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>",
        fill = escape_text(fill),
        stroke = escape_text(stroke),
    ));
    out.push_str(&format!(
        "<rect x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{hh}\" rx=\"4\" ry=\"4\" fill=\"{header_fill}\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>",
        hh = header_h,
        header_fill = escape_text(header_fill),
        stroke = escape_text(stroke),
    ));
    out.push_str(&format!(
        "<line x1=\"{x}\" y1=\"{ly}\" x2=\"{x2}\" y2=\"{ly}\" stroke=\"{stroke}\" stroke-width=\"1\"/>",
        ly = y + header_h,
        x2 = x + w,
        stroke = escape_text(stroke),
    ));

    let display_name = namespace_separator
        .filter(|sep| !sep.is_empty())
        .map(|sep| node.name.replace("::", sep))
        .unwrap_or_else(|| node.name.clone());
    out.push_str(&format!(
        "<text x=\"{tx}\" y=\"{ty}\" text-anchor=\"middle\" font-family=\"{ff}\" font-size=\"{fs}\" font-weight=\"600\" fill=\"{fc}\" text-decoration=\"underline\">{txt}</text>",
        tx = x + w / 2,
        ty = y + header_h - 9,
        ff = escape_text(font_family),
        fs = title_font_size,
        fc = escape_text(font_color),
        txt = escape_text(&display_name),
    ));

    let sep_x = x + (w / 2).max(60);
    out.push_str(&format!(
        "<line x1=\"{sep_x}\" y1=\"{y1}\" x2=\"{sep_x}\" y2=\"{y2}\" stroke=\"{stroke}\" stroke-width=\"1\"/>",
        y1 = y + header_h,
        y2 = y + h,
        stroke = escape_text(stroke),
    ));

    let mut my = y + header_h + 16;
    for member in &node.members {
        let Some((key, value)) = map_entry_parts(&member.text) else {
            continue;
        };
        out.push_str(&format!(
            "<text class=\"uml-map-key\" data-uml-map-row=\"{row}\" x=\"{kx}\" y=\"{my}\" text-anchor=\"start\" font-family=\"{ff}\" font-size=\"{fs}\" fill=\"{mc}\">{key_txt}</text>",
            row = escape_text(key),
            kx = x + MAP_COL_PAD,
            ff = escape_text(font_family),
            fs = member_font_size,
            mc = escape_text(member_color),
            key_txt = escape_text(key),
        ));
        let value_text = escape_text(value);
        out.push_str(&format!(
            "<text class=\"uml-map-value\" data-uml-map-row=\"{row}\" x=\"{vx}\" y=\"{my}\" text-anchor=\"start\" font-family=\"{ff}\" font-size=\"{fs}\" fill=\"{mc}\">{value_text}</text>",
            row = escape_text(key),
            vx = sep_x + MAP_COL_PAD,
            my = my,
            ff = escape_text(font_family),
            fs = member_font_size,
            mc = escape_text(member_color),
            value_text = value_text,
        ));
        my += MAP_ROW_HEIGHT;
    }
}

/// Returns true if the kind belongs to the C4 family.
fn is_c4_kind(kind: FamilyNodeKind) -> bool {""",
    )

    print("ch04 patch applied")


if __name__ == "__main__":
    main()
