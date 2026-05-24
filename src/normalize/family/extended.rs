use super::*;

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
    let mut mainframe = None;
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
                activity_step_counter += 1;
                let kind = activity_step_node_kind(&step.kind);
                let name = format!("__act_{activity_step_counter:04}");
                let mut label = step.label;
                let fill_color = extract_activity_inline_fill(&mut label);
                let sdl_shape = extract_activity_sdl_shape(&mut label);
                let note_meta = extract_activity_note_meta(&mut label);
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
                let mut alias_parts = vec![
                    format!("activity::{:?}", step.kind),
                    format!("lane={lane}"),
                    format!("fork_depth={activity_fork_depth}"),
                    format!("fork_branch={activity_fork_branch}"),
                ];
                if let Some(shape) = &sdl_shape {
                    alias_parts.push(format!("sdl={shape}"));
                }
                if let Some((side, floating)) = &note_meta {
                    alias_parts.push(format!("position={side}"));
                    alias_parts.push(format!("note_side={side}"));
                    if *floating {
                        alias_parts.push("note_floating=1".to_string());
                    }
                }
                let alias = alias_parts.join("|");
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
                            "activity::Note|position={}|note_side={}|lane={}|fork_depth={}|fork_branch={}",
                            position, position, "default", activity_fork_depth, activity_fork_branch
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
            StatementKind::Mainframe(v) => mainframe = Some(v),
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
            | StatementKind::AllowMixing
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
            StatementKind::Unknown(line)
            | StatementKind::UnsupportedSyntax(line)
            | StatementKind::DeferredRaw(line)
            | StatementKind::CommentLowered(line)
            | StatementKind::MalformedSyntax(line) => {
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
        mainframe,
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
