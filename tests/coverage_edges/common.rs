pub use assert_cmd::Command;
pub use predicates::prelude::*;
pub use puml::ast::DiagramKind;
pub use puml::layout;
pub use puml::model::{
    Participant, ParticipantRole, SequenceDocument, SequenceEvent, SequenceEventKind,
    VirtualEndpoint, VirtualEndpointKind, VirtualEndpointSide,
};
pub use puml::normalize;
pub use puml::parser::{parse_with_options, ParseOptions};
pub use puml::scene::{LayoutOptions, TextOverflowPolicy};
pub use puml::source::Span;
pub use puml::theme::{
    classify_sequence_skinparam, SequenceSkinParamSupport, SequenceSkinParamValue,
};
pub use puml::{normalize_family, parse, render, NormalizedDocument};
pub use std::fs;
pub use tempfile::tempdir;

pub fn fixture(name: &str) -> String {
    format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"))
}
