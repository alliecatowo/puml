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
    let mut common = CommonDirectives::default();
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
                            ClassSkinParamValue::AttributeIcons(enabled) => {
                                class_style.attribute_icons = enabled;
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
                let (name, typed_label) = if resolved_kind == FamilyNodeKind::Object {
                    split_object_instance_type(decl.name)
                } else {
                    (decl.name, None)
                };
                let node_id = clean_alias.as_deref().unwrap_or(&name).to_string();
                if resolved_kind == FamilyNodeKind::Map {
                    relations.extend(extract_map_row_relations(&members, &node_id));
                }
                upsert_family_node(
                    &mut nodes,
                    FamilyNode {
                        kind: resolved_kind,
                        name,
                        alias: clean_alias,
                        members,
                        depth: 0,
                        label: typed_label,
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
                let resolved_kind = resolve_usecase_node_kind(&mut members);
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
                        let nk = if node_kind == FamilyNodeKind::UseCase {
                            resolve_usecase_node_kind(&mut encoded_members)
                        } else {
                            match node_kind {
                                FamilyNodeKind::Object => FamilyNodeKind::Object,
                                _ => FamilyNodeKind::Class,
                            }
                        };
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
            StatementKind::Title(v) => common.title(v),
            StatementKind::Header(v) => common.raw_header(v),
            StatementKind::Footer(v) => common.raw_footer(v),
            StatementKind::Caption(v) => common.caption(v),
            StatementKind::Legend(v) => common.legend(v, LegendTextMode::Raw),
            StatementKind::Mainframe(v) => common.mainframe(v),
            StatementKind::Theme(value) => {
                class_style = class_style_from_sequence_theme(
                    &resolve_sequence_theme_preset(&value)
                        .map_err(|msg| Diagnostic::error(msg).with_span(stmt.span))?
                        .style,
                );
            }
            StatementKind::Pragma(_)
            | StatementKind::AllowMixing
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
            StatementKind::Unknown(line)
            | StatementKind::UnsupportedSyntax(line)
            | StatementKind::DeferredRaw(line)
            | StatementKind::CommentLowered(line)
            | StatementKind::MalformedSyntax(line)
                if family_kind == DiagramKind::Salt =>
            {
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
            StatementKind::Unknown(line)
            | StatementKind::UnsupportedSyntax(line)
            | StatementKind::DeferredRaw(line)
            | StatementKind::CommentLowered(line)
            | StatementKind::MalformedSyntax(line) => {
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
        title: common.title,
        header: common.header,
        footer: common.footer,
        caption: common.caption,
        legend: common.legend,
        mainframe: common.mainframe,
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
