use clap::{ArgAction, ArgGroup, Args, Parser, Subcommand, ValueEnum};
pub use puml::output::OutputFormat;
use std::path::PathBuf;

// Re-export so that main.rs can import EnvArgs from cli without knowing cli_env.
pub use crate::cli_env::EnvArgs;

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
    #[arg(long, value_enum, default_value_t = OutputFormat::Svg)]
    pub format: OutputFormat,

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
        assert!(cli.encodesprite.is_empty());
        assert!(!cli.htmlcss);
        assert!(!cli.pipe);
        assert!(!cli.check_syntax);
        assert!(!cli.preproc);
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

        let check_syntax_err = Cli::try_parse_from(["puml", "--metadata", "--check-syntax"])
            .expect_err("metadata + check-syntax should conflict");
        assert_eq!(
            check_syntax_err.kind(),
            clap::error::ErrorKind::ArgumentConflict
        );

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
    fn pipe_compat_flag_selects_stdin_stdout_mode() {
        let cli = Cli::try_parse_from(["puml", "--pipe", "--format", "svg"])
            .expect("--pipe should parse");
        assert!(cli.pipe);
        assert!(cli.input.is_none());
        assert!(cli.output.is_none());
        assert_eq!(cli.format, OutputFormat::Svg);
    }

    #[test]
    fn pipe_compat_flag_conflicts_with_file_outputs() {
        let input_err = Cli::try_parse_from(["puml", "--pipe", "diag.puml"])
            .expect_err("--pipe should not accept a positional input file");
        assert_eq!(input_err.kind(), clap::error::ErrorKind::ArgumentConflict);

        let output_err = Cli::try_parse_from(["puml", "--pipe", "-o", "diag.svg"])
            .expect_err("--pipe should always write to stdout");
        assert_eq!(output_err.kind(), clap::error::ErrorKind::ArgumentConflict);
    }

    #[test]
    fn preproc_compat_flag_selects_stdout_preprocessor_dump() {
        let cli = Cli::try_parse_from(["puml", "--preproc", "diag.puml"])
            .expect("--preproc should parse");
        assert!(cli.preproc);
        assert_eq!(cli.input, Some(PathBuf::from("diag.puml")));
        assert!(cli.output.is_none());
    }

    #[test]
    fn preproc_compat_flag_conflicts_with_other_non_render_modes_and_output() {
        for args in [
            vec!["puml", "--preproc", "--check"],
            vec!["puml", "--preproc", "--check-syntax"],
            vec!["puml", "--preproc", "--dump", "ast"],
            vec!["puml", "--preproc", "--metadata"],
            vec!["puml", "--preproc", "-o", "out.puml"],
        ] {
            let err = Cli::try_parse_from(args).expect_err("--preproc combination should fail");
            assert_eq!(err.kind(), clap::error::ErrorKind::ArgumentConflict);
        }
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
            Command::Count(_) => panic!("unexpected count command"),
            Command::Env(_) => panic!("unexpected env command"),
            Command::Hash(_) => panic!("unexpected hash command"),
            Command::Lint(_) => panic!("unexpected lint command"),
            Command::Stats(_) => panic!("unexpected stats command"),
        }
    }

    #[test]
    fn count_subcommand_parses_file_and_flags() {
        let cli = Cli::try_parse_from(["puml", "count", "--by-kind", "diag.puml"])
            .expect("count subcommand should parse");
        match cli.command.expect("count command should be present") {
            Command::Count(args) => {
                assert_eq!(args.file, PathBuf::from("diag.puml"));
                assert!(args.by_kind);
            }
            Command::Env(_) => panic!("unexpected env command"),
            Command::Format(_) => panic!("unexpected format command"),
            Command::Hash(_) => panic!("unexpected hash command"),
            Command::Lint(_) => panic!("unexpected lint command"),
            Command::Stats(_) => panic!("unexpected stats command"),
        }
    }

    #[test]
    fn lint_subcommand_parses_file_and_flags() {
        let cli = Cli::try_parse_from(["puml", "lint", "my.puml", "--format", "json", "--quiet"])
            .expect("lint subcommand should parse");
        match cli.command.expect("lint command should be present") {
            Command::Lint(args) => {
                assert_eq!(args.file, PathBuf::from("my.puml"));
                assert_eq!(args.format, LintFormat::Json);
                assert!(args.quiet);
            }
            Command::Count(_) => panic!("unexpected count command"),
            Command::Env(_) => panic!("unexpected env command"),
            Command::Format(_) => panic!("unexpected format command"),
            Command::Hash(_) => panic!("unexpected hash command"),
            Command::Stats(_) => panic!("unexpected stats command"),
        }
    }

    #[test]
    fn stats_subcommand_parses_file_and_format() {
        let cli = Cli::try_parse_from(["puml", "stats", "--format", "json", "diag.puml"])
            .expect("stats subcommand should parse");
        match cli.command.expect("stats command should be present") {
            Command::Stats(args) => {
                assert_eq!(args.file, PathBuf::from("diag.puml"));
                assert_eq!(args.format, StatsFormat::Json);
            }
            Command::Count(_) => panic!("unexpected count command"),
            Command::Env(_) => panic!("unexpected env command"),
            Command::Format(_) => panic!("unexpected format command"),
            Command::Hash(_) => panic!("unexpected hash command"),
            Command::Lint(_) => panic!("unexpected lint command"),
        }
    }

    #[test]
    fn hash_subcommand_parses_file_format_and_algorithm() {
        let cli = Cli::try_parse_from([
            "puml",
            "hash",
            "--algo",
            "fnv",
            "--format",
            "base64",
            "diag.puml",
        ])
        .expect("hash subcommand should parse");
        match cli.command.expect("hash command should be present") {
            Command::Hash(args) => {
                assert_eq!(args.file, PathBuf::from("diag.puml"));
                assert_eq!(args.algo, HashAlgoArg::Fnv);
                assert_eq!(args.format, HashFormatArg::Base64);
            }
            Command::Count(_) => panic!("unexpected count command"),
            Command::Env(_) => panic!("unexpected env command"),
            Command::Format(_) => panic!("unexpected format command"),
            Command::Lint(_) => panic!("unexpected lint command"),
            Command::Stats(_) => panic!("unexpected stats command"),
        }
    }

    #[test]
    fn lint_subcommand_defaults_to_human_format() {
        let cli =
            Cli::try_parse_from(["puml", "lint", "diag.puml"]).expect("lint should parse bare");
        match cli.command.expect("command should be present") {
            Command::Lint(args) => {
                assert_eq!(args.format, LintFormat::Human);
                assert!(!args.quiet);
            }
            Command::Count(_) => panic!("unexpected count command"),
            Command::Env(_) => panic!("unexpected env command"),
            Command::Format(_) => panic!("unexpected format command"),
            Command::Hash(_) => panic!("unexpected hash command"),
            Command::Stats(_) => panic!("unexpected stats command"),
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
    fn check_syntax_alias_enables_check_mode() {
        let cli = Cli::try_parse_from([
            "puml",
            "--check-syntax",
            "--lint-input",
            "a.puml",
            "--lint-report",
            "json",
        ])
        .expect("--check-syntax should satisfy check-mode lint flags");

        assert!(!cli.check);
        assert!(cli.check_syntax);
        assert_eq!(cli.lint_input, vec![PathBuf::from("a.puml")]);
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

    #[test]
    fn encodesprite_parses_format_and_image() {
        let cli = Cli::try_parse_from(["puml", "--encodesprite", "16z", "icon.png"])
            .expect("encodesprite should parse");
        assert_eq!(cli.encodesprite, vec!["16z", "icon.png"]);
    }

    #[test]
    fn htmlcss_compat_flag_parses() {
        let cli = Cli::try_parse_from(["puml", "--htmlcss", "--format", "html", "diag.puml"])
            .expect("--htmlcss should parse");
        assert!(cli.htmlcss);
        assert_eq!(cli.format, OutputFormat::Html);
        assert_eq!(cli.input, Some(PathBuf::from("diag.puml")));
    }

    #[test]
    fn env_subcommand_parses_default_format() {
        let cli = Cli::try_parse_from(["puml", "env"]).expect("env subcommand should parse");
        match cli.command.expect("env command should be present") {
            Command::Env(args) => {
                assert_eq!(args.format, crate::cli_env::EnvFormat::Human);
            }
            Command::Count(_) => panic!("unexpected count command"),
            Command::Format(_) => panic!("unexpected Format command"),
            Command::Hash(_) => panic!("unexpected hash command"),
            Command::Lint(_) => panic!("unexpected lint command"),
            Command::Stats(_) => panic!("unexpected stats command"),
        }
    }

    #[test]
    fn env_subcommand_parses_json_format() {
        let cli = Cli::try_parse_from(["puml", "env", "--format", "json"])
            .expect("env --format json should parse");
        match cli.command.expect("env command should be present") {
            Command::Env(args) => {
                assert_eq!(args.format, crate::cli_env::EnvFormat::Json);
            }
            Command::Count(_) => panic!("unexpected count command"),
            Command::Format(_) => panic!("unexpected Format command"),
            Command::Hash(_) => panic!("unexpected hash command"),
            Command::Lint(_) => panic!("unexpected lint command"),
            Command::Stats(_) => panic!("unexpected stats command"),
        }
    }
}
