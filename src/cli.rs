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

    /// Permit multiple diagrams.
    #[arg(long, action = ArgAction::SetTrue)]
    pub multi: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum, Eq, PartialEq)]
pub enum DumpKind {
    Ast,
    Model,
    Scene,
}
