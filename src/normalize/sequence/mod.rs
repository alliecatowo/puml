use super::common::{self, CommonDirectives, LegendTextMode};
use super::*;
use crate::model::SequenceParticipantGroup;

mod autonumber;
mod groups;
mod lifecycle;
mod messages;
mod pagination;
mod participants;
mod state;
mod style;
mod validation;

use state::SequenceNormalizeState;

pub(super) fn paginate(document: &SequenceDocument) -> Vec<SequencePage> {
    pagination::paginate(document)
}

pub(super) fn normalize_with_options(
    document: Document,
    _options: &NormalizeOptions,
) -> Result<SequenceDocument, Diagnostic> {
    if document.kind != DiagramKind::Sequence {
        return Err(validation::unsupported_family_diagnostic(document.kind));
    }

    let mut state = SequenceNormalizeState::default();
    for stmt in document.statements {
        state.handle_statement(stmt)?;
    }
    state.finish()
}

/// Extract `<<stereotype>>` from an alias string like `"myAlias <<person>>"`.
///
/// Returns `(clean_alias, Option<FamilyNodeKind>, Option<raw_stereotype>)` where:
/// - `clean_alias` has the `<<…>>` suffix stripped.
/// - The `FamilyNodeKind` is set when the stereotype is a recognised C4 marker.
/// - `raw_stereotype` carries the full `<<…>>` token when the stereotype is NOT a
///   C4 marker (e.g. `"<<aws-EC2>>"`) so callers can inject it as a member for
///   downstream renderers (e.g. the cloud-icon renderer, Refs #1258).
pub(crate) fn extract_c4_stereotype(
    alias: Option<String>,
) -> (Option<String>, Option<FamilyNodeKind>, Option<String>) {
    let Some(raw) = alias else {
        return (None, None, None);
    };
    if let Some(start) = raw.find("<<") {
        if let Some(end) = raw[start..].find(">>") {
            let stereotype = raw[start + 2..start + end].trim().to_ascii_lowercase();
            let raw_stereotype_token = raw[start..start + end + 2].to_string();
            let clean_alias = raw[..start].trim().to_string();
            let kind = match stereotype.as_str() {
                "person" => Some(FamilyNodeKind::C4Person),
                "external-person" => Some(FamilyNodeKind::C4PersonExt),
                "system" => Some(FamilyNodeKind::C4System),
                "external-system" => Some(FamilyNodeKind::C4SystemExt),
                "system-db" | "systemdb" => Some(FamilyNodeKind::C4SystemDb),
                "system-queue" | "systemqueue" => Some(FamilyNodeKind::C4SystemQueue),
                "container" => Some(FamilyNodeKind::C4Container),
                "external-container" => Some(FamilyNodeKind::C4ContainerExt),
                "container-db" | "containerdb" => Some(FamilyNodeKind::C4ContainerDb),
                "container-queue" | "containerqueue" => Some(FamilyNodeKind::C4ContainerQueue),
                "c4-component" | "component" => Some(FamilyNodeKind::C4Component),
                "external-c4-component" | "external-component" => {
                    Some(FamilyNodeKind::C4ComponentExt)
                }
                "component-db" | "componentdb" => Some(FamilyNodeKind::C4ComponentDb),
                "component-queue" | "componentqueue" => Some(FamilyNodeKind::C4ComponentQueue),
                "boundary" | "enterprise-boundary" | "system-boundary" | "container-boundary" => {
                    Some(FamilyNodeKind::C4Boundary)
                }
                _ => None,
            };
            let clean = if clean_alias.is_empty() {
                None
            } else {
                Some(clean_alias)
            };
            // For non-C4 stereotypes, return the raw token so callers can preserve it.
            let extra = if kind.is_none() {
                Some(raw_stereotype_token)
            } else {
                None
            };
            return (clean, kind, extra);
        }
    }
    (Some(raw), None, None)
}
