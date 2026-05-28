use super::*;

pub(super) fn upsert_family_node(nodes: &mut Vec<FamilyNode>, mut node: FamilyNode) {
    let target_name = node.name.as_str();
    let target_alias = node.alias.as_deref();
    if let Some(existing) = nodes.iter_mut().find(|existing| {
        existing.name == target_name
            || existing.alias.as_deref() == Some(target_name)
            || target_alias.is_some_and(|alias| existing.name == alias)
            || (target_alias.is_some() && existing.alias.as_deref() == target_alias)
    }) {
        if existing.label.is_none() {
            existing.label = node.label.take();
        }
        if existing.alias.is_none() {
            existing.alias = node.alias.take();
        }
        if existing.fill_color.is_none() {
            existing.fill_color = node.fill_color.take();
        }
        if matches!(
            (existing.kind, node.kind),
            (FamilyNodeKind::UseCase, FamilyNodeKind::BusinessUseCase)
                | (FamilyNodeKind::Actor, FamilyNodeKind::BusinessActor)
                | (FamilyNodeKind::UseCase, FamilyNodeKind::Actor)
                | (FamilyNodeKind::UseCase, FamilyNodeKind::BusinessActor)
        ) {
            existing.kind = node.kind;
        }
        existing.members.append(&mut node.members);
        return;
    }
    nodes.push(node);
}

pub(super) fn split_object_instance_type(name: String) -> (String, Option<String>) {
    let Some((instance, class_name)) = name.split_once(" : ") else {
        return (name, None);
    };
    let instance = instance.trim();
    let class_name = class_name.trim();
    if instance.is_empty() || class_name.is_empty() || class_name.contains('=') {
        return (name, None);
    }
    (
        instance.to_string(),
        Some(format!("{instance} : {class_name}")),
    )
}

pub(super) fn resolve_usecase_node_kind(members: &mut Vec<ClassMember>) -> FamilyNodeKind {
    let has_actor_marker = members
        .iter()
        .any(|member| member.text.trim().eq_ignore_ascii_case("<<actor>>"));
    let has_business_marker = members
        .iter()
        .any(|member| member.text.trim().eq_ignore_ascii_case("<<business>>"));
    members.retain(|member| {
        let text = member.text.trim();
        !text.eq_ignore_ascii_case("<<actor>>") && !text.eq_ignore_ascii_case("<<business>>")
    });

    match (has_actor_marker, has_business_marker) {
        (true, true) => FamilyNodeKind::BusinessActor,
        (true, false) => FamilyNodeKind::Actor,
        (false, true) => FamilyNodeKind::BusinessUseCase,
        (false, false) => FamilyNodeKind::UseCase,
    }
}

pub(super) fn extract_family_node_fill_color(members: &mut Vec<ClassMember>) -> Option<String> {
    let mut fill_color = None;
    members.retain(|member| {
        let Some(color) = member.text.strip_prefix("\x1fstyle:fill:") else {
            return true;
        };
        if fill_color.is_none() {
            fill_color = Some(color.trim().to_string());
        }
        false
    });
    fill_color
}

pub(super) fn extract_activity_inline_fill(label: &mut Option<String>) -> Option<String> {
    let value = label.take()?;
    let Some(rest) = value.strip_prefix("\x1fstyle:fill:") else {
        *label = Some(value);
        return None;
    };
    let Some((color, display)) = rest.split_once('\x1f') else {
        *label = Some(value);
        return None;
    };
    *label = (!display.is_empty()).then(|| display.to_string());
    Some(color.trim().to_string())
}

pub(super) fn extract_activity_partition_block(label: &mut Option<String>) -> bool {
    let Some(value) = label.take() else {
        return false;
    };
    let Some(display) = value.strip_prefix("\x1factivity:partition:block\x1f") else {
        *label = Some(value);
        return false;
    };
    *label = (!display.is_empty()).then(|| display.to_string());
    true
}

/// Extract a SDL action shape marker (`\x1fsdl:<shape>\x1f<body>`) from the label.
/// Returns the shape name and leaves the display label in `label`.
pub(super) fn extract_activity_sdl_shape(label: &mut Option<String>) -> Option<String> {
    let value = label.take()?;
    let Some(rest) = value.strip_prefix("\x1fsdl:") else {
        *label = Some(value);
        return None;
    };
    let Some((shape, display)) = rest.split_once('\x1f') else {
        *label = Some(value);
        return None;
    };
    *label = (!display.is_empty()).then(|| display.to_string());
    Some(shape.trim().to_string())
}

pub(super) fn extract_activity_note_meta(label: &mut Option<String>) -> Option<(String, bool)> {
    let value = label.take()?;
    let Some(rest) = value.strip_prefix("\x1factivity:note:") else {
        *label = Some(value);
        return None;
    };
    let Some((encoded, display)) = rest.split_once('\x1f') else {
        *label = Some(value);
        return None;
    };
    let mut side = "right".to_string();
    let mut floating = false;
    for part in encoded.split(':') {
        if let Some(value) = part.strip_prefix("side=") {
            side = value.to_string();
        } else if let Some(value) = part.strip_prefix("floating=") {
            floating = value == "1" || value.eq_ignore_ascii_case("true");
        }
    }
    *label = (!display.is_empty()).then(|| display.to_string());
    Some((side, floating))
}

/// Extract swimlane display metadata (`\x1fswim:bold\x1f` / `\x1fswim:stereotype=...\x1f`)
/// from a PartitionStart label and return the clean lane identifier.
///
/// The swim markers carry display-only info (bold header, stereotype) and must
/// be stripped before the label is used as a lane routing identifier.  The
/// returned tuple is `(clean_name, bold, stereotype)`.
pub(super) fn extract_activity_swim_display(label: &mut Option<String>) -> (bool, Option<String>) {
    let Some(value) = label.take() else {
        return (false, None);
    };
    let (clean, bold, stereotype) = puml_parser_activity_extract_swim(&value);
    *label = (!clean.is_empty()).then(|| clean.to_string());
    (bold, stereotype.map(str::to_string))
}

fn puml_parser_activity_extract_swim(value: &str) -> (&str, bool, Option<&str>) {
    let mut rest: &str = value;
    let mut bold = false;
    let mut stereotype: Option<&str> = None;
    loop {
        if let Some(after) = rest.strip_prefix("\x1fswim:bold\x1f") {
            bold = true;
            rest = after;
        } else if let Some(after) = rest.strip_prefix("\x1fswim:stereotype=") {
            if let Some(end) = after.find('\x1f') {
                stereotype = Some(&after[..end]);
                rest = &after[end + 1..];
            } else {
                break;
            }
        } else {
            break;
        }
    }
    (rest, bold, stereotype)
}

pub(super) fn ensure_family_class_node(nodes: &mut Vec<FamilyNode>, name: &str) {
    if name.is_empty()
        || nodes
            .iter()
            .any(|node| node.name == name || node.alias.as_deref() == Some(name))
    {
        return;
    }
    nodes.push(FamilyNode {
        kind: FamilyNodeKind::Class,
        name: name.to_string(),
        alias: None,
        members: Vec::new(),
        depth: 0,
        label: None,
        mindmap_side: MindMapSide::Right,
        wbs_checkbox: None,
        fill_color: None,
    });
}

pub(super) fn component_node_kind(kind: ComponentNodeKind) -> FamilyNodeKind {
    crate::registry::graph_element_for_component_kind(kind)
        .map(|spec| spec.family_node_kind)
        .unwrap_or(FamilyNodeKind::Component)
}

pub(super) fn activity_step_node_kind(kind: &ActivityStepKind) -> FamilyNodeKind {
    match kind {
        ActivityStepKind::Start => FamilyNodeKind::ActivityStart,
        ActivityStepKind::Stop
        | ActivityStepKind::End
        | ActivityStepKind::Kill
        | ActivityStepKind::Detach => FamilyNodeKind::ActivityStop,
        ActivityStepKind::Action => FamilyNodeKind::ActivityAction,
        ActivityStepKind::Connector => FamilyNodeKind::ActivityAction,
        ActivityStepKind::Note => FamilyNodeKind::Note,
        ActivityStepKind::IfStart
        | ActivityStepKind::WhileStart
        | ActivityStepKind::RepeatWhile => FamilyNodeKind::ActivityDecision,
        ActivityStepKind::Else | ActivityStepKind::EndIf | ActivityStepKind::EndWhile => {
            FamilyNodeKind::ActivityMerge
        }
        ActivityStepKind::Fork | ActivityStepKind::ForkAgain => FamilyNodeKind::ActivityFork,
        ActivityStepKind::EndFork => FamilyNodeKind::ActivityForkEnd,
        ActivityStepKind::RepeatStart => FamilyNodeKind::ActivityMerge,
        ActivityStepKind::Arrow
        | ActivityStepKind::PartitionStart
        | ActivityStepKind::PartitionEnd => FamilyNodeKind::ActivityPartition,
    }
}

pub(super) fn timing_decl_node_kind(kind: TimingDeclKind) -> FamilyNodeKind {
    match kind {
        TimingDeclKind::Concise => FamilyNodeKind::TimingConcise,
        TimingDeclKind::Robust => FamilyNodeKind::TimingRobust,
        TimingDeclKind::Clock => FamilyNodeKind::TimingClock,
        TimingDeclKind::Binary => FamilyNodeKind::TimingBinary,
    }
}

pub(super) fn family_note_node(idx: usize, note: crate::ast::Note) -> FamilyNode {
    let mut label = note.text.trim().to_string();
    if label.is_empty() {
        label = "note".to_string();
    }
    let target = note.target.unwrap_or_default();
    FamilyNode {
        kind: FamilyNodeKind::Note,
        name: format!("__note_{idx:04}"),
        alias: Some(format!("note::{}|target={target}", note.position)),
        members: Vec::new(),
        depth: 0,
        label: Some(label),
        mindmap_side: MindMapSide::Right,
        wbs_checkbox: None,
        fill_color: None,
    }
}
