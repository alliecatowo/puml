mod cli;

use clap::Parser;
use cli::{Cli, OutputFormat};
use serde::Serialize;
use std::fs;
use std::io::{self, Read};
use std::path::Path;
use std::process::ExitCode;

const EXIT_OK: u8 = 0;
const EXIT_CLI: u8 = 2;
const EXIT_IO: u8 = 3;
const EXIT_INPUT: u8 = 4;
const EXIT_CHECK_FAILED: u8 = 5;

#[derive(Debug, Serialize)]
struct DiagramRecord {
    index: usize,
    bytes: usize,
    lines: usize,
    has_start_enduml: bool,
    source: String,
    content: String,
}

fn main() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(err) => {
            let code = if err.use_stderr() { EXIT_CLI } else { EXIT_OK };
            let _ = err.print();
            return ExitCode::from(code);
        }
    };

    match run(cli) {
        Ok(code) => ExitCode::from(code),
        Err((code, msg)) => {
            eprintln!("{msg}");
            ExitCode::from(code)
        }
    }
}

fn run(cli: Cli) -> Result<u8, (u8, String)> {
    let (source_name, raw) = read_input(cli.input.as_deref())?;
    let diagrams = split_diagrams(&raw);

    if diagrams.is_empty() {
        return Err((EXIT_INPUT, "no diagram content provided".to_string()));
    }

    if !cli.multi && diagrams.len() > 1 {
        return Err((
            EXIT_INPUT,
            "multiple diagrams detected; rerun with --multi".to_string(),
        ));
    }

    let records: Vec<DiagramRecord> = diagrams
        .iter()
        .enumerate()
        .map(|(idx, content)| DiagramRecord {
            index: idx,
            bytes: content.len(),
            lines: content.lines().count().max(1),
            has_start_enduml: content.contains("@startuml") && content.contains("@enduml"),
            source: source_name.clone(),
            content: content.to_string(),
        })
        .collect();

    if cli.check {
        let failures: Vec<_> = records
            .iter()
            .filter(|r| !is_valid_diagram(&r.content))
            .map(|r| r.index)
            .collect();
        if failures.is_empty() {
            println!("ok: {} diagram(s) passed validation", records.len());
            return Ok(EXIT_OK);
        }
        return Err((
            EXIT_CHECK_FAILED,
            format!("validation failed for diagram indexes: {failures:?}"),
        ));
    }

    if cli.dump || cli.multi {
        let payload = serde_json::to_string_pretty(&records)
            .map_err(|e| (EXIT_INPUT, format!("failed to serialize dump output: {e}")))?;
        println!("{payload}");
        return Ok(EXIT_OK);
    }

    let first = &records[0];
    match cli.format {
        OutputFormat::Text => println!("{}", first.content),
        OutputFormat::Json => {
            let payload = serde_json::to_string_pretty(first)
                .map_err(|e| (EXIT_INPUT, format!("failed to serialize json output: {e}")))?;
            println!("{payload}");
        }
    }

    Ok(EXIT_OK)
}

fn read_input(path: Option<&Path>) -> Result<(String, String), (u8, String)> {
    match path {
        Some(p) if p != Path::new("-") => {
            let raw = fs::read_to_string(p)
                .map_err(|e| (EXIT_IO, format!("failed to read '{}': {e}", p.display())))?;
            Ok((p.display().to_string(), raw))
        }
        _ => {
            let mut raw = String::new();
            io::stdin()
                .read_to_string(&mut raw)
                .map_err(|e| (EXIT_IO, format!("failed to read stdin: {e}")))?;
            Ok(("stdin".to_string(), raw))
        }
    }
}

fn split_diagrams(raw: &str) -> Vec<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    let mut blocks = Vec::new();

    if trimmed.contains("@startuml") {
        let mut current = Vec::new();
        let mut in_block = false;
        for line in raw.lines() {
            if line.contains("@startuml") {
                in_block = true;
                current.clear();
            }
            if in_block {
                current.push(line);
            }
            if in_block && line.contains("@enduml") {
                blocks.push(current.join("\n").trim().to_string());
                current.clear();
                in_block = false;
            }
        }
        if !blocks.is_empty() {
            return blocks;
        }
    }

    raw.split("\n---\n")
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn is_valid_diagram(diagram: &str) -> bool {
    let has_content = !diagram.trim().is_empty();
    let has_start_end = diagram.contains("@startuml") && diagram.contains("@enduml");
    let has_relation = diagram.contains("->") || diagram.contains("-->");
    has_content && (has_start_end || has_relation)
}
