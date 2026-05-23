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
        if let Some((start_kind, qualifier)) = parse_start_block_kind_with_qualifier(line) {
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
            // For raw-body blocks (chart, regex, ebnf, …) emit any inline qualifier
            // after @startXxx as a synthetic first body line so the normalizer can
            // use it (e.g. `@startchart area` → subtype "area").
            if !qualifier.is_empty()
                && (is_raw_body_block(start_kind) || block_kind_is_raw_body(start_kind))
            {
                statements.push(Statement {
                    span,
                    kind: StatementKind::RawBody(qualifier.to_string()),
                });
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
                    || later_lines_contain_usecase_family_declaration(&lines, i)
                    || later_lines_contain_sequence_family_keywords(&lines, i));
            let ambiguous_sequence_participant_prefers_sequence = detected_kind.is_none()
                && component_decl_keyword(line).is_some_and(|(kw, _)| {
                    is_ambiguous_sequence_participant_keyword(kw)
                        && later_lines_contain_sequence_family_keywords(&lines, i)
                });
            let ambiguous_activity_keyword_prefers_activity = detected_kind.is_none()
                && component_decl_keyword(line).is_some_and(|(kw, _)| {
                    is_ambiguous_activity_keyword(kw) && later_lines_contain_activity_context(&lines, i)
                });
            let ambiguous_usecase_prefers_usecase = detected_kind.is_none()
                && (line.starts_with("usecase ") || line.starts_with('('))
                && later_lines_contain_usecase_family_declaration(&lines, i);
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
                let is_deployment_scope = matches!(detected_kind, Some(DiagramKind::Deployment))
                    || matches!(
                    &kind,
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
                let family = if is_deployment_scope {
                    DiagramKind::Deployment
                } else {
                    DiagramKind::Component
                };
                detected_kind = Some(select_diagram_kind(detected_kind, family, span)?);
                statements.push(Statement { span, kind });
                i = end_idx + 1;
                continue;
            }
            if !ambiguous_class_interface
                && !actor_prefers_non_component
                && !ambiguous_sequence_participant_prefers_sequence
                && !ambiguous_activity_keyword_prefers_activity
                && !ambiguous_usecase_prefers_usecase
                && !ambiguous_class_scope
            {
                if let Some((kind, end_idx)) = parse_component_multiline_decl(&lines, i, line)? {
                    let family = if matches!(detected_kind, Some(DiagramKind::Component)) {
                        DiagramKind::Component
                    } else {
                        DiagramKind::Deployment
                    };
                    detected_kind = Some(select_diagram_kind(detected_kind, family, span)?);
                    statements.push(Statement { span, kind });
                    i = end_idx + 1;
                    continue;
                }
                if let Some(kind) = parse_component_decl(line) {
                    let family = if matches!(detected_kind, Some(DiagramKind::Deployment)) {
                        DiagramKind::Deployment
                    } else {
                        match &kind {
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
            if let Some((kind, end_idx)) = parse_multiline_note_block(&lines, i, line) {
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
/// Parse a skinparam block: `skinparam <prefix> { Key Value ... }`.
///
/// Expands the block into individual `SkinParam` statements where the key is
/// the concatenation of `prefix + innerKey`. For example:
/// ```text
/// skinparam class {
///   BackgroundColor Yellow
///   BorderColor<<Abstract>> Red
/// }
/// ```
/// becomes:
/// - `SkinParam { key: "classBackgroundColor", value: "Yellow" }`
/// - `SkinParam { key: "classBackgroundColor<<Abstract>>", value: "Yellow" }` (if stereotype scoped)
///
/// Returns `None` if the line is not a block-form skinparam opener.
fn parse_skinparam_block(
    lines: &[(&str, Span)],
    start_idx: usize,
    line: &str,
) -> Option<(Vec<StatementKind>, usize)> {
    let lower = line.to_ascii_lowercase();
    let rest = lower.strip_prefix("skinparam ")?;
    // The block form ends with `{` (possibly separated by whitespace or not).
    let rest_trimmed = rest.trim_end();
    if !rest_trimmed.ends_with('{') {
        return None;
    }
    // Extract the prefix: everything between "skinparam " and the final `{`.
    let prefix_raw = rest_trimmed.trim_end_matches('{').trim();
    if prefix_raw.is_empty() {
        return None;
    }
    // Preserve original casing from the source line for the prefix.
    let original_rest = line["skinparam ".len()..].trim_end();
    let original_prefix = original_rest.trim_end_matches('{').trim();

    // Scan for the closing `}`.
    let mut kinds: Vec<StatementKind> = Vec::new();
    let mut end_idx = start_idx;
    for (idx, (raw, _span)) in lines.iter().enumerate().skip(start_idx + 1) {
        let inner = strip_inline_plantuml_comment(raw).trim();
        if inner == "}" {
            end_idx = idx;
            break;
        }
        if inner.is_empty() {
            continue;
        }
        // Each inner line is expected to be: `InnerKey Value` or just be ignored.
        // Split on the first whitespace to get key and value parts.
        let (inner_key, inner_value) = inner
            .split_once(|c: char| c.is_whitespace())
            .map(|(k, v)| (k.trim(), v.trim()))
            .unwrap_or((inner, ""));
        if inner_key.is_empty() {
            continue;
        }
        // Combine prefix with inner key: "class" + "BackgroundColor" → "classBackgroundColor".
        // Handle stereotype-scoped inner keys: "BackgroundColor<<Abstract>>" stays as-is after prefix.
        let combined_key = format!("{original_prefix}{inner_key}");
        kinds.push(StatementKind::SkinParam {
            key: combined_key,
            value: inner_value.to_string(),
        });
        // Track the last line we successfully read as end_idx
        end_idx = idx;
    }
    Some((kinds, end_idx))
}

/// Parse a minimal PlantUML `<style>...</style>` block and map sequence-only
/// style rules to equivalent `SkinParam` statements.
///
/// Supported subset:
/// - `sequenceDiagram { ... }`
/// - optional nested selectors under sequenceDiagram:
///   - `participant { ... }`
///   - `note { ... }`
///   - `group { ... }`
/// - declarations in `Property Value` or `Property: Value;` form
fn parse_style_block(
    lines: &[(&str, Span)],
    start_idx: usize,
    line: &str,
) -> Result<Option<(Vec<StatementKind>, usize)>, Diagnostic> {
    if !line.eq_ignore_ascii_case("<style>") {
        return Ok(None);
    }

    let mut kinds: Vec<StatementKind> = Vec::new();
    let mut in_sequence = false;
    let mut nested_selector: Option<String> = None;

    for (idx, (raw, _span)) in lines.iter().enumerate().skip(start_idx + 1) {
        let inner = strip_inline_plantuml_comment(raw).trim();
        if inner.eq_ignore_ascii_case("</style>") {
            return Ok(Some((kinds, idx)));
        }
        if inner.is_empty() {
            continue;
        }
        if inner.eq_ignore_ascii_case("sequenceDiagram {") {
            in_sequence = true;
            nested_selector = None;
            continue;
        }
        if inner == "}" {
            if nested_selector.is_some() {
                nested_selector = None;
            } else {
                in_sequence = false;
            }
            continue;
        }
        if !in_sequence {
            continue;
        }
        if inner.ends_with('{') {
            let selector = inner.trim_end_matches('{').trim().to_ascii_lowercase();
            nested_selector = Some(selector);
            continue;
        }

        let (raw_key, raw_value) = inner
            .split_once(':')
            .or_else(|| inner.split_once(|c: char| c.is_whitespace()))
            .map(|(k, v)| (k.trim(), v.trim()))
            .unwrap_or((inner, ""));
        if raw_key.is_empty() || raw_value.is_empty() {
            continue;
        }
        let value = raw_value.trim_end_matches(';').trim();
        if value.is_empty() {
            continue;
        }

        let key = match nested_selector.as_deref() {
            None => match raw_key.to_ascii_lowercase().as_str() {
                "arrowcolor" => Some("ArrowColor".to_string()),
                "lifelinebordercolor" => Some("LifelineBorderColor".to_string()),
                "backgroundcolor" => Some("BackgroundColor".to_string()),
                _ => None,
            },
            Some("participant") => match raw_key.to_ascii_lowercase().as_str() {
                "backgroundcolor" => Some("ParticipantBackgroundColor".to_string()),
                "bordercolor" => Some("ParticipantBorderColor".to_string()),
                "fontcolor" => Some("ParticipantFontColor".to_string()),
                _ => None,
            },
            Some("note") => match raw_key.to_ascii_lowercase().as_str() {
                "backgroundcolor" => Some("NoteBackgroundColor".to_string()),
                "bordercolor" => Some("NoteBorderColor".to_string()),
                _ => None,
            },
            Some("group") => match raw_key.to_ascii_lowercase().as_str() {
                "backgroundcolor" => Some("GroupBackgroundColor".to_string()),
                "bordercolor" => Some("GroupBorderColor".to_string()),
                "headerfontcolor" => Some("GroupHeaderFontColor".to_string()),
                "headerfontstyle" => Some("GroupHeaderFontStyle".to_string()),
                _ => None,
            },
            Some(_) => None,
        };

        if let Some(key) = key {
            kinds.push(StatementKind::SkinParam {
                key,
                value: value.to_string(),
            });
        }
    }

    Err(Diagnostic::error(
        "[E_STYLE_BLOCK_UNCLOSED] `<style>` block is missing closing `</style>`",
    )
    .with_span(lines[start_idx].1))
}

fn parse_sprite_statement(
    lines: &[(&str, Span)],
    start_idx: usize,
    line: &str,
) -> Result<Option<(StatementKind, usize)>, Diagnostic> {
    let Some(rest) = line.strip_prefix("sprite ") else {
        return Ok(None);
    };
    let mut parts = rest.split_whitespace();
    let Some(raw_name) = parts.next() else {
        return Err(Diagnostic::error("[E_SPRITE_INVALID] sprite name is missing")
            .with_span(lines[start_idx].1));
    };
    let after_name = rest[raw_name.len()..].trim();
    if after_name.is_empty() {
        return Err(Diagnostic::error("[E_SPRITE_INVALID] sprite body is missing")
            .with_span(lines[start_idx].1));
    }

    if after_name.starts_with("jar:") {
        let sprite = crate::sprites::builtin_sprite(raw_name, after_name);
        return Ok(Some((StatementKind::SpriteDef(sprite), start_idx)));
    }

    if after_name.to_ascii_lowercase().starts_with("<svg") {
        let mut svg_lines = vec![after_name.to_string()];
        let mut end_idx = start_idx;
        if !after_name.to_ascii_lowercase().contains("</svg>") {
            let mut found = false;
            for (idx, (raw, _span)) in lines.iter().enumerate().skip(start_idx + 1) {
                svg_lines.push((*raw).to_string());
                end_idx = idx;
                if raw.to_ascii_lowercase().contains("</svg>") {
                    found = true;
                    break;
                }
            }
            if !found {
                return Err(Diagnostic::error(
                    "[E_SPRITE_INVALID] inline SVG sprite is missing </svg>",
                )
                .with_span(lines[start_idx].1));
            }
        }
        let sprite = crate::sprites::parse_svg_sprite(raw_name, &svg_lines.join("\n"))
            .map_err(|d| d.with_span(lines[start_idx].1))?;
        return Ok(Some((StatementKind::SpriteDef(sprite), end_idx)));
    }

    let (spec, after_spec) = if after_name.starts_with('[') {
        let Some(close) = after_name.find(']') else {
            return Err(Diagnostic::error("[E_SPRITE_INVALID] sprite size spec is not closed")
                .with_span(lines[start_idx].1));
        };
        (&after_name[..=close], after_name[close + 1..].trim())
    } else {
        ("", after_name)
    };
    let parsed_spec = if spec.is_empty() {
        None
    } else {
        Some(crate::sprites::parse_sprite_header_spec(spec).ok_or_else(|| {
            Diagnostic::error(format!(
                "[E_SPRITE_INVALID] invalid sprite size/depth spec `{spec}`"
            ))
            .with_span(lines[start_idx].1)
        })?)
    };

    if let Some(first_payload) = after_spec.strip_prefix('{') {
        let mut rows: Vec<String> = Vec::new();
        let mut end_idx = start_idx;
        let inline_after_open = first_payload.trim();
        if let Some(before_close) = inline_after_open.strip_suffix('}') {
            let compact = before_close.trim();
            if !compact.is_empty() {
                rows.extend(compact.split_whitespace().map(str::to_string));
            }
        } else {
            if !inline_after_open.is_empty() {
                rows.extend(inline_after_open.split_whitespace().map(str::to_string));
            }
            let mut found = false;
            for (idx, (raw, span)) in lines.iter().enumerate().skip(start_idx + 1) {
                let trimmed = strip_inline_plantuml_comment(raw).trim();
                if trimmed == "}" {
                    end_idx = idx;
                    found = true;
                    break;
                }
                if let Some(before_close) = trimmed.strip_suffix('}') {
                    let compact = before_close.trim();
                    if !compact.is_empty() {
                        rows.extend(compact.split_whitespace().map(str::to_string));
                    }
                    end_idx = idx;
                    found = true;
                    break;
                }
                if trimmed.is_empty() {
                    end_idx = idx;
                    continue;
                }
                if trimmed.chars().any(char::is_whitespace) {
                    return Err(Diagnostic::error(
                        "[E_SPRITE_INVALID] sprite rows cannot contain whitespace",
                    )
                    .with_span(*span));
                }
                rows.push(trimmed.to_string());
                end_idx = idx;
            }
            if !found {
                return Err(Diagnostic::error(
                    "[E_SPRITE_INVALID] sprite block is missing closing `}`",
                )
                .with_span(lines[start_idx].1));
            }
        }
        let (width, height, levels, _compressed) =
            parsed_spec.unwrap_or((0, 0, 16, false));
        let sprite = crate::sprites::parse_hex_grid_sprite(
            raw_name,
            (width > 0).then_some(width),
            (height > 0).then_some(height),
            levels,
            &rows,
        )
        .map_err(|d| d.with_span(lines[start_idx].1))?;
        return Ok(Some((StatementKind::SpriteDef(sprite), end_idx)));
    }

    if let Some((width, height, levels, compressed)) = parsed_spec {
        if after_spec.is_empty() {
            return Err(Diagnostic::error("[E_SPRITE_INVALID] encoded sprite payload is missing")
                .with_span(lines[start_idx].1));
        }
        let sprite = crate::sprites::parse_packed_sprite(
            raw_name,
            width,
            height,
            levels,
            compressed,
            after_spec,
        )
        .map_err(|d| d.with_span(lines[start_idx].1))?;
        return Ok(Some((StatementKind::SpriteDef(sprite), start_idx)));
    }

    Err(Diagnostic::error(format!(
        "[E_SPRITE_INVALID] unsupported sprite syntax `{line}`"
    ))
    .with_span(lines[start_idx].1))
}
