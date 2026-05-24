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
/// Returns `(clean_alias, Option<FamilyNodeKind>)` where `clean_alias` has the
/// stereotype stripped. When the stereotype is not a recognised C4 marker the
/// kind is `None` and the caller keeps `FamilyNodeKind::Object`.
pub(crate) fn extract_c4_stereotype(
    alias: Option<String>,
) -> (Option<String>, Option<FamilyNodeKind>) {
    let Some(raw) = alias else {
        return (None, None);
    };
    if let Some(start) = raw.find("<<") {
        if let Some(end) = raw[start..].find(">>") {
            let stereotype = raw[start + 2..start + end].trim().to_ascii_lowercase();
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
            return (clean, kind);
        }
    }
    (Some(raw), None)
}
