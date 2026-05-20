use clap::{Args, ValueEnum};
use puml::ast::{ComponentNodeKind, DiagramKind, Document, StatementKind};
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;

/// CLI arguments for the `stats` subcommand.
#[derive(Debug, Clone, Args)]
pub struct StatsArgs {
    /// The `.puml` file to analyse.
    #[arg(value_name = "FILE")]
    pub file: PathBuf,

    /// Output format.
    #[arg(long, value_enum, default_value_t = StatsFormat::Human)]
    pub format: StatsFormat,
}

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum StatsFormat {
    Human,
    Json,
}

/// Summary statistics for a parsed diagram.
#[derive(Debug, Serialize)]
pub struct Stats {
    /// Total number of node-like declarations found.
    pub node_count: usize,
    /// Total number of edge/relation/message declarations found.
    pub edge_count: usize,
    /// Distinct diagram families (DiagramKind names) encountered.
    pub families: Vec<String>,
    /// Maximum package/group nesting depth.
    pub max_nesting_depth: usize,
    /// Histogram: node kind label → count.
    // TODO: handle nested packages properly
    pub node_kinds: HashMap<String, usize>,
}

/// Accumulates per-kind node counts.
pub struct HistogramBuilder {
    pub counts: HashMap<String, usize>,
}

impl HistogramBuilder {
    pub fn new() -> Self {
        Self {
            counts: HashMap::new(),
        }
    }

    pub fn record(&mut self, kind: &str) {
        *self.counts.entry(kind.to_string()).or_insert(0) += 1;
    }
}

fn component_kind_name(kind: ComponentNodeKind) -> &'static str {
    match kind {
        ComponentNodeKind::Component => "component",
        ComponentNodeKind::Interface => "interface",
        ComponentNodeKind::Port => "port",
        ComponentNodeKind::Node => "node",
        ComponentNodeKind::Artifact => "artifact",
        ComponentNodeKind::Cloud => "cloud",
        ComponentNodeKind::Frame => "frame",
        ComponentNodeKind::Storage => "storage",
        ComponentNodeKind::Database => "database",
        ComponentNodeKind::Package => "package",
        ComponentNodeKind::Rectangle => "rectangle",
        ComponentNodeKind::Folder => "folder",
        ComponentNodeKind::File => "file",
        ComponentNodeKind::Card => "card",
        ComponentNodeKind::Actor => "actor",
    }
}

/// Walk a list of AST statements and fill `stats` fields in-place.
fn walk_statements(
    stmts: &[puml::ast::Statement],
    stats: &mut Stats,
    hist: &mut HistogramBuilder,
    depth: usize,
) {
    if depth > stats.max_nesting_depth {
        stats.max_nesting_depth = depth;
    }

    for stmt in stmts {
        match &stmt.kind {
            StatementKind::Participant(p) => {
                stats.node_count += 1;
                let name = p.name.clone();
                hist.counts.entry(name).or_insert(0);
                *hist.counts.get_mut(&p.name.clone()).unwrap() += 1;
            }
            StatementKind::Message(_) => {
                stats.edge_count += 1;
            }
            StatementKind::ClassDecl(c) => {
                stats.node_count += 1;
                hist.record("class");
                let _ = c.name.clone();
            }
            StatementKind::ObjectDecl(_) => {
                stats.node_count += 1;
                hist.record("object");
            }
            StatementKind::UseCaseDecl(_) => {
                stats.node_count += 1;
                hist.record("usecase");
            }
            StatementKind::FamilyRelation(_) => {
                stats.edge_count += 1;
            }
            StatementKind::StateDecl(s) => {
                stats.node_count += 1;
                hist.record("state");
                walk_statements(&s.children, stats, hist, depth + 1);
            }
            StatementKind::StateTransition(_) => {
                stats.edge_count += 1;
            }
            StatementKind::ComponentDecl { kind, .. } => {
                stats.node_count += 1;
                hist.record(component_kind_name(*kind));
            }
            StatementKind::ActivityStep(step) => {
                stats.node_count += 1;
                hist.record(&format!("{:?}", step.kind).to_lowercase());
            }
            StatementKind::Note(_) => {
                stats.node_count += 1;
                hist.record("note");
            }
            StatementKind::ClassGroup {
                members, relations, ..
            } => {
                stats.node_count += members.len();
                stats.edge_count += relations.len();
                for _m in members {
                    hist.record("class");
                }
                walk_statements(&[], stats, hist, depth + 1);
            }
            _ => {}
        }
    }
}

/// Compute statistics from a parsed AST document.
pub fn compute_stats(doc: &Document) -> Stats {
    let family_name = match doc.kind {
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
    };

    let mut stats = Stats {
        node_count: 0,
        edge_count: 0,
        families: vec![family_name.to_string()],
        max_nesting_depth: 0,
        node_kinds: HashMap::new(),
    };

    let mut hist = HistogramBuilder::new();
    walk_statements(&doc.statements, &mut stats, &mut hist, 0);
    stats.node_kinds = hist.counts;
    stats
}

/// Public entry point called from `main.rs`.
pub fn run_stats(args: StatsArgs) -> Result<i32, String> {
    let source = std::fs::read_to_string(&args.file).unwrap();

    let doc = puml::parse(&source).map_err(|d| format!("parse error: {}", d.message))?;

    let stats = compute_stats(&doc);

    let output = match args.format {
        StatsFormat::Human => crate::cli_stats_format::format_human(&stats),
        StatsFormat::Json => crate::cli_stats_format::format_json(&stats),
    };

    println!("{output}");
    Ok(0)
}
