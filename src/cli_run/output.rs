use super::{EXIT_INTERNAL, EXIT_IO, EXIT_VALIDATION};
use crate::cli::OutputFormat;
use puml::output::{render_svg_export_content, OutputError, OutputErrorKind, RenderedBinaryOutput};
use puml::{render, render_svg_pages_from_model, NormalizedDocument};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize)]
pub(super) struct MultiSvgOut {
    pub(super) name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) svg: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) html: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) text: Option<String>,
}

pub(super) fn render_pages_from_model(
    model: &NormalizedDocument,
    format: OutputFormat,
) -> Vec<String> {
    match format.text_mode() {
        Some(mode) => render::render_text_pages(model, mode),
        None => render_svg_pages_from_model(model)
            .into_iter()
            .map(|svg| render_svg_export_content(&svg, format))
            .collect(),
    }
}

pub(super) fn output_err(error: OutputError) -> (u8, String) {
    let code = match error.kind() {
        OutputErrorKind::Validation => EXIT_VALIDATION,
        OutputErrorKind::Io => EXIT_IO,
        OutputErrorKind::Internal | OutputErrorKind::Unsupported => EXIT_INTERNAL,
    };
    (code, error.message().to_string())
}

// All CLI pipeline parameters are required at call sites; grouping them into a
// struct would not reduce complexity here — the lint is a false positive.
pub(super) fn default_output_base(
    input: &Path,
    format: OutputFormat,
) -> Result<PathBuf, (u8, String)> {
    let stem = input.file_stem().and_then(|s| s.to_str()).ok_or_else(|| {
        (
            EXIT_IO,
            format!(
                "cannot derive output name from '{}': invalid stem",
                input.display()
            ),
        )
    })?;
    Ok(input.with_file_name(format!("{stem}.{}", format.extension())))
}

pub(super) fn write_markdown_output_files(
    input: &Path,
    outputs: &[RenderedBinaryOutput],
) -> Result<(), (u8, String)> {
    let parent = input.parent().unwrap_or_else(|| Path::new("."));
    let mut files = Vec::with_capacity(outputs.len());
    for (idx, out) in outputs.iter().enumerate() {
        let name = out.name_hint.as_ref().ok_or_else(|| {
            (
                EXIT_INTERNAL,
                format!("missing markdown output name for diagram {}", idx + 1),
            )
        })?;
        let path = parent.join(name);
        files.push((path, out.bytes.clone()));
    }
    write_files_transactionally(files)
}

pub(super) fn write_output_files(base: &Path, payloads: &[Vec<u8>]) -> Result<(), (u8, String)> {
    if payloads.len() == 1 {
        return write_files_transactionally(vec![(base.to_path_buf(), payloads[0].clone())]);
    }

    let stem = base.file_stem().and_then(|s| s.to_str()).ok_or_else(|| {
        (
            EXIT_IO,
            format!(
                "cannot derive output stem from '{}': invalid stem",
                base.display()
            ),
        )
    })?;
    let ext = base
        .extension()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("svg");
    let parent = base.parent().unwrap_or_else(|| Path::new("."));
    let mut files = Vec::with_capacity(payloads.len());

    for (idx, payload) in payloads.iter().enumerate() {
        let path = parent.join(format!("{stem}-{}.{}", idx + 1, ext));
        files.push((path, payload.clone()));
    }

    write_files_transactionally(files)
}

#[derive(Debug)]
struct StagedWrite {
    target: PathBuf,
    staged: PathBuf,
    backup: Option<PathBuf>,
    published: bool,
}

pub(super) fn write_files_transactionally(
    files: Vec<(PathBuf, Vec<u8>)>,
) -> Result<(), (u8, String)> {
    if files.is_empty() {
        return Ok(());
    }

    let pid = std::process::id();
    let mut staged_writes = Vec::with_capacity(files.len());

    for (idx, (target, contents)) in files.into_iter().enumerate() {
        if target.is_dir() {
            cleanup_staged_artifacts(&staged_writes);
            return Err((
                EXIT_IO,
                format!(
                    "failed to write '{}': target is a directory",
                    target.display()
                ),
            ));
        }
        let staged = staging_path_for(&target, "stage", pid, idx);
        fs::write(&staged, contents).map_err(|e| {
            cleanup_staged_artifacts(&staged_writes);
            (
                EXIT_IO,
                format!("failed to write '{}': {e}", target.display()),
            )
        })?;
        staged_writes.push(StagedWrite {
            target,
            staged,
            backup: None,
            published: false,
        });
    }

    let fail_after = transactional_write_fail_after();

    for idx in 0..staged_writes.len() {
        let target_display = staged_writes[idx].target.display().to_string();

        if staged_writes[idx].target.exists() {
            let backup = staging_path_for(&staged_writes[idx].target, "backup", pid, idx);
            if let Err(e) = fs::rename(&staged_writes[idx].target, &backup) {
                rollback_staged_writes(&mut staged_writes);
                return Err((
                    EXIT_IO,
                    format!("failed to prepare output '{target_display}': {e}"),
                ));
            }
            staged_writes[idx].backup = Some(backup);
        }

        if fail_after == Some(idx) {
            rollback_staged_writes(&mut staged_writes);
            return Err((
                EXIT_IO,
                format!("failed to write '{target_display}': simulated write failure"),
            ));
        }

        if let Err(e) = fs::rename(&staged_writes[idx].staged, &staged_writes[idx].target) {
            rollback_staged_writes(&mut staged_writes);
            return Err((EXIT_IO, format!("failed to write '{target_display}': {e}")));
        }

        staged_writes[idx].published = true;
    }

    for item in staged_writes {
        if let Some(backup) = item.backup {
            let _ = fs::remove_file(backup);
        }
    }

    Ok(())
}

fn staging_path_for(target: &Path, kind: &str, pid: u32, idx: usize) -> PathBuf {
    let parent = target.parent().unwrap_or_else(|| Path::new("."));
    let name = target
        .file_name()
        .and_then(|v| v.to_str())
        .unwrap_or("output");
    let base = format!(".{name}.puml.{kind}.{pid}.{idx}");
    for attempt in 0..32 {
        let candidate = parent.join(format!("{base}.{attempt}.tmp"));
        if !candidate.exists() {
            return candidate;
        }
    }
    parent.join(format!("{base}.overflow.tmp"))
}

fn rollback_staged_writes(staged_writes: &mut [StagedWrite]) {
    for item in staged_writes.iter_mut().rev() {
        if item.published {
            let _ = fs::remove_file(&item.target);
            if let Some(backup) = item.backup.take() {
                let _ = fs::rename(&backup, &item.target);
            }
        } else {
            let _ = fs::remove_file(&item.staged);
            if let Some(backup) = item.backup.take() {
                let _ = fs::rename(&backup, &item.target);
            }
        }
    }
}

fn cleanup_staged_artifacts(staged_writes: &[StagedWrite]) {
    for item in staged_writes {
        let _ = fs::remove_file(&item.staged);
    }
}

fn transactional_write_fail_after() -> Option<usize> {
    std::env::var("PUML_FAIL_OUTPUT_AFTER")
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
}
