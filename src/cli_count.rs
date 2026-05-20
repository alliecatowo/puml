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

    let doc = puml::parse_with_pipeline_options(&source, &options)
        .map_err(|e| (1i32, format!("error: could not parse '{}': {e}", args.file.display())))?;

    let model = normalize_family(doc).map_err(|d| (1i32, format!("error: {}", d.message)))?;

    let counts = extract_counts(&model, args.by_kind);

    println!("{} nodes, {} edges", counts.nodes, counts.edges);

    if let Some(ref by_kind) = counts.by_kind {
        let mut sorted: Vec<(&String, &usize)> = by_kind.iter().collect();
        sorted.sort_by_key(|(k, _)| k.as_str());
        for (kind, n) in sorted {
            println!("  {kind}: {n}");
        }
    }

    Ok(0)
}

fn extract_counts(model: &NormalizedDocument, by_kind: bool) -> Counts {
    let mut counts = Counts::default();

    match model {
        NormalizedDocument::Sequence(seq) => {
            let total_participants = seq.participants.len();
            let removed = seq
                .participants
                .iter()
                .filter(|p| p.role == puml::model::ParticipantRole::Actor)
                .count();
            counts.nodes = total_participants.saturating_sub(removed);

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
            eprintln!(
                "warning: counting is not yet supported for this diagram family; \
                 showing 0 nodes, 0 edges"
            );
        }
    }

    counts
}
