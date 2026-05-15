use clap::{ArgAction, Parser, ValueEnum};
use std::path::PathBuf;

#[derive(Debug, Clone, Parser)]
#[command(
    name = "puml",
    version,
    about = "PlantUML CLI scaffold for parser/layout/render pipeline"
)]
pub struct Cli {
    /// Input file path. Use '-' or omit to read stdin.
    #[arg(value_name = "INPUT")]
    pub input: Option<PathBuf>,

    /// Validate input without rendering.
    #[arg(long, action = ArgAction::SetTrue, conflicts_with = "dump")]
    pub check: bool,

    /// Dump intermediate diagram payload.
    #[arg(long, action = ArgAction::SetTrue, conflicts_with = "check")]
    pub dump: bool,

    /// Permit multiple diagrams and return JSON array output.
    #[arg(long, action = ArgAction::SetTrue)]
    pub multi: bool,

    /// Output format for single-diagram mode.
    #[arg(long, default_value = "text")]
    pub format: OutputFormat,
}

#[derive(Debug, Clone, Copy, ValueEnum, Eq, PartialEq)]
pub enum OutputFormat {
    Text,
    Json,
}
