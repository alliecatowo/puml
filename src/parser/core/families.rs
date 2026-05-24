fn parse_component_or_deployment_core(
    lines: &[(&str, Span)],
    i: usize,
    line: &str,
    span: Span,
    allow_mixing: bool,
    detected_kind: &mut Option<DiagramKind>,
    statements: &mut Vec<Statement>,
) -> Result<Option<usize>, Diagnostic> {
    if !matches!(
        detected_kind,
        None | Some(DiagramKind::Component | DiagramKind::Deployment)
    ) {
        return Ok(None);
    }

    let ambiguous_class_interface = detected_kind.is_none()
        && line.starts_with("interface ")
        && later_lines_contain_class_family_declaration(lines, i);
    let actor_prefers_non_component = detected_kind.is_none()
        && line.starts_with("actor ")
        && (line.contains("<<")
            || line.contains('"')
            || later_lines_contain_usecase_family_declaration(lines, i)
            || later_lines_contain_sequence_family_keywords(lines, i));
    let ambiguous_sequence_participant_prefers_sequence = detected_kind.is_none()
        && component_decl_keyword(line).is_some_and(|(kw, _)| {
            is_ambiguous_sequence_participant_keyword(kw)
                && later_lines_contain_sequence_family_keywords(lines, i)
        });
    let ambiguous_activity_keyword_prefers_activity = detected_kind.is_none()
        && component_decl_keyword(line).is_some_and(|(kw, _)| {
            is_ambiguous_activity_keyword(kw) && later_lines_contain_activity_context(lines, i)
        });
    let ambiguous_usecase_prefers_usecase = detected_kind.is_none()
        && (line.starts_with("usecase ") || line.starts_with('('))
        && later_lines_contain_usecase_family_declaration(lines, i);
    let ambiguous_class_scope = detected_kind.is_none()
        && (line.starts_with("package ") || line.starts_with("namespace "))
        && line.trim_end().ends_with('{')
        && {
            let end_idx = find_scoping_block_end(lines, i);
            end_idx > i
                && !group_body_contains_component_family(lines, i, end_idx)
                && (group_body_contains_class_family(lines, i, end_idx)
                    || group_body_contains_object_family(lines, i, end_idx)
                    || group_body_contains_usecase_family(lines, i, end_idx))
        };

    if !ambiguous_class_scope {
        if let Some((kind, end_idx)) = parse_component_scoping_block(lines, i, line)? {
            let family = component_scope_family(*detected_kind, &kind);
            *detected_kind = Some(select_diagram_kind_with_mixing(
                *detected_kind,
                family,
                span,
                allow_mixing,
            )?);
            statements.push(Statement { span, kind });
            return Ok(Some(end_idx + 1));
        }
    }

    if ambiguous_class_interface
        || actor_prefers_non_component
        || ambiguous_sequence_participant_prefers_sequence
        || ambiguous_activity_keyword_prefers_activity
        || ambiguous_usecase_prefers_usecase
        || ambiguous_class_scope
    {
        return Ok(None);
    }

    if matches!(detected_kind, Some(DiagramKind::Deployment)) {
        if let Some(kind) = parse_deployment_usecase_decl(line) {
            *detected_kind = Some(select_diagram_kind_with_mixing(
                *detected_kind,
                DiagramKind::Deployment,
                span,
                allow_mixing,
            )?);
            statements.push(Statement { span, kind });
            return Ok(Some(i + 1));
        }
    }
    if let Some((kind, end_idx)) = parse_component_multiline_decl(lines, i, line)? {
        let family = if matches!(detected_kind, Some(DiagramKind::Component)) {
            DiagramKind::Component
        } else {
            DiagramKind::Deployment
        };
        *detected_kind = Some(select_diagram_kind_with_mixing(
            *detected_kind,
            family,
            span,
            allow_mixing,
        )?);
        statements.push(Statement { span, kind });
        return Ok(Some(end_idx + 1));
    }
    if let Some(kind) = parse_component_decl(line) {
        let family = component_decl_family(*detected_kind, &kind);
        *detected_kind = Some(select_diagram_kind_with_mixing(
            *detected_kind,
            family,
            span,
            allow_mixing,
        )?);
        statements.push(Statement { span, kind });
        return Ok(Some(i + 1));
    }
    Ok(None)
}

fn component_scope_family(detected_kind: Option<DiagramKind>, kind: &StatementKind) -> DiagramKind {
    let is_deployment_scope = matches!(detected_kind, Some(DiagramKind::Deployment))
        || matches!(
            kind,
            StatementKind::ClassGroup { kind, .. }
                if matches!(
                    kind.as_str(),
                    "action"
                        | "artifact"
                        | "cloud"
                        | "container"
                        | "database"
                        | "frame"
                        | "hexagon"
                        | "node"
                        | "process"
                        | "queue"
                        | "stack"
                        | "storage"
                )
        );
    if is_deployment_scope {
        DiagramKind::Deployment
    } else {
        DiagramKind::Component
    }
}

fn component_decl_family(detected_kind: Option<DiagramKind>, kind: &StatementKind) -> DiagramKind {
    if matches!(detected_kind, Some(DiagramKind::Deployment)) {
        return DiagramKind::Deployment;
    }
    match kind {
        StatementKind::ComponentDecl {
            kind:
                ComponentNodeKind::Node
                | ComponentNodeKind::Action
                | ComponentNodeKind::Agent
                | ComponentNodeKind::Artifact
                | ComponentNodeKind::Boundary
                | ComponentNodeKind::Cloud
                | ComponentNodeKind::Circle
                | ComponentNodeKind::Collections
                | ComponentNodeKind::Container
                | ComponentNodeKind::Control
                | ComponentNodeKind::Frame
                | ComponentNodeKind::Storage
                | ComponentNodeKind::Database
                | ComponentNodeKind::Entity
                | ComponentNodeKind::Package
                | ComponentNodeKind::Rectangle
                | ComponentNodeKind::Folder
                | ComponentNodeKind::File
                | ComponentNodeKind::Card
                | ComponentNodeKind::Hexagon
                | ComponentNodeKind::Label
                | ComponentNodeKind::Person
                | ComponentNodeKind::Process
                | ComponentNodeKind::Queue
                | ComponentNodeKind::Stack
                | ComponentNodeKind::UseCase,
            ..
        } => DiagramKind::Deployment,
        _ => DiagramKind::Component,
    }
}

fn parse_timeline_or_state_core(
    lines: &[(&str, Span)],
    i: usize,
    line: &str,
    span: Span,
    detected_kind: Option<DiagramKind>,
    statements: &mut Vec<Statement>,
) -> Result<Option<usize>, Diagnostic> {
    match detected_kind {
        Some(DiagramKind::Gantt) => {
            Ok(Some(parse_gantt_core_line(lines, i, line, span, statements)?))
        }
        Some(DiagramKind::Chronology) => Ok(Some(parse_chronology_core_line(
            lines, i, line, span, statements,
        ))),
        Some(DiagramKind::State) => Ok(Some(parse_state_core_line(lines, i, line, span, statements)?)),
        _ => Ok(None),
    }
}

fn parse_gantt_core_line(
    lines: &[(&str, Span)],
    i: usize,
    line: &str,
    span: Span,
    statements: &mut Vec<Statement>,
) -> Result<usize, Diagnostic> {
    if let Some(kind) = parse_gantt_baseline_statement(line) {
        statements.push(Statement { span, kind });
        return Ok(i + 1);
    }
    if let Some((kind, end_idx)) = parse_multiline_keyword_block(lines, i, line) {
        statements.push(Statement {
            span: Span::new(span.start, lines[end_idx].1.end),
            kind,
        });
        return Ok(end_idx + 1);
    }
    if let Some((kind, end_idx)) = parse_multiline_note_block(lines, i, line) {
        statements.push(Statement {
            span: Span::new(span.start, lines[end_idx].1.end),
            kind,
        });
        return Ok(end_idx + 1);
    }
    if let Some(kind) = parse_keyword(line) {
        if is_timeline_metadata_statement(&kind) {
            statements.push(Statement { span, kind });
            return Ok(i + 1);
        }
    }
    statements.push(Statement {
        span,
        kind: StatementKind::UnsupportedSyntax(format!(
            "[E_GANTT_UNSUPPORTED] unsupported gantt baseline syntax: `{line}`"
        )),
    });
    Ok(i + 1)
}

fn parse_chronology_core_line(
    lines: &[(&str, Span)],
    i: usize,
    line: &str,
    span: Span,
    statements: &mut Vec<Statement>,
) -> usize {
    if let Some(kind) = parse_chronology_baseline_statement(line) {
        statements.push(Statement { span, kind });
        return i + 1;
    }
    if let Some((kind, end_idx)) = parse_multiline_keyword_block(lines, i, line) {
        statements.push(Statement {
            span: Span::new(span.start, lines[end_idx].1.end),
            kind,
        });
        return end_idx + 1;
    }
    if let Some(kind) = parse_keyword(line) {
        if is_timeline_metadata_statement(&kind) {
            statements.push(Statement { span, kind });
            return i + 1;
        }
    }
    statements.push(Statement {
        span,
        kind: StatementKind::UnsupportedSyntax(format!(
            "[E_CHRONOLOGY_UNSUPPORTED] unsupported chronology baseline syntax: `{line}`"
        )),
    });
    i + 1
}

fn parse_state_core_line(
    lines: &[(&str, Span)],
    i: usize,
    line: &str,
    span: Span,
    statements: &mut Vec<Statement>,
) -> Result<usize, Diagnostic> {
    if let Some((kind, end_idx)) = parse_state_statement(lines, i, line)? {
        let block_span = if end_idx > i {
            Span::new(span.start, lines[end_idx].1.end)
        } else {
            span
        };
        statements.push(Statement {
            span: block_span,
            kind,
        });
        return Ok(end_idx + 1);
    }
    statements.push(Statement {
        span,
        kind: StatementKind::UnsupportedSyntax(line.to_string()),
    });
    Ok(i + 1)
}
