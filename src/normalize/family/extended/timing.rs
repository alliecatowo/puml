use super::*;

#[derive(Default)]
pub(super) struct TimingNormalizeState {
    pub(super) current_time: Option<String>,
    current_signal: Option<String>,
    anchors: std::collections::BTreeMap<String, String>,
    clock_scales: std::collections::BTreeMap<String, (i64, i64)>,
}

pub(super) fn normalize_timing_decl(
    nodes: &mut Vec<FamilyNode>,
    state: &mut TimingNormalizeState,
    kind: TimingDeclKind,
    name: String,
    label: Option<String>,
    controls: Vec<String>,
) {
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
            state
                .clock_scales
                .insert(name.clone(), (period, offset.unwrap_or(0)));
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

pub(super) fn normalize_timing_event(
    nodes: &mut Vec<FamilyNode>,
    state: &mut TimingNormalizeState,
    time: String,
    signal: Option<String>,
    event_state: Option<String>,
    note: Option<String>,
) {
    if event_state.is_none()
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
                if let Some(sig_node) = nodes
                    .iter_mut()
                    .find(|n| matches!(n.kind, FamilyNodeKind::TimingRobust) && n.name == *sig_name)
                {
                    sig_node.members.push(crate::ast::ClassMember {
                        text: format!("__timing:order:{order_payload}"),
                        modifier: None,
                    });
                }
            }
        }
        return;
    }
    if signal.is_none()
        && event_state.is_none()
        && note.as_deref().is_some_and(|n| n.starts_with("__timing:"))
    {
        let normalized_time = normalize_timing_time(
            &time,
            state.current_time.as_deref(),
            &state.anchors,
            &state.clock_scales,
        );
        if let Some(anchor) = note
            .as_deref()
            .and_then(|n| n.strip_prefix("__timing:anchor:"))
        {
            if !normalized_time.is_empty() {
                state
                    .anchors
                    .insert(anchor.to_string(), normalized_time.clone());
                state.current_time = Some(normalized_time);
            }
            return;
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
        return;
    }

    let mut signal = signal;
    if signal.is_none()
        && event_state.is_none()
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
        state.current_signal = Some(time);
        return;
    }
    if signal.is_none() && event_state.is_some() {
        signal = state.current_signal.clone();
    }
    let effective_time = if time.is_empty() || signal.as_deref() == Some(time.as_str()) {
        state.current_time.clone().unwrap_or_default()
    } else {
        let normalized_time = normalize_timing_time(
            &time,
            state.current_time.as_deref(),
            &state.anchors,
            &state.clock_scales,
        );
        state.current_time = Some(normalized_time.clone());
        normalized_time
    };
    let note = note.map(|note| {
        normalize_timing_range_note(
            &note,
            state.current_time.as_deref(),
            &state.anchors,
            &state.clock_scales,
        )
    });
    let display = match (&signal, &event_state, &note) {
        (Some(s), Some(st), _) => format!("{s} is {st}"),
        (None, None, Some(n)) => n.clone(),
        _ => String::new(),
    };
    nodes.push(FamilyNode {
        kind: FamilyNodeKind::TimingEvent,
        name: effective_time,
        alias: signal,
        members: event_state
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

pub(super) fn normalize_timing_relation_endpoint(
    raw: &str,
    state: &TimingNormalizeState,
) -> String {
    normalize_timing_endpoint(
        raw,
        state.current_time.as_deref(),
        &state.anchors,
        &state.clock_scales,
    )
}

pub(super) fn normalize_timing_scale_node(nodes: &mut Vec<FamilyNode>, body: String) {
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
