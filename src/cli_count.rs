use std::collections::BTreeMap;
use std::fs;

use puml::model::{
    FamilyDocument, FamilyNodeKind, NormalizedDocument, ParticipantRole, SequenceEventKind,
    StateNodeKind,
};
use puml::{normalize_family, ParsePipelineOptions};

use crate::cli::CountArgs;

/// Totals extracted from a parsed diagram.
#[derive(Debug, Default)]
pub struct Counts {
    pub nodes: usize,
    pub edges: usize,
    pub by_kind: Option<BTreeMap<String, usize>>,
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

    let model = normalize_family(doc).map_err(|d| (1i32, format!("error: {}", d.message)))?;
    let counts = extract_counts(&model, args.by_kind);

    println!("{} nodes, {} edges", counts.nodes, counts.edges);

    if let Some(ref by_kind) = counts.by_kind {
        for (kind, n) in by_kind {
            println!("  {kind}: {n}");
        }
    }

    Ok(0)
}

fn extract_counts(model: &NormalizedDocument, by_kind: bool) -> Counts {
    let mut counts = Counts::default();

    match model {
        NormalizedDocument::Sequence(seq) => {
            counts.nodes = seq.participants.len();
            counts.edges = seq
                .events
                .iter()
                .filter(|e| matches!(e.kind, SequenceEventKind::Message { .. }))
                .count();

            if by_kind {
                let mut kinds = BTreeMap::new();
                for p in &seq.participants {
                    increment(&mut kinds, participant_role_label(p.role));
                }
                counts.by_kind = Some(kinds);
            }
        }
        NormalizedDocument::Family(fam) => {
            counts_family_document(&mut counts, fam, by_kind);
        }
        NormalizedDocument::FamilyPages(pages) => {
            let mut kinds = by_kind.then(BTreeMap::new);
            for page in pages {
                counts.nodes += page.nodes.len();
                counts.edges += page.relations.len();
                if let Some(ref mut kinds) = kinds {
                    for node in &page.nodes {
                        increment(kinds, family_node_kind_label(node.kind));
                    }
                }
            }
            counts.by_kind = kinds;
        }
        NormalizedDocument::State(state) => {
            counts.nodes = state.nodes.len();
            counts.edges = state.transitions.len();

            if by_kind {
                let mut kinds = BTreeMap::new();
                for node in &state.nodes {
                    increment(&mut kinds, state_node_kind_label(&node.kind));
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

fn counts_family_document(counts: &mut Counts, fam: &FamilyDocument, by_kind: bool) {
    counts.nodes = fam.nodes.len();
    counts.edges = fam.relations.len();

    if by_kind {
        let mut kinds = BTreeMap::new();
        for node in &fam.nodes {
            increment(&mut kinds, family_node_kind_label(node.kind));
        }
        counts.by_kind = Some(kinds);
    }
}

fn increment(kinds: &mut BTreeMap<String, usize>, label: &'static str) {
    *kinds.entry(label.to_string()).or_insert(0) += 1;
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

fn family_node_kind_label(kind: FamilyNodeKind) -> &'static str {
    match kind {
        FamilyNodeKind::Class => "class",
        FamilyNodeKind::Object => "object",
        FamilyNodeKind::Map => "map",
        FamilyNodeKind::Diamond => "diamond",
        FamilyNodeKind::UseCase => "use-case",
        FamilyNodeKind::Salt => "salt",
        FamilyNodeKind::MindMap => "mindmap",
        FamilyNodeKind::Wbs => "wbs",
        FamilyNodeKind::Component => "component",
        FamilyNodeKind::Interface => "interface",
        FamilyNodeKind::Port => "port",
        FamilyNodeKind::Action => "action",
        FamilyNodeKind::Agent => "agent",
        FamilyNodeKind::Node => "node",
        FamilyNodeKind::Artifact => "artifact",
        FamilyNodeKind::Boundary => "boundary",
        FamilyNodeKind::Cloud => "cloud",
        FamilyNodeKind::Circle => "circle",
        FamilyNodeKind::Collections => "collections",
        FamilyNodeKind::Frame => "frame",
        FamilyNodeKind::Storage => "storage",
        FamilyNodeKind::Container => "container",
        FamilyNodeKind::Control => "control",
        FamilyNodeKind::Database => "database",
        FamilyNodeKind::Entity => "entity",
        FamilyNodeKind::Package => "package",
        FamilyNodeKind::Rectangle => "rectangle",
        FamilyNodeKind::Folder => "folder",
        FamilyNodeKind::File => "file",
        FamilyNodeKind::Card => "card",
        FamilyNodeKind::Actor => "actor",
        FamilyNodeKind::BusinessActor => "business-actor",
        FamilyNodeKind::BusinessUseCase => "business-use-case",
        FamilyNodeKind::Hexagon => "hexagon",
        FamilyNodeKind::Label => "label",
        FamilyNodeKind::Person => "person",
        FamilyNodeKind::Process => "process",
        FamilyNodeKind::Queue => "queue",
        FamilyNodeKind::Stack => "stack",
        FamilyNodeKind::UseCaseDeployment => "use-case-deployment",
        FamilyNodeKind::State => "state",
        FamilyNodeKind::StateInitial => "state-initial",
        FamilyNodeKind::StateFinal => "state-final",
        FamilyNodeKind::StateHistory => "state-history",
        FamilyNodeKind::ActivityStart => "activity-start",
        FamilyNodeKind::ActivityStop => "activity-stop",
        FamilyNodeKind::ActivityAction => "activity-action",
        FamilyNodeKind::ActivityDecision => "activity-decision",
        FamilyNodeKind::ActivityFork => "activity-fork",
        FamilyNodeKind::ActivityForkEnd => "activity-fork-end",
        FamilyNodeKind::ActivityMerge => "activity-merge",
        FamilyNodeKind::ActivityPartition => "activity-partition",
        FamilyNodeKind::Note => "note",
        FamilyNodeKind::TimingConcise => "timing-concise",
        FamilyNodeKind::TimingRobust => "timing-robust",
        FamilyNodeKind::TimingClock => "timing-clock",
        FamilyNodeKind::TimingBinary => "timing-binary",
        FamilyNodeKind::TimingEvent => "timing-event",
        FamilyNodeKind::C4Person => "c4-person",
        FamilyNodeKind::C4PersonExt => "c4-person-ext",
        FamilyNodeKind::C4System => "c4-system",
        FamilyNodeKind::C4SystemExt => "c4-system-ext",
        FamilyNodeKind::C4SystemDb => "c4-system-db",
        FamilyNodeKind::C4SystemQueue => "c4-system-queue",
        FamilyNodeKind::C4Container => "c4-container",
        FamilyNodeKind::C4ContainerExt => "c4-container-ext",
        FamilyNodeKind::C4ContainerDb => "c4-container-db",
        FamilyNodeKind::C4ContainerQueue => "c4-container-queue",
        FamilyNodeKind::C4Component => "c4-component",
        FamilyNodeKind::C4ComponentExt => "c4-component-ext",
        FamilyNodeKind::C4ComponentDb => "c4-component-db",
        FamilyNodeKind::C4ComponentQueue => "c4-component-queue",
        FamilyNodeKind::C4Boundary => "c4-boundary",
    }
}

fn state_node_kind_label(kind: &StateNodeKind) -> &'static str {
    match kind {
        StateNodeKind::Normal => "state",
        StateNodeKind::StartEnd => "start-end",
        StateNodeKind::HistoryShallow => "history-shallow",
        StateNodeKind::HistoryDeep => "history-deep",
        StateNodeKind::Fork => "fork",
        StateNodeKind::Join => "join",
        StateNodeKind::Choice => "choice",
        StateNodeKind::End => "end",
        StateNodeKind::EntryPoint => "entry-point",
        StateNodeKind::ExitPoint => "exit-point",
        StateNodeKind::InputPin => "input-pin",
        StateNodeKind::OutputPin => "output-pin",
        StateNodeKind::ExpansionInput => "expansion-input",
        StateNodeKind::ExpansionOutput => "expansion-output",
        StateNodeKind::Note => "note",
        StateNodeKind::JsonProjection => "json-projection",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn participant_role_labels_cover_all_count_cli_roles() {
        let cases = [
            (ParticipantRole::Participant, "participant"),
            (ParticipantRole::Actor, "actor"),
            (ParticipantRole::Boundary, "boundary"),
            (ParticipantRole::Control, "control"),
            (ParticipantRole::Entity, "entity"),
            (ParticipantRole::Database, "database"),
            (ParticipantRole::Collections, "collections"),
            (ParticipantRole::Queue, "queue"),
        ];

        for (role, label) in cases {
            assert_eq!(participant_role_label(role), label);
        }
    }

    #[test]
    fn family_node_kind_labels_cover_count_cli_surface() {
        let cases = [
            (FamilyNodeKind::Class, "class"),
            (FamilyNodeKind::Object, "object"),
            (FamilyNodeKind::Map, "map"),
            (FamilyNodeKind::Diamond, "diamond"),
            (FamilyNodeKind::UseCase, "use-case"),
            (FamilyNodeKind::Salt, "salt"),
            (FamilyNodeKind::MindMap, "mindmap"),
            (FamilyNodeKind::Wbs, "wbs"),
            (FamilyNodeKind::Component, "component"),
            (FamilyNodeKind::Interface, "interface"),
            (FamilyNodeKind::Port, "port"),
            (FamilyNodeKind::Action, "action"),
            (FamilyNodeKind::Agent, "agent"),
            (FamilyNodeKind::Node, "node"),
            (FamilyNodeKind::Artifact, "artifact"),
            (FamilyNodeKind::Boundary, "boundary"),
            (FamilyNodeKind::Cloud, "cloud"),
            (FamilyNodeKind::Circle, "circle"),
            (FamilyNodeKind::Collections, "collections"),
            (FamilyNodeKind::Frame, "frame"),
            (FamilyNodeKind::Storage, "storage"),
            (FamilyNodeKind::Container, "container"),
            (FamilyNodeKind::Control, "control"),
            (FamilyNodeKind::Database, "database"),
            (FamilyNodeKind::Entity, "entity"),
            (FamilyNodeKind::Package, "package"),
            (FamilyNodeKind::Rectangle, "rectangle"),
            (FamilyNodeKind::Folder, "folder"),
            (FamilyNodeKind::File, "file"),
            (FamilyNodeKind::Card, "card"),
            (FamilyNodeKind::Actor, "actor"),
            (FamilyNodeKind::BusinessActor, "business-actor"),
            (FamilyNodeKind::BusinessUseCase, "business-use-case"),
            (FamilyNodeKind::Hexagon, "hexagon"),
            (FamilyNodeKind::Label, "label"),
            (FamilyNodeKind::Person, "person"),
            (FamilyNodeKind::Process, "process"),
            (FamilyNodeKind::Queue, "queue"),
            (FamilyNodeKind::Stack, "stack"),
            (FamilyNodeKind::UseCaseDeployment, "use-case-deployment"),
            (FamilyNodeKind::State, "state"),
            (FamilyNodeKind::StateInitial, "state-initial"),
            (FamilyNodeKind::StateFinal, "state-final"),
            (FamilyNodeKind::StateHistory, "state-history"),
            (FamilyNodeKind::ActivityStart, "activity-start"),
            (FamilyNodeKind::ActivityStop, "activity-stop"),
            (FamilyNodeKind::ActivityAction, "activity-action"),
            (FamilyNodeKind::ActivityDecision, "activity-decision"),
            (FamilyNodeKind::ActivityFork, "activity-fork"),
            (FamilyNodeKind::ActivityForkEnd, "activity-fork-end"),
            (FamilyNodeKind::ActivityMerge, "activity-merge"),
            (FamilyNodeKind::ActivityPartition, "activity-partition"),
            (FamilyNodeKind::Note, "note"),
            (FamilyNodeKind::TimingConcise, "timing-concise"),
            (FamilyNodeKind::TimingRobust, "timing-robust"),
            (FamilyNodeKind::TimingClock, "timing-clock"),
            (FamilyNodeKind::TimingBinary, "timing-binary"),
            (FamilyNodeKind::TimingEvent, "timing-event"),
            (FamilyNodeKind::C4Person, "c4-person"),
            (FamilyNodeKind::C4PersonExt, "c4-person-ext"),
            (FamilyNodeKind::C4System, "c4-system"),
            (FamilyNodeKind::C4SystemExt, "c4-system-ext"),
            (FamilyNodeKind::C4SystemDb, "c4-system-db"),
            (FamilyNodeKind::C4SystemQueue, "c4-system-queue"),
            (FamilyNodeKind::C4Container, "c4-container"),
            (FamilyNodeKind::C4ContainerExt, "c4-container-ext"),
            (FamilyNodeKind::C4ContainerDb, "c4-container-db"),
            (FamilyNodeKind::C4ContainerQueue, "c4-container-queue"),
            (FamilyNodeKind::C4Component, "c4-component"),
            (FamilyNodeKind::C4ComponentExt, "c4-component-ext"),
            (FamilyNodeKind::C4ComponentDb, "c4-component-db"),
            (FamilyNodeKind::C4ComponentQueue, "c4-component-queue"),
            (FamilyNodeKind::C4Boundary, "c4-boundary"),
        ];

        for (kind, label) in cases {
            assert_eq!(family_node_kind_label(kind), label);
        }
    }

    #[test]
    fn state_node_kind_labels_cover_count_cli_surface() {
        let cases = [
            (StateNodeKind::Normal, "state"),
            (StateNodeKind::StartEnd, "start-end"),
            (StateNodeKind::HistoryShallow, "history-shallow"),
            (StateNodeKind::HistoryDeep, "history-deep"),
            (StateNodeKind::Fork, "fork"),
            (StateNodeKind::Join, "join"),
            (StateNodeKind::Choice, "choice"),
            (StateNodeKind::End, "end"),
            (StateNodeKind::EntryPoint, "entry-point"),
            (StateNodeKind::ExitPoint, "exit-point"),
            (StateNodeKind::InputPin, "input-pin"),
            (StateNodeKind::OutputPin, "output-pin"),
            (StateNodeKind::ExpansionInput, "expansion-input"),
            (StateNodeKind::ExpansionOutput, "expansion-output"),
            (StateNodeKind::Note, "note"),
            (StateNodeKind::JsonProjection, "json-projection"),
        ];

        for (kind, label) in cases {
            assert_eq!(state_node_kind_label(&kind), label);
        }
    }

    fn normalize_source(source: &str) -> NormalizedDocument {
        let options = ParsePipelineOptions {
            frontend: puml::FrontendSelection::Auto,
            compat: puml::CompatMode::Strict,
            determinism: puml::DeterminismMode::Strict,
            include_root: None,
            allow_url_includes: false,
            inject_vars: Default::default(),
        };
        let doc = puml::parse_with_pipeline_options(source, &options).expect("parse fixture");
        normalize_family(doc).expect("normalize fixture")
    }

    #[test]
    fn extract_counts_handles_sequence_messages_and_roles() {
        let model = normalize_source(
            "@startuml
actor User
participant Service
database DB
User -> Service: request
Service -> DB: query
@enduml
",
        );

        let counts = extract_counts(&model, true);
        let by_kind = counts.by_kind.expect("kind breakdown");

        assert_eq!(counts.nodes, 3);
        assert_eq!(counts.edges, 2);
        assert_eq!(by_kind["actor"], 1);
        assert_eq!(by_kind["participant"], 1);
        assert_eq!(by_kind["database"], 1);
    }

    #[test]
    fn extract_counts_handles_family_documents_and_pages() {
        let family = normalize_source(
            "@startuml
class A
class B
A --> B
@enduml
",
        );
        let family_counts = extract_counts(&family, true);
        assert_eq!(family_counts.nodes, 2);
        assert_eq!(family_counts.edges, 1);
        assert_eq!(family_counts.by_kind.expect("family kinds")["class"], 2);

        let pages = normalize_source(
            "@startuml
class A
newpage
class B
A --> B
@enduml
",
        );
        let page_counts = extract_counts(&pages, true);
        let page_kinds = page_counts.by_kind.expect("page kinds");
        assert_eq!(page_counts.nodes, 2);
        assert_eq!(page_counts.edges, 1);
        assert_eq!(page_kinds["class"], 2);
    }

    #[test]
    fn extract_counts_handles_state_nodes_and_transitions() {
        let model = normalize_source(
            "@startuml
[*] --> Running
Running --> [H]
Running --> [*]
@enduml
",
        );

        let counts = extract_counts(&model, true);
        let by_kind = counts.by_kind.as_ref().expect("state kinds");

        assert!(counts.nodes >= 3, "{counts:?}");
        assert_eq!(counts.edges, 3);
        assert!(by_kind["state"] >= 1, "{by_kind:?}");
        assert!(by_kind["start-end"] >= 1, "{by_kind:?}");
        assert_eq!(by_kind["history-shallow"], 1);
    }

    #[test]
    fn extract_counts_returns_zero_for_unsupported_diagram_families() {
        let model = normalize_source(
            "@startgantt
[Task] lasts 2 days
@endgantt
",
        );

        let counts = extract_counts(&model, true);

        assert_eq!(counts.nodes, 0);
        assert_eq!(counts.edges, 0);
        assert!(counts.by_kind.is_none());
    }

    #[test]
    fn run_count_reports_io_and_parse_failures_with_expected_codes() {
        let missing = CountArgs {
            file: std::path::PathBuf::from("definitely-missing-count-fixture.puml"),
            by_kind: false,
        };
        let (code, msg) = run_count(&missing).expect_err("missing file");
        assert_eq!(code, 2);
        assert!(msg.contains("could not read"));

        let dir = tempfile::tempdir().expect("tempdir");
        let invalid_path = dir.path().join("invalid.puml");
        fs::write(&invalid_path, "@startuml\nAlice ->\n@enduml\n").expect("write invalid");
        let invalid = CountArgs {
            file: invalid_path,
            by_kind: false,
        };
        let (code, msg) = run_count(&invalid).expect_err("parse failure");
        assert_eq!(code, 1);
        assert!(msg.contains("could not parse"));
    }
}
