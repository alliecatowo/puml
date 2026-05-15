mod cli;

use clap::Parser;
use cli::{
    Cli, CompatMode as CliCompatMode, DeterminismMode as CliDeterminismMode, DiagnosticsFormat,
    Dialect as CliDialect, DumpKind,
};
use puml::ast::{
    DiagramKind, Document, Group, Message, Note, ParticipantDecl,
    ParticipantRole as AstParticipantRole, Statement, StatementKind,
};
use puml::layout;
use puml::model::{
    Participant, ParticipantRole as ModelParticipantRole, SequenceDocument, SequenceEvent,
    SequenceEventKind, VirtualEndpoint, VirtualEndpointKind, VirtualEndpointSide,
};
use puml::scene::LayoutOptions;
use puml::source::Span;
use puml::{
    extract_markdown_diagrams, normalize, render, CompatMode, DeterminismMode, Diagnostic,
    DiagnosticJson, DiagramInput, FrontendSelection, ParsePipelineOptions,
};
use serde::Serialize;
use serde_json::{json, Value};
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const EXIT_OK: u8 = 0;
const EXIT_VALIDATION: u8 = 1;
const EXIT_IO: u8 = 2;
const EXIT_INTERNAL: u8 = 3;

#[derive(Debug, Serialize)]
struct MultiSvgOut {
    name: String,
    svg: String,
}

#[derive(Debug, Serialize)]
struct SceneDump {
    size: SceneSize,
    lanes: Vec<SceneLane>,
    rows: Vec<SceneRow>,
}

#[derive(Debug, Serialize)]
struct SceneSize {
    width: i32,
    height: i32,
}

#[derive(Debug, Serialize)]
struct SceneLane {
    id: String,
    display: String,
    role: String,
    x: i32,
}

#[derive(Debug, Serialize)]
struct SceneRow {
    y: i32,
    event: Value,
}

#[derive(Debug, Clone)]
struct InputDiagram {
    source: String,
    source_span: Option<Span>,
    frontend_hint: Option<FrontendSelection>,
    output_name_hint: Option<String>,
}

#[derive(Debug, Clone)]
struct RenderedOutput {
    name_hint: Option<String>,
    svg: String,
}

#[derive(Debug, Serialize)]
struct DiagnosticsPayload {
    diagnostics: Vec<DiagnosticJson>,
}

fn main() -> ExitCode {
    let cli = match Cli::try_parse() {
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

    match run(cli) {
        Ok(()) => ExitCode::from(EXIT_OK),
        Err((code, msg)) => {
            eprintln!("{msg}");
            ExitCode::from(code)
        }
    }
}

fn run(cli: Cli) -> Result<(), (u8, String)> {
    if cli.dump_capabilities {
        println!(
            "{}",
            serde_json::to_string_pretty(&lsp_capabilities_manifest()).map_err(|e| (
                EXIT_INTERNAL,
                format!("failed to serialize capability manifest: {e}")
            ))?
        );
        return Ok(());
    }

    if let Some(path) = &cli.check_fixture {
        let src = fs::read_to_string(path).map_err(|e| {
            (
                EXIT_IO,
                format!("failed to read fixture '{}': {e}", path.display()),
            )
        })?;
        let include_root = path.parent().map(|p| p.to_path_buf());
        let doc = parse_for_cli(
            &src,
            include_root,
            cli.dialect,
            cli.compat,
            cli.determinism,
            None,
        )
        .map_err(|d| diag_err_with_source(&src, d, cli.diagnostics))?;
        let model = normalize(doc).map_err(|d| diag_err_with_source(&src, d, cli.diagnostics))?;
        emit_warnings(&model, &src, None, cli.diagnostics);
        return Ok(());
    }

    let (_input_name, raw, input_path) = read_input(cli.input.as_deref())?;
    let include_root = cli
        .include_root
        .clone()
        .or_else(|| input_path.and_then(|p| p.parent().map(|d| d.to_path_buf())));
    let from_markdown = should_extract_markdown(cli.from_markdown, input_path);
    let markdown_name_prefix = input_path
        .and_then(|path| path.file_stem())
        .and_then(|stem| stem.to_str())
        .map(|stem| stem.to_string());
    let diagrams = split_diagrams(&raw, from_markdown, markdown_name_prefix.as_deref())
        .map_err(|d| diag_err_with_source(&raw, d, cli.diagnostics))?;

    if diagrams.is_empty() {
        return Err((EXIT_VALIDATION, "no diagram content provided".to_string()));
    }

    let should_require_multi = input_path.is_none();
    if diagrams.len() > 1 && should_require_multi && !cli.multi {
        return Err((
            EXIT_VALIDATION,
            "multiple diagrams detected from stdin input; rerun with --multi".to_string(),
        ));
    }

    if cli.check {
        for source in &diagrams {
            let doc = parse_for_cli(
                &source.source,
                include_root.clone(),
                cli.dialect,
                cli.compat,
                cli.determinism,
                source.frontend_hint,
            )
            .map_err(|d| diag_err_mapped(&raw, source.source_span, d, cli.diagnostics))?;
            let model = normalize(doc)
                .map_err(|d| diag_err_mapped(&raw, source.source_span, d, cli.diagnostics))?;
            emit_warnings(&model, &raw, source.source_span, cli.diagnostics);
        }
        return Ok(());
    }

    if let Some(dump_kind) = cli.dump {
        let values = diagrams
            .iter()
            .map(|source| match dump_kind {
                DumpKind::Ast => {
                    let doc = parse_for_cli(
                        &source.source,
                        include_root.clone(),
                        cli.dialect,
                        cli.compat,
                        cli.determinism,
                        source.frontend_hint,
                    )
                    .map_err(|d| diag_err_mapped(&raw, source.source_span, d, cli.diagnostics))?;
                    Ok(ast_to_json(&doc))
                }
                DumpKind::Model => {
                    let doc = parse_for_cli(
                        &source.source,
                        include_root.clone(),
                        cli.dialect,
                        cli.compat,
                        cli.determinism,
                        source.frontend_hint,
                    )
                    .map_err(|d| diag_err_mapped(&raw, source.source_span, d, cli.diagnostics))?;
                    let model = normalize(doc).map_err(|d| {
                        diag_err_mapped(&raw, source.source_span, d, cli.diagnostics)
                    })?;
                    emit_warnings(&model, &raw, source.source_span, cli.diagnostics);
                    Ok(model_to_json(&model))
                }
                DumpKind::Scene => {
                    let doc = parse_for_cli(
                        &source.source,
                        include_root.clone(),
                        cli.dialect,
                        cli.compat,
                        cli.determinism,
                        source.frontend_hint,
                    )
                    .map_err(|d| diag_err_mapped(&raw, source.source_span, d, cli.diagnostics))?;
                    let model = normalize(doc).map_err(|d| {
                        diag_err_mapped(&raw, source.source_span, d, cli.diagnostics)
                    })?;
                    emit_warnings(&model, &raw, source.source_span, cli.diagnostics);
                    Ok(scene_to_json(&model))
                }
            })
            .collect::<Result<Vec<_>, _>>()?;

        if values.len() == 1 {
            println!(
                "{}",
                serde_json::to_string_pretty(&values[0]).map_err(|e| (
                    EXIT_INTERNAL,
                    format!("failed to serialize dump output: {e}")
                ))?
            );
        } else {
            println!(
                "{}",
                serde_json::to_string_pretty(&values).map_err(|e| (
                    EXIT_INTERNAL,
                    format!("failed to serialize dump output: {e}")
                ))?
            );
        }
        return Ok(());
    }

    let outputs = diagrams.iter().try_fold(Vec::new(), |mut all, source| {
        let doc = parse_for_cli(
            &source.source,
            include_root.clone(),
            cli.dialect,
            cli.compat,
            cli.determinism,
            source.frontend_hint,
        )
        .map_err(|d| diag_err_mapped(&raw, source.source_span, d, cli.diagnostics))?;
        let model = normalize(doc)
            .map_err(|d| diag_err_mapped(&raw, source.source_span, d, cli.diagnostics))?;
        emit_warnings(&model, &raw, source.source_span, cli.diagnostics);
        let scenes = layout::layout_pages(&model, LayoutOptions::default());
        let pages = scenes.iter().map(render::render_svg).collect::<Vec<_>>();
        let page_count = pages.len();
        for (page_idx, svg) in pages.into_iter().enumerate() {
            let name_hint = source.output_name_hint.as_ref().map(|base| {
                if page_count == 1 {
                    format!("{base}.svg")
                } else {
                    format!("{base}-{}.svg", page_idx + 1)
                }
            });
            all.push(RenderedOutput { name_hint, svg });
        }
        Ok::<_, (u8, String)>(all)
    })?;

    if input_path.is_none() && outputs.len() > 1 && !cli.multi {
        return Err((
            EXIT_VALIDATION,
            "multiple pages detected from stdin input; rerun with --multi".to_string(),
        ));
    }

    if input_path.is_none() && outputs.len() > 1 {
        let payload = outputs
            .iter()
            .enumerate()
            .map(|(idx, out)| MultiSvgOut {
                name: out
                    .name_hint
                    .clone()
                    .unwrap_or_else(|| format!("diagram-{}.svg", idx + 1)),
                svg: out.svg.clone(),
            })
            .collect::<Vec<_>>();

        let json = serde_json::to_string_pretty(&payload).map_err(|e| {
            (
                EXIT_INTERNAL,
                format!("failed to serialize multi output: {e}"),
            )
        })?;
        println!("{json}");
        return Ok(());
    }

    if let Some(path) = cli.output {
        let svgs = outputs
            .iter()
            .map(|out| out.svg.clone())
            .collect::<Vec<_>>();
        write_output_files(&path, &svgs)?;
        return Ok(());
    }

    if let Some(input) = input_path {
        if from_markdown {
            write_markdown_output_files(input, &outputs)?;
        } else {
            let default_base = default_output_base(input)?;
            let svgs = outputs
                .iter()
                .map(|out| out.svg.clone())
                .collect::<Vec<_>>();
            write_output_files(&default_base, &svgs)?;
        }
        return Ok(());
    }

    if outputs.len() == 1 {
        println!("{}", outputs[0].svg);
        return Ok(());
    }

    Err((EXIT_INTERNAL, "unexpected stdin output mode".to_string()))
}

fn lsp_capabilities_manifest() -> Value {
    json!({
      "server": "puml-lsp",
      "protocol": "3.17",
      "languageId": "puml",
      "extensions": [".puml", ".plantuml", ".iuml", ".pu"],
      "lifecycle": ["initialize", "initialized", "shutdown", "exit"],
      "textSync": ["didOpen", "didChange", "didSave", "didClose"],
      "languageFeatures": [
        "diagnostics", "completion", "hover", "definition", "references", "rename",
        "documentSymbols", "workspaceSymbols", "semanticTokens", "formatting", "codeActions",
        "foldingRanges", "selectionRanges", "documentLinks", "documentColor"
      ],
      "customRequests": [
        "puml.applyFormat", "puml.renderSvg"
      ]
    })
}

fn diag_err_with_source(source: &str, d: Diagnostic, fmt: DiagnosticsFormat) -> (u8, String) {
    match fmt {
        DiagnosticsFormat::Human => (EXIT_VALIDATION, d.render_with_source(source)),
        DiagnosticsFormat::Json => (EXIT_VALIDATION, diagnostics_json_payload(vec![d], source)),
    }
}

fn diag_err_mapped(
    raw_source: &str,
    mapping: Option<Span>,
    d: Diagnostic,
    fmt: DiagnosticsFormat,
) -> (u8, String) {
    let mapped = map_diagnostic_span(d, mapping);
    diag_err_with_source(raw_source, mapped, fmt)
}

fn emit_warnings(
    model: &SequenceDocument,
    source: &str,
    mapping: Option<Span>,
    fmt: DiagnosticsFormat,
) {
    for warning in &model.warnings {
        let warning = map_diagnostic_span(warning.clone(), mapping);
        match fmt {
            DiagnosticsFormat::Human => eprintln!("{}", warning.render_with_source(source)),
            DiagnosticsFormat::Json => {
                eprintln!("{}", diagnostics_json_payload(vec![warning], source));
            }
        }
    }
}

fn parse_for_cli(
    source: &str,
    include_root: Option<PathBuf>,
    cli_dialect: CliDialect,
    cli_compat: CliCompatMode,
    cli_determinism: CliDeterminismMode,
    frontend_hint: Option<FrontendSelection>,
) -> Result<Document, Diagnostic> {
    let options = ParsePipelineOptions {
        frontend: map_frontend(cli_dialect, frontend_hint),
        compat: map_compat(cli_compat),
        determinism: map_determinism(cli_determinism),
        include_root,
    };
    puml::parse_with_pipeline_options(source, &options)
}

fn map_frontend(
    dialect: CliDialect,
    frontend_hint: Option<FrontendSelection>,
) -> FrontendSelection {
    if matches!(dialect, CliDialect::Auto) {
        if let Some(hint) = frontend_hint {
            return hint;
        }
    }

    match dialect {
        CliDialect::Auto => FrontendSelection::Auto,
        CliDialect::Plantuml => FrontendSelection::Plantuml,
        CliDialect::Mermaid => FrontendSelection::Mermaid,
        CliDialect::Picouml => FrontendSelection::Picouml,
    }
}

fn map_compat(mode: CliCompatMode) -> CompatMode {
    match mode {
        CliCompatMode::Strict => CompatMode::Strict,
        CliCompatMode::Extended => CompatMode::Extended,
    }
}

fn map_determinism(mode: CliDeterminismMode) -> DeterminismMode {
    match mode {
        CliDeterminismMode::Strict => DeterminismMode::Strict,
        CliDeterminismMode::Full => DeterminismMode::Full,
    }
}

fn read_input(path: Option<&Path>) -> Result<(String, String, Option<&Path>), (u8, String)> {
    match path {
        Some(p) if p != Path::new("-") => {
            let raw = fs::read_to_string(p)
                .map_err(|e| (EXIT_IO, format!("failed to read '{}': {e}", p.display())))?;
            Ok((p.display().to_string(), raw, Some(p)))
        }
        _ => {
            let mut raw = String::new();
            io::stdin()
                .read_to_string(&mut raw)
                .map_err(|e| (EXIT_IO, format!("failed to read stdin: {e}")))?;
            Ok(("stdin".to_string(), raw, None))
        }
    }
}

fn should_extract_markdown(from_markdown_flag: bool, input_path: Option<&Path>) -> bool {
    if from_markdown_flag {
        return true;
    }

    input_path
        .and_then(|path| path.extension())
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            matches!(
                ext.to_ascii_lowercase().as_str(),
                "md" | "markdown" | "mdown"
            )
        })
        .unwrap_or(false)
}

fn split_diagrams(
    raw: &str,
    from_markdown: bool,
    markdown_name_prefix: Option<&str>,
) -> Result<Vec<InputDiagram>, Diagnostic> {
    if from_markdown {
        let diagrams = extract_markdown_diagrams(raw)
            .into_iter()
            .enumerate()
            .map(
                |(
                    idx,
                    DiagramInput {
                        source,
                        span_in_input,
                        fence_frontend,
                    },
                )| InputDiagram {
                    source,
                    source_span: Some(span_in_input),
                    frontend_hint: Some(fence_frontend),
                    output_name_hint: Some(match markdown_name_prefix {
                        Some(prefix) => format!("{prefix}_snippet_{}", idx + 1),
                        None => format!("snippet-{}", idx + 1),
                    }),
                },
            )
            .collect::<Vec<_>>();
        return Ok(diagrams);
    }

    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    let mut blocks = Vec::new();

    if trimmed.to_ascii_lowercase().contains("@startuml") {
        let mut current = Vec::new();
        let mut in_block = false;
        let mut block_start_line = 0usize;
        for (line_idx, line) in raw.lines().enumerate() {
            let marker = line.trim();
            if marker.eq_ignore_ascii_case("@startuml") {
                if in_block {
                    return Err(Diagnostic::error(format!(
                        "unmatched @startuml/@enduml boundary: found @startuml at line {} before closing previous block started at line {}",
                        line_idx + 1,
                        block_start_line
                    )));
                }
                in_block = true;
                block_start_line = line_idx + 1;
                current.clear();
            }
            if marker.eq_ignore_ascii_case("@enduml") && !in_block {
                return Err(Diagnostic::error(format!(
                    "unmatched @startuml/@enduml boundary: found @enduml at line {} without a preceding @startuml",
                    line_idx + 1
                )));
            }
            if in_block {
                current.push(line);
            }
            if in_block && marker.eq_ignore_ascii_case("@enduml") {
                blocks.push(InputDiagram {
                    source: current.join("\n").trim().to_string(),
                    source_span: None,
                    frontend_hint: None,
                    output_name_hint: None,
                });
                current.clear();
                in_block = false;
            }
        }
        if in_block {
            return Err(Diagnostic::error(format!(
                "unmatched @startuml/@enduml boundary: @startuml at line {} is missing a closing @enduml",
                block_start_line
            )));
        }
        if !blocks.is_empty() {
            return Ok(blocks);
        }
    }

    Ok(vec![InputDiagram {
        source: trimmed.to_string(),
        source_span: None,
        frontend_hint: None,
        output_name_hint: None,
    }])
}

fn map_diagnostic_span(mut d: Diagnostic, mapping: Option<Span>) -> Diagnostic {
    if let (Some(span), Some(base)) = (d.span, mapping) {
        d.span = Some(Span::new(base.start + span.start, base.start + span.end));
    }
    d
}

fn diagnostics_json_payload(diags: Vec<Diagnostic>, source: &str) -> String {
    let payload = DiagnosticsPayload {
        diagnostics: diags
            .iter()
            .map(|d| d.to_json_with_source(source))
            .collect::<Vec<_>>(),
    };
    serde_json::to_string_pretty(&payload).unwrap_or_else(|_| {
        "{\"diagnostics\":[{\"severity\":\"error\",\"message\":\"failed to serialize diagnostics\",\"span\":null,\"line\":null,\"column\":null,\"snippet\":null,\"caret\":null}]}".to_string()
    })
}

fn default_output_base(input: &Path) -> Result<PathBuf, (u8, String)> {
    let stem = input.file_stem().and_then(|s| s.to_str()).ok_or_else(|| {
        (
            EXIT_IO,
            format!(
                "cannot derive output name from '{}': invalid stem",
                input.display()
            ),
        )
    })?;
    Ok(input.with_file_name(format!("{stem}.svg")))
}

fn write_markdown_output_files(
    input: &Path,
    outputs: &[RenderedOutput],
) -> Result<(), (u8, String)> {
    let parent = input.parent().unwrap_or_else(|| Path::new("."));
    for (idx, out) in outputs.iter().enumerate() {
        let name = out.name_hint.as_ref().ok_or_else(|| {
            (
                EXIT_INTERNAL,
                format!("missing markdown output name for diagram {}", idx + 1),
            )
        })?;
        let path = parent.join(name);
        fs::write(&path, &out.svg).map_err(|e| {
            (
                EXIT_IO,
                format!("failed to write '{}': {e}", path.display()),
            )
        })?;
    }
    Ok(())
}

fn write_output_files(base: &Path, svgs: &[String]) -> Result<(), (u8, String)> {
    if svgs.len() == 1 {
        fs::write(base, &svgs[0]).map_err(|e| {
            (
                EXIT_IO,
                format!("failed to write '{}': {e}", base.display()),
            )
        })?;
        return Ok(());
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

    for (idx, svg) in svgs.iter().enumerate() {
        let path = parent.join(format!("{stem}-{}.{}", idx + 1, ext));
        fs::write(&path, svg).map_err(|e| {
            (
                EXIT_IO,
                format!("failed to write '{}': {e}", path.display()),
            )
        })?;
    }

    Ok(())
}

fn ast_to_json(doc: &Document) -> Value {
    json!({
        "kind": match doc.kind { DiagramKind::Sequence => "Sequence", DiagramKind::Unknown => "Unknown" },
        "statements": doc.statements.iter().map(statement_to_json).collect::<Vec<_>>()
    })
}

fn statement_to_json(s: &Statement) -> Value {
    json!({
        "span": {"start": s.span.start, "end": s.span.end},
        "kind": statement_kind_to_json(&s.kind)
    })
}

fn statement_kind_to_json(kind: &StatementKind) -> Value {
    match kind {
        StatementKind::Participant(p) => json!({"Participant": participant_decl_to_json(p)}),
        StatementKind::Message(m) => json!({"Message": message_to_json(m)}),
        StatementKind::Note(n) => json!({"Note": note_to_json(n)}),
        StatementKind::Group(g) => json!({"Group": group_to_json(g)}),
        StatementKind::Title(v) => json!({"Title": v}),
        StatementKind::Header(v) => json!({"Header": v}),
        StatementKind::Footer(v) => json!({"Footer": v}),
        StatementKind::Caption(v) => json!({"Caption": v}),
        StatementKind::Legend(v) => json!({"Legend": v}),
        StatementKind::Theme(v) => json!({"Theme": v}),
        StatementKind::SkinParam { key, value } => {
            json!({"SkinParam": {"key": key, "value": value}})
        }
        StatementKind::Footbox(v) => json!({"Footbox": v}),
        StatementKind::Delay(v) => json!({"Delay": v}),
        StatementKind::Divider(v) => json!({"Divider": v}),
        StatementKind::Separator(v) => json!({"Separator": v}),
        StatementKind::Spacer => json!("Spacer"),
        StatementKind::NewPage(v) => json!({"NewPage": v}),
        StatementKind::IgnoreNewPage => json!("IgnoreNewPage"),
        StatementKind::Autonumber(v) => json!({"Autonumber": v}),
        StatementKind::Activate(v) => json!({"Activate": v}),
        StatementKind::Deactivate(v) => json!({"Deactivate": v}),
        StatementKind::Destroy(v) => json!({"Destroy": v}),
        StatementKind::Create(v) => json!({"Create": v}),
        StatementKind::Return(v) => json!({"Return": v}),
        StatementKind::Include(v) => json!({"Include": v}),
        StatementKind::Define { name, value } => json!({"Define": {"name": name, "value": value}}),
        StatementKind::Undef(v) => json!({"Undef": v}),
        StatementKind::Unknown(v) => json!({"Unknown": v}),
    }
}

fn participant_decl_to_json(p: &ParticipantDecl) -> Value {
    json!({
        "role": ast_role_to_str(p.role),
        "name": p.name,
        "alias": p.alias,
        "display": p.display
    })
}

fn message_to_json(m: &Message) -> Value {
    let mut message = json!({"from": m.from, "to": m.to, "arrow": m.arrow, "label": m.label});
    if let Some(ep) = m.from_virtual {
        message["from_virtual"] = json!({
            "side": match ep.side {
                puml::ast::VirtualEndpointSide::Left => "left",
                puml::ast::VirtualEndpointSide::Right => "right",
            },
            "kind": match ep.kind {
                puml::ast::VirtualEndpointKind::Plain => "plain",
                puml::ast::VirtualEndpointKind::Circle => "circle",
                puml::ast::VirtualEndpointKind::Cross => "cross",
                puml::ast::VirtualEndpointKind::Filled => "filled",
            }
        });
    }
    if let Some(ep) = m.to_virtual {
        message["to_virtual"] = json!({
            "side": match ep.side {
                puml::ast::VirtualEndpointSide::Left => "left",
                puml::ast::VirtualEndpointSide::Right => "right",
            },
            "kind": match ep.kind {
                puml::ast::VirtualEndpointKind::Plain => "plain",
                puml::ast::VirtualEndpointKind::Circle => "circle",
                puml::ast::VirtualEndpointKind::Cross => "cross",
                puml::ast::VirtualEndpointKind::Filled => "filled",
            }
        });
    }
    message
}

fn note_to_json(n: &Note) -> Value {
    json!({"position": n.position, "target": n.target, "text": n.text})
}

fn group_to_json(g: &Group) -> Value {
    json!({"kind": g.kind, "label": g.label})
}

fn ast_role_to_str(role: AstParticipantRole) -> &'static str {
    match role {
        AstParticipantRole::Participant => "Participant",
        AstParticipantRole::Actor => "Actor",
        AstParticipantRole::Boundary => "Boundary",
        AstParticipantRole::Control => "Control",
        AstParticipantRole::Entity => "Entity",
        AstParticipantRole::Database => "Database",
        AstParticipantRole::Collections => "Collections",
        AstParticipantRole::Queue => "Queue",
    }
}

fn model_to_json(model: &SequenceDocument) -> Value {
    json!({
        "participants": model.participants.iter().map(model_participant_to_json).collect::<Vec<_>>(),
        "events": model.events.iter().map(model_event_to_json).collect::<Vec<_>>(),
        "title": model.title,
        "header": model.header,
        "footer": model.footer,
        "caption": model.caption,
        "legend": model.legend,
        "skinparams": model.skinparams,
        "footbox_visible": model.footbox_visible
    })
}

fn model_participant_to_json(p: &Participant) -> Value {
    json!({
        "id": p.id,
        "display": p.display,
        "role": model_role_to_str(p.role),
        "explicit": p.explicit
    })
}

fn model_role_to_str(role: ModelParticipantRole) -> &'static str {
    match role {
        ModelParticipantRole::Participant => "Participant",
        ModelParticipantRole::Actor => "Actor",
        ModelParticipantRole::Boundary => "Boundary",
        ModelParticipantRole::Control => "Control",
        ModelParticipantRole::Entity => "Entity",
        ModelParticipantRole::Database => "Database",
        ModelParticipantRole::Collections => "Collections",
        ModelParticipantRole::Queue => "Queue",
    }
}

fn model_event_to_json(e: &SequenceEvent) -> Value {
    json!({
        "span": {"start": e.span.start, "end": e.span.end},
        "kind": model_event_kind_to_json(&e.kind)
    })
}

fn model_event_kind_to_json(kind: &SequenceEventKind) -> Value {
    match kind {
        SequenceEventKind::Message {
            from,
            to,
            arrow,
            label,
            from_virtual,
            to_virtual,
        } => {
            let mut message = json!({"from": from, "to": to, "arrow": arrow, "label": label});
            if let Some(ep) = from_virtual {
                message["from_virtual"] = virtual_endpoint_to_json(*ep);
            }
            if let Some(ep) = to_virtual {
                message["to_virtual"] = virtual_endpoint_to_json(*ep);
            }
            json!({"Message": message})
        }
        SequenceEventKind::Note {
            position,
            target,
            text,
        } => {
            json!({"Note": {"position": position, "target": target, "text": text}})
        }
        SequenceEventKind::GroupStart { kind, label } => {
            json!({"GroupStart": {"kind": kind, "label": label}})
        }
        SequenceEventKind::GroupEnd => json!("GroupEnd"),
        SequenceEventKind::Delay(v) => json!({"Delay": v}),
        SequenceEventKind::Divider(v) => json!({"Divider": v}),
        SequenceEventKind::Separator(v) => json!({"Separator": v}),
        SequenceEventKind::Spacer => json!("Spacer"),
        SequenceEventKind::NewPage(v) => json!({"NewPage": v}),
        SequenceEventKind::Autonumber(v) => json!({"Autonumber": v}),
        SequenceEventKind::Activate(v) => json!({"Activate": v}),
        SequenceEventKind::Deactivate(v) => json!({"Deactivate": v}),
        SequenceEventKind::Destroy(v) => json!({"Destroy": v}),
        SequenceEventKind::Create(v) => json!({"Create": v}),
        SequenceEventKind::Return { label, from, to } => {
            json!({"Return": {"label": label, "from": from, "to": to}})
        }
        SequenceEventKind::IncludePlaceholder(v) => json!({"IncludePlaceholder": v}),
        SequenceEventKind::DefinePlaceholder { name, value } => {
            json!({"DefinePlaceholder": {"name": name, "value": value}})
        }
        SequenceEventKind::UndefPlaceholder(v) => json!({"UndefPlaceholder": v}),
    }
}

fn virtual_endpoint_to_json(ep: VirtualEndpoint) -> Value {
    json!({
        "side": match ep.side {
            VirtualEndpointSide::Left => "left",
            VirtualEndpointSide::Right => "right",
        },
        "kind": match ep.kind {
            VirtualEndpointKind::Plain => "plain",
            VirtualEndpointKind::Circle => "circle",
            VirtualEndpointKind::Cross => "cross",
            VirtualEndpointKind::Filled => "filled",
        }
    })
}

fn scene_to_json(model: &SequenceDocument) -> Value {
    let lane_spacing = 140;
    let lane_start = 100;
    let row_spacing = 40;
    let row_start = 120;
    let width = 200 + (model.participants.len() as i32 * lane_spacing);
    let height = 120 + (model.events.len() as i32 * row_spacing);

    let lanes = model
        .participants
        .iter()
        .enumerate()
        .map(|(idx, p)| SceneLane {
            id: p.id.clone(),
            display: p.display.clone(),
            role: model_role_to_str(p.role).to_string(),
            x: lane_start + (idx as i32 * lane_spacing),
        })
        .collect::<Vec<_>>();

    let rows = model
        .events
        .iter()
        .enumerate()
        .map(|(idx, e)| SceneRow {
            y: row_start + (idx as i32 * row_spacing),
            event: model_event_to_json(e),
        })
        .collect::<Vec<_>>();

    let scene = SceneDump {
        size: SceneSize { width, height },
        lanes,
        rows,
    };
    serde_json::to_value(scene).unwrap_or_else(|_| json!({"error": "scene serialization failed"}))
}
