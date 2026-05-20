use puml::ast::{
    DiagramKind, Document, Group, Message, Note, ParticipantDecl,
    ParticipantRole as AstParticipantRole, Statement, StatementKind,
};
use puml::model::{
    Participant, ParticipantRole as ModelParticipantRole, SequenceDocument, SequenceEvent,
    SequenceEventKind, StateDocument, TimelineDocument, VirtualEndpoint, VirtualEndpointKind,
    VirtualEndpointSide,
};
use puml::{render, NormalizedDocument};
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

pub fn ast_to_json(doc: &Document) -> Value {
    json!({
        "kind": match doc.kind {
            DiagramKind::Sequence => "Sequence",
            DiagramKind::Class => "Class",
            DiagramKind::Object => "Object",
            DiagramKind::UseCase => "UseCase",
            DiagramKind::MindMap => "MindMap",
            DiagramKind::Wbs => "Wbs",
            DiagramKind::Gantt => "Gantt",
            DiagramKind::Chronology => "Chronology",
            DiagramKind::Component => "Component",
            DiagramKind::Deployment => "Deployment",
            DiagramKind::State => "State",
            DiagramKind::Activity => "Activity",
            DiagramKind::Timing => "Timing",
            DiagramKind::Salt => "Salt",
            DiagramKind::Json => "Json",
            DiagramKind::Yaml => "Yaml",
            DiagramKind::Nwdiag => "Nwdiag",
            DiagramKind::Archimate => "Archimate",
            DiagramKind::Regex => "Regex",
            DiagramKind::Ebnf => "Ebnf",
            DiagramKind::Math => "Math",
            DiagramKind::Sdl => "Sdl",
            DiagramKind::Ditaa => "Ditaa",
            DiagramKind::Chart => "Chart",
            DiagramKind::Chen => "Chen",
            DiagramKind::Unknown => "Unknown",
        },
        "statements": doc.statements.iter().map(statement_to_json).collect::<Vec<_>>()
    })
}

fn statement_to_json(s: &Statement) -> Value {
    json!({
        "span": {"start": s.span.start, "end": s.span.end},
        "kind": statement_kind_to_json(&s.kind)
    })
}

fn statement_kind_to_json(kind: &StatementKind) -> Value {
    match kind {
        StatementKind::Participant(p) => json!({"Participant": participant_decl_to_json(p)}),
        StatementKind::Message(m) => json!({"Message": message_to_json(m)}),
        StatementKind::ClassDecl(v) => {
            json!({"ClassDecl": {"name": v.name, "alias": v.alias, "members": v.members}})
        }
        StatementKind::ObjectDecl(v) => {
            json!({"ObjectDecl": {"name": v.name, "alias": v.alias, "members": v.members}})
        }
        StatementKind::UseCaseDecl(v) => {
            json!({"UseCaseDecl": {"name": v.name, "alias": v.alias, "members": v.members}})
        }
        StatementKind::FamilyRelation(v) => {
            json!({"FamilyRelation": {"from": v.from, "to": v.to, "arrow": v.arrow, "label": v.label}})
        }
        StatementKind::StateDecl(v) => {
            json!({"StateDecl": {"name": v.name, "alias": v.alias, "stereotype": v.stereotype}})
        }
        StatementKind::StateTransition(v) => {
            json!({"StateTransition": {"from": v.from, "to": v.to, "label": v.label}})
        }
        StatementKind::StateInternalAction(v) => {
            json!({"StateInternalAction": {"state": v.state, "kind": v.kind, "action": v.action}})
        }
        StatementKind::StateRegionDivider => json!("StateRegionDivider"),
        StatementKind::StateHistory { deep } => json!({"StateHistory": {"deep": deep}}),
        StatementKind::GanttTaskDecl {
            name, resources, ..
        } => json!({"GanttTaskDecl": {"name": name, "resources": resources}}),
        StatementKind::GanttMilestoneDecl { name, happens_on } => {
            json!({"GanttMilestoneDecl": {"name": name, "happens_on": happens_on}})
        }
        StatementKind::GanttConstraint {
            subject,
            kind,
            target,
        } => {
            json!({"GanttConstraint": {"subject": subject, "kind": kind, "target": target}})
        }
        StatementKind::GanttCalendarClosed { day } => {
            json!({"GanttCalendarClosed": {"day": day}})
        }
        StatementKind::GanttCalendarOpen { day } => {
            json!({"GanttCalendarOpen": {"day": day}})
        }
        StatementKind::GanttCalendarClosedDateRange {
            start_date,
            end_date,
        } => json!({
            "GanttCalendarClosedDateRange": {
                "start_date": start_date,
                "end_date": end_date
            }
        }),
        StatementKind::GanttCalendarOpenDateRange {
            start_date,
            end_date,
        } => json!({
            "GanttCalendarOpenDateRange": {
                "start_date": start_date,
                "end_date": end_date
            }
        }),
        StatementKind::ChronologyHappensOn { subject, when } => {
            json!({"ChronologyHappensOn": {"subject": subject, "when": when}})
        }
        StatementKind::Note(n) => json!({"Note": note_to_json(n)}),
        StatementKind::Group(g) => json!({"Group": group_to_json(g)}),
        StatementKind::Title(v) => json!({"Title": v}),
        StatementKind::Header(v) => json!({"Header": v}),
        StatementKind::Footer(v) => json!({"Footer": v}),
        StatementKind::Caption(v) => json!({"Caption": v}),
        StatementKind::Legend(v) => json!({"Legend": v}),
        StatementKind::Theme(v) => json!({"Theme": v}),
        StatementKind::Pragma(v) => json!({"Pragma": v}),
        StatementKind::SkinParam { key, value } => {
            json!({"SkinParam": {"key": key, "value": value}})
        }
        StatementKind::Footbox(v) => json!({"Footbox": v}),
        StatementKind::Delay(v) => json!({"Delay": v}),
        StatementKind::Divider(v) => json!({"Divider": v}),
        StatementKind::Separator(v) => json!({"Separator": v}),
        StatementKind::Spacer(pixels) => json!({"Spacer": pixels}),
        StatementKind::NewPage(v) => json!({"NewPage": v}),
        StatementKind::IgnoreNewPage => json!("IgnoreNewPage"),
        StatementKind::Autonumber(v) => json!({"Autonumber": v}),
        StatementKind::Activate(v) => json!({"Activate": v}),
        StatementKind::Deactivate(v) => json!({"Deactivate": v}),
        StatementKind::Destroy(v) => json!({"Destroy": v}),
        StatementKind::Create(v) => json!({"Create": v}),
        StatementKind::Return(v) => json!({"Return": v}),
        StatementKind::Include(v) => json!({"Include": v}),
        StatementKind::Define { name, value } => json!({"Define": {"name": name, "value": value}}),
        StatementKind::Undef(v) => json!({"Undef": v}),
        StatementKind::Unknown(v) => json!({"Unknown": v}),
        StatementKind::JsonProjection { alias, body } => json!({
            "JsonProjection": {"alias": alias, "body": body}
        }),
        StatementKind::YamlProjection { alias, body } => json!({
            "YamlProjection": {"alias": alias, "body": body}
        }),
        other => json!({"Other": format!("{other:?}")}),
    }
}

fn participant_decl_to_json(p: &ParticipantDecl) -> Value {
    json!({
        "role": ast_role_to_str(p.role),
        "name": p.name,
        "alias": p.alias,
        "display": p.display
    })
}

fn message_to_json(m: &Message) -> Value {
    let mut message = json!({"from": m.from, "to": m.to, "arrow": m.arrow, "label": m.label});
    if let Some(ep) = m.from_virtual {
        message["from_virtual"] = json!({
            "side": match ep.side {
                puml::ast::VirtualEndpointSide::Left => "left",
                puml::ast::VirtualEndpointSide::Right => "right",
            },
            "kind": match ep.kind {
                puml::ast::VirtualEndpointKind::Plain => "plain",
                puml::ast::VirtualEndpointKind::Circle => "circle",
                puml::ast::VirtualEndpointKind::Cross => "cross",
                puml::ast::VirtualEndpointKind::Filled => "filled",
            }
        });
    }
    if let Some(ep) = m.to_virtual {
        message["to_virtual"] = json!({
            "side": match ep.side {
                puml::ast::VirtualEndpointSide::Left => "left",
                puml::ast::VirtualEndpointSide::Right => "right",
            },
            "kind": match ep.kind {
                puml::ast::VirtualEndpointKind::Plain => "plain",
                puml::ast::VirtualEndpointKind::Circle => "circle",
                puml::ast::VirtualEndpointKind::Cross => "cross",
                puml::ast::VirtualEndpointKind::Filled => "filled",
            }
        });
    }
    message
}

fn note_to_json(n: &Note) -> Value {
    json!({"position": n.position, "target": n.target, "text": n.text})
}

fn group_to_json(g: &Group) -> Value {
    json!({"kind": g.kind, "label": g.label})
}

fn ast_role_to_str(role: AstParticipantRole) -> &'static str {
    match role {
        AstParticipantRole::Participant => "Participant",
        AstParticipantRole::Actor => "Actor",
        AstParticipantRole::Boundary => "Boundary",
        AstParticipantRole::Control => "Control",
        AstParticipantRole::Entity => "Entity",
        AstParticipantRole::Database => "Database",
        AstParticipantRole::Collections => "Collections",
        AstParticipantRole::Queue => "Queue",
    }
}

pub fn normalized_model_to_json(model: &NormalizedDocument) -> Value {
    match model {
        NormalizedDocument::Sequence(sequence) => model_to_json(sequence),
        NormalizedDocument::Family(family) => family_model_to_json(family),
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
        NormalizedDocument::Chen(doc) => json!({"kind": "Chen", "warnings": doc.warnings.len()}),
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
        "chronology_events": model
            .chronology_events
            .iter()
            .map(|e| json!({"subject": e.subject, "when": e.when}))
            .collect::<Vec<_>>(),
        "project_start": model.project_start,
        "project_start_day": model.project_start_day,
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

pub fn normalized_scene_to_json(model: &NormalizedDocument) -> Value {
    match model {
        NormalizedDocument::Sequence(sequence) => scene_to_json(sequence),
        NormalizedDocument::Family(family) => {
            let svg = render::render_family_stub_svg(family);
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
                "title": timeline.title,
                "header": timeline.header,
                "footer": timeline.footer,
                "caption": timeline.caption,
                "legend": timeline.legend,
                "svg_preview": render::render_timeline_svg(timeline),
                "warnings": timeline.warnings.iter().map(|d| d.message.clone()).collect::<Vec<_>>()
            })
        }
        NormalizedDocument::State(state) => {
            let svg = render::render_state_svg(state);
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
