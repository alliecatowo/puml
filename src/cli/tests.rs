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
    assert_eq!(cli.lint_report, LintReportFormat::Human);
    assert_eq!(cli.charset, "UTF-8");
    assert!(cli.encodesprite.is_empty());
    assert!(!cli.htmlcss);
    assert!(!cli.pipe);
    assert!(!cli.check_syntax);
    assert!(!cli.preproc);
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
    let cli =
        Cli::try_parse_from(["puml", "--pipe", "--format", "svg"]).expect("--pipe should parse");
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
    let cli =
        Cli::try_parse_from(["puml", "--preproc", "diag.puml"]).expect("--preproc should parse");
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
    let cli = Cli::try_parse_from(["puml", "lint", "diag.puml"]).expect("lint should parse bare");
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
    let cli = Cli::try_parse_from(["puml", "--format", "pdf"]).expect("--format pdf should parse");
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
