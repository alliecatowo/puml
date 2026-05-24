use puml::ast::DiagramKind;
use puml::model::{
    Participant, ParticipantRole as ModelParticipantRole, SequenceDocument, SequenceEvent,
    SequenceEventKind, StateDocument, TimelineDocument, VirtualEndpoint, VirtualEndpointKind,
    VirtualEndpointSide,
};
use puml::NormalizedDocument;
use serde::Serialize;
use serde_json::{json, Value};

#[derive(Debug, Serialize)]
struct SceneDump {
    size: SceneSize,
    lanes: Vec<SceneLane>,
    rows: Vec<SceneRow>,
}

#[derive(Debug, Serialize)]
struct SceneSize {
    width: i32,
    height: i32,
}

#[derive(Debug, Serialize)]
struct SceneLane {
    id: String,
    display: String,
    role: String,
    x: i32,
}

#[derive(Debug, Serialize)]
struct SceneRow {
    y: i32,
    event: Value,
}

pub(crate) fn normalized_model_to_json(model: &NormalizedDocument) -> Value {
    match model {
        NormalizedDocument::Sequence(sequence) => model_to_json(sequence),
        NormalizedDocument::Family(family) => family_model_to_json(family),
        NormalizedDocument::FamilyPages(pages) => json!({
            "kind": "FamilyPages",
            "pages": pages.iter().map(family_model_to_json).collect::<Vec<_>>()
        }),
        NormalizedDocument::Timeline(timeline) => timeline_model_to_json(timeline),
        NormalizedDocument::State(state) => state_model_to_json(state),
        NormalizedDocument::Json(doc) => json!({"kind": "Json", "warnings": doc.warnings.len()}),
        NormalizedDocument::Yaml(doc) => json!({"kind": "Yaml", "warnings": doc.warnings.len()}),
        NormalizedDocument::Nwdiag(doc) => {
            json!({"kind": "Nwdiag", "warnings": doc.warnings.len()})
        }
        NormalizedDocument::Archimate(doc) => {
            json!({"kind": "Archimate", "warnings": doc.warnings.len()})
        }
        NormalizedDocument::Regex(doc) => json!({"kind": "Regex", "warnings": doc.warnings.len()}),
        NormalizedDocument::Ebnf(doc) => json!({"kind": "Ebnf", "warnings": doc.warnings.len()}),
        NormalizedDocument::Math(doc) => json!({"kind": "Math", "warnings": doc.warnings.len()}),
        NormalizedDocument::Sdl(doc) => json!({"kind": "Sdl", "warnings": doc.warnings.len()}),
        NormalizedDocument::Ditaa(doc) => json!({"kind": "Ditaa", "warnings": doc.warnings.len()}),
        NormalizedDocument::Chart(doc) => json!({"kind": "Chart", "warnings": doc.warnings.len()}),
    }
}

fn state_model_to_json(model: &StateDocument) -> Value {
    json!({
        "kind": "State",
        "nodes": model.nodes.iter().map(|n| json!({
            "name": n.name,
            "display": n.display,
            "kind": match n.kind {
                puml::model::StateNodeKind::Normal => "Normal",
                puml::model::StateNodeKind::StartEnd => "StartEnd",
                puml::model::StateNodeKind::HistoryShallow => "HistoryShallow",
                puml::model::StateNodeKind::HistoryDeep => "HistoryDeep",
                puml::model::StateNodeKind::Fork => "Fork",
                puml::model::StateNodeKind::Join => "Join",
                puml::model::StateNodeKind::Choice => "Choice",
                puml::model::StateNodeKind::End => "End",
                puml::model::StateNodeKind::EntryPoint => "EntryPoint",
                puml::model::StateNodeKind::ExitPoint => "ExitPoint",
                puml::model::StateNodeKind::InputPin => "InputPin",
                puml::model::StateNodeKind::OutputPin => "OutputPin",
                puml::model::StateNodeKind::ExpansionInput => "ExpansionInput",
                puml::model::StateNodeKind::ExpansionOutput => "ExpansionOutput",
                puml::model::StateNodeKind::Note => "Note",
                puml::model::StateNodeKind::JsonProjection => "JsonProjection",
            },
            "style": {
                "fill_color": n.style.fill_color,
                "border_color": n.style.border_color,
                "border_dashed": n.style.border_dashed,
                "border_thickness": n.style.border_thickness,
                "text_color": n.style.text_color,
            },
            "internal_actions": n.internal_actions.iter().map(|a| json!({
                "kind": a.kind,
                "action": a.action
            })).collect::<Vec<_>>()
        })).collect::<Vec<_>>(),
        "transitions": model.transitions.iter().map(|t| json!({
            "from": t.from,
            "to": t.to,
            "label": t.label
        })).collect::<Vec<_>>(),
        "title": model.title,
        "hide_empty_description": model.hide_empty_description,
        "warnings": model.warnings.iter().map(|d| d.message.clone()).collect::<Vec<_>>()
    })
}

fn model_to_json(model: &SequenceDocument) -> Value {
    json!({
        "participants": model.participants.iter().map(model_participant_to_json).collect::<Vec<_>>(),
        "events": model.events.iter().map(model_event_to_json).collect::<Vec<_>>(),
        "teoz": model.teoz,
        "title": model.title,
        "header": model.header,
        "footer": model.footer,
        "caption": model.caption,
        "legend": model.legend,
        "skinparams": model.skinparams,
        "style": {
            "arrow_color": model.style.arrow_color,
            "lifeline_border_color": model.style.lifeline_border_color,
            "participant_background_color": model.style.participant_background_color,
            "participant_border_color": model.style.participant_border_color,
            "note_background_color": model.style.note_background_color,
            "note_border_color": model.style.note_border_color,
            "group_background_color": model.style.group_background_color,
            "group_border_color": model.style.group_border_color
        },
        "footbox_visible": model.footbox_visible
    })
}

fn family_model_to_json(model: &puml::FamilyDocument) -> Value {
    json!({
        "kind": format!("{:?}", model.kind),
        "nodes": model
            .nodes
            .iter()
            .map(|n| {
                json!({
                    "kind": format!("{:?}", n.kind),
                    "name": n.name,
                    "alias": n.alias
                })
            })
            .collect::<Vec<_>>(),
        "relations": model
            .relations
            .iter()
            .map(|r| {
                json!({
                    "from": r.from,
                    "to": r.to,
                    "arrow": r.arrow,
                    "label": r.label
                })
            })
            .collect::<Vec<_>>(),
        "title": model.title,
        "header": model.header,
        "footer": model.footer,
        "caption": model.caption,
        "legend": model.legend,
        "warnings": model.warnings.iter().map(|d| d.message.clone()).collect::<Vec<_>>()
    })
}

fn timeline_model_to_json(model: &TimelineDocument) -> Value {
    json!({
        "kind": match model.kind {
            DiagramKind::Salt => "Salt",
            DiagramKind::Gantt => "Gantt",
            DiagramKind::Chronology => "Chronology",
            _ => "Timeline",
        },
        "tasks": model
            .tasks
            .iter()
            .map(|t| json!({"name": t.name, "start_day": t.start_day, "workload_days": t.workload_days, "duration_days": t.duration_days, "resources": t.resources, "resource_allocations": t.resource_allocations.iter().map(|r| json!({"name": r.name, "load_percent": r.load_percent})).collect::<Vec<_>>()}))
            .collect::<Vec<_>>(),
        "milestones": model.milestones.iter().map(|m| json!({"name": m.name, "happens_on": m.happens_on})).collect::<Vec<_>>(),
        "separators": model.separators.iter().map(|s| json!({"label": s.label, "target": s.target})).collect::<Vec<_>>(),
        "constraints": model
            .constraints
            .iter()
            .map(|c| json!({"subject": c.subject, "kind": c.kind, "target": c.target}))
            .collect::<Vec<_>>(),
        "closed_weekdays": model.closed_weekdays,
        "closed_ranges": model
            .closed_ranges
            .iter()
            .map(|r| json!({"start_date": r.start_date, "end_date": r.end_date, "start_day": r.start_day, "end_day": r.end_day}))
            .collect::<Vec<_>>(),
        "open_ranges": model
            .open_ranges
            .iter()
            .map(|r| json!({"start_date": r.start_date, "end_date": r.end_date, "start_day": r.start_day, "end_day": r.end_day}))
            .collect::<Vec<_>>(),
        "named_dates": model
            .named_dates
            .iter()
            .map(|n| json!({"date": n.date, "label": n.label, "day": n.day}))
            .collect::<Vec<_>>(),
        "chronology_events": model
            .chronology_events
            .iter()
            .map(|e| json!({"subject": e.subject, "when": e.when}))
            .collect::<Vec<_>>(),
        "project_start": model.project_start,
        "project_start_day": model.project_start_day,
        "print_start": model.print_start,
        "print_end": model.print_end,
        "print_start_day": model.print_start_day,
        "print_end_day": model.print_end_day,
        "title": model.title,
        "header": model.header,
        "footer": model.footer,
        "caption": model.caption,
        "legend": model.legend,
        "warnings": model.warnings.iter().map(|d| d.message.clone()).collect::<Vec<_>>()
    })
}

fn model_participant_to_json(p: &Participant) -> Value {
    json!({
        "id": p.id,
        "display": p.display,
        "role": model_role_to_str(p.role),
        "explicit": p.explicit
    })
}

fn model_role_to_str(role: ModelParticipantRole) -> &'static str {
    match role {
        ModelParticipantRole::Participant => "Participant",
        ModelParticipantRole::Actor => "Actor",
        ModelParticipantRole::Boundary => "Boundary",
        ModelParticipantRole::Control => "Control",
        ModelParticipantRole::Entity => "Entity",
        ModelParticipantRole::Database => "Database",
        ModelParticipantRole::Collections => "Collections",
        ModelParticipantRole::Queue => "Queue",
    }
}

fn model_event_to_json(e: &SequenceEvent) -> Value {
    json!({
        "span": {"start": e.span.start, "end": e.span.end},
        "kind": model_event_kind_to_json(&e.kind)
    })
}

fn model_event_kind_to_json(kind: &SequenceEventKind) -> Value {
    match kind {
        SequenceEventKind::Message {
            from,
            to,
            arrow,
            label,
            style: _,
            from_virtual,
            to_virtual,
        } => {
            let mut message = json!({"from": from, "to": to, "arrow": arrow, "label": label});
            if let Some(ep) = from_virtual {
                message["from_virtual"] = virtual_endpoint_to_json(*ep);
            }
            if let Some(ep) = to_virtual {
                message["to_virtual"] = virtual_endpoint_to_json(*ep);
            }
            json!({"Message": message})
        }
        SequenceEventKind::Note {
            kind,
            position,
            target,
            text,
            ..
        } => {
            json!({"Note": {"kind": format!("{:?}", kind), "position": position, "target": target, "text": text}})
        }
        SequenceEventKind::GroupStart { kind, label } => {
            json!({"GroupStart": {"kind": kind, "label": label}})
        }
        SequenceEventKind::GroupEnd => json!("GroupEnd"),
        SequenceEventKind::Delay(v) => json!({"Delay": v}),
        SequenceEventKind::Divider(v) => json!({"Divider": v}),
        SequenceEventKind::Separator(v) => json!({"Separator": v}),
        SequenceEventKind::Spacer(pixels) => json!({"Spacer": pixels}),
        SequenceEventKind::NewPage(v) => json!({"NewPage": v}),
        SequenceEventKind::Autonumber(v) => json!({"Autonumber": v}),
        SequenceEventKind::Activate(v) => json!({"Activate": v}),
        SequenceEventKind::Deactivate(v) => json!({"Deactivate": v}),
        SequenceEventKind::Destroy(v) => json!({"Destroy": v}),
        SequenceEventKind::Create(v) => json!({"Create": v}),
        SequenceEventKind::Return { label, from, to } => {
            json!({"Return": {"label": label, "from": from, "to": to}})
        }
        SequenceEventKind::IncludePlaceholder(v) => json!({"IncludePlaceholder": v}),
        SequenceEventKind::DefinePlaceholder { name, value } => {
            json!({"DefinePlaceholder": {"name": name, "value": value}})
        }
        SequenceEventKind::UndefPlaceholder(v) => json!({"UndefPlaceholder": v}),
    }
}

fn virtual_endpoint_to_json(ep: VirtualEndpoint) -> Value {
    json!({
        "side": match ep.side {
            VirtualEndpointSide::Left => "left",
            VirtualEndpointSide::Right => "right",
        },
        "kind": match ep.kind {
            VirtualEndpointKind::Plain => "plain",
            VirtualEndpointKind::Circle => "circle",
            VirtualEndpointKind::Cross => "cross",
            VirtualEndpointKind::Filled => "filled",
            VirtualEndpointKind::Short => "short",
        }
    })
}

fn scene_to_json(model: &SequenceDocument) -> Value {
    let lane_spacing = 140;
    let lane_start = 100;
    let row_spacing = 40;
    let row_start = 120;
    let width = 200 + (model.participants.len() as i32 * lane_spacing);
    let height = 120 + (model.events.len() as i32 * row_spacing);

    let lanes = model
        .participants
        .iter()
        .enumerate()
        .map(|(idx, p)| SceneLane {
            id: p.id.clone(),
            display: p.display.clone(),
            role: model_role_to_str(p.role).to_string(),
            x: lane_start + (idx as i32 * lane_spacing),
        })
        .collect::<Vec<_>>();

    let rows = model
        .events
        .iter()
        .enumerate()
        .map(|(idx, e)| SceneRow {
            y: row_start + (idx as i32 * row_spacing),
            event: model_event_to_json(e),
        })
        .collect::<Vec<_>>();

    let scene = SceneDump {
        size: SceneSize { width, height },
        lanes,
        rows,
    };
    serde_json::to_value(scene).unwrap_or_else(|_| json!({"error": "scene serialization failed"}))
}

pub(crate) fn normalized_scene_to_json(model: &NormalizedDocument) -> Value {
    match model {
        NormalizedDocument::Sequence(sequence) => scene_to_json(sequence),
        NormalizedDocument::Family(family) => {
            let svg = puml::render::render_family_stub_svg(family);
            json!({
                "kind": "FamilyStub",
                "family": format!("{:?}", family.kind),
                "nodes": family
                    .nodes
                    .iter()
                    .map(|n| {
                        json!({
                            "kind": format!("{:?}", n.kind),
                            "name": n.name,
                            "alias": n.alias
                        })
                    })
                    .collect::<Vec<_>>(),
                "relations": family
                    .relations
                    .iter()
                    .map(|r| {
                        json!({
                            "from": r.from,
                            "to": r.to,
                            "arrow": r.arrow,
                            "label": r.label
                        })
                    })
                    .collect::<Vec<_>>(),
                "svg_preview": svg
            })
        }
        NormalizedDocument::FamilyPages(pages) => json!({
            "kind": "FamilyPages",
            "pages": pages.iter().map(|family| {
                let svg = puml::render::render_family_stub_svg(family);
                json!({
                    "kind": "FamilyStub",
                    "family": format!("{:?}", family.kind),
                    "nodes": family.nodes.iter().map(|n| json!({
                        "kind": format!("{:?}", n.kind),
                        "name": n.name,
                        "alias": n.alias
                    })).collect::<Vec<_>>(),
                    "relations": family.relations.iter().map(|r| json!({
                        "from": r.from,
                        "to": r.to,
                        "arrow": r.arrow,
                        "label": r.label
                    })).collect::<Vec<_>>(),
                    "svg_preview": svg
                })
            }).collect::<Vec<_>>()
        }),
        NormalizedDocument::Timeline(timeline) => {
            json!({
                "kind": "TimelineScene",
                "family": match timeline.kind {
                    DiagramKind::Salt => "Salt",
                    DiagramKind::Gantt => "Gantt",
                    DiagramKind::Chronology => "Chronology",
                    _ => "Timeline",
                },
                "tasks": timeline
                    .tasks
                    .iter()
                    .map(|t| json!({"name": t.name, "start_day": t.start_day, "workload_days": t.workload_days, "duration_days": t.duration_days, "resources": t.resources, "resource_allocations": t.resource_allocations.iter().map(|r| json!({"name": r.name, "load_percent": r.load_percent})).collect::<Vec<_>>()}))
                    .collect::<Vec<_>>(),
                "milestones": timeline.milestones.iter().map(|m| json!({"name": m.name, "happens_on": m.happens_on})).collect::<Vec<_>>(),
                "separators": timeline.separators.iter().map(|s| json!({"label": s.label, "target": s.target})).collect::<Vec<_>>(),
                "constraints": timeline.constraints.iter().map(|c| json!({"subject": c.subject, "kind": c.kind, "target": c.target})).collect::<Vec<_>>(),
                "closed_weekdays": timeline.closed_weekdays,
                "closed_ranges": timeline.closed_ranges.iter().map(|r| json!({"start_date": r.start_date, "end_date": r.end_date, "start_day": r.start_day, "end_day": r.end_day})).collect::<Vec<_>>(),
                "open_ranges": timeline.open_ranges.iter().map(|r| json!({"start_date": r.start_date, "end_date": r.end_date, "start_day": r.start_day, "end_day": r.end_day})).collect::<Vec<_>>(),
                "chronology_events": timeline.chronology_events.iter().map(|e| json!({"subject": e.subject, "when": e.when})).collect::<Vec<_>>(),
                "project_start": timeline.project_start,
                "project_start_day": timeline.project_start_day,
                "print_start": timeline.print_start,
                "print_end": timeline.print_end,
                "print_start_day": timeline.print_start_day,
                "print_end_day": timeline.print_end_day,
                "title": timeline.title,
                "header": timeline.header,
                "footer": timeline.footer,
                "caption": timeline.caption,
                "legend": timeline.legend,
                "svg_preview": puml::render::render_timeline_svg(timeline),
                "warnings": timeline.warnings.iter().map(|d| d.message.clone()).collect::<Vec<_>>()
            })
        }
        NormalizedDocument::State(state) => {
            let svg = puml::render::render_state_svg(state);
            json!({
                "kind": "StateDiagram",
                "nodes": state.nodes.len(),
                "transitions": state.transitions.len(),
                "svg_preview": svg
            })
        }
        other => normalized_model_to_json(other),
    }
}
