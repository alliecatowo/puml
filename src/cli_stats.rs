use std::collections::{BTreeMap, BTreeSet};
use std::fs;

use puml::ast::{
    ActivityStepKind, ComponentNodeKind, DiagramKind, Document, ParticipantRole, Statement,
    StatementKind, TimingDeclKind,
};
use puml::ParsePipelineOptions;
use serde::Serialize;

use crate::cli::{StatsArgs, StatsFormat};

/// Structural statistics collected from the parsed AST.
#[derive(Debug, Default, Serialize)]
pub struct Stats {
    pub node_count: usize,
    pub edge_count: usize,
    pub families: Vec<String>,
    pub max_nesting_depth: usize,
    pub node_kinds: BTreeMap<String, usize>,
}

#[derive(Debug, Default)]
struct StatsBuilder {
    stats: Stats,
}

impl StatsBuilder {
    fn record_node(&mut self, kind: &'static str) {
        self.stats.node_count += 1;
        *self.stats.node_kinds.entry(kind.to_string()).or_insert(0) += 1;
    }

    fn record_nodes(&mut self, kind: &'static str, count: usize) {
        self.stats.node_count += count;
        *self.stats.node_kinds.entry(kind.to_string()).or_insert(0) += count;
    }

    fn record_edge(&mut self) {
        self.stats.edge_count += 1;
    }

    fn observe_depth(&mut self, depth: usize) {
        self.stats.max_nesting_depth = self.stats.max_nesting_depth.max(depth);
    }

    fn finish(self) -> Stats {
        self.stats
    }
}

pub fn run_stats(args: &StatsArgs) -> Result<i32, (i32, String)> {
    let source = fs::read_to_string(&args.file).map_err(|e| {
        (
            2i32,
            format!("error: could not read '{}': {e}", args.file.display()),
        )
    })?;

    let options = ParsePipelineOptions {
        frontend: puml::FrontendSelection::Auto,
        compat: puml::CompatMode::Strict,
        determinism: puml::DeterminismMode::Strict,
        include_root: args.file.parent().map(|p| p.to_path_buf()),
        allow_url_includes: false,
        inject_vars: Default::default(),
    };

    let doc = puml::parse_with_pipeline_options(&source, &options).map_err(|e| {
        (
            1i32,
            format!(
                "error: could not parse '{}': {}",
                args.file.display(),
                e.render_with_source(&source)
            ),
        )
    })?;

    let stats = compute_stats(&doc);
    match args.format {
        StatsFormat::Human => println!("{}", format_human(&stats)),
        StatsFormat::Json => {
            let json = serde_json::to_string_pretty(&stats)
                .map_err(|e| (3i32, format!("error: could not serialize stats: {e}")))?;
            println!("{json}");
        }
    }

    Ok(0)
}

pub fn compute_stats(doc: &Document) -> Stats {
    let mut builder = StatsBuilder::default();
    builder
        .stats
        .families
        .push(diagram_kind_label(doc.kind).to_string());
    walk_statements(&doc.statements, &mut builder, 0);
    builder.finish()
}

fn format_human(stats: &Stats) -> String {
    let mut out = String::new();

    out.push_str(&format!("nodes:         {}\n", stats.node_count));
    out.push_str(&format!("edges:         {}\n", stats.edge_count));
    out.push_str(&format!("families:      {}\n", stats.families.join(", ")));
    out.push_str(&format!("max nesting:   {}\n", stats.max_nesting_depth));

    if stats.node_kinds.is_empty() {
        out.push_str("node kinds:    (none)");
    } else {
        out.push_str("node kinds:\n");
        for (kind, count) in &stats.node_kinds {
            out.push_str(&format!("  {kind:<20} {count}\n"));
        }
        out.truncate(out.trim_end().len());
    }

    out
}

fn walk_statements(stmts: &[Statement], builder: &mut StatsBuilder, depth: usize) {
    builder.observe_depth(depth);

    for stmt in stmts {
        match &stmt.kind {
            StatementKind::Participant(p) => builder.record_node(participant_role_label(p.role)),
            StatementKind::Message(_) => builder.record_edge(),
            StatementKind::ClassDecl(_) => builder.record_node("class"),
            StatementKind::ObjectDecl(_) => builder.record_node("object"),
            StatementKind::UseCaseDecl(_) => builder.record_node("use-case"),
            StatementKind::FamilyRelation(_) => builder.record_edge(),
            StatementKind::StateDecl(state) => {
                builder.record_node("state");
                walk_statements(&state.children, builder, depth + 1);
            }
            StatementKind::StateTransition(_) => builder.record_edge(),
            StatementKind::StateHistory { .. } => builder.record_node("state-history"),
            StatementKind::ComponentDecl { kind, .. } => {
                builder.record_node(component_kind_label(*kind));
            }
            StatementKind::ActivityStep(step) => {
                builder.record_node(activity_step_kind_label(&step.kind));
            }
            StatementKind::TimingDecl { kind, .. } => builder.record_node(timing_kind_label(*kind)),
            StatementKind::TimingEvent { .. } => builder.record_edge(),
            StatementKind::Note(_) => builder.record_node("note"),
            StatementKind::Group(_) => builder.observe_depth(depth + 1),
            StatementKind::GanttTaskDecl { .. } => builder.record_node("gantt-task"),
            StatementKind::GanttCompound { .. } => builder.record_node("gantt-task"),
            StatementKind::GanttMilestoneDecl { .. } => builder.record_node("gantt-milestone"),
            StatementKind::GanttConstraint { .. } => builder.record_edge(),
            StatementKind::ChronologyHappensOn { .. } => builder.record_node("chronology-event"),
            StatementKind::ClassGroup {
                kind,
                label,
                members,
                relations,
            } => {
                let (group_count, max_scope_depth) =
                    class_group_scope_stats(members, label.as_deref());
                builder.record_nodes(class_group_kind_label(kind), group_count);
                builder.record_nodes("member", members.len());
                builder.stats.edge_count += relations.len();
                builder.observe_depth(depth + max_scope_depth);
            }
            StatementKind::AssociationClass { .. } => builder.record_edge(),
            _ => {}
        }
    }
}

fn participant_role_label(role: ParticipantRole) -> &'static str {
    match role {
        ParticipantRole::Participant => "participant",
        ParticipantRole::Actor => "actor",
        ParticipantRole::Boundary => "boundary",
        ParticipantRole::Control => "control",
        ParticipantRole::Entity => "entity",
        ParticipantRole::Database => "database",
        ParticipantRole::Collections => "collections",
        ParticipantRole::Queue => "queue",
    }
}

fn component_kind_label(kind: ComponentNodeKind) -> &'static str {
    match kind {
        ComponentNodeKind::Action => "action",
        ComponentNodeKind::Agent => "agent",
        ComponentNodeKind::Component => "component",
        ComponentNodeKind::Interface => "interface",
        ComponentNodeKind::Port => "port",
        ComponentNodeKind::Node => "node",
        ComponentNodeKind::Artifact => "artifact",
        ComponentNodeKind::Boundary => "boundary",
        ComponentNodeKind::Cloud => "cloud",
        ComponentNodeKind::Circle => "circle",
        ComponentNodeKind::Collections => "collections",
        ComponentNodeKind::Frame => "frame",
        ComponentNodeKind::Storage => "storage",
        ComponentNodeKind::Container => "container",
        ComponentNodeKind::Control => "control",
        ComponentNodeKind::Database => "database",
        ComponentNodeKind::Entity => "entity",
        ComponentNodeKind::Package => "package",
        ComponentNodeKind::Rectangle => "rectangle",
        ComponentNodeKind::Folder => "folder",
        ComponentNodeKind::File => "file",
        ComponentNodeKind::Card => "card",
        ComponentNodeKind::Actor => "actor",
        ComponentNodeKind::Hexagon => "hexagon",
        ComponentNodeKind::Label => "label",
        ComponentNodeKind::Person => "person",
        ComponentNodeKind::Process => "process",
        ComponentNodeKind::Queue => "queue",
        ComponentNodeKind::Stack => "stack",
        ComponentNodeKind::UseCase => "use-case",
    }
}

fn activity_step_kind_label(kind: &ActivityStepKind) -> &'static str {
    match kind {
        ActivityStepKind::Start => "activity-start",
        ActivityStepKind::Stop => "activity-stop",
        ActivityStepKind::End => "activity-end",
        ActivityStepKind::Action => "activity-action",
        ActivityStepKind::Arrow => "activity-arrow",
        ActivityStepKind::Connector => "activity-connector",
        ActivityStepKind::Note => "activity-note",
        ActivityStepKind::Kill => "activity-kill",
        ActivityStepKind::Detach => "activity-detach",
        ActivityStepKind::IfStart => "activity-if",
        ActivityStepKind::Else => "activity-else",
        ActivityStepKind::EndIf => "activity-endif",
        ActivityStepKind::RepeatStart => "activity-repeat",
        ActivityStepKind::RepeatWhile => "activity-repeat-while",
        ActivityStepKind::WhileStart => "activity-while",
        ActivityStepKind::EndWhile => "activity-endwhile",
        ActivityStepKind::Fork => "activity-fork",
        ActivityStepKind::ForkAgain => "activity-fork-again",
        ActivityStepKind::EndFork => "activity-endfork",
        ActivityStepKind::PartitionStart => "activity-partition",
        ActivityStepKind::PartitionEnd => "activity-partition-end",
    }
}

fn timing_kind_label(kind: TimingDeclKind) -> &'static str {
    match kind {
        TimingDeclKind::Concise => "timing-concise",
        TimingDeclKind::Robust => "timing-robust",
        TimingDeclKind::Clock => "timing-clock",
        TimingDeclKind::Binary => "timing-binary",
    }
}

fn class_group_kind_label(kind: &str) -> &'static str {
    match kind {
        "namespace" => "namespace",
        "package" => "package",
        "together" => "together",
        _ => "group",
    }
}

fn class_group_scope_stats(members: &[String], fallback_label: Option<&str>) -> (usize, usize) {
    let mut scopes = BTreeSet::new();
    let mut max_depth = 0;

    for member in members {
        let qualified_name = member.split('\t').next().unwrap_or(member);
        let parts: Vec<&str> = qualified_name.split("::").collect();
        if parts.len() > 1 {
            max_depth = max_depth.max(parts.len() - 1);
            for depth in 1..parts.len() {
                scopes.insert(parts[..depth].join("::"));
            }
        }
    }

    if scopes.is_empty() {
        if let Some(label) = fallback_label.filter(|label| !label.is_empty()) {
            scopes.insert(label.to_string());
            max_depth = 1;
        }
    }

    (scopes.len(), max_depth)
}

fn diagram_kind_label(kind: DiagramKind) -> &'static str {
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
        DiagramKind::Regex => "regex",
        DiagramKind::Ebnf => "ebnf",
        DiagramKind::Math => "math",
        DiagramKind::Sdl => "sdl",
        DiagramKind::Ditaa => "ditaa",
        DiagramKind::Chart => "chart",
        DiagramKind::Unknown => "unknown",
    }
}
