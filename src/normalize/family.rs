use super::*;

mod extended;
mod tree;

pub(super) use self::extended::normalize_extended_family;
pub(super) use self::tree::normalize_family_tree;

pub(super) fn normalize_stub_family(document: Document) -> Result<FamilyDocument, Diagnostic> {
    let family_kind = document.kind;
    let node_kind = match family_kind {
        DiagramKind::Class => FamilyNodeKind::Class,
        DiagramKind::Object => FamilyNodeKind::Object,
        DiagramKind::UseCase => FamilyNodeKind::UseCase,
        DiagramKind::Salt => FamilyNodeKind::Salt,
        _ => {
            return Err(Diagnostic::error(
                "[E_FAMILY_STUB_INTERNAL] invalid family for stub normalization",
            ));
        }
    };

    let mut nodes = Vec::new();
    let mut relations = Vec::new();
    let mut groups = Vec::new();
    let mut json_projections: Vec<crate::model::JsonProjection> = Vec::new();
    let mut hide_options = std::collections::BTreeSet::new();
    let mut namespace_separator: Option<String> = None;
    let mut title = None;
    let mut header = None;
    let mut footer = None;
    let mut caption = None;
    let mut legend = None;
    let mut class_style = ClassStyle::default();
    let mut warnings: Vec<Diagnostic> = Vec::new();
    let mut note_counter: usize = 0;

    for stmt in document.statements {
        match stmt.kind {
            StatementKind::SkinParam { key, value } => {
                if family_kind == DiagramKind::Salt {
                    nodes.push(FamilyNode {
                        kind: FamilyNodeKind::Salt,
                        name: format!("SALT_ROW\x1fL:saltstyle {key} {value}"),
                        alias: None,
                        members: Vec::new(),
                        depth: 0,
                        label: None,
                        mindmap_side: MindMapSide::Right,
                        wbs_checkbox: None,
                        fill_color: None,
                    });
                    continue;
                }
                match classify_class_skinparam(&key, &value) {
                    SkinParamSupport::SupportedNoop => {}
                    SkinParamSupport::SupportedWithValue(v) => {
                        use crate::theme::ClassSkinParamValue;
                        match v {
                            ClassSkinParamValue::BackgroundColor(c) => {
                                class_style.background_color = c;
                            }
                            ClassSkinParamValue::BorderColor(c) => {
                                class_style.border_color = c;
                            }
                            ClassSkinParamValue::HeaderBackgroundColor(c) => {
                                class_style.header_color = c;
                            }
                            ClassSkinParamValue::MemberFontColor(c) => {
                                class_style.member_color = c;
                            }
                            ClassSkinParamValue::FontColor(c) => {
                                class_style.font_color = c;
                            }
                            ClassSkinParamValue::ArrowColor(c) => {
                                class_style.arrow_color = c;
                            }
                            ClassSkinParamValue::FontSize(n) => {
                                class_style.font_size = Some(n);
                            }
                            ClassSkinParamValue::FontName(n) => {
                                class_style.font_name = Some(n);
                            }
                        }
                    }
                    SkinParamSupport::UnsupportedKey => {
                        // Class diagrams accept generic sequence keys silently
                        // (PlantUML applies them across all families).
                        use crate::theme::{classify_sequence_skinparam, SequenceSkinParamSupport};
                        if !matches!(
                            classify_sequence_skinparam(&key, &value),
                            SequenceSkinParamSupport::UnsupportedKey
                        ) {
                            // Recognized sequence key — no warning.
                        } else {
                            warnings.push(
                                Diagnostic::warning(format!(
                                    "[W_SKINPARAM_UNSUPPORTED] unsupported skinparam `{}`",
                                    key
                                ))
                                .with_span(stmt.span),
                            );
                        }
                    }
                    SkinParamSupport::UnsupportedValue => {
                        warnings.push(
                            Diagnostic::warning(format!(
                                "[W_SKINPARAM_UNSUPPORTED_VALUE] unsupported value `{}` for skinparam `{}`",
                                value, key
                            ))
                            .with_span(stmt.span),
                        );
                    }
                }
            }
            StatementKind::JsonProjection { alias, body } => {
                json_projections.push(crate::model::JsonProjection {
                    alias,
                    body,
                    format: "json".to_string(),
                });
            }
            StatementKind::YamlProjection { alias, body } => {
                json_projections.push(crate::model::JsonProjection {
                    alias,
                    body,
                    format: "yaml".to_string(),
                });
            }
            StatementKind::ClassDecl(decl) => {
                if node_kind != FamilyNodeKind::Class {
                    return Err(Diagnostic::error(format!(
                        "[E_FAMILY_MIXED] mixed diagram families are not supported: found class declaration in {} diagram",
                        family_kind_name(family_kind)
                    ))
                    .with_span(stmt.span));
                }
                let mut members = decl.members;
                let fill_color = extract_family_node_fill_color(&mut members);
                upsert_family_node(
                    &mut nodes,
                    FamilyNode {
                        kind: FamilyNodeKind::Class,
                        name: decl.name,
                        alias: decl.alias,
                        members,
                        depth: 0,
                        label: None,
                        mindmap_side: MindMapSide::Right,
                        wbs_checkbox: None,
                        fill_color,
                    },
                );
            }
            StatementKind::ObjectDecl(decl) => {
                if node_kind != FamilyNodeKind::Object {
                    return Err(Diagnostic::error(format!(
                        "[E_FAMILY_MIXED] mixed diagram families are not supported: found object declaration in {} diagram",
                        family_kind_name(family_kind)
                    ))
                    .with_span(stmt.span));
                }
                // Detect and strip C4 stereotypes embedded in the alias
                // (e.g. `u <<person>>` → alias `u`, kind `C4Person`).
                let (clean_alias, c4_kind) = sequence::extract_c4_stereotype(decl.alias);
                let resolved_kind = c4_kind.unwrap_or(FamilyNodeKind::Object);
                let mut members = decl.members;
                let fill_color = extract_family_node_fill_color(&mut members);
                upsert_family_node(
                    &mut nodes,
                    FamilyNode {
                        kind: resolved_kind,
                        name: decl.name,
                        alias: clean_alias,
                        members,
                        depth: 0,
                        label: None,
                        mindmap_side: MindMapSide::Right,
                        wbs_checkbox: None,
                        fill_color,
                    },
                );
            }
            StatementKind::UseCaseDecl(decl) => {
                if node_kind != FamilyNodeKind::UseCase {
                    return Err(Diagnostic::error(format!(
                        "[E_FAMILY_MIXED] mixed diagram families are not supported: found usecase declaration in {} diagram",
                        family_kind_name(family_kind)
                    ))
                    .with_span(stmt.span));
                }
                let mut members = decl.members;
                let fill_color = extract_family_node_fill_color(&mut members);
                // An `actor` declaration becomes a UseCaseDecl with `<<actor>>` as its
                // first member. Detect this and promote it to Actor kind so the renderer
                // can draw a stick figure instead of an ellipse.
                let resolved_kind = if members
                    .first()
                    .is_some_and(|m| m.text.trim() == "<<actor>>")
                {
                    let _ = members.remove(0); // strip the marker — it was only for detection
                    FamilyNodeKind::Actor
                } else {
                    FamilyNodeKind::UseCase
                };
                upsert_family_node(
                    &mut nodes,
                    FamilyNode {
                        kind: resolved_kind,
                        name: decl.name,
                        alias: decl.alias,
                        members,
                        depth: 0,
                        label: None,
                        mindmap_side: MindMapSide::Right,
                        wbs_checkbox: None,
                        fill_color,
                    },
                );
            }
            StatementKind::FamilyRelation(rel) => relations.push(ModelFamilyRelation {
                from: rel.from,
                to: rel.to,
                arrow: rel.arrow,
                label: rel.label,
                stereotype: rel.stereotype,
                left_cardinality: rel.left_cardinality,
                right_cardinality: rel.right_cardinality,
                left_role: rel.left_role,
                right_role: rel.right_role,
                line_color: rel.line_color,
                dashed: rel.dashed,
                hidden: rel.hidden,
                thickness: rel.thickness,
                direction: rel.direction,
                left_lollipop: rel.left_lollipop,
                right_lollipop: rel.right_lollipop,
            }),
            StatementKind::Note(note) => {
                note_counter += 1;
                nodes.push(family_note_node(note_counter, note));
            }
            StatementKind::ClassGroup {
                kind,
                label,
                members,
                relations: group_relations,
            } => {
                // Auto-create nodes for members declared inside a package/namespace block
                // if they haven't already been declared as top-level statements.
                let mut group_member_ids = Vec::with_capacity(members.len());
                for member_id in &members {
                    let mut parts = member_id.split('\t');
                    let node_id = parts.next().unwrap_or(member_id.as_str()).to_string();
                    let mut encoded_members = parts
                        .map(|text| ClassMember {
                            text: text.to_string(),
                            modifier: None,
                        })
                        .filter(|member| {
                            let trimmed = member.text.trim();
                            if hide_options.contains("stereotype")
                                && trimmed.starts_with("<<")
                                && trimmed.ends_with(">>")
                            {
                                return false;
                            }
                            if hide_options.contains("circle") && trimmed == "()" {
                                return false;
                            }
                            if (hide_options.contains("empty members")
                                || hide_options.contains("empty methods")
                                || hide_options.contains("empty fields"))
                                && (trimmed.is_empty() || trimmed == "--" || trimmed == "..")
                            {
                                return false;
                            }
                            true
                        })
                        .collect::<Vec<_>>();
                    let fill_color = extract_family_node_fill_color(&mut encoded_members);
                    group_member_ids.push(node_id.clone());
                    let already_exists = nodes.iter().any(|n: &FamilyNode| {
                        n.name == node_id || n.alias.as_deref() == Some(node_id.as_str())
                    });
                    if !already_exists {
                        // Detect actor marker embedded by the parser for `actor`
                        // declarations inside usecase scoping blocks (e.g.
                        // `rectangle "System" { actor User }`).
                        let has_actor_marker =
                            encoded_members.iter().any(|m| m.text.trim() == "<<actor>>");
                        let nk = if has_actor_marker {
                            FamilyNodeKind::Actor
                        } else {
                            match node_kind {
                                FamilyNodeKind::Object => FamilyNodeKind::Object,
                                FamilyNodeKind::UseCase => FamilyNodeKind::UseCase,
                                _ => FamilyNodeKind::Class,
                            }
                        };
                        // Strip the actor marker from the members list — it was
                        // only needed for kind detection.
                        encoded_members.retain(|m| m.text.trim() != "<<actor>>");
                        nodes.push(FamilyNode {
                            kind: nk,
                            name: node_id,
                            alias: None,
                            members: encoded_members,
                            depth: 0,
                            label: None,
                            mindmap_side: MindMapSide::Right,
                            wbs_checkbox: None,
                            fill_color,
                        });
                    }
                }
                groups.push(FamilyGroup {
                    kind,
                    label,
                    member_ids: group_member_ids,
                });
                for rel in group_relations {
                    relations.push(ModelFamilyRelation {
                        from: rel.from,
                        to: rel.to,
                        arrow: rel.arrow,
                        label: rel.label,
                        stereotype: rel.stereotype,
                        left_cardinality: rel.left_cardinality,
                        right_cardinality: rel.right_cardinality,
                        left_role: rel.left_role,
                        right_role: rel.right_role,
                        line_color: rel.line_color,
                        dashed: rel.dashed,
                        hidden: rel.hidden,
                        thickness: rel.thickness,
                        direction: rel.direction,
                        left_lollipop: rel.left_lollipop,
                        right_lollipop: rel.right_lollipop,
                    });
                }
            }
            StatementKind::SetOption { key, value } => {
                if key.eq_ignore_ascii_case("namespaceSeparator") {
                    namespace_separator = Some(value);
                }
            }
            StatementKind::HideOption(opt) => {
                hide_options.insert(opt.to_ascii_lowercase());
            }
            StatementKind::Title(v) => title = Some(v),
            StatementKind::Header(v) => header = Some(v),
            StatementKind::Footer(v) => footer = Some(v),
            StatementKind::Caption(v) => caption = Some(v),
            StatementKind::Legend(v) => legend = Some(v),
            StatementKind::Theme(value) => {
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
            StatementKind::SaltGridRow { cells } => {
                if family_kind != DiagramKind::Salt {
                    return Err(Diagnostic::error(
                        "[E_FAMILY_MIXED] mixed diagram families are not supported in one document",
                    )
                    .with_span(stmt.span));
                }
                // Encode the cells into the name field using a special separator
                // so the renderer can reconstruct the grid row.
                use crate::ast::SaltCell as SC;
                let cell_strs: Vec<String> = cells
                    .into_iter()
                    .map(|c| match c {
                        SC::Label(t) => format!("L:{t}"),
                        SC::Input(t) => format!("I:{t}"),
                        SC::Button(t) => format!("B:{t}"),
                        SC::Combo(t) => format!("C:{t}"),
                        SC::CheckboxChecked(t) => format!("CX:{t}"),
                        SC::CheckboxUnchecked(t) => format!("CU:{t}"),
                        SC::RadioOn(t) => format!("RO:{t}"),
                        SC::RadioOff(t) => format!("RF:{t}"),
                    })
                    .collect();
                nodes.push(FamilyNode {
                    kind: FamilyNodeKind::Salt,
                    name: format!("SALT_ROW\x1f{}", cell_strs.join("\x1e")),
                    alias: None,
                    members: Vec::new(),
                    depth: 0,
                    label: None,
                    mindmap_side: MindMapSide::Right,
                    wbs_checkbox: None,
                    fill_color: None,
                });
            }
            StatementKind::Unknown(line) if family_kind == DiagramKind::Salt => {
                if line.trim() == "---" {
                    continue;
                }
                // Treat non-row unknown lines as plain label rows
                nodes.push(FamilyNode {
                    kind: FamilyNodeKind::Salt,
                    name: format!("SALT_ROW\x1fL:{line}"),
                    alias: None,
                    members: Vec::new(),
                    depth: 0,
                    label: None,
                    mindmap_side: MindMapSide::Right,
                    wbs_checkbox: None,
                    fill_color: None,
                });
            }
            StatementKind::Unknown(line) => {
                if family_kind == DiagramKind::Salt {
                    let text = line.trim();
                    if !text.is_empty() {
                        nodes.push(FamilyNode {
                            kind: FamilyNodeKind::Salt,
                            name: text.to_string(),
                            alias: None,
                            members: Vec::new(),
                            depth: 0,
                            label: None,
                            mindmap_side: MindMapSide::Right,
                            wbs_checkbox: None,
                            fill_color: None,
                        });
                    }
                    continue;
                }
                return Err(Diagnostic::error(format!(
                    "[E_PARSE_UNKNOWN] unsupported syntax: `{}`",
                    line
                ))
                .with_span(stmt.span));
            }
            _ => {
                return Err(Diagnostic::error(format!(
                    "[E_FAMILY_STUB_UNSUPPORTED] unsupported {} syntax in bootstrap slice",
                    family_kind_name(family_kind)
                ))
                .with_span(stmt.span));
            }
        }
    }

    // Merge duplicate Rel() pairs (same from→to→arrow) by joining their labels
    // with a newline so they render as stacked lines on a single arrow rather than
    // visually concatenated strings on overlapping arrows. This is the correct
    // PlantUML behaviour for C4 diagrams where Rel() is a macro that emits a
    // simple arrow — multiple calls with the same endpoints should coalesce.
    // Only applies to the stub/C4 path; the class/component paths keep separate
    // edges intentionally (e.g. bidirectional cardinality pairs). (#425)
    let relations = merge_duplicate_rel_labels(relations);

    Ok(FamilyDocument {
        kind: family_kind,
        nodes,
        relations,
        groups,
        json_projections,
        hide_options,
        namespace_separator,
        title,
        header,
        footer,
        caption,
        legend,
        orientation: FamilyOrientation::TopToBottom,
        style: SequenceStyle::default(),
        family_style: Some(FamilyStyle::Class(class_style)),
        text_overflow_policy: TextOverflowPolicy::WrapAndGrow,
        warnings,
    })
}

/// Merge relations that share the same `(from, to, arrow)` triple by joining
/// their labels with `\n`.  Duplicate Rel() macro calls in C4 diagrams produce
/// overlapping arrows that visually concatenate their labels with no delimiter;
/// coalescing them into one arrow with a newline-separated label is the correct
/// PlantUML parity behaviour (#425).
///
/// Only relations that are otherwise identical (same direction, color, style)
/// are merged; differing style attributes keep the relations separate.
fn merge_duplicate_rel_labels(relations: Vec<ModelFamilyRelation>) -> Vec<ModelFamilyRelation> {
    // Use an ordered map keyed by (from, to, arrow, direction, line_color,
    // dashed, hidden) so determinism is preserved (BTreeMap, not HashMap).
    // Value: index into `out` for the already-inserted canonical relation.
    type RelKey = (
        String,
        String,
        String,
        Option<String>,
        Option<String>,
        bool,
        bool,
    );
    let mut seen: std::collections::BTreeMap<RelKey, usize> = std::collections::BTreeMap::new();
    let mut out: Vec<ModelFamilyRelation> = Vec::with_capacity(relations.len());

    for rel in relations {
        let key = (
            rel.from.clone(),
            rel.to.clone(),
            rel.arrow.clone(),
            rel.direction.clone(),
            rel.line_color.clone(),
            rel.dashed,
            rel.hidden,
        );
        if let Some(&idx) = seen.get(&key) {
            // Merge this relation's label into the existing one.
            if let Some(new_label) = rel.label {
                let existing = &mut out[idx].label;
                *existing = Some(match existing.take() {
                    Some(prev) => format!("{prev}\n{new_label}"),
                    None => new_label,
                });
            }
            // Merge stereotype similarly.
            if let Some(new_st) = rel.stereotype {
                let existing = &mut out[idx].stereotype;
                if existing.is_none() {
                    *existing = Some(new_st);
                }
            }
        } else {
            seen.insert(key, out.len());
            out.push(rel);
        }
    }
    out
}

fn upsert_family_node(nodes: &mut Vec<FamilyNode>, mut node: FamilyNode) {
    let target_name = node.name.as_str();
    let target_alias = node.alias.as_deref();
    if let Some(existing) = nodes.iter_mut().find(|existing| {
        existing.name == target_name
            || existing.alias.as_deref() == Some(target_name)
            || target_alias.is_some_and(|alias| existing.name == alias)
            || (target_alias.is_some() && existing.alias.as_deref() == target_alias)
    }) {
        if existing.label.is_none() {
            existing.label = node.label.take();
        }
        if existing.alias.is_none() {
            existing.alias = node.alias.take();
        }
        if existing.fill_color.is_none() {
            existing.fill_color = node.fill_color.take();
        }
        existing.members.append(&mut node.members);
        return;
    }
    nodes.push(node);
}

fn extract_family_node_fill_color(members: &mut Vec<ClassMember>) -> Option<String> {
    let mut fill_color = None;
    members.retain(|member| {
        let Some(color) = member.text.strip_prefix("\x1fstyle:fill:") else {
            return true;
        };
        if fill_color.is_none() {
            fill_color = Some(color.trim().to_string());
        }
        false
    });
    fill_color
}

fn family_note_node(idx: usize, note: crate::ast::Note) -> FamilyNode {
    let mut label = note.text.trim().to_string();
    if label.is_empty() {
        label = "note".to_string();
    }
    let target = note.target.unwrap_or_default();
    FamilyNode {
        kind: FamilyNodeKind::Note,
        name: format!("__note_{idx:04}"),
        alias: Some(format!("note::{}|target={target}", note.position)),
        members: Vec::new(),
        depth: 0,
        label: Some(label),
        mindmap_side: MindMapSide::Right,
        wbs_checkbox: None,
        fill_color: None,
    }
}

pub(super) fn family_kind_name(kind: DiagramKind) -> &'static str {
    match kind {
        DiagramKind::Sequence => "sequence",
        DiagramKind::Class => "class",
        DiagramKind::Object => "object",
        DiagramKind::UseCase => "usecase",
        DiagramKind::Salt => "salt",
        DiagramKind::MindMap => "mindmap",
        DiagramKind::Wbs => "wbs",
        DiagramKind::Gantt => "gantt",
        DiagramKind::Chronology => "chronology",
        DiagramKind::Component => "component",
        DiagramKind::Deployment => "deployment",
        DiagramKind::State => "state",
        DiagramKind::Activity => "activity",
        DiagramKind::Timing => "timing",
        DiagramKind::Json => "json",
        DiagramKind::Yaml => "yaml",
        DiagramKind::Nwdiag => "nwdiag",
        DiagramKind::Archimate => "archimate",
        DiagramKind::Regex => "regex",
        DiagramKind::Ebnf => "ebnf",
        DiagramKind::Math => "math",
        DiagramKind::Sdl => "sdl",
        DiagramKind::Ditaa => "ditaa",
        DiagramKind::Chart => "chart",
        DiagramKind::Chen => "chen",
        DiagramKind::Unknown => "unknown",
    }
}
