use crate::ast::{
    ActivityStep, ActivityStepKind, ClassDecl, ClassMember, ComponentNodeKind, DiagramKind,
    Document, FamilyRelation, Group, MemberModifier, Message, MessageStyle, Note, ObjectDecl,
    ParticipantDecl, ParticipantRole, SaltCell, StateDecl, StateInternalAction, StateTransition,
    Statement, StatementKind, StyleRule, TimingDeclKind, UseCaseDecl, VirtualEndpoint,
    VirtualEndpointKind, VirtualEndpointSide,
};
use crate::diagnostic::Diagnostic;
use crate::preproc::preprocess;
pub use crate::preproc::ParseOptions;
use crate::source::Span;

pub fn parse(source: &str) -> Result<Document, Diagnostic> {
    parse_with_options(source, &ParseOptions::default())
}

pub fn parse_with_options(source: &str, options: &ParseOptions) -> Result<Document, Diagnostic> {
    let expanded = preprocess(source, options)?;
    parse_preprocessed(&expanded)
}

pub fn preprocess_with_options(source: &str, options: &ParseOptions) -> Result<String, Diagnostic> {
    preprocess(source, options)
}

include!("parser/core.rs");
include!("parser/blocks.rs");
include!("parser/family.rs");
include!("parser/component_groups.rs");
include!("parser/detect.rs");
include!("parser/gantt.rs");
include!("parser/component.rs");
include!("parser/activity.rs");
include!("parser/timing.rs");
include!("parser/state.rs");
include!("parser/multiline.rs");
include!("parser/sequence.rs");
include!("parser/projection_salt.rs");
include!("parser/tests.rs");
