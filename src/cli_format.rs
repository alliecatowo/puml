use crate::cli::FormatArgs;
use crate::cli_output::write_files_transactionally;
use crate::{EXIT_IO, EXIT_VALIDATION};
use std::fs;
use std::path::Path;

pub(crate) fn run_format_command(args: FormatArgs) -> Result<(), (u8, String)> {
    let mut changed_paths = Vec::new();
    let mut writes = Vec::new();

    for path in &args.files {
        if path == Path::new("-") {
            return Err((
                EXIT_VALIDATION,
                "puml format requires file paths; stdin cannot be formatted in place".to_string(),
            ));
        }
        let raw = fs::read_to_string(path)
            .map_err(|e| (EXIT_IO, format!("failed to read '{}': {e}", path.display())))?;
        let result = puml::formatter::format_source(&raw);
        if result.changed {
            changed_paths.push(path.clone());
            if args.diff {
                print!("{}", format_unified_diff(path, &raw, &result.formatted));
            }
            if !args.check && !args.diff {
                writes.push((path.clone(), result.formatted.into_bytes()));
            }
        }
    }

    if !writes.is_empty() {
        write_files_transactionally(writes)?;
    }

    if args.check && !changed_paths.is_empty() {
        let files = changed_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join(", ");
        return Err((
            EXIT_VALIDATION,
            format!("formatting changes needed: {files}"),
        ));
    }

    Ok(())
}

fn format_unified_diff(path: &Path, old: &str, new: &str) -> String {
    let old_display = old.replace("\r\n", "\n").replace('\r', "\n");
    let path_display = path.display();
    let mut diff = format!("--- {path_display}\n+++ {path_display} (formatted)\n");

    if old_display == new {
        diff.push_str("@@ line endings @@\n");
        diff.push_str("-contains CRLF or CR line endings\n");
        diff.push_str("+uses LF line endings\n");
        return diff;
    }

    let old_lines = diff_lines(&old_display);
    let new_lines = diff_lines(new);
    diff.push_str(&format!(
        "@@ -1,{} +1,{} @@\n",
        old_lines.len(),
        new_lines.len()
    ));
    for line in old_lines {
        diff.push('-');
        diff.push_str(line);
        diff.push('\n');
    }
    for line in new_lines {
        diff.push('+');
        diff.push_str(line);
        diff.push('\n');
    }
    diff
}

fn diff_lines(source: &str) -> Vec<&str> {
    let mut lines = source.split('\n').collect::<Vec<_>>();
    if source.ends_with('\n') {
        lines.pop();
    }
    lines
}
