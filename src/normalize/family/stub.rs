use super::*;
use crate::ast::RawSyntaxCategory;
use crate::normalize::common::{self, CommonDirectives, LegendTextMode, RawSyntaxContext};

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
    let graph_family = match family_kind {
        DiagramKind::Class => crate::theme::GraphStyleFamily::Class,
        DiagramKind::Object => crate::theme::GraphStyleFamily::Object,
        DiagramKind::UseCase => crate::theme::GraphStyleFamily::UseCase,
        _ => crate::theme::GraphStyleFamily::Class,
    };
    let mut style_cascade = crate::theme::GraphStyleCascade::new(graph_family);
    let mut salt_style = crate::theme::SaltStyle::default();
    let mut style_params: Vec<StyleParamRecord> = Vec::new();
    let mut warnings: Vec<Diagnostic> = Vec::new();
    let mut note_counter: usize = 0;
    let mut sprites = crate::sprites::SpriteRegistry::new();
    let mut list_sprites = false;
    let mut last_relation: Option<(String, String)> = None;
    let mut orientation = FamilyOrientation::TopToBottom;

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
                    salt_style.apply_key(&key, &value);
                    continue;
                }
                style_cascade.apply_skinparam(&key, &value, stmt.span, &mut warnings);
            }
            StatementKind::StyleParam {
                selector,
                property,
                key,
                value,
            } => {
                style_params.push(StyleParamRecord {
                    selector,
                    property,
                    key,
                    value,
                    span: stmt.span,
                });
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
                // Non-C4 stereotypes (e.g. `<<aws-EC2>>`) are returned as
                // `extra_stereotype` and re-injected as a member so that
                // downstream renderers (e.g. cloud-icon renderer, Refs #1258)
                // can discover them via the node's member list.
                let (clean_alias, c4_kind, extra_stereotype) =
                    sequence::extract_c4_stereotype(decl.alias);
                let mut members = decl.members;
                if let Some(stereo) = extra_stereotype {
                    members.insert(
                        0,
                        crate::ast::ClassMember {
                            text: stereo,
                            modifier: None,
                        },
                    );
                }
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
                relations.push(model_relation_from_ast(rel)?);
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
                        kind: FamilyNodeKind::Diamond,
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
                // Force the association node to Diamond kind even if a plain
                // `class Enrollment` declaration appeared earlier.
                if let Some(n) = nodes.iter_mut().find(|n| n.name == association) {
                    n.kind = FamilyNodeKind::Diamond;
                }
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
                let target_member = note.target_member.clone();
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
                        // When a member qualifier like `Foo::counter` is present,
                        // route the relation from the qualified endpoint so the
                        // renderer can anchor the connector at the member row.
                        let from_endpoint = if let Some(ref member) = target_member {
                            format!("{target}::{member}")
                        } else {
                            relation_node_endpoint(&target)
                        };
                        relations.push(simple_family_relation(
                            from_endpoint,
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
                fill_color: group_fill_color,
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
                    fill_color: group_fill_color,
                });
                for rel in group_relations {
                    last_relation = Some((rel.from.clone(), rel.to.clone()));
                    relations.push(model_relation_from_ast(rel)?);
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
            StatementKind::Header(v) => common.header(v),
            StatementKind::Footer(v) => common.footer(v),
            StatementKind::Caption(v) => common.caption(v),
            StatementKind::Legend(v) => common.legend(v, LegendTextMode::ParsePackedPosition),
            StatementKind::Mainframe(v) => common.mainframe(v),
            StatementKind::Scale(body) => common.scale(&body),
            StatementKind::Theme(value) => {
                if family_kind == DiagramKind::Salt {
                    let preset = crate::theme::resolve_sequence_theme_preset(&value)
                        .map_err(|msg| Diagnostic::error(msg).with_span(stmt.span))?;
                    salt_style = crate::theme::salt_style_from_sequence_theme(&preset.style);
                    continue;
                }
                style_cascade.apply_theme(&value, stmt.span)?;
            }
            StatementKind::LegendPos(pos) => {
                common.legend_position(&pos);
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
            kind if family_kind == DiagramKind::Salt && kind.raw_syntax().is_some() => {
                let raw = kind.raw_syntax().expect("raw syntax guard");
                match raw.category {
                    RawSyntaxCategory::Unsupported | RawSyntaxCategory::LegacyUnknown => {
                        // Graceful degradation: skip the unsupported line and emit a
                        // non-fatal feature-loss warning so the valid remainder renders.
                        warnings.push(common::raw_syntax_feature_loss_warning(
                            raw,
                            stmt.span,
                            RawSyntaxContext::Family(family_kind),
                        ));
                        continue;
                    }
                    RawSyntaxCategory::BenignPassthrough => {}
                    _ => {
                        return Err(common::raw_syntax_diagnostic(
                            raw,
                            stmt.span,
                            RawSyntaxContext::Family(family_kind),
                        ));
                    }
                }
                let line = raw.line;
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
            kind if kind.raw_syntax().is_some() => {
                let raw = kind.raw_syntax().expect("raw syntax guard");
                let line = raw.line;
                // Handle `left to right direction` / `top to bottom direction`
                if raw.category == RawSyntaxCategory::BenignPassthrough {
                    if let Some(dir) = parse_family_orientation_directive(line) {
                        orientation = dir;
                        continue;
                    }
                }
                match raw.category {
                    RawSyntaxCategory::Unsupported | RawSyntaxCategory::LegacyUnknown => {
                        // Graceful degradation: skip the unsupported line and emit a
                        // non-fatal feature-loss warning so the valid remainder renders.
                        warnings.push(common::raw_syntax_feature_loss_warning(
                            raw,
                            stmt.span,
                            RawSyntaxContext::Family(family_kind),
                        ));
                        continue;
                    }
                    RawSyntaxCategory::BenignPassthrough => {
                        // Unconsumed passthrough (direction not matched): still a
                        // parser-bug signal, keep as hard error below.
                    }
                    _ => {
                        return Err(common::raw_syntax_diagnostic(
                            raw,
                            stmt.span,
                            RawSyntaxContext::Family(family_kind),
                        ));
                    }
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
                return Err(common::raw_syntax_diagnostic(
                    raw,
                    stmt.span,
                    RawSyntaxContext::Family(family_kind),
                ));
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
    for param in style_params {
        if family_kind == DiagramKind::Salt {
            let applied = if let Some(key) = param.key.as_deref() {
                salt_style.apply_key(key, &param.value)
            } else {
                salt_style.apply_property(param.selector.as_deref(), &param.property, &param.value)
            };
            if !applied {
                warnings.push(
                    Diagnostic::warning(format!(
                        "[W_STYLE_UNSUPPORTED] unsupported style `{}` in selector `{}`",
                        param.property,
                        param.selector.as_deref().unwrap_or("saltDiagram")
                    ))
                    .with_span(param.span),
                );
            }
        } else {
            style_cascade.apply_style_param(
                param.selector.as_deref(),
                &param.property,
                param.key.as_deref(),
                &param.value,
                param.span,
                &mut warnings,
            );
        }
    }
    common::sort_diagnostics_by_message_and_span(&mut warnings);
    let sepia = style_cascade.sepia();
    let family_style = if family_kind == DiagramKind::Salt {
        crate::model::FamilyStyle::Salt(Box::new(salt_style))
    } else {
        style_cascade.into_family_style()
    };

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
        header_align: common.header_align,
        footer: common.footer,
        footer_align: common.footer_align,
        caption: common.caption,
        legend: common.legend,
        legend_halign: common.legend_halign,
        legend_valign: common.legend_valign,
        mainframe: common.mainframe,
        scale: common.scale,
        orientation,
        style: SequenceStyle {
            sepia,
            ..SequenceStyle::default()
        },
        family_style: Some(family_style),
        text_overflow_policy: TextOverflowPolicy::WrapAndGrow,
        maximum_width: None,
        sprites,
        list_sprites,
        warnings,
    })
}

struct StyleParamRecord {
    selector: Option<String>,
    property: String,
    key: Option<String>,
    value: String,
    span: crate::source::Span,
}
