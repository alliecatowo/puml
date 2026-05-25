use clap::{ArgAction, ArgGroup, Args, Parser, Subcommand, ValueEnum};
pub use puml::output::OutputFormat;
use std::path::PathBuf;

mod options;
pub use options::{
    parse_define, parse_dpi, parse_threads, ColorChoice, CompatMode, DeterminismMode,
    DiagnosticsFormat, Dialect, DumpKind, LintReportFormat,
};

// Re-export so that main.rs can import EnvArgs from cli without knowing cli_env.
pub use crate::cli_env::EnvArgs;

#[derive(Debug, Clone, Parser)]
#[command(
    name = "puml",
    version,
    about = "Rust-native PlantUML-compatible diagram renderer with PicoUML and Mermaid adapter frontends"
)]
#[command(group(
    ArgGroup::new("check_mode")
        .args(["check", "check_syntax"])
        .multiple(true)
))]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Print puml-lsp capability manifest and exit.
    #[arg(long, action = ArgAction::SetTrue)]
    pub dump_capabilities: bool,

    /// List reachable local stdlib include paths and exit.
    ///
    /// PlantUML-compatible `-stdlib` is accepted as an alias.
    #[arg(
        long = "stdlib",
        action = ArgAction::SetTrue,
        conflicts_with_all = [
            "input",
            "output",
            "pipe",
            "check_fixture",
            "lint_input",
            "lint_glob",
            "preproc",
            "metadata",
            "check",
            "check_syntax",
            "dump",
            "encodesprite",
            "watch"
        ]
    )]
    pub stdlib: bool,

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

    /// Output file path. For multi outputs, numbered sibling files are generated (`<stem>-<n>.<ext>`).
    #[arg(short = 'o', long = "output", value_name = "OUTPUT")]
    pub output: Option<PathBuf>,

    /// PlantUML-compatible stdin-to-stdout render mode.
    #[arg(long, action = ArgAction::SetTrue, conflicts_with_all = ["input", "output"])]
    pub pipe: bool,

    /// Render output format.
    #[arg(long, visible_alias = "output-format", value_enum, default_value_t = OutputFormat::Svg)]
    pub format: OutputFormat,

    /// Unsupported PlantUML output format requested through a parity alias.
    #[arg(long = "unsupported-output-format", hide = true, value_name = "FORMAT")]
    pub unsupported_output_format: Option<String>,

    /// PNG rasterization DPI (used only when `--format png`).
    #[arg(long, default_value_t = 96.0, value_parser = parse_dpi)]
    pub dpi: f32,

    /// Parse and normalize only; do not render or write outputs.
    #[arg(long, action = ArgAction::SetTrue, conflicts_with = "dump")]
    pub check: bool,

    /// PlantUML-compatible alias for `--check`.
    #[arg(long = "check-syntax", action = ArgAction::SetTrue, conflicts_with = "dump")]
    pub check_syntax: bool,

    /// Dump preprocessed source after include/macro expansion.
    #[arg(
        long,
        action = ArgAction::SetTrue,
        conflicts_with_all = ["check", "check_syntax", "dump", "metadata", "output"]
    )]
    pub preproc: bool,

    /// Emit structured JSON metadata after parse and normalization.
    #[arg(
        long,
        action = ArgAction::SetTrue,
        conflicts_with_all = ["check", "check_syntax", "dump"]
    )]
    pub metadata: bool,

    /// Lint/check mode inputs (repeatable file paths).
    #[arg(
        long,
        action = ArgAction::Append,
        value_name = "INPUT",
        requires = "check_mode"
    )]
    pub lint_input: Vec<PathBuf>,

    /// Lint/check mode glob patterns (repeatable).
    #[arg(
        long,
        action = ArgAction::Append,
        value_name = "GLOB",
        requires = "check_mode"
    )]
    pub lint_glob: Vec<String>,

    /// Lint/check summary report format.
    #[arg(long, value_enum, default_value_t = LintReportFormat::Human)]
    pub lint_report: LintReportFormat,

    /// Dump intermediate representation.
    #[arg(long, value_enum, value_name = "KIND", conflicts_with = "check")]
    pub dump: Option<DumpKind>,

    /// Permit multiple stdin render outputs (multiple @startuml blocks and/or `newpage` pages).
    /// File inputs always emit numbered files for multi outputs without this flag.
    #[arg(long, action = ArgAction::SetTrue)]
    pub multi: bool,

    /// Extract diagrams from markdown fenced code blocks (puml/pumlx/picouml/plantuml/uml/puml-sequence/uml-sequence/mermaid).
    #[arg(long, action = ArgAction::SetTrue)]
    pub from_markdown: bool,

    /// Diagnostics output format.
    #[arg(long, value_enum, default_value_t = DiagnosticsFormat::Human)]
    pub diagnostics: DiagnosticsFormat,

    /// When to use ANSI color in human CLI output.
    #[arg(long, value_enum, default_value_t = ColorChoice::Auto)]
    pub color: ColorChoice,

    /// Input frontend dialect (`auto` uses file extensions and markdown fence tags).
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

    /// Inject a preprocessor variable. Format: KEY=VALUE or KEY (empty value).
    /// Repeatable. Variables are accessible as `$KEY` in diagram source.
    /// Mirrors PlantUML `-DKEY=VALUE` convention.
    #[arg(
        short = 'D',
        value_name = "KEY=VALUE",
        action = ArgAction::Append,
        value_parser = parse_define,
        allow_hyphen_values = true
    )]
    pub defines: Vec<(String, String)>,

    /// No-op compatibility flag (outputs are always overwritten in place).
    #[arg(long, action = ArgAction::SetTrue)]
    pub overwrite: bool,

    /// No-op PlantUML compatibility flag for HTML exports.
    ///
    /// `puml --format html` already emits a self-contained HTML document with CSS.
    #[arg(long, action = ArgAction::SetTrue)]
    pub htmlcss: bool,

    /// PlantUML compatibility thread-count hint; execution remains deterministic.
    #[arg(long, value_name = "N", default_value_t = 1, value_parser = parse_threads)]
    pub threads: usize,

    /// No-op PlantUML compatibility flag; this CLI already stops on the first fatal error.
    #[arg(long, action = ArgAction::SetTrue)]
    pub failfast2: bool,

    /// Split a multi-diagram input into deterministic .puml source files.
    #[arg(
        long,
        action = ArgAction::SetTrue,
        conflicts_with_all = ["check", "check_syntax", "dump", "metadata", "preproc"]
    )]
    pub extract: bool,

    /// Regex filter applied to lint/check file selection.
    #[arg(long, value_name = "REGEX")]
    pub pattern: Option<String>,

    /// Encode a PNG/JPEG/WebP image as a PlantUML sprite. Format: 4, 8, 16, 4z, 8z, or 16z.
    #[arg(
        long = "encodesprite",
        value_names = ["FORMAT", "IMAGE"],
        num_args = 2,
        conflicts_with_all = [
            "input",
            "check_fixture",
            "lint_input",
            "lint_glob",
            "metadata",
            "check",
            "dump"
        ]
    )]
    pub encodesprite: Vec<String>,

    /// Exit with code 1 if any warnings are emitted.
    #[arg(long, action = ArgAction::SetTrue)]
    pub fail_on_warn: bool,

    /// No-op compatibility flag (only UTF-8 input is supported).
    #[arg(long, value_name = "CHARSET", default_value = "UTF-8")]
    pub charset: String,

    /// Watch the input file for changes and re-render on each mtime update.
    /// Polls the file; no inotify or OS-watch dependency.
    #[arg(long, action = ArgAction::SetTrue)]
    pub watch: bool,

    /// Print elapsed wall time to stderr after run completes.
    #[arg(long, action = ArgAction::SetTrue)]
    pub duration: bool,

    /// Suppress non-error stderr output.
    #[arg(long, short = 'q', action = ArgAction::SetTrue, conflicts_with = "verbose")]
    pub quiet: bool,

    /// Emit additional stage timings/diagnostics to stderr.
    #[arg(long, short = 'v', action = ArgAction::SetTrue)]
    pub verbose: bool,

    /// Emit diagnostics in single-line tab-separated format:
    /// `<severity>\t<code>\t<file>:<line>:<col>\t<message>`. Suppresses multi-line
    /// source-context output. Exit codes are unchanged.
    #[arg(long, action = ArgAction::SetTrue)]
    pub stdrpt: bool,

    /// Allow URL includes (`!include https://...`, `!includeurl`, and `file://`).
    /// URL includes are disabled by default to avoid surprise network or local file reads.
    #[arg(long, action = ArgAction::SetTrue, conflicts_with = "no_url_includes")]
    pub allow_url_includes: bool,

    /// Disable URL includes (`!include https://...`). Kept as a compatibility
    /// flag; URL includes are already disabled unless `--allow-url-includes` is passed.
    #[arg(long, action = ArgAction::SetTrue)]
    pub no_url_includes: bool,
}

#[derive(Debug, Clone, Subcommand)]
pub enum Command {
    /// Count normalized diagram nodes and edges.
    Count(CountArgs),
    /// Print PUML-related environment variables and their resolved values.
    Env(EnvArgs),
    /// Format PlantUML-compatible source files in place, or verify/print formatting changes.
    Format(FormatArgs),
    /// Print a deterministic raw-byte content hash of a file.
    Hash(HashArgs),
    /// Parse and normalize a .puml file and emit diagnostics without rendering.
    ///
    /// Useful as a fast pre-commit check. Exits 0 when no errors are found,
    /// 1 when any diagnostic errors are emitted, and 2 on I/O failure.
    Lint(LintArgs),
    /// Summarize parsed diagram structure as human-readable text or JSON.
    Stats(StatsArgs),
}

#[derive(Debug, Clone, Args)]
pub struct StatsArgs {
    /// PlantUML source file to inspect.
    #[arg(value_name = "FILE", required = true)]
    pub file: PathBuf,

    /// Output format for the summary.
    #[arg(long, value_enum, default_value_t = StatsFormat::Human)]
    pub format: StatsFormat,
}

#[derive(Debug, Clone, Copy, ValueEnum, Eq, PartialEq)]
pub enum StatsFormat {
    Human,
    Json,
}

#[derive(Debug, Clone, Args)]
pub struct CountArgs {
    /// PlantUML source file to count.
    #[arg(value_name = "FILE", required = true)]
    pub file: PathBuf,

    /// Print a per-kind node breakdown.
    #[arg(long, action = ArgAction::SetTrue)]
    pub by_kind: bool,
}

#[derive(Debug, Clone, Args)]
pub struct LintArgs {
    /// PlantUML source file to lint. Use '-' to read from stdin.
    #[arg(value_name = "FILE", required = true)]
    pub file: PathBuf,

    /// Output format for diagnostics.
    #[arg(long, value_enum, default_value_t = LintFormat::Human)]
    pub format: LintFormat,

    /// Suppress all non-error output (warnings, summary lines).
    #[arg(long, short = 'q', action = ArgAction::SetTrue)]
    pub quiet: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum, Eq, PartialEq)]
pub enum LintFormat {
    Human,
    Json,
}

#[derive(Debug, Clone, Args)]
pub struct FormatArgs {
    /// Exit with code 1 when any file would be reformatted.
    #[arg(long, action = ArgAction::SetTrue)]
    pub check: bool,

    /// Print a readable unified diff instead of writing files.
    #[arg(long, action = ArgAction::SetTrue)]
    pub diff: bool,

    /// PlantUML-compatible source files to format.
    #[arg(value_name = "FILE", required = true)]
    pub files: Vec<PathBuf>,
}

#[derive(Debug, Clone, Args)]
pub struct HashArgs {
    /// Hash algorithm to use.
    #[arg(long, value_enum, default_value_t = HashAlgoArg::Fnv)]
    pub algo: HashAlgoArg,

    /// Output encoding for the hash digest.
    #[arg(long, value_enum, default_value_t = HashFormatArg::Hex)]
    pub format: HashFormatArg,

    /// The .puml file to hash.
    #[arg(value_name = "FILE")]
    pub file: PathBuf,
}

#[derive(Debug, Clone, Copy, ValueEnum, Eq, PartialEq)]
pub enum HashAlgoArg {
    Fnv,
}

#[derive(Debug, Clone, Copy, ValueEnum, Eq, PartialEq)]
pub enum HashFormatArg {
    Hex,
    Base64,
}

#[cfg(test)]
mod tests;
