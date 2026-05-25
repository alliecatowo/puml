use serde_json::{json, Value};

use super::{completion_items, semantic_token_legend, syntax_token_specs, CompletionItemKind};
use crate::registry::{diagram_family_specs, graph_element_specs};

pub fn language_service_surface_json() -> Value {
    json!({
        "schema": "puml.languageService",
        "schemaVersion": 1,
        "families": diagram_family_specs().iter().map(|spec| {
            json!({
                "name": spec.name,
                "publicFamily": spec.public_family.as_str(),
                "renderKind": format!("{:?}", spec.render_kind),
                "capabilities": {
                    "svg": spec.capabilities.svg,
                    "text": spec.capabilities.text,
                    "metadata": spec.capabilities.metadata,
                    "languageService": spec.capabilities.language_service,
                    "frontends": {
                        "plantuml": spec.capabilities.plantuml_frontend,
                        "mermaid": spec.capabilities.mermaid_frontend,
                        "picouml": spec.capabilities.picouml_frontend
                    }
                }
            })
        }).collect::<Vec<_>>(),
        "graphElements": graph_element_specs().iter().map(|spec| {
            json!({
                "keyword": spec.keyword,
                "aliases": spec.aliases,
                "families": spec.source_families.iter().map(|kind| format!("{kind:?}")).collect::<Vec<_>>(),
                "shape": format!("{:?}", spec.shape_kind),
                "relationEndpoint": format!("{:?}", spec.relation_endpoint)
            })
        }).collect::<Vec<_>>(),
        "completion": {
            "isIncomplete": false,
            "items": completion_items().items.iter().map(|item| {
                json!({
                    "label": item.label,
                    "kind": completion_kind(item.kind),
                    "detail": item.detail,
                    "documentation": item.documentation
                })
            }).collect::<Vec<_>>()
        },
        "syntax": {
            "tokens": syntax_token_specs().iter().map(|spec| {
                json!({
                    "lexeme": spec.lexeme,
                    "kind": format!("{:?}", spec.kind),
                    "families": spec.families
                })
            }).collect::<Vec<_>>()
        },
        "semanticTokens": {
            "legend": semantic_token_legend()
        }
    })
}

fn completion_kind(kind: CompletionItemKind) -> &'static str {
    match kind {
        CompletionItemKind::Keyword => "keyword",
        CompletionItemKind::Operator => "operator",
        CompletionItemKind::Snippet => "snippet",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surface_includes_registry_frontends_graph_elements_and_styles() {
        let surface = language_service_surface_json();
        assert_eq!(surface["schema"], "puml.languageService");
        assert!(surface["families"]
            .as_array()
            .expect("families")
            .iter()
            .any(|family| family["name"] == "class"
                && family["capabilities"]["frontends"]["mermaid"] == true));
        assert!(surface["graphElements"]
            .as_array()
            .expect("graph elements")
            .iter()
            .any(|element| element["keyword"] == "component"));
        assert!(surface["completion"]["items"]
            .as_array()
            .expect("completion items")
            .iter()
            .any(|item| item["label"] == "ArrowColor"
                && item["documentation"]
                    .as_str()
                    .expect("documentation")
                    .contains("Value type: color")));
    }
}
