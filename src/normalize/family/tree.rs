use super::*;

pub(crate) fn normalize_family_tree(document: Document) -> Result<FamilyDocument, Diagnostic> {
    let mut nodes = Vec::new();
    let mut relations = Vec::new();
    let mut title = None;
    let mut header = None;
    let mut footer = None;
    let mut caption = None;
    let mut legend = None;

    let family_kind = document.kind;
    let mut warnings = Vec::new();
    let mut orientation = FamilyOrientation::TopToBottom;
    let mut style = SequenceStyle::default();
    let mut text_overflow_policy = TextOverflowPolicy::WrapAndGrow;
    // MindMap: track whether subsequent depth-1 nodes should go on the left side.
    let mut mindmap_left_side_mode = false;

    for stmt in document.statements {
        match stmt.kind {
            StatementKind::Title(v) => title = Some(v),
            StatementKind::Header(v) => header = Some(v),
            StatementKind::Footer(v) => footer = Some(v),
            StatementKind::Caption(v) => caption = Some(v),
            StatementKind::Legend(v) => legend = Some(v),
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
            StatementKind::Theme(value) => {
                style = resolve_sequence_theme_preset(&value)
                    .map_err(|msg| Diagnostic::error(msg).with_span(stmt.span))?
                    .style;
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
            StatementKind::Unknown(line) => {
                if line.trim().is_empty() {
                    continue;
                }
                if let Some(value) = parse_family_orientation_directive(&line) {
                    orientation = value;
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
                if let Some(mut node_info) = parse_mindmap_or_wbs_node(&line) {
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
                    nodes.push(FamilyNode {
                        kind,
                        name: node_info.name,
                        alias: None,
                        members: Vec::new(),
                        depth: node_info.depth,
                        label: None,
                        mindmap_side: node_info.side,
                        wbs_checkbox: node_info.checkbox,
                        fill_color: node_info.fill_color,
                    });
                    continue;
                }
                return Err(Diagnostic::error(format!(
                    "[E_PARSE_UNKNOWN] unsupported syntax: `{}`",
                    line
                ))
                .with_span(stmt.span));
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

    Ok(FamilyDocument {
        kind: family_kind,
        nodes,
        relations,
        title,
        header,
        footer,
        caption,
        legend,
        orientation,
        style,
        family_style: None,
        text_overflow_policy,
        warnings,
        groups: Vec::new(),
        json_projections: Vec::new(),
        hide_options: std::collections::BTreeSet::new(),
        namespace_separator: None,
    })
}

fn normalize_family_tree_warnings(warnings: &mut [Diagnostic]) {
    warnings.sort_by(|a, b| {
        let sa = a.span.map(|s| s.start).unwrap_or_default();
        let sb = b.span.map(|s| s.start).unwrap_or_default();
        (a.message.as_str(), sa).cmp(&(b.message.as_str(), sb))
    });
}

fn build_family_tree_relations(nodes: &mut [FamilyNode], relations: &mut Vec<ModelFamilyRelation>) {
    let mut parents: Vec<usize> = Vec::new();
    for idx in 0..nodes.len() {
        let depth = nodes[idx].depth;
        while parents.len() > depth {
            parents.pop();
        }
        if let Some(parent_idx) = parents.last().copied() {
            relations.push(ModelFamilyRelation {
                from: nodes[parent_idx].name.clone(),
                to: nodes[idx].name.clone(),
                arrow: "->".to_string(),
                label: None,
                stereotype: None,
                left_cardinality: None,
                right_cardinality: None,
                left_role: None,
                right_role: None,
                line_color: None,
                dashed: false,
                hidden: false,
                thickness: None,
                direction: None,
                left_lollipop: false,
                right_lollipop: false,
            });
        }
        parents.push(idx);
    }
}

fn handle_family_overflow_skinparam(
    key: &str,
    value: &str,
    policy: &mut TextOverflowPolicy,
    warnings: &mut Vec<Diagnostic>,
    span: crate::source::Span,
) -> bool {
    let normalized_key = key.trim().to_ascii_lowercase();
    let normalized_value = value.trim().to_ascii_lowercase();
    if normalized_key != "textoverflowpolicy" && normalized_key != "text_overflow_policy" {
        return false;
    }

    let parsed = match normalized_value.as_str() {
        "wrap" | "wrapandgrow" | "wrap_and_grow" | "wrapgrow" => {
            Some(TextOverflowPolicy::WrapAndGrow)
        }
        "ellipsis" | "ellipsesingleline" | "ellipsissingleline" | "singleline" | "nowrap" => {
            Some(TextOverflowPolicy::EllipsisSingleLine)
        }
        _ => {
            warnings.push(
                Diagnostic::warning(format!(
                    "[W_SKINPARAM_UNSUPPORTED_VALUE] unsupported value `{}` for skinparam `{}`",
                    value, key
                ))
                .with_span(span),
            );
            None
        }
    };
    if let Some(parsed) = parsed {
        *policy = parsed;
    }
    true
}

fn parse_family_orientation_directive(line: &str) -> Option<FamilyOrientation> {
    let tokens = line
        .split_whitespace()
        .map(|t| t.to_ascii_lowercase())
        .collect::<Vec<_>>();
    if tokens.len() == 4 && tokens[3].as_str() == "direction" {
        let key = [&tokens[0][..], &tokens[1][..], &tokens[2][..]].join(" ");
        return match key.as_str() {
            "left to right" => Some(FamilyOrientation::LeftToRight),
            "right to left" => Some(FamilyOrientation::RightToLeft),
            "top to bottom" => Some(FamilyOrientation::TopToBottom),
            "bottom to top" => Some(FamilyOrientation::BottomToTop),
            _ => None,
        };
    }
    None
}

struct MindMapWbsNode {
    depth: usize,
    name: String,
    side: MindMapSide,
    checkbox: Option<WbsCheckbox>,
    fill_color: Option<String>,
}

/// Parse a MindMap / WBS node line. Handles:
///
/// - `* Root`, `** Child`, `*** Grandchild` — star-depth (depth = stars - 1)
/// - `*[#Orange] Root`, `**[#fef3c7] Child` — PlantUML-style node color tags
/// - `** Left child` after a `left side` keyword (tracked externally)
/// - `+** Right`, `-** Left` — explicit side prefix on first depth-2+ star
/// - WBS annotations: `[x]` checked, `[ ]` unchecked, `[%NN]` progress
fn parse_mindmap_or_wbs_node(line: &str) -> Option<MindMapWbsNode> {
    let trimmed = line.trim_start();

    // Detect optional side prefix: `+` = right, `-` = left (only matters at
    // depth >= 1 in MindMap, but we parse it universally and let the renderer
    // decide what to do with it).
    let (side_prefix, rest) = if let Some(s) = trimmed.strip_prefix('+') {
        (Some(MindMapSide::Right), s)
    } else if let Some(s) = trimmed.strip_prefix('-') {
        (Some(MindMapSide::Left), s)
    } else {
        (None, trimmed)
    };

    let star_prefix = rest.bytes().take_while(|c| *c == b'*').count();
    if star_prefix == 0 {
        return None;
    }

    let mut label = rest[star_prefix..].trim().to_string();
    let fill_color = parse_mindmap_wbs_color_tag(&mut label);
    if label.is_empty() {
        return None;
    }
    // PlantUML interprets `\n` in label text as a line break (#560).
    // Convert the literal backslash-n sequence to an actual newline so the
    // renderer's multi-line text emission path can wrap it.
    label = label.replace("\\n", "\n");

    // Parse WBS checkbox suffix: `[x]`, `[ ]`, `[%NN]` at end of label.
    let checkbox = parse_wbs_checkbox(&mut label);

    // Side defaults to Right unless explicitly prefixed.
    let side = side_prefix.unwrap_or(MindMapSide::Right);
    let depth = star_prefix.saturating_sub(1);

    Some(MindMapWbsNode {
        depth,
        name: label,
        side,
        checkbox,
        fill_color,
    })
}

/// Parse a leading PlantUML color tag from MindMap/WBS labels.
///
/// PlantUML examples use tags such as `[#Orange]` and `[#lightgreen]`; SVG
/// accepts named colors without the leading `#`, while hex colors keep it.
fn parse_mindmap_wbs_color_tag(label: &mut String) -> Option<String> {
    let trimmed = label.trim_start();
    let rest = trimmed.strip_prefix('[')?;
    let close = rest.find(']')?;
    let raw = rest[..close].trim();
    let value = raw.strip_prefix('#')?.trim();
    if value.is_empty() {
        return None;
    }
    let normalized =
        if matches!(value.len(), 3 | 6 | 8) && value.chars().all(|ch| ch.is_ascii_hexdigit()) {
            format!("#{value}")
        } else {
            value.to_string()
        };
    *label = rest[close + 1..].trim_start().to_string();
    Some(normalized)
}

/// Try to parse a WBS checkbox annotation from the end of a label, stripping it
/// from the label string if found.
fn parse_wbs_checkbox(label: &mut String) -> Option<WbsCheckbox> {
    let trimmed = label.trim_end();
    if let Some(inner) = trimmed.strip_suffix(']') {
        if let Some(bracket_start) = inner.rfind('[') {
            let content = &inner[bracket_start + 1..];
            let checkbox = if content == "x" || content == "X" {
                Some(WbsCheckbox::Checked)
            } else if content == " " || content.is_empty() {
                Some(WbsCheckbox::Unchecked)
            } else if let Some(pct_str) = content.strip_prefix('%') {
                pct_str
                    .trim()
                    .parse::<u8>()
                    .ok()
                    .filter(|&n| n <= 100)
                    .map(WbsCheckbox::Progress)
            } else {
                None
            };
            if checkbox.is_some() {
                let prefix = &inner[..bracket_start].trim_end().to_string();
                *label = prefix.to_string();
                return checkbox;
            }
        }
    }
    None
}
