use super::*;

pub(super) fn parse_scoped_history_endpoint(name: &str) -> Option<(&str, bool)> {
    if let Some(owner) = name.strip_suffix("[H*]") {
        let owner = owner.trim();
        if !owner.is_empty() {
            return Some((owner, true));
        }
    }
    if let Some(owner) = name.strip_suffix("[H]") {
        let owner = owner.trim();
        if !owner.is_empty() {
            return Some((owner, false));
        }
    }
    None
}

pub(super) fn attach_scoped_history_endpoints(
    nodes: &mut [StateNode],
    transitions: &[ModelStateTransition],
) {
    let endpoint_names: std::collections::BTreeSet<String> = transitions
        .iter()
        .flat_map(|t| [t.from.as_str(), t.to.as_str()])
        .filter(|name| parse_scoped_history_endpoint(name).is_some())
        .map(|name| name.to_string())
        .collect();

    for endpoint in endpoint_names {
        let Some((owner_name, _deep)) = parse_scoped_history_endpoint(endpoint.as_str()) else {
            continue;
        };
        let Some(owner_idx) = nodes.iter().position(|node| node.name == owner_name) else {
            continue;
        };
        let owner = &mut nodes[owner_idx];
        let has_regions = owner.regions.iter().any(|region| !region.is_empty());
        if !has_regions {
            continue;
        }
        if owner.regions.is_empty() {
            owner.regions.push(Vec::new());
        }
        ensure_region_state_node(&mut owner.regions[0], endpoint.as_str());
    }
}

/// Recursively collect all `StateTransition` statements nested inside a composite state
/// declaration's children (and their children). These are added to the global transition list
/// so the renderer can route them even though they live inside composite nodes.
/// Also ensures that all referenced endpoint names have a corresponding flat node entry.
///
/// `[*]` references inside a composite are scoped to the parent composite name:
/// - `[*]` as source becomes `[*]__in__{parent}` (initial pseudo-state inside the composite)
/// - `[*]` as target becomes `[*]__end__{parent}` (local final state inside the composite)
///
/// This prevents internal flow from hijacking the outer diagram's global pseudo-state.
pub(super) fn ensure_region_state_node(region: &mut Vec<StateNode>, name: &str) {
    if region.iter().any(|node| node.name == name) {
        return;
    }
    let node = placeholder_state_node(name);
    // Initial pseudo-states go at the top of the region (index 0) so they are
    // placed above all child states in the layout.  Final pseudo-states go at
    // the end (default push).
    if name.starts_with("[*]__in__") {
        region.insert(0, node);
    } else {
        region.push(node);
    }
}

pub(super) fn upsert_region_state_node(region: &mut Vec<StateNode>, node: StateNode) {
    if let Some(existing) = region
        .iter_mut()
        .find(|existing| existing.name == node.name)
    {
        merge_state_node(existing, node);
    } else {
        region.push(node);
    }
}

pub(super) fn merge_state_node(existing: &mut StateNode, node: StateNode) {
    if existing.kind == StateNodeKind::Normal && node.kind != StateNodeKind::Normal {
        existing.kind = node.kind;
    }
    if node.regions.iter().any(|region| !region.is_empty()) {
        existing.regions = node.regions;
    }
    existing.internal_actions.extend(node.internal_actions);
    if node.stereotype.is_some() && existing.stereotype.is_none() {
        existing.stereotype = node.stereotype;
    }
    if node.display.is_some() && existing.display.is_none() {
        existing.display = node.display;
    }
    merge_state_node_style(&mut existing.style, node.style);
}

pub(super) fn merge_state_node_style(existing: &mut StateNodeStyle, incoming: StateNodeStyle) {
    if incoming.fill_color.is_some() && existing.fill_color.is_none() {
        existing.fill_color = incoming.fill_color;
    }
    if incoming.border_color.is_some() && existing.border_color.is_none() {
        existing.border_color = incoming.border_color;
    }
    if incoming.border_dashed {
        existing.border_dashed = true;
    }
    if incoming.border_thickness.is_some() && existing.border_thickness.is_none() {
        existing.border_thickness = incoming.border_thickness;
    }
    if incoming.text_color.is_some() && existing.text_color.is_none() {
        existing.text_color = incoming.text_color;
    }
}

pub(super) fn placeholder_state_node(name: &str) -> StateNode {
    let kind = if name == "[*]" {
        StateNodeKind::StartEnd
    } else if name.ends_with("[H*]") {
        StateNodeKind::HistoryDeep
    } else if name.ends_with("[H]") {
        StateNodeKind::HistoryShallow
    } else if name.starts_with("[*]__in__") {
        StateNodeKind::StartEnd
    } else if name.starts_with("[*]__end__") {
        StateNodeKind::End
    } else {
        StateNodeKind::Normal
    };
    let display = if name.ends_with("[H*]") {
        Some("H*".to_string())
    } else if name.ends_with("[H]") {
        Some("H".to_string())
    } else {
        None
    };
    StateNode {
        name: name.to_string(),
        display,
        kind,
        stereotype: None,
        style: Default::default(),
        internal_actions: Vec::new(),
        regions: Vec::new(),
    }
}

pub(super) fn is_composite_region_endpoint(name: &str, parent_name: &str) -> bool {
    !matches!(name, "[*]" | "[H]" | "[H*]") && name != parent_name
}

pub(super) fn collect_decl_transitions(
    decl: &crate::ast::StateDecl,
    nodes: &mut Vec<StateNode>,
    transitions: &mut Vec<ModelStateTransition>,
) {
    // Mirror state_decl_to_node's naming logic.
    let parent_name = decl.alias.as_deref().unwrap_or(decl.name.as_str());

    // Track the current region index by mirroring state_decl_to_node's divider
    // logic, so that [*] pseudo-state names are region-qualified and match the
    // names produced by state_decl_to_node (needed for consistent node identity).
    let mut region_idx = 0usize;
    let mut divider_iter = decl.region_dividers.iter().peekable();

    for (child_idx, child_stmt) in decl.children.iter().enumerate() {
        while divider_iter.peek() == Some(&&child_idx) {
            divider_iter.next();
            region_idx += 1;
        }
        match &child_stmt.kind {
            StatementKind::StateTransition(t) => {
                let from = scope_pseudo_star_region(&t.from, parent_name, region_idx, false);
                let to = scope_pseudo_star_region(&t.to, parent_name, region_idx, true);
                // Do NOT add composite-scoped [*] pseudo-states to the flat top-level
                // nodes list — they are placed inside the composite region by
                // state_decl_to_node.  Only add genuinely top-level (non-scoped) nodes.
                // Use starts_with("[*]__in__") / starts_with("[*]__end__") instead of
                // contains("__in__") / contains("__end__") to avoid false-positive
                // matches on legitimate user state names that happen to contain those
                // substrings (e.g. a state named "EndStateA" or "inline__end__proc").
                if !from.starts_with("[*]__in__") && !from.starts_with("[*]__end__") {
                    ensure_state_node(nodes, &from);
                }
                if !to.starts_with("[*]__in__") && !to.starts_with("[*]__end__") {
                    ensure_state_node(nodes, &to);
                }
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

/// Rewrite `[*]` to a region-unique composite-scoped synthetic name.
/// Includes the region index so concurrent regions under the same parent
/// produce distinct names (e.g. `[*]__in__Parent__r0`, `[*]__in__Parent__r1`).
/// Non-`[*]` names are passed through unchanged.
pub(super) fn scope_pseudo_star_region(
    name: &str,
    parent: &str,
    region_idx: usize,
    is_target: bool,
) -> String {
    if name == "[*]" {
        if is_target {
            format!("[*]__end__{parent}__r{region_idx}")
        } else {
            format!("[*]__in__{parent}__r{region_idx}")
        }
    } else {
        name.to_string()
    }
}

pub(super) fn state_decl_to_node(decl: &crate::ast::StateDecl) -> StateNode {
    let kind = if decl.name.ends_with("[H*]") {
        StateNodeKind::HistoryDeep
    } else if decl.name.ends_with("[H]") {
        StateNodeKind::HistoryShallow
    } else {
        state_kind_from_stereotype(decl.stereotype.as_deref())
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
                upsert_region_state_node(&mut current_region, state_decl_to_node(child_decl));
            }
            StatementKind::StateHistory { deep } => {
                upsert_region_state_node(
                    &mut current_region,
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
                        kind: if *deep {
                            StateNodeKind::HistoryDeep
                        } else {
                            StateNodeKind::HistoryShallow
                        },
                        stereotype: None,
                        style: Default::default(),
                        internal_actions: Vec::new(),
                        regions: Vec::new(),
                    },
                );
            }
            StatementKind::StateTransition(t) => {
                // The parent name used for scoping pseudo-states matches the logic in
                // collect_decl_transitions (alias takes precedence over name).
                let parent_name = decl.alias.as_deref().unwrap_or(decl.name.as_str());
                // Include the region index so concurrent regions under the same parent
                // get distinct synthetic names for their [*] pseudo-state anchors.
                // Without this, all regions share the same scoped name (e.g.
                // "[*]__in__Parent"), causing later regions to overwrite the placement
                // of earlier ones and transitions to render from the wrong location.
                let region_idx = regions.len();
                for (endpoint, is_target) in [(&t.from, false), (&t.to, true)] {
                    if endpoint == "[*]" {
                        // Add the composite-scoped [*] pseudo-node to the region so it
                        // is rendered inside the composite box, not at the top level.
                        let scoped =
                            scope_pseudo_star_region(endpoint, parent_name, region_idx, is_target);
                        ensure_region_state_node(&mut current_region, &scoped);
                    } else if is_composite_region_endpoint(endpoint, decl.name.as_str()) {
                        ensure_region_state_node(&mut current_region, endpoint);
                    }
                }
            }
            StatementKind::StateInternalAction(a) => {
                // Apply to parent node's internal actions (will be collected below)
                let _ = a;
            }
            StatementKind::Note(note) => {
                let note_name = format!("__state_note_in_{}_{child_idx:04}", decl.name);
                upsert_region_state_node(
                    &mut current_region,
                    StateNode {
                        name: note_name,
                        display: Some(if note.text.trim().is_empty() {
                            "note".to_string()
                        } else {
                            note.text.trim().to_string()
                        }),
                        kind: StateNodeKind::Note,
                        stereotype: Some(note.position.clone()),
                        style: Default::default(),
                        internal_actions: Vec::new(),
                        regions: Vec::new(),
                    },
                );
            }
            StatementKind::JsonProjection { alias, body } => {
                let projection_name = format!("__state_json_in_{}_{child_idx:04}", decl.name);
                upsert_region_state_node(
                    &mut current_region,
                    StateNode {
                        name: projection_name,
                        display: Some(format!("{}\n{}", alias.trim(), body.trim())),
                        kind: StateNodeKind::JsonProjection,
                        stereotype: Some("json".to_string()),
                        style: Default::default(),
                        internal_actions: Vec::new(),
                        regions: Vec::new(),
                    },
                );
            }
            StatementKind::YamlProjection { alias, body } => {
                let projection_name = format!("__state_yaml_in_{}_{child_idx:04}", decl.name);
                upsert_region_state_node(
                    &mut current_region,
                    StateNode {
                        name: projection_name,
                        display: Some(format!("{}\n{}", alias.trim(), body.trim())),
                        kind: StateNodeKind::JsonProjection,
                        stereotype: Some("yaml".to_string()),
                        style: Default::default(),
                        internal_actions: Vec::new(),
                        regions: Vec::new(),
                    },
                );
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
        display: state_decl_display(decl),
        kind,
        stereotype: decl.stereotype.clone(),
        style: state_style_from_decl(&decl.style),
        internal_actions,
        regions,
    }
}

pub(super) fn state_decl_display(decl: &crate::ast::StateDecl) -> Option<String> {
    if decl.name.ends_with("[H*]") {
        Some("H*".to_string())
    } else if decl.name.ends_with("[H]") {
        Some("H".to_string())
    } else {
        Some(decl.name.clone())
    }
}

pub(super) fn state_kind_from_stereotype(stereotype: Option<&str>) -> StateNodeKind {
    match stereotype.unwrap_or_default().to_ascii_lowercase().as_str() {
        "start" => StateNodeKind::StartEnd,
        "fork" => StateNodeKind::Fork,
        "join" => StateNodeKind::Join,
        "choice" => StateNodeKind::Choice,
        "end" => StateNodeKind::End,
        "history" => StateNodeKind::HistoryShallow,
        "history*" => StateNodeKind::HistoryDeep,
        "entrypoint" => StateNodeKind::EntryPoint,
        "exitpoint" => StateNodeKind::ExitPoint,
        "inputpin" => StateNodeKind::InputPin,
        "outputpin" => StateNodeKind::OutputPin,
        "expansioninput" => StateNodeKind::ExpansionInput,
        "expansionoutput" => StateNodeKind::ExpansionOutput,
        "terminate" => StateNodeKind::Terminate,
        "sdlreceive" => StateNodeKind::SdlReceive,
        "sdlsend" => StateNodeKind::SdlSend,
        _ => StateNodeKind::Normal,
    }
}

pub(super) fn state_style_from_decl(style: &crate::ast::StateDeclStyle) -> StateNodeStyle {
    StateNodeStyle {
        fill_color: style.fill_color.clone(),
        border_color: style.border_color.clone(),
        border_dashed: style.border_dashed,
        border_thickness: style.border_thickness,
        text_color: style.text_color.clone(),
    }
}

/// Ensure a state node exists in the list, creating a Normal node if absent.
pub(super) fn ensure_state_node(nodes: &mut Vec<StateNode>, name: &str) {
    if nodes.iter().any(|n| n.name == name) {
        return;
    }
    nodes.push(placeholder_state_node(name));
}

/// Upsert a state node: if one with the same name already exists, update it; otherwise push.
pub(super) fn upsert_state_node(nodes: &mut Vec<StateNode>, node: StateNode) {
    if let Some(existing) = nodes.iter_mut().find(|n| n.name == node.name) {
        merge_state_node(existing, node);
    } else {
        nodes.push(node);
    }
}
