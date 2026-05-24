use super::types::{
    CompatMode, DeterminismMode, DiagramFamily, FrontendSelection, ParsePipelineOptions,
    ParsePipelineResult,
};
use crate::ast::{self, Document};
use crate::diagnostic::Diagnostic;
use crate::frontend;
use crate::model::{NormalizedDocument, SequenceDocument};
use crate::{normalize as normalize_mod, parser, registry};

pub fn parse(source: &str) -> Result<Document, Diagnostic> {
    parser::parse(source)
}

pub fn parse_with_pipeline_options(
    source: &str,
    options: &ParsePipelineOptions,
) -> Result<Document, Diagnostic> {
    parse_with_pipeline_result_options(source, options).map(|result| result.document)
}

pub fn parse_with_pipeline_result_options(
    source: &str,
    options: &ParsePipelineOptions,
) -> Result<ParsePipelineResult, Diagnostic> {
    let parser_options = interpret_parser_contract(options)?;
    interpret_determinism_contract(options.determinism);

    match options.frontend {
        FrontendSelection::Auto | FrontendSelection::Plantuml => {
            parser::parse_with_options(source, &parser_options).map(|document| {
                ParsePipelineResult {
                    document,
                    diagnostics: Vec::new(),
                }
            })
        }
        FrontendSelection::Mermaid => {
            let adapted = frontend::mermaid::adapt(source)?;
            let frontend::FrontendResult {
                source: adapted_source,
                source_map,
                diagnostics,
            } = adapted;
            parser::parse_with_options(&adapted_source, &parser_options)
                .map_err(|diagnostic| source_map.map_diagnostic(diagnostic))
                .map(|document| ParsePipelineResult {
                    document,
                    diagnostics,
                })
        }
        FrontendSelection::Picouml => {
            let adapted = frontend::picouml::adapt(source)?;
            let frontend::FrontendResult {
                source: adapted_source,
                source_map,
                diagnostics,
            } = adapted;
            parser::parse_with_options(&adapted_source, &parser_options)
                .map_err(|diagnostic| source_map.map_diagnostic(diagnostic))
                .map(|document| ParsePipelineResult {
                    document,
                    diagnostics,
                })
        }
    }
}

pub fn preprocess_with_pipeline_options(
    source: &str,
    options: &ParsePipelineOptions,
) -> Result<String, Diagnostic> {
    let parser_options = interpret_parser_contract(options)?;
    interpret_determinism_contract(options.determinism);

    match options.frontend {
        FrontendSelection::Auto | FrontendSelection::Plantuml => {
            parser::preprocess_with_options(source, &parser_options)
        }
        FrontendSelection::Mermaid => {
            let adapted = frontend::mermaid::adapt(source)?;
            let frontend::FrontendResult {
                source: adapted_source,
                source_map,
                diagnostics,
            } = adapted;
            let _ = diagnostics;
            parser::preprocess_with_options(&adapted_source, &parser_options)
                .map_err(|diagnostic| source_map.map_diagnostic(diagnostic))
        }
        FrontendSelection::Picouml => {
            let adapted = frontend::picouml::adapt(source)?;
            let frontend::FrontendResult {
                source: adapted_source,
                source_map,
                diagnostics,
            } = adapted;
            let _ = diagnostics;
            parser::preprocess_with_options(&adapted_source, &parser_options)
                .map_err(|diagnostic| source_map.map_diagnostic(diagnostic))
        }
    }
}

fn interpret_parser_contract(
    options: &ParsePipelineOptions,
) -> Result<parser::ParseOptions, Diagnostic> {
    let include_root = match options.compat {
        CompatMode::Strict => options.include_root.clone(),
        CompatMode::Extended => options.include_root.clone(),
    };
    Ok(parser::ParseOptions {
        include_root,
        allow_url_includes: options.allow_url_includes,
        inject_vars: options.inject_vars.clone(),
    })
}

fn interpret_determinism_contract(_mode: DeterminismMode) {
    // Determinism behavior is currently fully deterministic across modes.
    // Keep this explicit interpretation point to avoid split-brain routing.
}

pub fn normalize(document: Document) -> Result<SequenceDocument, Diagnostic> {
    normalize_mod::normalize(document)
}

pub fn normalize_family(document: Document) -> Result<NormalizedDocument, Diagnostic> {
    normalize_mod::normalize_family(document)
}

pub fn detect_diagram_family(source: &str) -> Result<DiagramFamily, Diagnostic> {
    let document = parse(source)?;
    Ok(map_ast_kind_to_family(document.kind))
}

pub(super) fn map_ast_kind_to_family(kind: ast::DiagramKind) -> DiagramFamily {
    registry::diagram_family_for_ast(kind)
}

#[cfg(test)]
mod tests {
    use super::{parse_with_pipeline_options, CompatMode, FrontendSelection, ParsePipelineOptions};

    #[test]
    fn extended_mode_without_include_root_does_not_fallback_to_cwd_in_library_api() {
        let options = ParsePipelineOptions {
            frontend: FrontendSelection::Auto,
            compat: CompatMode::Extended,
            include_root: None,
            ..ParsePipelineOptions::default()
        };
        let err = parse_with_pipeline_options("!include __puml_missing__.puml\n", &options)
            .expect_err("expected include-root diagnostic");
        assert!(
            err.message.contains("E_INCLUDE_ROOT_REQUIRED"),
            "unexpected error: {}",
            err.message
        );
    }
}
