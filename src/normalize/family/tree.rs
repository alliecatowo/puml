use super::*;
use crate::ast::RawSyntaxCategory;
use crate::normalize::common::{self, CommonDirectives, LegendTextMode, RawSyntaxContext};

pub(super) fn normalize_family_tree(document: Document) -> Result<FamilyDocument, Diagnostic> {
    let mut nodes = Vec::new();
    let mut relations = Vec::new();
    let mut common = CommonDirectives::default();

    let family_kind = document.kind;
    let mut warnings = Vec::new();
    let mut orientation = FamilyOrientation::TopToBottom;
    let mut style = SequenceStyle::default();
    let mut monochrome_mode = None;
    let mut text_overflow_policy = TextOverflowPolicy::WrapAndGrow;
    let mut maximum_width: Option<i32> = None;
    let mut mindmap_style = MindMapStyle::default();
    let mut mindmap_style_block: Option<String> = None;
    let mut sprites = crate::sprites::SpriteRegistry::new();
    let mut list_sprites = false;
    // MindMap: track whether subsequent depth-1 nodes should go on the left side.
    let mut mindmap_left_side_mode = false;
    let mut mindmap_multiline: Option<MindmapMultilineDraft> = None;

    for stmt in document.statements {
        match stmt.kind {
            StatementKind::SpriteDef(sprite) => {
                sprites.insert(sprite.name.clone(), sprite);
            }
            StatementKind::ListSprites => {
                list_sprites = true;
            }
            StatementKind::Title(v) => common.title(v),
            StatementKind::Header(v) => common.raw_header(v),
            StatementKind::Footer(v) => common.raw_footer(v),
            StatementKind::Caption(v) => common.caption(v),
            StatementKind::Legend(v) => common.legend(v, LegendTextMode::Raw),
            StatementKind::Mainframe(v) => common.mainframe(v),
            StatementKind::Scale(body) => common.scale(&body),
            StatementKind::SkinParam { key, value } => {
                if handle_family_overflow_skinparam(
                    &key,
                    &value,
                    &mut text_overflow_policy,
                    &mut warnings,
                    stmt.span,
                ) {
                    continue;
                }
                if family_kind == DiagramKind::MindMap
                    && handle_mindmap_maximum_width_skinparam(
                        &key,
                        &value,
                        &mut maximum_width,
                        &mut warnings,
                        stmt.span,
                    )
                {
                    continue;
                }
                match classify_sequence_skinparam(&key, &value) {
                    SequenceSkinParamSupport::SupportedNoop => {}
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::FootboxVisible(_),
                    ) => {}
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::ArrowColor(color),
                    ) => {
                        style.arrow_color = color;
                    }
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::LifelineBorderColor(color),
                    ) => {
                        style.lifeline_border_color = color;
                    }
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::ParticipantBackgroundColor(color),
                    ) => {
                        style.participant_background_color = color;
                    }
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::ParticipantBorderColor(color),
                    ) => {
                        style.participant_border_color = color;
                    }
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::ParticipantFontColor(color),
                    ) => {
                        style.participant_font_color = Some(color);
                    }
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::NoteBackgroundColor(color),
                    ) => {
                        style.note_background_color = color;
                    }
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::NoteBorderColor(color),
                    ) => {
                        style.note_border_color = color;
                    }
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::GroupBackgroundColor(color),
                    ) => {
                        style.group_background_color = color;
                    }
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::GroupBorderColor(color),
                    ) => {
                        style.group_border_color = color;
                    }
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::RoundCorner(n),
                    ) => {
                        style.round_corner = n;
                    }
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::Shadowing(s),
                    ) => {
                        style.shadowing = s;
                    }
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::DefaultFontName(name),
                    ) => {
                        style.default_font_name = Some(name);
                    }
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::DefaultFontSize(sz),
                    ) => {
                        style.default_font_size = Some(sz);
                    }
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::BackgroundColor(color),
                    ) => {
                        style.background_color = Some(color);
                    }
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::DefaultTextAlignment(align),
                    ) => {
                        style.text_alignment = align;
                    }
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::ParticipantPadding(n),
                    ) => {
                        style.participant_padding = Some(n);
                    }
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::BoxPadding(n),
                    ) => {
                        style.box_padding = Some(n);
                    }
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::MessageAlign(a),
                    ) => {
                        style.message_align = a;
                    }
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::ResponseMessageBelowArrow(b),
                    ) => {
                        style.response_message_below_arrow = b;
                    }
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::LifelineThickness(n),
                    ) => {
                        style.lifeline_thickness = Some(n);
                    }
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::MessageLineColor(c),
                    ) => {
                        style.message_line_color = Some(c);
                    }
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::ReferenceBackgroundColor(c),
                    ) => {
                        style.reference_background_color = Some(c);
                    }
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::ReferenceBorderColor(c),
                    ) => {
                        style.reference_border_color = Some(c);
                    }
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::GroupHeaderFontColor(c),
                    ) => {
                        style.group_header_font_color = Some(c);
                    }
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::GroupHeaderFontStyle(s),
                    ) => {
                        style.group_header_font_style = s;
                    }
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::Monochrome(mode),
                    ) => {
                        monochrome_mode = Some(mode);
                    }
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::Handwritten(enabled),
                    ) => {
                        style.hand_drawn = enabled;
                    }
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::LifelineNoSolid(nosolid),
                    ) => {
                        style.lifeline_nosolid = nosolid;
                    }
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::Sepia(enabled),
                    ) => {
                        style.sepia = enabled;
                    }
                    SequenceSkinParamSupport::UnsupportedValue => {
                        warnings.push(
                            Diagnostic::warning(format!(
                                "[W_SKINPARAM_UNSUPPORTED_VALUE] unsupported value `{}` for skinparam `{}`",
                                value, key
                            ))
                            .with_span(stmt.span),
                        );
                    }
                    SequenceSkinParamSupport::UnsupportedKey => {
                        warnings.push(
                            Diagnostic::warning(format!(
                                "[W_SKINPARAM_UNSUPPORTED] unsupported skinparam `{}`",
                                key
                            ))
                            .with_span(stmt.span),
                        );
                    }
                }
            }
            StatementKind::StyleParam {
                selector,
                property,
                key,
                value,
            } => {
                if let Some(key) = key {
                    match classify_sequence_skinparam(&key, &value) {
                        SequenceSkinParamSupport::SupportedNoop
                        | SequenceSkinParamSupport::SupportedWithValue(_) => {}
                        SequenceSkinParamSupport::UnsupportedValue => warnings.push(
                            common::unsupported_skinparam_value_warning(&key, &value, stmt.span),
                        ),
                        SequenceSkinParamSupport::UnsupportedKey => {
                            warnings.push(common::unsupported_skinparam_warning(&key, stmt.span))
                        }
                    }
                } else {
                    warnings.push(common::unsupported_style_warning(
                        selector.as_deref(),
                        &property,
                        stmt.span,
                    ));
                }
            }
            StatementKind::Theme(value) => {
                style = resolve_sequence_theme_preset(&value)
                    .map_err(|msg| Diagnostic::error(msg).with_span(stmt.span))?
                    .style;
                if matches!(family_kind, DiagramKind::MindMap | DiagramKind::Wbs) {
                    mindmap_style = mindmap_style_from_sequence_theme(&style);
                }
            }
            StatementKind::Pragma(v) => {
                let trimmed = v.trim();
                let lower = trimmed.to_ascii_lowercase();
                if lower.starts_with("teoz ") || lower == "teoz" {
                    // Accept teoz pragma as a deterministic no-op compatibility hint.
                } else if lower == "sequencemessagespan true"
                    || lower == "sequence message span true"
                {
                    style.sequence_message_span = true;
                } else if lower == "sequencemessagespan false"
                    || lower == "sequence message span false"
                {
                    style.sequence_message_span = false;
                } else {
                    warnings.push(
                        Diagnostic::warning(format!(
                            "[W_PRAGMA_UNSUPPORTED] unsupported pragma `{}`",
                            trimmed
                        ))
                        .with_span(stmt.span),
                    );
                }
            }
            StatementKind::FamilyRelation(rel) => {
                relations.push(model_relation_from_ast(rel)?);
            }
            kind if kind.raw_syntax().is_some() => {
                let raw = kind.raw_syntax().expect("raw syntax guard");
                let line = raw.line;
                let raw_category = raw.category;
                if line.trim().is_empty() {
                    continue;
                }
                match raw_category {
                    RawSyntaxCategory::Malformed => {
                        // Parser-bug signal: remain a hard error.
                        return Err(common::raw_syntax_diagnostic(
                            raw,
                            stmt.span,
                            RawSyntaxContext::Family(family_kind),
                        ));
                    }
                    RawSyntaxCategory::LegacyUnknown => {
                        // Graceful degradation: skip the unknown line and emit a
                        // non-fatal feature-loss warning so the valid remainder renders.
                        warnings.push(common::raw_syntax_feature_loss_warning(
                            raw,
                            stmt.span,
                            RawSyntaxContext::Family(family_kind),
                        ));
                        continue;
                    }
                    RawSyntaxCategory::BenignPassthrough => {
                        if let Some(value) = parse_family_orientation_directive(line) {
                            orientation = value;
                            continue;
                        }
                    }
                    RawSyntaxCategory::Unsupported
                    | RawSyntaxCategory::Deferred
                    | RawSyntaxCategory::CommentLowered => {}
                }
                if family_kind == DiagramKind::MindMap
                    && collect_mindmap_style_line(
                        line,
                        &mut mindmap_style_block,
                        &mut mindmap_style,
                    )
                {
                    continue;
                }
                // MindMap `left side` / `right side` keyword switches which side
                // subsequent depth-1 nodes appear on when no explicit +/- prefix.
                if family_kind == DiagramKind::MindMap {
                    let lower = line.trim().to_ascii_lowercase();
                    if lower == "left side" {
                        mindmap_left_side_mode = true;
                        continue;
                    } else if lower == "right side" {
                        mindmap_left_side_mode = false;
                        continue;
                    }
                }
                if let Some(ref mut draft) = mindmap_multiline {
                    if let Some(node) = draft.append_line(line) {
                        nodes.push(node);
                        mindmap_multiline = None;
                    }
                    continue;
                }
                if let Some(mut node_info) = parse_mindmap_or_wbs_node(line) {
                    let kind = match family_kind {
                        DiagramKind::MindMap => FamilyNodeKind::MindMap,
                        DiagramKind::Wbs => FamilyNodeKind::Wbs,
                        _ => FamilyNodeKind::Salt,
                    };
                    // Apply left-side mode: if depth >= 1 and no explicit +/-
                    // prefix was given (we detect this by checking if the original
                    // line had a prefix), use the current mode.
                    if family_kind == DiagramKind::MindMap && node_info.depth >= 1 {
                        let has_explicit = line.trim_start().starts_with('+')
                            || line.trim_start().starts_with('-');
                        if !has_explicit && mindmap_left_side_mode {
                            node_info.side = MindMapSide::Left;
                        }
                    }
                    if let Some(body) = node_info.name.strip_prefix(':') {
                        let first = body.trim_start();
                        if !first.contains(';') {
                            mindmap_multiline = Some(MindmapMultilineDraft {
                                kind,
                                depth: node_info.depth,
                                name: first.to_string(),
                                alias: node_info.alias.clone(),
                                side: node_info.side,
                                checkbox: node_info.checkbox,
                                fill_color: node_info.fill_color,
                            });
                            continue;
                        }
                        node_info.name = first.trim_end_matches(';').trim_end().to_string();
                    }
                    nodes.push(FamilyNode {
                        kind,
                        name: node_info.name,
                        alias: node_info.alias,
                        members: Vec::new(),
                        depth: node_info.depth,
                        label: None,
                        mindmap_side: node_info.side,
                        wbs_checkbox: node_info.checkbox,
                        fill_color: node_info.fill_color,
                    });
                    continue;
                }
                // Unsupported syntax that doesn't parse as a node: graceful degradation.
                // Deferred/CommentLowered that reach here remain hard errors (parser bugs).
                if raw_category == RawSyntaxCategory::Unsupported {
                    // raw is Copy so still available here.
                    warnings.push(common::raw_syntax_feature_loss_warning(
                        raw,
                        stmt.span,
                        RawSyntaxContext::Family(family_kind),
                    ));
                    continue;
                }
                // BenignPassthrough (unconsumed), Deferred, or CommentLowered: hard error.
                // raw is Copy so still available here.
                return Err(common::raw_syntax_diagnostic(
                    raw,
                    stmt.span,
                    RawSyntaxContext::Family(family_kind),
                ));
            }
            _ => {
                return Err(Diagnostic::error(format!(
                    "[E_FAMILY_STUB_UNSUPPORTED] unsupported {} syntax in bootstrap slice",
                    family_kind_name(family_kind)
                ))
                .with_span(stmt.span));
            }
        }
    }

    build_family_tree_relations(&mut nodes, &mut relations);
    normalize_family_tree_warnings(&mut warnings);
    if let Some(mode) = monochrome_mode {
        apply_monochrome_to_sequence_style(&mut style, mode);
    }
    let family_style = if matches!(family_kind, DiagramKind::MindMap | DiagramKind::Wbs)
        && !mindmap_style.depth_styles.is_empty()
    {
        Some(FamilyStyle::MindMap(mindmap_style))
    } else {
        None
    };

    Ok(FamilyDocument {
        kind: family_kind,
        nodes,
        relations,
        title: common.title,
        header: common.header,
        footer: common.footer,
        caption: common.caption,
        legend: common.legend,
        mainframe: common.mainframe,
        scale: common.scale,
        orientation,
        style,
        family_style,
        text_overflow_policy,
        maximum_width,
        sprites,
        list_sprites,
        warnings,
        groups: Vec::new(),
        json_projections: Vec::new(),
        hide_options: std::collections::BTreeSet::new(),
        namespace_separator: None,
    })
}
