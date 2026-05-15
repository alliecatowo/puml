use clap::{ArgAction, Parser, ValueEnum};
use std::path::PathBuf;

#[derive(Debug, Clone, Parser)]
#[command(name = "puml", version, about = "PlantUML CLI")]
pub struct Cli {
    /// Input file path. Use '-' or omit to read stdin.
    #[arg(value_name = "INPUT")]
    pub input: Option<PathBuf>,

    /// Output file path. For multi-diagram file output, numbered files are generated.
    #[arg(short = 'o', long = "output", value_name = "OUTPUT")]
    pub output: Option<PathBuf>,

    /// Parse and normalize only; do not render or write outputs.
    #[arg(long, action = ArgAction::SetTrue, conflicts_with = "dump")]
    pub check: bool,

    /// Dump intermediate representation.
    #[arg(long, value_enum, value_name = "KIND", conflicts_with = "check")]
    pub dump: Option<DumpKind>,

    /// Permit multiple stdin outputs (multiple @startuml blocks and/or newpage pages).
    #[arg(long, action = ArgAction::SetTrue)]
    pub multi: bool,

    /// Extract diagrams from markdown fenced code blocks (```puml / ```plantuml / ```uml).
    #[arg(long, action = ArgAction::SetTrue)]
    pub from_markdown: bool,

    /// Diagnostics output format.
    #[arg(long, value_enum, default_value_t = DiagnosticsFormat::Human)]
    pub diagnostics: DiagnosticsFormat,

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
