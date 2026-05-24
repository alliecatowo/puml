use super::super::EXIT_INTERNAL;
use super::{LintFileResult, LintSummary};
use crate::cli::LintReportFormat;
use serde::Serialize;

const LINT_REPORT_SCHEMA: &str = "puml.lint_report";
const LINT_REPORT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Serialize)]
struct LintReportPayload {
    schema: &'static str,
    schema_version: u32,
    summary: LintSummary,
    files: Vec<LintFileResult>,
}

pub(super) fn emit_lint_report(
    fmt: LintReportFormat,
    summary: &LintSummary,
    files: &[LintFileResult],
) -> Result<(), (u8, String)> {
    match fmt {
        LintReportFormat::Human => {
            println!(
                "lint summary: files={} passed={} failed={} diagrams={} passed_diagrams={} failed_diagrams={} warnings={} errors={}",
                summary.total_files,
                summary.passed_files,
                summary.failed_files,
                summary.total_diagrams,
                summary.passed_diagrams,
                summary.failed_diagrams,
                summary.warning_count,
                summary.error_count
            );
            for file in files.iter().filter(|f| !f.passed) {
                println!(
                    " - FAIL {} (diagrams={}, failed_diagrams={}, warnings={}, errors={})",
                    file.path, file.diagrams, file.failed_diagrams, file.warnings, file.errors
                );
            }
            Ok(())
        }
        LintReportFormat::Json => {
            let payload = LintReportPayload {
                schema: LINT_REPORT_SCHEMA,
                schema_version: LINT_REPORT_SCHEMA_VERSION,
                summary: LintSummary {
                    total_files: summary.total_files,
                    passed_files: summary.passed_files,
                    failed_files: summary.failed_files,
                    total_diagrams: summary.total_diagrams,
                    passed_diagrams: summary.passed_diagrams,
                    failed_diagrams: summary.failed_diagrams,
                    warning_count: summary.warning_count,
                    error_count: summary.error_count,
                },
                files: files
                    .iter()
                    .map(|f| LintFileResult {
                        path: f.path.clone(),
                        diagrams: f.diagrams,
                        failed_diagrams: f.failed_diagrams,
                        warnings: f.warnings,
                        errors: f.errors,
                        passed: f.passed,
                    })
                    .collect(),
            };
            println!(
                "{}",
                serde_json::to_string_pretty(&payload).map_err(|e| {
                    (
                        EXIT_INTERNAL,
                        format!("failed to serialize lint report: {e}"),
                    )
                })?
            );
            Ok(())
        }
    }
}
