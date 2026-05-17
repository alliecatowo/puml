fn strip_inline_plantuml_comment(line: &str) -> &str {
    let mut in_quotes = false;
    for (idx, ch) in line.char_indices() {
        if ch == '"' {
            in_quotes = !in_quotes;
            continue;
        }
        if ch == '\'' && !in_quotes {
            return &line[..idx];
        }
    }
    line
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BlockKind {
    Uml,
    Salt,
    MindMap,
    Wbs,
    Gantt,
    Chronology,
    Json,
    Yaml,
    Nwdiag,
    Archimate,
    Regex,
    Ebnf,
    Math,
    Sdl,
    Ditaa,
    Chart,
}

fn parse_start_block_kind(line: &str) -> Option<BlockKind> {
    parse_block_marker_kind(line, true)
}

fn parse_end_block_kind(line: &str) -> Option<BlockKind> {
    parse_block_marker_kind(line, false)
}

fn parse_block_marker_kind(line: &str, start: bool) -> Option<BlockKind> {
    let lower = line.to_ascii_lowercase();
    // NOTE: longer markers must come before shorter prefixes that they share.
    let markers: &[(&str, BlockKind)] = if start {
        &[
            ("@startmindmap", BlockKind::MindMap),
            ("@startchronology", BlockKind::Chronology),
            ("@startjson", BlockKind::Json),
            ("@startyaml", BlockKind::Yaml),
            ("@startnwdiag", BlockKind::Nwdiag),
            ("@startarchimate", BlockKind::Archimate),
            ("@startregex", BlockKind::Regex),
            ("@startebnf", BlockKind::Ebnf),
            ("@startlatex", BlockKind::Math),
            ("@startmath", BlockKind::Math),
            ("@startditaa", BlockKind::Ditaa),
            ("@startchart", BlockKind::Chart),
            ("@startsdl", BlockKind::Sdl),
            ("@startgantt", BlockKind::Gantt),
            ("@startwbs", BlockKind::Wbs),
            ("@startsalt", BlockKind::Salt),
            ("@startuml", BlockKind::Uml),
        ]
    } else {
        &[
            ("@endmindmap", BlockKind::MindMap),
            ("@endchronology", BlockKind::Chronology),
            ("@endjson", BlockKind::Json),
            ("@endyaml", BlockKind::Yaml),
            ("@endnwdiag", BlockKind::Nwdiag),
            ("@endarchimate", BlockKind::Archimate),
            ("@endregex", BlockKind::Regex),
            ("@endebnf", BlockKind::Ebnf),
            ("@endlatex", BlockKind::Math),
            ("@endmath", BlockKind::Math),
            ("@endditaa", BlockKind::Ditaa),
            ("@endchart", BlockKind::Chart),
            ("@endsdl", BlockKind::Sdl),
            ("@endgantt", BlockKind::Gantt),
            ("@endwbs", BlockKind::Wbs),
            ("@endsalt", BlockKind::Salt),
            ("@enduml", BlockKind::Uml),
        ]
    };
    for (marker, kind) in markers {
        if lower.starts_with(marker) {
            let rest = &line[marker.len()..];
            if rest.is_empty() || rest.starts_with(char::is_whitespace) {
                return Some(*kind);
            }
        }
    }
    None
}

fn start_block_family(kind: BlockKind) -> Option<DiagramKind> {
    match kind {
        BlockKind::Uml => None,
        BlockKind::Salt => Some(DiagramKind::Salt),
        BlockKind::MindMap => Some(DiagramKind::MindMap),
        BlockKind::Wbs => Some(DiagramKind::Wbs),
        BlockKind::Gantt => Some(DiagramKind::Gantt),
        BlockKind::Chronology => Some(DiagramKind::Chronology),
        BlockKind::Json => Some(DiagramKind::Json),
        BlockKind::Yaml => Some(DiagramKind::Yaml),
        BlockKind::Nwdiag => Some(DiagramKind::Nwdiag),
        BlockKind::Archimate => Some(DiagramKind::Archimate),
        BlockKind::Regex => Some(DiagramKind::Regex),
        BlockKind::Ebnf => Some(DiagramKind::Ebnf),
        BlockKind::Math => Some(DiagramKind::Math),
        BlockKind::Sdl => Some(DiagramKind::Sdl),
        BlockKind::Ditaa => Some(DiagramKind::Ditaa),
        BlockKind::Chart => Some(DiagramKind::Chart),
    }
}

fn block_kind_name(kind: BlockKind) -> &'static str {
    match kind {
        BlockKind::Uml => "uml",
        BlockKind::Salt => "salt",
        BlockKind::MindMap => "mindmap",
        BlockKind::Wbs => "wbs",
        BlockKind::Gantt => "gantt",
        BlockKind::Chronology => "chronology",
        BlockKind::Json => "json",
        BlockKind::Yaml => "yaml",
        BlockKind::Nwdiag => "nwdiag",
        BlockKind::Archimate => "archimate",
        BlockKind::Regex => "regex",
        BlockKind::Ebnf => "ebnf",
        BlockKind::Math => "math",
        BlockKind::Sdl => "sdl",
        BlockKind::Ditaa => "ditaa",
        BlockKind::Chart => "chart",
    }
}

fn is_raw_body_block(kind: BlockKind) -> bool {
    matches!(
        kind,
        BlockKind::Json | BlockKind::Yaml | BlockKind::Nwdiag | BlockKind::Archimate
    )
}

fn block_kind_is_raw_body(kind: BlockKind) -> bool {
    matches!(
        kind,
        BlockKind::Regex
            | BlockKind::Ebnf
            | BlockKind::Math
            | BlockKind::Sdl
            | BlockKind::Ditaa
            | BlockKind::Chart
    )
}

fn select_diagram_kind(
    current: Option<DiagramKind>,
    candidate: DiagramKind,
    span: Span,
) -> Result<DiagramKind, Diagnostic> {
    let Some(current) = current else {
        return Ok(candidate);
    };
    if current == candidate {
        return Ok(current);
    }
    if current == DiagramKind::Unknown || candidate == DiagramKind::Unknown {
        return Ok(DiagramKind::Unknown);
    }
    Err(Diagnostic::error(format!(
        "[E_FAMILY_MIXED] mixed diagram families are not supported: found `{}` syntax in `{}` diagram",
        diagram_kind_name(candidate),
        diagram_kind_name(current)
    ))
    .with_span(span))
}

fn looks_like_unsupported_family_syntax(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower.starts_with("state ")
        || lower.starts_with("component ")
        || lower.starts_with("activity ")
        || lower.starts_with("deployment ")
        || lower.starts_with('*')
        || lower.starts_with("mindmap")
        || lower.starts_with("wbs")
        || lower.starts_with("node ")
        || lower.starts_with("clock ")
        || lower.starts_with("binary ")
        || lower.starts_with("robust ")
        || lower.starts_with("concise ")
}

fn diagram_kind_name(kind: DiagramKind) -> &'static str {
    match kind {
        DiagramKind::Sequence => "sequence",
        DiagramKind::Class => "class",
        DiagramKind::Object => "object",
        DiagramKind::UseCase => "usecase",
        DiagramKind::Salt => "salt",
        DiagramKind::MindMap => "mindmap",
        DiagramKind::Wbs => "wbs",
        DiagramKind::Gantt => "gantt",
        DiagramKind::Chronology => "chronology",
        DiagramKind::Component => "component",
        DiagramKind::Deployment => "deployment",
        DiagramKind::State => "state",
        DiagramKind::Activity => "activity",
        DiagramKind::Timing => "timing",
        DiagramKind::Json => "json",
        DiagramKind::Yaml => "yaml",
        DiagramKind::Nwdiag => "nwdiag",
        DiagramKind::Archimate => "archimate",
        DiagramKind::Regex => "regex",
        DiagramKind::Ebnf => "ebnf",
        DiagramKind::Math => "math",
        DiagramKind::Sdl => "sdl",
        DiagramKind::Ditaa => "ditaa",
        DiagramKind::Chart => "chart",
        DiagramKind::Unknown => "unknown",
    }
}

fn family_for_declaration(kind: &StatementKind) -> DiagramKind {
    match kind {
        StatementKind::ClassDecl(_) => DiagramKind::Class,
        StatementKind::ObjectDecl(_) => DiagramKind::Object,
        StatementKind::UseCaseDecl(_) => DiagramKind::UseCase,
        _ => DiagramKind::Unknown,
    }
}
