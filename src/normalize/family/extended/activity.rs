use super::*;

#[derive(Default)]
pub(super) struct ActivityNormalizeState {
    pub(super) step_counter: usize,
    pub(super) active_partition: Option<String>,
    pub(super) partition_stack: Vec<Option<String>>,
    pub(super) fork_depth: usize,
    pub(super) fork_branch: usize,
}

pub(super) fn normalize_activity_step(
    nodes: &mut Vec<FamilyNode>,
    state: &mut ActivityNormalizeState,
    step: crate::ast::ActivityStep,
) {
    state.step_counter += 1;
    let kind = activity_step_node_kind(&step.kind);
    let name = format!("__act_{:04}", state.step_counter);
    let mut label = step.label;
    let fill_color = extract_activity_inline_fill(&mut label);
    let partition_block = extract_activity_partition_block(&mut label);
    let sdl_shape = extract_activity_sdl_shape(&mut label);
    let note_meta = extract_activity_note_meta(&mut label);
    let is_activity_note_step = matches!(step.kind, ActivityStepKind::Note);
    match step.kind {
        ActivityStepKind::PartitionStart => {
            if partition_block {
                state.partition_stack.push(state.active_partition.clone());
            }
            state.active_partition = label.clone();
        }
        ActivityStepKind::PartitionEnd => {
            state.active_partition = state.partition_stack.pop().flatten();
        }
        ActivityStepKind::Fork => {
            state.fork_depth += 1;
            state.fork_branch = 0;
        }
        ActivityStepKind::ForkAgain => {
            state.fork_branch += 1;
        }
        ActivityStepKind::EndFork => {
            state.fork_depth = state.fork_depth.saturating_sub(1);
            state.fork_branch = 0;
        }
        _ => {}
    }
    let lane = if is_activity_note_step {
        "default".to_string()
    } else {
        state
            .active_partition
            .clone()
            .unwrap_or_else(|| "default".to_string())
    };
    let mut alias_parts = vec![
        format!("activity::{:?}", step.kind),
        format!("lane={lane}"),
        format!("fork_depth={}", state.fork_depth),
        format!("fork_branch={}", state.fork_branch),
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

pub(super) fn normalize_activity_note(
    nodes: &mut Vec<FamilyNode>,
    state: &mut ActivityNormalizeState,
    note: crate::ast::Note,
) {
    state.step_counter += 1;
    let position = note.position.trim().to_string();
    let text = note.text.trim();
    let label = if text.is_empty() {
        "note".to_string()
    } else {
        text.to_string()
    };
    nodes.push(FamilyNode {
        kind: FamilyNodeKind::Note,
        name: format!("__act_{:04}", state.step_counter),
        alias: Some(format!(
            "activity::Note|position={}|note_side={}|lane={}|fork_depth={}|fork_branch={}",
            position, position, "default", state.fork_depth, state.fork_branch
        )),
        members: Vec::new(),
        depth: 0,
        label: Some(label),
        mindmap_side: MindMapSide::Right,
        wbs_checkbox: None,
        fill_color: None,
    });
}

pub(super) fn normalize_activity_unknown_line(
    nodes: &mut Vec<FamilyNode>,
    state: &mut ActivityNormalizeState,
    line: &str,
) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return true;
    }
    state.step_counter += 1;
    if trimmed.starts_with('|') && trimmed.ends_with('|') && trimmed.len() > 2 {
        state.active_partition = Some(trimmed.trim_matches('|').trim().to_string());
    }
    let lane = state
        .active_partition
        .clone()
        .unwrap_or_else(|| "default".to_string());
    nodes.push(FamilyNode {
        kind: if trimmed.starts_with('|') && trimmed.ends_with('|') {
            FamilyNodeKind::ActivityPartition
        } else {
            FamilyNodeKind::ActivityAction
        },
        name: format!("__act_{:04}", state.step_counter),
        alias: Some(format!(
            "activity::OldStyle|lane={lane}|fork_depth={}|fork_branch={}",
            state.fork_depth, state.fork_branch
        )),
        members: Vec::new(),
        depth: 0,
        label: Some(trimmed.to_string()),
        mindmap_side: MindMapSide::Right,
        wbs_checkbox: None,
        fill_color: None,
    });
    true
}
