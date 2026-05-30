mod cli;
mod cli_count;
mod cli_dump;
mod cli_dump_ast;
mod cli_env;
mod cli_hash;
mod cli_run;
mod cli_stats;
mod cli_watch;

use clap::{CommandFactory, FromArgMatches};
use cli::{Cli, ColorChoice as CliColorChoice};
use std::ffi::OsString;
use std::process::ExitCode;

const EXIT_OK: u8 = 0;
const EXIT_VALIDATION: u8 = 1;

fn main() -> ExitCode {
    let args = expand_plantuml_cli_compat_args(std::env::args_os());
    let clap_color = clap_color_choice_from_args(&args);
    let cli = match Cli::command()
        .color(clap_color)
        .try_get_matches_from(args)
        .and_then(|matches| Cli::from_arg_matches(&matches))
    {
        Ok(cli) => cli,
        Err(err) => {
            let code = if err.use_stderr() {
                EXIT_VALIDATION
            } else {
                EXIT_OK
            };
            let _ = err.print();
            return ExitCode::from(code);
        }
    };

    let failfast2 = cli.failfast2;
    match cli_run::run(cli) {
        Ok(()) => ExitCode::from(EXIT_OK),
        Err((code, msg)) => {
            if !msg.is_empty() {
                eprintln!("{msg}");
            }
            // --failfast2: remap validation exit code 1 → 2 to match PlantUML convention.
            let final_code = if failfast2 && code == EXIT_VALIDATION {
                2
            } else {
                code
            };
            ExitCode::from(final_code)
        }
    }
}

fn expand_plantuml_cli_compat_args<I>(args: I) -> Vec<OsString>
where
    I: IntoIterator<Item = OsString>,
{
    let mut expanded = Vec::new();
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--format" | "--output-format") => {
                if let Some(value) = args.next() {
                    push_format_arg(&mut expanded, &value, "--format");
                } else {
                    expanded.push(arg);
                }
            }
            Some(raw) if raw.starts_with("--format=") => {
                let value = raw.trim_start_matches("--format=");
                push_format_value(&mut expanded, value, "--format");
            }
            Some(raw) if raw.starts_with("--output-format=") => {
                let value = raw.trim_start_matches("--output-format=");
                push_format_value(&mut expanded, value, "--format");
            }
            Some("-txt") => {
                expanded.push(OsString::from("--format"));
                expanded.push(OsString::from("txt"));
            }
            Some("-atxt") => {
                expanded.push(OsString::from("--format"));
                expanded.push(OsString::from("atxt"));
            }
            Some("-utxt") => {
                expanded.push(OsString::from("--format"));
                expanded.push(OsString::from("utxt"));
            }
            Some("-encodesprite") => expanded.push(OsString::from("--encodesprite")),
            Some("-stdlib") => expanded.push(OsString::from("--stdlib")),
            Some(raw) if raw.starts_with("-t") && raw.len() > 2 => {
                push_format_value(&mut expanded, &raw[2..], "--format");
            }
            _ => expanded.push(arg),
        }
    }
    expanded
}

fn push_format_arg(expanded: &mut Vec<OsString>, value: &OsString, flag: &str) {
    if let Some(raw) = value.to_str() {
        push_format_value(expanded, raw, flag);
    } else {
        expanded.push(OsString::from(flag));
        expanded.push(value.clone());
    }
}

fn push_format_value(expanded: &mut Vec<OsString>, raw: &str, flag: &str) {
    match normalize_supported_format_alias(raw) {
        Some(format) => {
            expanded.push(OsString::from(flag));
            expanded.push(OsString::from(format));
        }
        None if is_known_unsupported_format(raw) => {
            expanded.push(OsString::from("--unsupported-output-format"));
            expanded.push(OsString::from(raw));
        }
        None => {
            expanded.push(OsString::from(flag));
            expanded.push(OsString::from(raw));
        }
    }
}

fn normalize_supported_format_alias(raw: &str) -> Option<&'static str> {
    match raw.to_ascii_lowercase().as_str() {
        "svg" => Some("svg"),
        "html" => Some("html"),
        "png" => Some("png"),
        "jpg" | "jpeg" => Some("jpg"),
        "webp" => Some("webp"),
        "pdf" => Some("pdf"),
        "txt" => Some("txt"),
        "atxt" => Some("atxt"),
        "utxt" => Some("utxt"),
        _ => None,
    }
}

fn is_known_unsupported_format(raw: &str) -> bool {
    matches!(
        raw.to_ascii_lowercase().as_str(),
        "latex" | "latex:nopreamble"
    )
}

fn clap_color_choice_from_args(args: &[OsString]) -> clap::ColorChoice {
    match color_choice_from_args(args).unwrap_or_else(default_color_choice_from_env) {
        CliColorChoice::Always => clap::ColorChoice::Always,
        CliColorChoice::Never => clap::ColorChoice::Never,
        CliColorChoice::Auto => clap::ColorChoice::Auto,
    }
}

fn default_color_choice_from_env() -> CliColorChoice {
    if std::env::var_os("NO_COLOR").is_some() {
        CliColorChoice::Never
    } else {
        CliColorChoice::Auto
    }
}

fn color_choice_from_args(args: &[OsString]) -> Option<CliColorChoice> {
    let mut iter = args.iter().skip(1);
    while let Some(arg) = iter.next() {
        let Some(raw) = arg.to_str() else {
            continue;
        };
        if let Some(value) = raw.strip_prefix("--color=") {
            return parse_color_choice(value);
        }
        if raw == "--color" {
            return iter
                .next()
                .and_then(|value| value.to_str())
                .and_then(parse_color_choice);
        }
    }
    None
}

fn parse_color_choice(raw: &str) -> Option<CliColorChoice> {
    match raw {
        "auto" => Some(CliColorChoice::Auto),
        "always" => Some(CliColorChoice::Always),
        "never" => Some(CliColorChoice::Never),
        _ => None,
    }
}
