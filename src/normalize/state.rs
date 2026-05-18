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

    // ── Post-process: split [*] into initial pseudostate + final state ──────
    // Per UML spec, the initial pseudostate [*] should have exactly ONE outgoing
    // transition (to the first sub-state). Exit transitions (Foo --> [*]) must
    // terminate at a distinct FinalState node (filled circle with ring), not at
    // the same dot as the initial pseudostate.
    //
    // If [*] is used as BOTH source (initial) and target (final) in this diagram,
    // we create a synthetic node "[*]__end" with kind=End and rewrite all
    // incoming transitions from "[*]" to "[*]__end".
    {
        let star_as_source = transitions.iter().any(|t| t.from == "[*]");
        let star_as_target = transitions.iter().any(|t| t.to == "[*]");
        if star_as_source && star_as_target {
            // Insert the synthetic final-state node
            let final_node = StateNode {
                name: "[*]__end".to_string(),
                display: None,
                kind: StateNodeKind::End,
                stereotype: None,
                internal_actions: Vec::new(),
                regions: Vec::new(),
            };
            nodes.push(final_node);
            // Rewrite all transitions whose target is [*] to point at [*]__end
            for t in transitions.iter_mut() {
                if t.to == "[*]" {
                    t.to = "[*]__end".to_string();
                }
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
///
/// `[*]` references inside a composite are scoped to the parent composite name:
/// - `[*]` as source becomes `[*]__in__{parent}` (initial pseudo-state inside the composite)
/// - `[*]` as target becomes `[*]__end__{parent}` (local final state inside the composite)
/// This prevents internal flow from hijacking the outer diagram's global pseudo-state.
fn collect_decl_transitions(
    decl: &crate::ast::StateDecl,
    nodes: &mut Vec<StateNode>,
    transitions: &mut Vec<ModelStateTransition>,
) {
    // Mirror state_decl_to_node's naming logic.
    let parent_name = decl.alias.as_deref().unwrap_or(decl.name.as_str());

    for child_stmt in &decl.children {
        match &child_stmt.kind {
            StatementKind::StateTransition(t) => {
                let from = scope_pseudo_star(&t.from, parent_name, false);
                let to = scope_pseudo_star(&t.to, parent_name, true);
                ensure_state_node(nodes, &from);
                ensure_state_node(nodes, &to);
                transitions.push(ModelStateTransition {
                    from,
                    to,
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

/// Rewrite `[*]` to a composite-scoped synthetic name.
/// Non-`[*]` names are passed through unchanged.
fn scope_pseudo_star(name: &str, parent: &str, is_target: bool) -> String {
    if name == "[*]" {
        if is_target {
            format!("[*]__end__{parent}")
        } else {
            format!("[*]__in__{parent}")
        }
    } else {
        name.to_string()
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
    let kind = if name == "[*]" {
        StateNodeKind::StartEnd
    } else if name == "[H]" {
        StateNodeKind::HistoryShallow
    } else if name == "[H*]" {
        StateNodeKind::HistoryDeep
    } else if name.starts_with("[*]__in__") {
        StateNodeKind::StartEnd
    } else if name.starts_with("[*]__end__") {
        StateNodeKind::End
    } else {
        StateNodeKind::Normal
    };
    let display = match name {
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
