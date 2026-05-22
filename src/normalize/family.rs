use super::*;

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
    let mut class_monochrome_mode = None;
    let mut warnings: Vec<Diagnostic> = Vec::new();
    let mut note_counter: usize = 0;
    let mut sprites = crate::sprites::SpriteRegistry::new();
    let mut list_sprites = false;
    let mut last_relation: Option<(String, String)> = None;
    let mut orientation = FamilyOrientation::TopToBottom;
    let mut sepia = false;

    for stmt in document.statements {
        match stmt.kind {
            StatementKind::SpriteDef(sprite) => {
                sprites.insert(sprite.name.clone(), sprite);
            }
            StatementKind::ListSprites => {
                list_sprites = true;
            }
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
                            ClassSkinParamValue::ActorStyle(style) => {
                                class_style.actor_style = style;
                            }
                            ClassSkinParamValue::Monochrome(mode) => {
                                class_monochrome_mode = Some(mode);
                            }
                            ClassSkinParamValue::StereotypeBackgroundColor(stereotype, c) => {
                                class_style
                                    .stereotype_styles
                                    .entry(stereotype)
                                    .or_default()
                                    .background_color = Some(c);
                            }
                            ClassSkinParamValue::StereotypeBorderColor(stereotype, c) => {
                                class_style
                                    .stereotype_styles
                                    .entry(stereotype)
                                    .or_default()
                                    .border_color = Some(c);
                            }
                            ClassSkinParamValue::StereotypeHeaderBackgroundColor(stereotype, c) => {
                                class_style
                                    .stereotype_styles
                                    .entry(stereotype)
                                    .or_default()
                                    .header_color = Some(c);
                            }
                            ClassSkinParamValue::StereotypeFontColor(stereotype, c) => {
                                class_style
                                    .stereotype_styles
                                    .entry(stereotype)
                                    .or_default()
                                    .font_color = Some(c);
                            }
                        }
                    }
                    SkinParamSupport::UnsupportedKey => {
                        // Class diagrams accept generic sequence keys silently
                        // (PlantUML applies them across all families).
                        use crate::theme::{
                            classify_sequence_skinparam, SequenceSkinParamSupport,
                            SequenceSkinParamValue,
                        };
                        if key.trim().eq_ignore_ascii_case("sepia") {
                            if let SequenceSkinParamSupport::SupportedWithValue(
                                SequenceSkinParamValue::Sepia(enabled),
                            ) = classify_sequence_skinparam(&key, &value)
                            {
                                sepia = enabled;
                            }
                        } else if !matches!(
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
                let source_id = decl.alias.as_deref().unwrap_or(&decl.name).to_string();
                let heritage_relations =
                    extract_family_heritage_relations(&mut members, &source_id);
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
                for rel in &heritage_relations {
                    ensure_family_class_node(&mut nodes, &rel.from);
                }
                relations.extend(heritage_relations);
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
                let mut members = decl.members;
                let resolved_kind = if members.first().is_some_and(|m| m.text.trim() == "<<map>>") {
                    let _ = members.remove(0);
                    FamilyNodeKind::Map
                } else if members
                    .first()
                    .is_some_and(|m| m.text.trim() == "<<diamond>>")
                {
                    let _ = members.remove(0);
                    FamilyNodeKind::Diamond
                } else {
                    c4_kind.unwrap_or(FamilyNodeKind::Object)
                };
                let fill_color = extract_family_node_fill_color(&mut members);
                let node_id = clean_alias.as_deref().unwrap_or(&decl.name).to_string();
                if resolved_kind == FamilyNodeKind::Map {
                    relations.extend(extract_map_row_relations(&members, &node_id));
                }
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
            StatementKind::FamilyRelation(rel) => {
                last_relation = Some((rel.from.clone(), rel.to.clone()));
                relations.push(model_relation_from_ast(rel));
            }
            StatementKind::AssociationClass {
                left,
                right,
                association,
                arrow,
            } => {
                upsert_family_node(
                    &mut nodes,
                    FamilyNode {
                        kind: FamilyNodeKind::Class,
                        name: association.clone(),
                        alias: None,
                        members: Vec::new(),
                        depth: 0,
                        label: None,
                        mindmap_side: MindMapSide::Right,
                        wbs_checkbox: None,
                        fill_color: None,
                    },
                );
                relations.push(simple_family_relation(left.clone(), right.clone(), arrow));
                relations.push(simple_family_relation(
                    association.clone(),
                    left.clone(),
                    "..".to_string(),
                ));
                relations.push(simple_family_relation(association, right, "..".to_string()));
            }
            StatementKind::Note(note) => {
                note_counter += 1;
                let target = note.target.clone();
                let note_node = family_note_node(note_counter, note);
                let note_name = note_node.name.clone();
                nodes.push(note_node);
                if let Some(target) = target {
                    let target = if target.eq_ignore_ascii_case("on link") {
                        last_relation
                            .as_ref()
                            .map(|(_, to)| to.clone())
                            .unwrap_or_default()
                    } else {
                        target
                    };
                    if !target.is_empty() {
                        relations.push(simple_family_relation(
                            relation_node_endpoint(&target),
                            note_name,
                            "..".to_string(),
                        ));
                    }
                }
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
                    last_relation = Some((rel.from.clone(), rel.to.clone()));
                    relations.push(model_relation_from_ast(rel));
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
                // Handle `left to right direction` / `top to bottom direction`
                if let Some(dir) = parse_family_orientation_directive(&line) {
                    orientation = dir;
                    continue;
                }
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
    apply_class_visibility_controls(&mut nodes, &mut relations, &mut groups, &hide_options);
    let relations = merge_duplicate_rel_labels(relations);
    if let Some(mode) = class_monochrome_mode {
        apply_monochrome_to_class_style(&mut class_style, mode);
    }

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
        orientation,
        style: SequenceStyle {
            sepia,
            ..SequenceStyle::default()
        },
        family_style: Some(FamilyStyle::Class(class_style)),
        text_overflow_policy: TextOverflowPolicy::WrapAndGrow,
        maximum_width: None,
        sprites,
        list_sprites,
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

fn extract_activity_inline_fill(label: &mut Option<String>) -> Option<String> {
    let value = label.take()?;
    let Some(rest) = value.strip_prefix("\x1fstyle:fill:") else {
        *label = Some(value);
        return None;
    };
    let Some((color, display)) = rest.split_once('\x1f') else {
        *label = Some(value);
        return None;
    };
    *label = (!display.is_empty()).then(|| display.to_string());
    Some(color.trim().to_string())
}

/// Extract a SDL action shape marker (`\x1fsdl:<shape>\x1f<body>`) from the label.
/// Returns the shape name and leaves the display label in `label`.
fn extract_activity_sdl_shape(label: &mut Option<String>) -> Option<String> {
    let value = label.take()?;
    let Some(rest) = value.strip_prefix("\x1fsdl:") else {
        *label = Some(value);
        return None;
    };
    let Some((shape, display)) = rest.split_once('\x1f') else {
        *label = Some(value);
        return None;
    };
    *label = (!display.is_empty()).then(|| display.to_string());
    Some(shape.trim().to_string())
}

fn extract_family_heritage_relations(
    members: &mut Vec<ClassMember>,
    source_id: &str,
) -> Vec<ModelFamilyRelation> {
    let mut out = Vec::new();
    members.retain(|member| {
        let Some(rest) = member.text.strip_prefix("\x1fheritage:") else {
            return true;
        };
        if let Some((arrow, target)) = rest.split_once(':') {
            let target = target.trim();
            if !target.is_empty() {
                out.push(simple_family_relation(
                    target.to_string(),
                    source_id.to_string(),
                    arrow.to_string(),
                ));
            }
        }
        false
    });
    out
}

fn extract_map_row_relations(members: &[ClassMember], source_id: &str) -> Vec<ModelFamilyRelation> {
    members
        .iter()
        .filter_map(|member| parse_map_row_relation(&member.text, source_id))
        .collect()
}

fn parse_map_row_relation(row: &str, source_id: &str) -> Option<ModelFamilyRelation> {
    let trimmed = row.trim();
    for marker in [
        "*--->", "*-->", "*---", "*--", "*->", "-->", "---", "--", "..>", "...", "..",
    ] {
        let Some((key, target)) = trimmed.split_once(marker) else {
            continue;
        };
        let key = key.trim();
        let target = target.trim();
        if key.is_empty() || target.is_empty() {
            return None;
        }
        return Some(simple_family_relation(
            format!("{source_id}::{key}"),
            target.to_string(),
            marker.to_string(),
        ));
    }
    None
}

fn model_relation_from_ast(rel: crate::ast::FamilyRelation) -> ModelFamilyRelation {
    ModelFamilyRelation {
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
    }
}

fn simple_family_relation(from: String, to: String, arrow: String) -> ModelFamilyRelation {
    ModelFamilyRelation {
        from,
        to,
        arrow,
        label: None,
        stereotype: None,
        left_cardinality: None,
        right_cardinality: None,
        left_role: None,
        right_role: None,
        line_color: None,
        dashed: false,
        hidden: false,
        thickness: None,
        direction: None,
        left_lollipop: false,
        right_lollipop: false,
    }
}

fn ensure_family_class_node(nodes: &mut Vec<FamilyNode>, name: &str) {
    if name.is_empty()
        || nodes
            .iter()
            .any(|node| node.name == name || node.alias.as_deref() == Some(name))
    {
        return;
    }
    nodes.push(FamilyNode {
        kind: FamilyNodeKind::Class,
        name: name.to_string(),
        alias: None,
        members: Vec::new(),
        depth: 0,
        label: None,
        mindmap_side: MindMapSide::Right,
        wbs_checkbox: None,
        fill_color: None,
    });
}

fn apply_class_visibility_controls(
    nodes: &mut Vec<FamilyNode>,
    relations: &mut Vec<ModelFamilyRelation>,
    groups: &mut Vec<FamilyGroup>,
    hide_options: &std::collections::BTreeSet<String>,
) {
    if hide_options.is_empty() {
        return;
    }

    let removed = collect_filtered_node_names(nodes, relations, hide_options);
    if !removed.is_empty() {
        nodes.retain(|node| !node_matches_any_filter(node, &removed));
        relations.retain(|rel| {
            !name_matches_any_filter(&relation_node_endpoint(&rel.from), &removed)
                && !name_matches_any_filter(&relation_node_endpoint(&rel.to), &removed)
        });
        for group in groups {
            group
                .member_ids
                .retain(|member_id| !name_matches_any_filter(member_id, &removed));
        }
    }

    for node in nodes {
        let node_key = node
            .alias
            .as_deref()
            .unwrap_or(&node.name)
            .to_ascii_lowercase();
        node.members
            .retain(|member| class_member_visible_for_node(member, &node_key, hide_options));
    }
}

fn collect_filtered_node_names(
    nodes: &[FamilyNode],
    relations: &[ModelFamilyRelation],
    hide_options: &std::collections::BTreeSet<String>,
) -> std::collections::BTreeSet<String> {
    let mut removed = std::collections::BTreeSet::new();
    for opt in hide_options {
        if let Some(name) = opt
            .strip_prefix("hide node ")
            .or_else(|| opt.strip_prefix("remove node "))
        {
            removed.insert(clean_filter_name(name));
        }
    }
    if hide_options.contains("hide @unlinked") {
        let mut linked = std::collections::BTreeSet::new();
        for rel in relations {
            linked.insert(relation_node_endpoint(&rel.from).to_ascii_lowercase());
            linked.insert(relation_node_endpoint(&rel.to).to_ascii_lowercase());
        }
        for node in nodes {
            let name = node.name.to_ascii_lowercase();
            let alias = node.alias.as_deref().map(str::to_ascii_lowercase);
            if !linked.contains(&name) && alias.as_ref().is_none_or(|a| !linked.contains(a)) {
                removed.insert(name);
            }
        }
    }
    for opt in hide_options {
        if let Some(name) = opt.strip_prefix("restore node ") {
            removed.remove(&clean_filter_name(name));
        }
    }
    removed
}

fn class_member_visible_for_node(
    member: &ClassMember,
    node_key: &str,
    hide_options: &std::collections::BTreeSet<String>,
) -> bool {
    let text = member.text.trim();
    if hide_options.contains("stereotype") && text.starts_with("<<") && text.ends_with(">>") {
        return false;
    }
    if hide_options.contains("circle") && text == "()" {
        return false;
    }
    let visibility = member_visibility(text);
    let kind = member_kind(member);
    let show_key = format!("show {node_key} {kind}");
    let show_members_key = format!("show {node_key} members");
    if hide_options.contains(&show_key) || hide_options.contains(&show_members_key) {
        return true;
    }
    if hide_options.contains("members") || hide_options.contains(&format!("{visibility} members")) {
        return false;
    }
    if hide_options.contains(kind) || hide_options.contains(&format!("{visibility} {kind}")) {
        return false;
    }
    true
}

fn member_visibility(text: &str) -> &'static str {
    match text.trim_start().chars().next() {
        Some('+') => "public",
        Some('-') => "private",
        Some('#') => "protected",
        Some('~') => "package",
        _ => "public",
    }
}

fn member_kind(member: &ClassMember) -> &'static str {
    match member.modifier {
        Some(crate::ast::MemberModifier::Method) => "methods",
        Some(crate::ast::MemberModifier::Field) => "fields",
        _ => {
            let text = member
                .text
                .trim_start_matches(['+', '-', '#', '~'])
                .trim_start();
            if text.contains('(') {
                "methods"
            } else {
                "fields"
            }
        }
    }
}

fn node_matches_any_filter(
    node: &FamilyNode,
    filters: &std::collections::BTreeSet<String>,
) -> bool {
    name_matches_any_filter(&node.name, filters)
        || node
            .alias
            .as_deref()
            .is_some_and(|alias| name_matches_any_filter(alias, filters))
}

fn name_matches_any_filter(name: &str, filters: &std::collections::BTreeSet<String>) -> bool {
    filters.contains(&clean_filter_name(name))
}

fn clean_filter_name(name: &str) -> String {
    name.trim().trim_matches('"').to_ascii_lowercase()
}

fn relation_node_endpoint(endpoint: &str) -> String {
    let trimmed = endpoint.trim();
    if let Some((owner, member)) = trimmed.rsplit_once("::") {
        if !owner.is_empty() && !member.is_empty() {
            return owner.to_string();
        }
    }
    trimmed.to_string()
}
pub(super) fn normalize_family_tree(document: Document) -> Result<FamilyDocument, Diagnostic> {
    let mut nodes = Vec::new();
    let mut relations = Vec::new();
    let mut title = None;
    let mut header = None;
    let mut footer = None;
    let mut caption = None;
    let mut legend = None;

    let family_kind = document.kind;
    let mut warnings = Vec::new();
    let mut orientation = FamilyOrientation::TopToBottom;
    let mut style = SequenceStyle::default();
    let mut monochrome_mode = None;
    let mut text_overflow_policy = TextOverflowPolicy::WrapAndGrow;
    let mut maximum_width: Option<i32> = None;
    let mut mindmap_style = MindMapStyle::default();
    let mut mindmap_style_block: Option<String> = None;
    let mut sprites = crate::sprites::SpriteRegistry::new();
    let mut list_sprites = false;
    // MindMap: track whether subsequent depth-1 nodes should go on the left side.
    let mut mindmap_left_side_mode = false;
    let mut mindmap_multiline: Option<MindmapMultilineDraft> = None;

    for stmt in document.statements {
        match stmt.kind {
            StatementKind::SpriteDef(sprite) => {
                sprites.insert(sprite.name.clone(), sprite);
            }
            StatementKind::ListSprites => {
                list_sprites = true;
            }
            StatementKind::Title(v) => title = Some(v),
            StatementKind::Header(v) => header = Some(v),
            StatementKind::Footer(v) => footer = Some(v),
            StatementKind::Caption(v) => caption = Some(v),
            StatementKind::Legend(v) => legend = Some(v),
            StatementKind::SkinParam { key, value } => {
                if handle_family_overflow_skinparam(
                    &key,
                    &value,
                    &mut text_overflow_policy,
                    &mut warnings,
                    stmt.span,
                ) {
                    continue;
                }
                if family_kind == DiagramKind::MindMap
                    && handle_mindmap_maximum_width_skinparam(
                        &key,
                        &value,
                        &mut maximum_width,
                        &mut warnings,
                        stmt.span,
                    )
                {
                    continue;
                }
                match classify_sequence_skinparam(&key, &value) {
                    SequenceSkinParamSupport::SupportedNoop => {}
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::FootboxVisible(_),
                    ) => {}
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::ArrowColor(color),
                    ) => {
                        style.arrow_color = color;
                    }
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::LifelineBorderColor(color),
                    ) => {
                        style.lifeline_border_color = color;
                    }
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::ParticipantBackgroundColor(color),
                    ) => {
                        style.participant_background_color = color;
                    }
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::ParticipantBorderColor(color),
                    ) => {
                        style.participant_border_color = color;
                    }
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::ParticipantFontColor(color),
                    ) => {
                        style.participant_font_color = Some(color);
                    }
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::NoteBackgroundColor(color),
                    ) => {
                        style.note_background_color = color;
                    }
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::NoteBorderColor(color),
                    ) => {
                        style.note_border_color = color;
                    }
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::GroupBackgroundColor(color),
                    ) => {
                        style.group_background_color = color;
                    }
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::GroupBorderColor(color),
                    ) => {
                        style.group_border_color = color;
                    }
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::RoundCorner(n),
                    ) => {
                        style.round_corner = n;
                    }
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::Shadowing(s),
                    ) => {
                        style.shadowing = s;
                    }
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::DefaultFontName(name),
                    ) => {
                        style.default_font_name = Some(name);
                    }
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::DefaultFontSize(sz),
                    ) => {
                        style.default_font_size = Some(sz);
                    }
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::BackgroundColor(color),
                    ) => {
                        style.background_color = Some(color);
                    }
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::DefaultTextAlignment(align),
                    ) => {
                        style.text_alignment = align;
                    }
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::ParticipantPadding(n),
                    ) => {
                        style.participant_padding = Some(n);
                    }
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::BoxPadding(n),
                    ) => {
                        style.box_padding = Some(n);
                    }
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::MessageAlign(a),
                    ) => {
                        style.message_align = a;
                    }
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::ResponseMessageBelowArrow(b),
                    ) => {
                        style.response_message_below_arrow = b;
                    }
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::LifelineThickness(n),
                    ) => {
                        style.lifeline_thickness = Some(n);
                    }
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::MessageLineColor(c),
                    ) => {
                        style.message_line_color = Some(c);
                    }
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::ReferenceBackgroundColor(c),
                    ) => {
                        style.reference_background_color = Some(c);
                    }
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::ReferenceBorderColor(c),
                    ) => {
                        style.reference_border_color = Some(c);
                    }
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::GroupHeaderFontColor(c),
                    ) => {
                        style.group_header_font_color = Some(c);
                    }
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::GroupHeaderFontStyle(s),
                    ) => {
                        style.group_header_font_style = s;
                    }
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::Monochrome(mode),
                    ) => {
                        monochrome_mode = Some(mode);
                    }
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::Handwritten(enabled),
                    ) => {
                        style.hand_drawn = enabled;
                    }
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::LifelineNoSolid(nosolid),
                    ) => {
                        style.lifeline_nosolid = nosolid;
                    }
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::Sepia(enabled),
                    ) => {
                        style.sepia = enabled;
                    }
                    SequenceSkinParamSupport::UnsupportedValue => {
                        warnings.push(
                            Diagnostic::warning(format!(
                                "[W_SKINPARAM_UNSUPPORTED_VALUE] unsupported value `{}` for skinparam `{}`",
                                value, key
                            ))
                            .with_span(stmt.span),
                        );
                    }
                    SequenceSkinParamSupport::UnsupportedKey => {
                        warnings.push(
                            Diagnostic::warning(format!(
                                "[W_SKINPARAM_UNSUPPORTED] unsupported skinparam `{}`",
                                key
                            ))
                            .with_span(stmt.span),
                        );
                    }
                }
            }
            StatementKind::Theme(value) => {
                style = resolve_sequence_theme_preset(&value)
                    .map_err(|msg| Diagnostic::error(msg).with_span(stmt.span))?
                    .style;
            }
            StatementKind::Pragma(v) => {
                let trimmed = v.trim();
                let lower = trimmed.to_ascii_lowercase();
                if lower.starts_with("teoz ") || lower == "teoz" {
                    // Accept teoz pragma as a deterministic no-op compatibility hint.
                } else if lower == "sequencemessagespan true"
                    || lower == "sequence message span true"
                {
                    style.sequence_message_span = true;
                } else if lower == "sequencemessagespan false"
                    || lower == "sequence message span false"
                {
                    style.sequence_message_span = false;
                } else {
                    warnings.push(
                        Diagnostic::warning(format!(
                            "[W_PRAGMA_UNSUPPORTED] unsupported pragma `{}`",
                            trimmed
                        ))
                        .with_span(stmt.span),
                    );
                }
            }
            StatementKind::FamilyRelation(rel) => {
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
            StatementKind::Unknown(line) => {
                if line.trim().is_empty() {
                    continue;
                }
                if family_kind == DiagramKind::MindMap
                    && collect_mindmap_style_line(
                        &line,
                        &mut mindmap_style_block,
                        &mut mindmap_style,
                    )
                {
                    continue;
                }
                if let Some(value) = parse_family_orientation_directive(&line) {
                    orientation = value;
                    continue;
                }
                // MindMap `left side` / `right side` keyword switches which side
                // subsequent depth-1 nodes appear on when no explicit +/- prefix.
                if family_kind == DiagramKind::MindMap {
                    let lower = line.trim().to_ascii_lowercase();
                    if lower == "left side" {
                        mindmap_left_side_mode = true;
                        continue;
                    } else if lower == "right side" {
                        mindmap_left_side_mode = false;
                        continue;
                    }
                }
                if let Some(ref mut draft) = mindmap_multiline {
                    if let Some(node) = draft.append_line(&line) {
                        nodes.push(node);
                        mindmap_multiline = None;
                    }
                    continue;
                }
                if let Some(mut node_info) = parse_mindmap_or_wbs_node(&line) {
                    let kind = match family_kind {
                        DiagramKind::MindMap => FamilyNodeKind::MindMap,
                        DiagramKind::Wbs => FamilyNodeKind::Wbs,
                        _ => FamilyNodeKind::Salt,
                    };
                    // Apply left-side mode: if depth >= 1 and no explicit +/-
                    // prefix was given (we detect this by checking if the original
                    // line had a prefix), use the current mode.
                    if family_kind == DiagramKind::MindMap && node_info.depth >= 1 {
                        let has_explicit = line.trim_start().starts_with('+')
                            || line.trim_start().starts_with('-');
                        if !has_explicit && mindmap_left_side_mode {
                            node_info.side = MindMapSide::Left;
                        }
                    }
                    if let Some(body) = node_info.name.strip_prefix(':') {
                        let first = body.trim_start();
                        if !first.contains(';') {
                            mindmap_multiline = Some(MindmapMultilineDraft {
                                kind,
                                depth: node_info.depth,
                                name: first.to_string(),
                                alias: node_info.alias.clone(),
                                side: node_info.side,
                                checkbox: node_info.checkbox,
                                fill_color: node_info.fill_color,
                            });
                            continue;
                        }
                        node_info.name = first.trim_end_matches(';').trim_end().to_string();
                    }
                    nodes.push(FamilyNode {
                        kind,
                        name: node_info.name,
                        alias: node_info.alias,
                        members: Vec::new(),
                        depth: node_info.depth,
                        label: None,
                        mindmap_side: node_info.side,
                        wbs_checkbox: node_info.checkbox,
                        fill_color: node_info.fill_color,
                    });
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

    build_family_tree_relations(&mut nodes, &mut relations);
    normalize_family_tree_warnings(&mut warnings);
    if let Some(mode) = monochrome_mode {
        apply_monochrome_to_sequence_style(&mut style, mode);
    }
    let family_style =
        if family_kind == DiagramKind::MindMap && !mindmap_style.depth_styles.is_empty() {
            Some(FamilyStyle::MindMap(mindmap_style))
        } else {
            None
        };

    Ok(FamilyDocument {
        kind: family_kind,
        nodes,
        relations,
        title,
        header,
        footer,
        caption,
        legend,
        orientation,
        style,
        family_style,
        text_overflow_policy,
        maximum_width,
        sprites,
        list_sprites,
        warnings,
        groups: Vec::new(),
        json_projections: Vec::new(),
        hide_options: std::collections::BTreeSet::new(),
        namespace_separator: None,
    })
}

fn normalize_family_tree_warnings(warnings: &mut [Diagnostic]) {
    warnings.sort_by(|a, b| {
        let sa = a.span.map(|s| s.start).unwrap_or_default();
        let sb = b.span.map(|s| s.start).unwrap_or_default();
        (a.message.as_str(), sa).cmp(&(b.message.as_str(), sb))
    });
}

fn build_family_tree_relations(nodes: &mut [FamilyNode], relations: &mut Vec<ModelFamilyRelation>) {
    let mut parents: Vec<usize> = Vec::new();
    for idx in 0..nodes.len() {
        let depth = nodes[idx].depth;
        while parents.len() > depth {
            parents.pop();
        }
        if let Some(parent_idx) = parents.last().copied() {
            relations.push(ModelFamilyRelation {
                from: nodes[parent_idx].name.clone(),
                to: nodes[idx].name.clone(),
                arrow: "->".to_string(),
                label: None,
                stereotype: None,
                left_cardinality: None,
                right_cardinality: None,
                left_role: None,
                right_role: None,
                line_color: None,
                dashed: false,
                hidden: false,
                thickness: None,
                direction: None,
                left_lollipop: false,
                right_lollipop: false,
            });
        }
        parents.push(idx);
    }
}

struct MindmapMultilineDraft {
    kind: FamilyNodeKind,
    depth: usize,
    name: String,
    alias: Option<String>,
    side: MindMapSide,
    checkbox: Option<WbsCheckbox>,
    fill_color: Option<String>,
}

impl MindmapMultilineDraft {
    /// Append `line` to the in-progress multiline body. Returns `Some(node)` when the
    /// block ends on a line containing `;` (PlantUML ch17.4 / ch18.4).
    fn append_line(&mut self, line: &str) -> Option<FamilyNode> {
        let trimmed_end = line.trim_end();
        if trimmed_end.ends_with(';') {
            let tail = trimmed_end.trim_end_matches(';').trim_end();
            if !tail.is_empty() {
                if !self.name.is_empty() {
                    self.name.push('\n');
                }
                self.name.push_str(tail);
            }
            return Some(FamilyNode {
                kind: self.kind,
                name: self.name.clone(),
                alias: self.alias.clone(),
                members: Vec::new(),
                depth: self.depth,
                label: None,
                mindmap_side: self.side,
                wbs_checkbox: self.checkbox.clone(),
                fill_color: self.fill_color.clone(),
            });
        }
        let piece = line.trim();
        if !piece.is_empty() {
            if !self.name.is_empty() {
                self.name.push('\n');
            }
            self.name.push_str(piece);
        }
        None
    }
}

fn handle_mindmap_maximum_width_skinparam(
    key: &str,
    value: &str,
    maximum_width: &mut Option<i32>,
    warnings: &mut Vec<Diagnostic>,
    span: crate::source::Span,
) -> bool {
    if !key.trim().eq_ignore_ascii_case("maximumwidth") {
        return false;
    }
    match value.trim().parse::<i32>() {
        Ok(n) if n > 0 => *maximum_width = Some(n),
        _ => warnings.push(
            Diagnostic::warning(format!(
                "[W_SKINPARAM_UNSUPPORTED_VALUE] unsupported value `{}` for skinparam `{}`",
                value, key
            ))
            .with_span(span),
        ),
    }
    true
}

fn collect_mindmap_style_line(
    line: &str,
    block: &mut Option<String>,
    style: &mut MindMapStyle,
) -> bool {
    let lower = line.trim_start().to_ascii_lowercase();
    if let Some(source) = block {
        if let Some((before_end, _)) = split_style_end(line) {
            source.push('\n');
            source.push_str(before_end);
            parse_mindmap_style_source(source, style);
            *block = None;
        } else {
            source.push('\n');
            source.push_str(line);
        }
        return true;
    }

    if !lower.starts_with("<style") {
        return false;
    }

    let after_start = line
        .split_once('>')
        .map(|(_, after)| after)
        .unwrap_or_default();
    if let Some((before_end, _)) = split_style_end(after_start) {
        parse_mindmap_style_source(before_end, style);
    } else {
        *block = Some(after_start.to_string());
    }
    true
}

fn split_style_end(line: &str) -> Option<(&str, &str)> {
    let lower = line.to_ascii_lowercase();
    lower.find("</style>").map(|idx| {
        let end = idx + "</style>".len();
        (&line[..idx], &line[end..])
    })
}

fn parse_mindmap_style_source(source: &str, style: &mut MindMapStyle) {
    let prepared = source.replace('{', "\n{\n").replace('}', "\n}\n");
    let mut stack: Vec<String> = Vec::new();
    let mut pending_selector: Option<String> = None;
    for raw in prepared.lines() {
        let line = raw.trim().trim_end_matches(';').trim();
        if line.is_empty() {
            continue;
        }
        if line == "{" {
            if let Some(selector) = pending_selector.take() {
                stack.push(selector);
            }
            continue;
        }
        if line == "}" {
            stack.pop();
            continue;
        }
        if apply_mindmap_style_property(line, &stack, style) {
            continue;
        }
        pending_selector = Some(line.to_string());
    }
}

fn apply_mindmap_style_property(line: &str, stack: &[String], style: &mut MindMapStyle) -> bool {
    let mut parts = line.splitn(2, char::is_whitespace);
    let Some(raw_key) = parts.next() else {
        return false;
    };
    let key = raw_key.trim_end_matches(':').to_ascii_lowercase();
    if !matches!(key.as_str(), "backgroundcolor" | "fontcolor" | "linecolor") {
        return false;
    }
    let value = parts
        .next()
        .unwrap_or_default()
        .trim()
        .trim_start_matches(':')
        .trim()
        .trim_end_matches(';')
        .trim();
    if value.is_empty() {
        return true;
    }

    let in_mindmap = stack
        .iter()
        .any(|selector| selector.eq_ignore_ascii_case("mindmapDiagram"));
    let depth = stack.iter().rev().find_map(|selector| {
        let selector = selector.trim();
        let inner = selector.strip_prefix(":depth(")?.strip_suffix(')')?;
        inner.trim().parse::<usize>().ok()
    });
    if in_mindmap {
        if let Some(depth) = depth {
            apply_mindmap_depth_property(style.depth_styles.entry(depth).or_default(), &key, value);
        }
    }
    true
}

fn apply_mindmap_depth_property(patch: &mut MindMapDepthStyle, key: &str, value: &str) {
    match key {
        "backgroundcolor" => patch.background_color = Some(value.to_string()),
        "fontcolor" => patch.font_color = Some(value.to_string()),
        "linecolor" => patch.border_color = Some(value.to_string()),
        _ => {}
    }
}

fn handle_family_overflow_skinparam(
    key: &str,
    value: &str,
    policy: &mut TextOverflowPolicy,
    warnings: &mut Vec<Diagnostic>,
    span: crate::source::Span,
) -> bool {
    let normalized_key = key.trim().to_ascii_lowercase();
    let normalized_value = value.trim().to_ascii_lowercase();
    if normalized_key != "textoverflowpolicy" && normalized_key != "text_overflow_policy" {
        return false;
    }

    let parsed = match normalized_value.as_str() {
        "wrap" | "wrapandgrow" | "wrap_and_grow" | "wrapgrow" => {
            Some(TextOverflowPolicy::WrapAndGrow)
        }
        "ellipsis" | "ellipsesingleline" | "ellipsissingleline" | "singleline" | "nowrap" => {
            Some(TextOverflowPolicy::EllipsisSingleLine)
        }
        _ => {
            warnings.push(
                Diagnostic::warning(format!(
                    "[W_SKINPARAM_UNSUPPORTED_VALUE] unsupported value `{}` for skinparam `{}`",
                    value, key
                ))
                .with_span(span),
            );
            None
        }
    };
    if let Some(parsed) = parsed {
        *policy = parsed;
    }
    true
}

fn parse_family_orientation_directive(line: &str) -> Option<FamilyOrientation> {
    let tokens = line
        .split_whitespace()
        .map(|t| t.to_ascii_lowercase())
        .collect::<Vec<_>>();
    if tokens.len() == 4 && tokens[3].as_str() == "direction" {
        let key = [&tokens[0][..], &tokens[1][..], &tokens[2][..]].join(" ");
        return match key.as_str() {
            "left to right" => Some(FamilyOrientation::LeftToRight),
            "right to left" => Some(FamilyOrientation::RightToLeft),
            "top to bottom" => Some(FamilyOrientation::TopToBottom),
            "bottom to top" => Some(FamilyOrientation::BottomToTop),
            _ => None,
        };
    }
    None
}

struct MindMapWbsNode {
    depth: usize,
    name: String,
    alias: Option<String>,
    side: MindMapSide,
    checkbox: Option<WbsCheckbox>,
    fill_color: Option<String>,
}

/// Parse a MindMap / WBS node line. Handles:
///
/// - `* Root`, `** Child`, `*** Grandchild` — star-depth (depth = stars - 1)
/// - `*[#Orange] Root`, `**[#fef3c7] Child` — PlantUML-style node color tags
/// - `** Left child` after a `left side` keyword (tracked externally)
/// - `+** Right`, `-** Left` — explicit side prefix on first depth-2+ star
/// - WBS annotations: `[x]` checked, `[ ]` unchecked, `[%NN]` progress
fn parse_mindmap_or_wbs_node(line: &str) -> Option<MindMapWbsNode> {
    let trimmed = line.trim_start();

    // Detect optional side prefix: `+` = right, `-` = left (only matters at
    // depth >= 1 in MindMap, but we parse it universally and let the renderer
    // decide what to do with it).
    let (side_prefix, rest) = if let Some(s) = trimmed.strip_prefix('+') {
        (Some(MindMapSide::Right), s)
    } else if let Some(s) = trimmed.strip_prefix('-') {
        (Some(MindMapSide::Left), s)
    } else {
        (None, trimmed)
    };

    let star_prefix = rest.bytes().take_while(|c| *c == b'*').count();
    if star_prefix == 0 {
        return None;
    }

    let mut label = rest[star_prefix..].trim().to_string();
    let alias = parse_mindmap_wbs_alias(&mut label);
    let fill_color = parse_mindmap_wbs_color_tag(&mut label);
    if label.is_empty() {
        return None;
    }
    // PlantUML interprets `\n` in label text as a line break (#560).
    // Convert the literal backslash-n sequence to an actual newline so the
    // renderer's multi-line text emission path can wrap it.
    label = label.replace("\\n", "\n");

    // Parse WBS checkbox suffix: `[x]`, `[ ]`, `[%NN]` at end of label.
    let checkbox = parse_wbs_checkbox(&mut label);

    // Side defaults to Right unless explicitly prefixed.
    let side = side_prefix.unwrap_or(MindMapSide::Right);
    let depth = star_prefix.saturating_sub(1);

    Some(MindMapWbsNode {
        depth,
        name: label,
        alias,
        side,
        checkbox,
        fill_color,
    })
}

fn parse_mindmap_wbs_alias(label: &mut String) -> Option<String> {
    let trimmed = label.trim_start();
    if !trimmed.starts_with('(') {
        return None;
    }
    let close = trimmed.find(')')?;
    if close <= 1 {
        return None;
    }
    let alias = trimmed[1..close].trim().to_string();
    if alias.is_empty() {
        return None;
    }
    let remainder = &trimmed[close + 1..];
    if !remainder.is_empty() && !remainder.starts_with(char::is_whitespace) {
        return None;
    }
    *label = remainder.trim_start().to_string();
    Some(alias)
}

/// Parse a leading PlantUML color tag from MindMap/WBS labels.
///
/// PlantUML examples use tags such as `[#Orange]` and `[#lightgreen]`; SVG
/// accepts named colors without the leading `#`, while hex colors keep it.
fn parse_mindmap_wbs_color_tag(label: &mut String) -> Option<String> {
    let trimmed = label.trim_start();
    let rest = trimmed.strip_prefix('[')?;
    let close = rest.find(']')?;
    let raw = rest[..close].trim();
    let value = raw.strip_prefix('#')?.trim();
    if value.is_empty() {
        return None;
    }
    let normalized =
        if matches!(value.len(), 3 | 6 | 8) && value.chars().all(|ch| ch.is_ascii_hexdigit()) {
            format!("#{value}")
        } else {
            value.to_string()
        };
    *label = rest[close + 1..].trim_start().to_string();
    Some(normalized)
}

/// Try to parse a WBS checkbox annotation from the end of a label, stripping it
/// from the label string if found.
fn parse_wbs_checkbox(label: &mut String) -> Option<WbsCheckbox> {
    let trimmed = label.trim_end();
    if let Some(inner) = trimmed.strip_suffix(']') {
        if let Some(bracket_start) = inner.rfind('[') {
            let content = &inner[bracket_start + 1..];
            let checkbox = if content == "x" || content == "X" {
                Some(WbsCheckbox::Checked)
            } else if content == " " || content.is_empty() {
                Some(WbsCheckbox::Unchecked)
            } else if let Some(pct_str) = content.strip_prefix('%') {
                pct_str
                    .trim()
                    .parse::<u8>()
                    .ok()
                    .filter(|&n| n <= 100)
                    .map(WbsCheckbox::Progress)
            } else {
                None
            };
            if checkbox.is_some() {
                let prefix = &inner[..bracket_start].trim_end().to_string();
                *label = prefix.to_string();
                return checkbox;
            }
        }
    }
    None
}

pub(super) fn normalize_extended_family(document: Document) -> Result<FamilyDocument, Diagnostic> {
    let family_kind = document.kind;
    let mut nodes = Vec::new();
    let mut relations = Vec::new();
    let mut groups = Vec::new();
    let mut json_projections: Vec<crate::model::JsonProjection> = Vec::new();
    let mut title = None;
    let mut header = None;
    let mut footer = None;
    let mut caption = None;
    let mut legend = None;
    let mut hide_options: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut activity_step_counter: usize = 0;
    let mut activity_active_partition: Option<String> = None;
    let mut activity_fork_depth: usize = 0;
    let mut activity_fork_branch: usize = 0;
    let mut timing_current_time: Option<String> = None;
    let mut timing_current_signal: Option<String> = None;
    let mut timing_anchors = std::collections::BTreeMap::new();
    let mut timing_clock_scales = std::collections::BTreeMap::new();
    let mut component_style = ComponentStyle::default();
    let mut activity_style = ActivityStyle::default();
    let mut timing_style = TimingStyle::default();
    let mut component_monochrome_mode = None;
    let mut activity_monochrome_mode = None;
    let mut timing_monochrome_mode = None;
    let mut ext_warnings: Vec<Diagnostic> = Vec::new();
    let mut note_counter: usize = 0;
    let mut last_relation: Option<(String, String)> = None;
    let mut sprites = crate::sprites::SpriteRegistry::new();
    let mut list_sprites = false;
    let mut orientation = FamilyOrientation::TopToBottom;
    let mut sepia = false;

    for stmt in document.statements {
        match stmt.kind {
            StatementKind::SpriteDef(sprite) => {
                sprites.insert(sprite.name.clone(), sprite);
            }
            StatementKind::ListSprites => {
                list_sprites = true;
            }
            StatementKind::ComponentDecl {
                kind,
                name,
                alias,
                label,
                mut members,
            } => {
                let node_kind = component_node_kind(kind);
                let fill_color = extract_family_node_fill_color(&mut members);
                nodes.push(FamilyNode {
                    kind: node_kind,
                    name,
                    alias,
                    members,
                    depth: 0,
                    label,
                    mindmap_side: MindMapSide::Right,
                    wbs_checkbox: None,
                    fill_color,
                });
            }
            StatementKind::StateDecl(decl) => nodes.push(FamilyNode {
                kind: FamilyNodeKind::State,
                name: decl.name,
                alias: decl.alias,
                members: Vec::new(),
                depth: 0,
                label: None,
                mindmap_side: MindMapSide::Right,
                wbs_checkbox: None,
                fill_color: None,
            }),
            StatementKind::ActivityStep(step) => {
                activity_step_counter += 1;
                let kind = activity_step_node_kind(&step.kind);
                let name = format!("__act_{activity_step_counter:04}");
                let mut label = step.label;
                let fill_color = extract_activity_inline_fill(&mut label);
                let sdl_shape = extract_activity_sdl_shape(&mut label);
                let is_activity_note_step = matches!(step.kind, ActivityStepKind::Note);
                match step.kind {
                    ActivityStepKind::PartitionStart => {
                        activity_active_partition = label.clone();
                    }
                    ActivityStepKind::PartitionEnd => {
                        activity_active_partition = None;
                    }
                    ActivityStepKind::Fork => {
                        activity_fork_depth += 1;
                        activity_fork_branch = 0;
                    }
                    ActivityStepKind::ForkAgain => {
                        activity_fork_branch += 1;
                    }
                    ActivityStepKind::EndFork => {
                        activity_fork_depth = activity_fork_depth.saturating_sub(1);
                        activity_fork_branch = 0;
                    }
                    _ => {}
                }
                let lane = if is_activity_note_step {
                    "default".to_string()
                } else {
                    activity_active_partition
                        .clone()
                        .unwrap_or_else(|| "default".to_string())
                };
                let alias = if let Some(shape) = &sdl_shape {
                    format!(
                        "activity::{:?}|lane={}|fork_depth={}|fork_branch={}|sdl={}",
                        step.kind, lane, activity_fork_depth, activity_fork_branch, shape
                    )
                } else {
                    format!(
                        "activity::{:?}|lane={}|fork_depth={}|fork_branch={}",
                        step.kind, lane, activity_fork_depth, activity_fork_branch
                    )
                };
                nodes.push(FamilyNode {
                    kind,
                    name,
                    alias: Some(alias),
                    members: Vec::new(),
                    depth: 0,
                    label,
                    mindmap_side: MindMapSide::Right,
                    wbs_checkbox: None,
                    fill_color,
                });
            }
            StatementKind::TimingDecl {
                kind,
                name,
                label,
                controls,
            } => {
                if matches!(kind, TimingDeclKind::Clock) {
                    let period = controls.iter().find_map(|control| {
                        let rest = control.strip_prefix("period ")?;
                        rest.split_whitespace().next()?.parse::<i64>().ok()
                    });
                    let offset = controls.iter().find_map(|control| {
                        let rest = control.strip_prefix("offset ")?;
                        rest.split_whitespace().next()?.parse::<i64>().ok()
                    });
                    if let Some(period) = period {
                        timing_clock_scales.insert(name.clone(), (period, offset.unwrap_or(0)));
                    }
                }
                let node_kind = timing_decl_node_kind(kind);
                nodes.push(FamilyNode {
                    kind: node_kind,
                    name,
                    alias: None,
                    members: controls
                        .into_iter()
                        .map(|text| crate::ast::ClassMember {
                            text,
                            modifier: None,
                        })
                        .collect(),
                    depth: 0,
                    label,
                    mindmap_side: MindMapSide::Right,
                    wbs_checkbox: None,
                    fill_color: None,
                });
            }
            StatementKind::TimingEvent {
                time,
                signal,
                state,
                note,
            } => {
                // §10.27: `<signal> has <values>` ordering declaration — append ordering
                // to the already-created signal node so the renderer can use it.
                if family_kind == DiagramKind::Timing
                    && state.is_none()
                    && note
                        .as_deref()
                        .is_some_and(|n| n.starts_with("__timing:order:"))
                {
                    if let (Some(sig_name), Some(note_str)) = (&signal, &note) {
                        let order_payload = note_str
                            .strip_prefix("__timing:order:")
                            .unwrap_or("")
                            .to_string();
                        if !order_payload.is_empty() {
                            if let Some(sig_node) = nodes.iter_mut().find(|n| {
                                matches!(n.kind, FamilyNodeKind::TimingRobust)
                                    && n.name == *sig_name
                            }) {
                                sig_node.members.push(crate::ast::ClassMember {
                                    text: format!("__timing:order:{order_payload}"),
                                    modifier: None,
                                });
                            }
                        }
                    }
                    continue;
                }
                if family_kind == DiagramKind::Timing
                    && signal.is_none()
                    && state.is_none()
                    && note.as_deref().is_some_and(|n| n.starts_with("__timing:"))
                {
                    let normalized_time = normalize_timing_time(
                        &time,
                        timing_current_time.as_deref(),
                        &timing_anchors,
                        &timing_clock_scales,
                    );
                    if let Some(anchor) = note
                        .as_deref()
                        .and_then(|n| n.strip_prefix("__timing:anchor:"))
                    {
                        if !normalized_time.is_empty() {
                            timing_anchors.insert(anchor.to_string(), normalized_time.clone());
                            timing_current_time = Some(normalized_time);
                        }
                        continue;
                    }
                    nodes.push(FamilyNode {
                        kind: FamilyNodeKind::TimingEvent,
                        name: normalized_time,
                        alias: None,
                        members: Vec::new(),
                        depth: 0,
                        label: note,
                        mindmap_side: MindMapSide::Right,
                        wbs_checkbox: None,
                        fill_color: None,
                    });
                    continue;
                }
                let mut signal = signal;
                if family_kind == DiagramKind::Timing
                    && signal.is_none()
                    && state.is_none()
                    && note.is_none()
                    && !time.is_empty()
                    && nodes.iter().any(|node: &FamilyNode| {
                        matches!(
                            node.kind,
                            FamilyNodeKind::TimingConcise
                                | FamilyNodeKind::TimingRobust
                                | FamilyNodeKind::TimingClock
                                | FamilyNodeKind::TimingBinary
                        ) && node.name == time
                    })
                {
                    timing_current_signal = Some(time);
                    continue;
                }
                if family_kind == DiagramKind::Timing && signal.is_none() && state.is_some() {
                    signal = timing_current_signal.clone();
                }
                let effective_time = if time.is_empty() || signal.as_deref() == Some(time.as_str())
                {
                    timing_current_time.clone().unwrap_or_default()
                } else {
                    let normalized_time = normalize_timing_time(
                        &time,
                        timing_current_time.as_deref(),
                        &timing_anchors,
                        &timing_clock_scales,
                    );
                    timing_current_time = Some(normalized_time.clone());
                    normalized_time
                };
                let note = note.map(|note| {
                    normalize_timing_range_note(
                        &note,
                        timing_current_time.as_deref(),
                        &timing_anchors,
                        &timing_clock_scales,
                    )
                });
                let display = match (&signal, &state, &note) {
                    (Some(s), Some(st), _) => format!("{s} is {st}"),
                    (None, None, Some(n)) => n.clone(),
                    _ => String::new(),
                };
                nodes.push(FamilyNode {
                    kind: FamilyNodeKind::TimingEvent,
                    name: effective_time,
                    alias: signal,
                    members: state
                        .into_iter()
                        .map(|s| crate::ast::ClassMember {
                            text: s,
                            modifier: None,
                        })
                        .collect(),
                    depth: 0,
                    label: if display.is_empty() {
                        None
                    } else {
                        Some(display)
                    },
                    mindmap_side: MindMapSide::Right,
                    wbs_checkbox: None,
                    fill_color: None,
                });
            }
            StatementKind::FamilyRelation(rel) => {
                let (from, to) = if family_kind == DiagramKind::Timing {
                    (
                        normalize_timing_endpoint(
                            &rel.from,
                            timing_current_time.as_deref(),
                            &timing_anchors,
                            &timing_clock_scales,
                        ),
                        normalize_timing_endpoint(
                            &rel.to,
                            timing_current_time.as_deref(),
                            &timing_anchors,
                            &timing_clock_scales,
                        ),
                    )
                } else {
                    (rel.from, rel.to)
                };
                last_relation = Some((from.clone(), to.clone()));
                // Component/Deployment: auto-create nodes for relation endpoints
                // declared only via bracket shorthand (e.g. `[WebServer] --> [DB]`).
                if matches!(
                    family_kind,
                    DiagramKind::Component | DiagramKind::Deployment
                ) {
                    for endpoint in [&from, &to] {
                        if !endpoint.is_empty()
                            && !nodes.iter().any(|n| {
                                n.name == *endpoint || n.alias.as_deref() == Some(endpoint.as_str())
                            })
                        {
                            let node_kind = if family_kind == DiagramKind::Component {
                                FamilyNodeKind::Component
                            } else {
                                FamilyNodeKind::Node
                            };
                            nodes.push(FamilyNode {
                                kind: node_kind,
                                name: endpoint.clone(),
                                alias: None,
                                members: Vec::new(),
                                depth: 0,
                                label: None,
                                mindmap_side: MindMapSide::Right,
                                wbs_checkbox: None,
                                fill_color: None,
                            });
                        }
                    }
                }
                relations.push(ModelFamilyRelation {
                    from,
                    to,
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
            StatementKind::Note(note) => {
                note_counter += 1;
                if family_kind == DiagramKind::Activity {
                    activity_step_counter += 1;
                    let position = note.position.trim().to_string();
                    let text = note.text.trim();
                    let label = if text.is_empty() {
                        "note".to_string()
                    } else {
                        text.to_string()
                    };
                    nodes.push(FamilyNode {
                        kind: FamilyNodeKind::Note,
                        name: format!("__act_{activity_step_counter:04}"),
                        alias: Some(format!(
                            "activity::Note|position={}|lane={}|fork_depth={}|fork_branch={}",
                            position, "default", activity_fork_depth, activity_fork_branch
                        )),
                        members: Vec::new(),
                        depth: 0,
                        label: Some(label),
                        mindmap_side: MindMapSide::Right,
                        wbs_checkbox: None,
                        fill_color: None,
                    });
                } else {
                    let target = note.target.clone();
                    let note_node = family_note_node(note_counter, note);
                    let note_name = note_node.name.clone();
                    nodes.push(note_node);
                    if let Some(target) = target {
                        let target = if target.eq_ignore_ascii_case("on link") {
                            last_relation
                                .as_ref()
                                .map(|(_, to)| to.clone())
                                .unwrap_or_default()
                        } else {
                            target
                        };
                        if !target.is_empty() {
                            relations.push(simple_family_relation(
                                relation_node_endpoint(&target),
                                note_name,
                                "..".to_string(),
                            ));
                        }
                    }
                }
            }
            StatementKind::ClassGroup {
                kind,
                label,
                members,
                relations: group_relations,
            } => {
                let mut group_member_ids = Vec::with_capacity(members.len());
                for member_id in &members {
                    let mut parts = member_id.split('\t');
                    let node_id = parts.next().unwrap_or(member_id.as_str()).to_string();
                    let display_label = parts.next().map(str::to_string);
                    let node_kind_hint = parts.next();
                    let fill_color = parts
                        .find_map(|part| part.strip_prefix("\x1fstyle:fill:").map(str::to_string));
                    let unscoped_alias = node_id
                        .rsplit("::")
                        .next()
                        .filter(|alias| *alias != node_id)
                        .map(str::to_string);
                    group_member_ids.push(node_id.clone());
                    let already_exists = nodes.iter().any(|n: &FamilyNode| {
                        n.name == node_id || n.alias.as_deref() == Some(node_id.as_str())
                    });
                    if !already_exists {
                        let fallback_kind = node_kind_hint
                            .and_then(scoped_component_kind_hint)
                            .unwrap_or(match family_kind {
                                DiagramKind::Deployment => FamilyNodeKind::Node,
                                DiagramKind::Component => FamilyNodeKind::Component,
                                _ => FamilyNodeKind::Component,
                            });
                        nodes.push(FamilyNode {
                            kind: fallback_kind,
                            name: node_id,
                            alias: unscoped_alias,
                            members: display_label
                                .as_deref()
                                .map(extract_inline_stereotype_members)
                                .unwrap_or_default(),
                            depth: 0,
                            label: display_label.map(strip_inline_stereotypes),
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
            StatementKind::Title(v) => title = Some(v),
            StatementKind::Header(v) => header = Some(v),
            StatementKind::Footer(v) => footer = Some(v),
            StatementKind::Caption(v) => caption = Some(v),
            StatementKind::Legend(v) => {
                legend = Some(sequence::strip_legend_pos_prefix(&v));
            }
            StatementKind::SkinParam { key, value } => {
                let mut handled = false;
                if key.trim().eq_ignore_ascii_case("monochrome") {
                    handled = true;
                    match classify_sequence_skinparam(&key, &value) {
                        SequenceSkinParamSupport::SupportedNoop => {}
                        SequenceSkinParamSupport::SupportedWithValue(
                            SequenceSkinParamValue::Monochrome(mode),
                        ) => match family_kind {
                            DiagramKind::Component | DiagramKind::Deployment => {
                                component_monochrome_mode = Some(mode);
                            }
                            DiagramKind::Activity => {
                                activity_monochrome_mode = Some(mode);
                            }
                            DiagramKind::Timing => {
                                timing_monochrome_mode = Some(mode);
                            }
                            _ => {}
                        },
                        _ => ext_warnings.push(
                            Diagnostic::warning(format!(
                                "[W_SKINPARAM_UNSUPPORTED_VALUE] unsupported value `{}` for skinparam `{}`",
                                value, key
                            ))
                            .with_span(stmt.span),
                        ),
                    }
                } else if key.trim().eq_ignore_ascii_case("handwritten") {
                    handled = true;
                    match classify_sequence_skinparam(&key, &value) {
                        SequenceSkinParamSupport::SupportedNoop
                        | SequenceSkinParamSupport::SupportedWithValue(
                            SequenceSkinParamValue::Handwritten(_),
                        ) => {}
                        _ => ext_warnings.push(
                            Diagnostic::warning(format!(
                                "[W_SKINPARAM_UNSUPPORTED_VALUE] unsupported value `{}` for skinparam `{}`",
                                value, key
                            ))
                            .with_span(stmt.span),
                        ),
                    }
                } else if key.trim().eq_ignore_ascii_case("sepia") {
                    handled = true;
                    if let SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::Sepia(enabled),
                    ) = classify_sequence_skinparam(&key, &value)
                    {
                        sepia = enabled;
                    }
                }
                if matches!(
                    family_kind,
                    DiagramKind::Component | DiagramKind::Deployment
                ) {
                    use crate::theme::ComponentSkinParamValue;
                    match classify_component_skinparam(&key, &value) {
                        SkinParamSupport::SupportedNoop => {
                            handled = true;
                        }
                        SkinParamSupport::SupportedWithValue(v) => {
                            handled = true;
                            match v {
                                ComponentSkinParamValue::BackgroundColor(c) => {
                                    component_style.background_color = c;
                                }
                                ComponentSkinParamValue::BorderColor(c) => {
                                    component_style.border_color = c;
                                }
                                ComponentSkinParamValue::InterfaceColor(c) => {
                                    component_style.interface_color = c;
                                }
                                ComponentSkinParamValue::FontColor(c) => {
                                    component_style.font_color = c;
                                }
                                ComponentSkinParamValue::ArrowColor(c) => {
                                    component_style.arrow_color = c;
                                }
                                ComponentSkinParamValue::StyleMode(mode) => {
                                    component_style.component_style_mode = mode;
                                }
                            }
                        }
                        SkinParamSupport::UnsupportedKey => {}
                        SkinParamSupport::UnsupportedValue => {
                            handled = true;
                            ext_warnings.push(
                                Diagnostic::warning(format!(
                                    "[W_SKINPARAM_UNSUPPORTED_VALUE] unsupported value `{}` for skinparam `{}`",
                                    value, key
                                ))
                                .with_span(stmt.span),
                            );
                        }
                    }
                }
                if !handled && matches!(family_kind, DiagramKind::Activity) {
                    use crate::theme::ActivitySkinParamValue;
                    match classify_activity_skinparam(&key, &value) {
                        SkinParamSupport::SupportedNoop => {
                            handled = true;
                        }
                        SkinParamSupport::SupportedWithValue(v) => {
                            handled = true;
                            match v {
                                ActivitySkinParamValue::BackgroundColor(c) => {
                                    activity_style.background_color = c;
                                }
                                ActivitySkinParamValue::BorderColor(c) => {
                                    activity_style.border_color = c;
                                }
                                ActivitySkinParamValue::DiamondBackgroundColor(c) => {
                                    activity_style.diamond_color = c;
                                }
                                ActivitySkinParamValue::BarColor(c) => {
                                    activity_style.fork_color = c;
                                }
                                ActivitySkinParamValue::FontColor(c) => {
                                    activity_style.font_color = c;
                                }
                                ActivitySkinParamValue::ArrowColor(c) => {
                                    activity_style.arrow_color = c;
                                }
                            }
                        }
                        SkinParamSupport::UnsupportedKey => {}
                        SkinParamSupport::UnsupportedValue => {
                            handled = true;
                            ext_warnings.push(
                                Diagnostic::warning(format!(
                                    "[W_SKINPARAM_UNSUPPORTED_VALUE] unsupported value `{}` for skinparam `{}`",
                                    value, key
                                ))
                                .with_span(stmt.span),
                            );
                        }
                    }
                }
                if !handled && matches!(family_kind, DiagramKind::Timing) {
                    use crate::theme::TimingSkinParamValue;
                    match classify_timing_skinparam(&key, &value) {
                        SkinParamSupport::SupportedNoop => {
                            handled = true;
                        }
                        SkinParamSupport::SupportedWithValue(v) => {
                            handled = true;
                            match v {
                                TimingSkinParamValue::BackgroundColor(c) => {
                                    timing_style.background_color = c;
                                }
                                TimingSkinParamValue::AxisColor(c) => {
                                    timing_style.axis_color = c;
                                }
                                TimingSkinParamValue::GridColor(c) => {
                                    timing_style.grid_color = c;
                                }
                                TimingSkinParamValue::SignalBackgroundColor(c) => {
                                    timing_style.signal_background_color = c;
                                }
                                TimingSkinParamValue::SignalBorderColor(c) => {
                                    timing_style.signal_border_color = c;
                                }
                                TimingSkinParamValue::ArrowColor(c) => {
                                    timing_style.arrow_color = c;
                                }
                                TimingSkinParamValue::FontColor(c) => {
                                    timing_style.font_color = c;
                                }
                            }
                        }
                        SkinParamSupport::UnsupportedKey => {}
                        SkinParamSupport::UnsupportedValue => {
                            handled = true;
                            ext_warnings.push(
                                Diagnostic::warning(format!(
                                    "[W_SKINPARAM_UNSUPPORTED_VALUE] unsupported value `{}` for skinparam `{}`",
                                    value, key
                                ))
                                .with_span(stmt.span),
                            );
                        }
                    }
                }
                if !handled {
                    ext_warnings.push(
                        Diagnostic::warning(format!(
                            "[W_SKINPARAM_UNSUPPORTED] unsupported skinparam `{}`",
                            key
                        ))
                        .with_span(stmt.span),
                    );
                }
            }
            StatementKind::Theme(value) => {
                let style = resolve_sequence_theme_preset(&value)
                    .map_err(|msg| Diagnostic::error(msg).with_span(stmt.span))?
                    .style;
                match family_kind {
                    DiagramKind::Component | DiagramKind::Deployment => {
                        component_style = component_style_from_sequence_theme(&style);
                    }
                    DiagramKind::Activity => {
                        activity_style = activity_style_from_sequence_theme(&style);
                    }
                    DiagramKind::Timing => {
                        timing_style = timing_style_from_sequence_theme(&style);
                    }
                    _ => {}
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
            StatementKind::HideOption(opt) => {
                hide_options.insert(opt.to_ascii_lowercase());
            }
            StatementKind::Pragma(_)
            | StatementKind::Include(_)
            | StatementKind::Define { .. }
            | StatementKind::Undef(_)
            | StatementKind::LegendPos(_) => {}
            StatementKind::Scale(body) => {
                if family_kind == DiagramKind::Timing && body.contains(" as ") {
                    nodes.push(FamilyNode {
                        kind: FamilyNodeKind::TimingEvent,
                        name: String::new(),
                        alias: None,
                        members: Vec::new(),
                        depth: 0,
                        label: Some(format!("__timing:scale:{body}")),
                        mindmap_side: MindMapSide::Right,
                        wbs_checkbox: None,
                        fill_color: None,
                    });
                }
            }
            StatementKind::Unknown(line) => {
                // Handle `left to right direction` / `top to bottom direction`
                // (and reverse variants) for component/state/activity diagrams.
                if let Some(dir) = parse_family_orientation_directive(&line) {
                    orientation = dir;
                    continue;
                }
                if family_kind == DiagramKind::Activity {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    activity_step_counter += 1;
                    if trimmed.starts_with('|') && trimmed.ends_with('|') && trimmed.len() > 2 {
                        activity_active_partition =
                            Some(trimmed.trim_matches('|').trim().to_string());
                    }
                    let lane = activity_active_partition
                        .clone()
                        .unwrap_or_else(|| "default".to_string());
                    nodes.push(FamilyNode {
                        kind: if trimmed.starts_with('|') && trimmed.ends_with('|') {
                            FamilyNodeKind::ActivityPartition
                        } else {
                            FamilyNodeKind::ActivityAction
                        },
                        name: format!("__act_{activity_step_counter:04}"),
                        alias: Some(format!(
                            "activity::OldStyle|lane={lane}|fork_depth={activity_fork_depth}|fork_branch={activity_fork_branch}"
                        )),
                        members: Vec::new(),
                        depth: 0,
                        label: Some(trimmed.to_string()),
                        mindmap_side: MindMapSide::Right,
                        wbs_checkbox: None,
                        fill_color: None,
                    });
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
                    "[E_FAMILY_{}_UNSUPPORTED_STMT] unsupported {} syntax",
                    family_kind_name(family_kind).to_uppercase(),
                    family_kind_name(family_kind)
                ))
                .with_span(stmt.span));
            }
        }
    }

    let family_style = match family_kind {
        DiagramKind::Component | DiagramKind::Deployment => {
            if let Some(mode) = component_monochrome_mode {
                apply_monochrome_to_component_style(&mut component_style, mode);
            }
            Some(FamilyStyle::Component(component_style))
        }
        DiagramKind::Activity => {
            if let Some(mode) = activity_monochrome_mode {
                apply_monochrome_to_activity_style(&mut activity_style, mode);
            }
            Some(FamilyStyle::Activity(activity_style))
        }
        DiagramKind::Timing => {
            if let Some(mode) = timing_monochrome_mode {
                apply_monochrome_to_timing_style(&mut timing_style, mode);
            }
            Some(FamilyStyle::Timing(timing_style))
        }
        _ => None,
    };

    if matches!(
        family_kind,
        DiagramKind::Component | DiagramKind::Deployment
    ) {
        apply_component_visibility_controls(&mut nodes, &mut relations, &hide_options);
    }

    Ok(FamilyDocument {
        kind: family_kind,
        nodes,
        relations,
        title,
        header,
        footer,
        caption,
        legend,
        orientation,
        style: SequenceStyle {
            sepia,
            ..SequenceStyle::default()
        },
        family_style,
        text_overflow_policy: TextOverflowPolicy::WrapAndGrow,
        maximum_width: None,
        sprites,
        list_sprites,
        warnings: ext_warnings,
        groups,
        json_projections,
        hide_options,
        namespace_separator: None,
    })
}

fn apply_component_visibility_controls(
    nodes: &mut Vec<FamilyNode>,
    relations: &mut Vec<ModelFamilyRelation>,
    hide_options: &std::collections::BTreeSet<String>,
) {
    let node_tags: Vec<std::collections::BTreeSet<String>> =
        nodes.iter_mut().map(extract_component_node_tags).collect();

    if hide_options.is_empty() {
        return;
    }

    let hidden_nodes: std::collections::BTreeSet<String> = nodes
        .iter()
        .zip(node_tags.iter())
        .filter(|(node, tags)| component_node_hidden(node, tags, hide_options))
        .flat_map(|(node, _)| component_node_match_keys(node))
        .collect();
    if !hidden_nodes.is_empty() {
        nodes.retain(|node| !component_node_matches_any(node, &hidden_nodes));
        relations.retain(|rel| {
            !hidden_nodes.contains(&rel.from.to_ascii_lowercase())
                && !hidden_nodes.contains(&rel.to.to_ascii_lowercase())
        });
    }

    // Apply hide/remove @unlinked for component and deployment diagrams.
    // A node is "unlinked" if neither its name nor alias appears in any relation endpoint.
    if hide_options.contains("hide @unlinked") || hide_options.contains("remove @unlinked") {
        let mut linked: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        for rel in relations.iter() {
            linked.insert(rel.from.to_ascii_lowercase());
            linked.insert(rel.to.to_ascii_lowercase());
        }
        nodes.retain(|node| {
            let name_lc = node.name.to_ascii_lowercase();
            let alias_lc = node.alias.as_deref().map(str::to_ascii_lowercase);
            linked.contains(&name_lc) || alias_lc.as_ref().is_some_and(|a| linked.contains(a))
        });
    }
}

fn extract_component_node_tags(node: &mut FamilyNode) -> std::collections::BTreeSet<String> {
    let mut tags = std::collections::BTreeSet::new();
    node.members.retain(|member| {
        let Some(tag) = member.text.strip_prefix("\x1fcomponent:tag:") else {
            return true;
        };
        tags.insert(tag.to_ascii_lowercase());
        false
    });
    tags
}

fn component_node_hidden(
    node: &FamilyNode,
    tags: &std::collections::BTreeSet<String>,
    hide_options: &std::collections::BTreeSet<String>,
) -> bool {
    let mut hidden = hide_options.contains("hide node *") || hide_options.contains("remove node *");

    for tag in tags {
        let hide_tag = format!("hide node {tag}");
        let remove_tag = format!("remove node {tag}");
        if hide_options.contains(&hide_tag) || hide_options.contains(&remove_tag) {
            hidden = true;
        }
    }

    for key in component_node_match_keys(node) {
        if key.starts_with('$') {
            continue;
        }
        let hide_node = format!("hide node {key}");
        let remove_node = format!("remove node {key}");
        if hide_options.contains(&hide_node) || hide_options.contains(&remove_node) {
            hidden = true;
        }
    }

    if hide_options.contains("restore node *") {
        hidden = false;
    }
    for tag in tags {
        if hide_options.contains(&format!("restore node {tag}")) {
            hidden = false;
        }
    }
    for key in component_node_match_keys(node) {
        if key.starts_with('$') {
            continue;
        }
        if hide_options.contains(&format!("restore node {key}")) {
            hidden = false;
        }
    }

    hidden
}

fn component_node_match_keys(node: &FamilyNode) -> std::collections::BTreeSet<String> {
    let mut keys = std::collections::BTreeSet::from([node.name.to_ascii_lowercase()]);
    if let Some(alias) = &node.alias {
        keys.insert(alias.to_ascii_lowercase());
    }
    keys
}

fn component_node_matches_any(
    node: &FamilyNode,
    keys: &std::collections::BTreeSet<String>,
) -> bool {
    component_node_match_keys(node)
        .iter()
        .any(|key| keys.contains(key))
}

fn extract_inline_stereotype_members(label: &str) -> Vec<crate::ast::ClassMember> {
    let (_, stereotypes) = strip_inline_stereotypes_with_values(label);
    declaration_stereotype_members(stereotypes)
}

fn scoped_component_kind_hint(kind: &str) -> Option<FamilyNodeKind> {
    Some(match kind {
        "action" => FamilyNodeKind::Action,
        "agent" => FamilyNodeKind::Agent,
        "component" => FamilyNodeKind::Component,
        "interface" => FamilyNodeKind::Interface,
        "port" => FamilyNodeKind::Port,
        "node" => FamilyNodeKind::Node,
        "artifact" => FamilyNodeKind::Artifact,
        "boundary" => FamilyNodeKind::Boundary,
        "cloud" => FamilyNodeKind::Cloud,
        "circle" => FamilyNodeKind::Circle,
        "collections" => FamilyNodeKind::Collections,
        "frame" => FamilyNodeKind::Frame,
        "storage" => FamilyNodeKind::Storage,
        "container" => FamilyNodeKind::Container,
        "control" => FamilyNodeKind::Control,
        "database" => FamilyNodeKind::Database,
        "entity" => FamilyNodeKind::Entity,
        "package" => FamilyNodeKind::Package,
        "rectangle" => FamilyNodeKind::Rectangle,
        "folder" => FamilyNodeKind::Folder,
        "file" => FamilyNodeKind::File,
        "card" => FamilyNodeKind::Card,
        "actor" => FamilyNodeKind::Actor,
        "hexagon" => FamilyNodeKind::Hexagon,
        "label" => FamilyNodeKind::Label,
        "person" => FamilyNodeKind::Person,
        "process" => FamilyNodeKind::Process,
        "queue" => FamilyNodeKind::Queue,
        "stack" => FamilyNodeKind::Stack,
        "usecase" => FamilyNodeKind::UseCaseDeployment,
        _ => return None,
    })
}

fn strip_inline_stereotypes(label: String) -> String {
    strip_inline_stereotypes_with_values(&label).0
}

fn strip_inline_stereotypes_with_values(label: &str) -> (String, Vec<String>) {
    let mut remaining = label.trim().to_string();
    let mut stereotypes = Vec::new();
    while let Some(start) = remaining.find("<<") {
        let Some(end_rel) = remaining[start + 2..].find(">>") else {
            break;
        };
        let end = start + 2 + end_rel;
        let value = remaining[start + 2..end].trim();
        if !value.is_empty() {
            stereotypes.push(value.to_string());
        }
        remaining.replace_range(start..end + 2, "");
    }
    (remaining.trim().to_string(), stereotypes)
}

fn declaration_stereotype_members(stereotypes: Vec<String>) -> Vec<crate::ast::ClassMember> {
    stereotypes
        .into_iter()
        .map(|stereotype| crate::ast::ClassMember {
            text: format!("<<{stereotype}>>"),
            modifier: None,
        })
        .collect()
}

fn normalize_timing_time(
    raw: &str,
    current: Option<&str>,
    anchors: &std::collections::BTreeMap<String, String>,
    clocks: &std::collections::BTreeMap<String, (i64, i64)>,
) -> String {
    let trimmed = raw.trim().trim_start_matches('@');
    if let Some(anchor_expr) = trimmed.strip_prefix(':') {
        return normalize_timing_anchor_expr(anchor_expr, current, anchors);
    }
    if let Some((clock_name, multiplier)) = trimmed.split_once('*') {
        if let Some((period, offset)) = clocks.get(clock_name.trim()) {
            if let Ok(n) = multiplier.trim().parse::<i64>() {
                return period.saturating_mul(n).saturating_add(*offset).to_string();
            }
        }
    }
    if let Some(delta) = trimmed
        .strip_prefix('+')
        .and_then(|v| v.parse::<i64>().ok())
    {
        let base = current.and_then(|v| v.parse::<i64>().ok()).unwrap_or(0);
        return base.saturating_add(delta).to_string();
    }
    if let Some(delta) = trimmed
        .strip_prefix('-')
        .and_then(|v| v.parse::<i64>().ok())
    {
        let base = current.and_then(|v| v.parse::<i64>().ok()).unwrap_or(0);
        return base.saturating_sub(delta).to_string();
    }
    trimmed.to_string()
}

fn normalize_timing_anchor_expr(
    raw: &str,
    current: Option<&str>,
    anchors: &std::collections::BTreeMap<String, String>,
) -> String {
    let trimmed = raw.trim();
    let split_idx = trimmed
        .char_indices()
        .skip(1)
        .find(|(_, ch)| *ch == '+' || *ch == '-')
        .map(|(idx, _)| idx);
    let (name, offset) = match split_idx {
        Some(idx) => (&trimmed[..idx], Some(&trimmed[idx..])),
        None => (trimmed, None),
    };
    let base = anchors
        .get(name)
        .cloned()
        .unwrap_or_else(|| current.unwrap_or_default().to_string());
    let Some(offset) = offset else {
        return base;
    };
    let base_num = base.parse::<i64>().unwrap_or(0);
    let delta = offset.parse::<i64>().unwrap_or(0);
    base_num.saturating_add(delta).to_string()
}

fn normalize_timing_endpoint(
    raw: &str,
    current: Option<&str>,
    anchors: &std::collections::BTreeMap<String, String>,
    clocks: &std::collections::BTreeMap<String, (i64, i64)>,
) -> String {
    let trimmed = raw.trim();
    let Some((signal, time)) = trimmed.split_once('@') else {
        return current
            .filter(|time| !time.is_empty())
            .map(|time| format!("{trimmed}@{time}"))
            .unwrap_or_else(|| trimmed.to_string());
    };
    let normalized_time = normalize_timing_time(time, current, anchors, clocks);
    format!("{}@{}", signal.trim(), normalized_time)
}

fn normalize_timing_range_note(
    note: &str,
    current: Option<&str>,
    anchors: &std::collections::BTreeMap<String, String>,
    clocks: &std::collections::BTreeMap<String, (i64, i64)>,
) -> String {
    let Some(rest) = note.strip_prefix("range:") else {
        return note.to_string();
    };
    let (end, label) = rest.split_once(':').unwrap_or((rest, ""));
    let normalized_end = normalize_timing_time(end, current, anchors, clocks);
    if label.is_empty() {
        format!("range:{normalized_end}")
    } else {
        format!("range:{normalized_end}:{label}")
    }
}

fn component_node_kind(kind: ComponentNodeKind) -> FamilyNodeKind {
    match kind {
        ComponentNodeKind::Action => FamilyNodeKind::Action,
        ComponentNodeKind::Agent => FamilyNodeKind::Agent,
        ComponentNodeKind::Component => FamilyNodeKind::Component,
        ComponentNodeKind::Interface => FamilyNodeKind::Interface,
        ComponentNodeKind::Port => FamilyNodeKind::Port,
        ComponentNodeKind::Node => FamilyNodeKind::Node,
        ComponentNodeKind::Artifact => FamilyNodeKind::Artifact,
        ComponentNodeKind::Boundary => FamilyNodeKind::Boundary,
        ComponentNodeKind::Cloud => FamilyNodeKind::Cloud,
        ComponentNodeKind::Circle => FamilyNodeKind::Circle,
        ComponentNodeKind::Collections => FamilyNodeKind::Collections,
        ComponentNodeKind::Frame => FamilyNodeKind::Frame,
        ComponentNodeKind::Storage => FamilyNodeKind::Storage,
        ComponentNodeKind::Container => FamilyNodeKind::Container,
        ComponentNodeKind::Control => FamilyNodeKind::Control,
        ComponentNodeKind::Database => FamilyNodeKind::Database,
        ComponentNodeKind::Entity => FamilyNodeKind::Entity,
        ComponentNodeKind::Package => FamilyNodeKind::Package,
        ComponentNodeKind::Rectangle => FamilyNodeKind::Rectangle,
        ComponentNodeKind::Folder => FamilyNodeKind::Folder,
        ComponentNodeKind::File => FamilyNodeKind::File,
        ComponentNodeKind::Card => FamilyNodeKind::Card,
        ComponentNodeKind::Actor => FamilyNodeKind::Actor,
        ComponentNodeKind::Hexagon => FamilyNodeKind::Hexagon,
        ComponentNodeKind::Label => FamilyNodeKind::Label,
        ComponentNodeKind::Person => FamilyNodeKind::Person,
        ComponentNodeKind::Process => FamilyNodeKind::Process,
        ComponentNodeKind::Queue => FamilyNodeKind::Queue,
        ComponentNodeKind::Stack => FamilyNodeKind::Stack,
        ComponentNodeKind::UseCase => FamilyNodeKind::UseCaseDeployment,
    }
}

fn activity_step_node_kind(kind: &ActivityStepKind) -> FamilyNodeKind {
    match kind {
        ActivityStepKind::Start => FamilyNodeKind::ActivityStart,
        ActivityStepKind::Stop
        | ActivityStepKind::End
        | ActivityStepKind::Kill
        | ActivityStepKind::Detach => FamilyNodeKind::ActivityStop,
        ActivityStepKind::Action => FamilyNodeKind::ActivityAction,
        ActivityStepKind::Connector => FamilyNodeKind::ActivityAction,
        ActivityStepKind::Note => FamilyNodeKind::Note,
        ActivityStepKind::IfStart
        | ActivityStepKind::WhileStart
        | ActivityStepKind::RepeatWhile => FamilyNodeKind::ActivityDecision,
        ActivityStepKind::Else | ActivityStepKind::EndIf | ActivityStepKind::EndWhile => {
            FamilyNodeKind::ActivityMerge
        }
        ActivityStepKind::Fork | ActivityStepKind::ForkAgain => FamilyNodeKind::ActivityFork,
        ActivityStepKind::EndFork => FamilyNodeKind::ActivityForkEnd,
        ActivityStepKind::RepeatStart => FamilyNodeKind::ActivityMerge,
        ActivityStepKind::Arrow
        | ActivityStepKind::PartitionStart
        | ActivityStepKind::PartitionEnd => FamilyNodeKind::ActivityPartition,
    }
}

fn timing_decl_node_kind(kind: TimingDeclKind) -> FamilyNodeKind {
    match kind {
        TimingDeclKind::Concise => FamilyNodeKind::TimingConcise,
        TimingDeclKind::Robust => FamilyNodeKind::TimingRobust,
        TimingDeclKind::Clock => FamilyNodeKind::TimingClock,
        TimingDeclKind::Binary => FamilyNodeKind::TimingBinary,
    }
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
        DiagramKind::Unknown => "unknown",
    }
}
