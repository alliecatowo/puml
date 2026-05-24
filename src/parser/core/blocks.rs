fn parse_raw_body_block_line(
    statements: &mut Vec<Statement>,
    raw_line: &str,
    span: Span,
    in_block: &mut bool,
    block_kind: &mut Option<BlockKind>,
    block_start_span: &mut Option<Span>,
) -> Result<bool, Diagnostic> {
    let Some(bk) = *block_kind else {
        return Ok(false);
    };
    if !(is_raw_body_block(bk) || block_kind_is_raw_body(bk)) {
        return Ok(false);
    }

    if let Some(end_kind) = parse_end_block_kind(raw_line.trim()) {
        if *block_kind == Some(end_kind) {
            *in_block = false;
            *block_kind = None;
            *block_start_span = None;
            return Ok(true);
        }
        return Err(Diagnostic::error(format!(
            "[E_BLOCK_MISMATCH] closing marker `@end{}` does not match opening `@start{}`",
            block_kind_name(end_kind),
            block_kind_name(bk)
        ))
        .with_span(span));
    }

    statements.push(Statement {
        span,
        kind: StatementKind::RawBody(raw_line.to_string()),
    });
    Ok(true)
}

fn parse_block_boundary_line(
    statements: &mut Vec<Statement>,
    line: &str,
    span: Span,
    detected_kind: &mut Option<DiagramKind>,
    in_block: &mut bool,
    block_kind: &mut Option<BlockKind>,
    block_start_span: &mut Option<Span>,
) -> Result<bool, Diagnostic> {
    if let Some((start_kind, qualifier)) = parse_start_block_kind_with_qualifier(line) {
        if *in_block {
            return Err(Diagnostic::error(
                "unmatched @startuml/@enduml boundary: found new @start marker before closing previous block",
            )
            .with_span(span));
        }
        *in_block = true;
        *block_kind = Some(start_kind);
        *block_start_span = Some(span);
        if let Some(candidate) = start_block_family(start_kind) {
            *detected_kind = Some(select_diagram_kind(*detected_kind, candidate, span)?);
        }
        if !qualifier.is_empty() && (is_raw_body_block(start_kind) || block_kind_is_raw_body(start_kind)) {
            statements.push(Statement {
                span,
                kind: StatementKind::RawBody(qualifier.to_string()),
            });
        }
        return Ok(true);
    }

    let Some(end_kind) = parse_end_block_kind(line) else {
        return Ok(false);
    };
    if !*in_block {
        return Err(Diagnostic::error(
            "unmatched @startuml/@enduml boundary: found @end marker without a preceding @startuml",
        )
        .with_span(span));
    }
    if *block_kind != Some(end_kind) {
        return Err(Diagnostic::error(format!(
            "[E_BLOCK_MISMATCH] closing marker `@end{}` does not match opening `@start{}`",
            block_kind_name(end_kind),
            block_kind_name(block_kind.unwrap_or(BlockKind::Uml))
        ))
        .with_span(span));
    }
    *in_block = false;
    *block_kind = None;
    *block_start_span = None;
    Ok(true)
}
