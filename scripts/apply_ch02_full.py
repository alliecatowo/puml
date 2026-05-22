#!/usr/bin/env python3
"""Apply all Chapter 2 use case parity patches atomically."""

from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def patch(path: str, old: str, new: str) -> None:
    p = ROOT / path
    text = p.read_text()
    if old not in text:
        raise SystemExit(f"MISS {path}: {old[:60]!r}...")
    p.write_text(text.replace(old, new, 1))


def main() -> None:
    # theme (idempotent if already applied)
    if "ActorStyleKind" not in (ROOT / "src/theme.rs").read_text():
        import subprocess
        import sys

        subprocess.run(
            [sys.executable, str(ROOT / "scripts" / "apply_ch02_patch.py")],
            check=True,
        )

    patch(
        "src/parser/family.rs",
        "fn parse_family_declaration(",
        '''pub(crate) fn parse_page_break_line(line: &str) -> Option<StatementKind> {
    let lower = line.trim().to_ascii_lowercase();
    if lower.starts_with("newpage") {
        return Some(StatementKind::NewPage(
            line[7..].trim().to_string().into(),
        ));
    }
    if lower == "ignore newpage" {
        return Some(StatementKind::IgnoreNewPage);
    }
    None
}

fn parse_colon_actor_usecase_decl(line: &str) -> Option<StatementKind> {
    let trimmed = line.trim();
    if !trimmed.starts_with(':') {
        return None;
    }
    let inner = trimmed.strip_prefix(':')?;
    let close_idx = inner.rfind(':')?;
    let name_raw = inner[..close_idx].trim();
    if name_raw.is_empty() {
        return None;
    }
    let mut rest = inner[close_idx + 1..].trim();
    let mut stereotypes = Vec::new();
    if rest == "/" {
        stereotypes.push("business".to_string());
        rest = "";
    } else if let Some(after_slash) = rest.strip_prefix('/') {
        stereotypes.push("business".to_string());
        rest = after_slash.trim();
    }
    let name = clean_ident(name_raw);
    if name.is_empty() {
        return None;
    }
    let alias = rest
        .strip_prefix("as ")
        .map(str::trim)
        .map(clean_ident)
        .filter(|v| !v.is_empty());
    let members = declaration_marker_members(Some("<<actor>>"), stereotypes);
    Some(StatementKind::UseCaseDecl(UseCaseDecl {
        name,
        alias,
        members,
    }))
}

fn parse_family_declaration(''',
    )

    patch(
        "src/parser/family.rs",
        ") -> Result<Option<(StatementKind, usize)>, Diagnostic> {\n    for (keyword, marker) in [",
        ") -> Result<Option<(StatementKind, usize)>, Diagnostic> {\n    if let Some(kind) = parse_page_break_line(line) {\n        return Ok(Some((kind, start)));\n    }\n    if let Some(kind) = parse_colon_actor_usecase_decl(line) {\n        return Ok(Some((kind, start)));\n    }\n\n    for (keyword, marker) in [",
    )

    patch(
        "src/parser/family.rs",
        """        let mut members = Vec::new();
        append_inline_fill_member(&mut members, fill_color);""",
        """        let mut members = declaration_marker_members(None, stereotypes);
        append_inline_fill_member(&mut members, fill_color);""",
    )

    patch(
        "src/parser/family.rs",
        'for (keyword, marker) in [("actor", Some("<<actor>>")), ("usecase", None)] {',
        """for (keyword, marker, business) in [
        ("usecase/", None, true),
        ("actor/", Some("<<actor>>"), true),
        ("actor", Some("<<actor>>"), false),
        ("usecase", None, false),
    ] {""",
    )

    patch(
        "src/parser/family.rs",
        """        } = decl;
        let mut members = if has_block {
            let mut members = parse_family_decl_members(lines, start, keyword, &name)?;
            if let Some(marker) = marker {
                members.insert(
                    0,
                    ClassMember {
                        text: marker.to_string(),
                        modifier: None,
                    },
                );
            }
            for stereotype in stereotypes.iter().rev() {
                members.insert(
                    0,
                    ClassMember {
                        text: format!("<<{stereotype}>>"),
                        modifier: None,
                    },
                );
            }
            members
        } else {
            declaration_marker_members(marker, stereotypes)
        };
        append_inline_fill_member(&mut members, fill_color);
        return Ok(Some((
            StatementKind::UseCaseDecl(UseCaseDecl {
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
    Ok(None)
}""",
        """        } = decl;
        let mut stereotypes = stereotypes;
        if business {
            stereotypes.push("business".to_string());
        }
        let mut members = if has_block {
            let mut members = parse_family_decl_members(lines, start, keyword, &name)?;
            if let Some(marker) = marker {
                members.insert(
                    0,
                    ClassMember {
                        text: marker.to_string(),
                        modifier: None,
                    },
                );
            }
            for stereotype in stereotypes.iter().rev() {
                members.insert(
                    0,
                    ClassMember {
                        text: format!("<<{stereotype}>>"),
                        modifier: None,
                    },
                );
            }
            members
        } else {
            declaration_marker_members(marker, stereotypes)
        };
        append_inline_fill_member(&mut members, fill_color);
        return Ok(Some((
            StatementKind::UseCaseDecl(UseCaseDecl {
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
    Ok(None)
}""",
    )

    # fix parenthesized - need stereotypes in destructuring first
    patch(
        "src/parser/family.rs",
        """            has_block,
            fill_color,
            ..
        } = decl;
        let mut members = declaration_marker_members(None, stereotypes);""",
        """            has_block,
            stereotypes,
            fill_color,
            ..
        } = decl;
        let mut members = declaration_marker_members(None, stereotypes);""",
    )

    patch(
        "src/parser/family.rs",
        """    let (rest, fill_color) = split_declaration_inline_fill(rest);
    let rest = rest.trim();
    let alias = rest
        .strip_prefix("as ")
        .map(str::trim)
        .map(clean_ident)
        .filter(|v| !v.is_empty());
    Some(FamilyDeclParts {
        name: clean_ident(name_raw),
        alias,
        has_block,
        stereotypes: Vec::new(),""",
        """    let (rest, fill_color) = split_declaration_inline_fill(rest);
    let mut rest = rest.trim();
    let mut stereotypes = Vec::new();
    if rest == "/" {
        stereotypes.push("business".to_string());
        rest = "";
    } else if let Some(after_slash) = rest.strip_prefix('/') {
        stereotypes.push("business".to_string());
        rest = after_slash.trim();
    }
    let alias = rest
        .strip_prefix("as ")
        .map(str::trim)
        .map(clean_ident)
        .filter(|v| !v.is_empty());
    Some(FamilyDeclParts {
        name: clean_ident(name_raw),
        alias,
        has_block,
        stereotypes,""",
    )

    patch(
        "src/parser/sequence.rs",
        """            | StatementKind::Pragma(_)
    )
}

fn is_family_common_keyword_before_detection""",
        """            | StatementKind::Pragma(_)
            | StatementKind::NewPage(_)
            | StatementKind::IgnoreNewPage
    )
}

fn is_family_common_keyword_before_detection""",
    )

    patch(
        "src/normalize/family.rs",
        """                            ClassSkinParamValue::StereotypeFontColor(stereotype, c) => {
                                class_style
                                    .stereotype_styles
                                    .entry(stereotype)
                                    .or_default()
                                    .font_color = Some(c);
                            }
                        }
                    }
                    SkinParamSupport::UnsupportedKey => {
                        // Class diagrams accept generic sequence keys silently""",
        """                            ClassSkinParamValue::StereotypeFontColor(stereotype, c) => {
                                class_style
                                    .stereotype_styles
                                    .entry(stereotype)
                                    .or_default()
                                    .font_color = Some(c);
                            }
                            ClassSkinParamValue::ActorStyle(style) => {
                                class_style.actor_style = style;
                            }
                        }
                    }
                    SkinParamSupport::UnsupportedKey => {
                        // Class diagrams accept generic sequence keys silently""",
    )

    patch(
        "src/normalize/family.rs",
        "    let mut last_relation: Option<(String, String)> = None;\n\n    for stmt in document.statements {",
        "    let mut last_relation: Option<(String, String)> = None;\n    let mut ignore_newpage = false;\n\n    for stmt in document.statements {",
    )

    patch(
        "src/normalize/family.rs",
        """            StatementKind::Theme(value) => {
                class_style = class_style_from_sequence_theme(
                    &resolve_sequence_theme_preset(&value)
                        .map_err(|msg| Diagnostic::error(msg).with_span(stmt.span))?
                        .style,
                );
            }
            StatementKind::Pragma(_)
            | StatementKind::Include(_)
            | StatementKind::Define { .. }
            | StatementKind::Undef(_) => {}
            StatementKind::SaltGridRow { cells } => {""",
        """            StatementKind::Theme(value) => {
                class_style = class_style_from_sequence_theme(
                    &resolve_sequence_theme_preset(&value)
                        .map_err(|msg| Diagnostic::error(msg).with_span(stmt.span))?
                        .style,
                );
            }
            StatementKind::NewPage(title) => {
                if !ignore_newpage {
                    nodes.push(family_newpage_marker_node(title));
                }
            }
            StatementKind::IgnoreNewPage => {
                ignore_newpage = true;
            }
            StatementKind::Pragma(_)
            | StatementKind::Include(_)
            | StatementKind::Define { .. }
            | StatementKind::Undef(_) => {}
            StatementKind::SaltGridRow { cells } => {""",
    )

    newpage_tail = '''

pub(crate) const FAMILY_NEWPAGE_MARKER: &str = "\\x1fpuml:newpage";

fn family_newpage_marker_node(title: Option<String>) -> FamilyNode {
    FamilyNode {
        kind: FamilyNodeKind::Label,
        name: FAMILY_NEWPAGE_MARKER.to_string(),
        alias: title,
        members: Vec::new(),
        depth: 0,
        label: None,
        mindmap_side: MindMapSide::Right,
        wbs_checkbox: None,
        fill_color: None,
    }
}

fn is_family_newpage_marker(node: &FamilyNode) -> bool {
    node.name == FAMILY_NEWPAGE_MARKER
}

pub fn paginate_family(document: &FamilyDocument) -> Vec<FamilyDocument> {
    let marker_indices: Vec<usize> = document
        .nodes
        .iter()
        .enumerate()
        .filter_map(|(idx, node)| is_family_newpage_marker(node).then_some(idx))
        .collect();
    if marker_indices.is_empty() {
        return vec![document.clone()];
    }

    let mut pages = Vec::new();
    let mut segment_start = 0usize;
    let mut pending_title = document.title.clone();
    for marker_idx in marker_indices {
        if marker_idx > segment_start {
            pages.push(family_page_from_segment(
                document,
                segment_start,
                marker_idx,
                pending_title.clone(),
            ));
        }
        pending_title = document.nodes[marker_idx]
            .alias
            .clone()
            .filter(|title| !title.trim().is_empty());
        segment_start = marker_idx + 1;
    }
    if segment_start < document.nodes.len() {
        pages.push(family_page_from_segment(
            document,
            segment_start,
            document.nodes.len(),
            pending_title,
        ));
    }
    if pages.is_empty() {
        pages.push(document.clone());
    }
    pages
}

fn family_page_from_segment(
    source: &FamilyDocument,
    node_start: usize,
    node_end: usize,
    title: Option<String>,
) -> FamilyDocument {
    let nodes: Vec<FamilyNode> = source.nodes[node_start..node_end]
        .iter()
        .filter(|node| !is_family_newpage_marker(node))
        .cloned()
        .collect();
    let node_keys: std::collections::BTreeSet<String> = nodes
        .iter()
        .flat_map(|node| {
            let mut keys = vec![node.name.clone()];
            if let Some(alias) = &node.alias {
                keys.push(alias.clone());
            }
            keys
        })
        .collect();
    let relations: Vec<ModelFamilyRelation> = source
        .relations
        .iter()
        .filter(|rel| node_keys.contains(&rel.from) && node_keys.contains(&rel.to))
        .cloned()
        .collect();
    let groups: Vec<FamilyGroup> = source
        .groups
        .iter()
        .map(|group| FamilyGroup {
            member_ids: group
                .member_ids
                .iter()
                .filter(|id| node_keys.contains(*id))
                .cloned()
                .collect(),
            ..group.clone()
        })
        .filter(|group| !group.member_ids.is_empty())
        .collect();
    FamilyDocument {
        title,
        nodes,
        relations,
        groups,
        ..source.clone()
    }
}
'''
    nf = ROOT / "src/normalize/family.rs"
    if "FAMILY_NEWPAGE_MARKER" not in nf.read_text():
        nf.write_text(nf.read_text() + newpage_tail)

    nm = ROOT / "src/normalize/mod.rs"
    if "paginate_family" not in nm.read_text():
        patch(
            "src/normalize/mod.rs",
            "pub fn paginate(document: &SequenceDocument) -> Vec<SequencePage> {\n    sequence::paginate(document)\n}\n\npub fn normalize_with_options(",
            "pub fn paginate(document: &SequenceDocument) -> Vec<SequencePage> {\n    sequence::paginate(document)\n}\n\npub fn paginate_family(document: &FamilyDocument) -> Vec<FamilyDocument> {\n    family::paginate_family(document)\n}\n\npub fn normalize_with_options(",
        )

    svg = ROOT / "src/render/svg.rs"
    if "render_actor_figure" not in svg.read_text():
        patch(
            "src/render/svg.rs",
            "/// Canonical actor stick-figure renderer",
            "use crate::theme::ActorStyleKind;\n\n/// Canonical actor stick-figure renderer",
        )
        patch(
            "src/render/svg.rs",
            '    ));\n}\n\npub(crate) fn escape_text(input: &str) -> String {',
            '''    ));
}

pub(crate) fn render_actor_hollow_figure(out: &mut String, cx: i32, cy: i32, stroke: &str) {
    let head_cy = cy - 15;
    out.push_str(&format!(
        "<circle cx=\\"{cx}\\" cy=\\"{head_cy}\\" r=\\"6\\" fill=\\"none\\" stroke=\\"{stroke}\\" stroke-width=\\"1\\"/>"
    ));
    let neck_y = head_cy + 6;
    let hip_y = head_cy + 20;
    out.push_str(&format!(
        "<line x1=\\"{cx}\\" y1=\\"{neck_y}\\" x2=\\"{cx}\\" y2=\\"{hip_y}\\" stroke=\\"{stroke}\\" stroke-width=\\"1\\"/>"
    ));
    let arm_y = neck_y + 4;
    out.push_str(&format!(
        "<line x1=\\"{}\\" y1=\\"{arm_y}\\" x2=\\"{}\\" y2=\\"{arm_y}\\" stroke=\\"{stroke}\\" stroke-width=\\"1\\"/>",
        cx - 10, cx + 10
    ));
    let leg_end_y = hip_y + 16;
    out.push_str(&format!(
        "<line x1=\\"{cx}\\" y1=\\"{hip_y}\\" x2=\\"{}\\" y2=\\"{leg_end_y}\\" stroke=\\"{stroke}\\" stroke-width=\\"1\\"/>", cx - 8
    ));
    out.push_str(&format!(
        "<line x1=\\"{cx}\\" y1=\\"{hip_y}\\" x2=\\"{}\\" y2=\\"{leg_end_y}\\" stroke=\\"{stroke}\\" stroke-width=\\"1\\"/>", cx + 8
    ));
}

pub(crate) fn render_actor_awesome_figure(out: &mut String, cx: i32, cy: i32, stroke: &str) {
    let head_cy = cy - 15;
    out.push_str(&format!(
        "<circle cx=\\"{cx}\\" cy=\\"{head_cy}\\" r=\\"7\\" fill=\\"{stroke}\\" stroke=\\"{stroke}\\" stroke-width=\\"1\\"/>"
    ));
    let neck_y = head_cy + 7;
    let hip_y = head_cy + 21;
    out.push_str(&format!(
        "<line x1=\\"{cx}\\" y1=\\"{neck_y}\\" x2=\\"{cx}\\" y2=\\"{hip_y}\\" stroke=\\"{stroke}\\" stroke-width=\\"2.5\\"/>"
    ));
    let arm_y = neck_y + 4;
    out.push_str(&format!(
        "<line x1=\\"{}\\" y1=\\"{arm_y}\\" x2=\\"{}\\" y2=\\"{arm_y}\\" stroke=\\"{stroke}\\" stroke-width=\\"2.5\\"/>", cx - 11, cx + 11
    ));
    let leg_end_y = hip_y + 16;
    out.push_str(&format!(
        "<line x1=\\"{cx}\\" y1=\\"{hip_y}\\" x2=\\"{}\\" y2=\\"{leg_end_y}\\" stroke=\\"{stroke}\\" stroke-width=\\"2.5\\"/>", cx - 9
    ));
    out.push_str(&format!(
        "<line x1=\\"{cx}\\" y1=\\"{hip_y}\\" x2=\\"{}\\" y2=\\"{leg_end_y}\\" stroke=\\"{stroke}\\" stroke-width=\\"2.5\\"/>", cx + 9
    ));
}

pub(crate) fn render_business_actor_figure(out: &mut String, cx: i32, cy: i32, stroke: &str, fill: &str) {
    let head_cy = cy - 14;
    out.push_str(&format!(
        "<circle cx=\\"{cx}\\" cy=\\"{head_cy}\\" r=\\"6\\" fill=\\"{fill}\\" stroke=\\"{stroke}\\" stroke-width=\\"1.5\\"/>"
    ));
    let body_y = head_cy + 8;
    out.push_str(&format!(
        "<rect x=\\"{}\\" y=\\"{body_y}\\" width=\\"24\\" height=\\"26\\" rx=\\"6\\" ry=\\"6\\" fill=\\"{fill}\\" stroke=\\"{stroke}\\" stroke-width=\\"1.5\\"/>", cx - 12
    ));
}

pub(crate) fn render_actor_figure(
    out: &mut String, cx: i32, cy: i32, stroke: &str, fill: &str,
    style: ActorStyleKind, business: bool,
) {
    if business {
        render_business_actor_figure(out, cx, cy, stroke, fill);
        return;
    }
    match style {
        ActorStyleKind::Stick => render_actor_stick_figure(out, cx, cy, stroke),
        ActorStyleKind::Awesome => render_actor_awesome_figure(out, cx, cy, stroke),
        ActorStyleKind::Hollow => render_actor_hollow_figure(out, cx, cy, stroke),
    }
}

pub(crate) fn escape_text(input: &str) -> String {''',
        )

    patch(
        "src/render/family.rs",
        "use super::svg::{creole_text, escape_text, render_actor_stick_figure};",
        "use super::svg::{creole_text, escape_text, render_actor_figure};\nuse crate::theme::ActorStyleKind;",
    )

    patch(
        "src/render/family.rs",
        """fn first_user_stereotype_key(node: &crate::model::FamilyNode) -> Option<String> {
    node.members.iter().find_map(|member| {
        let text = member.text.trim();
        is_user_stereotype(text).then(|| {
            text.trim_start_matches("<<")
                .trim_end_matches(">>")
                .trim()
                .to_ascii_lowercase()
        })
    })
}

fn render_class_node(""",
        """fn first_user_stereotype_key(node: &crate::model::FamilyNode) -> Option<String> {
    node.members.iter().find_map(|member| {
        let text = member.text.trim();
        is_user_stereotype(text).then(|| {
            text.trim_start_matches("<<")
                .trim_end_matches(">>")
                .trim()
                .to_ascii_lowercase()
        })
    })
}

fn family_node_has_stereotype(node: &crate::model::FamilyNode, stereotype: &str) -> bool {
    let needle = stereotype.to_ascii_lowercase();
    node.members.iter().any(|member| {
        let text = member.text.trim();
        text.starts_with("<<")
            && text.ends_with(">>")
            && text[2..text.len() - 2].trim().eq_ignore_ascii_case(&needle)
    })
}

fn render_class_node(""",
    )

    patch(
        "src/render/family.rs",
        """    if matches!(node.kind, FamilyNodeKind::Actor) {
        // Canonical stick-figure rendering for actors (issue #715).
        // Proportions are shared with the sequence renderer via render_actor_stick_figure.
        // The figure centre cy is placed at y + 21 so the head top sits at y + 0.
        let cx = x + w / 2;
        let fig_cy = y + 21; // centre of figure; head top = fig_cy - 21
        render_actor_stick_figure(out, cx, fig_cy, stroke);""",
        """    if matches!(node.kind, FamilyNodeKind::Actor) {
        let cx = x + w / 2;
        let fig_cy = y + 21;
        let business = family_node_has_stereotype(node, "business");
        let style_attr = match class_style.actor_style {
            ActorStyleKind::Stick => "stick",
            ActorStyleKind::Awesome => "awesome",
            ActorStyleKind::Hollow => "hollow",
        };
        out.push_str(&format!(
            "<g data-actor-style=\\"{style_attr}\\" data-actor-business=\\"{business}\\">"
        ));
        render_actor_figure(out, cx, fig_cy, stroke, fill, class_style.actor_style, business);
        out.push_str("</g>");""",
    )

    patch(
        "src/render/family.rs",
        """    if matches!(node.kind, FamilyNodeKind::UseCase) {
        // Ellipse rendering for use cases
        let cx = x + w / 2;
        let cy = y + h / 2;
        let rx = w / 2;
        let ry = h / 2;
        out.push_str(&format!(
            "<ellipse cx=\\"{cx}\\" cy=\\"{cy}\\" rx=\\"{rx}\\" ry=\\"{ry}\\" fill=\\"{fill}\\" stroke=\\"{stroke}\\" stroke-width=\\"1.5\\"/>",
        ));""",
        """    if matches!(node.kind, FamilyNodeKind::UseCase) {
        let cx = x + w / 2;
        let cy = y + h / 2;
        let business = family_node_has_stereotype(node, "business");
        if business {
            let corner = (w.min(h) / 4).max(8);
            out.push_str(&format!(
                "<rect x=\\"{x}\\" y=\\"{y}\\" width=\\"{w}\\" height=\\"{h}\\" rx=\\"{corner}\\" ry=\\"{corner}\\" fill=\\"{fill}\\" stroke=\\"{stroke}\\" stroke-width=\\"1.5\\" data-usecase-kind=\\"business\\"/>",
            ));
        } else {
            let rx = w / 2;
            let ry = h / 2;
            out.push_str(&format!(
                "<ellipse cx=\\"{cx}\\" cy=\\"{cy}\\" rx=\\"{rx}\\" ry=\\"{ry}\\" fill=\\"{fill}\\" stroke=\\"{stroke}\\" stroke-width=\\"1.5\\"/>",
            ));
        }""",
    )

    patch(
        "src/lib.rs",
        "Ok(vec![render_family_document_svg(&family_doc)])",
        "Ok(normalize::paginate_family(&family_doc)\n                .into_iter()\n                .map(|page| render_family_document_svg(&page))\n                .collect())",
    )

    patch(
        "src/lib.rs",
        "vec![render_family_document_svg(family)]",
        "normalize::paginate_family(family)\n            .into_iter()\n            .map(|page| render_family_document_svg(&page))\n            .collect()",
    )

    print("ch02 full patch OK")


if __name__ == "__main__":
    main()
