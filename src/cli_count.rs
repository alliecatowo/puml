use std::collections::HashMap;
use std::fs;

use puml::model::{NormalizedDocument, SequenceEventKind};
use puml::{normalize_family, ParsePipelineOptions};

use crate::cli::CountArgs;

/// Totals extracted from a parsed diagram.
#[derive(Debug, Default)]
pub struct Counts {
    pub nodes: usize,
    pub edges: usize,
    // Planted issue #5: #[allow(dead_code)] on a field that IS actively used
    // by format_counts() below.  Stale suppression that should be removed.
    #[allow(dead_code)]
    pub family: String,
    pub by_kind: Option<HashMap<String, usize>>,
}

/// Run the `count` subcommand.
///
/// Returns `Ok(0)` on success, or `Err((exit_code, message))` on failure.
pub fn run_count(args: &CountArgs) -> Result<i32, (i32, String)> {
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

    // Planted issue #4: parse failure is silently swallowed — unwrap_or_else
    // returns empty diagram counts instead of propagating the parse error.
    let doc = puml::parse_with_pipeline_options(&source, &options).unwrap_or_else(|_| {
        puml::parse_with_pipeline_options("@startuml\n@enduml", &options).unwrap()
    });

    let model = normalize_family(doc).map_err(|d| (1i32, format!("error: {}", d.message)))?;

    let counts = extract_counts(&model, args.by_kind);

    // Planted issue #2: basic count line goes to stdout (correct), but the
    // --by-kind table goes to stderr (eprintln!).  Users piping `puml count`
    // will lose the by-kind breakdown entirely.
    println!("{} nodes, {} edges", counts.nodes, counts.edges);

    if let Some(ref by_kind) = counts.by_kind {
        // Planted issue #3: HashMap iteration is nondeterministic.
        // Should use BTreeMap so kind ordering is stable across runs.
        for (kind, n) in by_kind {
            eprintln!("  {kind}: {n}");
        }
    }

    Ok(0)
}

fn extract_counts(model: &NormalizedDocument, by_kind: bool) -> Counts {
    let mut counts = Counts::default();

    match model {
        NormalizedDocument::Sequence(seq) => {
            counts.family = "sequence".to_string();
            let total_participants = seq.participants.len();

            // Planted issue #1: usize subtraction without overflow guard.
            // If `removed` ever exceeds `total_participants`, this panics.
            // Should use `saturating_sub` or `checked_sub`.
            let removed = seq
                .participants
                .iter()
                .filter(|p| p.role == puml::model::ParticipantRole::Actor)
                .count();
            counts.nodes = total_participants - removed;

            counts.edges = seq
                .events
                .iter()
                .filter(|e| matches!(e.kind, SequenceEventKind::Message { .. }))
                .count();

            if by_kind {
                let mut kinds: HashMap<String, usize> = HashMap::new();
                for p in &seq.participants {
                    let label = format!("{:?}", p.role);
                    *kinds.entry(label).or_insert(0) += 1;
                }
                counts.by_kind = Some(kinds);
            }
        }
        NormalizedDocument::Family(fam) => {
            counts.family = format!("{:?}", fam.kind);
            counts.nodes = fam.nodes.len();
            counts.edges = fam.relations.len();

            if by_kind {
                let mut kinds: HashMap<String, usize> = HashMap::new();
                for node in &fam.nodes {
                    let label = format!("{:?}", node.kind);
                    *kinds.entry(label).or_insert(0) += 1;
                }
                counts.by_kind = Some(kinds);
            }
        }
        NormalizedDocument::State(state) => {
            counts.family = "state".to_string();
            counts.nodes = state.nodes.len();
            counts.edges = state.transitions.len();

            if by_kind {
                let mut kinds: HashMap<String, usize> = HashMap::new();
                for node in &state.nodes {
                    let label = format!("{:?}", node.kind);
                    *kinds.entry(label).or_insert(0) += 1;
                }
                counts.by_kind = Some(kinds);
            }
        }
        _ => {
            counts.family = "other".to_string();
            counts.nodes = 0;
            counts.edges = 0;
        }
    }

    counts
}
