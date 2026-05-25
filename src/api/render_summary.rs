use crate::model::{FamilyDocument, NormalizedDocument};
use serde_json::{json, Value};

pub fn normalized_model_summary_to_json(model: &NormalizedDocument) -> Value {
    match model {
        NormalizedDocument::Sequence(sequence) => json!({
            "kind": "Sequence",
            "participants": sequence.participants.len(),
            "events": sequence.events.len(),
            "warnings": sequence.warnings.len(),
            "title": sequence.title,
            "header": sequence.header,
            "footer": sequence.footer,
            "caption": sequence.caption
        }),
        NormalizedDocument::Family(family) => family_model_summary_to_json(family),
        NormalizedDocument::FamilyPages(pages) => json!({
            "kind": "FamilyPages",
            "pages": pages.iter().map(family_model_summary_to_json).collect::<Vec<_>>()
        }),
        NormalizedDocument::Timeline(timeline) => json!({
            "kind": "Timeline",
            "tasks": timeline.tasks.len(),
            "milestones": timeline.milestones.len(),
            "constraints": timeline.constraints.len(),
            "warnings": timeline.warnings.len(),
            "title": timeline.title
        }),
        NormalizedDocument::State(state) => json!({
            "kind": "State",
            "nodes": state.nodes.len(),
            "transitions": state.transitions.len(),
            "warnings": state.warnings.len(),
            "title": state.title
        }),
        NormalizedDocument::Json(doc) => json!({"kind": "Json", "warnings": doc.warnings.len()}),
        NormalizedDocument::Yaml(doc) => json!({"kind": "Yaml", "warnings": doc.warnings.len()}),
        NormalizedDocument::Nwdiag(doc) => {
            json!({"kind": "Nwdiag", "warnings": doc.warnings.len()})
        }
        NormalizedDocument::Archimate(doc) => {
            json!({"kind": "Archimate", "warnings": doc.warnings.len()})
        }
        NormalizedDocument::Regex(doc) => json!({"kind": "Regex", "warnings": doc.warnings.len()}),
        NormalizedDocument::Ebnf(doc) => json!({"kind": "Ebnf", "warnings": doc.warnings.len()}),
        NormalizedDocument::Math(doc) => json!({"kind": "Math", "warnings": doc.warnings.len()}),
        NormalizedDocument::Sdl(doc) => json!({"kind": "Sdl", "warnings": doc.warnings.len()}),
        NormalizedDocument::Ditaa(doc) => json!({"kind": "Ditaa", "warnings": doc.warnings.len()}),
        NormalizedDocument::Chart(doc) => json!({"kind": "Chart", "warnings": doc.warnings.len()}),
        NormalizedDocument::Stdlib(doc) => json!({
            "kind": "Stdlib",
            "entries": doc.entries.len(),
            "packs": doc.packs.len(),
            "aliases": doc.aliases.len(),
            "missing_packs": doc.missing_packs,
            "warnings": doc.warnings.len()
        }),
        NormalizedDocument::Chen(doc) => json!({
            "kind": "Chen",
            "nodes": doc.nodes.len(),
            "relations": doc.relations.len(),
            "inheritances": doc.inheritances.len(),
            "warnings": doc.warnings.len()
        }),
    }
}

pub(super) fn family_model_summary_to_json(family: &FamilyDocument) -> Value {
    json!({
        "kind": format!("{:?}", family.kind),
        "nodes": family.nodes.len(),
        "relations": family.relations.len(),
        "groups": family.groups.len(),
        "warnings": family.warnings.len(),
        "title": family.title
    })
}
