use super::*;

pub(super) fn normalize_state(document: Document) -> Result<StateDocument, Diagnostic> {
    let mut nodes: Vec<StateNode> = Vec::new();
    let mut transitions: Vec<ModelStateTransition> = Vec::new();
    let mut title = None;
    let mut header = None;
    let mut footer = None;
    let mut caption = None;
    let mut legend = None;
    let mut state_style = StateStyle::default();
    let mut warnings: Vec<Diagnostic> = Vec::new();

    for stmt in &document.statements {
        match &stmt.kind {
            StatementKind::StateDecl(decl) => {
                let node = state_decl_to_node(decl);
                // Collect all transitions nested inside this composite state declaration
                collect_decl_transitions(decl, &mut nodes, &mut transitions);
                upsert_state_node(&mut nodes, node);
            }
            StatementKind::StateTransition(t) => {
                // Ensure endpoints exist as nodes
                ensure_state_node(&mut nodes, &t.from);
                ensure_state_node(&mut nodes, &t.to);
                transitions.push(ModelStateTransition {
                    from: t.from.clone(),
                    to: t.to.clone(),
                    label: t.label.clone(),
                    line_color: t.line_color.clone(),
                    dashed: t.dashed,
                    hidden: t.hidden,
                    thickness: t.thickness,
                    direction: t.direction.clone(),
                });
            }
            StatementKind::StateInternalAction(a) => {
                ensure_state_node(&mut nodes, &a.state);
                // Add internal action to existing node
                if let Some(node) = nodes.iter_mut().find(|n| n.name == a.state) {
                    node.internal_actions.push(ModelStateInternalAction {
                        kind: a.kind.clone(),
                        action: a.action.clone(),
                    });
                }
            }
            StatementKind::StateHistory { deep } => {
                let kind = if *deep {
                    StateNodeKind::HistoryDeep
                } else {
                    StateNodeKind::HistoryShallow
                };
                upsert_state_node(
                    &mut nodes,
                    StateNode {
                        name: if *deep {
                            "[H*]".to_string()
                        } else {
                            "[H]".to_string()
                        },
                        display: Some(if *deep {
                            "H*".to_string()
                        } else {
                            "H".to_string()
                        }),
                        kind,
                        stereotype: None,
                        internal_actions: Vec::new(),
                        regions: Vec::new(),
                    },
                );
            }
            StatementKind::Title(v) => title = Some(v.clone()),
            StatementKind::Header(v) => header = Some(v.clone()),
            StatementKind::Footer(v) => footer = Some(v.clone()),
            StatementKind::Caption(v) => caption = Some(v.clone()),
            StatementKind::Legend(v) => legend = Some(v.clone()),
            StatementKind::SkinParam { key, value } => {
                use crate::theme::StateSkinParamValue;
                match classify_state_skinparam(key, value) {
                    SkinParamSupport::SupportedNoop => {}
                    SkinParamSupport::SupportedWithValue(v) => match v {
                        StateSkinParamValue::BackgroundColor(c) => {
                            state_style.background_color = c;
                        }
                        StateSkinParamValue::BorderColor(c) => {
                            state_style.border_color = c;
                        }
                        StateSkinParamValue::ArrowColor(c) => {
                            state_style.arrow_color = c;
                        }
                        StateSkinParamValue::StartColor(c) => {
                            state_style.start_color = c;
                        }
                        StateSkinParamValue::FontColor(c) => {
                            state_style.font_color = c;
                        }
                        StateSkinParamValue::FontSize(n) => {
                            state_style.font_size = Some(n);
                        }
                    },
                    SkinParamSupport::UnsupportedKey => {
                        warnings.push(
                            Diagnostic::warning(format!(
                                "[W_SKINPARAM_UNSUPPORTED] unsupported skinparam `{}`",
                                key
                            ))
                            .with_span(stmt.span),
                        );
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
            StatementKind::Theme(value) => {
                state_style = state_style_from_sequence_theme(
                    &resolve_sequence_theme_preset(value)
                        .map_err(|msg| Diagnostic::error(msg).with_span(stmt.span))?
                        .style,
                );
            }
            StatementKind::Pragma(_)
            | StatementKind::Include(_)
            | StatementKind::Define { .. }
            | StatementKind::Undef(_) => {}
            StatementKind::StateRegionDivider => {}
            StatementKind::Unknown(line) => {
                return Err(Diagnostic::error(format!(
                    "[E_STATE_UNSUPPORTED_SYNTAX] unsupported state diagram syntax: `{}`",
                    line
                ))
                .with_span(stmt.span));
            }
            _ => {
                return Err(Diagnostic::error(
                    "[E_STATE_MIXED] mixed diagram families are not supported in one document",
                )
                .with_span(stmt.span));
            }
        }
    }

    Ok(StateDocument {
        kind: document.kind,
        nodes,
        transitions,
        title,
        header,
        footer,
        caption,
        legend,
        state_style,
        warnings,
    })
}

/// Recursively collect all `StateTransition` statements nested inside a composite state
/// declaration's children (and their children). These are added to the global transition list
/// so the renderer can route them even though they live inside composite nodes.
/// Also ensures that all referenced endpoint names have a corresponding flat node entry.
fn collect_decl_transitions(
    decl: &crate::ast::StateDecl,
    nodes: &mut Vec<StateNode>,
    transitions: &mut Vec<ModelStateTransition>,
) {
    for child_stmt in &decl.children {
        match &child_stmt.kind {
            StatementKind::StateTransition(t) => {
                ensure_state_node(nodes, &t.from);
                ensure_state_node(nodes, &t.to);
                transitions.push(ModelStateTransition {
                    from: t.from.clone(),
                    to: t.to.clone(),
                    label: t.label.clone(),
                    line_color: t.line_color.clone(),
                    dashed: t.dashed,
                    hidden: t.hidden,
                    thickness: t.thickness,
                    direction: t.direction.clone(),
                });
            }
            StatementKind::StateDecl(child_decl) => {
                // Recurse into nested composite states
                collect_decl_transitions(child_decl, nodes, transitions);
            }
            _ => {}
        }
    }
}

fn state_decl_to_node(decl: &crate::ast::StateDecl) -> StateNode {
    let kind = match decl.stereotype.as_deref() {
        Some("fork") => StateNodeKind::Fork,
        Some("join") => StateNodeKind::Join,
        Some("choice") => StateNodeKind::Choice,
        Some("end") => StateNodeKind::End,
        _ => StateNodeKind::Normal,
    };

    // Parse children into regions separated by region_dividers
    let mut regions: Vec<Vec<StateNode>> = Vec::new();
    let mut current_region: Vec<StateNode> = Vec::new();
    let mut divider_iter = decl.region_dividers.iter().peekable();

    for (child_idx, child_stmt) in decl.children.iter().enumerate() {
        // Check if a divider appears before this child
        while divider_iter.peek() == Some(&&child_idx) {
            divider_iter.next();
            regions.push(std::mem::take(&mut current_region));
        }
        match &child_stmt.kind {
            StatementKind::StateDecl(child_decl) => {
                current_region.push(state_decl_to_node(child_decl));
            }
            StatementKind::StateHistory { deep } => {
                current_region.push(StateNode {
                    name: if *deep {
                        "[H*]".to_string()
                    } else {
                        "[H]".to_string()
                    },
                    display: Some(if *deep {
                        "H*".to_string()
                    } else {
                        "H".to_string()
                    }),
                    kind: if *deep {
                        StateNodeKind::HistoryDeep
                    } else {
                        StateNodeKind::HistoryShallow
                    },
                    stereotype: None,
                    internal_actions: Vec::new(),
                    regions: Vec::new(),
                });
            }
            StatementKind::StateInternalAction(a) => {
                // Apply to parent node's internal actions (will be collected below)
                let _ = a;
            }
            _ => {}
        }
    }
    regions.push(current_region);

    // Collect internal actions from direct children
    let mut internal_actions: Vec<ModelStateInternalAction> = Vec::new();
    for child_stmt in &decl.children {
        if let StatementKind::StateInternalAction(a) = &child_stmt.kind {
            // Only collect actions targeted at this parent state
            if a.state == decl.name {
                internal_actions.push(ModelStateInternalAction {
                    kind: a.kind.clone(),
                    action: a.action.clone(),
                });
            }
        }
    }

    StateNode {
        name: decl.alias.clone().unwrap_or_else(|| decl.name.clone()),
        display: Some(decl.name.clone()),
        kind,
        stereotype: decl.stereotype.clone(),
        internal_actions,
        regions,
    }
}

/// Ensure a state node exists in the list, creating a Normal node if absent.
fn ensure_state_node(nodes: &mut Vec<StateNode>, name: &str) {
    if nodes.iter().any(|n| n.name == name) {
        return;
    }
    let kind = match name {
        "[*]" => StateNodeKind::StartEnd,
        "[H]" => StateNodeKind::HistoryShallow,
        "[H*]" => StateNodeKind::HistoryDeep,
        _ => StateNodeKind::Normal,
    };
    let display = match name {
        "[*]" => None,
        "[H]" => Some("H".to_string()),
        "[H*]" => Some("H*".to_string()),
        _ => None,
    };
    nodes.push(StateNode {
        name: name.to_string(),
        display,
        kind,
        stereotype: None,
        internal_actions: Vec::new(),
        regions: Vec::new(),
    });
}

/// Upsert a state node: if one with the same name already exists, update it; otherwise push.
fn upsert_state_node(nodes: &mut Vec<StateNode>, node: StateNode) {
    if let Some(existing) = nodes.iter_mut().find(|n| n.name == node.name) {
        // Merge: preserve richer kind, regions, internal_actions
        if existing.kind == StateNodeKind::Normal && node.kind != StateNodeKind::Normal {
            existing.kind = node.kind;
        }
        if !node.regions.is_empty() {
            existing.regions = node.regions;
        }
        existing.internal_actions.extend(node.internal_actions);
        if node.stereotype.is_some() && existing.stereotype.is_none() {
            existing.stereotype = node.stereotype;
        }
        if node.display.is_some() && existing.display.is_none() {
            existing.display = node.display;
        }
    } else {
        nodes.push(node);
    }
}
