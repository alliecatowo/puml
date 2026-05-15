use clap::{ArgAction, Parser, ValueEnum};
use std::path::PathBuf;

#[derive(Debug, Clone, Parser)]
#[command(name = "puml", version, about = "PicoUML polymorphic sequence CLI")]
pub struct Cli {
    /// Print puml-lsp capability manifest and exit.
    #[arg(long, action = ArgAction::SetTrue)]
    pub dump_capabilities: bool,

    /// Validate a fixture file with parser+normalizer and print diagnostics.
    #[arg(
        long,
        value_name = "FIXTURE",
        conflicts_with_all = ["input", "lint_input", "lint_glob"]
    )]
    pub check_fixture: Option<PathBuf>,
    /// Input file path. Use '-' or omit to read stdin.
    #[arg(value_name = "INPUT", conflicts_with_all = ["lint_input", "lint_glob"])]
    pub input: Option<PathBuf>,

    /// Output file path. For multi-diagram file output, numbered files are generated.
    #[arg(short = 'o', long = "output", value_name = "OUTPUT")]
    pub output: Option<PathBuf>,

    /// Parse and normalize only; do not render or write outputs.
    #[arg(long, action = ArgAction::SetTrue, conflicts_with = "dump")]
    pub check: bool,

    /// Lint/check mode inputs (repeatable file paths).
    #[arg(long, action = ArgAction::Append, value_name = "INPUT", requires = "check")]
    pub lint_input: Vec<PathBuf>,

    /// Lint/check mode glob patterns (repeatable).
    #[arg(long, action = ArgAction::Append, value_name = "GLOB", requires = "check")]
    pub lint_glob: Vec<String>,

    /// Lint/check summary report format.
    #[arg(long, value_enum, default_value_t = LintReportFormat::Human)]
    pub lint_report: LintReportFormat,

    /// Dump intermediate representation.
    #[arg(long, value_enum, value_name = "KIND", conflicts_with = "check")]
    pub dump: Option<DumpKind>,

    /// Permit multiple stdin outputs (multiple @startuml blocks and/or newpage pages).
    #[arg(long, action = ArgAction::SetTrue)]
    pub multi: bool,

    /// Extract diagrams from markdown fenced code blocks (puml/pumlx/picouml/plantuml/uml/puml-sequence/uml-sequence/mermaid).
    #[arg(long, action = ArgAction::SetTrue)]
    pub from_markdown: bool,

    /// Diagnostics output format.
    #[arg(long, value_enum, default_value_t = DiagnosticsFormat::Human)]
    pub diagnostics: DiagnosticsFormat,

    /// Input dialect frontend.
    #[arg(long, value_enum, default_value_t = Dialect::Auto)]
    pub dialect: Dialect,

    /// Compatibility policy for semantic interpretation.
    #[arg(long, value_enum, default_value_t = CompatMode::Strict)]
    pub compat: CompatMode,

    /// Determinism policy for layout/output behavior.
    #[arg(long, value_enum, default_value_t = DeterminismMode::Strict)]
    pub determinism: DeterminismMode,

    /// Root directory used to resolve !include when reading from stdin.
    #[arg(long, value_name = "DIR")]
    pub include_root: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, ValueEnum, Eq, PartialEq)]
pub enum DiagnosticsFormat {
    Human,
    Json,
}

#[derive(Debug, Clone, Copy, ValueEnum, Eq, PartialEq)]
pub enum DumpKind {
    Ast,
    Model,
    Scene,
}

#[derive(Debug, Clone, Copy, ValueEnum, Eq, PartialEq)]
pub enum LintReportFormat {
    Human,
    Json,
}

#[derive(Debug, Clone, Copy, ValueEnum, Eq, PartialEq)]
pub enum Dialect {
    Auto,
    Plantuml,
    Mermaid,
    Picouml,
}

#[derive(Debug, Clone, Copy, ValueEnum, Eq, PartialEq)]
pub enum CompatMode {
    Strict,
    Extended,
}

#[derive(Debug, Clone, Copy, ValueEnum, Eq, PartialEq)]
pub enum DeterminismMode {
    Strict,
    Full,
}
