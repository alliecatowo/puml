use crate::ast::{
    ActivityStep, ActivityStepKind, ClassDecl, ClassMember, ComponentNodeKind, DiagramKind,
    Document, FamilyRelation, Group, MemberModifier, Message, MessageStyle, Note, ObjectDecl,
    ParticipantDecl, ParticipantRole, SaltCell, StateDecl, StateInternalAction, StateTransition,
    Statement, StatementKind, TimingDeclKind, UseCaseDecl, VirtualEndpoint, VirtualEndpointKind,
    VirtualEndpointSide,
};
use crate::diagnostic::Diagnostic;
use crate::preproc::preprocess;
pub use crate::preproc::ParseOptions;
use crate::source::Span;

pub fn parse(source: &str) -> Result<Document, Diagnostic> {
    parse_with_options(source, &ParseOptions::default())
}

pub fn parse_with_options(source: &str, options: &ParseOptions) -> Result<Document, Diagnostic> {
    let expanded = preprocess(source, options)?;
    parse_preprocessed(&expanded)
}

fn parse_preprocessed(source: &str) -> Result<Document, Diagnostic> {
    let mut statements = Vec::new();
    let mut lines = Vec::new();
    let mut offset = 0usize;
    for raw_line in source.lines() {
        let span = Span::new(offset, offset + raw_line.len());
        lines.push((raw_line, span));
        offset += raw_line.len() + 1;
    }

    let mut detected_kind: Option<DiagramKind> = None;
    let mut in_block = false;
    let mut block_kind: Option<BlockKind> = None;
    let mut block_start_span: Option<Span> = None;
    let mut i = 0usize;
    while i < lines.len() {
        let (raw_line, span) = lines[i];
        let line = strip_inline_plantuml_comment(raw_line).trim();
        // In raw-body family blocks we never strip empty lines or interpret comments.
        // Check for the closing marker first; otherwise capture verbatim.
        if let Some(bk) = block_kind {
            if is_raw_body_block(bk) || block_kind_is_raw_body(bk) {
                if let Some(end_kind) = parse_end_block_kind(raw_line.trim()) {
                    if block_kind == Some(end_kind) {
                        in_block = false;
                        block_kind = None;
                        block_start_span = None;
                        i += 1;
                        continue;
                    } else {
                        return Err(Diagnostic::error(format!(
                            "[E_BLOCK_MISMATCH] closing marker `@end{}` does not match opening `@start{}`",
                            block_kind_name(end_kind),
                            block_kind_name(bk)
                        ))
                        .with_span(span));
                    }
                }
                statements.push(Statement {
                    span,
                    kind: StatementKind::RawBody(raw_line.to_string()),
                });
                i += 1;
                continue;
            }
        }

        if line.is_empty()
            || (line.starts_with('"')
                && split_family_arrow(line).is_none()
                && split_arrow(line).is_none())
        {
            i += 1;
            continue;
        }
        if let Some(start_kind) = parse_start_block_kind(line) {
            if in_block {
                return Err(Diagnostic::error(
                    "unmatched @startuml/@enduml boundary: found new @start marker before closing previous block",
                )
                .with_span(span));
            }
            in_block = true;
            block_kind = Some(start_kind);
            block_start_span = Some(span);
            if let Some(candidate) = start_block_family(start_kind) {
                detected_kind = Some(select_diagram_kind(detected_kind, candidate, span)?);
            }
            i += 1;
            continue;
        }
        if let Some(end_kind) = parse_end_block_kind(line) {
            if !in_block {
                return Err(Diagnostic::error(
                    "unmatched @startuml/@enduml boundary: found @end marker without a preceding @startuml",
                )
                .with_span(span));
            }
            if block_kind != Some(end_kind) {
                return Err(Diagnostic::error(format!(
                    "[E_BLOCK_MISMATCH] closing marker `@end{}` does not match opening `@start{}`",
                    block_kind_name(end_kind),
                    block_kind_name(block_kind.unwrap_or(BlockKind::Uml))
                ))
                .with_span(span));
            }
            in_block = false;
            block_kind = None;
            block_start_span = None;
            i += 1;
            continue;
        }

        if let Some(kind) = parse_keyword(line) {
            let multiline_note_head =
                matches!(&kind, StatementKind::Note(_)) && note_block_continues(&lines, i, line);
            let multiline_text_head = matches!(
                &kind,
                StatementKind::Title(_)
                    | StatementKind::Header(_)
                    | StatementKind::Footer(_)
                    | StatementKind::Caption(_)
                    | StatementKind::Legend(_)
            ) && text_block_continues(&lines, i, line);
            if detected_kind.is_some()
                && is_family_common_keyword(&kind)
                && !multiline_note_head
                && !multiline_text_head
            {
                statements.push(Statement { span, kind });
                i += 1;
                continue;
            }
        }

        if detected_kind.is_none() && looks_like_old_activity_flow(line) {
            detected_kind = Some(DiagramKind::Activity);
        }

        if matches!(
            detected_kind,
            None | Some(DiagramKind::Component | DiagramKind::Deployment)
        ) {
            let ambiguous_class_interface = detected_kind.is_none()
                && line.starts_with("interface ")
                && later_lines_contain_class_family_declaration(&lines, i);
            let actor_prefers_non_component = detected_kind.is_none()
                && line.starts_with("actor ")
                && (line.contains("<<")
                    || line.contains('"')
                    || later_lines_contain_usecase_family_declaration(&lines, i));
            let ambiguous_class_scope = detected_kind.is_none()
                && (line.starts_with("package ") || line.starts_with("namespace "))
                && line.trim_end().ends_with('{')
                && {
                    let end_idx = find_scoping_block_end(&lines, i);
                    end_idx > i
                        && (group_body_contains_class_family(&lines, i, end_idx)
                            || group_body_contains_object_family(&lines, i, end_idx)
                            || group_body_contains_usecase_family(&lines, i, end_idx))
                };
            if let Some((kind, end_idx)) = parse_component_scoping_block(&lines, i, line)? {
                let family = if matches!(detected_kind, Some(DiagramKind::Deployment)) {
                    DiagramKind::Deployment
                } else {
                    DiagramKind::Component
                };
                detected_kind = Some(select_diagram_kind(detected_kind, family, span)?);
                statements.push(Statement { span, kind });
                i = end_idx + 1;
                continue;
            }
            if !ambiguous_class_interface && !actor_prefers_non_component && !ambiguous_class_scope
            {
                if let Some(kind) = parse_component_decl(line) {
                    let family = if matches!(detected_kind, Some(DiagramKind::Deployment)) {
                        DiagramKind::Deployment
                    } else {
                        match &kind {
                            StatementKind::ComponentDecl {
                                kind:
                                    ComponentNodeKind::Node
                                    | ComponentNodeKind::Artifact
                                    | ComponentNodeKind::Cloud
                                    | ComponentNodeKind::Frame
                                    | ComponentNodeKind::Storage
                                    | ComponentNodeKind::Database
                                    | ComponentNodeKind::Package
                                    | ComponentNodeKind::Rectangle
                                    | ComponentNodeKind::Folder
                                    | ComponentNodeKind::File
                                    | ComponentNodeKind::Card,
                                ..
                            } => DiagramKind::Deployment,
                            _ => DiagramKind::Component,
                        }
                    };
                    detected_kind = Some(select_diagram_kind(detected_kind, family, span)?);
                    statements.push(Statement { span, kind });
                    i += 1;
                    continue;
                }
            }
        }

        // Inline JSON/YAML projection: `json $alias {` / `yaml $alias {` ... `}`.
        // In a UML block this should not force the document to class-family if
        // a component/deployment/etc. family has already been established.
        if let Some((kind, end_idx)) = parse_json_projection_block(&lines, i, line)? {
            let projection_family = match detected_kind {
                Some(DiagramKind::Component) => DiagramKind::Component,
                Some(DiagramKind::Deployment) => DiagramKind::Deployment,
                Some(DiagramKind::Object) => DiagramKind::Object,
                Some(DiagramKind::UseCase) => DiagramKind::UseCase,
                Some(DiagramKind::Class) | None => DiagramKind::Class,
                Some(other) => other,
            };
            detected_kind = Some(select_diagram_kind(detected_kind, projection_family, span)?);
            let block_span = Span::new(span.start, lines[end_idx].1.end);
            statements.push(Statement {
                span: block_span,
                kind,
            });
            i = end_idx + 1;
            continue;
        }

        if matches!(
            detected_kind,
            None | Some(DiagramKind::Class | DiagramKind::Object | DiagramKind::UseCase)
        ) && !(detected_kind.is_none()
            && in_block
            && block_kind == Some(BlockKind::Uml)
            && ((line.starts_with("interface ")
                && !later_lines_contain_class_family_declaration(&lines, i))
                || (line.starts_with("actor ")
                    && !line.contains("<<")
                    && !later_lines_contain_usecase_family_declaration(&lines, i))))
        {
            if let Some((kind, end_idx)) = parse_family_declaration(&lines, i, line)? {
                let family = family_for_declaration(&kind);
                detected_kind = Some(select_diagram_kind(detected_kind, family, span)?);
                let block_span = Span::new(span.start, lines[end_idx].1.end);
                statements.push(Statement {
                    span: block_span,
                    kind,
                });
                i = end_idx + 1;
                continue;
            }
        }

        if let Some(kind) = parse_family_member_row(line, detected_kind) {
            statements.push(Statement { span, kind });
            i += 1;
            continue;
        }

        if let Some(kind) = parse_family_relation(line, detected_kind) {
            statements.push(Statement { span, kind });
            i += 1;
            continue;
        }

        if matches!(
            detected_kind,
            None | Some(DiagramKind::Class | DiagramKind::Object | DiagramKind::UseCase)
        ) {
            if let Some((kind, end_idx)) = parse_class_scoping_block(&lines, i, line)? {
                let scoped_family = scoped_family_kind_for_block(&lines, i, end_idx);
                detected_kind = Some(select_diagram_kind(detected_kind, scoped_family, span)?);
                let block_span = Span::new(span.start, lines[end_idx].1.end);
                statements.push(Statement {
                    span: block_span,
                    kind,
                });
                i = end_idx + 1;
                continue;
            }
        }

        if detected_kind.is_none() && detect_non_sequence_family(line) != Some(DiagramKind::State) {
            if let Some(kind) = parse_message(line) {
                detected_kind = Some(select_diagram_kind(
                    detected_kind,
                    DiagramKind::Sequence,
                    span,
                )?);
                statements.push(Statement { span, kind });
                i += 1;
                continue;
            }
        }

        if detected_kind.is_none()
            && in_block
            && block_kind == Some(BlockKind::Uml)
            && !(line.starts_with("actor ") && line.contains("<<"))
        {
            if let Some(kind) = parse_participant(line) {
                detected_kind = Some(select_diagram_kind(
                    detected_kind,
                    DiagramKind::Sequence,
                    span,
                )?);
                statements.push(Statement { span, kind });
                i += 1;
                continue;
            }
        }

        if detected_kind.is_none() {
            if let Some(kind) = detect_non_sequence_family(line) {
                detected_kind = Some(kind);
            } else if parse_component_decl(line).is_some() {
                detected_kind = Some(DiagramKind::Component);
            } else if looks_like_unsupported_family_syntax(line) {
                detected_kind = Some(DiagramKind::Unknown);
            }
        }

        // Family-specific inline parsing for the newly-implemented families.
        if matches!(
            detected_kind,
            Some(DiagramKind::Component) | Some(DiagramKind::Deployment)
        ) {
            if let Some((kind, end_idx)) = parse_component_scoping_block(&lines, i, line)? {
                statements.push(Statement { span, kind });
                i = end_idx + 1;
                continue;
            }
            if let Some(kind) = parse_component_decl(line) {
                statements.push(Statement { span, kind });
                i += 1;
                continue;
            }
            // Try a relation again now that detection settled.
            if let Some(kind) = parse_family_relation(line, detected_kind) {
                statements.push(Statement { span, kind });
                i += 1;
                continue;
            }
        }

        if matches!(detected_kind, Some(DiagramKind::Activity)) {
            if let Some(kinds) = parse_activity_old_style_flow(line) {
                statements.extend(kinds.into_iter().map(|kind| Statement { span, kind }));
                i += 1;
                continue;
            }
            if let Some(kind) = parse_activity_step(line) {
                statements.push(Statement { span, kind });
                i += 1;
                continue;
            }
        }

        if matches!(detected_kind, Some(DiagramKind::Timing)) {
            if let Some(kind) = parse_timing_decl(line) {
                statements.push(Statement { span, kind });
                i += 1;
                continue;
            }
            if let Some(kind) = parse_timing_event(line) {
                statements.push(Statement { span, kind });
                i += 1;
                continue;
            }
        }

        let allow_sequence_parse =
            detected_kind.is_none() || matches!(detected_kind, Some(DiagramKind::Sequence));
        let allow_gantt_parse = matches!(detected_kind, Some(DiagramKind::Gantt));
        let allow_chronology_parse = matches!(detected_kind, Some(DiagramKind::Chronology));
        let allow_state_parse = matches!(detected_kind, Some(DiagramKind::State));
        // MindMap and WBS also support multiline legend/title/caption/header/footer blocks.
        let allow_family_keyword_block =
            matches!(detected_kind, Some(DiagramKind::MindMap | DiagramKind::Wbs));

        if allow_sequence_parse {
            if let Some((kind, end_idx)) = parse_multiline_keyword_block(&lines, i, line) {
                detected_kind = Some(select_diagram_kind(
                    detected_kind,
                    DiagramKind::Sequence,
                    span,
                )?);
                let block_span = Span::new(span.start, lines[end_idx].1.end);
                statements.push(Statement {
                    span: block_span,
                    kind,
                });
                i = end_idx + 1;
                continue;
            }
        }

        if allow_family_keyword_block {
            if let Some((kind, end_idx)) = parse_multiline_keyword_block(&lines, i, line) {
                let block_span = Span::new(span.start, lines[end_idx].1.end);
                statements.push(Statement {
                    span: block_span,
                    kind,
                });
                i = end_idx + 1;
                continue;
            }
        }

        if allow_gantt_parse {
            if let Some(kind) = parse_gantt_baseline_statement(line) {
                statements.push(Statement { span, kind });
                i += 1;
                continue;
            }
            if let Some((kind, end_idx)) = parse_multiline_keyword_block(&lines, i, line) {
                statements.push(Statement {
                    span: Span::new(span.start, lines[end_idx].1.end),
                    kind,
                });
                i = end_idx + 1;
                continue;
            }
            if let Some(kind) = parse_keyword(line) {
                if is_timeline_metadata_statement(&kind) {
                    statements.push(Statement { span, kind });
                    i += 1;
                    continue;
                }
            }
            statements.push(Statement {
                span,
                kind: StatementKind::Unknown(format!(
                    "[E_GANTT_UNSUPPORTED] unsupported gantt baseline syntax: `{line}`"
                )),
            });
            i += 1;
            continue;
        }

        if allow_chronology_parse {
            if let Some(kind) = parse_chronology_baseline_statement(line) {
                statements.push(Statement { span, kind });
                i += 1;
                continue;
            }
            if let Some((kind, end_idx)) = parse_multiline_keyword_block(&lines, i, line) {
                statements.push(Statement {
                    span: Span::new(span.start, lines[end_idx].1.end),
                    kind,
                });
                i = end_idx + 1;
                continue;
            }
            if let Some(kind) = parse_keyword(line) {
                if is_timeline_metadata_statement(&kind) {
                    statements.push(Statement { span, kind });
                    i += 1;
                    continue;
                }
            }
            statements.push(Statement {
                span,
                kind: StatementKind::Unknown(format!(
                    "[E_CHRONOLOGY_UNSUPPORTED] unsupported chronology baseline syntax: `{line}`"
                )),
            });
            i += 1;
            continue;
        }

        if allow_state_parse {
            if let Some((kind, end_idx)) = parse_state_statement(&lines, i, line)? {
                let block_span = if end_idx > i {
                    Span::new(span.start, lines[end_idx].1.end)
                } else {
                    span
                };
                statements.push(Statement {
                    span: block_span,
                    kind,
                });
                i = end_idx + 1;
                continue;
            }
            // Any non-empty line in a state diagram that wasn't recognised above
            // is stored as Unknown for normalizer to reject gracefully.
            statements.push(Statement {
                span,
                kind: StatementKind::Unknown(line.to_string()),
            });
            i += 1;
            continue;
        }

        if allow_sequence_parse {
            if let Some((kind, end_idx)) = parse_multiline_note_block(&lines, i, line) {
                detected_kind = Some(select_diagram_kind(
                    detected_kind,
                    DiagramKind::Sequence,
                    span,
                )?);
                let block_span = Span::new(span.start, lines[end_idx].1.end);
                statements.push(Statement {
                    span: block_span,
                    kind,
                });
                i = end_idx + 1;
                continue;
            }
        }
        if allow_sequence_parse {
            if let Some((kind, end_idx)) = parse_multiline_ref_block(&lines, i, line) {
                detected_kind = Some(select_diagram_kind(
                    detected_kind,
                    DiagramKind::Sequence,
                    span,
                )?);
                let block_span = Span::new(span.start, lines[end_idx].1.end);
                statements.push(Statement {
                    span: block_span,
                    kind,
                });
                i = end_idx + 1;
                continue;
            }
        }

        if allow_sequence_parse {
            if let Some(kind) = parse_participant(line) {
                detected_kind = Some(select_diagram_kind(
                    detected_kind,
                    DiagramKind::Sequence,
                    span,
                )?);
                statements.push(Statement { span, kind });
                i += 1;
                continue;
            }
        }
        if allow_sequence_parse {
            if let Some(kind) = parse_message(line) {
                detected_kind = Some(select_diagram_kind(
                    detected_kind,
                    DiagramKind::Sequence,
                    span,
                )?);
                statements.push(Statement { span, kind });
                i += 1;
                continue;
            }
        }
        if allow_sequence_parse && looks_like_arrow_syntax(line) {
            return Err(Diagnostic::error(format!(
                "[E_ARROW_INVALID] malformed sequence arrow syntax: `{}`",
                line
            ))
            .with_span(span));
        }

        if let Some(kind) = parse_keyword(line) {
            if is_sequence_keyword(&kind) {
                detected_kind = Some(select_diagram_kind(
                    detected_kind,
                    DiagramKind::Sequence,
                    span,
                )?);
            }
            statements.push(Statement { span, kind });
            i += 1;
            continue;
        }

        // Legacy position kept as a fallback for projection-like lines that
        // did not match before family/member/relation parsing.
        if let Some((kind, end_idx)) = parse_json_projection_block(&lines, i, line)? {
            detected_kind = Some(select_diagram_kind(
                detected_kind,
                DiagramKind::Class,
                span,
            )?);
            let block_span = Span::new(span.start, lines[end_idx].1.end);
            statements.push(Statement {
                span: block_span,
                kind,
            });
            i = end_idx + 1;
            continue;
        }

        // Salt wireframe grid row parsing: `|cell|cell|cell|`
        if matches!(detected_kind, Some(DiagramKind::Salt)) {
            if let Some(kind) = parse_salt_grid_row(line) {
                statements.push(Statement { span, kind });
                i += 1;
                continue;
            }
            // Skip structural braces and separator sentinels inside salt blocks.
            // Rich containers such as `{+`, `{#`, `{SI`, and `{^` are parsed
            // as Salt rows above so the renderer can preserve widget metadata.
            let trimmed = line.trim();
            if matches!(trimmed, "{" | "{-" | "---") || trimmed.is_empty() {
                i += 1;
                continue;
            }
        }

        statements.push(Statement {
            span,
            kind: StatementKind::Unknown(line.to_string()),
        });
        i += 1;
    }

    if in_block {
        return Err(Diagnostic::error(
            "unmatched @startuml/@enduml boundary: opening @start marker is missing a closing @enduml",
        )
        .with_span(block_start_span.unwrap_or(Span::new(0, 0))));
    }

    Ok(Document {
        kind: detected_kind.unwrap_or(DiagramKind::Unknown),
        statements,
    })
}

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

fn parse_family_declaration(
    lines: &[(&str, Span)],
    start: usize,
    line: &str,
) -> Result<Option<(StatementKind, usize)>, Diagnostic> {
    for (keyword, marker) in [
        ("abstract class", Some("<<abstract class>>")),
        ("interface", Some("<<interface>>")),
        ("enum", Some("<<enum>>")),
        ("annotation", Some("<<annotation>>")),
        ("protocol", Some("<<protocol>>")),
        ("struct", Some("<<struct>>")),
        ("abstract", Some("<<abstract>>")),
        ("class", None),
    ] {
        let Some(decl) = parse_named_family_decl(line, keyword) else {
            continue;
        };
        let FamilyDeclParts {
            name,
            alias,
            has_block,
            stereotypes,
            fill_color,
        } = decl;
        let mut members = if has_block {
            let mut members = parse_family_decl_members(lines, start, keyword, &name)?;
            if let Some(marker) = marker {
                members.insert(
                    0,
                    ClassMember {
                        text: marker.to_string(),
                        modifier: None,
                    },
                );
            }
            for stereotype in stereotypes.iter().rev() {
                members.insert(
                    0,
                    ClassMember {
                        text: format!("<<{stereotype}>>"),
                        modifier: None,
                    },
                );
            }
            members
        } else {
            declaration_marker_members(marker, stereotypes)
        };
        append_inline_fill_member(&mut members, fill_color);
        return Ok(Some((
            StatementKind::ClassDecl(ClassDecl {
                name,
                alias,
                members,
            }),
            if has_block {
                find_family_decl_end(lines, start)
            } else {
                start
            },
        )));
    }

    for (keyword, marker) in [("map", Some("<<map>>")), ("object", None)] {
        let Some(decl) = parse_named_family_decl(line, keyword) else {
            continue;
        };
        let FamilyDeclParts {
            name,
            alias,
            has_block,
            stereotypes,
            fill_color,
        } = decl;
        let mut members = if has_block {
            let mut members = parse_family_decl_members(lines, start, keyword, &name)?;
            if let Some(marker) = marker {
                members.insert(
                    0,
                    ClassMember {
                        text: marker.to_string(),
                        modifier: None,
                    },
                );
            }
            for stereotype in stereotypes.iter().rev() {
                members.insert(
                    0,
                    ClassMember {
                        text: format!("<<{stereotype}>>"),
                        modifier: None,
                    },
                );
            }
            members
        } else {
            declaration_marker_members(marker, stereotypes)
        };
        append_inline_fill_member(&mut members, fill_color);
        return Ok(Some((
            StatementKind::ObjectDecl(ObjectDecl {
                name,
                alias,
                members,
            }),
            if has_block {
                find_family_decl_end(lines, start)
            } else {
                start
            },
        )));
    }

    if let Some(decl) = parse_parenthesized_usecase_decl(line) {
        let FamilyDeclParts {
            name,
            alias,
            has_block,
            fill_color,
            ..
        } = decl;
        let mut members = Vec::new();
        append_inline_fill_member(&mut members, fill_color);
        return Ok(Some((
            StatementKind::UseCaseDecl(UseCaseDecl {
                name,
                alias,
                members,
            }),
            if has_block {
                find_family_decl_end(lines, start)
            } else {
                start
            },
        )));
    }

    for (keyword, marker) in [("actor", Some("<<actor>>")), ("usecase", None)] {
        let Some(decl) = parse_named_family_decl(line, keyword) else {
            continue;
        };
        let FamilyDeclParts {
            name,
            alias,
            has_block,
            stereotypes,
            fill_color,
        } = decl;
        let mut members = if has_block {
            let mut members = parse_family_decl_members(lines, start, keyword, &name)?;
            if let Some(marker) = marker {
                members.insert(
                    0,
                    ClassMember {
                        text: marker.to_string(),
                        modifier: None,
                    },
                );
            }
            for stereotype in stereotypes.iter().rev() {
                members.insert(
                    0,
                    ClassMember {
                        text: format!("<<{stereotype}>>"),
                        modifier: None,
                    },
                );
            }
            members
        } else {
            declaration_marker_members(marker, stereotypes)
        };
        append_inline_fill_member(&mut members, fill_color);
        return Ok(Some((
            StatementKind::UseCaseDecl(UseCaseDecl {
                name,
                alias,
                members,
            }),
            if has_block {
                find_family_decl_end(lines, start)
            } else {
                start
            },
        )));
    }
    Ok(None)
}

fn later_lines_contain_class_family_declaration(lines: &[(&str, Span)], start: usize) -> bool {
    lines.iter().skip(start + 1).any(|(raw, _)| {
        let line = raw.trim();
        line.starts_with("abstract class ")
            || line.starts_with("abstract ")
            || line.starts_with("annotation ")
            || line.starts_with("class ")
            || line.starts_with("enum ")
            || line.starts_with("protocol ")
            || line.starts_with("struct ")
    })
}

fn later_lines_contain_usecase_family_declaration(lines: &[(&str, Span)], start: usize) -> bool {
    lines.iter().skip(start + 1).any(|(raw, _)| {
        let line = raw.trim();
        line.starts_with("usecase ") || line.starts_with("usecase(")
    })
}

#[derive(Debug, Clone)]
struct FamilyDeclParts {
    name: String,
    alias: Option<String>,
    has_block: bool,
    stereotypes: Vec<String>,
    fill_color: Option<String>,
}

fn parse_named_family_decl(line: &str, keyword: &str) -> Option<FamilyDeclParts> {
    if !line.starts_with(keyword) {
        return None;
    }
    if line.len() > keyword.len()
        && !line[keyword.len()..]
            .chars()
            .next()
            .is_some_and(char::is_whitespace)
    {
        return None;
    }
    let rest = line[keyword.len()..].trim();
    if rest.is_empty() {
        return None;
    }

    let has_block = rest.ends_with('{');
    let trimmed = if has_block {
        rest.trim_end_matches('{').trim()
    } else {
        rest
    };
    let (trimmed, fill_color) = split_declaration_inline_fill(trimmed);
    let trimmed = trimmed.trim();

    let (name_raw, alias_raw) = if let Some((lhs, rhs)) = trimmed.split_once(" as ") {
        (lhs.trim(), Some(rhs.trim()))
    } else {
        (trimmed, None)
    };

    let (name_without_stereotypes, stereotypes) = strip_declaration_stereotypes(name_raw);
    let name = clean_ident(&name_without_stereotypes);
    if name.is_empty() {
        return None;
    }
    let alias = alias_raw.map(clean_ident).filter(|v| !v.is_empty());
    Some(FamilyDeclParts {
        name,
        alias,
        has_block,
        stereotypes,
        fill_color,
    })
}

fn append_inline_fill_member(members: &mut Vec<ClassMember>, fill_color: Option<String>) {
    if let Some(color) = fill_color {
        members.push(ClassMember {
            text: format!("\x1fstyle:fill:{color}"),
            modifier: None,
        });
    }
}

fn split_declaration_inline_fill(input: &str) -> (String, Option<String>) {
    let trimmed = input.trim();
    let mut in_quote = false;
    let mut last_hash: Option<usize> = None;
    for (idx, ch) in trimmed.char_indices() {
        if ch == '"' {
            in_quote = !in_quote;
            continue;
        }
        if !in_quote && ch == '#' {
            last_hash = Some(idx);
        }
    }
    let Some(hash_idx) = last_hash else {
        return (trimmed.to_string(), None);
    };
    if hash_idx > 0
        && !trimmed[..hash_idx]
            .chars()
            .last()
            .is_some_and(char::is_whitespace)
    {
        return (trimmed.to_string(), None);
    }
    let after = &trimmed[hash_idx..];
    let token_len = after
        .char_indices()
        .take_while(|(_, ch)| ch.is_ascii_alphanumeric() || matches!(ch, '#' | '_' | '-' | ':'))
        .map(|(idx, ch)| idx + ch.len_utf8())
        .last()
        .unwrap_or(0);
    if token_len == 0 {
        return (trimmed.to_string(), None);
    }
    let token = &after[..token_len];
    let Some(color) = parse_relation_color_token(token) else {
        return (trimmed.to_string(), None);
    };
    let before = trimmed[..hash_idx].trim_end();
    let suffix = after[token_len..].trim_start();
    let mut cleaned = before.to_string();
    if !suffix.is_empty() {
        if !cleaned.is_empty() {
            cleaned.push(' ');
        }
        cleaned.push_str(suffix);
    }
    (cleaned, Some(color))
}

fn declaration_marker_members(marker: Option<&str>, stereotypes: Vec<String>) -> Vec<ClassMember> {
    let mut members = Vec::new();
    if let Some(marker) = marker {
        members.push(ClassMember {
            text: marker.to_string(),
            modifier: None,
        });
    }
    for stereotype in stereotypes {
        members.push(ClassMember {
            text: format!("<<{stereotype}>>"),
            modifier: None,
        });
    }
    members
}

fn strip_declaration_stereotypes(input: &str) -> (String, Vec<String>) {
    let mut remaining = input.trim().to_string();
    let mut stereotypes = Vec::new();
    while let Some(start) = remaining.find("<<") {
        let Some(end_rel) = remaining[start + 2..].find(">>") else {
            break;
        };
        let end = start + 2 + end_rel;
        let value = remaining[start + 2..end].trim();
        if !value.is_empty() {
            stereotypes.push(value.to_string());
        }
        remaining.replace_range(start..end + 2, "");
    }
    (remaining.trim().to_string(), stereotypes)
}

fn parse_parenthesized_usecase_decl(line: &str) -> Option<FamilyDeclParts> {
    let trimmed = line.trim();
    let trimmed = trimmed.strip_prefix("usecase ").unwrap_or(trimmed).trim();
    if !trimmed.starts_with('(') {
        return None;
    }
    let close = trimmed.find(')')?;
    let name_raw = trimmed[1..close].trim();
    if name_raw.is_empty() {
        return None;
    }
    let rest = trimmed[close + 1..].trim();
    let has_block = rest.ends_with('{');
    let rest = if has_block {
        rest.trim_end_matches('{').trim()
    } else {
        rest
    };
    let (rest, fill_color) = split_declaration_inline_fill(rest);
    let rest = rest.trim();
    let alias = rest
        .strip_prefix("as ")
        .map(str::trim)
        .map(clean_ident)
        .filter(|v| !v.is_empty());
    Some(FamilyDeclParts {
        name: clean_ident(name_raw),
        alias,
        has_block,
        stereotypes: Vec::new(),
        fill_color,
    })
}

fn parse_family_decl_members(
    lines: &[(&str, Span)],
    start: usize,
    keyword: &str,
    name: &str,
) -> Result<Vec<ClassMember>, Diagnostic> {
    let end_idx = find_family_decl_end(lines, start);
    if end_idx == start {
        return Err(Diagnostic::error(format!(
            "[E_FAMILY_DECL_BLOCK_UNCLOSED] unclosed {keyword} declaration block for `{name}`: missing `}}`",
        ))
        .with_span(lines[start].1));
    }
    let mut members = Vec::new();
    for (raw, _) in lines.iter().take(end_idx).skip(start + 1) {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            members.push(parse_class_member(trimmed));
        }
    }
    Ok(members)
}

/// Parse a single member line, extracting any `{field}`, `{method}`, `{abstract}`,
/// `{static}`, or `{class}` modifier token (trailing or leading), as well as
/// `<<abstract>>` and `<<static>>` stereotype tokens.
fn parse_class_member(raw: &str) -> ClassMember {
    // Check for leading brace modifier: `{field} +id: UUID`
    if let Some(rest) = try_strip_leading_brace_modifier(raw) {
        let modifier = parse_brace_modifier_word(leading_brace_word(raw));
        return ClassMember {
            text: rest.trim().to_string(),
            modifier,
        };
    }

    // Check for trailing brace modifier: `+id: UUID {field}`
    if let Some((text_part, mod_word)) = try_strip_trailing_brace_modifier(raw) {
        let modifier = parse_brace_modifier_word(mod_word);
        return ClassMember {
            text: text_part.trim().to_string(),
            modifier,
        };
    }

    // Check for leading `<<abstract>>` or `<<static>>` stereotype
    if let Some((modifier, rest)) = try_strip_leading_stereotype_modifier(raw) {
        return ClassMember {
            text: rest.trim().to_string(),
            modifier: Some(modifier),
        };
    }

    // Check for trailing `<<abstract>>` or `<<static>>` stereotype
    if let Some((text_part, modifier)) = try_strip_trailing_stereotype_modifier(raw) {
        return ClassMember {
            text: text_part.trim().to_string(),
            modifier: Some(modifier),
        };
    }

    ClassMember {
        text: raw.to_string(),
        modifier: None,
    }
}

fn leading_brace_word(s: &str) -> &str {
    // returns the content between the first { and }
    if let Some(rest) = s.strip_prefix('{') {
        if let Some(end) = rest.find('}') {
            return rest[..end].trim();
        }
    }
    ""
}

fn try_strip_leading_brace_modifier(s: &str) -> Option<&str> {
    let s = s.trim_start();
    if !s.starts_with('{') {
        return None;
    }
    let rest = &s[1..];
    let end = rest.find('}')?;
    let word = rest[..end].trim();
    if is_member_modifier_word(word) {
        Some(rest[end + 1..].trim())
    } else {
        None
    }
}

fn try_strip_trailing_brace_modifier(s: &str) -> Option<(&str, &str)> {
    let s = s.trim_end();
    if !s.ends_with('}') {
        return None;
    }
    let start = s.rfind('{')?;
    let word = s[start + 1..s.len() - 1].trim();
    if is_member_modifier_word(word) {
        Some((&s[..start], word))
    } else {
        None
    }
}

fn try_strip_leading_stereotype_modifier(s: &str) -> Option<(MemberModifier, &str)> {
    let s = s.trim_start();
    if !s.starts_with("<<") {
        return None;
    }
    let rest = &s[2..];
    let end = rest.find(">>")?;
    let word = rest[..end].trim();
    let modifier = match word.to_ascii_lowercase().as_str() {
        "abstract" => MemberModifier::Abstract,
        "static" => MemberModifier::Static,
        _ => return None,
    };
    Some((modifier, rest[end + 2..].trim()))
}

fn try_strip_trailing_stereotype_modifier(s: &str) -> Option<(&str, MemberModifier)> {
    let s = s.trim_end();
    if !s.ends_with(">>") {
        return None;
    }
    let start = s.rfind("<<")?;
    let word = s[start + 2..s.len() - 2].trim();
    let modifier = match word.to_ascii_lowercase().as_str() {
        "abstract" => MemberModifier::Abstract,
        "static" => MemberModifier::Static,
        _ => return None,
    };
    Some((&s[..start], modifier))
}

fn is_member_modifier_word(word: &str) -> bool {
    matches!(
        word.to_ascii_lowercase().as_str(),
        "field" | "method" | "abstract" | "static" | "class"
    )
}

fn parse_brace_modifier_word(word: &str) -> Option<MemberModifier> {
    match word.to_ascii_lowercase().as_str() {
        "field" => Some(MemberModifier::Field),
        "method" => Some(MemberModifier::Method),
        "abstract" => Some(MemberModifier::Abstract),
        "static" | "class" => Some(MemberModifier::Static),
        _ => None,
    }
}

fn find_family_decl_end(lines: &[(&str, Span)], start: usize) -> usize {
    for (idx, (raw, _)) in lines.iter().enumerate().skip(start + 1) {
        if raw.trim() == "}" {
            return idx;
        }
    }
    start
}

fn parse_family_relation(line: &str, family: Option<DiagramKind>) -> Option<StatementKind> {
    match family {
        Some(DiagramKind::Class)
        | Some(DiagramKind::Object)
        | Some(DiagramKind::UseCase)
        | Some(DiagramKind::Salt)
        | Some(DiagramKind::Component)
        | Some(DiagramKind::Deployment) => {}
        _ => return None,
    }

    let (core, raw_label) = split_family_relation_label(line);
    let (lhs, arrow, relation_style, rhs) = split_family_arrow_styled(core)?;
    if !arrow.contains('-') && !arrow.contains('.') {
        return None;
    }
    let (rhs, trailing_stereotype) = split_relation_trailing_stereotype(rhs);
    let (label, label_stereotype) = split_relation_label_stereotype(raw_label);
    let (lhs_core, left_cardinality, left_role) = parse_relation_side_annotations(lhs, true);
    let (rhs_core, right_cardinality, right_role) = parse_relation_side_annotations(rhs, false);
    let (lhs_core, left_lollipop) = strip_lollipop_endpoint(&lhs_core);
    let (rhs_core, right_lollipop) = strip_lollipop_endpoint(&rhs_core);
    if normalize_virtual_endpoint(&lhs_core).is_some()
        || normalize_virtual_endpoint(&rhs_core).is_some()
        || looks_like_virtual_endpoint_syntax(&lhs_core)
        || looks_like_virtual_endpoint_syntax(&rhs_core)
    {
        return None;
    }
    let from = clean_bracketed_ident(&lhs_core);
    let to = clean_bracketed_ident(&rhs_core);
    if from.is_empty() || to.is_empty() {
        return None;
    }
    Some(StatementKind::FamilyRelation(FamilyRelation {
        from,
        to,
        arrow,
        label,
        stereotype: label_stereotype.or(trailing_stereotype),
        left_cardinality,
        right_cardinality,
        left_role,
        right_role,
        line_color: relation_style.line_color,
        dashed: relation_style.dashed,
        hidden: relation_style.hidden,
        thickness: relation_style.thickness,
        direction: relation_style.direction,
        left_lollipop,
        right_lollipop,
    }))
}

fn strip_lollipop_endpoint(side: &str) -> (String, bool) {
    let trimmed = side.trim();
    if let Some(rest) = trimmed.strip_prefix("()") {
        return (rest.trim_start().to_string(), true);
    }
    if let Some(rest) = trimmed.strip_suffix("()") {
        return (rest.trim_end().to_string(), true);
    }
    (trimmed.to_string(), false)
}

fn split_relation_label_stereotype(label: Option<String>) -> (Option<String>, Option<String>) {
    let Some(label) = label else {
        return (None, None);
    };
    let trimmed = label.trim();
    if let Some((stereotype, rest)) = parse_leading_stereotype(trimmed) {
        let label = rest.trim();
        return (
            (!label.is_empty()).then(|| label.to_string()),
            Some(stereotype),
        );
    }
    (Some(label), None)
}

fn split_relation_trailing_stereotype(side: &str) -> (&str, Option<String>) {
    let trimmed = side.trim();
    let Some(open) = trimmed.rfind("<<") else {
        return (side, None);
    };
    let before = trimmed[..open].trim_end();
    let tail = trimmed[open..].trim();
    if before.is_empty() {
        return (side, None);
    }
    if let Some((stereotype, rest)) = parse_leading_stereotype(tail) {
        if rest.trim().is_empty() {
            return (before, Some(stereotype));
        }
    }
    (side, None)
}

fn parse_leading_stereotype(s: &str) -> Option<(String, &str)> {
    let rest = s.trim_start().strip_prefix("<<")?;
    let close = rest.find(">>")?;
    let value = rest[..close].trim();
    if value.is_empty() {
        return None;
    }
    Some((value.to_string(), &rest[close + 2..]))
}

fn parse_family_member_row(line: &str, family: Option<DiagramKind>) -> Option<StatementKind> {
    let family = match family {
        Some(DiagramKind::Class | DiagramKind::Object | DiagramKind::UseCase) => family?,
        _ => return None,
    };
    if split_family_arrow(line).is_some() {
        return None;
    }
    let (owner, member) = line.split_once(':')?;
    if owner.contains("--") || owner.contains("..") || owner.contains("->") || owner.contains("<-")
    {
        return None;
    }
    let owner = clean_bracketed_ident(owner);
    let member = member.trim();
    if owner.is_empty() || member.is_empty() {
        return None;
    }
    let members = vec![parse_class_member(member)];
    Some(match family {
        DiagramKind::Object => StatementKind::ObjectDecl(ObjectDecl {
            name: owner,
            alias: None,
            members,
        }),
        DiagramKind::UseCase => StatementKind::UseCaseDecl(UseCaseDecl {
            name: owner,
            alias: None,
            members,
        }),
        _ => StatementKind::ClassDecl(ClassDecl {
            name: owner,
            alias: None,
            members,
        }),
    })
}

fn parse_relation_side_annotations(
    side: &str,
    is_left: bool,
) -> (String, Option<String>, Option<String>) {
    let trimmed = side.trim();
    if trimmed.is_empty() {
        return (String::new(), None, None);
    }

    let mut rem = trimmed.to_string();
    let mut cardinality: Option<String> = None;
    let mut role: Option<String> = None;

    if is_left {
        loop {
            let t = rem.trim_end();
            if t.ends_with(']') {
                if let Some(start_bracket) = t.rfind('[') {
                    let value = t[start_bracket + 1..t.len() - 1].trim();
                    let endpoint = t[..start_bracket].trim_end();
                    if !value.is_empty() && !endpoint.is_empty() {
                        if role.is_none() {
                            role = Some(value.to_string());
                        }
                        rem = endpoint.to_string();
                        continue;
                    }
                }
            }
            if let Some(stripped) = t.strip_suffix('"') {
                if let Some(start_quote) = stripped.rfind('"') {
                    let value = stripped[start_quote + 1..].trim();
                    let endpoint = t[..start_quote].trim_end();
                    if !value.is_empty() && !endpoint.is_empty() {
                        if cardinality.is_none() {
                            cardinality = Some(value.to_string());
                        }
                        rem = endpoint.to_string();
                        continue;
                    }
                }
            }
            if let Some(colon) = t.rfind(" :") {
                let value = t[colon + 2..].trim();
                let endpoint = t[..colon].trim_end();
                if !value.is_empty() && !endpoint.is_empty() {
                    if role.is_none() {
                        role = Some(value.to_string());
                    }
                    rem = endpoint.to_string();
                    continue;
                }
            }
            break;
        }
    } else {
        loop {
            let t = rem.trim_start();
            if let Some(rest) = t.strip_prefix('"') {
                if let Some(end_quote_rel) = rest.find('"') {
                    let value = rest[..end_quote_rel].trim();
                    let endpoint = rest[end_quote_rel + 1..].trim_start();
                    if !value.is_empty() && !endpoint.is_empty() {
                        if cardinality.is_none() {
                            cardinality = Some(value.to_string());
                        }
                        rem = endpoint.to_string();
                        continue;
                    }
                }
            }
            if let Some(rest) = t.strip_prefix('[') {
                if let Some(end_bracket_rel) = rest.find(']') {
                    let value = rest[..end_bracket_rel].trim();
                    let endpoint = rest[end_bracket_rel + 1..].trim_start();
                    if !value.is_empty() && !endpoint.is_empty() {
                        if role.is_none() {
                            role = Some(value.to_string());
                        }
                        rem = endpoint.to_string();
                        continue;
                    }
                }
            }
            if let Some(rest) = t.strip_prefix(':') {
                let value_len = rest
                    .char_indices()
                    .take_while(|(_, ch)| !ch.is_whitespace())
                    .map(|(idx, ch)| idx + ch.len_utf8())
                    .last()
                    .unwrap_or(0);
                if value_len > 0 {
                    let value = rest[..value_len].trim();
                    let endpoint = rest[value_len..].trim_start();
                    if !value.is_empty() && !endpoint.is_empty() {
                        if role.is_none() {
                            role = Some(value.to_string());
                        }
                        rem = endpoint.to_string();
                        continue;
                    }
                }
            }
            break;
        }
    }

    (rem.trim().to_string(), cardinality, role)
}

#[derive(Debug, Clone, Default)]
struct ParsedFamilyRelationStyle {
    line_color: Option<String>,
    dashed: bool,
    hidden: bool,
    thickness: Option<u8>,
    direction: Option<String>,
}

fn split_family_arrow(core: &str) -> Option<(&str, String, &str)> {
    split_family_arrow_styled(core).map(|(lhs, arrow, _, rhs)| (lhs, arrow, rhs))
}

fn split_family_arrow_styled(
    core: &str,
) -> Option<(&str, String, ParsedFamilyRelationStyle, &str)> {
    let mut in_quote = false;
    for (idx, ch) in core.char_indices() {
        if ch == '"' {
            in_quote = !in_quote;
            continue;
        }
        if in_quote {
            continue;
        }
        if !matches!(ch, '-' | '.' | '<' | '*' | 'o' | '+' | '|') {
            continue;
        }
        let rest = &core[idx..];
        let Some(len) = family_arrow_token_len(rest) else {
            continue;
        };
        if len == 1 {
            continue;
        }
        let lhs = core[..idx].trim();
        let rhs = core[idx + len..].trim();
        if lhs.is_empty() || rhs.is_empty() {
            continue;
        }
        let raw_arrow = &rest[..len];
        let arrow = normalize_family_arrow_token(raw_arrow);
        if arrow.is_empty() {
            continue;
        }
        let mut relation_style = parse_family_relation_style(raw_arrow);
        relation_style.direction = parse_family_relation_direction(raw_arrow);
        return Some((lhs, arrow, relation_style, rhs));
    }
    None
}

fn parse_family_relation_style(raw_arrow: &str) -> ParsedFamilyRelationStyle {
    let mut style = ParsedFamilyRelationStyle::default();
    let mut rest = raw_arrow;
    while let Some(open) = rest.find('[') {
        let after_open = &rest[open + 1..];
        let Some(close) = after_open.find(']') else {
            break;
        };
        let content = &after_open[..close];
        for part in content.split(|ch: char| ch == ',' || ch == ';' || ch.is_whitespace()) {
            let token = part.trim();
            if token.is_empty() {
                continue;
            }
            let lower_raw = token.to_ascii_lowercase();
            let lower = lower_raw
                .strip_prefix("line.")
                .or_else(|| lower_raw.strip_prefix("line:"))
                .unwrap_or(lower_raw.as_str());
            match lower {
                "dashed" | "dotted" | "dash" | "dot" => style.dashed = true,
                "hidden" => style.hidden = true,
                "bold" | "thick" => style.thickness = Some(style.thickness.unwrap_or(3).max(3)),
                "thin" => style.thickness = Some(1),
                _ => {
                    if let Some(value) = lower
                        .strip_prefix("thickness=")
                        .or_else(|| lower.strip_prefix("thickness:"))
                        .or_else(|| lower.strip_prefix("weight="))
                        .or_else(|| lower.strip_prefix("weight:"))
                    {
                        if let Ok(n) = value.trim().parse::<u8>() {
                            style.thickness = Some(n.clamp(1, 8));
                        }
                    } else if let Some(color) = parse_relation_color_token(token) {
                        style.line_color = Some(color);
                    }
                }
            }
        }
        rest = &after_open[close + 1..];
    }
    style
}

fn parse_family_relation_direction(raw_arrow: &str) -> Option<String> {
    let mut cleaned = String::new();
    let mut in_bracket = false;
    for ch in raw_arrow.chars() {
        match ch {
            '[' => in_bracket = true,
            ']' => in_bracket = false,
            _ if !in_bracket => cleaned.push(ch),
            _ => {}
        }
    }
    let lower = cleaned.to_ascii_lowercase();
    for (needle, direction) in [
        ("left", "left"),
        ("right", "right"),
        ("up", "up"),
        ("down", "down"),
        ("l", "left"),
        ("r", "right"),
        ("u", "up"),
        ("d", "down"),
    ] {
        if lower.contains(needle) {
            return Some(direction.to_string());
        }
    }
    None
}

fn parse_relation_color_token(token: &str) -> Option<String> {
    let trimmed = token.trim();
    if trimmed.len() == 7
        && trimmed.starts_with('#')
        && trimmed[1..].chars().all(|ch| ch.is_ascii_hexdigit())
    {
        return Some(trimmed.to_ascii_lowercase());
    }
    crate::theme::css3_color_to_hex(trimmed.trim_start_matches('#')).map(str::to_string)
}

fn family_arrow_token_len(s: &str) -> Option<usize> {
    if let Some(len) = directional_family_arrow_token_len(s) {
        return Some(len);
    }

    let len = s
        .char_indices()
        .take_while(|(_, ch)| matches!(ch, '-' | '.' | '<' | '>' | '|' | '*' | 'o' | '+'))
        .map(|(idx, ch)| idx + ch.len_utf8())
        .last()?;
    let token = &s[..len];
    if is_family_arrow_token(token) {
        Some(len)
    } else {
        None
    }
}

fn directional_family_arrow_token_len(s: &str) -> Option<usize> {
    let dirs = ["left", "right", "up", "down", "l", "r", "u", "d"];
    for prefix_len in 1..=2 {
        let prefix = s.get(..prefix_len)?;
        if !prefix.chars().all(|ch| matches!(ch, '-' | '.')) {
            continue;
        }
        let after_prefix = &s[prefix_len..];
        if let Some(after_directive) = after_prefix.strip_prefix('[') {
            if let Some(close) = after_directive.find(']') {
                let after_with_optional_dir = &after_directive[close + 1..];
                let after = dirs
                    .iter()
                    .find_map(|dir| after_with_optional_dir.strip_prefix(dir))
                    .unwrap_or(after_with_optional_dir);
                let dir_len = after_with_optional_dir.len().saturating_sub(after.len());
                let suffix_len = after
                    .char_indices()
                    .take_while(|(_, ch)| matches!(ch, '-' | '.' | '<' | '>' | '|'))
                    .map(|(idx, ch)| idx + ch.len_utf8())
                    .last()
                    .unwrap_or(0);
                if suffix_len > 0 {
                    return Some(prefix_len + close + 2 + dir_len + suffix_len);
                }
            }
        }
        for dir in dirs {
            if let Some(after_dir) = after_prefix.strip_prefix(dir) {
                let suffix_len = after_dir
                    .char_indices()
                    .take_while(|(_, ch)| matches!(ch, '-' | '.' | '<' | '>' | '|'))
                    .map(|(idx, ch)| idx + ch.len_utf8())
                    .last()
                    .unwrap_or(0);
                if suffix_len > 0 {
                    return Some(prefix_len + dir.len() + suffix_len);
                }
            }
        }
    }
    None
}

fn is_family_arrow_token(token: &str) -> bool {
    token.contains('-') || token.contains('<') || token.contains('>') || token.contains("..")
}

fn normalize_family_arrow_token(token: &str) -> String {
    let mut out = String::new();
    let mut chars = token.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '[' {
            for next in chars.by_ref() {
                if next == ']' {
                    break;
                }
            }
            continue;
        }
        if ch.is_ascii_alphabetic() {
            continue;
        }
        out.push(ch);
    }
    out
}

fn clean_bracketed_ident(s: &str) -> String {
    let trimmed = s.trim();
    // Preserve special state markers like [*] verbatim.
    if trimmed == "[*]" || trimmed == "[H]" || trimmed == "[H*]" {
        return trimmed.to_string();
    }
    // Allow `[Name]` shorthand: strip the surrounding brackets if balanced and no interior bracket.
    if let Some(inner) = trimmed.strip_prefix('[').and_then(|v| v.strip_suffix(']')) {
        if !inner.contains('[') && !inner.contains(']') && !inner.is_empty() {
            return clean_ident(inner);
        }
    }
    // Strip `()` interface-style prefix `() Name`.
    if let Some(rest) = trimmed.strip_prefix("()") {
        return clean_ident(rest.trim());
    }
    clean_ident(trimmed)
}

#[derive(Debug, Clone, Default)]
struct ScopedGroupContent {
    members: Vec<String>,
    relations: Vec<FamilyRelation>,
}

/// Parse `together { ... }`, `package "name" { ... }`, `namespace ns { ... }` blocks.
/// Returns (StatementKind, end_line_index) where end_line_index points to the closing `}`.
fn parse_class_scoping_block(
    lines: &[(&str, Span)],
    start: usize,
    line: &str,
) -> Result<Option<(StatementKind, usize)>, Diagnostic> {
    let lower = line.to_ascii_lowercase();

    // together { ... }
    if lower == "together {" || lower.starts_with("together {") {
        let end_idx = find_family_decl_end(lines, start);
        if end_idx == start {
            return Err(Diagnostic::error(
                "[E_CLASS_TOGETHER_UNCLOSED] unclosed `together` block: missing `}`",
            )
            .with_span(lines[start].1));
        }
        let members: Vec<String> = lines[start + 1..end_idx]
            .iter()
            .map(|(raw, _)| raw.trim())
            .filter(|s| !s.is_empty())
            .map(clean_ident)
            .filter(|s| !s.is_empty())
            .collect();
        return Ok(Some((
            StatementKind::ClassGroup {
                kind: "together".to_string(),
                label: None,
                members,
                relations: Vec::new(),
            },
            end_idx,
        )));
    }

    // package "label" { ... } or package label { ... }
    if lower.starts_with("package ") && line.trim_end().ends_with('{') {
        let rest = line.strip_prefix("package ").unwrap_or("").trim();
        let label_raw = rest.trim_end_matches('{').trim();
        let label = clean_ident(label_raw.trim_matches('"'));
        let end_idx = find_scoping_block_end(lines, start);
        if end_idx == start {
            return Err(Diagnostic::error(
                "[E_CLASS_PACKAGE_UNCLOSED] unclosed `package` block: missing `}`",
            )
            .with_span(lines[start].1));
        }
        if group_body_contains_component_family(lines, start, end_idx)
            && !group_body_contains_class_family(lines, start, end_idx)
            && !group_body_contains_object_family(lines, start, end_idx)
            && !group_body_contains_usecase_family(lines, start, end_idx)
        {
            return Ok(None);
        }
        let content =
            collect_scoped_class_group_content(lines, start, end_idx, std::slice::from_ref(&label));
        return Ok(Some((
            StatementKind::ClassGroup {
                kind: "package".to_string(),
                label: if label.is_empty() { None } else { Some(label) },
                members: content.members,
                relations: content.relations,
            },
            end_idx,
        )));
    }

    // namespace ns { ... }
    if lower.starts_with("namespace ") && line.trim_end().ends_with('{') {
        let rest = line.strip_prefix("namespace ").unwrap_or("").trim();
        let label_raw = rest.trim_end_matches('{').trim();
        let label = clean_ident(label_raw.trim_matches('"'));
        let end_idx = find_scoping_block_end(lines, start);
        if end_idx == start {
            return Err(Diagnostic::error(
                "[E_CLASS_NAMESPACE_UNCLOSED] unclosed `namespace` block: missing `}`",
            )
            .with_span(lines[start].1));
        }
        let content =
            collect_scoped_class_group_content(lines, start, end_idx, std::slice::from_ref(&label));
        return Ok(Some((
            StatementKind::ClassGroup {
                kind: "namespace".to_string(),
                label: if label.is_empty() { None } else { Some(label) },
                members: content.members,
                relations: content.relations,
            },
            end_idx,
        )));
    }

    Ok(None)
}

fn collect_scoped_class_group_content(
    lines: &[(&str, Span)],
    start: usize,
    end_idx: usize,
    scope: &[String],
) -> ScopedGroupContent {
    let mut content = ScopedGroupContent::default();
    let mut idx = start + 1;
    while idx < end_idx {
        let line = lines[idx].0.trim();
        let lower = line.to_ascii_lowercase();
        if line.is_empty() || line == "}" {
            idx += 1;
            continue;
        }
        if let Some(StatementKind::FamilyRelation(rel)) =
            parse_family_relation(line, Some(DiagramKind::Class))
        {
            content.relations.push(qualify_scoped_relation(rel, scope));
            idx += 1;
            continue;
        }
        if (lower.starts_with("package ") || lower.starts_with("namespace "))
            && line.trim_end().ends_with('{')
        {
            let keyword = if lower.starts_with("package ") {
                "package"
            } else {
                "namespace"
            };
            let label = clean_ident(
                line[keyword.len()..]
                    .trim()
                    .trim_end_matches('{')
                    .trim()
                    .trim_matches('"'),
            );
            let nested_end = find_scoping_block_end(lines, idx);
            if nested_end > idx {
                let mut nested_scope = scope.to_vec();
                if !label.is_empty() {
                    nested_scope.push(label);
                }
                let nested =
                    collect_scoped_class_group_content(lines, idx, nested_end, &nested_scope);
                content.members.extend(nested.members);
                content.relations.extend(nested.relations);
                idx = nested_end + 1;
                continue;
            }
        }
        if let Some(decl) = parse_parenthesized_usecase_decl(line) {
            let FamilyDeclParts {
                name,
                alias,
                has_block,
                fill_color,
                ..
            } = decl;
            let has_alias = alias.is_some();
            let id = alias.unwrap_or_else(|| name.clone());
            let mut encoded = qualify_scoped_identifier(id, scope);
            if has_alias {
                encoded.push('\t');
                encoded.push_str(&name);
            }
            if let Some(fill_color) = fill_color {
                encoded.push('\t');
                encoded.push_str(&format!("\x1fstyle:fill:{fill_color}"));
            }
            content.members.push(encoded);
            if has_block {
                let nested_end = find_family_decl_end(lines, idx);
                if nested_end > idx {
                    idx = nested_end + 1;
                    continue;
                }
            }
            idx += 1;
            continue;
        }
        let declaration_keywords = [
            "abstract class",
            "annotation",
            "interface",
            "abstract",
            "enum",
            "protocol",
            "struct",
            "class",
            "object",
            "map",
            "actor",
            "usecase",
        ];
        let mut handled_declaration = false;
        for keyword in declaration_keywords {
            if let Some(decl) = parse_named_family_decl(line, keyword) {
                let FamilyDeclParts {
                    name,
                    alias,
                    has_block,
                    fill_color,
                    ..
                } = decl;
                let has_alias = alias.is_some();
                let id = alias.unwrap_or_else(|| name.clone());
                let scoped_name = qualify_scoped_identifier(id, scope);
                let mut encoded = scoped_name.clone();
                if has_alias {
                    encoded.push('\t');
                    encoded.push_str(&name);
                }
                if let Some(fill_color) = fill_color {
                    encoded.push('\t');
                    encoded.push_str(&format!("\x1fstyle:fill:{fill_color}"));
                }
                let nested_end = if has_block {
                    let nested_end = find_family_decl_end(lines, idx);
                    let members_text = parse_family_decl_members(lines, idx, keyword, &scoped_name)
                        .map(|members| {
                            members
                                .into_iter()
                                .map(|member| member.text)
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();
                    for member in members_text {
                        encoded.push('\t');
                        encoded.push_str(&member);
                    }
                    nested_end
                } else {
                    idx
                };
                content.members.push(encoded);
                idx = if nested_end > idx {
                    nested_end + 1
                } else {
                    idx + 1
                };
                handled_declaration = true;
                break;
            }
        }
        if handled_declaration {
            continue;
        }
        for keyword in [
            "abstract class",
            "annotation",
            "interface",
            "abstract",
            "enum",
            "protocol",
            "struct",
            "class",
            "object",
            "map",
            "actor",
            "usecase",
        ] {
            if let Some(decl) = parse_named_family_decl(line, keyword).filter(|decl| decl.has_block)
            {
                let FamilyDeclParts {
                    name,
                    alias,
                    fill_color,
                    ..
                } = decl;
                let has_alias = alias.is_some();
                let id = alias.unwrap_or_else(|| name.clone());
                let scoped_name = qualify_scoped_identifier(id, scope);
                let nested_end = find_family_decl_end(lines, idx);
                let members_text = parse_family_decl_members(lines, idx, keyword, &scoped_name)
                    .map(|members| {
                        members
                            .into_iter()
                            .map(|member| member.text)
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                let mut encoded = scoped_name;
                if has_alias {
                    encoded.push('\t');
                    encoded.push_str(&name);
                }
                if let Some(fill_color) = fill_color {
                    encoded.push('\t');
                    encoded.push_str(&format!("\x1fstyle:fill:{fill_color}"));
                }
                for member in members_text {
                    encoded.push('\t');
                    encoded.push_str(&member);
                }
                content.members.push(encoded);
                if nested_end > idx {
                    idx = nested_end + 1;
                    continue;
                }
            } else if let Some(decl) =
                parse_named_family_decl(line, keyword).filter(|decl| !decl.has_block)
            {
                let FamilyDeclParts {
                    name,
                    alias,
                    fill_color,
                    ..
                } = decl;
                let has_alias = alias.is_some();
                let id = alias.unwrap_or_else(|| name.clone());
                let mut encoded = qualify_scoped_identifier(id, scope);
                if has_alias {
                    encoded.push('\t');
                    encoded.push_str(&name);
                }
                if let Some(fill_color) = fill_color {
                    encoded.push('\t');
                    encoded.push_str(&format!("\x1fstyle:fill:{fill_color}"));
                }
                content.members.push(encoded);
                idx += 1;
                continue;
            }
        }
        let name = extract_class_member_name(line);
        if !name.is_empty() {
            let scoped = qualify_scoped_identifier(name, scope);
            content.members.push(scoped);
        }
        if line.ends_with('{') {
            let nested_end = find_family_decl_end(lines, idx);
            if nested_end > idx {
                idx = nested_end + 1;
                continue;
            }
        }
        idx += 1;
    }
    content
}

fn scoped_prefix(scope: &[String]) -> String {
    scope
        .iter()
        .filter(|s| !s.is_empty())
        .cloned()
        .collect::<Vec<_>>()
        .join("::")
}

fn qualify_scoped_identifier(name: String, scope: &[String]) -> String {
    let prefix = scoped_prefix(scope);
    if prefix.is_empty()
        || name.is_empty()
        || name.contains("::")
        || name == "[*]"
        || name == "[H]"
        || name == "[H*]"
    {
        name
    } else {
        format!("{prefix}::{name}")
    }
}

fn qualify_scoped_relation(mut rel: FamilyRelation, scope: &[String]) -> FamilyRelation {
    rel.from = qualify_scoped_identifier(rel.from, scope);
    rel.to = qualify_scoped_identifier(rel.to, scope);
    rel
}

fn group_body_contains_component_family(
    lines: &[(&str, Span)],
    start: usize,
    end_idx: usize,
) -> bool {
    lines[start + 1..end_idx].iter().any(|(raw, _)| {
        let line = strip_inline_plantuml_comment(raw).trim();
        let lower = line.to_ascii_lowercase();
        lower.starts_with("component ")
            || lower.starts_with("interface ")
            || lower.starts_with("node ")
            || lower.starts_with("artifact ")
            || lower.starts_with("database ")
            || lower.starts_with("cloud ")
            || lower.starts_with("frame ")
            || lower.starts_with("storage ")
            || lower.starts_with("rectangle ")
            || lower.starts_with("folder ")
            || lower.starts_with("file ")
            || lower.starts_with("card ")
            || lower.starts_with("actor ")
            || lower.starts_with("port ")
            || lower.starts_with("portin ")
            || lower.starts_with("portout ")
    })
}

fn group_body_contains_class_family(lines: &[(&str, Span)], start: usize, end_idx: usize) -> bool {
    lines[start + 1..end_idx].iter().any(|(raw, _)| {
        let line = strip_inline_plantuml_comment(raw).trim();
        let lower = line.to_ascii_lowercase();
        lower.starts_with("abstract class ")
            || lower.starts_with("annotation ")
            || lower.starts_with("interface ")
            || lower.starts_with("abstract ")
            || lower.starts_with("enum ")
            || lower.starts_with("protocol ")
            || lower.starts_with("struct ")
            || lower.starts_with("class ")
    })
}

fn group_body_contains_object_family(lines: &[(&str, Span)], start: usize, end_idx: usize) -> bool {
    lines[start + 1..end_idx].iter().any(|(raw, _)| {
        let line = strip_inline_plantuml_comment(raw).trim();
        let lower = line.to_ascii_lowercase();
        lower.starts_with("object ") || lower.starts_with("map ")
    })
}

fn group_body_contains_usecase_family(
    lines: &[(&str, Span)],
    start: usize,
    end_idx: usize,
) -> bool {
    lines[start + 1..end_idx].iter().any(|(raw, _)| {
        let line = strip_inline_plantuml_comment(raw).trim();
        let lower = line.to_ascii_lowercase();
        lower.starts_with("usecase ")
            || lower.starts_with("usecase(")
            || parse_parenthesized_usecase_decl(line).is_some()
    })
}

fn scoped_family_kind_for_block(
    lines: &[(&str, Span)],
    start: usize,
    end_idx: usize,
) -> DiagramKind {
    if group_body_contains_object_family(lines, start, end_idx) {
        DiagramKind::Object
    } else if group_body_contains_usecase_family(lines, start, end_idx) {
        DiagramKind::UseCase
    } else {
        DiagramKind::Class
    }
}

fn parse_component_scoping_block(
    lines: &[(&str, Span)],
    start: usize,
    line: &str,
) -> Result<Option<(StatementKind, usize)>, Diagnostic> {
    let trimmed = line.trim();
    let lower = trimmed.to_ascii_lowercase();
    let Some((kind, label_raw)) = lower
        .starts_with("package ")
        .then(|| {
            (
                "package",
                trimmed.strip_prefix("package ").unwrap_or("").trim(),
            )
        })
        .or_else(|| {
            lower
                .starts_with("node ")
                .then(|| ("node", trimmed.strip_prefix("node ").unwrap_or("").trim()))
        })
        .or_else(|| {
            lower
                .starts_with("frame ")
                .then(|| ("frame", trimmed.strip_prefix("frame ").unwrap_or("").trim()))
        })
        .or_else(|| {
            lower
                .starts_with("cloud ")
                .then(|| ("cloud", trimmed.strip_prefix("cloud ").unwrap_or("").trim()))
        })
        .or_else(|| {
            lower.starts_with("rectangle ").then(|| {
                (
                    "rectangle",
                    trimmed.strip_prefix("rectangle ").unwrap_or("").trim(),
                )
            })
        })
        .or_else(|| {
            lower.starts_with("namespace ").then(|| {
                (
                    "namespace",
                    trimmed.strip_prefix("namespace ").unwrap_or("").trim(),
                )
            })
        })
    else {
        return Ok(None);
    };
    if !trimmed.ends_with('{') {
        return Ok(None);
    }
    let end_idx = find_scoping_block_end(lines, start);
    if end_idx == start {
        return Err(Diagnostic::error(format!(
            "[E_COMPONENT_GROUP_UNCLOSED] unclosed `{kind}` block: missing `}}`",
        ))
        .with_span(lines[start].1));
    }
    if matches!(kind, "namespace" | "package") {
        let has_component_family = group_body_contains_component_family(lines, start, end_idx);
        if !has_component_family
            || group_body_contains_object_family(lines, start, end_idx)
            || group_body_contains_usecase_family(lines, start, end_idx)
        {
            return Ok(None);
        }
    }
    if kind == "namespace" && !group_body_contains_component_family(lines, start, end_idx) {
        return Ok(None);
    }
    let label = clean_ident(label_raw.trim_end_matches('{').trim().trim_matches('"'));
    let content =
        collect_scoped_component_group_content(lines, start, end_idx, std::slice::from_ref(&label));
    Ok(Some((
        StatementKind::ClassGroup {
            kind: kind.to_string(),
            label: if label.is_empty() { None } else { Some(label) },
            members: content.members,
            relations: content.relations,
        },
        end_idx,
    )))
}

fn collect_scoped_component_group_content(
    lines: &[(&str, Span)],
    start: usize,
    end_idx: usize,
    scope: &[String],
) -> ScopedGroupContent {
    let mut content = ScopedGroupContent::default();
    let mut idx = start + 1;
    while idx < end_idx {
        let line = lines[idx].0.trim();
        let lower = line.to_ascii_lowercase();
        if line.is_empty() || line == "}" {
            idx += 1;
            continue;
        }
        if let Some(StatementKind::FamilyRelation(rel)) =
            parse_family_relation(line, Some(DiagramKind::Component))
        {
            content.relations.push(qualify_scoped_relation(rel, scope));
            idx += 1;
            continue;
        }
        if (lower.starts_with("package ")
            || lower.starts_with("namespace ")
            || lower.starts_with("node ")
            || lower.starts_with("frame ")
            || lower.starts_with("cloud ")
            || lower.starts_with("rectangle "))
            && line.trim_end().ends_with('{')
        {
            let keyword = [
                "package",
                "namespace",
                "node",
                "frame",
                "cloud",
                "rectangle",
            ]
            .into_iter()
            .find(|kw| lower.starts_with(&format!("{kw} ")))
            .unwrap_or("package");
            let label = clean_ident(
                line[keyword.len()..]
                    .trim()
                    .trim_end_matches('{')
                    .trim()
                    .trim_matches('"'),
            );
            let nested_end = find_scoping_block_end(lines, idx);
            if nested_end > idx {
                let mut nested_scope = scope.to_vec();
                if !label.is_empty() {
                    nested_scope.push(label);
                }
                let nested =
                    collect_scoped_component_group_content(lines, idx, nested_end, &nested_scope);
                content.members.extend(nested.members);
                content.relations.extend(nested.relations);
                idx = nested_end + 1;
                continue;
            }
        }
        if let Some(StatementKind::ComponentDecl {
            kind,
            name,
            alias,
            label,
            members,
            ..
        }) = parse_component_decl(line)
        {
            let fill_color = members.iter().find_map(|member| {
                member
                    .text
                    .strip_prefix("\x1fstyle:fill:")
                    .map(str::to_string)
            });
            let local_id = alias.clone().unwrap_or_else(|| name.clone());
            let scoped_id = qualify_scoped_identifier(local_id, scope);
            let display = label
                .or_else(|| alias.as_ref().map(|_| name.clone()))
                .or_else(|| (scoped_id != name).then(|| name.clone()))
                .filter(|value| value != &scoped_id);
            let display = append_component_declaration_metadata(display, &members);
            let mut encoded = scoped_id;
            if let Some(display) = display {
                encoded.push('\t');
                encoded.push_str(&display);
            }
            encoded.push('\t');
            encoded.push_str(component_decl_kind_name(kind));
            if let Some(fill_color) = fill_color {
                encoded.push('\t');
                encoded.push_str(&format!("\x1fstyle:fill:{fill_color}"));
            }
            content.members.push(encoded);
        } else {
            let name = extract_component_group_member_name(line);
            if !name.is_empty() {
                content.members.push(qualify_scoped_identifier(name, scope));
            }
        }
        if line.ends_with('{') {
            let nested_end = find_family_decl_end(lines, idx);
            if nested_end > idx {
                idx = nested_end + 1;
                continue;
            }
        }
        idx += 1;
    }
    content
}

fn component_decl_kind_name(kind: ComponentNodeKind) -> &'static str {
    match kind {
        ComponentNodeKind::Component => "component",
        ComponentNodeKind::Interface => "interface",
        ComponentNodeKind::Port => "port",
        ComponentNodeKind::Node => "node",
        ComponentNodeKind::Artifact => "artifact",
        ComponentNodeKind::Cloud => "cloud",
        ComponentNodeKind::Frame => "frame",
        ComponentNodeKind::Storage => "storage",
        ComponentNodeKind::Database => "database",
        ComponentNodeKind::Package => "package",
        ComponentNodeKind::Rectangle => "rectangle",
        ComponentNodeKind::Folder => "folder",
        ComponentNodeKind::File => "file",
        ComponentNodeKind::Card => "card",
        ComponentNodeKind::Actor => "actor",
    }
}

fn append_component_declaration_metadata(
    display: Option<String>,
    members: &[ClassMember],
) -> Option<String> {
    let stereotypes = members
        .iter()
        .map(|member| member.text.trim())
        .filter(|text| text.starts_with("<<") && text.ends_with(">>"))
        .collect::<Vec<_>>();
    if stereotypes.is_empty() {
        return display;
    }
    let mut label = display.unwrap_or_default();
    if !label.is_empty() {
        label.push(' ');
    }
    label.push_str(&stereotypes.join(" "));
    Some(label)
}

fn find_scoping_block_end(lines: &[(&str, Span)], start: usize) -> usize {
    let mut depth = 0usize;
    for (idx, (raw, _)) in lines.iter().enumerate().skip(start) {
        let trimmed = strip_inline_plantuml_comment(raw).trim();
        if trimmed.ends_with('{') {
            depth += 1;
        }
        if trimmed == "}" {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                return idx;
            }
        }
    }
    start
}

fn detect_non_sequence_family(line: &str) -> Option<DiagramKind> {
    if line.starts_with("component ")
        || line.starts_with("interface ")
        || line.starts_with("port ")
        || line.starts_with("portin ")
        || line.starts_with("portout ")
        || line.starts_with("package ")
        || line.starts_with("rectangle ")
        || line.starts_with("folder ")
        || line.starts_with("file ")
        || line.starts_with("card ")
        || line.starts_with("actor ")
    {
        return Some(DiagramKind::Component);
    }

    if line.starts_with("node ")
        || line.starts_with("artifact ")
        || line.starts_with("cloud ")
        || line.starts_with("frame ")
        || line.starts_with("storage ")
        || line.starts_with("database ")
    {
        return Some(DiagramKind::Deployment);
    }

    if line.starts_with("state ") || line == "[*]" || line == "[H]" || line == "[H*]" {
        return Some(DiagramKind::State);
    }
    // State transitions involving pseudo-states
    if (line.starts_with("[*]") || line.starts_with("[H]") || line.starts_with("[H*]"))
        && line.contains("-->")
    {
        return Some(DiagramKind::State);
    }
    // Any line that is `X --> Y` where Y is `[*]`, `[H]`, or `[H*]`
    if line.contains("-->") {
        if let Some(idx) = line.find("-->") {
            let rhs = line[idx + 3..].trim();
            // Strip label part
            let rhs_base = rhs.split(':').next().unwrap_or(rhs).trim();
            if matches!(rhs_base, "[*]" | "[H]" | "[H*]") {
                return Some(DiagramKind::State);
            }
        }
    }

    if line.starts_with('*')
        || line.starts_with('+')
        || line.starts_with('-')
        || line.starts_with('#')
    {
        return Some(DiagramKind::MindMap);
    }

    if line.starts_with("wbs ") {
        return Some(DiagramKind::Wbs);
    }

    if line.starts_with("start")
        || line.starts_with("stop")
        || line.starts_with(':')
        || line.starts_with("(*)")
        || line.starts_with("if ")
        || line.starts_with("elseif ")
        || line == "else"
        || line.starts_with("endif")
        || line.starts_with("switch ")
        || line.starts_with("case ")
        || line.starts_with("endswitch")
        || line.starts_with("repeat")
        || line.starts_with("while ")
        || line.starts_with("fork")
        || line.starts_with("split")
        || line.starts_with("end split")
        || line.starts_with("kill")
        || line.starts_with("break")
        || line.starts_with("continue")
        || line.starts_with("label ")
        || line.starts_with("goto ")
        || line.starts_with("backward")
        || line.starts_with("partition ")
        || line.starts_with("swimlane ")
        || line.starts_with('|')
        || line.starts_with("detach")
    {
        return Some(DiagramKind::Activity);
    }

    if line.starts_with("robust ")
        || line.starts_with("concise ")
        || line.starts_with("clock ")
        || line.starts_with("binary ")
        || line.starts_with('@')
        // Timing-specific scale syntax: "scale N as N" (maps clock units to pixels).
        // Plain "scale 1.5" / "scale 800*600" / "scale max N" is the output-scale
        // directive and should not be classified as a timing diagram.
        || (line.starts_with("scale ") && line.contains(" as "))
    {
        return Some(DiagramKind::Timing);
    }
    if line.starts_with("salt ") {
        return Some(DiagramKind::Salt);
    }

    if line.starts_with("salt ") {
        return Some(DiagramKind::Salt);
    }

    None
}

fn parse_gantt_baseline_statement(line: &str) -> Option<StatementKind> {
    let trimmed = line.trim();
    if let Some(kind) = parse_keyword(trimmed) {
        return Some(kind);
    }

    if let Some(scale) = parse_gantt_scale_directive(trimmed) {
        return Some(StatementKind::GanttConstraint {
            subject: "Project".to_string(),
            kind: "scale".to_string(),
            target: scale,
        });
    }
    if let Some(rest) = trimmed.strip_prefix("Project starts ") {
        let date = rest
            .trim()
            .strip_prefix("on ")
            .or_else(|| rest.trim().strip_prefix("the "))
            .unwrap_or_else(|| rest.trim())
            .trim();
        if is_iso_date_literal(date) {
            return Some(StatementKind::GanttConstraint {
                subject: "Project".to_string(),
                kind: "starts".to_string(),
                target: date.to_string(),
            });
        }
    }
    if let Some(rest) = trimmed.strip_prefix("Project ends ") {
        let date = rest
            .trim()
            .strip_prefix("on ")
            .or_else(|| rest.trim().strip_prefix("the "))
            .unwrap_or_else(|| rest.trim())
            .trim();
        if is_iso_date_literal(date) {
            return Some(StatementKind::GanttConstraint {
                subject: "Project".to_string(),
                kind: "ends".to_string(),
                target: date.to_string(),
            });
        }
    }
    if let Some((start_date, end_date)) = parse_gantt_closed_date_range(trimmed) {
        return Some(StatementKind::GanttCalendarClosedDateRange {
            start_date,
            end_date,
        });
    }
    if let Some((start_date, end_date)) = parse_gantt_open_date_range(trimmed) {
        return Some(StatementKind::GanttCalendarOpenDateRange {
            start_date,
            end_date,
        });
    }
    if let Some(day) = parse_gantt_closed_weekday(trimmed) {
        return Some(StatementKind::GanttCalendarClosed { day });
    }
    if let Some(day) = parse_gantt_open_weekday(trimmed) {
        return Some(StatementKind::GanttCalendarOpen { day });
    }
    if let Some(label) = parse_gantt_horizontal_separator(trimmed) {
        return Some(StatementKind::Separator(Some(label)));
    }
    if let Some((label, target)) = parse_gantt_vertical_separator(trimmed) {
        return Some(StatementKind::GanttConstraint {
            subject: format!("__separator::{label}"),
            kind: "separator".to_string(),
            target,
        });
    }
    let (subject, rest) = parse_bracket_subject(trimmed)?;
    if rest.is_empty() {
        return Some(StatementKind::GanttTaskDecl {
            name: subject,
            start_date: None,
            duration_days: None,
            depends_on: Vec::new(),
            resources: Vec::new(),
        });
    }
    let rest = rest.trim();
    let (rest_without_resources, resources) = extract_gantt_resources(rest);
    let rest = rest_without_resources.trim();
    if let Some(rest) = rest.strip_prefix(':') {
        if subject.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            let label = rest.trim();
            if !label.is_empty() {
                return Some(StatementKind::GanttMilestoneDecl {
                    name: label.to_string(),
                    happens_on: Some(subject),
                });
            }
        }
        return Some(StatementKind::GanttTaskDecl {
            name: rest.trim().to_string(),
            start_date: None,
            duration_days: None,
            depends_on: Vec::new(),
            resources,
        });
    }
    if let Some((start_date, duration_days)) = parse_gantt_start_and_duration(rest) {
        return Some(StatementKind::GanttTaskDecl {
            name: subject,
            start_date: Some(start_date),
            duration_days: Some(duration_days),
            depends_on: Vec::new(),
            resources,
        });
    }
    if let Some(duration_days) = parse_gantt_duration_clause(rest) {
        return Some(StatementKind::GanttTaskDecl {
            name: subject,
            start_date: None,
            duration_days: Some(duration_days),
            depends_on: Vec::new(),
            resources,
        });
    }
    if let Some(start_date) = parse_gantt_start_date_clause(rest) {
        return Some(StatementKind::GanttTaskDecl {
            name: subject,
            start_date: Some(start_date),
            duration_days: None,
            depends_on: Vec::new(),
            resources,
        });
    }
    if !resources.is_empty() {
        return Some(StatementKind::GanttTaskDecl {
            name: subject,
            start_date: None,
            duration_days: None,
            depends_on: Vec::new(),
            resources,
        });
    }
    let rest = rest.trim();
    if let Some(rest) = rest.strip_prefix(':') {
        return Some(StatementKind::GanttTaskDecl {
            name: rest.trim().to_string(),
            start_date: None,
            duration_days: None,
            depends_on: Vec::new(),
            resources: Vec::new(),
        });
    }
    let lower = rest.to_ascii_lowercase();
    if matches!(
        lower.as_str(),
        "is critical" | "critical" | "is on critical path" | "is on the critical path"
    ) {
        return Some(StatementKind::GanttConstraint {
            subject,
            kind: "critical".to_string(),
            target: "true".to_string(),
        });
    }
    if let Some(target) = lower
        .strip_prefix("baseline ")
        .and_then(|_| rest.get("baseline ".len()..))
        .or_else(|| {
            lower
                .strip_prefix("has baseline ")
                .and_then(|_| rest.get("has baseline ".len()..))
        })
        .or_else(|| {
            lower
                .strip_prefix("planned ")
                .and_then(|_| rest.get("planned ".len()..))
        })
    {
        return Some(StatementKind::GanttConstraint {
            subject,
            kind: "baseline".to_string(),
            target: target.trim().to_string(),
        });
    }
    if lower.starts_with("happens") {
        return Some(StatementKind::GanttMilestoneDecl {
            name: subject,
            happens_on: parse_gantt_happens_target(rest),
        });
    }
    for kind in ["starts", "ends", "requires"] {
        if lower.starts_with(kind) {
            let target = rest[kind.len()..]
                .trim()
                .strip_prefix("at ")
                .unwrap_or_else(|| rest[kind.len()..].trim())
                .trim()
                .to_string();
            return Some(StatementKind::GanttConstraint {
                subject,
                kind: kind.to_string(),
                target,
            });
        }
    }
    None
}

fn parse_gantt_closed_weekday(line: &str) -> Option<String> {
    parse_gantt_weekday_status(line, "closed")
}

fn parse_gantt_open_weekday(line: &str) -> Option<String> {
    parse_gantt_weekday_status(line, "open")
        .or_else(|| parse_gantt_weekday_status(line, "opened"))
        .or_else(|| parse_gantt_weekday_status(line, "reopened"))
}

fn parse_gantt_weekday_status(line: &str, status: &str) -> Option<String> {
    let lower = line.trim().to_ascii_lowercase();
    let day = [
        "monday",
        "tuesday",
        "wednesday",
        "thursday",
        "friday",
        "saturday",
        "sunday",
    ]
    .into_iter()
    .find(|day| {
        lower == format!("{day} is {status}")
            || lower == format!("{day} are {status}")
            || lower == format!("{day}s are {status}")
    })?;
    Some(day.to_string())
}

fn parse_gantt_closed_date_range(line: &str) -> Option<(String, String)> {
    parse_gantt_date_range_status(line, &[" is closed", " are closed"])
}

fn parse_gantt_open_date_range(line: &str) -> Option<(String, String)> {
    parse_gantt_date_range_status(
        line,
        &[
            " is open",
            " are open",
            " is opened",
            " are opened",
            " is reopened",
            " are reopened",
        ],
    )
}

fn parse_gantt_date_range_status(line: &str, suffixes: &[&str]) -> Option<(String, String)> {
    let trimmed = line.trim();
    let lower = trimmed.to_ascii_lowercase();
    let suffix_len = suffixes
        .iter()
        .find(|suffix| lower.ends_with(**suffix))
        .map(|suffix| suffix.len())?;
    let range = trimmed[..trimmed.len().saturating_sub(suffix_len)].trim();
    let lower_range = lower[..lower.len().saturating_sub(suffix_len)].trim();
    let sep = " to ";
    let (start_date, end_date) = if let Some(idx) = lower_range.find(sep) {
        (range[..idx].trim(), range[idx + sep.len()..].trim())
    } else {
        (range, range)
    };
    if !is_iso_date_literal(start_date) || !is_iso_date_literal(end_date) {
        return None;
    }
    Some((start_date.to_string(), end_date.to_string()))
}

fn parse_gantt_scale_directive(line: &str) -> Option<String> {
    let trimmed = line.trim();
    let lower = trimmed.to_ascii_lowercase();
    let value = lower
        .strip_prefix("printscale ")
        .or_else(|| lower.strip_prefix("scale "))?
        .trim();
    let normalized = match value {
        "daily" | "day" | "days" => "daily",
        "weekly" | "week" | "weeks" => "weekly",
        "monthly" | "month" | "months" => "monthly",
        "quarterly" | "quarter" | "quarters" => "quarterly",
        "yearly" | "year" | "years" => "yearly",
        _ => return None,
    };
    Some(normalized.to_string())
}

fn parse_gantt_vertical_separator(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim();
    let lower = trimmed.to_ascii_lowercase();
    let rest = lower
        .strip_prefix("separator just ")
        .and_then(|_| trimmed.get("Separator just ".len()..))
        .or_else(|| {
            lower
                .strip_prefix("separator ")
                .and_then(|_| trimmed.get("Separator ".len()..))
        })?
        .trim();
    if rest.is_empty() {
        return None;
    }
    Some(("Separator".to_string(), rest.to_string()))
}

fn parse_gantt_horizontal_separator(line: &str) -> Option<String> {
    let trimmed = line.trim();
    let inner = trimmed.strip_prefix("--")?.strip_suffix("--")?.trim();
    Some(if inner.is_empty() {
        "Separator".to_string()
    } else {
        inner.to_string()
    })
}

fn parse_gantt_start_and_duration(rest: &str) -> Option<(String, u32)> {
    let lower = rest.to_ascii_lowercase();
    let (idx, marker_len) = lower
        .find(" and lasts ")
        .map(|idx| (idx, " and lasts ".len()))
        .or_else(|| {
            lower
                .find(" and requires ")
                .map(|idx| (idx, " and requires ".len()))
        })?;
    let start_clause = rest[..idx].trim();
    let duration_clause = rest[idx + marker_len..].trim();
    let start_date = parse_gantt_start_date_clause(start_clause)?;
    Some((start_date, parse_gantt_duration_clause(duration_clause)?))
}

fn parse_gantt_start_date_clause(rest: &str) -> Option<String> {
    let start_date = rest
        .trim()
        .strip_prefix("starts ")?
        .trim()
        .strip_prefix("at ")
        .unwrap_or_else(|| rest.trim().strip_prefix("starts ").unwrap().trim())
        .trim();
    if !is_iso_date_literal(start_date) {
        return None;
    }
    Some(start_date.to_string())
}

fn parse_gantt_duration_clause(rest: &str) -> Option<u32> {
    let trimmed = rest.trim();
    let clause = trimmed
        .strip_prefix("lasts ")
        .or_else(|| trimmed.strip_prefix("requires "))
        .map(str::trim)
        .unwrap_or(trimmed);
    let mut total = 0u32;
    let mut parts = clause.split_whitespace().peekable();
    while parts.peek().is_some() {
        if parts.peek().copied() == Some("and") {
            parts.next();
            continue;
        }
        let n = parts.next()?.parse::<u32>().ok()?;
        let unit = parts.next()?.to_ascii_lowercase();
        let days = match unit.as_str() {
            "day" | "days" => n,
            "week" | "weeks" => n.saturating_mul(7),
            "month" | "months" => n.saturating_mul(30),
            _ => return None,
        };
        total = total.saturating_add(days);
    }
    if total == 0 {
        None
    } else {
        Some(total)
    }
}

fn extract_gantt_resources(rest: &str) -> (String, Vec<String>) {
    let lower = rest.to_ascii_lowercase();
    let Some(on_idx) = lower
        .find(" on {")
        .or_else(|| lower.strip_prefix("on {").map(|_| 0))
    else {
        return (rest.to_string(), Vec::new());
    };
    let mut cursor = if on_idx == 0 {
        "on ".len()
    } else {
        on_idx + " on ".len()
    };
    let mut resources = Vec::new();
    while cursor < rest.len() {
        let skipped = rest[cursor..].len() - rest[cursor..].trim_start().len();
        cursor += skipped;
        if !rest[cursor..].starts_with('{') {
            break;
        }
        let Some(end_rel) = rest[cursor + 1..].find('}') else {
            break;
        };
        let end = cursor + 1 + end_rel;
        let resource = rest[cursor + 1..end].trim();
        for resource in resource
            .split(',')
            .map(str::trim)
            .filter(|resource| !resource.is_empty())
        {
            resources.push(resource.to_string());
        }
        cursor = end + 1;
    }
    if resources.is_empty() {
        return (rest.to_string(), Vec::new());
    }
    let prefix = rest[..on_idx].trim_end();
    let suffix = rest[cursor..]
        .trim_start()
        .strip_prefix("and ")
        .unwrap_or_else(|| rest[cursor..].trim_start())
        .trim_start();
    let cleaned = if prefix.is_empty() {
        suffix.to_string()
    } else if suffix.is_empty() {
        prefix.to_string()
    } else {
        format!("{prefix} {suffix}")
    };
    (cleaned, resources)
}

fn parse_gantt_happens_target(rest: &str) -> Option<String> {
    let lower = rest.to_ascii_lowercase();
    let target = lower
        .strip_prefix("happens on ")
        .and_then(|_| rest.get("happens on ".len()..))
        .or_else(|| {
            lower
                .strip_prefix("happens at ")
                .and_then(|_| rest.get("happens at ".len()..))
        })?
        .trim();
    if target.is_empty() {
        None
    } else {
        Some(target.to_string())
    }
}

fn is_iso_date_literal(raw: &str) -> bool {
    let mut parts = raw.trim().split('-');
    let Some(y) = parts.next() else {
        return false;
    };
    let Some(m) = parts.next() else {
        return false;
    };
    let Some(d) = parts.next() else {
        return false;
    };
    if parts.next().is_some() {
        return false;
    }
    if y.len() != 4 || m.len() != 2 || d.len() != 2 {
        return false;
    }
    y.chars().all(|c| c.is_ascii_digit())
        && m.chars().all(|c| c.is_ascii_digit())
        && d.chars().all(|c| c.is_ascii_digit())
}

fn parse_component_decl(line: &str) -> Option<StatementKind> {
    use crate::ast::ComponentNodeKind as K;
    let keywords: &[(&str, K)] = &[
        ("component", K::Component),
        ("interface", K::Interface),
        ("portin", K::Port),
        ("portout", K::Port),
        ("port", K::Port),
        ("node", K::Node),
        ("database", K::Database),
        ("cloud", K::Cloud),
        ("frame", K::Frame),
        ("storage", K::Storage),
        ("package", K::Package),
        ("rectangle", K::Rectangle),
        ("folder", K::Folder),
        ("file", K::File),
        ("card", K::Card),
        ("artifact", K::Artifact),
        ("actor", K::Actor),
    ];
    for (kw, kind) in keywords.iter().copied() {
        let trimmed = line.trim();
        if !trimmed.starts_with(kw) {
            continue;
        }
        let rest_raw = trimmed[kw.len()..].trim();
        if rest_raw.is_empty() {
            return None;
        }
        if looks_like_family_relation_tail(rest_raw) {
            continue;
        }
        if rest_raw.starts_with('-') || rest_raw.starts_with('.') || rest_raw.starts_with('<') {
            return None;
        }
        // Must be followed by whitespace OR the rest is a non-identifier prefix; require space.
        if !line
            .as_bytes()
            .get(kw.len())
            .copied()
            .is_some_and(|b| b == b' ' || b == b'\t')
        {
            // For the very first char after kw, ensure it's whitespace.
            // (line is already trimmed by caller; recompute on trimmed)
            let bytes = trimmed.as_bytes();
            if let Some(&b) = bytes.get(kw.len()) {
                if !(b == b' ' || b == b'\t') {
                    continue;
                }
            }
        }
        let rest = rest_raw.trim_end_matches('{').trim();
        let (rest, fill_color) = split_declaration_inline_fill(rest);
        let rest = rest.trim();
        let (rest_without_stereotypes, stereotypes) = strip_declaration_stereotypes(rest);
        let rest = rest_without_stereotypes.trim();
        let (label, rest_after_label) = if rest.starts_with('"') {
            let stripped = rest.strip_prefix('"')?;
            let end = stripped.find('"')?;
            (
                Some(stripped[..end].to_string()),
                stripped[end + 1..].trim(),
            )
        } else if rest.starts_with('[') {
            let stripped = rest.strip_prefix('[')?;
            let end = stripped.find(']')?;
            (
                Some(stripped[..end].trim().to_string()),
                stripped[end + 1..].trim(),
            )
        } else {
            (None, rest)
        };
        let (name_raw, alias_raw) = if let Some(alias) = rest_after_label.strip_prefix("as ") {
            (label.as_deref().unwrap_or("").trim(), Some(alias.trim()))
        } else if let Some((lhs, rhs)) = rest_after_label.split_once(" as ") {
            (lhs.trim(), Some(rhs.trim()))
        } else if rest_after_label.is_empty() {
            (label.as_deref().unwrap_or("").trim(), None)
        } else {
            (rest_after_label, None)
        };
        let name = clean_bracketed_ident(name_raw);
        if name.is_empty() {
            return None;
        }
        let alias = alias_raw.map(clean_ident).filter(|v| !v.is_empty());
        let mut members = declaration_marker_members(None, stereotypes);
        match kw {
            "portin" => members.push(ClassMember {
                text: "<<portin>>".to_string(),
                modifier: None,
            }),
            "portout" => members.push(ClassMember {
                text: "<<portout>>".to_string(),
                modifier: None,
            }),
            _ => {}
        }
        append_inline_fill_member(&mut members, fill_color);
        return Some(StatementKind::ComponentDecl {
            kind,
            name,
            alias,
            label,
            members,
        });
    }
    // Anonymous shorthand: `[Name]` declares a component, `() Name` declares an interface.
    let trimmed = line.trim();
    if let Some(rest) = trimmed.strip_prefix('[') {
        if let Some(end) = rest.find(']') {
            let inner = rest[..end].trim();
            let suffix = rest[end + 1..].trim();
            if !suffix.is_empty() && !suffix.starts_with("as ") {
                return None;
            }
            let bracketed_inner = format!("[{inner}]");
            if normalize_virtual_endpoint(&bracketed_inner).is_some()
                || matches!(inner, "*" | "H" | "H*")
            {
                return None;
            }
            let alias = suffix
                .strip_prefix("as ")
                .map(str::trim)
                .map(clean_ident)
                .filter(|v| !v.is_empty());
            if !inner.is_empty() && !inner.contains('[') && !inner.contains(']') {
                let name = alias.clone().unwrap_or_else(|| clean_ident(inner));
                let label = alias.as_ref().map(|_| inner.to_string());
                return Some(StatementKind::ComponentDecl {
                    kind: ComponentNodeKind::Component,
                    name,
                    alias,
                    label,
                    members: Vec::new(),
                });
            }
        }
    }
    if let Some(rest) = trimmed.strip_prefix("()") {
        let rest = rest.trim();
        if !rest.is_empty() {
            let (label, rest_after_label) = if rest.starts_with('"') {
                let stripped = rest.strip_prefix('"')?;
                let end = stripped.find('"')?;
                (
                    Some(stripped[..end].to_string()),
                    stripped[end + 1..].trim(),
                )
            } else {
                (None, rest)
            };
            let (name_raw, alias) = if let Some(alias) = rest_after_label.strip_prefix("as ") {
                (
                    label.as_deref().unwrap_or("").trim(),
                    Some(clean_ident(alias.trim())),
                )
            } else if let Some((lhs, rhs)) = rest_after_label.split_once(" as ") {
                (lhs.trim(), Some(clean_ident(rhs.trim())))
            } else {
                (rest_after_label, None)
            };
            let name = alias
                .clone()
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| clean_ident(name_raw));
            if !name.is_empty() {
                return Some(StatementKind::ComponentDecl {
                    kind: ComponentNodeKind::Interface,
                    name,
                    alias: alias.filter(|v| !v.is_empty()),
                    label,
                    members: Vec::new(),
                });
            }
        }
    }
    if let Some(inner) = trimmed.strip_prefix('[').and_then(|v| v.strip_suffix(']')) {
        let bracketed_inner = format!("[{inner}]");
        if normalize_virtual_endpoint(&bracketed_inner).is_none()
            && !matches!(inner, "*" | "H" | "H*")
            && !inner.is_empty()
            && !inner.contains('[')
            && !inner.contains(']')
        {
            return Some(StatementKind::ComponentDecl {
                kind: ComponentNodeKind::Component,
                name: clean_ident(inner),
                alias: None,
                label: None,
                members: Vec::new(),
            });
        }
    }
    if let Some(rest) = trimmed.strip_prefix("()") {
        let rest = rest.trim();
        if !rest.is_empty() {
            return Some(StatementKind::ComponentDecl {
                kind: ComponentNodeKind::Interface,
                name: clean_ident(rest),
                alias: None,
                label: None,
                members: Vec::new(),
            });
        }
    }
    None
}

fn looks_like_family_relation_tail(rest: &str) -> bool {
    rest.contains("--")
        || rest.contains("..")
        || rest.contains("->")
        || rest.contains("<-")
        || rest.contains("-[")
        || rest.contains(".[")
}

fn parse_activity_step(line: &str) -> Option<StatementKind> {
    let trimmed = line.trim();
    if let Some(label) = parse_activity_swimlane(trimmed) {
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::PartitionStart,
            label: Some(label),
        }));
    }
    if let Some(label) = parse_activity_colored_action(trimmed) {
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::Action,
            label: Some(label),
        }));
    }
    // `:action;` or `:action` form
    if let Some(rest) = trimmed.strip_prefix(':') {
        let body = rest.trim_end_matches(';').trim();
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::Action,
            label: if body.is_empty() {
                None
            } else {
                Some(body.to_string())
            },
        }));
    }
    if trimmed == "start" {
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::Start,
            label: None,
        }));
    }
    if trimmed == "stop" {
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::Stop,
            label: None,
        }));
    }
    if trimmed == "end" {
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::End,
            label: None,
        }));
    }
    if trimmed == "else" || trimmed.starts_with("else ") || trimmed.starts_with("else(") {
        let label = if trimmed == "else" {
            None
        } else {
            extract_paren_label(trimmed.trim_start_matches("else").trim())
        };
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::Else,
            label,
        }));
    }
    if trimmed == "endif" {
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::EndIf,
            label: None,
        }));
    }
    if trimmed == "fork" {
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::Fork,
            label: None,
        }));
    }
    if trimmed == "fork again" {
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::ForkAgain,
            label: None,
        }));
    }
    if trimmed == "end fork" || trimmed == "endfork" {
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::EndFork,
            label: None,
        }));
    }
    if trimmed == "split" {
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::Fork,
            label: Some("split".to_string()),
        }));
    }
    if trimmed == "split again" {
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::ForkAgain,
            label: Some("split again".to_string()),
        }));
    }
    if trimmed == "end split" || trimmed == "endsplit" || trimmed == "end merge" {
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::EndFork,
            label: Some("end split".to_string()),
        }));
    }
    if trimmed == "endwhile" || trimmed == "end while" {
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::EndWhile,
            label: None,
        }));
    }
    if trimmed == "repeat" {
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::RepeatStart,
            label: None,
        }));
    }
    if let Some(rest) = trimmed.strip_prefix("if ") {
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::IfStart,
            label: Some(parse_activity_if_label(rest.trim())),
        }));
    }
    if let Some(rest) = trimmed.strip_prefix("switch ") {
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::IfStart,
            label: Some(format!(
                "switch {}",
                extract_paren_label(rest.trim()).unwrap_or_else(|| rest.trim().to_string())
            )),
        }));
    }
    if let Some(rest) = trimmed.strip_prefix("case ") {
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::Else,
            label: extract_paren_label(rest.trim()).or_else(|| Some(rest.trim().to_string())),
        }));
    }
    if trimmed == "endswitch" || trimmed == "end switch" {
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::EndIf,
            label: Some("endswitch".to_string()),
        }));
    }
    if let Some(rest) = trimmed.strip_prefix("elseif ") {
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::Else,
            label: Some(format!("elseif {}", parse_activity_if_label(rest.trim()))),
        }));
    }
    if let Some(label) = parse_activity_note_step(trimmed) {
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::Action,
            label: Some(label),
        }));
    }
    if let Some(rest) = trimmed.strip_prefix("while ") {
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::WhileStart,
            label: Some(parse_activity_condition_with_branches(rest.trim())),
        }));
    }
    if let Some(rest) = trimmed.strip_prefix("repeat while") {
        let r = rest.trim();
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::RepeatWhile,
            label: (!r.is_empty()).then(|| parse_activity_condition_with_branches(r)),
        }));
    }
    if trimmed == "end repeat" || trimmed == "endrepeat" {
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::EndWhile,
            label: Some("end repeat".to_string()),
        }));
    }
    if let Some(rest) = trimmed.strip_prefix("partition ") {
        let label = rest.trim().trim_end_matches('{').trim();
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::PartitionStart,
            label: Some(label.to_string()),
        }));
    }
    if trimmed == "}" {
        // Treat lone `}` inside activity as partition close.
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::PartitionEnd,
            label: None,
        }));
    }
    if let Some(rest) = trimmed.strip_prefix("label ") {
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::Action,
            label: Some(format!("label {}", rest.trim())),
        }));
    }
    if let Some(rest) = trimmed.strip_prefix("goto ") {
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::Action,
            label: Some(format!("goto {}", rest.trim())),
        }));
    }
    if let Some(rest) = trimmed.strip_prefix("backward") {
        let label = rest
            .trim()
            .trim_start_matches(':')
            .trim_end_matches(';')
            .trim();
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::Action,
            label: Some(if label.is_empty() {
                "backward".to_string()
            } else {
                format!("backward {label}")
            }),
        }));
    }
    if trimmed == "kill" || trimmed == "detach" {
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::Stop,
            label: Some(trimmed.to_string()),
        }));
    }
    if trimmed == "break" || trimmed == "continue" {
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::Action,
            label: Some(trimmed.to_string()),
        }));
    }
    None
}

fn looks_like_old_activity_flow(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with("(*)")
        || trimmed.starts_with("-->")
        || (trimmed.contains("-->") && trimmed.contains("(*)"))
}

fn parse_activity_old_style_flow(line: &str) -> Option<Vec<StatementKind>> {
    let trimmed = line.trim();
    let arrow_idx = trimmed.find("-->")?;
    let lhs = trimmed[..arrow_idx].trim();
    let rhs = trimmed[arrow_idx + 3..].trim();
    let mut steps = Vec::new();

    if lhs == "(*)" {
        steps.push(activity_step_statement(ActivityStepKind::Start, None));
    }

    if let Some(target) = parse_old_activity_arrow_target(rhs) {
        if target == "(*)" {
            steps.push(activity_step_statement(ActivityStepKind::Stop, None));
        } else {
            steps.push(activity_step_statement(
                ActivityStepKind::Action,
                Some(target),
            ));
        }
    }

    (!steps.is_empty()).then_some(steps)
}

fn parse_old_activity_arrow_target(rhs: &str) -> Option<String> {
    let mut rest = rhs.trim();
    if let Some(after_label) = rest.strip_prefix('[') {
        let close = after_label.find(']')?;
        rest = after_label[close + 1..].trim();
    }
    if let Some(after_dir) = rest.strip_prefix("right of ") {
        rest = after_dir.trim();
    } else if let Some(after_dir) = rest.strip_prefix("left of ") {
        rest = after_dir.trim();
    } else if let Some(after_dir) = rest.strip_prefix("up of ") {
        rest = after_dir.trim();
    } else if let Some(after_dir) = rest.strip_prefix("down of ") {
        rest = after_dir.trim();
    }

    if rest == "(*)" {
        return Some(rest.to_string());
    }

    if let Some(label) = parse_quoted_activity_label(rest) {
        return Some(label);
    }

    let label = rest.trim_end_matches(';').trim();
    (!label.is_empty()).then(|| label.to_string())
}

fn parse_quoted_activity_label(input: &str) -> Option<String> {
    let input = input.trim();
    let rest = input.strip_prefix('"')?;
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

fn parse_activity_swimlane(line: &str) -> Option<String> {
    if !line.starts_with('|') || !line.ends_with('|') {
        return None;
    }
    let parts: Vec<&str> = line
        .trim_matches('|')
        .split('|')
        .map(str::trim)
        .filter(|part| !part.is_empty() && !part.starts_with('#'))
        .collect();
    parts.last().map(|part| (*part).to_string())
}

fn parse_activity_colored_action(line: &str) -> Option<String> {
    let rest = line.strip_prefix('#')?;
    let (_color, body) = rest.split_once(':')?;
    let label = body.trim().trim_end_matches(';').trim();
    (!label.is_empty()).then(|| label.to_string())
}

fn activity_step_statement(kind: ActivityStepKind, label: Option<String>) -> StatementKind {
    StatementKind::ActivityStep(ActivityStep { kind, label })
}

fn parse_activity_note_step(line: &str) -> Option<String> {
    let lower = line.to_ascii_lowercase();
    let prefixes = [
        "floating note left",
        "floating note right",
        "floating note",
        "note left",
        "note right",
        "note top",
        "note bottom",
    ];
    let prefix = prefixes.iter().find(|prefix| lower.starts_with(**prefix))?;
    let rest = line[prefix.len()..]
        .trim()
        .trim_start_matches(':')
        .trim_end_matches(';')
        .trim();
    let display_prefix = prefix.replace("floating ", "");
    Some(if rest.is_empty() {
        display_prefix
    } else {
        format!("{display_prefix}: {rest}")
    })
}

fn parse_activity_if_label(input: &str) -> String {
    let lower = input.to_ascii_lowercase();
    if let Some(idx) = lower.find(" then ") {
        let condition_raw = input[..idx].trim();
        let then_raw = input[idx + " then ".len()..].trim();
        let condition = parse_activity_condition_with_branches(condition_raw);
        if let Some(branch) = extract_paren_label(then_raw) {
            if !branch.is_empty() {
                return format!("{condition} / {branch}");
            }
        }
        return condition;
    }
    let body = input.trim_end_matches("then").trim();
    extract_paren_label(body).unwrap_or_else(|| body.to_string())
}

fn parse_activity_condition_with_branches(input: &str) -> String {
    let trimmed = input.trim();
    let condition = extract_first_paren_label(trimmed).unwrap_or_else(|| {
        trimmed
            .split_once(" is ")
            .map(|(lhs, _)| lhs.trim())
            .unwrap_or(trimmed)
            .trim_end_matches("then")
            .trim()
            .to_string()
    });
    let mut parts = vec![condition];
    for marker in [" is ", " then ", " not "] {
        if let Some((_, tail)) = trimmed.split_once(marker) {
            if let Some(value) = extract_first_paren_label(tail) {
                if !value.is_empty() {
                    parts.push(value);
                }
            }
        }
    }
    parts.join(" / ")
}

fn extract_first_paren_label(input: &str) -> Option<String> {
    let s = input.trim();
    let open = s.find('(')?;
    let close = s[open + 1..].find(')')? + open + 1;
    if close <= open {
        return None;
    }
    Some(s[open + 1..close].trim().to_string())
}

fn extract_paren_label(input: &str) -> Option<String> {
    let s = input.trim();
    let open = s.find('(')?;
    let close = s.rfind(')')?;
    if close <= open {
        return None;
    }
    Some(s[open + 1..close].trim().to_string())
}

fn parse_timing_decl(line: &str) -> Option<StatementKind> {
    let trimmed = line.trim();
    let kinds: &[(&str, TimingDeclKind)] = &[
        ("concise", TimingDeclKind::Concise),
        ("robust", TimingDeclKind::Robust),
        ("clock", TimingDeclKind::Clock),
        ("binary", TimingDeclKind::Binary),
    ];
    for (kw, kind) in kinds.iter().copied() {
        if let Some(rest) = trimmed.strip_prefix(kw) {
            if !rest.starts_with(char::is_whitespace) {
                continue;
            }
            let rest = rest.trim();
            if rest.is_empty() {
                return None;
            }
            let (label, name_raw) = if rest.starts_with('"') {
                let stripped = rest.strip_prefix('"')?;
                let end = stripped.find('"')?;
                let rem = stripped[end + 1..].trim();
                let name = rem.strip_prefix("as ").map(str::trim).unwrap_or(rem).trim();
                (Some(stripped[..end].to_string()), name)
            } else if let Some((lhs, rhs)) = rest.split_once(" as ") {
                (Some(lhs.trim().to_string()), rhs.trim())
            } else {
                (None, rest)
            };
            let (name_raw, controls) = split_timing_decl_controls(name_raw);
            let name = clean_ident(&name_raw);
            if name.is_empty() {
                return None;
            }
            return Some(StatementKind::TimingDecl {
                kind,
                name,
                label,
                controls,
            });
        }
    }
    None
}

fn split_timing_decl_controls(input: &str) -> (String, Vec<String>) {
    let trimmed = input.trim();
    let lower = trimmed.to_ascii_lowercase();
    if let Some(idx) = lower.find(" with ") {
        let name = trimmed[..idx].trim().to_string();
        let controls = trimmed[idx + " with ".len()..]
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect();
        return (name, controls);
    }
    (trimmed.to_string(), Vec::new())
}

fn parse_timing_event(line: &str) -> Option<StatementKind> {
    let trimmed = line.trim();
    if let Some((start, end, label)) = parse_timing_highlight(trimmed) {
        return Some(StatementKind::TimingEvent {
            time: start,
            signal: None,
            state: None,
            note: Some(format!("range:{end}:{label}")),
        });
    }
    // `@<time>` standalone, or `<signal> is <state>` or `@<time> <signal> is <state>`
    if let Some(rest) = trimmed.strip_prefix('@') {
        let rest = rest.trim();
        if rest.is_empty() {
            return Some(StatementKind::TimingEvent {
                time: String::new(),
                signal: None,
                state: None,
                note: None,
            });
        }
        // split at first whitespace
        let (time, after) = rest
            .split_once(char::is_whitespace)
            .map(|(a, b)| (a.trim().to_string(), b.trim()))
            .unwrap_or_else(|| (rest.to_string(), ""));
        if after.is_empty() {
            return Some(StatementKind::TimingEvent {
                time,
                signal: None,
                state: None,
                note: None,
            });
        }
        if let Some((end, label)) = parse_timing_range_after_time(after) {
            return Some(StatementKind::TimingEvent {
                time,
                signal: None,
                state: None,
                note: Some(format!("range:{end}:{label}")),
            });
        }
        // after may contain "signal is state"
        if let Some((sig, state)) = split_is(after) {
            return Some(StatementKind::TimingEvent {
                time,
                signal: Some(sig),
                state: Some(normalize_timing_state_literal(&state)),
                note: None,
            });
        }
        return Some(StatementKind::TimingEvent {
            time,
            signal: None,
            state: None,
            note: Some(after.to_string()),
        });
    }
    if let Some((time, state)) = parse_timing_oriented_state(trimmed) {
        return Some(StatementKind::TimingEvent {
            time,
            signal: None,
            state: Some(normalize_timing_state_literal(&state)),
            note: None,
        });
    }
    if let Some((sig, state)) = split_is(trimmed) {
        return Some(StatementKind::TimingEvent {
            time: String::new(),
            signal: Some(sig),
            state: Some(normalize_timing_state_literal(&state)),
            note: None,
        });
    }
    None
}

fn parse_timing_oriented_state(line: &str) -> Option<(String, String)> {
    let (time, state) = split_is(line)?;
    if time.trim().is_empty()
        || !time
            .trim()
            .chars()
            .next()
            .is_some_and(|c| c == '+' || c == '-' || c.is_ascii_digit() || c == ':')
    {
        return None;
    }
    Some((time.trim().to_string(), state.trim().to_string()))
}

fn normalize_timing_state_literal(state: &str) -> String {
    let trimmed = state.trim().trim_matches('"').trim();
    let body = trimmed
        .strip_prefix('{')
        .and_then(|v| v.strip_suffix('}'))
        .unwrap_or(trimmed)
        .trim();
    match body.to_ascii_lowercase().as_str() {
        "up" | "hi" | "high" | "on" | "true" => "high".to_string(),
        "down" | "lo" | "low" | "off" | "false" => "low".to_string(),
        _ => body.to_string(),
    }
}

fn parse_timing_range_after_time(after: &str) -> Option<(String, String)> {
    let rest = after.trim().strip_prefix("<->")?.trim();
    let rest = rest.strip_prefix('@').unwrap_or(rest).trim();
    let (end, label) = rest
        .split_once(':')
        .map(|(e, l)| (e.trim(), l.trim()))
        .unwrap_or((rest, ""));
    if end.is_empty() {
        return None;
    }
    Some((end.to_string(), label.trim_matches('"').to_string()))
}

fn parse_timing_highlight(line: &str) -> Option<(String, String, String)> {
    let rest = line.strip_prefix("highlight ")?.trim();
    let lower = rest.to_ascii_lowercase();
    let idx = lower.find(" to ")?;
    let start = rest[..idx].trim().trim_start_matches('@');
    let after = rest[idx + " to ".len()..].trim();
    let (end_part, label) = after
        .split_once(':')
        .map(|(e, l)| (e.trim(), l.trim()))
        .unwrap_or((after, ""));
    let end = end_part
        .split_whitespace()
        .next()
        .unwrap_or("")
        .trim_start_matches('@');
    if start.is_empty() || end.is_empty() {
        return None;
    }
    let label = if label.is_empty() {
        "highlight".to_string()
    } else {
        label.trim_matches('"').to_string()
    };
    Some((start.to_string(), end.to_string(), label))
}

fn split_is(s: &str) -> Option<(String, String)> {
    let needle = " is ";
    let idx = s.find(needle)?;
    let lhs = s[..idx].trim();
    let rhs = s[idx + needle.len()..]
        .trim()
        .trim_matches('"')
        .trim_matches('{')
        .trim_matches('}');
    if lhs.is_empty() || rhs.is_empty() {
        return None;
    }
    Some((lhs.to_string(), rhs.to_string()))
}

fn parse_chronology_baseline_statement(line: &str) -> Option<StatementKind> {
    let trimmed = line.trim();
    if let Some(kind) = parse_keyword(trimmed) {
        return Some(kind);
    }
    let lower = trimmed.to_ascii_lowercase();
    let marker = " happens on ";
    if let Some(idx) = lower.find(marker) {
        let subject = trimmed[..idx].trim().trim_matches('"').to_string();
        let when = trimmed[idx + marker.len()..].trim().to_string();
        if subject.is_empty() || when.is_empty() {
            return None;
        }
        return Some(StatementKind::ChronologyHappensOn { subject, when });
    }
    // Accept ISO `YYYY-MM-DD : Label` shorthand
    if let Some((lhs, rhs)) = trimmed.split_once(':') {
        let when = lhs.trim();
        let subject = rhs.trim().trim_matches('"');
        if !when.is_empty()
            && !subject.is_empty()
            && when.chars().next().is_some_and(|c| c.is_ascii_digit())
        {
            return Some(StatementKind::ChronologyHappensOn {
                subject: subject.to_string(),
                when: when.to_string(),
            });
        }
    }
    None
}

/// Parse a state diagram statement from the current line.
/// Returns `Some((kind, end_index))` where `end_index` is the last consumed line.
fn parse_state_statement(
    lines: &[(&str, Span)],
    start: usize,
    line: &str,
) -> Result<Option<(StatementKind, usize)>, Diagnostic> {
    // Handle common keywords that are valid in any diagram
    if let Some(kind) = parse_keyword(line) {
        return Ok(Some((kind, start)));
    }

    // `[H]` or `[H*]` — history pseudo-states
    if line == "[H]" {
        return Ok(Some((StatementKind::StateHistory { deep: false }, start)));
    }
    if line == "[H*]" {
        return Ok(Some((StatementKind::StateHistory { deep: true }, start)));
    }

    for (keyword, stereotype) in [
        ("choice", "choice"),
        ("fork", "fork"),
        ("join", "join"),
        ("end", "end"),
    ] {
        if let Some(rest) = line.strip_prefix(keyword).map(str::trim) {
            if rest.is_empty() {
                continue;
            }
            let (name_raw, alias) = if let Some((lhs, rhs)) = rest.split_once(" as ") {
                let alias = clean_ident(rhs.trim());
                (
                    clean_ident(lhs.trim()),
                    (!alias.is_empty()).then_some(alias),
                )
            } else {
                (clean_ident(rest), None)
            };
            if !name_raw.is_empty() {
                return Ok(Some((
                    StatementKind::StateDecl(StateDecl {
                        name: name_raw,
                        alias,
                        stereotype: Some(stereotype.to_string()),
                        children: Vec::new(),
                        region_dividers: Vec::new(),
                    }),
                    start,
                )));
            }
        }
    }

    // `state Name` or `state Name <<stereotype>>` or `state Name { ... }`
    if line.starts_with("state ") {
        let rest = line.strip_prefix("state ").unwrap_or("").trim();
        if rest.is_empty() {
            return Ok(None);
        }

        // Extract optional stereotype `<<...>>`
        let (name_part, stereotype) = if let Some(idx) = rest.find("<<") {
            let name = rest[..idx].trim();
            let after = &rest[idx + 2..];
            let stereo = after.find(">>").map(|end| after[..end].trim().to_string());
            (name, stereo)
        } else {
            (rest, None)
        };

        // Check if there's a block
        let (name_alias_part, has_block) = if name_part.ends_with('{') {
            (name_part.trim_end_matches('{').trim(), true)
        } else {
            (name_part, false)
        };

        // Extract alias
        let (name_raw, alias) = if let Some((lhs, rhs)) = name_alias_part.split_once(" as ") {
            let name = clean_ident(lhs.trim());
            let alias = clean_ident(rhs.trim());
            (name, if alias.is_empty() { None } else { Some(alias) })
        } else {
            (clean_ident(name_alias_part), None)
        };

        if name_raw.is_empty() {
            return Ok(None);
        }

        if has_block {
            // Parse nested children until matching `}`
            let (children, region_dividers, end_idx) = parse_state_block(lines, start, &name_raw)?;
            let decl = StateDecl {
                name: name_raw,
                alias,
                stereotype,
                children,
                region_dividers,
            };
            return Ok(Some((StatementKind::StateDecl(decl), end_idx)));
        } else {
            let decl = StateDecl {
                name: name_raw,
                alias,
                stereotype,
                children: Vec::new(),
                region_dividers: Vec::new(),
            };
            return Ok(Some((StatementKind::StateDecl(decl), start)));
        }
    }

    // Transition: `From --> To` or `From --> To : label`
    // Also handles `[*] --> X` and `X --> [*]`
    if let Some(transition) = parse_state_transition(line) {
        return Ok(Some((StatementKind::StateTransition(transition), start)));
    }

    // Internal action: `State : entry / action` or `State : exit / action` or `State : event / action`
    if let Some(action) = parse_state_internal_action(line) {
        return Ok(Some((StatementKind::StateInternalAction(action), start)));
    }

    Ok(None)
}

/// Parse the body of a `state X { ... }` block.
/// Returns (children, region_divider_indices, end_line_index).
fn parse_state_block(
    lines: &[(&str, Span)],
    start: usize,
    parent_state: &str,
) -> Result<(Vec<Statement>, Vec<usize>, usize), Diagnostic> {
    let mut children: Vec<Statement> = Vec::new();
    let mut region_dividers: Vec<usize> = Vec::new();
    let mut depth = 1i32;
    let mut j = start + 1;

    while j < lines.len() {
        let (raw, span) = lines[j];
        let inner = raw.trim();

        if inner.ends_with('{') || inner == "{" {
            depth += 1;
        }
        if inner == "}" {
            depth -= 1;
            if depth == 0 {
                return Ok((children, region_dividers, j));
            }
        }

        // `||` region divider
        if inner == "||" && depth == 1 {
            region_dividers.push(children.len());
            j += 1;
            continue;
        }

        // Recurse for nested state declarations inside a block
        if depth == 1 {
            if inner.is_empty() || inner.starts_with('\'') {
                j += 1;
                continue;
            }
            if inner == "[H]" {
                children.push(Statement {
                    span,
                    kind: StatementKind::StateHistory { deep: false },
                });
                j += 1;
                continue;
            }
            if inner == "[H*]" {
                children.push(Statement {
                    span,
                    kind: StatementKind::StateHistory { deep: true },
                });
                j += 1;
                continue;
            }
            if let Some((kind, end_idx)) = parse_state_statement(lines, j, inner)? {
                children.push(Statement {
                    span: if end_idx > j {
                        Span::new(span.start, lines[end_idx].1.end)
                    } else {
                        span
                    },
                    kind,
                });
                j = end_idx + 1;
                continue;
            }
            if let Some(transition) = parse_state_transition(inner) {
                children.push(Statement {
                    span,
                    kind: StatementKind::StateTransition(transition),
                });
                j += 1;
                continue;
            }
            if let Some(action) = parse_state_internal_action(inner) {
                children.push(Statement {
                    span,
                    kind: StatementKind::StateInternalAction(action),
                });
                j += 1;
                continue;
            }
            if let Some(action) = parse_state_bare_internal_action(parent_state, inner) {
                children.push(Statement {
                    span,
                    kind: StatementKind::StateInternalAction(action),
                });
                j += 1;
                continue;
            }
            if let Some(kind) = parse_keyword(inner) {
                children.push(Statement { span, kind });
                j += 1;
                continue;
            }
            // Unknown line inside block — store for normalizer
            children.push(Statement {
                span,
                kind: StatementKind::Unknown(inner.to_string()),
            });
        }
        j += 1;
    }

    // Unclosed block — treat as if closed at EOF
    Ok((children, region_dividers, lines.len().saturating_sub(1)))
}

/// Parse `From --> To` or `From --> To : label`
fn parse_state_transition(line: &str) -> Option<StateTransition> {
    let (core, label) = split_message_label(line);
    let (from_raw, arrow, relation_style, to_raw) = split_family_arrow_styled(core)?;

    if !arrow.contains('>') || from_raw.is_empty() || to_raw.is_empty() {
        return None;
    }

    Some(StateTransition {
        from: clean_bracketed_ident(from_raw),
        to: clean_bracketed_ident(to_raw),
        label,
        line_color: relation_style.line_color,
        dashed: relation_style.dashed,
        hidden: relation_style.hidden,
        thickness: relation_style.thickness,
        direction: relation_style.direction,
    })
}

/// Parse `State : entry / action` or `State : exit / action` or `State : event / action`
fn parse_state_internal_action(line: &str) -> Option<StateInternalAction> {
    let (state_part, rest) = line.split_once(':')?;
    let state = state_part.trim();
    if state.is_empty() || state.contains("-->") {
        return None;
    }
    // Rest should have form `kind / action` or `kind`
    let rest = rest.trim();
    if rest.is_empty() {
        return None;
    }
    let (kind, action) = if let Some((k, a)) = rest.split_once('/') {
        (k.trim().to_string(), a.trim().to_string())
    } else {
        (rest.to_string(), String::new())
    };
    if kind.is_empty() {
        return None;
    }
    Some(StateInternalAction {
        state: state.to_string(),
        kind,
        action,
    })
}

fn parse_state_bare_internal_action(parent_state: &str, line: &str) -> Option<StateInternalAction> {
    let trimmed = line.trim().trim_end_matches(';').trim();
    if trimmed.is_empty() || trimmed.contains("-->") || trimmed.starts_with("state ") {
        return None;
    }
    let lower = trimmed.to_ascii_lowercase();
    let known_prefix = ["entry", "exit", "do"]
        .into_iter()
        .any(|prefix| lower == prefix || lower.starts_with(&format!("{prefix} /")));
    if !known_prefix {
        return None;
    }
    let (kind, action) = if let Some((k, a)) = trimmed.split_once('/') {
        (k.trim().to_string(), a.trim().to_string())
    } else {
        (trimmed.to_string(), String::new())
    };
    Some(StateInternalAction {
        state: parent_state.to_string(),
        kind,
        action,
    })
}

fn is_timeline_metadata_statement(kind: &StatementKind) -> bool {
    matches!(
        kind,
        StatementKind::Title(_)
            | StatementKind::Header(_)
            | StatementKind::Footer(_)
            | StatementKind::Caption(_)
            | StatementKind::Legend(_)
            | StatementKind::SkinParam { .. }
            | StatementKind::Theme(_)
            | StatementKind::Pragma(_)
    )
}

fn parse_bracket_subject(line: &str) -> Option<(String, &str)> {
    let trimmed = line.trim();
    let stripped = trimmed.strip_prefix('[')?;
    let end = stripped.find(']')?;
    let name = stripped[..end].trim().to_string();
    if name.is_empty() {
        return None;
    }
    let rest = stripped[end + 1..].trim();
    Some((name, rest))
}
fn parse_multiline_keyword_block(
    lines: &[(&str, Span)],
    start: usize,
    line: &str,
) -> Option<(StatementKind, usize)> {
    let lower = line.to_ascii_lowercase();
    // Check for "legend" (alone or with positioning qualifiers: "legend left", etc.)
    let (key, legend_pos) = if lower == "legend" {
        ("legend", None)
    } else if lower.starts_with("legend ") {
        // Collect any position tokens after "legend"
        let pos_part = line[7..].trim();
        let pos_lower = pos_part.to_ascii_lowercase();
        // Verify all tokens are valid positioning keywords
        let all_pos = pos_lower
            .split_whitespace()
            .all(|t| matches!(t, "left" | "right" | "center" | "top" | "bottom"));
        if all_pos && !pos_part.is_empty() {
            ("legend", Some(pos_part.to_string()))
        } else {
            return None;
        }
    } else {
        let k = ["title", "header", "footer", "caption"]
            .into_iter()
            .find(|k| lower.as_str().eq(*k))?;
        (k, None)
    };

    let end_marker = format!("end {key}");
    let mut body = Vec::new();

    for (idx, (raw, _)) in lines.iter().enumerate().skip(start + 1) {
        let trimmed = raw.trim();
        if trimmed.eq_ignore_ascii_case(&end_marker) {
            let text = body.join("\n");
            let kind = match key {
                "title" => StatementKind::Title(text),
                "header" => StatementKind::Header(text),
                "footer" => StatementKind::Footer(text),
                "caption" => StatementKind::Caption(text),
                "legend" => {
                    // Emit Legend first; if there's position info emit LegendPos separately.
                    // We return the Legend text here; the LegendPos is handled by returning
                    // the Legend kind with position info embedded for the caller.
                    // Since we can only return one StatementKind, we pack the pos into the
                    // legend_pos field and handle it via a special kind.
                    let _ = legend_pos; // used below
                    StatementKind::Legend(text)
                }
                _ => StatementKind::Legend(text),
            };
            // If there was a position qualifier alongside the legend text, we need to
            // emit both. We return the Legend kind (which the caller will handle) and
            // separately emit a LegendPos. But since we can only return one statement,
            // we encode the position in a specially-prefixed Legend value when present.
            // Convention: if legend_pos is Some, we prefix the text with "LEGEND_POS:<pos>\n".
            // The normalizer detects and splits this prefix.
            if key == "legend" {
                if let Some(ref pos) = legend_pos {
                    let packed = format!("LEGEND_POS:{}\n{}", pos, body.join("\n"));
                    return Some((StatementKind::Legend(packed), idx));
                }
            }
            return Some((kind, idx));
        }
        body.push(trimmed.to_string());
    }

    None
}

fn parse_multiline_note_block(
    lines: &[(&str, Span)],
    start: usize,
    line: &str,
) -> Option<(StatementKind, usize)> {
    let lower = line.to_ascii_lowercase();
    let note_kw = if lower.starts_with("note ") {
        "note"
    } else if lower.starts_with("hnote ") {
        "hnote"
    } else if lower.starts_with("rnote ") {
        "rnote"
    } else {
        return None;
    };

    let tail = line[note_kw.len()..].trim();
    if tail.is_empty() {
        return None;
    }
    let (head, inline) = tail.split_once(':').unwrap_or((tail, ""));
    let (position, target) = parse_note_head(head.trim());
    if matches!(position.to_ascii_lowercase().as_str(), "left" | "right") && target.is_none() {
        return None;
    }
    let mut body = Vec::new();
    if !inline.trim().is_empty() {
        body.push(inline.trim().to_string());
    }

    for (idx, (raw, _)) in lines.iter().enumerate().skip(start + 1) {
        let trimmed = raw.trim();
        if trimmed.eq_ignore_ascii_case("end note") {
            return Some((
                StatementKind::Note(Note {
                    kind: note_kind_from_keyword(note_kw),
                    position,
                    target,
                    text: body.join("\n"),
                }),
                idx,
            ));
        }
        if note_end_matches(trimmed, note_kw) {
            return Some((
                StatementKind::Note(Note {
                    kind: note_kind_from_keyword(note_kw),
                    position,
                    target,
                    text: body.join("\n"),
                }),
                idx,
            ));
        }
        body.push(trimmed.to_string());
    }

    None
}

fn parse_multiline_ref_block(
    lines: &[(&str, Span)],
    start: usize,
    line: &str,
) -> Option<(StatementKind, usize)> {
    if !line.to_ascii_lowercase().starts_with("ref ") {
        return None;
    }
    let tail = line[4..].trim();
    let (head, inline) = tail.split_once(':').unwrap_or((tail, ""));
    let head = head.trim();
    if head.is_empty() {
        return None;
    }

    let mut body = Vec::new();
    let mut has_non_empty_body = false;
    if !inline.trim().is_empty() {
        body.push(inline.trim().to_string());
        has_non_empty_body = true;
    }
    for (idx, (raw, _)) in lines.iter().enumerate().skip(start + 1) {
        let trimmed = raw.trim();
        if trimmed.eq_ignore_ascii_case("end ref") {
            if !has_non_empty_body {
                return None;
            }
            let mut label = head.to_string();
            label.push('\n');
            label.push_str(&body.join("\n"));
            return Some((
                StatementKind::Group(Group {
                    kind: "ref".to_string(),
                    label: Some(label),
                }),
                idx,
            ));
        }
        if !trimmed.is_empty() {
            has_non_empty_body = true;
        }
        body.push(trimmed.to_string());
    }
    None
}

fn parse_participant(line: &str) -> Option<StatementKind> {
    let roles = [
        ("participant", ParticipantRole::Participant),
        ("actor", ParticipantRole::Actor),
        ("boundary", ParticipantRole::Boundary),
        ("control", ParticipantRole::Control),
        ("entity", ParticipantRole::Entity),
        ("database", ParticipantRole::Database),
        ("collections", ParticipantRole::Collections),
        ("queue", ParticipantRole::Queue),
    ];

    for (kw, role) in roles {
        if !line.starts_with(kw) {
            continue;
        }
        let rest = line[kw.len()..].trim();
        if rest.is_empty() {
            return None;
        }
        let (display, rem) = if let Some(stripped) = rest.strip_prefix('"') {
            let end = stripped.find('"')?;
            (
                Some(stripped[..end].to_string()),
                stripped[end + 1..].trim(),
            )
        } else {
            (None, rest)
        };
        let (rem, order) = split_participant_order(rem);

        let mut alias = None;
        let mut name = rem.to_string();
        if let Some(rhs) = rem.strip_prefix("as ") {
            alias = Some(clean_ident(rhs.trim()));
            name = alias.clone().unwrap_or_default();
        } else if let Some((lhs, rhs)) = rem.split_once(" as ") {
            let lhs = lhs.trim();
            let rhs = rhs.trim();
            if display.is_none() {
                name = lhs.to_string();
            }
            alias = Some(clean_ident(rhs));
        }

        if name.is_empty() {
            name = alias.clone().unwrap_or_default();
        }
        let name = clean_ident(&name);
        let display = display.or_else(|| Some(name.clone()));

        return Some(StatementKind::Participant(ParticipantDecl {
            role,
            name,
            alias,
            display,
            order,
        }));
    }
    None
}

fn split_participant_order(input: &str) -> (&str, Option<i32>) {
    let trimmed = input.trim();
    let mut tokens = trimmed.rsplitn(3, char::is_whitespace);
    let value = tokens.next().unwrap_or("");
    let keyword = tokens.next().unwrap_or("");
    let before = tokens.next().unwrap_or("");
    if keyword.eq_ignore_ascii_case("order") {
        if let Ok(order) = value.parse::<i32>() {
            return (before.trim_end(), Some(order));
        }
    }
    (trimmed, None)
}

fn parse_message(line: &str) -> Option<StatementKind> {
    let (line, parallel) = split_parallel_message_prefix(line);
    let (core, label) = split_message_label(line);
    let (lhs_raw, arrow, rhs_raw) = split_arrow(core)?;
    let mut style = parse_arrow_style(arrow);
    style.parallel = parallel;
    let parsed_arrow = parse_arrow(arrow)?;
    let (from_id_raw, from_modifier) = split_lifecycle_modifier(lhs_raw);
    let (to_id_raw, to_modifier) = split_lifecycle_modifier(rhs_raw);

    let from = if let Some(v) = normalize_virtual_endpoint(from_id_raw) {
        v
    } else {
        if looks_like_virtual_endpoint_syntax(from_id_raw) {
            return None;
        }
        clean_ident(from_id_raw)
    };
    let to = if let Some(v) = normalize_virtual_endpoint(to_id_raw) {
        v
    } else {
        if looks_like_virtual_endpoint_syntax(to_id_raw) {
            return None;
        }
        clean_ident(to_id_raw)
    };

    if from.is_empty() || to.is_empty() {
        return None;
    }

    let mut arrow_encoded = parsed_arrow.to_string();
    if let Some(modifier) = from_modifier {
        arrow_encoded.push_str("@L");
        arrow_encoded.push_str(modifier);
    }
    if let Some(modifier) = to_modifier {
        arrow_encoded.push_str("@R");
        arrow_encoded.push_str(modifier);
    }

    let from_virtual = ast_virtual_endpoint_from_id(&from, true);
    let to_virtual = ast_virtual_endpoint_from_id(&to, false);
    Some(StatementKind::Message(Message {
        from,
        to,
        arrow: arrow_encoded,
        label,
        style,
        from_virtual,
        to_virtual,
    }))
}

fn split_parallel_message_prefix(line: &str) -> (&str, bool) {
    let trimmed = line.trim_start();
    if let Some(rest) = trimmed.strip_prefix('&') {
        let rest = rest.trim_start();
        if !rest.is_empty() {
            return (rest, true);
        }
    }
    (line, false)
}

fn parse_arrow_style(arrow: &str) -> MessageStyle {
    let mut style = MessageStyle::default();
    if strip_sequence_arrow_brackets(arrow).contains('.') {
        style.dotted = true;
    }
    let mut chars = arrow.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '[' {
            continue;
        }
        let mut body = String::new();
        for inner in chars.by_ref() {
            if inner == ']' {
                break;
            }
            body.push(inner);
        }
        for token in body
            .split([',', ';'])
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            let lower = token.to_ascii_lowercase();
            match lower.as_str() {
                "hidden" | "line.hidden" => style.hidden = true,
                "dashed" | "line.dashed" => style.dashed = true,
                "dotted" | "line.dotted" => style.dotted = true,
                "bold" | "thick" | "line.bold" | "line.thick" => style.thickness = Some(3),
                "thin" | "line.thin" => style.thickness = Some(1),
                _ if token.starts_with('#')
                    && matches!(token.len(), 4 | 5 | 7 | 9)
                    && token[1..].bytes().all(|b| b.is_ascii_hexdigit()) =>
                {
                    style.color = Some(format!("#{}", token[1..].to_ascii_lowercase()));
                }
                _ if token.starts_with('#')
                    && token[1..].bytes().all(|b| b.is_ascii_alphabetic()) =>
                {
                    style.color = Some(token[1..].to_ascii_lowercase());
                }
                _ if token.bytes().all(|b| b.is_ascii_alphabetic()) => {
                    style.color = Some(lower);
                }
                _ => {
                    if let Some(value) = lower
                        .strip_prefix("thickness=")
                        .or_else(|| lower.strip_prefix("thickness:"))
                        .or_else(|| lower.strip_prefix("thickness "))
                        .or_else(|| lower.strip_prefix("line.thickness="))
                        .or_else(|| lower.strip_prefix("line.thickness:"))
                        .or_else(|| lower.strip_prefix("line.thickness "))
                    {
                        if let Ok(n) = value.trim().parse::<u8>() {
                            style.thickness = Some(n.clamp(1, 8));
                        }
                    }
                }
            }
        }
    }
    style
}

fn ast_virtual_endpoint_from_id(id: &str, is_from: bool) -> Option<VirtualEndpoint> {
    let (side, kind) = match id {
        "[" => (VirtualEndpointSide::Left, VirtualEndpointKind::Plain),
        "]" => (VirtualEndpointSide::Right, VirtualEndpointKind::Plain),
        "[o" => (VirtualEndpointSide::Left, VirtualEndpointKind::Circle),
        "o]" => (VirtualEndpointSide::Right, VirtualEndpointKind::Circle),
        "[x" => (VirtualEndpointSide::Left, VirtualEndpointKind::Cross),
        "x]" => (VirtualEndpointSide::Right, VirtualEndpointKind::Cross),
        "[*]" => (
            if is_from {
                VirtualEndpointSide::Left
            } else {
                VirtualEndpointSide::Right
            },
            VirtualEndpointKind::Filled,
        ),
        _ => return None,
    };
    Some(VirtualEndpoint { side, kind })
}

fn parse_keyword(line: &str) -> Option<StatementKind> {
    let lower = line.to_ascii_lowercase();

    for k in ["title", "header", "footer", "caption", "legend"] {
        if lower.starts_with(&(k.to_string() + " ")) {
            let v = line[k.len()..].trim().to_string();
            return Some(match k {
                "title" => StatementKind::Title(v),
                "header" => StatementKind::Header(v),
                "footer" => StatementKind::Footer(v),
                "caption" => StatementKind::Caption(v),
                _ => StatementKind::Legend(v),
            });
        }
    }

    if lower.starts_with("skinparam ") {
        let body = line[9..].trim();
        let (key, value) = body.split_once(' ').unwrap_or((body, ""));
        return Some(StatementKind::SkinParam {
            key: key.trim().to_string(),
            value: value.trim().to_string(),
        });
    }
    if lower.starts_with("!theme") {
        return Some(StatementKind::Theme(line[6..].trim().to_string()));
    }
    if lower.starts_with("!pragma") {
        let body = line[7..].trim();
        if body.is_empty() {
            return Some(StatementKind::Unknown(
                "[E_PRAGMA_INVALID] malformed pragma syntax: missing pragma body".to_string(),
            ));
        }
        return Some(StatementKind::Pragma(body.to_string()));
    }

    if lower == "hide footbox" {
        return Some(StatementKind::Footbox(false));
    }
    if lower == "show footbox" {
        return Some(StatementKind::Footbox(true));
    }
    if lower == "hide unlinked" {
        return Some(StatementKind::HideUnlinked);
    }

    // scale directive: "scale <factor>", "scale <w>*<h>", "scale max <n>"
    if lower.starts_with("scale ") {
        let body = line[6..].trim();
        return Some(StatementKind::Scale(body.to_string()));
    }

    // Class-diagram hide options (parsed here so they work before any class decl sets detected_kind)
    if lower.starts_with("hide ") {
        let rest = lower.strip_prefix("hide ").unwrap_or("").trim();
        let class_hide_opts = [
            "circle",
            "stereotype",
            "empty members",
            "empty methods",
            "empty fields",
        ];
        for opt in class_hide_opts {
            if rest == opt {
                return Some(StatementKind::HideOption(rest.to_string()));
            }
        }
    }

    // set namespaceSeparator <sep>
    if lower.starts_with("set namespaceseparator") {
        let rest = line["set namespaceSeparator".len()..].trim();
        return Some(StatementKind::SetOption {
            key: "namespaceSeparator".to_string(),
            value: rest.to_string(),
        });
    }

    let note_kw = if lower.starts_with("note ") {
        Some("note")
    } else if lower.starts_with("hnote ") {
        Some("hnote")
    } else if lower.starts_with("rnote ") {
        Some("rnote")
    } else {
        None
    };
    if let Some(note_kw) = note_kw {
        let tail = line[note_kw.len()..].trim();
        if tail.is_empty() {
            return Some(StatementKind::Unknown(
                "[E_NOTE_INVALID] malformed note syntax: missing note head".to_string(),
            ));
        }
        let (head, text) = tail.split_once(':').unwrap_or((tail, ""));
        let (pos, target) = parse_note_head(head);
        if pos.eq_ignore_ascii_case("of") || !is_valid_note_position(&pos) {
            return Some(StatementKind::Unknown(format!(
                "[E_NOTE_INVALID] malformed note syntax: `{}`",
                line
            )));
        }
        return Some(StatementKind::Note(Note {
            kind: note_kind_from_keyword(note_kw),
            position: pos,
            target,
            text: text.trim().to_string(),
        }));
    }
    if lower.starts_with("ref ") {
        let tail = line[4..].trim();
        let (head, text) = tail.split_once(':').unwrap_or((tail, ""));
        if head.is_empty() || text.trim().is_empty() {
            return Some(StatementKind::Unknown(format!(
                "[E_REF_INVALID] malformed ref syntax: `{}`",
                line
            )));
        }
        let label = format!("{}\n{}", head.trim(), text.trim());
        return Some(StatementKind::Group(Group {
            kind: "ref".to_string(),
            label: Some(label),
        }));
    }

    for g in [
        "alt", "opt", "loop", "par", "critical", "break", "group", "box",
    ] {
        if lower == g || lower.starts_with(&(g.to_string() + " ")) {
            let label = line[g.len()..].trim();
            return Some(StatementKind::Group(Group {
                kind: g.to_string(),
                label: if label.is_empty() {
                    None
                } else {
                    Some(label.to_string())
                },
            }));
        }
    }

    if lower == "else" || lower.starts_with("else ") {
        return Some(StatementKind::Group(Group {
            kind: "else".to_string(),
            label: Some(line[4..].trim().to_string()).filter(|s| !s.is_empty()),
        }));
    }

    if lower == "end" {
        return Some(StatementKind::Group(Group {
            kind: "end".to_string(),
            label: None,
        }));
    }
    if let Some(stripped) = lower.strip_prefix("end ") {
        let tail = stripped.trim();
        if matches!(
            tail,
            "alt" | "opt" | "loop" | "par" | "critical" | "break" | "group" | "ref" | "box"
        ) {
            return Some(StatementKind::Group(Group {
                kind: "end".to_string(),
                label: Some(tail.to_string()),
            }));
        }
    }

    if line == "..." {
        return Some(StatementKind::Spacer(None));
    }
    if lower.starts_with("...") && line.ends_with("...") && line.len() >= 6 {
        return Some(StatementKind::Divider(Some(
            line.trim_matches('.').trim().to_string(),
        )));
    }
    if lower.starts_with("|||") && line.ends_with("|||") {
        let body = line.trim_matches('|').trim();
        return Some(StatementKind::Spacer(
            body.parse::<i32>().ok().map(|n| n.clamp(1, 400)),
        ));
    }
    if lower.starts_with("||") && line.ends_with("||") && line.len() >= 4 {
        return Some(StatementKind::Delay(Some(
            line.trim_matches('|').trim().to_string(),
        )));
    }
    if lower == "||" {
        return Some(StatementKind::Delay(None));
    }
    if line.starts_with("==") && line.ends_with("==") && line.len() >= 4 {
        let label = line[2..line.len() - 2].trim().to_string();
        return Some(if label.is_empty() {
            StatementKind::Separator(None)
        } else {
            StatementKind::Separator(Some(label))
        });
    }
    if lower.starts_with("newpage") {
        return Some(StatementKind::NewPage(line[7..].trim().to_string().into()));
    }
    if lower == "ignore newpage" {
        return Some(StatementKind::IgnoreNewPage);
    }
    if lower.starts_with("autonumber") {
        return Some(StatementKind::Autonumber(
            line[10..].trim().to_string().into(),
        ));
    }

    for (kw, ctor) in [
        (
            "activate",
            StatementKind::Activate as fn(String) -> StatementKind,
        ),
        ("deactivate", StatementKind::Deactivate),
        ("destroy", StatementKind::Destroy),
        ("create", StatementKind::Create),
    ] {
        if lower.starts_with(&(kw.to_string() + " ")) {
            return Some(ctor(clean_ident(line[kw.len()..].trim())));
        }
    }

    if lower == "return" || lower.starts_with("return ") {
        return Some(StatementKind::Return(
            Some(line[6..].trim().to_string()).filter(|s| !s.is_empty()),
        ));
    }

    if lower.starts_with("!include") {
        return Some(StatementKind::Include(line[8..].trim().to_string()));
    }
    if lower.starts_with("!define") {
        let body = line[7..].trim();
        let (name, value) = body.split_once(' ').unwrap_or((body, ""));
        return Some(StatementKind::Define {
            name: name.trim().to_string(),
            value: Some(value.trim().to_string()).filter(|s| !s.is_empty()),
        });
    }
    if lower.starts_with("!undef") {
        return Some(StatementKind::Undef(line[6..].trim().to_string()));
    }

    None
}

fn parse_note_head(head: &str) -> (String, Option<String>) {
    let mut bits = head.split_whitespace();
    let position = bits.next().unwrap_or("over").to_string();
    let rest = bits.collect::<Vec<_>>();
    if rest.is_empty() {
        return (position, None);
    }
    if rest[0].eq_ignore_ascii_case("of") {
        let target = rest[1..].join(" ");
        return (
            position,
            (!target.trim().is_empty()).then(|| clean_ident(target.trim())),
        );
    }
    let target = rest.join(" ");
    (
        position,
        (!target.trim().is_empty()).then(|| clean_ident(target.trim())),
    )
}

fn note_kind_from_keyword(keyword: &str) -> crate::ast::NoteKind {
    match keyword.to_ascii_lowercase().as_str() {
        "hnote" => crate::ast::NoteKind::Hexagonal,
        "rnote" => crate::ast::NoteKind::Rectangle,
        _ => crate::ast::NoteKind::Folded,
    }
}

fn note_end_matches(line: &str, note_keyword: &str) -> bool {
    line.eq_ignore_ascii_case("end note")
        || (note_keyword.eq_ignore_ascii_case("hnote") && line.eq_ignore_ascii_case("endhnote"))
        || (note_keyword.eq_ignore_ascii_case("rnote") && line.eq_ignore_ascii_case("endrnote"))
}

fn is_valid_note_position(position: &str) -> bool {
    matches!(
        position.to_ascii_lowercase().as_str(),
        "left" | "right" | "top" | "bottom" | "over" | "across"
    )
}

fn clean_ident(s: &str) -> String {
    let mut out = s.trim().trim_matches('"').to_string();
    if let Some(rest) = out.strip_prefix("()") {
        out = rest.trim().to_string();
    }
    if let Some(rest) = out.strip_suffix("()") {
        out = rest.trim().to_string();
    }
    for suffix in ["++", "--", "**", "!!"] {
        out = out
            .strip_suffix(suffix)
            .map(str::trim_end)
            .unwrap_or(&out)
            .to_string();
    }
    out
}

/// Extract the class/interface/enum name from a member line inside a package/namespace block.
/// E.g. "class Service" → "Service", "interface IRepo" → "IRepo", "MyClass" → "MyClass".
fn extract_class_member_name(s: &str) -> String {
    let t = s.trim();
    let lower = t.to_ascii_lowercase();
    for kw in &[
        "abstract class ",
        "annotation ",
        "interface ",
        "abstract ",
        "enum ",
        "class ",
        "object ",
        "map ",
        "usecase ",
        "component ",
        "portin ",
        "portout ",
        "port ",
        "node ",
        "database ",
        "cloud ",
        "frame ",
        "storage ",
        "package ",
        "rectangle ",
        "folder ",
        "file ",
        "card ",
        "artifact ",
        "actor ",
    ] {
        if lower.starts_with(kw) {
            // Extract the first identifier token from the original (case-preserved) text
            let name_part = t[kw.len()..].trim();
            let name = name_part
                .split(|c: char| c.is_whitespace() || c == '{')
                .next()
                .unwrap_or("")
                .trim_matches('"');
            return clean_ident(name);
        }
    }
    // Plain identifier (like in a together block)
    clean_ident(t)
}

fn extract_component_group_member_name(s: &str) -> String {
    if let Some(StatementKind::ComponentDecl { name, alias, .. }) = parse_component_decl(s) {
        return alias.unwrap_or(name);
    }
    extract_class_member_name(s)
}

fn split_family_relation_label(line: &str) -> (&str, Option<String>) {
    if split_family_arrow(line).is_none() {
        return split_message_label(line);
    }
    if let Some(colon) = line.rfind(" :") {
        let suffix = line[colon + 2..].trim();
        if !suffix_has_family_relation_arrow(suffix) {
            let text = line[colon + 2..].trim();
            if !text.is_empty() {
                return (line[..colon].trim_end(), Some(text.to_string()));
            }
        }
    }
    let mut in_quote = false;
    let mut last_colon = None;
    for (idx, ch) in line.char_indices() {
        if ch == '"' {
            in_quote = !in_quote;
            continue;
        }
        if !in_quote && ch == ':' {
            last_colon = Some(idx);
        }
    }
    if let Some(colon) = last_colon {
        let prefix = line[..colon].trim_end();
        let suffix = line[colon + 1..].trim();
        if !suffix.is_empty()
            && !suffix_has_family_relation_arrow(suffix)
            && split_family_arrow(prefix).is_some()
        {
            return (prefix, Some(suffix.to_string()));
        }
    }
    (line.trim_end(), None)
}

fn suffix_has_family_relation_arrow(suffix: &str) -> bool {
    suffix.contains("--")
        || suffix.contains("..")
        || suffix.contains("->")
        || suffix.contains("<-")
        || suffix.contains("|>")
        || suffix.contains("<|")
}

fn split_message_label(line: &str) -> (&str, Option<String>) {
    if let Some(colon) = line.find(':') {
        let text = line[colon + 1..].trim();
        (
            line[..colon].trim_end(),
            Some(text.to_string()).filter(|s| !s.is_empty()),
        )
    } else {
        (line.trim_end(), None)
    }
}

fn split_arrow(core: &str) -> Option<(&str, &str, &str)> {
    fn is_arrow_char(c: char) -> bool {
        matches!(
            c,
            '-' | '.' | '<' | '>' | '[' | ']' | 'o' | 'x' | '/' | '\\'
        )
    }

    let mut run_start: Option<usize> = None;
    let mut in_bracket = false;
    let mut skip_until = 0usize;
    for (idx, ch) in core.char_indices() {
        if idx < skip_until {
            continue;
        }
        if let Some(start) = run_start {
            if in_bracket {
                if ch == ']' {
                    in_bracket = false;
                }
                continue;
            }
            if ch == '[' {
                in_bracket = true;
                continue;
            }
            if is_arrow_char(ch) {
                continue;
            }
            let candidate = &core[start..idx];
            if !candidate.contains('-')
                && !(candidate.contains('.')
                    && (candidate.contains('<') || candidate.contains('>')))
            {
                run_start = None;
                continue;
            }
            let lhs = core[..start].trim();
            let rhs = core[idx..].trim();
            if !lhs.is_empty() && !rhs.is_empty() {
                return Some((lhs, candidate.trim(), rhs));
            }
            run_start = None;
            continue;
        }
        if ch == '[' && core[..idx].trim().is_empty() {
            let mut skipped_open_endpoint = false;
            for endpoint in ["[o", "[x"] {
                if core[idx..].starts_with(endpoint)
                    && core[idx + endpoint.len()..]
                        .chars()
                        .next()
                        .is_some_and(char::is_whitespace)
                {
                    skip_until = idx + endpoint.len();
                    skipped_open_endpoint = true;
                    break;
                }
            }
            if skipped_open_endpoint {
                continue;
            }
            if let Some(close_rel) = core[idx..].find(']') {
                let bracket_body = &core[idx + ch.len_utf8()..idx + close_rel];
                if bracket_body.contains('-') {
                    continue;
                }
                let after_idx = idx + close_rel + 1;
                if core[after_idx..]
                    .chars()
                    .next()
                    .is_some_and(char::is_whitespace)
                {
                    skip_until = after_idx;
                    continue;
                }
            } else if core[idx + ch.len_utf8()..]
                .chars()
                .next()
                .is_some_and(char::is_whitespace)
            {
                continue;
            }
        }
        if is_arrow_char(ch) {
            if run_start.is_none() {
                run_start = Some(idx);
            }
            if ch == '[' {
                in_bracket = true;
            }
            continue;
        }
    }
    if let Some(start) = run_start {
        let candidate = &core[start..];
        if !candidate.contains('-')
            && !(candidate.contains('.') && (candidate.contains('<') || candidate.contains('>')))
        {
            return None;
        }
        let lhs = core[..start].trim();
        if lhs.is_empty() {
            return None;
        }
        return Some((lhs, candidate.trim(), ""));
    }
    None
}

fn parse_arrow(arrow: &str) -> Option<String> {
    const VALID_BASE_ARROWS: &[&str] = &[
        "->", "-->", "->>", "-->>", "<-", "<--", "<<-", "<<--", "<->", "<-->", "<<->>", "<<-->>",
    ];
    let arrow = strip_sequence_arrow_brackets(arrow);
    let mut squashed = String::with_capacity(arrow.len());
    let mut last_slash: Option<char> = None;
    let mut slash_run_len = 0usize;
    for ch in arrow.chars() {
        if matches!(ch, '/' | '\\') {
            if last_slash == Some(ch) {
                slash_run_len += 1;
            } else {
                last_slash = Some(ch);
                slash_run_len = 1;
            }
            if ch == '/' && slash_run_len > 1 {
                // Portable slash forms allow a single slash marker only.
                return None;
            }
            if slash_run_len == 1 {
                squashed.push(ch);
            }
            continue;
        }
        last_slash = None;
        slash_run_len = 0;
        squashed.push(ch);
    }

    let canonical = squashed.replace(['/', '\\'], "").replace('.', "-");
    if canonical.is_empty()
        || !canonical
            .chars()
            .all(|c| matches!(c, '-' | '<' | '>' | 'o' | 'x'))
        || !squashed
            .chars()
            .all(|c| matches!(c, '-' | '.' | '<' | '>' | 'o' | 'x' | '/' | '\\'))
    {
        return None;
    }
    let has_slash_marker = squashed.contains('/') || squashed.contains('\\');
    let has_dot_marker = squashed.contains('.');
    let expanded_marker = squashed.contains("-/") || squashed.contains("-\\");

    if has_slash_marker && matches!(canonical.as_str(), "-" | "--") {
        return Some(squashed);
    }

    if VALID_BASE_ARROWS.contains(&canonical.as_str()) {
        if has_dot_marker {
            return Some(canonical);
        }
        if has_slash_marker && !expanded_marker {
            return Some(canonical);
        }
        if expanded_marker
            && squashed.contains("-\\")
            && canonical == "-->>"
            && squashed.contains("->>")
        {
            return Some(squashed.replacen("->>", "-->>", 1));
        }
        return Some(squashed);
    }
    let with_left_trimmed = canonical
        .strip_prefix('o')
        .or_else(|| canonical.strip_prefix('x'))
        .unwrap_or(&canonical);
    let (core, right_marker_removed) = if let Some(stripped) = with_left_trimmed.strip_suffix('o') {
        (stripped, true)
    } else if let Some(stripped) = with_left_trimmed.strip_suffix('x') {
        (stripped, true)
    } else {
        (with_left_trimmed, false)
    };
    if core.is_empty() {
        return None;
    }
    if VALID_BASE_ARROWS.contains(&core) && (right_marker_removed || core != canonical) {
        if has_dot_marker {
            return Some(canonical);
        }
        if has_slash_marker && !expanded_marker {
            let mut out = core.to_string();
            if let Some(ch) = with_left_trimmed.chars().last() {
                if matches!(ch, 'o' | 'x') && right_marker_removed {
                    out.push(ch);
                }
            }
            return Some(out);
        }
        if expanded_marker && canonical.contains("-->>") && squashed.contains("->>") {
            return Some(squashed.replacen("->>", "-->>", 1));
        }
        return Some(squashed);
    }
    if let Some(stripped_core) = core.strip_prefix('-') {
        if VALID_BASE_ARROWS.contains(&stripped_core) && (right_marker_removed || core != canonical)
        {
            if has_dot_marker {
                return Some(canonical);
            }
            if has_slash_marker && !expanded_marker {
                let mut out = stripped_core.to_string();
                if let Some(ch) = with_left_trimmed.chars().last() {
                    if matches!(ch, 'o' | 'x') && right_marker_removed {
                        out.push(ch);
                    }
                }
                return Some(out);
            }
            if expanded_marker && canonical.contains("-->>") && squashed.contains("->>") {
                return Some(squashed.replacen("->>", "-->>", 1));
            }
            return Some(squashed);
        }
    }
    None
}

fn strip_sequence_arrow_brackets(arrow: &str) -> String {
    let mut out = String::with_capacity(arrow.len());
    let mut chars = arrow.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '[' {
            for next in chars.by_ref() {
                if next == ']' {
                    break;
                }
            }
            continue;
        }
        out.push(ch);
    }
    out
}

fn split_lifecycle_modifier(endpoint: &str) -> (&str, Option<&'static str>) {
    for suffix in ["++", "--", "**", "!!"] {
        if let Some(base) = endpoint.trim_end().strip_suffix(suffix) {
            return (base.trim_end(), Some(suffix));
        }
    }
    (endpoint, None)
}

fn normalize_virtual_endpoint(raw: &str) -> Option<String> {
    let t = raw.trim().trim_matches('"');
    let lower = t.to_ascii_lowercase();
    match lower.as_str() {
        "[*]" => Some("[*]".to_string()),
        "[" => Some("[".to_string()),
        "]" => Some("]".to_string()),
        "[o" | "o[" => Some("[o".to_string()),
        "o]" | "]o" => Some("o]".to_string()),
        "[x" | "x[" => Some("[x".to_string()),
        "x]" | "]x" => Some("x]".to_string()),
        _ => None,
    }
}

fn looks_like_virtual_endpoint_syntax(raw: &str) -> bool {
    let t = raw.trim().trim_matches('"').to_ascii_lowercase();
    t.contains('[') || t.contains(']')
}

fn looks_like_arrow_syntax(line: &str) -> bool {
    if line.starts_with('!') || line.starts_with('@') {
        return false;
    }
    line.contains("->")
        || line.contains("-->")
        || line.contains("..>")
        || line.contains("<..")
        || line.contains("<-")
        || line.contains("<--")
        || line.contains("<->")
        || line.contains("<-->")
        || line.contains("->>")
        || line.contains("-->>")
        || line.contains("-x")
        || line.contains("x-")
        || line.contains("-o")
        || line.contains("o-")
}

fn is_sequence_keyword(kind: &StatementKind) -> bool {
    matches!(
        kind,
        StatementKind::Group(_)
            | StatementKind::Footbox(_)
            | StatementKind::Delay(_)
            | StatementKind::Divider(_)
            | StatementKind::Separator(_)
            | StatementKind::Spacer(_)
            | StatementKind::NewPage(_)
            | StatementKind::IgnoreNewPage
            | StatementKind::Autonumber(_)
            | StatementKind::Activate(_)
            | StatementKind::Deactivate(_)
            | StatementKind::Destroy(_)
            | StatementKind::Create(_)
            | StatementKind::Return(_)
    )
}

fn note_block_continues(lines: &[(&str, Span)], idx: usize, line: &str) -> bool {
    let lower = line.trim().to_ascii_lowercase();
    if !(lower.starts_with("note ") || lower.starts_with("hnote ") || lower.starts_with("rnote ")) {
        return false;
    }
    for (candidate, _) in lines.iter().skip(idx + 1) {
        let trimmed = candidate.trim();
        if trimmed.eq_ignore_ascii_case("end note")
            || trimmed.eq_ignore_ascii_case("endnote")
            || trimmed.eq_ignore_ascii_case("endhnote")
            || trimmed.eq_ignore_ascii_case("endrnote")
        {
            return true;
        }
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with('@') {
            return false;
        }
    }
    !line.contains(':')
}

fn text_block_continues(lines: &[(&str, Span)], idx: usize, line: &str) -> bool {
    let lower = line.trim().to_ascii_lowercase();
    let keyword = ["title", "header", "footer", "caption", "legend"]
        .into_iter()
        .find(|keyword| lower.starts_with(&format!("{keyword} ")));
    let Some(keyword) = keyword else {
        return false;
    };
    let end_marker = format!("end {keyword}");
    for (candidate, _) in lines.iter().skip(idx + 1) {
        let trimmed = candidate.trim();
        if trimmed.eq_ignore_ascii_case(&end_marker) {
            return true;
        }
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with('@') {
            return false;
        }
    }
    false
}

fn is_family_common_keyword(kind: &StatementKind) -> bool {
    matches!(
        kind,
        StatementKind::Note(_)
            | StatementKind::Title(_)
            | StatementKind::Caption(_)
            | StatementKind::Header(_)
            | StatementKind::Footer(_)
            | StatementKind::Legend(_)
            | StatementKind::LegendPos(_)
            | StatementKind::SkinParam { .. }
            | StatementKind::Theme(_)
            | StatementKind::Scale(_)
            | StatementKind::SetOption { .. }
            | StatementKind::HideOption(_)
            | StatementKind::Pragma(_)
    )
}

/// Parse an inline `json $alias { ... }` or `yaml $alias { ... }` block.
/// Returns the projection statement and closing line index if found, else `None`.
/// Errors if a projection block is found but no matching closing `}` appears.
fn parse_json_projection_block(
    lines: &[(&str, Span)],
    start: usize,
    line: &str,
) -> Result<Option<(StatementKind, usize)>, Diagnostic> {
    // Match: `json|yaml` <whitespace> <identifier starting with optional $> `{`
    let lower = line.to_ascii_lowercase();
    let (keyword, is_yaml) = if lower.starts_with("json ") {
        ("json", false)
    } else if lower.starts_with("yaml ") {
        ("yaml", true)
    } else {
        return Ok(None);
    };
    let rest = line[keyword.len() + 1..].trim();
    if rest.is_empty() {
        return Ok(None);
    }

    // Parse alias (identifier, optionally starting with `$`)
    let (alias, after_alias) = {
        let mut end = 0;
        let chars: Vec<char> = rest.chars().collect();
        if chars.is_empty() {
            return Ok(None);
        }
        // Allow `$identifier` or plain `identifier`
        if chars[0] == '$' {
            end += 1;
        }
        while end < chars.len() && (chars[end].is_alphanumeric() || chars[end] == '_') {
            end += 1;
        }
        if end == 0 || (end == 1 && rest.starts_with('$')) {
            return Ok(None);
        }
        let alias = rest[..end].to_string();
        let after = rest[end..].trim();
        (alias, after)
    };

    // Must be followed by `{`
    if !after_alias.starts_with('{') {
        return Ok(None);
    }

    // Accumulate body lines until the matching `}` (depth-tracked).
    let mut body_lines: Vec<&str> = Vec::new();
    // The opening `{` may have content after it on the same line.
    let inline_after_brace = after_alias[1..].trim();
    let mut depth: i32 = 1;

    // If everything is on one line: `json $alias { ... }`
    if !inline_after_brace.is_empty() {
        let mut in_quotes = false;
        let mut prev_escape = false;
        for (j, ch) in inline_after_brace.char_indices() {
            if in_quotes {
                if ch == '"' && !prev_escape {
                    in_quotes = false;
                }
                prev_escape = ch == '\\' && !prev_escape;
                continue;
            }
            match ch {
                '"' => in_quotes = true,
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        let body = inline_after_brace[..j].trim().to_string();
                        let kind = if is_yaml {
                            StatementKind::YamlProjection { alias, body }
                        } else {
                            StatementKind::JsonProjection { alias, body }
                        };
                        return Ok(Some((kind, start)));
                    }
                }
                _ => {}
            }
            prev_escape = false;
        }
        // Depth > 0: content continues on next lines.
        body_lines.push(inline_after_brace);
    }

    // Continue scanning subsequent lines.
    let mut i = start + 1;
    while i < lines.len() {
        let (raw, _span) = lines[i];
        let trimmed = raw.trim();
        // Check for matching closing brace.
        let mut consumed_close = false;
        let mut close_pos = 0;
        let mut in_quotes = false;
        let mut prev_escape = false;
        for (pos, ch) in trimmed.char_indices() {
            if in_quotes {
                if ch == '"' && !prev_escape {
                    in_quotes = false;
                }
                prev_escape = ch == '\\' && !prev_escape;
                continue;
            }
            match ch {
                '"' => in_quotes = true,
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        consumed_close = true;
                        close_pos = pos;
                        break;
                    }
                }
                _ => {}
            }
            prev_escape = false;
        }
        if consumed_close {
            // Everything before the closing `}` is part of the body.
            let last_body = trimmed[..close_pos].trim();
            if !last_body.is_empty() {
                body_lines.push(last_body);
            }
            let body = body_lines.join("\n");
            let kind = if is_yaml {
                StatementKind::YamlProjection { alias, body }
            } else {
                StatementKind::JsonProjection { alias, body }
            };
            return Ok(Some((kind, i)));
        }
        body_lines.push(trimmed);
        i += 1;
    }

    // No closing brace found.
    Err(Diagnostic::error(format!(
        "[E_PROJECTION_UNCLOSED] `{keyword} {alias}` block has no matching closing `}}`"
    ))
    .with_span(lines[start].1))
}

/// Parse a single salt wireframe row line into a `SaltGridRow` statement.
/// A row is a `|`-delimited sequence of cell tokens.
/// Returns `None` if the line does not start with `|`.
fn parse_salt_grid_row(line: &str) -> Option<StatementKind> {
    let trimmed = line.trim();
    let lower = trimmed.to_ascii_lowercase();
    let whole_line_widget = lower.starts_with("{*")
        || lower.starts_with("{/")
        || lower.starts_with("{s")
        || lower.starts_with("{t")
        || lower.starts_with("{+")
        || lower.starts_with("{#")
        || lower.starts_with("{!")
        || lower.starts_with("{^")
        || lower == "tree"
        || lower.starts_with("tree ")
        || lower == "menu"
        || lower.starts_with("menu ")
        || lower == "tab"
        || lower.starts_with("tab ")
        || lower == "tabs"
        || lower.starts_with("tabs ")
        || lower.starts_with("scroll")
        || lower.contains("scrollbar");
    if whole_line_widget {
        return Some(StatementKind::SaltGridRow {
            cells: vec![SaltCell::Label(trimmed.to_string())],
        });
    }
    if !trimmed.contains('|') {
        return None;
    }
    // Split on `|` and parse each cell token.
    let parts: Vec<&str> = trimmed.split('|').collect();
    let mut cells = Vec::new();
    for part in parts {
        let cell_text = part.trim();
        if cell_text.is_empty() {
            continue;
        }
        cells.push(parse_salt_cell(cell_text));
    }
    if cells.is_empty() {
        return None;
    }
    Some(StatementKind::SaltGridRow { cells })
}

/// Parse a single salt cell token into a `SaltCell` variant.
fn parse_salt_cell(text: &str) -> SaltCell {
    // `"placeholder"` → Input
    if text.starts_with('"') && text.ends_with('"') && text.len() >= 2 {
        let inner = &text[1..text.len() - 1];
        return SaltCell::Input(inner.to_string());
    }
    // `[X] label`, `[ ] label`, or compact `[] label` → Checkbox
    if text.starts_with("[X]") || text.starts_with("[x]") {
        let label = text[3..].trim().to_string();
        return SaltCell::CheckboxChecked(label);
    }
    if let Some(rest) = text.strip_prefix("[ ]") {
        return SaltCell::CheckboxUnchecked(rest.trim().to_string());
    }
    if let Some(rest) = text.strip_prefix("[]") {
        return SaltCell::CheckboxUnchecked(rest.trim().to_string());
    }
    // `(X) label`, `( ) label`, or compact `() label` → Radio
    if text.starts_with("(X)") || text.starts_with("(x)") {
        let label = text[3..].trim().to_string();
        return SaltCell::RadioOn(label);
    }
    if let Some(rest) = text.strip_prefix("( )") {
        return SaltCell::RadioOff(rest.trim().to_string());
    }
    if let Some(rest) = text.strip_prefix("()") {
        return SaltCell::RadioOff(rest.trim().to_string());
    }
    // `[button text]` → Button
    if text.starts_with('[') && text.ends_with(']') && text.len() >= 2 {
        let inner = &text[1..text.len() - 1];
        return SaltCell::Button(inner.to_string());
    }
    // `^combo text^` → Combo
    if text.starts_with('^') && text.ends_with('^') && text.len() >= 2 {
        let inner = &text[1..text.len() - 1];
        return SaltCell::Combo(inner.to_string());
    }
    // Plain text → Label
    SaltCell::Label(text.to_string())
}

#[cfg(test)]
mod tests {
    use super::{parse_with_options, ParseOptions};
    use crate::ast::{ActivityStepKind, DiagramKind, StatementKind};
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn define_substitution_skips_quoted_strings() {
        let doc = parse_with_options(
            "!define NAME Alice\nparticipant NAME\nnote over NAME: \"NAME\"\n",
            &ParseOptions::default(),
        )
        .unwrap();

        assert!(matches!(
            doc.statements[0].kind,
            StatementKind::Participant(_)
        ));
        match &doc.statements[1].kind {
            StatementKind::Note(n) => {
                assert_eq!(n.target.as_deref(), Some("Alice"));
                assert_eq!(n.text, "\"NAME\"");
            }
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn pragma_directives_with_arguments_are_preserved_as_statements() {
        let doc = parse_with_options(
            "!pragma teoz true\nparticipant A\nparticipant B\nA -> B: hi\n",
            &ParseOptions::default(),
        )
        .unwrap();

        assert_eq!(doc.statements.len(), 4);
        assert!(matches!(doc.statements[0].kind, StatementKind::Pragma(_)));
        assert!(matches!(
            doc.statements[1].kind,
            StatementKind::Participant(_)
        ));
        assert!(matches!(
            doc.statements[2].kind,
            StatementKind::Participant(_)
        ));
        assert!(matches!(doc.statements[3].kind, StatementKind::Message(_)));
    }

    #[test]
    fn include_resolves_relative_to_root() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("inc.puml"), "A -> B\n").unwrap();

        let doc = parse_with_options(
            "!include inc.puml",
            &ParseOptions {
                include_root: Some(dir.path().to_path_buf()),
                ..ParseOptions::default()
            },
        )
        .unwrap();

        assert!(matches!(doc.statements[0].kind, StatementKind::Message(_)));
    }

    #[test]
    fn include_cycle_errors() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.puml"), "!include b.puml\n").unwrap();
        fs::write(dir.path().join("b.puml"), "!include a.puml\n").unwrap();

        let err = parse_with_options(
            "!include a.puml",
            &ParseOptions {
                include_root: Some(dir.path().to_path_buf()),
                ..ParseOptions::default()
            },
        )
        .unwrap_err();

        assert!(err.message.contains("include cycle detected"));
    }

    #[test]
    fn include_from_stdin_requires_root() {
        let err = parse_with_options("!include x.puml", &ParseOptions::default()).unwrap_err();
        assert!(err.message.contains("E_INCLUDE_ROOT_REQUIRED"));
    }

    #[test]
    fn include_rejects_parent_escape_outside_root() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("root");
        let outside = dir.path().join("outside.puml");
        fs::create_dir_all(&root).unwrap();
        fs::write(&outside, "A -> B\n").unwrap();

        let err = parse_with_options(
            "!include ../outside.puml",
            &ParseOptions {
                include_root: Some(root),
                ..ParseOptions::default()
            },
        )
        .unwrap_err();

        assert!(err.message.contains("E_INCLUDE_ESCAPE"));
    }

    #[cfg(unix)]
    #[test]
    fn include_rejects_symlink_target_outside_root() {
        use std::os::unix::fs::symlink;

        let dir = tempdir().unwrap();
        let root = dir.path().join("root");
        let outside = dir.path().join("outside.puml");
        let link = root.join("link_outside.puml");

        fs::create_dir_all(&root).unwrap();
        fs::write(&outside, "A -> B\n").unwrap();
        symlink(&outside, &link).unwrap();

        let err = parse_with_options(
            "!include link_outside.puml",
            &ParseOptions {
                include_root: Some(root),
                ..ParseOptions::default()
            },
        )
        .unwrap_err();

        assert!(err.message.contains("E_INCLUDE_ESCAPE"));
    }

    #[test]
    fn include_id_extracts_startsub_block() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("inc.puml"),
            "!startsub FLOW\nA -> B : one\n!endsub\n",
        )
        .unwrap();

        let doc = parse_with_options(
            "!include inc.puml!FLOW",
            &ParseOptions {
                include_root: Some(dir.path().to_path_buf()),
                ..ParseOptions::default()
            },
        )
        .unwrap();

        assert!(matches!(doc.statements[0].kind, StatementKind::Message(_)));
    }

    #[test]
    fn include_id_missing_tag_errors() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("inc.puml"),
            "!startsub FLOW\nA -> B : one\n!endsub\n",
        )
        .unwrap();

        let err = parse_with_options(
            "!include inc.puml!MISSING",
            &ParseOptions {
                include_root: Some(dir.path().to_path_buf()),
                ..ParseOptions::default()
            },
        )
        .unwrap_err();

        assert!(err.message.contains("E_INCLUDE_TAG_NOT_FOUND"));
    }

    #[test]
    fn include_url_disabled_errors() {
        let err = parse_with_options(
            "!include https://example.com/a.puml",
            &ParseOptions {
                allow_url_includes: false,
                ..ParseOptions::default()
            },
        )
        .unwrap_err();
        assert!(err.message.contains("E_INCLUDE_URL_DISABLED"));
    }

    #[test]
    fn import_resolves_stdlib_module_from_include_root() {
        let dir = tempdir().unwrap();
        let stdlib = dir.path().join("stdlib");
        fs::create_dir_all(stdlib.join("nested")).unwrap();
        fs::write(stdlib.join("core.puml"), "A -> B : core\n").unwrap();
        fs::write(
            stdlib.join("nested").join("extra.puml"),
            "B -> A : nested\n",
        )
        .unwrap();

        let doc = parse_with_options(
            "!import core\n!import nested/extra\n!import core\n",
            &ParseOptions {
                include_root: Some(dir.path().to_path_buf()),
                ..ParseOptions::default()
            },
        )
        .unwrap();
        assert_eq!(doc.statements.len(), 2);
    }

    #[test]
    fn include_angle_bracket_targets_resolve_from_stdlib_catalog() {
        let dir = tempdir().unwrap();
        let stdlib = dir.path().join("stdlib");
        fs::create_dir_all(stdlib.join("C4")).unwrap();
        fs::write(
            stdlib.join("C4").join("C4_Container.puml"),
            "!procedure Container($alias,$label)\n$alias -> $alias : [C4] $label\n!endprocedure\n",
        )
        .unwrap();

        let doc = parse_with_options(
            "!include <C4/C4_Container>\nContainer(Api, \"API\")\n",
            &ParseOptions {
                include_root: Some(dir.path().to_path_buf()),
                ..ParseOptions::default()
            },
        )
        .unwrap();
        assert_eq!(doc.statements.len(), 1);
    }

    #[test]
    fn import_and_include_catalog_support_aws_shape_stub_surface() {
        let dir = tempdir().unwrap();
        let stdlib = dir.path().join("stdlib");
        fs::create_dir_all(stdlib.join("awslib14").join("Compute")).unwrap();
        fs::write(
            stdlib.join("awslib14").join("AWSCommon.puml"),
            "!procedure AWSIcon($alias,$service,$label=\"\")\n$alias -> $alias : [AWS $service] $label\n!endprocedure\n",
        )
        .unwrap();
        fs::write(
            stdlib.join("awslib14").join("Compute").join("EC2.puml"),
            "!include <awslib14/AWSCommon>\n!procedure EC2($alias,$label=\"\")\nAWSIcon($alias,EC2,$label)\n!endprocedure\n",
        )
        .unwrap();

        let doc = parse_with_options(
            "!import awslib14/AWSCommon\n!include <awslib14/Compute/EC2>\nEC2(NodeA, \"ingress\")\n",
            &ParseOptions {
                include_root: Some(dir.path().to_path_buf()),
                ..ParseOptions::default()
            },
        )
        .unwrap();
        assert_eq!(doc.statements.len(), 1);
    }

    #[test]
    fn import_and_include_catalog_support_azure_and_gcp_shape_stub_surface() {
        let dir = tempdir().unwrap();
        let stdlib = dir.path().join("stdlib");

        fs::create_dir_all(stdlib.join("azure")).unwrap();
        fs::write(
            stdlib.join("azure").join("AzureCommon.puml"),
            "!procedure AzureIcon($alias,$service,$label=\"\")\n$alias -> $alias : [AZURE $service] $label\n!endprocedure\n",
        )
        .unwrap();
        fs::write(
            stdlib.join("azure").join("StorageAccount.puml"),
            "!include <azure/AzureCommon>\n!procedure AzureStorageAccount($alias,$label=\"\")\nAzureIcon($alias,StorageAccount,$label)\n!endprocedure\n",
        )
        .unwrap();

        fs::create_dir_all(stdlib.join("gcp")).unwrap();
        fs::write(
            stdlib.join("gcp").join("GCPCommon.puml"),
            "!procedure GCPIcon($alias,$service,$label=\"\")\n$alias -> $alias : [GCP $service] $label\n!endprocedure\n",
        )
        .unwrap();
        fs::write(
            stdlib.join("gcp").join("ComputeEngine.puml"),
            "!include <gcp/GCPCommon>\n!procedure GCPComputeEngine($alias,$label=\"\")\nGCPIcon($alias,ComputeEngine,$label)\n!endprocedure\n",
        )
        .unwrap();

        let doc = parse_with_options(
            "!import azure/AzureCommon\n!include <azure/StorageAccount>\nAzureStorageAccount(AzStore, \"assets\")\n!import gcp/GCPCommon\n!include <gcp/ComputeEngine>\nGCPComputeEngine(GceNode, \"ingress\")\n",
            &ParseOptions {
                include_root: Some(dir.path().to_path_buf()),
                ..ParseOptions::default()
            },
        )
        .unwrap();
        assert_eq!(doc.statements.len(), 2);
    }

    #[test]
    fn import_requires_stdlib_root_when_no_include_root_is_available() {
        let err = parse_with_options("!import core\n", &ParseOptions::default()).unwrap_err();
        assert!(err.message.contains("E_IMPORT_ROOT_REQUIRED"));
    }

    #[test]
    fn import_security_and_shape_errors_are_deterministic() {
        let dir = tempdir().unwrap();
        let stdlib = dir.path().join("stdlib");
        fs::create_dir_all(&stdlib).unwrap();
        fs::write(stdlib.join("ok.puml"), "A -> B\n").unwrap();

        let cases = [
            ("!import\n", "E_IMPORT_PATH_REQUIRED"),
            ("!import /tmp/abs.puml\n", "E_IMPORT_ABSOLUTE_PATH"),
            ("!import bad!TAG\n", "E_IMPORT_INVALID_FORM"),
            ("!import ../outside\n", "E_IMPORT_ESCAPE"),
            ("!import does/not/exist\n", "E_IMPORT_STDLIB_NOT_FOUND"),
        ];

        for (src, code) in cases {
            let err = parse_with_options(
                src,
                &ParseOptions {
                    include_root: Some(dir.path().to_path_buf()),
                    ..ParseOptions::default()
                },
            )
            .unwrap_err();
            assert!(
                err.message.contains(code),
                "missing {code}: {}",
                err.message
            );
        }
    }

    #[test]
    fn import_url_disabled_errors() {
        let dir = tempfile::tempdir().unwrap();
        let stdlib = dir.path().join("stdlib");
        fs::create_dir_all(&stdlib).unwrap();
        let err = parse_with_options(
            "!import https://example.com/lib.puml\n",
            &ParseOptions {
                include_root: Some(dir.path().to_path_buf()),
                allow_url_includes: false,
            },
        )
        .unwrap_err();
        assert!(
            err.message.contains("E_INCLUDE_URL_DISABLED"),
            "missing E_INCLUDE_URL_DISABLED: {}",
            err.message
        );
    }

    #[test]
    fn include_once_only_expands_first_occurrence() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("inc.puml"), "A -> B : once\n").unwrap();

        let doc = parse_with_options(
            "!include_once inc.puml\n!include_once inc.puml\n",
            &ParseOptions {
                include_root: Some(dir.path().to_path_buf()),
                ..ParseOptions::default()
            },
        )
        .unwrap();

        assert_eq!(doc.statements.len(), 1);
    }

    #[test]
    fn include_many_expands_each_occurrence() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("inc.puml"), "A -> B : many\n").unwrap();

        let doc = parse_with_options(
            "!include_many inc.puml\n!include_many inc.puml\n",
            &ParseOptions {
                include_root: Some(dir.path().to_path_buf()),
                ..ParseOptions::default()
            },
        )
        .unwrap();

        assert_eq!(doc.statements.len(), 2);
    }

    #[test]
    fn include_once_deduplicates_canonical_path_aliases() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("nested")).unwrap();
        fs::write(dir.path().join("inc.puml"), "A -> B : once\n").unwrap();

        let doc = parse_with_options(
            "!include_once ./inc.puml\n!include_once nested/../inc.puml\n",
            &ParseOptions {
                include_root: Some(dir.path().to_path_buf()),
                ..ParseOptions::default()
            },
        )
        .unwrap();

        assert_eq!(doc.statements.len(), 1);
    }

    #[test]
    fn includesub_requires_tag() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("inc.puml"), "A -> B : body\n").unwrap();

        let err = parse_with_options(
            "!includesub inc.puml\n",
            &ParseOptions {
                include_root: Some(dir.path().to_path_buf()),
                ..ParseOptions::default()
            },
        )
        .unwrap_err();

        assert!(err.message.contains("E_INCLUDESUB_TAG_REQUIRED"));
    }

    #[test]
    fn include_many_url_disabled_errors() {
        let err = parse_with_options(
            "!include_many https://example.com/a.puml",
            &ParseOptions {
                allow_url_includes: false,
                ..ParseOptions::default()
            },
        )
        .unwrap_err();
        assert!(err.message.contains("E_INCLUDE_URL_DISABLED"));
    }

    #[test]
    fn include_url_directive_disabled_errors_deterministically() {
        let err = parse_with_options(
            "!includeurl https://example.com/a.puml",
            &ParseOptions {
                allow_url_includes: false,
                ..ParseOptions::default()
            },
        )
        .unwrap_err();
        assert!(err.message.contains("E_INCLUDE_URL_DISABLED"));
        assert!(err
            .message
            .contains("!includeurl URL includes are disabled"));
    }

    #[test]
    fn conditional_if_elseif_else_selects_first_matching_branch() {
        let doc = parse_with_options(
            "!define FLAG 1\n!if FLAG == 1\nA -> B: first\n!elseif 1\nA -> B: second\n!else\nA -> B: third\n!endif\n",
            &ParseOptions::default(),
        )
        .unwrap();

        assert_eq!(doc.statements.len(), 1);
        match &doc.statements[0].kind {
            StatementKind::Message(m) => assert_eq!(m.label.as_deref(), Some("first")),
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn ifdef_and_ifndef_follow_define_state() {
        let doc = parse_with_options(
            "!define ENABLED 1\n!ifdef ENABLED\nA -> B: yes\n!endif\n!ifndef ENABLED\nA -> B: no\n!endif\n",
            &ParseOptions::default(),
        )
        .unwrap();

        assert_eq!(doc.statements.len(), 1);
        match &doc.statements[0].kind {
            StatementKind::Message(m) => assert_eq!(m.label.as_deref(), Some("yes")),
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn while_loops_execute_with_define_updates() {
        let doc = parse_with_options(
            "!define COUNT 2\n!while COUNT != 0\nA -> B: loop\n!if COUNT == 2\n!define COUNT 1\n!elseif COUNT == 1\n!define COUNT 0\n!endif\n!endwhile\n",
            &ParseOptions::default(),
        )
        .unwrap();

        assert_eq!(doc.statements.len(), 2);
        assert!(matches!(doc.statements[0].kind, StatementKind::Message(_)));
        assert!(matches!(doc.statements[1].kind, StatementKind::Message(_)));
    }

    #[test]
    fn preprocessor_function_and_procedure_blocks_are_accepted() {
        let doc = parse_with_options(
            "@startuml\n!function Echo($x)\n!return $x\n!endfunction\n!procedure Emit($x)\n!log $x\n!endprocedure\nA -> B: hi\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(doc.statements.len(), 1);
        assert!(matches!(doc.statements[0].kind, StatementKind::Message(_)));
    }

    #[test]
    fn preprocessor_variables_and_callable_args_are_applied() {
        let doc = parse_with_options(
            "@startuml\n!$from = Alice\n!$to ?= Bob\n!function F($x,$y=\"B\")\n!return $x + $y\n!endfunction\n!procedure P($a,$b=\"B\")\n$a -> $b: via-proc\n!endprocedure\n!P($from,$to)\n$from -> $to: %F(\"A\")\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(doc.statements.len(), 2);
        assert!(matches!(doc.statements[0].kind, StatementKind::Message(_)));
        assert!(matches!(doc.statements[1].kind, StatementKind::Message(_)));
    }

    #[test]
    fn preprocessor_concat_signature_and_arg_errors_are_deterministic() {
        let doc = parse_with_options(
            "@startuml\n!function Join($a##$b)\n!return $a ## $b\n!endfunction\nA -> B: %Join(Al, ice)\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap();
        match &doc.statements[0].kind {
            StatementKind::Message(m) => assert_eq!(m.label.as_deref(), Some("Alice")),
            other => panic!("unexpected statement: {other:?}"),
        }

        let missing = parse_with_options(
            "@startuml\n!function Need($a,$b)\n!return $a\n!endfunction\nA -> B: %Need(\"x\")\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap_err();
        assert!(missing.message.contains("E_PREPROC_ARG_REQUIRED"));
    }

    #[test]
    fn preprocessor_assert_false_is_rejected() {
        let err = parse_with_options(
            "@startuml\n!assert false : expected failure\nA -> B: hi\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap_err();
        assert!(err.message.contains("E_PREPROC_ASSERT"));
    }

    #[test]
    fn preprocessor_assert_requires_non_empty_expression() {
        let err = parse_with_options(
            "@startuml\n!assert\nA -> B: hi\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap_err();
        assert!(err.message.contains("E_PREPROC_ASSERT_EXPR_REQUIRED"));
    }

    #[test]
    fn preprocessor_unknown_builtin_is_rejected_deterministically() {
        // Truly-unknown `%xyz(...)` invocations must surface a deterministic
        // diagnostic so that drift in PlantUML's builtin surface fails fast
        // instead of silently going through.
        let err = parse_with_options(
            "@startuml\n!assert %nosuchbuiltin() : no\nA -> B: hi\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap_err();
        assert!(
            err.message.contains("E_PREPROC_BUILTIN_UNSUPPORTED"),
            "expected E_PREPROC_BUILTIN_UNSUPPORTED, got: {}",
            err.message
        );
    }

    #[test]
    fn preprocessor_builtin_basics_expand_inline() {
        // strlen, upper/lower, substr, intval, boolval — these used to error
        // out via E_PREPROC_BUILTIN_UNSUPPORTED. They now expand inline.
        let doc = parse_with_options(
            "@startuml\nA -> B : %strlen(\"hello\")=%upper(\"ab\")/%substr(\"plantuml\", 0, 5)\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap();
        match &doc.statements[0].kind {
            StatementKind::Message(m) => {
                assert_eq!(m.label.as_deref(), Some("5=AB/plant"));
            }
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn preprocessor_json_variable_round_trips_via_get_json_attribute() {
        // JSON variable assignment is now accepted; `%get_json_attribute`
        // reads a single top-level string value.
        let doc = parse_with_options(
            "@startuml\n!$cfg = { \"name\": \"alpha\", \"v\": 2 }\nA -> B : %get_json_attribute($cfg, \"name\")\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap();
        match &doc.statements[0].kind {
            StatementKind::Message(m) => assert_eq!(m.label.as_deref(), Some("alpha")),
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn preprocessor_invoke_procedure_dynamically_dispatches_to_callable() {
        // `%invoke_procedure("$Say", ...)` resolves to a previously declared
        // `!procedure` and executes its body deterministically.
        let doc = parse_with_options(
            "@startuml\n!procedure $Say($who)\nA -> $who : hi\n!endprocedure\n%invoke_procedure(\"$Say\", \"Bob\")\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap();
        match &doc.statements[0].kind {
            StatementKind::Message(m) => {
                assert_eq!(m.to, "Bob");
                assert_eq!(m.label.as_deref(), Some("hi"));
            }
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn preprocessor_call_user_func_supports_dynamic_function_invocation() {
        let doc = parse_with_options(
            "@startuml\n!function F($x,$y)\n!return $x + $y\n!endfunction\nA -> B : %call_user_func(\"F\", \"A\", \"B\")\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap();
        match &doc.statements[0].kind {
            StatementKind::Message(m) => {
                assert_eq!(m.label.as_deref(), Some("\"A\" + \"B\""));
            }
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn preprocessor_unclosed_function_is_rejected() {
        let err = parse_with_options(
            "@startuml\n!function Echo($x)\nA -> B: hi\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap_err();
        assert!(err.message.contains("E_FUNCTION_UNCLOSED"));
    }

    #[test]
    fn unknown_preprocessor_directive_errors_deterministically() {
        let err = parse_with_options("!totallynew thing\nA -> B\n", &ParseOptions::default())
            .unwrap_err();
        assert!(err.message.contains("E_PREPROC_UNSUPPORTED"));
        assert!(err.message.contains("!totallynew"));
    }

    #[test]
    fn conditional_requires_balancing_and_order() {
        let err = parse_with_options("!endif\n", &ParseOptions::default()).unwrap_err();
        assert!(err.message.contains("E_PREPROC_COND_UNEXPECTED"));

        let err = parse_with_options("!if 1\nA -> B\n", &ParseOptions::default()).unwrap_err();
        assert!(err.message.contains("E_PREPROC_COND_UNCLOSED"));

        let err = parse_with_options(
            "!if 1\n!else\n!elseif 1\n!endif\n",
            &ParseOptions::default(),
        )
        .unwrap_err();
        assert!(err.message.contains("E_PREPROC_COND_ORDER"));
    }

    #[test]
    fn preprocessor_parenthesized_logical_conditions_are_supported() {
        let doc = parse_with_options(
            "@startuml\n!if (1 && (0 || 1))\nA -> B : yes\n!endif\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(doc.statements.len(), 1);
        assert!(matches!(doc.statements[0].kind, StatementKind::Message(_)));
    }

    #[test]
    fn preprocessor_conditions_support_nested_integer_arithmetic() {
        let doc = parse_with_options(
            "@startuml\n!if (2 + 3 * (4 - 1)) == 11\nA -> B : math\n!endif\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(doc.statements.len(), 1);
        match &doc.statements[0].kind {
            StatementKind::Message(m) => assert_eq!(m.label.as_deref(), Some("math")),
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn preprocessor_macro_concat_collapses_expanded_function_body_tokens() {
        let doc = parse_with_options(
            "@startuml\n!function Join($a,$b)\n!return $a ## $b\n!endfunction\nA -> B : %Join(Al, ice)\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap();
        match &doc.statements[0].kind {
            StatementKind::Message(m) => assert_eq!(m.label.as_deref(), Some("Alice")),
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn preprocessor_function_like_define_and_collection_aliases_expand_inline() {
        let doc = parse_with_options(
            "@startuml\n!define EDGE(a,b,label) a -> b : %upper(label)\n!$items = %list(\"red\", %map(\"name\", \"blue\"))\n!$items = %list_set($items, 0, \"green\")\n!$cfg = %map(\"items\", $items)\n!assert not %map_is_empty($cfg) and %map_contains_value($cfg, \"blue\")\nEDGE(Alice, Bob, ok)\nA -> B : %eval_int(\"2 + 3 * 4\")/%get($cfg, \"items[1].name\")/%list_get(%get($cfg, \"items\"), 0)\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap();
        let labels = doc
            .statements
            .iter()
            .filter_map(|stmt| match &stmt.kind {
                StatementKind::Message(m) => m.label.as_deref(),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(labels, vec!["OK", "14/blue/green"]);
    }

    #[test]
    fn preprocessor_json_helpers_return_nested_objects_and_empty_keys() {
        let doc = parse_with_options(
            "@startuml\n!$cfg = { \"users\": [{ \"name\": \"Ada\", \"meta\": { \"team\": \"core\" }}], \"empty\": \"\" }\n!if %json_key_exists($cfg, \"empty\")\nA -> B : %get_json_attribute($cfg, \"users[0].meta\")\n!endif\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap();
        match &doc.statements[0].kind {
            StatementKind::Message(m) => {
                assert_eq!(m.label.as_deref(), Some("{\"team\":\"core\"}"));
            }
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn preprocessor_list_map_helpers_and_modulo_expand_inline() {
        let doc = parse_with_options(
            "@startuml\n!$cfg = { \"name\": \"Ada\", \"role\": \"core\" }\n!foreach $item in %split(\"red|blue\", \"|\")\nA -> B : $item\n!endfor\n!if 7 % 4 == 3\nA -> B : %get($cfg, \"name\")/%join([\"x\",\"y\"], \"-\")/%quote(ok)\n!endif\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap();
        let labels = doc
            .statements
            .iter()
            .filter_map(|stmt| match &stmt.kind {
                StatementKind::Message(m) => m.label.as_deref(),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(labels, vec!["red", "blue", "Ada/x-y/\"ok\""]);
    }

    #[test]
    fn preprocessor_foreach_binds_map_pairs_and_array_indices() {
        let doc = parse_with_options(
            "@startuml\n!$cfg = { \"name\": \"Ada\", \"role\": \"core\" }\n!foreach $key, $value in $cfg\nA -> B : $key=$value\n!endfor\n!foreach $idx, $color in [\"red\",\"blue\"]\nA -> B : $idx:$color\n!endfor\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap();
        let labels = doc
            .statements
            .iter()
            .filter_map(|stmt| match &stmt.kind {
                StatementKind::Message(m) => m.label.as_deref(),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(labels, vec!["name=Ada", "role=core", "0:red", "1:blue"]);
    }

    #[test]
    fn preprocessor_list_and_map_builtin_aliases_expand_inline() {
        let doc = parse_with_options(
            "@startuml\n!$list = %list_insert([\"a\",\"c\"], 1, \"b\")\n!$map = %map(\"name\", \"Ada\", \"role\", \"core\")\nA -> B : %join(%list_reverse($list), \"\")/%list_indexof($list, \"b\")/%first($list)/%last($list)\nA -> B : %json_type(%str2json($map))/%get_json_type($map)/%map_entries($map)\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap();
        let labels = doc
            .statements
            .iter()
            .filter_map(|stmt| match &stmt.kind {
                StatementKind::Message(m) => m.label.as_deref(),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(labels[0], "cba/1/a/c");
        assert!(labels[1].starts_with("object/object/[[\"name\",\"Ada\"],[\"role\",\"core\"]]"));
    }

    #[test]
    fn while_requires_balancing() {
        let err = parse_with_options("!endwhile\n", &ParseOptions::default()).unwrap_err();
        assert!(err.message.contains("E_PREPROC_WHILE_UNEXPECTED"));

        let err = parse_with_options("!while 1\nA -> B\n", &ParseOptions::default()).unwrap_err();
        assert!(err.message.contains("E_PREPROC_WHILE_UNCLOSED"));
    }

    #[test]
    fn parses_multiline_title_and_legend_blocks() {
        let doc = parse_with_options(
            "title\nLine 1\nLine 2\nend title\nlegend\nAlpha\nBeta\nend legend\nA -> B\n",
            &ParseOptions::default(),
        )
        .unwrap();

        match &doc.statements[0].kind {
            StatementKind::Title(v) => assert_eq!(v, "Line 1\nLine 2"),
            other => panic!("unexpected statement: {other:?}"),
        }
        match &doc.statements[1].kind {
            StatementKind::Legend(v) => assert_eq!(v, "Alpha\nBeta"),
            other => panic!("unexpected statement: {other:?}"),
        }
        assert!(matches!(doc.statements[2].kind, StatementKind::Message(_)));
    }

    #[test]
    fn parses_multiline_note_block() {
        let doc = parse_with_options(
            "A -> B\nnote right of B\nline 1\nline 2\nend note\n",
            &ParseOptions::default(),
        )
        .unwrap();

        match &doc.statements[1].kind {
            StatementKind::Note(n) => {
                assert_eq!(n.position, "right");
                assert_eq!(n.target.as_deref(), Some("B"));
                assert_eq!(n.text, "line 1\nline 2");
            }
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn parses_note_across_without_target() {
        let doc =
            parse_with_options("note across: shared context\n", &ParseOptions::default()).unwrap();

        match &doc.statements[0].kind {
            StatementKind::Note(n) => {
                assert_eq!(n.position, "across");
                assert!(n.target.is_none());
                assert_eq!(n.text, "shared context");
            }
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn parses_multiline_note_with_inline_head_text() {
        let doc = parse_with_options(
            "note over A, B: summary\nline 2\nend note\n",
            &ParseOptions::default(),
        )
        .unwrap();

        match &doc.statements[0].kind {
            StatementKind::Note(n) => {
                assert_eq!(n.position, "over");
                assert_eq!(n.target.as_deref(), Some("A, B"));
                assert_eq!(n.text, "summary\nline 2");
            }
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn parses_hnote_and_rnote_aliases_as_note() {
        let doc = parse_with_options(
            "hnote over A: alias form\nrnote right of A: rounded alias\n",
            &ParseOptions::default(),
        )
        .unwrap();

        match &doc.statements[0].kind {
            StatementKind::Note(n) => {
                assert_eq!(n.kind, crate::ast::NoteKind::Hexagonal);
                assert_eq!(n.position, "over");
                assert_eq!(n.target.as_deref(), Some("A"));
                assert_eq!(n.text, "alias form");
            }
            other => panic!("unexpected statement: {other:?}"),
        }
        match &doc.statements[1].kind {
            StatementKind::Note(n) => {
                assert_eq!(n.kind, crate::ast::NoteKind::Rectangle);
                assert_eq!(n.position, "right");
                assert_eq!(n.target.as_deref(), Some("A"));
                assert_eq!(n.text, "rounded alias");
            }
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn parses_hnote_and_rnote_multiline_terminators() {
        let doc = parse_with_options(
            "hnote over A\nhex body\nendhnote\nrnote over B\nrect body\nendrnote\n",
            &ParseOptions::default(),
        )
        .unwrap();

        match &doc.statements[0].kind {
            StatementKind::Note(n) => {
                assert_eq!(n.kind, crate::ast::NoteKind::Hexagonal);
                assert_eq!(n.text, "hex body");
            }
            other => panic!("unexpected statement: {other:?}"),
        }
        match &doc.statements[1].kind {
            StatementKind::Note(n) => {
                assert_eq!(n.kind, crate::ast::NoteKind::Rectangle);
                assert_eq!(n.text, "rect body");
            }
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn parses_multiline_ref_with_inline_head_text() {
        let doc = parse_with_options(
            "ref over A, B: summary\nline 2\nend ref\n",
            &ParseOptions::default(),
        )
        .unwrap();

        match &doc.statements[0].kind {
            StatementKind::Group(g) => {
                assert_eq!(g.kind, "ref");
                assert_eq!(g.label.as_deref(), Some("over A, B\nsummary\nline 2"));
            }
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn rejects_malformed_arrow_syntax() {
        let err = parse_with_options("A -x B", &ParseOptions::default()).unwrap_err();
        assert!(err.message.contains("E_ARROW_INVALID"));
    }

    #[test]
    fn parses_lifecycle_shortcut_suffixes() {
        let doc = parse_with_options("A -> B++: inc", &ParseOptions::default()).unwrap();
        match &doc.statements[0].kind {
            StatementKind::Message(m) => {
                assert_eq!(m.arrow, "->@R++");
                assert_eq!(m.to, "B");
            }
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn parses_expanded_slanted_arrow_tokens() {
        let doc = parse_with_options("A -/-> B\nB -\\\\->> A\n", &ParseOptions::default()).unwrap();
        match &doc.statements[0].kind {
            StatementKind::Message(m) => assert_eq!(m.arrow, "-/->"),
            other => panic!("unexpected statement: {other:?}"),
        }
        match &doc.statements[1].kind {
            StatementKind::Message(m) => assert_eq!(m.arrow, "-\\-->>"),
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn parses_filled_virtual_endpoint_side_from_message_context() {
        let doc = parse_with_options("[*] -> A\nA -> [*]\n", &ParseOptions::default()).unwrap();
        match &doc.statements[0].kind {
            StatementKind::Message(m) => {
                let from_virtual = m.from_virtual.expect("from virtual");
                assert_eq!(from_virtual.side, crate::ast::VirtualEndpointSide::Left);
                assert_eq!(from_virtual.kind, crate::ast::VirtualEndpointKind::Filled);
            }
            other => panic!("unexpected statement: {other:?}"),
        }
        match &doc.statements[1].kind {
            StatementKind::Message(m) => {
                let to_virtual = m.to_virtual.expect("to virtual");
                assert_eq!(to_virtual.side, crate::ast::VirtualEndpointSide::Right);
                assert_eq!(to_virtual.kind, crate::ast::VirtualEndpointKind::Filled);
            }
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn parses_queue_participant_and_separator() {
        let doc = parse_with_options(
            "queue Jobs as Q\n== Processing ==\n",
            &ParseOptions::default(),
        )
        .unwrap();

        match &doc.statements[0].kind {
            StatementKind::Participant(p) => {
                assert_eq!(p.name, "Jobs");
                assert_eq!(p.alias.as_deref(), Some("Q"));
            }
            other => panic!("unexpected statement: {other:?}"),
        }
        match &doc.statements[1].kind {
            StatementKind::Separator(v) => assert_eq!(v.as_deref(), Some("Processing")),
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn parses_typed_group_end_keyword() {
        let doc =
            parse_with_options("alt branch\nA -> B\nend alt\n", &ParseOptions::default()).unwrap();

        match &doc.statements[2].kind {
            StatementKind::Group(g) => {
                assert_eq!(g.kind, "end");
                assert_eq!(g.label.as_deref(), Some("alt"));
            }
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn parses_class_bootstrap_declarations_and_relations() {
        let doc = parse_with_options(
            "class User\nclass Account as Acct\nUser --> Acct : owns\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(doc.kind, DiagramKind::Class);
        assert!(matches!(
            doc.statements[0].kind,
            StatementKind::ClassDecl(_)
        ));
        assert!(matches!(
            doc.statements[1].kind,
            StatementKind::ClassDecl(_)
        ));
        match &doc.statements[2].kind {
            StatementKind::FamilyRelation(rel) => {
                assert_eq!(rel.from, "User");
                assert_eq!(rel.to, "Acct");
                assert_eq!(rel.arrow, "-->");
                assert_eq!(rel.label.as_deref(), Some("owns"));
            }
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn parses_object_and_usecase_bootstrap_kinds() {
        let object_doc =
            parse_with_options("object Order\nobject Customer\n", &ParseOptions::default())
                .unwrap();
        assert_eq!(object_doc.kind, DiagramKind::Object);

        let usecase_doc = parse_with_options(
            "usecase Authenticate\nusecase Authorize\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(usecase_doc.kind, DiagramKind::UseCase);
    }

    #[test]
    fn parses_core_uml_broad_partial_declaration_forms() {
        let class_doc = parse_with_options(
            "interface Gateway\nabstract class Shape\nannotation Trace\nstruct Payload\nGateway -[#blue,dashed]-> Shape : adapts\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(class_doc.kind, DiagramKind::Class);
        match &class_doc.statements[0].kind {
            StatementKind::ClassDecl(decl) => {
                assert_eq!(decl.name, "Gateway");
                assert_eq!(decl.members[0].text, "<<interface>>");
            }
            other => panic!("unexpected statement: {other:?}"),
        }
        match &class_doc.statements[1].kind {
            StatementKind::ClassDecl(decl) => {
                assert_eq!(decl.name, "Shape");
                assert_eq!(decl.members[0].text, "<<abstract class>>");
            }
            other => panic!("unexpected statement: {other:?}"),
        }
        assert!(matches!(
            class_doc.statements[4].kind,
            StatementKind::FamilyRelation(_)
        ));
        match &class_doc.statements[4].kind {
            StatementKind::FamilyRelation(rel) => assert_eq!(rel.arrow, "-->"),
            other => panic!("unexpected statement: {other:?}"),
        }

        let object_doc = parse_with_options(
            "map Settings {\n  theme => light\n}\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(object_doc.kind, DiagramKind::Object);
        match &object_doc.statements[0].kind {
            StatementKind::ObjectDecl(decl) => {
                assert_eq!(decl.name, "Settings");
                assert_eq!(decl.members[0].text, "<<map>>");
                assert_eq!(decl.members[1].text, "theme => light");
            }
            other => panic!("unexpected statement: {other:?}"),
        }

        let usecase_doc = parse_with_options(
            "actor Customer as C\nusecase (Login) as UC1\nC ..> UC1 : <<include>>\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(usecase_doc.kind, DiagramKind::UseCase);
        match &usecase_doc.statements[0].kind {
            StatementKind::UseCaseDecl(decl) => {
                assert_eq!(decl.name, "Customer");
                assert_eq!(decl.alias.as_deref(), Some("C"));
                assert_eq!(decl.members[0].text, "<<actor>>");
            }
            other => panic!("unexpected statement: {other:?}"),
        }
        match &usecase_doc.statements[1].kind {
            StatementKind::UseCaseDecl(decl) => {
                assert_eq!(decl.name, "Login");
                assert_eq!(decl.alias.as_deref(), Some("UC1"));
            }
            other => panic!("unexpected statement: {other:?}"),
        }
        match &usecase_doc.statements[2].kind {
            StatementKind::FamilyRelation(rel) => {
                assert_eq!(rel.arrow, "..>");
                assert_eq!(rel.label.as_deref(), None);
                assert_eq!(rel.stereotype.as_deref(), Some("include"));
            }
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn parses_family_relations_with_tight_labels_quotes_and_cardinality() {
        let doc = parse_with_options(
            "class \"Order-Service\"\nclass \"Line-Item\"\nclass \"Price-List\"\n\"Order-Service\" \"1\" --> \"0..*\" \"Line-Item\": contains\nLine-Item --> \"Price-List\": priced by\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(doc.kind, DiagramKind::Class);
        match &doc.statements[3].kind {
            StatementKind::FamilyRelation(rel) => {
                assert_eq!(rel.from, "Order-Service");
                assert_eq!(rel.to, "Line-Item");
                assert_eq!(rel.label.as_deref(), Some("contains"));
                assert_eq!(rel.left_cardinality.as_deref(), Some("1"));
                assert_eq!(rel.right_cardinality.as_deref(), Some("0..*"));
            }
            other => panic!("unexpected statement: {other:?}"),
        }
        match &doc.statements[4].kind {
            StatementKind::FamilyRelation(rel) => {
                assert_eq!(rel.from, "Line-Item");
                assert_eq!(rel.to, "Price-List");
                assert_eq!(rel.label.as_deref(), Some("priced by"));
            }
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn parses_component_namespace_groups_and_lollipop_endpoint_cleanup() {
        let doc = parse_with_options(
            "@startuml\nnamespace Edge {\n  component API\n  interface \"Orders\" as Orders\n}\nAPI --() Orders: provides\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(doc.kind, DiagramKind::Component);
        assert!(matches!(
            doc.statements[0].kind,
            StatementKind::ClassGroup { .. }
        ));
        match &doc.statements[1].kind {
            StatementKind::FamilyRelation(rel) => {
                assert_eq!(rel.from, "API");
                assert_eq!(rel.to, "Orders");
                assert_eq!(rel.label.as_deref(), Some("provides"));
            }
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn parses_scoped_core_uml_relations_and_lollipop_endpoints() {
        let doc = parse_with_options(
            "@startuml\npackage Domain {\n  namespace Core {\n    class Api\n    class Repo\n    Api \"1\" -[#green,dashed]-> \"0..*\" Repo : owns\n  }\n}\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap();
        match &doc.statements[0].kind {
            StatementKind::ClassGroup {
                members, relations, ..
            } => {
                assert!(members.iter().any(|m| m == "Domain::Core::Api"));
                assert_eq!(relations.len(), 1);
                assert_eq!(relations[0].from, "Domain::Core::Api");
                assert_eq!(relations[0].to, "Domain::Core::Repo");
                assert_eq!(relations[0].left_cardinality.as_deref(), Some("1"));
                assert_eq!(relations[0].right_cardinality.as_deref(), Some("0..*"));
                assert_eq!(relations[0].line_color.as_deref(), Some("#008000"));
                assert!(relations[0].dashed);
            }
            other => panic!("unexpected statement: {other:?}"),
        }

        let component_doc = parse_with_options(
            "@startuml\nnamespace Edge {\n  component API\n  interface Orders\n  API --() Orders : provides\n}\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap();
        match &component_doc.statements[0].kind {
            StatementKind::ClassGroup { relations, .. } => {
                assert_eq!(relations.len(), 1);
                assert_eq!(relations[0].from, "Edge::API");
                assert_eq!(relations[0].to, "Edge::Orders");
                assert!(relations[0].right_lollipop);
            }
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn parses_sequence_decorated_arrow_styles_as_portable_arrow_core() {
        let doc = parse_with_options(
            "participant A\nparticipant B\nA -[#red,dashed]> B : styled\nB ->[#blue,dashed]> A : open styled\nA -[hidden]-> B : hidden\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(doc.kind, DiagramKind::Sequence);
        match &doc.statements[2].kind {
            StatementKind::Message(m) => {
                assert_eq!(m.arrow, "->");
                assert_eq!(m.style.color.as_deref(), Some("red"));
                assert!(m.style.dashed);
                assert!(!m.style.hidden);
            }
            other => panic!("unexpected statement: {other:?}"),
        }
        match &doc.statements[3].kind {
            StatementKind::Message(m) => {
                assert_eq!(m.arrow, "->>");
                assert_eq!(m.style.color.as_deref(), Some("blue"));
                assert!(m.style.dashed);
                assert!(!m.style.hidden);
            }
            other => panic!("unexpected statement: {other:?}"),
        }
        match &doc.statements[4].kind {
            StatementKind::Message(m) => {
                assert_eq!(m.arrow, "-->");
                assert!(m.style.hidden);
            }
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn parses_activity_switch_split_goto_and_terminal_controls() {
        let doc = parse_with_options(
            "@startuml\nstart\nswitch (kind?)\ncase (A)\n:Do A;\ncase (B)\ngoto retry\nendswitch\nif (ready?) then (yes)\nelseif (warm?) then (maybe)\nendif\nrepeat\ncontinue\nbreak\nrepeat while (again?)\nend repeat\nsplit\n:one;\nsplit again\n:two;\nend split\nlabel retry\nbackward: retry path;\ndetach\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(doc.kind, DiagramKind::Activity);
        let steps = doc
            .statements
            .iter()
            .filter_map(|stmt| match &stmt.kind {
                StatementKind::ActivityStep(step) => Some(step),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(steps
            .iter()
            .any(|step| step.kind == ActivityStepKind::IfStart
                && step.label.as_deref() == Some("switch kind?")));
        assert!(steps
            .iter()
            .any(|step| step.kind == ActivityStepKind::Else && step.label.as_deref() == Some("A")));
        assert!(steps
            .iter()
            .any(|step| step.kind == ActivityStepKind::Fork
                && step.label.as_deref() == Some("split")));
        assert!(steps
            .iter()
            .any(|step| step.kind == ActivityStepKind::Action
                && step.label.as_deref() == Some("goto retry")));
        assert!(steps.iter().any(|step| step.kind == ActivityStepKind::Else
            && step.label.as_deref() == Some("elseif warm? / maybe")));
        assert!(steps
            .iter()
            .any(|step| step.kind == ActivityStepKind::Action
                && step.label.as_deref() == Some("continue")));
        assert!(steps
            .iter()
            .any(|step| step.kind == ActivityStepKind::Action
                && step.label.as_deref() == Some("break")));
        assert!(steps
            .iter()
            .any(|step| step.kind == ActivityStepKind::Action
                && step.label.as_deref() == Some("backward retry path")));
        assert!(steps
            .iter()
            .any(|step| step.kind == ActivityStepKind::Stop
                && step.label.as_deref() == Some("detach")));
    }

    #[test]
    fn parses_family_declaration_blocks_with_members() {
        let doc = parse_with_options(
            "class User {\n  +id: UUID\n  +name: String\n}\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(doc.kind, DiagramKind::Class);
        match &doc.statements[0].kind {
            StatementKind::ClassDecl(decl) => {
                assert_eq!(decl.name, "User");
                assert_eq!(decl.members.len(), 2);
                assert_eq!(decl.members[0].text, "+id: UUID");
                assert_eq!(decl.members[0].modifier, None);
                assert_eq!(decl.members[1].text, "+name: String");
                assert_eq!(decl.members[1].modifier, None);
            }
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn unclosed_family_declaration_block_reports_deterministic_error() {
        let err = parse_with_options(
            "object Config {\nkey = \"value\"\n",
            &ParseOptions::default(),
        )
        .unwrap_err();
        assert!(err.message.contains("E_FAMILY_DECL_BLOCK_UNCLOSED"));
    }

    #[test]
    fn parses_gantt_baseline_statements() {
        let doc = parse_with_options(
            "@startgantt\n[Build]\n[Milestone] happens on 2026-05-01\n[Build] starts 2026-04-01\n[Build] requires [Design]\n@endgantt\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(doc.kind, DiagramKind::Gantt);
        assert!(matches!(
            doc.statements[0].kind,
            StatementKind::GanttTaskDecl { .. }
        ));
        assert!(matches!(
            doc.statements[1].kind,
            StatementKind::GanttMilestoneDecl {
                happens_on: Some(_),
                ..
            }
        ));
        assert!(doc
            .statements
            .iter()
            .any(|stmt| matches!(stmt.kind, StatementKind::GanttConstraint { .. })));
    }

    #[test]
    fn parses_gantt_dates_and_duration_baseline_statements() {
        let doc = parse_with_options(
            "@startgantt\nProject starts 2026-05-01\n[Build] lasts 5 days\n[Test] starts 2026-05-06 and lasts 2 weeks\n@endgantt\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(doc.kind, DiagramKind::Gantt);
        assert!(matches!(
            doc.statements[0].kind,
            StatementKind::GanttConstraint {
                ref subject,
                ref kind,
                ref target
            } if subject == "Project" && kind == "starts" && target == "2026-05-01"
        ));
        assert!(matches!(
            doc.statements[1].kind,
            StatementKind::GanttTaskDecl {
                ref name,
                duration_days: Some(5),
                ..
            } if name == "Build"
        ));
        assert!(matches!(
            doc.statements[2].kind,
            StatementKind::GanttTaskDecl {
                ref name,
                start_date: Some(ref d),
                duration_days: Some(14),
                ..
            } if name == "Test" && d == "2026-05-06"
        ));
    }

    #[test]
    fn parses_gantt_closed_weekday_calendar_statements() {
        let doc = parse_with_options(
            "@startgantt\nProject starts 2026-05-01\nsaturday are closed\nsundays are closed\n[Build] lasts 2 days\n@endgantt\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(doc.kind, DiagramKind::Gantt);
        assert!(matches!(
            doc.statements[1].kind,
            StatementKind::GanttCalendarClosed { ref day } if day == "saturday"
        ));
        assert!(matches!(
            doc.statements[2].kind,
            StatementKind::GanttCalendarClosed { ref day } if day == "sunday"
        ));
    }

    #[test]
    fn parses_gantt_closed_date_range_calendar_statement() {
        let doc = parse_with_options(
            "@startgantt\nProject starts 2026-05-01\n2026-05-04 to 2026-05-05 is closed\n[Build] lasts 2 days\n@endgantt\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(doc.kind, DiagramKind::Gantt);
        assert!(matches!(
            doc.statements[1].kind,
            StatementKind::GanttCalendarClosedDateRange {
                ref start_date,
                ref end_date
            } if start_date == "2026-05-04" && end_date == "2026-05-05"
        ));
    }

    #[test]
    fn parses_chronology_happens_on_baseline_statement() {
        let doc = parse_with_options(
            "@startchronology\nRelease happens on 2026-05-15\n@endchronology\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(doc.kind, DiagramKind::Chronology);
        assert!(matches!(
            doc.statements[0].kind,
            StatementKind::ChronologyHappensOn { .. }
        ));
    }

    #[test]
    fn parses_usecase_relations_with_alias_and_label() {
        let doc = parse_with_options(
            "usecase Authenticate as Auth\nusecase User\nAuth --> User : validates\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(doc.kind, DiagramKind::UseCase);
        match &doc.statements[2].kind {
            StatementKind::FamilyRelation(rel) => {
                assert_eq!(rel.from, "Auth");
                assert_eq!(rel.to, "User");
                assert_eq!(rel.arrow, "-->");
                assert_eq!(rel.label.as_deref(), Some("validates"));
            }
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn malformed_family_relation_is_preserved_as_unknown_statement() {
        let doc = parse_with_options("class User\nUser -->\n", &ParseOptions::default()).unwrap();
        assert_eq!(doc.kind, DiagramKind::Class);
        assert!(matches!(doc.statements[1].kind, StatementKind::Unknown(_)));
    }

    #[test]
    fn state_keyword_is_parsed_as_state_decl() {
        let doc = parse_with_options("state Running\n", &ParseOptions::default()).unwrap();
        assert_eq!(doc.kind, DiagramKind::State);
        assert!(matches!(
            doc.statements[0].kind,
            StatementKind::StateDecl(_)
        ));
    }

    #[test]
    fn mixed_family_input_reports_deterministic_error() {
        let err = parse_with_options("class A\nnewpage\n", &ParseOptions::default()).unwrap_err();
        assert!(err.message.contains("E_FAMILY_MIXED"));
    }

    #[test]
    fn start_enduml_markers_accept_optional_block_suffixes() {
        let doc = parse_with_options(
            "@startuml \"Primary\"\nA -> B: one\n@enduml anything\n@startuml Second\nB -> A: two\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(doc.kind, DiagramKind::Sequence);
        let labels = doc
            .statements
            .iter()
            .filter_map(|s| match &s.kind {
                StatementKind::Message(m) => m.label.as_deref(),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(labels, vec!["one", "two"]);
    }

    #[test]
    fn start_end_timeline_markers_accept_optional_block_suffixes() {
        let gantt = parse_with_options(
            "@startgantt \"Gantt\"\n[2026-01] : one\n@endgantt anything\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(gantt.kind, DiagramKind::Gantt);

        let chronology = parse_with_options(
            "@startchronology\nEvent\n@endchronology now\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(chronology.kind, DiagramKind::Chronology);
    }

    #[test]
    fn startmindmap_and_startwbs_markers_set_family_kind() {
        let mindmap = parse_with_options(
            "@startmindmap\n* Root\n** Child\n@endmindmap\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(mindmap.kind, DiagramKind::MindMap);

        let wbs =
            parse_with_options("@startwbs\n* Scope\n@endwbs\n", &ParseOptions::default()).unwrap();
        assert_eq!(wbs.kind, DiagramKind::Wbs);

        let gantt = parse_with_options(
            "@startgantt\n[2026-01-01] : Kickoff\n@endgantt\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(gantt.kind, DiagramKind::Gantt);

        let chronology = parse_with_options(
            "@startchronology\n2026-01-01 : Event\n@endchronology\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(chronology.kind, DiagramKind::Chronology);
    }

    #[test]
    fn parses_activity_oldstyle_baseline_statements() {
        let doc = parse_with_options(
            "@startuml\n|Build|\n(*) --> \"Init\"\n#gold:Compile;\n-->[next] right of \"Test\"\n\"Test\" --> (*)\ndetach\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(doc.kind, DiagramKind::Activity);
        assert!(!doc.statements.is_empty());
    }

    #[test]
    fn parses_old_activity_edges_as_canonical_steps() {
        let doc = parse_with_options(
            "@startuml\n(*) --> \"Step1\"\n\"Step1\" -->[ok] \"Step2\"\n\"Step2\" --> (*)\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(doc.kind, DiagramKind::Activity);
        let steps: Vec<_> = doc
            .statements
            .iter()
            .filter_map(|stmt| match &stmt.kind {
                StatementKind::ActivityStep(step) => Some((step.kind.clone(), step.label.clone())),
                _ => None,
            })
            .collect();
        assert_eq!(
            steps,
            vec![
                (ActivityStepKind::Start, None),
                (ActivityStepKind::Action, Some("Step1".to_string())),
                (ActivityStepKind::Action, Some("Step2".to_string())),
                (ActivityStepKind::Stop, None),
            ]
        );
    }

    #[test]
    fn mismatched_start_end_family_markers_report_deterministic_error() {
        let err = parse_with_options("@startmindmap\n* Root\n@endwbs\n", &ParseOptions::default())
            .unwrap_err();
        assert!(err.message.contains("E_BLOCK_MISMATCH"));
    }

    #[test]
    fn apostrophe_comments_are_ignored_but_preserved_inside_quotes() {
        let doc = parse_with_options(
            "@startuml\n' full line comment\nA -> B: \"don't split\" ' trailing comment\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(doc.kind, DiagramKind::Sequence);
        assert_eq!(doc.statements.len(), 1);
        match &doc.statements[0].kind {
            StatementKind::Message(m) => {
                assert_eq!(m.label.as_deref(), Some("\"don't split\""));
            }
            other => panic!("unexpected statement: {other:?}"),
        }
    }
}
