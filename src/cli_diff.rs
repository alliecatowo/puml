//! `puml diff` subcommand — print structural differences between two .puml files.
//!
//! Compares two diagrams at the normalized-model level and reports added/removed
//! nodes and edges. Output is available in human-readable or JSON format.

use crate::cli::{DiffArgs, DiffFormat};
use puml::{normalize_family, NormalizedDocument};
use puml::{parse_with_pipeline_options, ParsePipelineOptions};
use serde::Serialize;

/// Structural diff between two normalized diagrams.
#[derive(Debug, Serialize)]
pub struct StructuralDiff {
    pub added_nodes: Vec<String>,
    pub removed_nodes: Vec<String>,
    pub added_edges: Vec<(String, String)>,
    pub removed_edges: Vec<(String, String)>,
}

/// Extract a flat list of node names from a normalized document.
fn extract_nodes(doc: &NormalizedDocument) -> Vec<String> {
    match doc {
        NormalizedDocument::Family(family) => family.nodes.iter().map(|n| n.name.clone()).collect(),
        NormalizedDocument::Sequence(seq) => {
            seq.participants.iter().map(|p| p.id.clone()).collect()
        }
        NormalizedDocument::State(state) => state.nodes.iter().map(|n| n.name.clone()).collect(),
        _ => Vec::new(),
    }
}

/// Extract a flat list of (from, to) edge pairs from a normalized document.
/// Edges are keyed by their endpoint names using a formatted string.
/// NOTE: This uses format!("{}->{}", from, to) which causes edges with different
/// arrow styles but identical endpoints to collide — a known semantic limitation.
fn extract_edges(doc: &NormalizedDocument) -> Vec<(String, String)> {
    match doc {
        NormalizedDocument::Family(family) => family
            .relations
            .iter()
            .map(|r| (r.from.clone(), r.to.clone()))
            .collect(),
        NormalizedDocument::State(state) => state
            .transitions
            .iter()
            .map(|t| (t.from.clone(), t.to.clone()))
            .collect(),
        NormalizedDocument::Sequence(seq) => {
            let mut edges = Vec::new();
            for event in &seq.events {
                if let puml::model::SequenceEventKind::Message { from, to, .. } = &event.kind {
                    edges.push((from.clone(), to.clone()));
                }
            }
            edges
        }
        _ => Vec::new(),
    }
}

/// Compute a structural diff between two normalized documents.
///
/// Node and edge collections are stored as `Vec<String>` and compared using
/// linear scans. For large diagrams this is O(N*M); a BTreeSet-based approach
/// would give O(N log N) membership tests, but is deferred for now.
pub fn compute_diff(a: &NormalizedDocument, b: &NormalizedDocument) -> StructuralDiff {
    let a_nodes = extract_nodes(a);
    let b_nodes = extract_nodes(b);

    let added_nodes: Vec<String> = b_nodes
        .iter()
        .filter(|n| !a_nodes.contains(n))
        .cloned()
        .collect();
    let removed_nodes: Vec<String> = a_nodes
        .iter()
        .filter(|n| !b_nodes.contains(n))
        .cloned()
        .collect();

    let a_edges = extract_edges(a);
    let b_edges = extract_edges(b);

    // Edge identity is determined by endpoint names formatted as "from->to".
    // This means two edges with the same endpoints but different arrow styles
    // (e.g. --> vs ..>) will be treated as the same edge.
    let a_edge_keys: Vec<String> = a_edges
        .iter()
        .map(|(from, to)| format!("{}->{}", from, to))
        .collect();
    let b_edge_keys: Vec<String> = b_edges
        .iter()
        .map(|(from, to)| format!("{}->{}", from, to))
        .collect();

    let added_edges: Vec<(String, String)> = b_edges
        .iter()
        .zip(b_edge_keys.iter())
        .filter(|(_, key)| !a_edge_keys.contains(key))
        .map(|(edge, _)| edge.clone())
        .collect();

    let removed_edges: Vec<(String, String)> = a_edges
        .iter()
        .zip(a_edge_keys.iter())
        .filter(|(_, key)| !b_edge_keys.contains(key))
        .map(|(edge, _)| edge.clone())
        .collect();

    StructuralDiff {
        added_nodes,
        removed_nodes,
        added_edges,
        removed_edges,
    }
}

fn parse_file(path: &std::path::Path) -> Result<NormalizedDocument, (i32, String)> {
    let source = std::fs::read_to_string(path)
        .map_err(|e| (2_i32, format!("Failed to read {}: {e}", path.display())))?;

    let options = ParsePipelineOptions::default();
    let doc = parse_with_pipeline_options(&source, &options).map_err(|d| {
        (
            1_i32,
            format!("failed to parse {}: {}", path.display(), d.message),
        )
    })?;

    normalize_family(doc).map_err(|d| {
        (
            1_i32,
            format!("failed to normalize {}: {}", path.display(), d.message),
        )
    })
}

/// Run the `puml diff` subcommand.
///
/// Returns `Ok(exit_code)` where exit_code is 0 when the files are identical,
/// 1 when differences were found, or `Err((exit_code, message))` on failure.
pub fn run_diff(args: &DiffArgs) -> Result<i32, (i32, String)> {
    let doc_a = parse_file(&args.file_a)?;
    let doc_b = parse_file(&args.file_b)?;

    let diff = compute_diff(&doc_a, &doc_b);

    let is_identical = diff.added_nodes.is_empty()
        && diff.removed_nodes.is_empty()
        && diff.added_edges.is_empty()
        && diff.removed_edges.is_empty();

    if is_identical {
        if args.format == DiffFormat::Human {
            println!("No structural differences found.");
        } else {
            let json = serde_json::to_string_pretty(&diff)
                .map_err(|e| (3_i32, format!("failed to serialize diff output: {e}")))?;
            println!("{json}");
        }
        return Ok(0);
    }

    match args.format {
        DiffFormat::Human => print_human_diff(&diff, &args.file_a, &args.file_b),
        DiffFormat::Json => {
            let json = serde_json::to_string_pretty(&diff)
                .map_err(|e| (3_i32, format!("failed to serialize diff output: {e}")))?;
            println!("{json}");
        }
    }

    Ok(1)
}

fn print_human_diff(diff: &StructuralDiff, file_a: &std::path::Path, file_b: &std::path::Path) {
    println!("--- {}\n+++ {}", file_a.display(), file_b.display());

    if !diff.removed_nodes.is_empty() {
        println!("\nRemoved nodes:");
        for node in &diff.removed_nodes {
            println!("  - {node}");
        }
    }

    if !diff.added_nodes.is_empty() {
        println!("\nAdded nodes:");
        for node in &diff.added_nodes {
            println!("  + {node}");
        }
    }

    if !diff.removed_edges.is_empty() {
        println!("\nRemoved edges:");
        for (from, to) in &diff.removed_edges {
            println!("  - {from} -> {to}");
        }
    }

    if !diff.added_edges.is_empty() {
        println!("\nAdded edges:");
        for (from, to) in &diff.added_edges {
            println!("  + {from} -> {to}");
        }
    }
}
