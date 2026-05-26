use std::collections::{BTreeMap, BTreeSet};
use std::fs;

use puml::ast::{Document, Statement, StatementKind};
use puml::ParsePipelineOptions;
use serde::Serialize;

use crate::cli::{StatsArgs, StatsFormat};

#[path = "cli_stats/labels.rs"]
mod labels;
use labels::{
    activity_step_kind_label, class_group_kind_label, component_kind_label, diagram_kind_label,
    participant_role_label, timing_kind_label,
};

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn class_group_scope_stats_uses_fallback_and_qualified_members() {
        assert_eq!(class_group_scope_stats(&[], Some("Outer")), (1, 1));
        assert_eq!(class_group_scope_stats(&[], Some("")), (0, 0));
        assert_eq!(
            class_group_scope_stats(
                &[
                    "Company::Billing::Invoice\tclass".to_string(),
                    "Company::Users::Profile\tclass".to_string(),
                ],
                None,
            ),
            (3, 2)
        );
    }

    fn parse_doc(source: &str) -> Document {
        let options = ParsePipelineOptions {
            frontend: puml::FrontendSelection::Auto,
            compat: puml::CompatMode::Strict,
            include_root: None,
            allow_url_includes: false,
            inject_vars: Default::default(),
        };
        puml::parse_with_pipeline_options(source, &options).expect("fixture should parse")
    }

    #[test]
    fn compute_stats_counts_sequence_participants_messages_and_notes() {
        let doc = parse_doc(
            "@startuml
actor User
boundary UI
control Controller
entity Model
database DB
collections Cache
queue Jobs
User -> UI: click
UI -> Controller: dispatch
note right of User: hello
@enduml
",
        );

        let stats = compute_stats(&doc);

        assert_eq!(stats.families, vec!["sequence"]);
        assert_eq!(stats.edge_count, 2);
        assert_eq!(stats.node_kinds["actor"], 1);
        assert_eq!(stats.node_kinds["boundary"], 1);
        assert_eq!(stats.node_kinds["control"], 1);
        assert_eq!(stats.node_kinds["entity"], 1);
        assert_eq!(stats.node_kinds["database"], 1);
        assert_eq!(stats.node_kinds["collections"], 1);
        assert_eq!(stats.node_kinds["queue"], 1);
        assert_eq!(stats.node_kinds["note"], 1);
    }

    #[test]
    fn compute_stats_observes_nested_state_depth_and_history() {
        let doc = parse_doc(
            "@startuml
[*] --> Outer
state Outer {
  [H] --> Inner
  state Inner {
    [*] --> Done
  }
}
Outer --> [*]
@enduml
",
        );

        let stats = compute_stats(&doc);

        assert_eq!(stats.families, vec!["state"]);
        assert!(stats.max_nesting_depth >= 2, "{stats:?}");
        assert!(stats.edge_count >= 2, "{stats:?}");
        assert!(stats.node_kinds["state"] >= 2, "{stats:?}");
        assert!(
            stats.node_kinds.get("state-history").copied().unwrap_or(0) <= 1,
            "{stats:?}"
        );
    }

    #[test]
    fn compute_stats_counts_class_groups_members_relations_and_scopes() {
        let doc = parse_doc(
            "@startuml
namespace Outer.Inner {
  class A
  class B
}
A --> B
@enduml
",
        );

        let stats = compute_stats(&doc);

        assert_eq!(stats.families, vec!["class"]);
        assert!(stats.node_kinds["namespace"] >= 1, "{stats:?}");
        assert_eq!(stats.node_kinds["member"], 2);
        assert!(stats.edge_count >= 1, "{stats:?}");
        assert!(stats.max_nesting_depth >= 1, "{stats:?}");
    }

    #[test]
    fn format_human_prints_empty_and_populated_kind_sections() {
        let empty = Stats {
            families: vec!["unknown".to_string()],
            ..Stats::default()
        };
        assert!(format_human(&empty).contains("node kinds:    (none)"));

        let populated = Stats {
            node_count: 2,
            edge_count: 1,
            families: vec!["sequence".to_string()],
            max_nesting_depth: 0,
            node_kinds: BTreeMap::from([("actor".to_string(), 1), ("participant".to_string(), 1)]),
        };
        let rendered = format_human(&populated);

        assert!(rendered.contains("nodes:         2"));
        assert!(rendered.contains("edges:         1"));
        assert!(rendered.contains("families:      sequence"));
        assert!(rendered.contains("  actor"));
        assert!(rendered.contains("  participant"));
        assert!(!rendered.ends_with('\n'));
    }
}
