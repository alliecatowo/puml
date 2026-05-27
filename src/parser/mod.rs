// Re-export external types used by parser submodules so they can access via `use super::*;`.
pub(crate) use crate::ast::{
    ActivityStep, ActivityStepKind, ChenAttribute, ChenDecl, ChenDeclKind, ChenInheritance,
    ChenRelation, ClassDecl, ClassMember, ComponentNodeKind, DiagramKind, Document, FamilyRelation,
    Group, MemberModifier, Message, MessageStyle, Note, ObjectDecl, ParticipantDecl,
    ParticipantRole, SaltCell, StateDecl, StateInternalAction, StateTransition, Statement,
    StatementKind, TimingDeclKind, UseCaseDecl, VirtualEndpoint, VirtualEndpointKind,
    VirtualEndpointSide,
};
pub(crate) use crate::diagnostic::Diagnostic;
pub use crate::preproc::ParseOptions;
pub(crate) use crate::source::Span;

use crate::preproc::{preprocess, preprocess_with_map};

pub fn parse(source: &str) -> Result<Document, Diagnostic> {
    parse_with_options(source, &ParseOptions::default())
}

pub fn parse_with_options(source: &str, options: &ParseOptions) -> Result<Document, Diagnostic> {
    let expanded = preprocess_with_map(source, options)?;
    parse_preprocessed(&expanded.source)
        .map_err(|diagnostic| expanded.source_map.map_diagnostic(diagnostic))
}

pub fn preprocess_with_options(source: &str, options: &ParseOptions) -> Result<String, Diagnostic> {
    preprocess(source, options)
}

mod activity;
mod blocks;
mod chen;
mod component;
mod component_groups;
mod core;
mod core_preprocessed;
mod detect;
mod directives;
mod family;
mod family_arrows;
mod family_context;
mod family_context_tail;
mod family_declarations;
mod family_members;
mod family_relations;
mod family_relations_c4;
mod family_scopes;
mod gantt;
mod multiline;
mod projection_salt;
mod sequence;
mod sequence_keywords;
mod sequence_messages;
mod sequence_participants;
mod shared_ident;
mod sprites;
mod state;
mod timing;

// Bring all submodule items into the parser module's flat namespace.
// Each submodule marks its items pub(crate) so this wildcard import picks them up.
pub(crate) use activity::*;
pub(crate) use blocks::*;
pub(crate) use chen::*;
pub(crate) use component::*;
pub(crate) use component_groups::*;
pub(crate) use core::*;
pub(crate) use core_preprocessed::*;
pub(crate) use detect::*;
pub(crate) use directives::*;
pub(crate) use family::*;
pub(crate) use family_arrows::*;
pub(crate) use family_context::*;
pub(crate) use family_context_tail::*;
pub(crate) use family_declarations::*;
pub(crate) use family_members::*;
pub(crate) use family_relations::*;
pub(crate) use family_relations_c4::*;
pub(crate) use family_scopes::*;
pub(crate) use gantt::*;
pub(crate) use multiline::*;
pub(crate) use projection_salt::*;
pub(crate) use sequence_keywords::*;
pub(crate) use sequence_messages::*;
pub(crate) use sequence_participants::*;
pub(crate) use shared_ident::*;
pub(crate) use sprites::*;
pub(crate) use state::*;
pub(crate) use timing::*;

#[cfg(test)]
mod tests;
