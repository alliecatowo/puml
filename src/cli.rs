use clap::{ArgAction, Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

/// Parse a single `-DKEY=VALUE` or `-D KEY=VALUE` argument into `(key, value)`.
/// A bare key with no `=` is accepted and yields an empty value.
pub fn parse_define(raw: &str) -> Result<(String, String), String> {
    match raw.split_once('=') {
        Some((key, val)) => {
            let key = key.trim().to_string();
            if key.is_empty() {
                return Err(format!("variable name cannot be empty in '-D{raw}'"));
            }
            Ok((key, val.to_string()))
        }
        None => {
            let key = raw.trim().to_string();
            if key.is_empty() {
                return Err("variable name cannot be empty in '-D' flag".to_string());
            }
            Ok((key, String::new()))
        }
    }
}

#[derive(Debug, Clone, Parser)]
#[command(
    name = "puml",
    version,
    about = "Rust-native PlantUML-compatible diagram renderer with PicoUML and Mermaid adapter frontends"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

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

    /// Output file path. For multi outputs, numbered sibling files are generated (`<stem>-<n>.<ext>`).
    #[arg(short = 'o', long = "output", value_name = "OUTPUT")]
    pub output: Option<PathBuf>,

    /// Render output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Svg)]
    pub format: OutputFormat,

    /// PNG rasterization DPI (used only when `--format png`).
    #[arg(long, default_value_t = 96.0, value_parser = parse_dpi)]
    pub dpi: f32,

    /// Parse and normalize only; do not render or write outputs.
    #[arg(long, action = ArgAction::SetTrue, conflicts_with = "dump")]
    pub check: bool,

    /// Emit structured JSON metadata after parse and normalization.
    #[arg(long, action = ArgAction::SetTrue, conflicts_with_all = ["check", "dump"])]
    pub metadata: bool,

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

    /// Exit with code 1 if any warnings are emitted.
    #[arg(long, action = ArgAction::SetTrue)]
    pub fail_on_warn: bool,

    /// No-op compatibility flag (only UTF-8 input is supported).
    #[arg(long, value_name = "CHARSET", default_value = "UTF-8")]
    pub charset: String,

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
    /// Format PlantUML-compatible source files in place, or verify/print formatting changes.
    Format(FormatArgs),
    /// Print a deterministic content hash of the parsed AST of a .puml file.
    ///
    /// Useful for "did this file change semantically?" checks in CI pipelines.
    Hash(HashArgs),
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
    #[arg(long, value_enum, default_value_t = HashAlgoArg::Sha256)]
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
    Sha256,
    Blake3,
    Fnv,
}

#[derive(Debug, Clone, Copy, ValueEnum, Eq, PartialEq)]
pub enum HashFormatArg {
    Hex,
    Base64,
}

#[derive(Debug, Clone, Copy, ValueEnum, Eq, PartialEq)]
pub enum OutputFormat {
    Svg,
    Html,
    Png,
    Jpg,
    Webp,
    /// PDF output via SVG-to-PDF vector conversion.
    Pdf,
    Txt,
    Atxt,
    Utxt,
}

#[derive(Debug, Clone, Copy, ValueEnum, Eq, PartialEq)]
pub enum DiagnosticsFormat {
    Human,
    Json,
    Stdrpt,
}

#[derive(Debug, Clone, Copy, ValueEnum, Eq, PartialEq)]
pub enum ColorChoice {
    Auto,
    Always,
    Never,
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

fn parse_dpi(raw: &str) -> Result<f32, String> {
    let value = raw
        .parse::<f32>()
        .map_err(|e| format!("invalid DPI '{raw}': {e}"))?;
    if (1.0..=1200.0).contains(&value) {
        Ok(value)
    } else {
        Err("dpi must be in range [1, 1200]".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_parse_as_expected() {
        let cli = Cli::try_parse_from(["puml"]).expect("default CLI parse should succeed");
        assert!(cli.command.is_none());
        assert_eq!(cli.format, OutputFormat::Svg);
        assert_eq!(cli.dpi, 96.0);
        assert_eq!(cli.diagnostics, DiagnosticsFormat::Human);
        assert_eq!(cli.color, ColorChoice::Auto);
        assert_eq!(cli.dialect, Dialect::Auto);
        assert_eq!(cli.compat, CompatMode::Strict);
        assert_eq!(cli.determinism, DeterminismMode::Strict);
        assert_eq!(cli.lint_report, LintReportFormat::Human);
        assert_eq!(cli.charset, "UTF-8");
    }

    #[test]
    fn dpi_parser_accepts_boundaries_and_rejects_invalid_values() {
        assert_eq!(parse_dpi("1").expect("lower-bound DPI should parse"), 1.0);
        assert_eq!(
            parse_dpi("1200").expect("upper-bound DPI should parse"),
            1200.0
        );
        assert!(parse_dpi("0.999").is_err());
        assert!(parse_dpi("1200.1").is_err());
        assert!(parse_dpi("not-a-number").is_err());
    }

    #[test]
    fn check_conflicts_with_dump() {
        let err = Cli::try_parse_from(["puml", "--check", "--dump", "ast"])
            .expect_err("check + dump should conflict");
        assert_eq!(err.kind(), clap::error::ErrorKind::ArgumentConflict);
    }

    #[test]
    fn metadata_conflicts_with_check_and_dump() {
        let check_err = Cli::try_parse_from(["puml", "--metadata", "--check"])
            .expect_err("metadata + check should conflict");
        assert_eq!(check_err.kind(), clap::error::ErrorKind::ArgumentConflict);

        let dump_err = Cli::try_parse_from(["puml", "--metadata", "--dump", "scene"])
            .expect_err("metadata + dump should conflict");
        assert_eq!(dump_err.kind(), clap::error::ErrorKind::ArgumentConflict);
    }

    #[test]
    fn lint_inputs_require_check_mode() {
        let err = Cli::try_parse_from(["puml", "--lint-input", "x.puml"])
            .expect_err("lint input without check should fail");
        assert_eq!(err.kind(), clap::error::ErrorKind::MissingRequiredArgument);
    }

    #[test]
    fn url_include_flags_conflict() {
        let err = Cli::try_parse_from(["puml", "--allow-url-includes", "--no-url-includes"])
            .expect_err("allow/no-url-includes should conflict");
        assert_eq!(err.kind(), clap::error::ErrorKind::ArgumentConflict);
    }

    #[test]
    fn format_subcommand_parses_flags_and_files() {
        let cli = Cli::try_parse_from(["puml", "format", "--check", "--diff", "a.puml", "b.puml"])
            .expect("format subcommand should parse");
        match cli.command.expect("format command should be present") {
            Command::Format(args) => {
                assert!(args.check);
                assert!(args.diff);
                assert_eq!(
                    args.files,
                    vec![PathBuf::from("a.puml"), PathBuf::from("b.puml")]
                );
            }
        }
    }

    #[test]
    fn lint_flags_parse_when_check_mode_enabled() {
        let cli = Cli::try_parse_from([
            "puml",
            "--check",
            "--lint-input",
            "a.puml",
            "--lint-input",
            "b.puml",
            "--lint-glob",
            "tests/**/*.puml",
            "--lint-report",
            "json",
        ])
        .expect("lint flags should parse in check mode");

        assert!(cli.check);
        assert_eq!(
            cli.lint_input,
            vec![PathBuf::from("a.puml"), PathBuf::from("b.puml")]
        );
        assert_eq!(cli.lint_glob, vec!["tests/**/*.puml".to_string()]);
        assert_eq!(cli.lint_report, LintReportFormat::Json);
    }

    #[test]
    fn check_fixture_conflicts_with_positional_input() {
        let err = Cli::try_parse_from(["puml", "--check-fixture", "fixture.puml", "stdin.puml"])
            .expect_err("check fixture should conflict with positional input");
        assert_eq!(err.kind(), clap::error::ErrorKind::ArgumentConflict);
    }

    #[test]
    fn pdf_format_parses() {
        let cli =
            Cli::try_parse_from(["puml", "--format", "pdf"]).expect("--format pdf should parse");
        assert_eq!(cli.format, OutputFormat::Pdf);
    }
}
