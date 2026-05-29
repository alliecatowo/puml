use super::*;
use crate::ast::RawSyntaxCategory;
use crate::normalize::common::{self, CommonDirectives, LegendTextMode, RawSyntaxContext};

mod activity;
mod component;
mod styles;
mod timing;

use self::activity::{
    normalize_activity_note, normalize_activity_step, normalize_activity_unknown_line,
    ActivityNormalizeState,
};
use self::component::normalize_component_decl;
use self::styles::{ExtendedFamilyStyles, StyleParamInput, StyleParamRecord};
use self::timing::{
    normalize_timing_decl, normalize_timing_event, normalize_timing_relation_endpoint,
    normalize_timing_scale_node, TimingNormalizeState,
};

pub(super) fn normalize_extended_family(document: Document) -> Result<FamilyDocument, Diagnostic> {
    let family_kind = document.kind;
    let mut nodes = Vec::new();
    let mut relations = Vec::new();
    let mut groups = Vec::new();
    let mut json_projections: Vec<crate::model::JsonProjection> = Vec::new();
    let mut common = CommonDirectives::default();
    let mut hide_options: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut activity_state = ActivityNormalizeState::default();
    let mut timing_state = TimingNormalizeState::default();
    let mut family_styles = ExtendedFamilyStyles::new(family_kind);
    let mut style_params: Vec<StyleParamRecord> = Vec::new();
    let mut ext_warnings: Vec<Diagnostic> = Vec::new();
    let mut note_counter: usize = 0;
    let mut last_relation: Option<(String, String)> = None;
    let mut sprites = crate::sprites::SpriteRegistry::new();
    let mut list_sprites = false;
    let mut orientation = FamilyOrientation::TopToBottom;
    let mut edge_routing = crate::render::graph_layout::EdgeRouting::default();

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
                members,
            } => {
                normalize_component_decl(&mut nodes, kind, name, alias, label, members);
            }
            StatementKind::ClassDecl(decl) => {
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
                let mut members = decl.members;
                let fill_color = extract_family_node_fill_color(&mut members);
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
                normalize_activity_step(&mut nodes, &mut activity_state, step);
            }
            StatementKind::TimingDecl {
                kind,
                name,
                label,
                controls,
            } => {
                normalize_timing_decl(&mut nodes, &mut timing_state, kind, name, label, controls);
            }
            StatementKind::TimingEvent {
                time,
                signal,
                state,
                note,
            } => {
                normalize_timing_event(&mut nodes, &mut timing_state, time, signal, state, note);
            }
            StatementKind::FamilyRelation(mut rel) => {
                let (from, to) = if family_kind == DiagramKind::Timing {
                    (
                        normalize_timing_relation_endpoint(&rel.from, &timing_state),
                        normalize_timing_relation_endpoint(&rel.to, &timing_state),
                    )
                } else {
                    (rel.from.clone(), rel.to.clone())
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
                rel.from = from;
                rel.to = to;
                relations.push(model_relation_from_ast(rel)?);
            }
            StatementKind::Note(note) => {
                note_counter += 1;
                if family_kind == DiagramKind::Activity {
                    normalize_activity_note(&mut nodes, &mut activity_state, note);
                } else {
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
            }
            StatementKind::ClassGroup {
                kind,
                label,
                members,
                relations: group_relations,
                fill_color: group_fill_color,
            } => {
                let mut group_member_ids = Vec::with_capacity(members.len());
                for member_id in &members {
                    let mut parts = member_id.split('\t');
                    let node_id = parts.next().unwrap_or(member_id.as_str()).to_string();
                    let display_label = parts.next().map(str::to_string);
                    let node_kind_hint = parts.next();
                    let mut fill_color = None;
                    let mut style_members = Vec::new();
                    for part in parts {
                        if let Some(color) = part.strip_prefix("\x1fstyle:fill:") {
                            fill_color = Some(color.to_string());
                        } else if part.starts_with("\x1fstyle:") {
                            style_members.push(ClassMember {
                                text: part.to_string(),
                                modifier: None,
                            });
                        }
                    }
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
                            members: {
                                let mut members = display_label
                                    .as_deref()
                                    .map(extract_inline_stereotype_members)
                                    .unwrap_or_default();
                                members.extend(style_members);
                                members
                            },
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
                    fill_color: group_fill_color,
                });
                for rel in group_relations {
                    // Auto-create component/deployment nodes for relation endpoints
                    // that appear only inside scoped group blocks (e.g. `[Frontend] --> ApiAlias`
                    // inside a `package "Outer" { ... }` block).
                    if matches!(
                        family_kind,
                        DiagramKind::Component | DiagramKind::Deployment
                    ) {
                        let node_kind = if family_kind == DiagramKind::Component {
                            FamilyNodeKind::Component
                        } else {
                            FamilyNodeKind::Node
                        };
                        for endpoint in [rel.from.as_str(), rel.to.as_str()] {
                            if !endpoint.is_empty()
                                && !nodes.iter().any(|n| {
                                    n.name == endpoint || n.alias.as_deref() == Some(endpoint)
                                })
                            {
                                nodes.push(FamilyNode {
                                    kind: node_kind,
                                    name: endpoint.to_string(),
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
                    relations.push(model_relation_from_ast(rel)?);
                }
            }
            StatementKind::Title(v) => common.title(v),
            StatementKind::Header(v) => common.header(v),
            StatementKind::Footer(v) => common.footer(v),
            StatementKind::Caption(v) => common.caption(v),
            StatementKind::Legend(v) => {
                common.legend(v, LegendTextMode::ParsePackedPosition);
            }
            StatementKind::Mainframe(v) => common.mainframe(v),
            StatementKind::SkinParam { key, value } => {
                // `skinparam linetype <value>` is a global edge-routing knob
                // that mirrors PlantUML's upstream `splines=` Graphviz mode.
                // It applies to every Graphviz-routed family so we extract it
                // here, before family-specific classifiers see it as a noop.
                if key.trim().eq_ignore_ascii_case("linetype") {
                    if let Some(mode) =
                        crate::render::graph_layout::EdgeRouting::parse_linetype(&value)
                    {
                        edge_routing = mode;
                    }
                }
                family_styles.handle_skinparam(
                    family_kind,
                    &key,
                    &value,
                    stmt.span,
                    &mut ext_warnings,
                );
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
            StatementKind::Theme(value) => {
                family_styles.apply_theme(family_kind, &value, stmt.span)?;
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
            StatementKind::LegendPos(pos) => {
                common.legend_position(&pos);
            }
            StatementKind::Pragma(_)
            | StatementKind::AllowMixing
            | StatementKind::Include(_)
            | StatementKind::Define { .. }
            | StatementKind::Undef(_) => {}
            StatementKind::Scale(body) => {
                common.scale(&body);
                if family_kind == DiagramKind::Timing && body.contains(" as ") {
                    normalize_timing_scale_node(&mut nodes, body);
                }
            }
            kind if kind.raw_syntax().is_some() => {
                let raw = kind.raw_syntax().expect("raw syntax guard");
                let line = raw.line;
                match raw.category {
                    RawSyntaxCategory::Malformed
                    | RawSyntaxCategory::Deferred
                    | RawSyntaxCategory::CommentLowered => {
                        // Parser-bug signals: remain hard errors.
                        return Err(common::raw_syntax_diagnostic(
                            raw,
                            stmt.span,
                            RawSyntaxContext::Family(family_kind),
                        ));
                    }
                    RawSyntaxCategory::LegacyUnknown => {
                        // Graceful degradation: skip the unknown line and emit a
                        // non-fatal feature-loss warning so the valid remainder renders.
                        ext_warnings.push(common::raw_syntax_feature_loss_warning(
                            raw,
                            stmt.span,
                            RawSyntaxContext::Family(family_kind),
                        ));
                        continue;
                    }
                    RawSyntaxCategory::BenignPassthrough => {
                        // Handle `left to right direction` / `top to bottom direction`
                        // (and reverse variants) for component/state/activity diagrams.
                        if let Some(dir) = parse_family_orientation_directive(line) {
                            orientation = dir;
                            continue;
                        }
                    }
                    RawSyntaxCategory::Unsupported => {
                        if family_kind == DiagramKind::Activity {
                            normalize_activity_unknown_line(&mut nodes, &mut activity_state, line);
                            continue;
                        }
                        // Graceful degradation: skip the unsupported line and emit a
                        // non-fatal feature-loss warning so the valid remainder renders.
                        ext_warnings.push(common::raw_syntax_feature_loss_warning(
                            raw,
                            stmt.span,
                            RawSyntaxContext::Family(family_kind),
                        ));
                        continue;
                    }
                }
                // BenignPassthrough that wasn't consumed by a direction directive:
                // fall through to the error below.
                return Err(common::raw_syntax_diagnostic(
                    raw,
                    stmt.span,
                    RawSyntaxContext::Family(family_kind),
                ));
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

    for param in style_params {
        family_styles.handle_style_param(StyleParamInput {
            family_kind,
            selector: param.selector.as_deref(),
            property: &param.property,
            key: param.key.as_deref(),
            value: &param.value,
            span: param.span,
            warnings: &mut ext_warnings,
        });
    }
    common::sort_diagnostics_by_message_and_span(&mut ext_warnings);
    let sepia = family_styles.sepia();
    let family_style = family_styles.into_family_style(family_kind);

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
        family_style,
        text_overflow_policy: TextOverflowPolicy::WrapAndGrow,
        maximum_width: None,
        sprites,
        list_sprites,
        edge_routing,
        warnings: ext_warnings,
        groups,
        json_projections,
        hide_options,
        namespace_separator: None,
    })
}
