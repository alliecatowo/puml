use crate::cli::{
    CompatMode as CliCompatMode, DeterminismMode as CliDeterminismMode, Dialect as CliDialect,
};
use puml::{
    preprocess_with_pipeline_options, CompatMode, DeterminismMode, Diagnostic, Document,
    FrontendSelection, ParsePipelineOptions, ParsePipelineResult,
};
use std::collections::BTreeMap;
use std::path::PathBuf;

pub(super) fn parse_for_cli(
    source: &str,
    include_root: Option<PathBuf>,
    cli_dialect: CliDialect,
    cli_compat: CliCompatMode,
    cli_determinism: CliDeterminismMode,
    frontend_hint: Option<FrontendSelection>,
    allow_url_includes: bool,
    inject_vars: BTreeMap<String, String>,
) -> Result<Document, Diagnostic> {
    parse_for_cli_with_diagnostics(
        source,
        include_root,
        cli_dialect,
        cli_compat,
        cli_determinism,
        frontend_hint,
        allow_url_includes,
        inject_vars,
    )
    .map(|result| result.document)
}

// Same rationale as preprocess_for_cli below — all args are required.
#[allow(clippy::too_many_arguments)]
pub(super) fn parse_for_cli_with_diagnostics(
    source: &str,
    include_root: Option<PathBuf>,
    cli_dialect: CliDialect,
    cli_compat: CliCompatMode,
    cli_determinism: CliDeterminismMode,
    frontend_hint: Option<FrontendSelection>,
    allow_url_includes: bool,
    inject_vars: BTreeMap<String, String>,
) -> Result<ParsePipelineResult, Diagnostic> {
    let include_root = include_root.or_else(|| match cli_compat {
        CliCompatMode::Strict => None,
        CliCompatMode::Extended => std::env::current_dir().ok(),
    });
    let options = ParsePipelineOptions {
        frontend: map_frontend(cli_dialect, frontend_hint),
        compat: map_compat(cli_compat),
        determinism: map_determinism(cli_determinism),
        include_root,
        allow_url_includes,
        inject_vars,
    };
    puml::parse_with_pipeline_result_options(source, &options)
}

// Same rationale as parse_for_cli above — all args are required.
#[allow(clippy::too_many_arguments)]
pub(super) fn preprocess_for_cli(
    source: &str,
    include_root: Option<PathBuf>,
    cli_dialect: CliDialect,
    cli_compat: CliCompatMode,
    cli_determinism: CliDeterminismMode,
    frontend_hint: Option<FrontendSelection>,
    allow_url_includes: bool,
    inject_vars: BTreeMap<String, String>,
) -> Result<String, Diagnostic> {
    let include_root = include_root.or_else(|| match cli_compat {
        CliCompatMode::Strict => None,
        CliCompatMode::Extended => std::env::current_dir().ok(),
    });
    let options = ParsePipelineOptions {
        frontend: map_frontend(cli_dialect, frontend_hint),
        compat: map_compat(cli_compat),
        determinism: map_determinism(cli_determinism),
        include_root,
        allow_url_includes,
        inject_vars,
    };
    preprocess_with_pipeline_options(source, &options)
}

fn map_frontend(
    dialect: CliDialect,
    frontend_hint: Option<FrontendSelection>,
) -> FrontendSelection {
    // Auto mode is the only mode that accepts routing hints from file
    // extensions (`.picouml`) or markdown fence tags (`picouml`, `mermaid`).
    // Explicit `--dialect` keeps user intent ahead of extension names.
    if matches!(dialect, CliDialect::Auto) {
        if let Some(hint) = frontend_hint {
            return hint;
        }
    }

    match dialect {
        CliDialect::Auto => FrontendSelection::Auto,
        CliDialect::Plantuml => FrontendSelection::Plantuml,
        CliDialect::Mermaid => FrontendSelection::Mermaid,
        CliDialect::Picouml => FrontendSelection::Picouml,
    }
}

fn map_compat(mode: CliCompatMode) -> CompatMode {
    match mode {
        CliCompatMode::Strict => CompatMode::Strict,
        CliCompatMode::Extended => CompatMode::Extended,
    }
}

fn map_determinism(mode: CliDeterminismMode) -> DeterminismMode {
    match mode {
        CliDeterminismMode::Strict => DeterminismMode::Strict,
        CliDeterminismMode::Full => DeterminismMode::Full,
    }
}
