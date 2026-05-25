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
    let mut allow_mixing = false;
    let mut block_kind: Option<BlockKind> = None;
    let mut block_start_span: Option<Span> = None;
    let mut i = 0usize;
    while i < lines.len() {
        let (raw_line, span) = lines[i];
        let line = strip_inline_plantuml_comment(raw_line).trim();
        if parse_raw_body_block_line(
            &mut statements,
            raw_line,
            span,
            &mut in_block,
            &mut block_kind,
            &mut block_start_span,
        )? {
            i += 1;
            continue;
        }

        if line.is_empty()
            || (line.starts_with('"')
                && split_family_arrow(line).is_none()
                && split_arrow(line).is_none())
        {
            i += 1;
            continue;
        }
        if let Some((kind, end_idx)) = parse_sprite_statement(&lines, i, line)? {
            statements.push(Statement {
                span: Span::new(span.start, lines[end_idx].1.end),
                kind,
            });
            i = end_idx + 1;
            continue;
        }
        if line.eq_ignore_ascii_case("listsprite") || line.eq_ignore_ascii_case("listsprites") {
            detected_kind = Some(select_diagram_kind(
                detected_kind,
                DiagramKind::Sequence,
                span,
            )?);
            statements.push(Statement {
                span,
                kind: StatementKind::ListSprites,
            });
            i += 1;
            continue;
        }
        if line.eq_ignore_ascii_case("stdlib") {
            detected_kind = Some(select_diagram_kind(
                detected_kind,
                DiagramKind::Stdlib,
                span,
            )?);
            statements.push(Statement {
                span,
                kind: StatementKind::StdlibInventory,
            });
            i += 1;
            continue;
        }
        if parse_block_boundary_line(
            &mut statements,
            line,
            span,
            &mut detected_kind,
            &mut in_block,
            &mut block_kind,
            &mut block_start_span,
        )? {
            i += 1;
            continue;
        }

        // Skinparam block form: `skinparam <prefix> { Key Value ... }`
        // Expand to individual SkinParam statements with concatenated keys.
        if let Some((skinparam_kinds, end_idx)) = parse_skinparam_block(&lines, i, line) {
            for kind in skinparam_kinds {
                statements.push(Statement { span, kind });
            }
            i = end_idx + 1;
            continue;
        }
        if let Some((style_kinds, end_idx)) = parse_style_block(&lines, i, line)? {
            for kind in style_kinds {
                statements.push(Statement { span, kind });
            }
            i = end_idx + 1;
            continue;
        }

        if let Some(kind) = parse_keyword(line) {
            if matches!(kind, StatementKind::AllowMixing) {
                allow_mixing = true;
                statements.push(Statement { span, kind });
                i += 1;
                continue;
            }
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
                && !(matches!(detected_kind, Some(DiagramKind::Gantt))
                    && matches!(&kind, StatementKind::Note(_)))
                && !multiline_note_head
                && !multiline_text_head
            {
                statements.push(Statement { span, kind });
                i += 1;
                continue;
            }
            if detected_kind.is_none()
                && is_family_common_keyword_before_detection(&kind)
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

        if let Some(next_i) = parse_component_or_deployment_core(
            &lines,
            i,
            line,
            span,
            allow_mixing,
            &mut detected_kind,
            &mut statements,
        )? {
            i = next_i;
            continue;
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
            detected_kind = Some(select_diagram_kind_with_mixing(
                detected_kind,
                projection_family,
                span,
                allow_mixing,
            )?);
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
                detected_kind = Some(select_diagram_kind_with_mixing(
                    detected_kind,
                    family,
                    span,
                    allow_mixing,
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

        if allow_mixing
            && detected_kind.is_some_and(crate::registry::is_mixed_graph_family)
            && !matches!(
                detected_kind,
                None | Some(DiagramKind::Class | DiagramKind::Object | DiagramKind::UseCase)
            )
        {
            if let Some((kind, end_idx)) = parse_family_declaration(&lines, i, line)? {
                let family = family_for_declaration(&kind);
                detected_kind = Some(select_diagram_kind_with_mixing(
                    detected_kind,
                    family,
                    span,
                    allow_mixing,
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

        if let Some(kind) = parse_family_visibility_control(line, detected_kind) {
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
                let ambiguous_sequence_participant = matches!(kind, DiagramKind::Deployment)
                    && component_decl_keyword(line).is_some_and(|(kw, _)| {
                        is_ambiguous_sequence_participant_keyword(kw)
                            && later_lines_contain_sequence_family_keywords(&lines, i)
                    });
                if !ambiguous_sequence_participant {
                    detected_kind = Some(kind);
                }
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
            if matches!(detected_kind, Some(DiagramKind::Deployment)) {
                if let Some(kind) = parse_deployment_usecase_decl(line) {
                    statements.push(Statement { span, kind });
                    i += 1;
                    continue;
                }
            }
            if let Some((kind, end_idx)) = parse_component_scoping_block(&lines, i, line)? {
                statements.push(Statement { span, kind });
                i = end_idx + 1;
                continue;
            }
            if let Some((kind, end_idx)) = parse_component_multiline_decl(&lines, i, line)? {
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
            if let Some((kind, end_idx)) = parse_activity_multiline_note_block(&lines, i, line) {
                statements.push(Statement {
                    span: Span::new(span.start, lines[end_idx].1.end),
                    kind,
                });
                i = end_idx + 1;
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

        if let Some(next_i) = parse_timeline_or_state_core(
            &lines,
            i,
            line,
            span,
            detected_kind,
            &mut statements,
        )? {
            i = next_i;
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
            kind: StatementKind::UnsupportedSyntax(line.to_string()),
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
