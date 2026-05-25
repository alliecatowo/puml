use super::{input::InputDiagram, output::write_files_transactionally, EXIT_IO};
use crate::cli::Cli;
use std::path::{Path, PathBuf};

pub(super) fn run_extract_mode(
    cli: &Cli,
    diagrams: &[InputDiagram],
    input_path: Option<&Path>,
) -> Result<(), (u8, String)> {
    let payloads = diagrams
        .iter()
        .map(|diagram| extracted_source_bytes(&diagram.source))
        .collect::<Vec<_>>();

    if cli.output.as_deref() == Some(Path::new("-"))
        || (input_path.is_none() && cli.output.is_none())
    {
        for (idx, payload) in payloads.iter().enumerate() {
            if idx > 0 {
                println!();
            }
            print!("{}", String::from_utf8_lossy(payload));
        }
        if cli.verbose {
            eprintln!(
                "[verbose] extracted {} diagram source(s) to stdout",
                payloads.len()
            );
        }
        return Ok(());
    }

    let targets = if let Some(output) = &cli.output {
        numbered_extract_paths(output, payloads.len())?
    } else if let Some(input) = input_path {
        default_extract_paths(input, diagrams)?
    } else {
        numbered_extract_paths(Path::new("diagram.puml"), payloads.len())?
    };
    let files = targets.into_iter().zip(payloads).collect::<Vec<_>>();
    let count = files.len();
    write_files_transactionally(files)?;
    if cli.verbose {
        eprintln!("[verbose] extracted {count} diagram source file(s)");
    }
    Ok(())
}

fn extracted_source_bytes(source: &str) -> Vec<u8> {
    let mut text = source.trim().to_string();
    text.push('\n');
    text.into_bytes()
}

fn default_extract_paths(
    input: &Path,
    diagrams: &[InputDiagram],
) -> Result<Vec<PathBuf>, (u8, String)> {
    let parent = input.parent().unwrap_or_else(|| Path::new("."));
    let stem = input.file_stem().and_then(|s| s.to_str()).ok_or_else(|| {
        (
            EXIT_IO,
            format!(
                "cannot derive extraction name from '{}': invalid stem",
                input.display()
            ),
        )
    })?;
    let multi = diagrams.len() > 1;
    Ok(diagrams
        .iter()
        .enumerate()
        .map(|(idx, diagram)| {
            if let Some(hint) = &diagram.output_name_hint {
                parent.join(format!("{hint}.puml"))
            } else if multi {
                parent.join(format!("{stem}-extracted-{}.puml", idx + 1))
            } else {
                parent.join(format!("{stem}-extracted.puml"))
            }
        })
        .collect())
}

fn numbered_extract_paths(base: &Path, count: usize) -> Result<Vec<PathBuf>, (u8, String)> {
    if count == 1 {
        return Ok(vec![base.to_path_buf()]);
    }
    let stem = base.file_stem().and_then(|s| s.to_str()).ok_or_else(|| {
        (
            EXIT_IO,
            format!(
                "cannot derive extraction stem from '{}': invalid stem",
                base.display()
            ),
        )
    })?;
    let ext = base
        .extension()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("puml");
    let parent = base.parent().unwrap_or_else(|| Path::new("."));
    Ok((1..=count)
        .map(|idx| parent.join(format!("{stem}-{idx}.{ext}")))
        .collect())
}
