use puml::ast::{
    DiagramKind, Document, Group, Message, Note, ParticipantDecl,
    ParticipantRole as AstParticipantRole, Statement, StatementKind,
};
use serde_json::{json, Value};

pub(crate) fn ast_to_json(doc: &Document) -> Value {
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
            DiagramKind::Stdlib => "Stdlib",
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
            name,
            alias,
            resources,
            ..
        } => json!({"GanttTaskDecl": {"name": name, "alias": alias, "resources": resources}}),
        StatementKind::GanttCompound {
            name,
            alias,
            resources,
            clauses,
            after_previous,
        } => {
            json!({"GanttCompound": {"name": name, "alias": alias, "resources": resources, "clauses": clauses, "after_previous": after_previous}})
        }
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
        StatementKind::GanttNamedDate { date, label } => {
            json!({"GanttNamedDate": {"date": date, "label": label}})
        }
        StatementKind::ChronologyHappensOn {
            subject,
            when,
            end,
            color,
            bracket,
        } => {
            json!({"ChronologyHappensOn": {"subject": subject, "when": when, "end": end, "color": color, "bracket": bracket}})
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
        StatementKind::AllowMixing => json!("AllowMixing"),
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
        StatementKind::SpriteDef(sprite) => json!({
            "SpriteDef": {
                "name": sprite.name,
                "width": sprite.width,
                "height": sprite.height,
                "gray_levels": sprite.gray_levels
            }
        }),
        StatementKind::ListSprites => json!("ListSprites"),
        StatementKind::StdlibInventory => json!("StdlibInventory"),
        StatementKind::UnsupportedSyntax(v) => json!({"UnsupportedSyntax": v}),
        StatementKind::DeferredRaw(v) => json!({"DeferredRaw": v}),
        StatementKind::CommentLowered(v) => json!({"CommentLowered": v}),
        StatementKind::MalformedSyntax(v) => json!({"MalformedSyntax": v}),
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
                puml::ast::VirtualEndpointKind::Short => "short",
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
                puml::ast::VirtualEndpointKind::Short => "short",
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
