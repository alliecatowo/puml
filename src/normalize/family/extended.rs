use super::*;

pub(crate) fn normalize_extended_family(document: Document) -> Result<FamilyDocument, Diagnostic> {
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
    let mut activity_step_counter: usize = 0;
    let mut activity_active_partition: Option<String> = None;
    let mut activity_fork_depth: usize = 0;
    let mut activity_fork_branch: usize = 0;
    let mut timing_current_time: Option<String> = None;
    let mut timing_current_signal: Option<String> = None;
    let mut component_style = ComponentStyle::default();
    let mut activity_style = ActivityStyle::default();
    let mut timing_style = TimingStyle::default();
    let mut ext_warnings: Vec<Diagnostic> = Vec::new();
    let mut note_counter: usize = 0;

    for stmt in document.statements {
        match stmt.kind {
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
                match step.kind {
                    ActivityStepKind::PartitionStart => {
                        activity_active_partition = step.label.clone();
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
                let lane = activity_active_partition
                    .clone()
                    .unwrap_or_else(|| "default".to_string());
                let alias = format!(
                    "activity::{:?}|lane={}|fork_depth={}|fork_branch={}",
                    step.kind, lane, activity_fork_depth, activity_fork_branch
                );
                nodes.push(FamilyNode {
                    kind,
                    name,
                    alias: Some(alias),
                    members: Vec::new(),
                    depth: 0,
                    label: step.label,
                    mindmap_side: MindMapSide::Right,
                    wbs_checkbox: None,
                    fill_color: None,
                });
            }
            StatementKind::TimingDecl {
                kind,
                name,
                label,
                controls,
            } => {
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
                    let normalized_time =
                        normalize_timing_time(&time, timing_current_time.as_deref());
                    timing_current_time = Some(normalized_time.clone());
                    normalized_time
                };
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
            StatementKind::Pragma(_)
            | StatementKind::Include(_)
            | StatementKind::Define { .. }
            | StatementKind::Undef(_)
            | StatementKind::Scale(_)
            | StatementKind::LegendPos(_) => {}
            StatementKind::Unknown(line) => {
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
            Some(FamilyStyle::Component(component_style))
        }
        DiagramKind::Activity => Some(FamilyStyle::Activity(activity_style)),
        DiagramKind::Timing => Some(FamilyStyle::Timing(timing_style)),
        _ => None,
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
        orientation: FamilyOrientation::TopToBottom,
        style: SequenceStyle::default(),
        family_style,
        text_overflow_policy: TextOverflowPolicy::WrapAndGrow,
        warnings: ext_warnings,
        groups,
        json_projections,
        hide_options: std::collections::BTreeSet::new(),
        namespace_separator: None,
    })
}

fn extract_inline_stereotype_members(label: &str) -> Vec<crate::ast::ClassMember> {
    let (_, stereotypes) = strip_inline_stereotypes_with_values(label);
    declaration_stereotype_members(stereotypes)
}

fn scoped_component_kind_hint(kind: &str) -> Option<FamilyNodeKind> {
    Some(match kind {
        "component" => FamilyNodeKind::Component,
        "interface" => FamilyNodeKind::Interface,
        "port" => FamilyNodeKind::Port,
        "node" => FamilyNodeKind::Node,
        "artifact" => FamilyNodeKind::Artifact,
        "cloud" => FamilyNodeKind::Cloud,
        "frame" => FamilyNodeKind::Frame,
        "storage" => FamilyNodeKind::Storage,
        "database" => FamilyNodeKind::Database,
        "package" => FamilyNodeKind::Package,
        "rectangle" => FamilyNodeKind::Rectangle,
        "folder" => FamilyNodeKind::Folder,
        "file" => FamilyNodeKind::File,
        "card" => FamilyNodeKind::Card,
        "actor" => FamilyNodeKind::Actor,
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

fn normalize_timing_time(raw: &str, current: Option<&str>) -> String {
    let trimmed = raw.trim().trim_start_matches('@');
    if let Some((_, multiplier)) = trimmed.split_once('*') {
        if let Ok(n) = multiplier.trim().parse::<i64>() {
            return n.to_string();
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

fn component_node_kind(kind: ComponentNodeKind) -> FamilyNodeKind {
    match kind {
        ComponentNodeKind::Component => FamilyNodeKind::Component,
        ComponentNodeKind::Interface => FamilyNodeKind::Interface,
        ComponentNodeKind::Port => FamilyNodeKind::Port,
        ComponentNodeKind::Node => FamilyNodeKind::Node,
        ComponentNodeKind::Artifact => FamilyNodeKind::Artifact,
        ComponentNodeKind::Cloud => FamilyNodeKind::Cloud,
        ComponentNodeKind::Frame => FamilyNodeKind::Frame,
        ComponentNodeKind::Storage => FamilyNodeKind::Storage,
        ComponentNodeKind::Database => FamilyNodeKind::Database,
        ComponentNodeKind::Package => FamilyNodeKind::Package,
        ComponentNodeKind::Rectangle => FamilyNodeKind::Rectangle,
        ComponentNodeKind::Folder => FamilyNodeKind::Folder,
        ComponentNodeKind::File => FamilyNodeKind::File,
        ComponentNodeKind::Card => FamilyNodeKind::Card,
        ComponentNodeKind::Actor => FamilyNodeKind::Actor,
    }
}

fn activity_step_node_kind(kind: &ActivityStepKind) -> FamilyNodeKind {
    match kind {
        ActivityStepKind::Start => FamilyNodeKind::ActivityStart,
        ActivityStepKind::Stop | ActivityStepKind::End => FamilyNodeKind::ActivityStop,
        ActivityStepKind::Action => FamilyNodeKind::ActivityAction,
        ActivityStepKind::IfStart
        | ActivityStepKind::WhileStart
        | ActivityStepKind::RepeatWhile => FamilyNodeKind::ActivityDecision,
        ActivityStepKind::Else | ActivityStepKind::EndIf | ActivityStepKind::EndWhile => {
            FamilyNodeKind::ActivityMerge
        }
        ActivityStepKind::Fork | ActivityStepKind::ForkAgain => FamilyNodeKind::ActivityFork,
        ActivityStepKind::EndFork => FamilyNodeKind::ActivityForkEnd,
        ActivityStepKind::RepeatStart => FamilyNodeKind::ActivityMerge,
        ActivityStepKind::PartitionStart | ActivityStepKind::PartitionEnd => {
            FamilyNodeKind::ActivityPartition
        }
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
