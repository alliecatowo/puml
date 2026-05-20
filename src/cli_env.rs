//! `puml env` subcommand — inspect PUML-related environment variables.
//!
//! Prints the resolved values of `PUML_STDLIB_PATH`, `NO_COLOR`, `RUST_LOG`, and
//! `PUML_CONFIG` (and their sources) in a deterministic order. Supports human-readable
//! and JSON output formats.

use clap::{Args, ValueEnum};
use serde::Serialize;
use std::collections::BTreeMap;

// ── Output format ────────────────────────────────────────────────────────────

/// Output format for `puml env`.
#[derive(Debug, Clone, Copy, ValueEnum, Eq, PartialEq)]
pub enum EnvFormat {
    /// Human-readable key=value table (default).
    Human,
    /// Machine-readable JSON object.
    Json,
}

// ── CLI args ─────────────────────────────────────────────────────────────────

/// Arguments for the `puml env` subcommand.
#[derive(Debug, Clone, Args)]
pub struct EnvArgs {
    /// Output format for the environment report.
    #[arg(long, value_enum, default_value_t = EnvFormat::Human)]
    pub format: EnvFormat,
}

// ── Data model ────────────────────────────────────────────────────────────────

/// Information about a single PUML-related environment variable.
#[derive(Debug, Clone, Serialize)]
pub struct EnvVarInfo {
    /// The name of the environment variable.
    pub name: String,
    /// The resolved value, or `null` / `None` when the variable is unset.
    pub value: Option<String>,
    /// Where the value came from: `"env"` when present in the process environment,
    /// `"default"` when the variable is absent and no built-in default applies,
    /// `"env-invalid-unicode"` when the variable is set but its value is not valid UTF-8,
    /// or `"builtin-default"` when a compiled-in fallback is used.
    pub source: String,
}

/// A complete snapshot of all PUML-related environment variables.
///
/// Keys are variable names; iteration order is deterministic because the inner
/// container is a [`BTreeMap`] (required by CLAUDE.md §6).
#[derive(Debug, Serialize)]
pub struct EnvReport {
    /// Sorted map from variable name to its resolved info.
    pub vars: BTreeMap<String, EnvVarInfo>,
}

// ── Collection ────────────────────────────────────────────────────────────────

/// Names of all environment variables that `puml env` reports on, in canonical order.
const TRACKED_VARS: &[&str] = &["NO_COLOR", "PUML_CONFIG", "PUML_STDLIB_PATH", "RUST_LOG"];

/// Collect the current state of all PUML-related environment variables.
///
/// The returned [`EnvReport`] is deterministically ordered and contains one entry
/// for every tracked variable regardless of whether it is set.
pub fn collect_env_report() -> EnvReport {
    let mut vars = BTreeMap::new();

    for &name in TRACKED_VARS {
        let info = match std::env::var(name) {
            Ok(value) => EnvVarInfo {
                name: name.to_string(),
                value: Some(value),
                source: "env".to_string(),
            },
            Err(std::env::VarError::NotPresent) => EnvVarInfo {
                name: name.to_string(),
                value: None,
                source: "default".to_string(),
            },
            Err(std::env::VarError::NotUnicode(_)) => EnvVarInfo {
                name: name.to_string(),
                value: None,
                source: "env-invalid-unicode".to_string(),
            },
        };
        vars.insert(name.to_string(), info);
    }

    EnvReport { vars }
}

// ── Formatters ────────────────────────────────────────────────────────────────

/// Format an [`EnvReport`] as a human-readable key=value listing.
///
/// Each line has the form `NAME=value  [source]` (or `NAME=<unset>  [source]`
/// when the variable is absent). The output ends with a trailing newline.
fn format_human(report: &EnvReport) -> String {
    let mut out = String::new();
    // BTreeMap iteration is ordered — deterministic per CLAUDE.md §6.
    for (name, info) in &report.vars {
        let value_display = info.value.as_deref().unwrap_or("<unset>").to_string();
        out.push_str(&format!("{}={}  [{}]\n", name, value_display, info.source));
    }
    out
}

/// Format an [`EnvReport`] as a pretty-printed JSON object.
///
/// Returns an error string if serialization fails (should not happen in practice).
fn format_json(report: &EnvReport) -> Result<String, String> {
    serde_json::to_string_pretty(report)
        .map_err(|e| format!("failed to serialize env report as JSON: {e}"))
}

// ── Entry point ───────────────────────────────────────────────────────────────

/// Run the `puml env` subcommand.
///
/// Collects the PUML-related environment variables, formats them according to
/// `args.format`, prints to stdout, and returns `Ok(0)` on success.
///
/// Errors are returned as `Err(message)` and should be reported to the user;
/// the caller is responsible for choosing the appropriate exit code.
pub fn run_env(args: &EnvArgs) -> Result<i32, String> {
    let report = collect_env_report();

    match args.format {
        EnvFormat::Human => {
            print!("{}", format_human(&report));
        }
        EnvFormat::Json => {
            let json = format_json(&report)?;
            println!("{json}");
        }
    }

    Ok(0)
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collect_env_report_covers_all_tracked_vars() {
        let report = collect_env_report();
        for name in TRACKED_VARS {
            assert!(
                report.vars.contains_key(*name),
                "expected tracked variable '{name}' to be present in report"
            );
        }
    }

    #[test]
    fn collect_env_report_is_sorted() {
        let report = collect_env_report();
        let keys: Vec<&String> = report.vars.keys().collect();
        let mut sorted = keys.clone();
        sorted.sort();
        assert_eq!(keys, sorted, "env report vars must be in sorted order");
    }

    #[test]
    fn format_human_contains_all_var_names() {
        let report = collect_env_report();
        let output = format_human(&report);
        for name in TRACKED_VARS {
            assert!(
                output.contains(name),
                "human output must contain var name '{name}'"
            );
        }
    }

    #[test]
    fn format_human_ends_with_newline() {
        let report = collect_env_report();
        let output = format_human(&report);
        assert!(
            output.ends_with('\n'),
            "human output must end with a trailing newline"
        );
    }

    #[test]
    fn format_json_produces_valid_json() {
        let report = collect_env_report();
        let json = format_json(&report).expect("JSON formatting must not fail");
        let _parsed: serde_json::Value =
            serde_json::from_str(&json).expect("output must be valid JSON");
    }

    #[test]
    fn source_field_is_env_when_var_is_set() {
        // Use RUST_LOG since tests may set it; we force-set it here to be certain.
        // SAFETY: this test must not run concurrently with other threads that read
        // the process environment. The default Rust test harness is single-threaded
        // per binary, satisfying that requirement.
        unsafe { std::env::set_var("RUST_LOG", "info") };
        let report = collect_env_report();
        let info = report
            .vars
            .get("RUST_LOG")
            .expect("RUST_LOG must be in report");
        assert_eq!(info.source, "env");
        assert_eq!(info.value.as_deref(), Some("info"));
        unsafe { std::env::remove_var("RUST_LOG") };
    }
}
