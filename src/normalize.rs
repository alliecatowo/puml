use std::collections::BTreeMap;

use crate::ast::{DiagramKind, Document, ParticipantRole as AstRole, StatementKind};
use crate::diagnostic::Diagnostic;
use crate::model::{
    ArchimateDocument, ArchimateElement, ArchimateLayer, ArchimateRelation, FamilyDocument,
    FamilyNode, FamilyNodeKind, FamilyRelation as ModelFamilyRelation, JsonDocument, JsonNode,
    JsonValueType, NormalizedDocument, NwdiagDocument, NwdiagNetwork, NwdiagNode, Participant,
    ParticipantRole, SaltCell as ModelSaltCell, SaltDocument, SaltRow, SequenceDocument,
    SequenceEvent, SequenceEventKind, SequencePage, StateDocument,
    StateInternalAction as ModelStateInternalAction, StateNode, StateNodeKind,
    StateTransition as ModelStateTransition, TimelineChronologyEvent, TimelineConstraint,
    TimelineDocument, TimelineMilestone, TimelineTask, VirtualEndpoint, VirtualEndpointKind,
    VirtualEndpointSide, YamlDocument, YamlNode, YamlValueType,
};
use crate::theme::{
    classify_sequence_skinparam, resolve_sequence_theme_preset, SequenceSkinParamSupport,
    SequenceSkinParamValue, SequenceStyle,
};

#[derive(Debug, Clone, Default)]
pub struct NormalizeOptions {
    pub include_root: Option<std::path::PathBuf>,
}

pub fn normalize(document: Document) -> Result<SequenceDocument, Diagnostic> {
    normalize_with_options(document, &NormalizeOptions::default())
}

pub fn normalize_family(document: Document) -> Result<NormalizedDocument, Diagnostic> {
    normalize_family_with_options(document, &NormalizeOptions::default())
}

pub fn normalize_family_with_options(
    document: Document,
    options: &NormalizeOptions,
) -> Result<NormalizedDocument, Diagnostic> {
    match document.kind {
        DiagramKind::Sequence => {
            normalize_with_options(document, options).map(NormalizedDocument::Sequence)
        }
        DiagramKind::Class | DiagramKind::Object | DiagramKind::UseCase => {
            normalize_stub_family(document).map(NormalizedDocument::Family)
        }
        DiagramKind::Gantt | DiagramKind::Chronology | DiagramKind::Activity => {
            normalize_timeline_baseline(document).map(NormalizedDocument::Timeline)
        }
        DiagramKind::State => normalize_state(document).map(NormalizedDocument::State),
        DiagramKind::Salt => normalize_salt(document).map(NormalizedDocument::Salt),
        DiagramKind::Json => normalize_json(document).map(NormalizedDocument::Json),
        DiagramKind::Yaml => normalize_yaml(document).map(NormalizedDocument::Yaml),
        DiagramKind::Nwdiag => normalize_nwdiag(document).map(NormalizedDocument::Nwdiag),
        DiagramKind::Archimate => {
            normalize_archimate(document).map(NormalizedDocument::Archimate)
        }
        DiagramKind::MindMap
        | DiagramKind::Wbs
        | DiagramKind::Component
        | DiagramKind::Deployment
        | DiagramKind::Timing => Err(unsupported_family_diagnostic(document.kind)),
        DiagramKind::Unknown => Err(Diagnostic::error(
            "[E_FAMILY_UNKNOWN] unable to detect supported diagram family; expected sequence/class/object/usecase/gantt/chronology syntax",
        )),
    }
}

fn normalize_timeline_baseline(document: Document) -> Result<TimelineDocument, Diagnostic> {
    let mut tasks = Vec::new();
    let mut milestones = Vec::new();
    let mut constraints = Vec::new();
    let mut chronology_events = Vec::new();
    let mut title = None;
    let mut header = None;
    let mut footer = None;
    let mut caption = None;
    let mut legend = None;

    for stmt in document.statements {
        match stmt.kind {
            StatementKind::GanttTaskDecl { name } => tasks.push(TimelineTask { name }),
            StatementKind::GanttMilestoneDecl { name } => {
                milestones.push(TimelineMilestone { name })
            }
            StatementKind::GanttConstraint {
                subject,
                kind,
                target,
            } => constraints.push(TimelineConstraint {
                subject,
                kind,
                target,
            }),
            StatementKind::ChronologyHappensOn { subject, when } => {
                chronology_events.push(TimelineChronologyEvent { subject, when })
            }
            StatementKind::Title(v) => title = Some(v),
            StatementKind::Header(v) => header = Some(v),
            StatementKind::Footer(v) => footer = Some(v),
            StatementKind::Caption(v) => caption = Some(v),
            StatementKind::Legend(v) => legend = Some(v),
            StatementKind::SkinParam { .. }
            | StatementKind::Theme(_)
            | StatementKind::Pragma(_) => {}
            StatementKind::Unknown(line) => {
                return Err(Diagnostic::error(line).with_span(stmt.span));
            }
            _ => {
                let family = family_kind_name(document.kind);
                return Err(Diagnostic::error(format!(
                    "[E_TIMELINE_BASELINE_UNSUPPORTED] unsupported {family} syntax in baseline slice"
                ))
                .with_span(stmt.span));
            }
        }
    }

    Ok(TimelineDocument {
        kind: document.kind,
        tasks,
        milestones,
        constraints,
        chronology_events,
        title,
        header,
        footer,
        caption,
        legend,
        warnings: Vec::new(),
    })
}

fn normalize_stub_family(document: Document) -> Result<FamilyDocument, Diagnostic> {
    let family_kind = document.kind;
    let node_kind = match family_kind {
        DiagramKind::Class => FamilyNodeKind::Class,
        DiagramKind::Object => FamilyNodeKind::Object,
        DiagramKind::UseCase => FamilyNodeKind::UseCase,
        _ => {
            return Err(Diagnostic::error(
                "[E_FAMILY_STUB_INTERNAL] invalid family for stub normalization",
            ));
        }
    };

    let mut nodes = Vec::new();
    let mut relations = Vec::new();
    let mut title = None;
    let mut header = None;
    let mut footer = None;
    let mut caption = None;
    let mut legend = None;

    for stmt in document.statements {
        match stmt.kind {
            StatementKind::ClassDecl(decl) => {
                if node_kind != FamilyNodeKind::Class {
                    return Err(Diagnostic::error(format!(
                        "[E_FAMILY_MIXED] mixed diagram families are not supported: found class declaration in {} diagram",
                        family_kind_name(family_kind)
                    ))
                    .with_span(stmt.span));
                }
                nodes.push(FamilyNode {
                    kind: FamilyNodeKind::Class,
                    name: decl.name,
                    alias: decl.alias,
                    members: decl.members,
                });
            }
            StatementKind::ObjectDecl(decl) => {
                if node_kind != FamilyNodeKind::Object {
                    return Err(Diagnostic::error(format!(
                        "[E_FAMILY_MIXED] mixed diagram families are not supported: found object declaration in {} diagram",
                        family_kind_name(family_kind)
                    ))
                    .with_span(stmt.span));
                }
                nodes.push(FamilyNode {
                    kind: FamilyNodeKind::Object,
                    name: decl.name,
                    alias: decl.alias,
                    members: decl.members,
                });
            }
            StatementKind::UseCaseDecl(decl) => {
                if node_kind != FamilyNodeKind::UseCase {
                    return Err(Diagnostic::error(format!(
                        "[E_FAMILY_MIXED] mixed diagram families are not supported: found usecase declaration in {} diagram",
                        family_kind_name(family_kind)
                    ))
                    .with_span(stmt.span));
                }
                nodes.push(FamilyNode {
                    kind: FamilyNodeKind::UseCase,
                    name: decl.name,
                    alias: decl.alias,
                    members: decl.members,
                });
            }
            StatementKind::FamilyRelation(rel) => relations.push(ModelFamilyRelation {
                from: rel.from,
                to: rel.to,
                arrow: rel.arrow,
                label: rel.label,
            }),
            StatementKind::Title(v) => title = Some(v),
            StatementKind::Header(v) => header = Some(v),
            StatementKind::Footer(v) => footer = Some(v),
            StatementKind::Caption(v) => caption = Some(v),
            StatementKind::Legend(v) => legend = Some(v),
            StatementKind::SkinParam { .. }
            | StatementKind::Theme(_)
            | StatementKind::Pragma(_)
            | StatementKind::Include(_)
            | StatementKind::Define { .. }
            | StatementKind::Undef(_) => {}
            StatementKind::Unknown(line) => {
                return Err(Diagnostic::error(format!(
                    "[E_PARSE_UNKNOWN] unsupported syntax: `{}`",
                    line
                ))
                .with_span(stmt.span));
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

    Ok(FamilyDocument {
        kind: family_kind,
        nodes,
        relations,
        title,
        header,
        footer,
        caption,
        legend,
        warnings: Vec::new(),
    })
}

fn normalize_state(document: Document) -> Result<StateDocument, Diagnostic> {
    let mut nodes: Vec<StateNode> = Vec::new();
    let mut transitions: Vec<ModelStateTransition> = Vec::new();
    let mut title = None;
    let mut header = None;
    let mut footer = None;
    let mut caption = None;
    let mut legend = None;

    for stmt in &document.statements {
        match &stmt.kind {
            StatementKind::StateDecl(decl) => {
                let node = state_decl_to_node(decl);
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
            StatementKind::SkinParam { .. }
            | StatementKind::Theme(_)
            | StatementKind::Pragma(_)
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
        warnings: Vec::new(),
    })
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
        if node.display.is_some() && existing.display.is_none() {
            existing.display = node.display;
        }
    } else {
        nodes.push(node);
    }
}

fn family_kind_name(kind: DiagramKind) -> &'static str {
    match kind {
        DiagramKind::Sequence => "sequence",
        DiagramKind::Class => "class",
        DiagramKind::Object => "object",
        DiagramKind::UseCase => "usecase",
        DiagramKind::Salt => "salt",
        DiagramKind::MindMap => "mindmap",
        DiagramKind::Wbs => "wbs",
        DiagramKind::Gantt => "gantt",
        DiagramKind::Chronology => "chronology",
        DiagramKind::Component => "component",
        DiagramKind::Deployment => "deployment",
        DiagramKind::State => "state",
        DiagramKind::Activity => "activity",
        DiagramKind::Timing => "timing",
        DiagramKind::Json => "json",
        DiagramKind::Yaml => "yaml",
        DiagramKind::Nwdiag => "nwdiag",
        DiagramKind::Archimate => "archimate",
        DiagramKind::Unknown => "unknown",
    }
}

// ─── Salt normalizer ──────────────────────────────────────────────────────────

fn normalize_salt(document: Document) -> Result<SaltDocument, Diagnostic> {
    let mut rows = Vec::new();
    let mut title = None;
    for stmt in document.statements {
        match stmt.kind {
            StatementKind::SaltGridRow { cells } => {
                let model_cells = cells
                    .into_iter()
                    .map(|c| match c {
                        crate::ast::SaltCell::Label(s) => ModelSaltCell::Label(s),
                        crate::ast::SaltCell::Input(s) => ModelSaltCell::Input(s),
                        crate::ast::SaltCell::Button(s) => ModelSaltCell::Button(s),
                        crate::ast::SaltCell::Combo(s) => ModelSaltCell::Combo(s),
                        crate::ast::SaltCell::CheckboxChecked(s) => {
                            ModelSaltCell::CheckboxChecked(s)
                        }
                        crate::ast::SaltCell::CheckboxUnchecked(s) => {
                            ModelSaltCell::CheckboxUnchecked(s)
                        }
                        crate::ast::SaltCell::RadioSelected(s) => ModelSaltCell::RadioSelected(s),
                        crate::ast::SaltCell::RadioUnselected(s) => {
                            ModelSaltCell::RadioUnselected(s)
                        }
                    })
                    .collect();
                rows.push(SaltRow { cells: model_cells });
            }
            StatementKind::Title(v) => title = Some(v),
            StatementKind::SkinParam { .. }
            | StatementKind::Theme(_)
            | StatementKind::Pragma(_)
            | StatementKind::Include(_)
            | StatementKind::Define { .. }
            | StatementKind::Undef(_) => {}
            _ => {}
        }
    }
    Ok(SaltDocument {
        rows,
        title,
        warnings: Vec::new(),
    })
}

// ─── JSON normalizer ──────────────────────────────────────────────────────────

fn normalize_json(document: Document) -> Result<JsonDocument, Diagnostic> {
    let mut raw = String::new();
    let mut title = None;
    for stmt in document.statements {
        match stmt.kind {
            StatementKind::RawBody(line) => {
                raw.push_str(&line);
                raw.push('\n');
            }
            StatementKind::Title(v) => title = Some(v),
            _ => {}
        }
    }
    let nodes = match serde_json::from_str::<serde_json::Value>(raw.trim()) {
        Ok(value) => {
            let mut out = Vec::new();
            flatten_json_value(&value, None, 0, &mut out);
            out
        }
        Err(_) => raw
            .lines()
            .map(|line| JsonNode {
                depth: 0,
                key: None,
                value_type: JsonValueType::String,
                display: line.trim_end().to_string(),
                has_children: false,
            })
            .collect(),
    };
    Ok(JsonDocument {
        nodes,
        title,
        warnings: Vec::new(),
    })
}

fn flatten_json_value(
    value: &serde_json::Value,
    label: Option<&str>,
    depth: usize,
    out: &mut Vec<JsonNode>,
) {
    use serde_json::Value;
    match value {
        Value::Object(map) => {
            let display = label
                .map(|l| format!("{l}: {{"))
                .unwrap_or_else(|| "{".to_string());
            out.push(JsonNode {
                depth,
                key: label.map(|s| s.to_string()),
                value_type: JsonValueType::Object,
                display,
                has_children: !map.is_empty(),
            });
            for (k, v) in map {
                flatten_json_value(v, Some(k), depth + 1, out);
            }
            out.push(JsonNode {
                depth,
                key: None,
                value_type: JsonValueType::Object,
                display: "}".to_string(),
                has_children: false,
            });
        }
        Value::Array(items) => {
            let display = label
                .map(|l| format!("{l}: ["))
                .unwrap_or_else(|| "[".to_string());
            out.push(JsonNode {
                depth,
                key: label.map(|s| s.to_string()),
                value_type: JsonValueType::Array,
                display,
                has_children: !items.is_empty(),
            });
            for (i, v) in items.iter().enumerate() {
                let key = format!("[{i}]");
                flatten_json_value(v, Some(&key), depth + 1, out);
            }
            out.push(JsonNode {
                depth,
                key: None,
                value_type: JsonValueType::Array,
                display: "]".to_string(),
                has_children: false,
            });
        }
        Value::String(s) => {
            let display = label
                .map(|l| format!("{l}: \"{s}\""))
                .unwrap_or_else(|| format!("\"{s}\""));
            out.push(JsonNode {
                depth,
                key: label.map(|s| s.to_string()),
                value_type: JsonValueType::String,
                display,
                has_children: false,
            });
        }
        Value::Number(n) => {
            let display = label
                .map(|l| format!("{l}: {n}"))
                .unwrap_or_else(|| n.to_string());
            out.push(JsonNode {
                depth,
                key: label.map(|s| s.to_string()),
                value_type: JsonValueType::Number,
                display,
                has_children: false,
            });
        }
        Value::Bool(b) => {
            let display = label
                .map(|l| format!("{l}: {b}"))
                .unwrap_or_else(|| b.to_string());
            out.push(JsonNode {
                depth,
                key: label.map(|s| s.to_string()),
                value_type: JsonValueType::Bool,
                display,
                has_children: false,
            });
        }
        Value::Null => {
            let display = label
                .map(|l| format!("{l}: null"))
                .unwrap_or_else(|| "null".to_string());
            out.push(JsonNode {
                depth,
                key: label.map(|s| s.to_string()),
                value_type: JsonValueType::Null,
                display,
                has_children: false,
            });
        }
    }
}

// ─── YAML normalizer ──────────────────────────────────────────────────────────

fn normalize_yaml(document: Document) -> Result<YamlDocument, Diagnostic> {
    let mut raw = String::new();
    let mut title = None;
    for stmt in document.statements {
        match stmt.kind {
            StatementKind::RawBody(line) => {
                raw.push_str(&line);
                raw.push('\n');
            }
            StatementKind::Title(v) => title = Some(v),
            _ => {}
        }
    }
    let mut nodes = Vec::new();
    for line in raw.lines() {
        let trimmed_end = line.trim_end();
        if trimmed_end.is_empty() || trimmed_end.trim_start().starts_with('#') {
            continue;
        }
        let indent = trimmed_end.len() - trimmed_end.trim_start().len();
        let depth = indent / 2;
        let content = trimmed_end.trim_start();
        let (label, value_type) = classify_yaml_line(content);
        nodes.push(YamlNode {
            depth,
            label,
            value_type,
        });
    }
    Ok(YamlDocument {
        nodes,
        title,
        warnings: Vec::new(),
    })
}

fn classify_yaml_line(content: &str) -> (String, YamlValueType) {
    if let Some((key, value)) = content.split_once(':') {
        let value = value.trim();
        if value.is_empty() {
            return (key.trim().to_string(), YamlValueType::Key);
        }
        let vtype = if value.starts_with('"') || value.starts_with('\'') {
            YamlValueType::StringValue
        } else if matches!(value, "true" | "false" | "yes" | "no") {
            YamlValueType::BoolValue
        } else if matches!(value, "null" | "~") {
            YamlValueType::NullValue
        } else if value.parse::<f64>().is_ok() {
            YamlValueType::NumberValue
        } else {
            YamlValueType::StringValue
        };
        (format!("{}: {}", key.trim(), value), vtype)
    } else if content.starts_with("- ") {
        (content.to_string(), YamlValueType::StringValue)
    } else {
        (content.to_string(), YamlValueType::Unknown)
    }
}

// ─── nwdiag normalizer ────────────────────────────────────────────────────────

fn normalize_nwdiag(document: Document) -> Result<NwdiagDocument, Diagnostic> {
    let mut raw = String::new();
    let mut title = None;
    for stmt in document.statements {
        match stmt.kind {
            StatementKind::RawBody(line) => {
                raw.push_str(&line);
                raw.push('\n');
            }
            StatementKind::Title(v) => title = Some(v),
            _ => {}
        }
    }
    let mut networks: Vec<NwdiagNetwork> = Vec::new();
    let mut current: Option<NwdiagNetwork> = None;
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("//") {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("network ") {
            if let Some(net) = current.take() {
                networks.push(net);
            }
            let name = rest.split('{').next().unwrap_or(rest).trim().to_string();
            current = Some(NwdiagNetwork {
                name,
                address: None,
                nodes: Vec::new(),
            });
            continue;
        }
        if trimmed == "}" {
            if let Some(net) = current.take() {
                networks.push(net);
            }
            continue;
        }
        if let Some(net) = current.as_mut() {
            if let Some(rest) = trimmed.strip_prefix("address") {
                let value = rest
                    .trim_start_matches([' ', '='])
                    .trim()
                    .trim_matches('"')
                    .to_string();
                net.address = Some(value);
                continue;
            }
            // NodeName [address = "..."] or NodeName
            let (name_part, attrs) = match trimmed.split_once('[') {
                Some((n, rest)) => (n.trim().to_string(), Some(rest.trim_end_matches(']'))),
                None => {
                    // bare name, possibly with trailing semicolon
                    let n = trimmed.trim_end_matches(';').trim().to_string();
                    (n, None)
                }
            };
            if name_part.is_empty() {
                continue;
            }
            let mut node_address: Option<String> = None;
            if let Some(attrs) = attrs {
                for kv in attrs.split(',') {
                    if let Some((k, v)) = kv.split_once('=') {
                        if k.trim() == "address" {
                            node_address = Some(v.trim().trim_matches('"').to_string());
                        }
                    }
                }
            }
            // Avoid duplicating nodes in the same network
            if !net.nodes.iter().any(|n| n.name == name_part) {
                net.nodes.push(NwdiagNode {
                    name: name_part,
                    address: node_address,
                });
            }
        }
    }
    if let Some(net) = current.take() {
        networks.push(net);
    }
    Ok(NwdiagDocument {
        networks,
        title,
        warnings: Vec::new(),
    })
}

// ─── Archimate normalizer ─────────────────────────────────────────────────────

fn normalize_archimate(document: Document) -> Result<ArchimateDocument, Diagnostic> {
    let mut raw = String::new();
    let mut title = None;
    for stmt in document.statements {
        match stmt.kind {
            StatementKind::RawBody(line) => {
                raw.push_str(&line);
                raw.push('\n');
            }
            StatementKind::Title(v) => title = Some(v),
            _ => {}
        }
    }
    let mut elements: Vec<ArchimateElement> = Vec::new();
    let mut relations: Vec<ArchimateRelation> = Vec::new();

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('\'') {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("archimate ") {
            if let Some(elem) = parse_archimate_element(rest) {
                elements.push(elem);
                continue;
            }
        }
        // Relation macros: Rel_Association(a, b, "label"), Rel_Realization(a, b)
        if let Some(open) = trimmed.find('(') {
            let macro_name = trimmed[..open].trim();
            if let Some(kind) = archimate_rel_kind_from_macro(macro_name) {
                let inside = trimmed[open + 1..].trim_end_matches([')', ' ', '\t']);
                let args = split_csv_args(inside);
                if args.len() >= 2 {
                    let from = args[0].trim().trim_matches('"').to_string();
                    let to = args[1].trim().trim_matches('"').to_string();
                    let label = args
                        .get(2)
                        .map(|s| s.trim().trim_matches('"').to_string())
                        .filter(|s| !s.is_empty());
                    relations.push(ArchimateRelation {
                        from,
                        to,
                        kind: kind.to_string(),
                        label,
                    });
                    continue;
                }
            }
        }
        // Plain arrow: a --> b : label
        if let Some(rel) = parse_archimate_arrow(trimmed) {
            relations.push(rel);
        }
    }

    Ok(ArchimateDocument {
        elements,
        relations,
        title,
        warnings: Vec::new(),
    })
}

fn parse_archimate_element(rest: &str) -> Option<ArchimateElement> {
    let mut s = rest.trim().to_string();
    let mut stereotype = "business".to_string();
    if let Some(open) = s.find("<<") {
        if let Some(close) = s[open + 2..].find(">>") {
            stereotype = s[open + 2..open + 2 + close].trim().to_string();
            s = format!("{} {}", &s[..open], &s[open + 2 + close + 2..]);
        }
    }
    let s = s.trim();
    let (name, alias) = if let Some(stripped) = s.strip_prefix('"') {
        let close = stripped.find('"')?;
        let name = stripped[..close].to_string();
        let rest = stripped[close + 1..].trim();
        let alias = rest.strip_prefix("as ").map(|a| a.trim().to_string());
        (name, alias)
    } else {
        let mut parts = s.split_whitespace();
        let name = parts.next()?.to_string();
        let alias = if parts.next() == Some("as") {
            parts.next().map(|s| s.to_string())
        } else {
            None
        };
        (name, alias)
    };
    let layer = ArchimateLayer::parse_layer(&stereotype);
    Some(ArchimateElement { name, alias, layer })
}

fn archimate_rel_kind_from_macro(name: &str) -> Option<&'static str> {
    match name {
        "Rel_Association" => Some("association"),
        "Rel_Realization" => Some("realization"),
        "Rel_Serving" => Some("serving"),
        "Rel_Composition" => Some("composition"),
        "Rel_Aggregation" => Some("aggregation"),
        "Rel_Used_By" => Some("used_by"),
        "Rel_Flow" => Some("flow"),
        "Rel_Influence" => Some("influence"),
        "Rel_Triggering" => Some("triggering"),
        _ => None,
    }
}

fn split_csv_args(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_quotes = false;
    for ch in s.chars() {
        if ch == '"' {
            in_quotes = !in_quotes;
            cur.push(ch);
        } else if ch == ',' && !in_quotes {
            out.push(std::mem::take(&mut cur));
        } else {
            cur.push(ch);
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn parse_archimate_arrow(line: &str) -> Option<ArchimateRelation> {
    for arrow in ["-->", "->", "<--", "<-"] {
        if let Some(ix) = line.find(arrow) {
            let lhs = line[..ix].trim();
            let rhs_full = line[ix + arrow.len()..].trim();
            if lhs.is_empty() || rhs_full.is_empty() {
                return None;
            }
            let (rhs, label) = match rhs_full.split_once(':') {
                Some((r, l)) => (r.trim(), Some(l.trim().to_string())),
                None => (rhs_full, None),
            };
            return Some(ArchimateRelation {
                from: lhs.to_string(),
                to: rhs.to_string(),
                kind: "uses".to_string(),
                label,
            });
        }
    }
    None
}

pub fn paginate(document: &SequenceDocument) -> Vec<SequencePage> {
    let mut pages = Vec::new();
    let mut page_events = Vec::new();
    let mut current_title = document.title.clone();

    for event in &document.events {
        if let SequenceEventKind::NewPage(next_title) = &event.kind {
            pages.push(page_from(document, &page_events, current_title.clone()));
            page_events.clear();
            current_title = cleaned_title(next_title).or_else(|| document.title.clone());
            continue;
        }
        page_events.push(event.clone());
    }

    pages.push(page_from(document, &page_events, current_title));
    pages
}

fn page_from(
    document: &SequenceDocument,
    events: &[SequenceEvent],
    title: Option<String>,
) -> SequencePage {
    SequencePage {
        participants: document.participants.clone(),
        events: events.to_vec(),
        title,
        header: document.header.clone(),
        footer: document.footer.clone(),
        caption: document.caption.clone(),
        legend: document.legend.clone(),
        skinparams: document.skinparams.clone(),
        style: document.style.clone(),
        footbox_visible: document.footbox_visible,
        warnings: document.warnings.clone(),
    }
}

fn cleaned_title(value: &Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(ToOwned::to_owned)
}

pub fn normalize_with_options(
    document: Document,
    _options: &NormalizeOptions,
) -> Result<SequenceDocument, Diagnostic> {
    if document.kind != DiagramKind::Sequence {
        return Err(unsupported_family_diagnostic(document.kind));
    }

    let mut participants: Vec<Participant> = Vec::new();
    let mut participant_ix: BTreeMap<String, usize> = BTreeMap::new();
    let mut events = Vec::new();

    let mut title = None;
    let mut header = None;
    let mut footer = None;
    let mut caption = None;
    let mut legend = None;
    let mut skinparams = Vec::new();
    let mut footbox_visible = true;
    let mut style = SequenceStyle::default();
    let mut warnings: Vec<Diagnostic> = Vec::new();
    let mut alive_by_id: BTreeMap<String, bool> = BTreeMap::new();
    let mut activation_stack: Vec<ActivationFrame> = Vec::new();
    let mut group_stack: Vec<GroupFrame> = Vec::new();
    let mut last_message: Option<(String, String)> = None;
    let mut ignore_newpage = false;

    for stmt in document.statements {
        match stmt.kind {
            StatementKind::Participant(p) => {
                mark_group_content(&mut group_stack);
                let id = p.alias.unwrap_or_else(|| p.name.clone());
                let display = p.display.unwrap_or_else(|| p.name.clone());
                upsert_participant(
                    &mut participants,
                    &mut participant_ix,
                    id,
                    display,
                    map_role(p.role),
                    true,
                )
                .map_err(|e| Diagnostic::error(e).with_span(stmt.span))?;
            }
            StatementKind::Message(m) => {
                mark_group_content(&mut group_stack);
                let parsed_arrow = parse_message_arrow(&m.arrow).ok_or_else(|| {
                    Diagnostic::error(format!(
                        "[E_ARROW_INVALID] malformed sequence arrow syntax: `{}`",
                        m.arrow
                    ))
                    .with_span(stmt.span)
                })?;
                let directions = if parsed_arrow.bidirectional {
                    vec![
                        (m.from.clone(), m.to.clone()),
                        (m.to.clone(), m.from.clone()),
                    ]
                } else {
                    vec![(m.from.clone(), m.to.clone())]
                };

                for (from, to) in directions {
                    let from_virtual = virtual_endpoint(from.as_str(), true);
                    let to_virtual = virtual_endpoint(to.as_str(), false);
                    validate_virtual_endpoint_combination(
                        stmt.span,
                        &from,
                        &to,
                        from_virtual,
                        to_virtual,
                    )?;
                    validate_and_touch_message_lifecycle(
                        stmt.span,
                        &from,
                        &to,
                        &mut participants,
                        &mut participant_ix,
                        &mut alive_by_id,
                    )?;
                    if !is_virtual_endpoint(&from) && !is_virtual_endpoint(&to) {
                        last_message = Some((from.clone(), to.clone()));
                    }
                    events.push(SequenceEvent {
                        span: stmt.span,
                        kind: SequenceEventKind::Message {
                            from: from.clone(),
                            to: to.clone(),
                            arrow: parsed_arrow.render_arrow.clone(),
                            label: m.label.clone(),
                            from_virtual,
                            to_virtual,
                        },
                    });
                }
                apply_lifecycle_shortcuts(
                    stmt.span,
                    &m.from,
                    &m.to,
                    &parsed_arrow,
                    &mut participants,
                    &mut participant_ix,
                    &mut alive_by_id,
                    &mut activation_stack,
                    &mut events,
                )?;
            }
            StatementKind::Note(n) => {
                mark_group_content(&mut group_stack);
                events.push(SequenceEvent {
                    span: stmt.span,
                    kind: SequenceEventKind::Note {
                        position: n.position,
                        target: n.target,
                        text: n.text,
                    },
                });
            }
            StatementKind::Group(g) => {
                if g.kind == "end" {
                    let Some(open) = group_stack.pop() else {
                        return Err(Diagnostic::error(
                            "[E_GROUP_END_UNMATCHED] `end` without an open group block",
                        )
                        .with_span(stmt.span));
                    };
                    if let Some(expected) = g.label.as_deref() {
                        if expected != open.kind {
                            return Err(Diagnostic::error(format!(
                                "[E_GROUP_END_KIND] `end {}` does not match open `{}` block",
                                expected, open.kind
                            ))
                            .with_span(stmt.span));
                        }
                    }
                    if rejects_empty_group(open.kind.as_str()) && !open.branch_has_content {
                        return Err(Diagnostic::error(format!(
                            "[E_GROUP_EMPTY] `{}` block must not be empty",
                            open.kind
                        ))
                        .with_span(stmt.span));
                    }
                    events.push(SequenceEvent {
                        span: stmt.span,
                        kind: SequenceEventKind::GroupEnd,
                    });
                } else if g.kind == "else" {
                    let Some(top) = group_stack.last_mut() else {
                        return Err(Diagnostic::error(
                            "[E_GROUP_ELSE_UNMATCHED] `else` without an open group block",
                        )
                        .with_span(stmt.span));
                    };
                    if !allows_else(top.kind.as_str()) {
                        return Err(Diagnostic::error(format!(
                            "[E_GROUP_ELSE_KIND] `else` is not valid inside `{}`",
                            top.kind
                        ))
                        .with_span(stmt.span));
                    }
                    if rejects_empty_group(top.kind.as_str()) && !top.branch_has_content {
                        return Err(Diagnostic::error(format!(
                            "[E_GROUP_EMPTY_BRANCH] `{}` block contains an empty branch before `else`",
                            top.kind
                        ))
                        .with_span(stmt.span));
                    }
                    top.branch_has_content = false;
                    events.push(SequenceEvent {
                        span: stmt.span,
                        kind: SequenceEventKind::GroupStart {
                            kind: g.kind,
                            label: g.label,
                        },
                    });
                } else {
                    mark_group_content(&mut group_stack);
                    if g.kind != "ref" {
                        group_stack.push(GroupFrame {
                            kind: g.kind.clone(),
                            span: stmt.span,
                            branch_has_content: false,
                        });
                    }
                    events.push(SequenceEvent {
                        span: stmt.span,
                        kind: SequenceEventKind::GroupStart {
                            kind: g.kind,
                            label: g.label,
                        },
                    });
                }
            }
            StatementKind::Title(v) => title = Some(v),
            StatementKind::Header(v) => header = Some(v),
            StatementKind::Footer(v) => footer = Some(v),
            StatementKind::Caption(v) => caption = Some(v),
            StatementKind::Legend(v) => legend = Some(v),
            StatementKind::SkinParam { key, value } => {
                mark_group_content(&mut group_stack);
                skinparams.push((key.clone(), value.clone()));
                match classify_sequence_skinparam(&key, &value) {
                    SequenceSkinParamSupport::SupportedNoop => {}
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::FootboxVisible(visible),
                    ) => {
                        footbox_visible = visible;
                    }
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::ArrowColor(color),
                    ) => style.arrow_color = color,
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::LifelineBorderColor(color),
                    ) => style.lifeline_border_color = color,
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::ParticipantBackgroundColor(color),
                    ) => style.participant_background_color = color,
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::ParticipantBorderColor(color),
                    ) => style.participant_border_color = color,
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::NoteBackgroundColor(color),
                    ) => style.note_background_color = color,
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::NoteBorderColor(color),
                    ) => style.note_border_color = color,
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::GroupBackgroundColor(color),
                    ) => style.group_background_color = color,
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::GroupBorderColor(color),
                    ) => style.group_border_color = color,
                    SequenceSkinParamSupport::UnsupportedValue => {
                        warnings.push(
                            Diagnostic::warning(format!(
                                "[W_SKINPARAM_UNSUPPORTED_VALUE] unsupported value `{}` for skinparam `{}`",
                                value, key
                            ))
                            .with_span(stmt.span),
                        );
                    }
                    SequenceSkinParamSupport::UnsupportedKey => {
                        warnings.push(
                            Diagnostic::warning(format!(
                                "[W_SKINPARAM_UNSUPPORTED] unsupported skinparam `{}`",
                                key
                            ))
                            .with_span(stmt.span),
                        );
                    }
                }
            }
            StatementKind::Theme(name) => {
                mark_group_content(&mut group_stack);
                let preset = resolve_sequence_theme_preset(&name)
                    .map_err(|msg| Diagnostic::error(msg).with_span(stmt.span))?;
                style = preset.style;
            }
            StatementKind::Pragma(value) => {
                mark_group_content(&mut group_stack);
                let trimmed = value.trim();
                let lower = trimmed.to_ascii_lowercase();
                if lower.starts_with("teoz ") || lower == "teoz" {
                    warnings.push(
                        Diagnostic::warning(
                            "[W_PRAGMA_TEOZ_UNSUPPORTED] !pragma teoz is not supported yet; continuing with default sequence layout semantics"
                                .to_string(),
                        )
                        .with_span(stmt.span),
                    );
                } else {
                    warnings.push(
                        Diagnostic::warning(format!(
                            "[W_PRAGMA_UNSUPPORTED] unsupported pragma `{}`",
                            trimmed
                        ))
                        .with_span(stmt.span),
                    );
                }
            }
            StatementKind::Footbox(v) => {
                mark_group_content(&mut group_stack);
                footbox_visible = v
            }
            StatementKind::Delay(v) => {
                mark_group_content(&mut group_stack);
                events.push(SequenceEvent {
                    span: stmt.span,
                    kind: SequenceEventKind::Delay(v),
                })
            }
            StatementKind::Divider(v) => {
                mark_group_content(&mut group_stack);
                events.push(SequenceEvent {
                    span: stmt.span,
                    kind: SequenceEventKind::Divider(v),
                })
            }
            StatementKind::Separator(v) => {
                mark_group_content(&mut group_stack);
                events.push(SequenceEvent {
                    span: stmt.span,
                    kind: SequenceEventKind::Separator(v),
                })
            }
            StatementKind::Spacer => {
                mark_group_content(&mut group_stack);
                events.push(SequenceEvent {
                    span: stmt.span,
                    kind: SequenceEventKind::Spacer,
                })
            }
            StatementKind::NewPage(v) => {
                mark_group_content(&mut group_stack);
                if !ignore_newpage {
                    events.push(SequenceEvent {
                        span: stmt.span,
                        kind: SequenceEventKind::NewPage(v),
                    });
                }
            }
            StatementKind::IgnoreNewPage => {
                mark_group_content(&mut group_stack);
                ignore_newpage = true;
            }
            StatementKind::Autonumber(v) => {
                mark_group_content(&mut group_stack);
                if let Some(raw) = v.as_deref() {
                    validate_autonumber_raw(raw).map_err(|reason| {
                        Diagnostic::error(format!("[E_AUTONUMBER_FORMAT_UNSUPPORTED] {reason}"))
                            .with_span(stmt.span)
                    })?;
                }
                events.push(SequenceEvent {
                    span: stmt.span,
                    kind: SequenceEventKind::Autonumber(
                        v.as_deref().and_then(canonicalize_autonumber_raw),
                    ),
                })
            }
            StatementKind::Activate(id) => {
                mark_group_content(&mut group_stack);
                ensure_implicit(&mut participants, &mut participant_ix, &id);
                if !is_alive(&alive_by_id, &id) {
                    return Err(Diagnostic::error(format!(
                        "[E_LIFECYCLE_ACTIVATE_DESTROYED] cannot activate destroyed participant `{}`",
                        id
                    ))
                    .with_span(stmt.span));
                }
                alive_by_id.insert(id.clone(), true);
                let caller = match &last_message {
                    Some((from, to)) if to == &id => Some(from.clone()),
                    _ => activation_stack.last().map(|f| f.participant.clone()),
                };
                activation_stack.push(ActivationFrame {
                    participant: id.clone(),
                    caller,
                });
                events.push(SequenceEvent {
                    span: stmt.span,
                    kind: SequenceEventKind::Activate(id),
                });
            }
            StatementKind::Deactivate(id) => {
                mark_group_content(&mut group_stack);
                ensure_implicit(&mut participants, &mut participant_ix, &id);
                if !is_alive(&alive_by_id, &id) {
                    return Err(Diagnostic::error(format!(
                        "[E_LIFECYCLE_DEACTIVATE_DESTROYED] cannot deactivate destroyed participant `{}`",
                        id
                    ))
                    .with_span(stmt.span));
                }
                alive_by_id.insert(id.clone(), true);
                match activation_stack.last() {
                    Some(frame) if frame.participant == id => {
                        activation_stack.pop();
                    }
                    Some(frame) => {
                        return Err(Diagnostic::error(format!(
                            "[E_LIFECYCLE_DEACTIVATE_ORDER] deactivate `{}` does not match current activation `{}`",
                            id, frame.participant
                        ))
                        .with_span(stmt.span));
                    }
                    None => {
                        return Err(Diagnostic::error(format!(
                            "[E_LIFECYCLE_DEACTIVATE_EMPTY] cannot deactivate `{}` without an active activation",
                            id
                        ))
                        .with_span(stmt.span));
                    }
                }
                events.push(SequenceEvent {
                    span: stmt.span,
                    kind: SequenceEventKind::Deactivate(id),
                });
            }
            StatementKind::Destroy(id) => {
                mark_group_content(&mut group_stack);
                ensure_implicit(&mut participants, &mut participant_ix, &id);
                if !is_alive(&alive_by_id, &id) {
                    return Err(Diagnostic::error(format!(
                        "[E_LIFECYCLE_DESTROY_TWICE] participant `{}` is already destroyed",
                        id
                    ))
                    .with_span(stmt.span));
                }
                if activation_stack.iter().any(|f| f.participant == id) {
                    return Err(Diagnostic::error(format!(
                        "[E_LIFECYCLE_DESTROY_ACTIVE] cannot destroy active participant `{}`",
                        id
                    ))
                    .with_span(stmt.span));
                }
                alive_by_id.insert(id.clone(), false);
                events.push(SequenceEvent {
                    span: stmt.span,
                    kind: SequenceEventKind::Destroy(id),
                });
            }
            StatementKind::Create(id) => {
                mark_group_content(&mut group_stack);
                ensure_implicit(&mut participants, &mut participant_ix, &id);
                if alive_by_id.get(&id).copied() == Some(true) {
                    return Err(Diagnostic::error(format!(
                        "[E_LIFECYCLE_CREATE_EXISTING] participant `{}` already exists; destroy before create",
                        id
                    ))
                    .with_span(stmt.span));
                }
                alive_by_id.insert(id.clone(), true);
                events.push(SequenceEvent {
                    span: stmt.span,
                    kind: SequenceEventKind::Create(id),
                });
            }
            StatementKind::Return(v) => {
                mark_group_content(&mut group_stack);
                events.push(SequenceEvent {
                    span: stmt.span,
                    kind: infer_return_event(stmt.span, v, &mut activation_stack, &last_message)?,
                })
            }
            StatementKind::Include(_) | StatementKind::Define { .. } | StatementKind::Undef(_) => {
                // Preprocessor directives should be expanded before normalization.
            }
            StatementKind::ClassDecl(_)
            | StatementKind::ObjectDecl(_)
            | StatementKind::UseCaseDecl(_)
            | StatementKind::FamilyRelation(_)
            | StatementKind::StateDecl(_)
            | StatementKind::StateTransition(_)
            | StatementKind::StateInternalAction(_)
            | StatementKind::StateRegionDivider
            | StatementKind::StateHistory { .. }
            | StatementKind::GanttTaskDecl { .. }
            | StatementKind::GanttMilestoneDecl { .. }
            | StatementKind::GanttConstraint { .. }
            | StatementKind::ChronologyHappensOn { .. } => {
                return Err(Diagnostic::error(
                    "[E_FAMILY_MIXED] mixed diagram families are not supported in one document",
                )
                .with_span(stmt.span));
            }
            StatementKind::Unknown(line) => {
                if line.trim() == "---" {
                    continue;
                }
                return Err(Diagnostic::error(format!(
                    "[E_PARSE_UNKNOWN] unsupported syntax: `{}`",
                    line
                ))
                .with_span(stmt.span));
            }
            // Non-sequence family bodies are stored as RawBody or SaltGridRow;
            // these should never reach the sequence normalizer.
            StatementKind::RawBody(_) | StatementKind::SaltGridRow { .. } => {
                return Err(Diagnostic::error(
                    "[E_FAMILY_MIXED] non-sequence family body found in sequence context",
                )
                .with_span(stmt.span));
            }
        }
    }

    if let Some(open) = group_stack.pop() {
        return Err(Diagnostic::error(format!(
            "[E_GROUP_UNCLOSED] missing `end` for open `{}` block",
            open.kind
        ))
        .with_span(open.span));
    }

    warnings.sort_by(|a, b| {
        let sa = a.span.map(|s| s.start).unwrap_or_default();
        let sb = b.span.map(|s| s.start).unwrap_or_default();
        (a.message.as_str(), sa).cmp(&(b.message.as_str(), sb))
    });

    Ok(SequenceDocument {
        participants,
        events,
        title,
        header,
        footer,
        caption,
        legend,
        skinparams,
        style,
        footbox_visible,
        warnings,
    })
}

fn unsupported_family_diagnostic(kind: DiagramKind) -> Diagnostic {
    let (code, family) = match kind {
        DiagramKind::Component => ("E_FAMILY_COMPONENT_UNSUPPORTED", "component"),
        DiagramKind::Deployment => ("E_FAMILY_DEPLOYMENT_UNSUPPORTED", "deployment"),
        DiagramKind::State => ("E_FAMILY_STATE_UNSUPPORTED", "state"),
        DiagramKind::Activity => ("E_FAMILY_ACTIVITY_UNSUPPORTED", "activity"),
        DiagramKind::Timing => ("E_FAMILY_TIMING_UNSUPPORTED", "timing"),
        DiagramKind::MindMap => ("E_FAMILY_MINDMAP_UNSUPPORTED", "mindmap"),
        DiagramKind::Wbs => ("E_FAMILY_WBS_UNSUPPORTED", "wbs"),
        DiagramKind::Gantt => ("E_FAMILY_GANTT_UNSUPPORTED", "gantt"),
        DiagramKind::Chronology => ("E_FAMILY_CHRONOLOGY_UNSUPPORTED", "chronology"),
        _ => ("E_FAMILY_UNSUPPORTED", "unknown"),
    };

    Diagnostic::error_code(
        code,
        format!(
            "diagram family `{family}` is not implemented yet; sequence is currently supported"
        ),
    )
}

fn is_alive(alive_by_id: &BTreeMap<String, bool>, id: &str) -> bool {
    alive_by_id.get(id).copied().unwrap_or(true)
}

#[derive(Debug, Clone)]
struct ActivationFrame {
    participant: String,
    caller: Option<String>,
}

#[derive(Debug, Clone)]
struct GroupFrame {
    kind: String,
    span: crate::source::Span,
    branch_has_content: bool,
}

fn mark_group_content(group_stack: &mut [GroupFrame]) {
    for frame in group_stack {
        frame.branch_has_content = true;
    }
}

fn allows_else(kind: &str) -> bool {
    matches!(kind, "alt" | "par" | "critical")
}

fn rejects_empty_group(kind: &str) -> bool {
    matches!(kind, "alt" | "par" | "critical")
}

fn infer_return_event(
    span: crate::source::Span,
    label: Option<String>,
    activation_stack: &mut Vec<ActivationFrame>,
    last_message: &Option<(String, String)>,
) -> Result<SequenceEventKind, Diagnostic> {
    if activation_stack.is_empty() {
        if let Some((from, to)) = last_message {
            return Ok(SequenceEventKind::Return {
                label,
                from: Some(to.clone()),
                to: Some(from.clone()),
            });
        }
    }
    let Some(frame) = activation_stack.pop() else {
        return Err(Diagnostic::error(
            "[E_RETURN_INFER_EMPTY] cannot infer `return` sender/target without an active activation",
        )
        .with_span(span));
    };

    let Some(caller) = frame.caller else {
        return Err(Diagnostic::error(format!(
            "[E_RETURN_INFER_CALLER] cannot infer `return` target for `{}`; use an explicit return message instead",
            frame.participant
        ))
        .with_span(span));
    };

    Ok(SequenceEventKind::Return {
        label,
        from: Some(frame.participant),
        to: Some(caller),
    })
}

fn ensure_implicit(
    participants: &mut Vec<Participant>,
    index: &mut BTreeMap<String, usize>,
    id: &str,
) {
    if index.contains_key(id) {
        return;
    }
    let pos = participants.len();
    participants.push(Participant {
        id: id.to_string(),
        display: id.to_string(),
        role: ParticipantRole::Participant,
        explicit: false,
    });
    index.insert(id.to_string(), pos);
}

fn upsert_participant(
    participants: &mut Vec<Participant>,
    index: &mut BTreeMap<String, usize>,
    id: String,
    display: String,
    role: ParticipantRole,
    explicit: bool,
) -> Result<(), String> {
    if let Some(ix) = index.get(&id).copied() {
        if explicit && participants[ix].explicit {
            return Err(format!(
                "[E_PARTICIPANT_DUPLICATE] duplicate participant id/alias `{}`",
                id
            ));
        }
        participants[ix].display = display;
        participants[ix].role = role;
        participants[ix].explicit = explicit;
        return Ok(());
    }

    let pos = participants.len();
    participants.push(Participant {
        id: id.clone(),
        display,
        role,
        explicit,
    });
    index.insert(id, pos);
    Ok(())
}

fn map_role(role: AstRole) -> ParticipantRole {
    match role {
        AstRole::Participant => ParticipantRole::Participant,
        AstRole::Actor => ParticipantRole::Actor,
        AstRole::Boundary => ParticipantRole::Boundary,
        AstRole::Control => ParticipantRole::Control,
        AstRole::Entity => ParticipantRole::Entity,
        AstRole::Database => ParticipantRole::Database,
        AstRole::Collections => ParticipantRole::Collections,
        AstRole::Queue => ParticipantRole::Queue,
    }
}

fn is_virtual_endpoint(id: &str) -> bool {
    matches!(id, "[*]" | "[" | "]" | "[o" | "o]" | "[x" | "x]")
}

fn virtual_endpoint(id: &str, is_from: bool) -> Option<VirtualEndpoint> {
    let (side, kind) = match id {
        "[" => (VirtualEndpointSide::Left, VirtualEndpointKind::Plain),
        "]" => (VirtualEndpointSide::Right, VirtualEndpointKind::Plain),
        "[o" => (VirtualEndpointSide::Left, VirtualEndpointKind::Circle),
        "o]" => (VirtualEndpointSide::Right, VirtualEndpointKind::Circle),
        "[x" => (VirtualEndpointSide::Left, VirtualEndpointKind::Cross),
        "x]" => (VirtualEndpointSide::Right, VirtualEndpointKind::Cross),
        "[*]" => (
            if is_from {
                VirtualEndpointSide::Left
            } else {
                VirtualEndpointSide::Right
            },
            VirtualEndpointKind::Filled,
        ),
        _ => return None,
    };
    Some(VirtualEndpoint { side, kind })
}

fn validate_virtual_endpoint_combination(
    span: crate::source::Span,
    from: &str,
    to: &str,
    from_virtual: Option<VirtualEndpoint>,
    to_virtual: Option<VirtualEndpoint>,
) -> Result<(), Diagnostic> {
    if from_virtual.is_some() && to_virtual.is_some() {
        return Err(Diagnostic::error(format!(
            "[E_ENDPOINT_COMBINATION] virtual endpoint messages must include at least one concrete participant: `{}` -> `{}`",
            from, to
        ))
        .with_span(span));
    }
    Ok(())
}

#[derive(Debug, Clone)]
struct ParsedMessageArrow {
    render_arrow: String,
    bidirectional: bool,
    left_modifier: Option<String>,
    right_modifier: Option<String>,
}

fn parse_message_arrow(raw: &str) -> Option<ParsedMessageArrow> {
    let (base, left_modifier, right_modifier) = decode_arrow_modifiers(raw)?;
    let canonical_base = base.replace(['/', '\\'], "");
    if canonical_base.is_empty()
        || !canonical_base
            .chars()
            .all(|c| matches!(c, '-' | '<' | '>' | 'o' | 'x'))
    {
        return None;
    }
    let stripped_left = canonical_base
        .strip_prefix('o')
        .or_else(|| canonical_base.strip_prefix('x'))
        .unwrap_or(&canonical_base);
    let stripped = stripped_left
        .strip_suffix('o')
        .or_else(|| stripped_left.strip_suffix('x'))
        .unwrap_or(stripped_left);
    let bidirectional = matches!(stripped, "<->" | "<-->" | "<<->>" | "<<-->>");
    let render_arrow = if bidirectional {
        if stripped.contains("--") {
            "-->".to_string()
        } else {
            "->".to_string()
        }
    } else {
        canonical_base
    };
    Some(ParsedMessageArrow {
        render_arrow,
        bidirectional,
        left_modifier,
        right_modifier,
    })
}

fn decode_arrow_modifiers(raw: &str) -> Option<(String, Option<String>, Option<String>)> {
    let mut rest = raw;
    let mut left_modifier = None;
    let mut right_modifier = None;
    while let Some(ix) = rest.find("@L").or_else(|| rest.find("@R")) {
        let side = &rest[ix..ix + 2];
        let token = rest.get(ix + 2..ix + 4)?;
        if !matches!(token, "++" | "--" | "**" | "!!") {
            return None;
        }
        if side == "@L" {
            left_modifier = Some(token.to_string());
        } else {
            right_modifier = Some(token.to_string());
        }
        rest = &rest[..ix];
    }
    Some((rest.to_string(), left_modifier, right_modifier))
}

fn validate_and_touch_message_lifecycle(
    span: crate::source::Span,
    from: &str,
    to: &str,
    participants: &mut Vec<Participant>,
    participant_ix: &mut BTreeMap<String, usize>,
    alive_by_id: &mut BTreeMap<String, bool>,
) -> Result<(), Diagnostic> {
    let from_virtual = is_virtual_endpoint(from);
    let to_virtual = is_virtual_endpoint(to);
    if !from_virtual {
        ensure_implicit(participants, participant_ix, from);
    }
    if !to_virtual {
        ensure_implicit(participants, participant_ix, to);
    }
    if !from_virtual && !is_alive(alive_by_id, from) {
        return Err(Diagnostic::error(format!(
            "[E_LIFECYCLE_DESTROYED_SENDER] message sender `{}` is destroyed",
            from
        ))
        .with_span(span));
    }
    if !to_virtual && !is_alive(alive_by_id, to) {
        return Err(Diagnostic::error(format!(
            "[E_LIFECYCLE_DESTROYED_TARGET] message target `{}` is destroyed (recreate it before sending messages to it)",
            to
        ))
        .with_span(span));
    }
    if !from_virtual {
        alive_by_id.insert(from.to_string(), true);
    }
    if !to_virtual {
        alive_by_id.insert(to.to_string(), true);
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn apply_lifecycle_shortcuts(
    span: crate::source::Span,
    from: &str,
    to: &str,
    parsed_arrow: &ParsedMessageArrow,
    participants: &mut Vec<Participant>,
    participant_ix: &mut BTreeMap<String, usize>,
    alive_by_id: &mut BTreeMap<String, bool>,
    activation_stack: &mut Vec<ActivationFrame>,
    events: &mut Vec<SequenceEvent>,
) -> Result<(), Diagnostic> {
    if let Some(token) = &parsed_arrow.left_modifier {
        let caller = shortcut_caller(from, to);
        apply_one_lifecycle_shortcut(
            span,
            from,
            token,
            caller,
            participants,
            participant_ix,
            alive_by_id,
            activation_stack,
            events,
        )?;
    }
    if let Some(token) = &parsed_arrow.right_modifier {
        let id = if token == "--" { from } else { to };
        let caller = shortcut_caller(id, if id == from { to } else { from });
        apply_one_lifecycle_shortcut(
            span,
            id,
            token,
            caller,
            participants,
            participant_ix,
            alive_by_id,
            activation_stack,
            events,
        )?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn apply_one_lifecycle_shortcut(
    span: crate::source::Span,
    id: &str,
    token: &str,
    caller: Option<String>,
    participants: &mut Vec<Participant>,
    participant_ix: &mut BTreeMap<String, usize>,
    alive_by_id: &mut BTreeMap<String, bool>,
    activation_stack: &mut Vec<ActivationFrame>,
    events: &mut Vec<SequenceEvent>,
) -> Result<(), Diagnostic> {
    if is_virtual_endpoint(id) {
        return Err(Diagnostic::error(format!(
            "[E_LIFECYCLE_SHORTCUT_VIRTUAL] cannot apply lifecycle shortcut `{}` to virtual endpoint",
            token
        ))
        .with_span(span));
    }
    ensure_implicit(participants, participant_ix, id);
    match token {
        "++" => {
            if !is_alive(alive_by_id, id) {
                return Err(Diagnostic::error(format!(
                    "[E_LIFECYCLE_ACTIVATE_DESTROYED] cannot activate destroyed participant `{}`",
                    id
                ))
                .with_span(span));
            }
            alive_by_id.insert(id.to_string(), true);
            activation_stack.push(ActivationFrame {
                participant: id.to_string(),
                caller,
            });
            events.push(SequenceEvent {
                span,
                kind: SequenceEventKind::Activate(id.to_string()),
            });
        }
        "--" => {
            if !is_alive(alive_by_id, id) {
                return Err(Diagnostic::error(format!(
                    "[E_LIFECYCLE_DEACTIVATE_DESTROYED] cannot deactivate destroyed participant `{}`",
                    id
                ))
                .with_span(span));
            }
            alive_by_id.insert(id.to_string(), true);
            match activation_stack.last() {
                Some(frame) if frame.participant == id => {
                    activation_stack.pop();
                }
                Some(frame) => {
                    return Err(Diagnostic::error(format!(
                        "[E_LIFECYCLE_DEACTIVATE_ORDER] deactivate `{}` does not match current activation `{}`",
                        id, frame.participant
                    ))
                    .with_span(span));
                }
                None => {
                    return Err(Diagnostic::error(format!(
                        "[E_LIFECYCLE_DEACTIVATE_EMPTY] cannot deactivate `{}` without an active activation",
                        id
                    ))
                    .with_span(span));
                }
            }
            events.push(SequenceEvent {
                span,
                kind: SequenceEventKind::Deactivate(id.to_string()),
            });
        }
        "**" => {
            alive_by_id.insert(id.to_string(), true);
            events.push(SequenceEvent {
                span,
                kind: SequenceEventKind::Create(id.to_string()),
            });
        }
        "!!" => {
            if !is_alive(alive_by_id, id) {
                return Err(Diagnostic::error(format!(
                    "[E_LIFECYCLE_DESTROY_TWICE] participant `{}` is already destroyed",
                    id
                ))
                .with_span(span));
            }
            if activation_stack.iter().any(|f| f.participant == id) {
                return Err(Diagnostic::error(format!(
                    "[E_LIFECYCLE_DESTROY_ACTIVE] cannot destroy active participant `{}`",
                    id
                ))
                .with_span(span));
            }
            alive_by_id.insert(id.to_string(), false);
            events.push(SequenceEvent {
                span,
                kind: SequenceEventKind::Destroy(id.to_string()),
            });
        }
        _ => {
            return Err(Diagnostic::error(format!(
                "[E_LIFECYCLE_SHORTCUT_INVALID] unknown lifecycle shortcut `{}`",
                token
            ))
            .with_span(span));
        }
    }
    Ok(())
}

fn shortcut_caller(active: &str, other: &str) -> Option<String> {
    if is_virtual_endpoint(active) || is_virtual_endpoint(other) {
        None
    } else {
        Some(other.to_string())
    }
}

fn canonicalize_autonumber_raw(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut out = String::with_capacity(trimmed.len());
    let mut in_quotes = false;
    let mut prev_space = false;
    for ch in trimmed.chars() {
        if ch == '"' {
            in_quotes = !in_quotes;
            prev_space = false;
            out.push(ch);
            continue;
        }
        if ch.is_whitespace() && !in_quotes {
            if !prev_space {
                out.push(' ');
            }
            prev_space = true;
            continue;
        }
        prev_space = false;
        out.push(ch);
    }
    Some(out.trim().to_string())
}

fn validate_autonumber_raw(raw: &str) -> Result<(), String> {
    let trimmed = raw.trim();
    if trimmed.is_empty()
        || trimmed.eq_ignore_ascii_case("stop")
        || trimmed.eq_ignore_ascii_case("off")
        || trimmed.eq_ignore_ascii_case("resume")
    {
        return Ok(());
    }

    let (format, body) = if trimmed.contains('"') {
        let Some((format, before)) = trailing_quoted_format(trimmed) else {
            return Err("malformed quoted autonumber format; quote-delimited format must be the final token".to_string());
        };
        (Some(format), before.trim_end())
    } else {
        (None, trimmed)
    };

    let mut tokens: Vec<&str> = body.split_whitespace().collect();
    let mut resume = false;
    if matches!(tokens.first(), Some(token) if token.eq_ignore_ascii_case("resume")) {
        resume = true;
        tokens.remove(0);
    }

    let mut idx = 0usize;
    let expected_numbers = if resume { 1 } else { 2 };
    while idx < tokens.len() && idx < expected_numbers && tokens[idx].parse::<u64>().is_ok() {
        idx += 1;
    }

    let unquoted_format = if idx < tokens.len() {
        let fmt = tokens[idx];
        idx += 1;
        Some(fmt)
    } else {
        None
    };

    if idx < tokens.len() {
        return Err(
            "unsupported autonumber syntax; expected `autonumber [start] [increment] [format]` or `autonumber resume [increment] [format]`".to_string(),
        );
    }

    if let Some(fmt) = format.or(unquoted_format.map(str::to_string)) {
        validate_autonumber_format(&fmt)?;
    }

    Ok(())
}

fn trailing_quoted_format(raw: &str) -> Option<(String, &str)> {
    let trimmed = raw.trim_end();
    let end = trimmed.strip_suffix('"')?;
    let start = end.rfind('"')?;
    let format = end[start + 1..].to_string();
    let prefix = &end[..start];
    Some((format, prefix))
}

fn validate_autonumber_format(format: &str) -> Result<(), String> {
    let fmt = format.trim();
    if fmt.is_empty() {
        return Err("autonumber format must not be empty".to_string());
    }
    if fmt.contains('<') || fmt.contains('>') {
        return Err(
            "autonumber format does not support HTML tags in this deterministic subset".to_string(),
        );
    }
    if fmt.contains('"') {
        return Err("autonumber format must not contain an embedded quote".to_string());
    }
    Ok(())
}
