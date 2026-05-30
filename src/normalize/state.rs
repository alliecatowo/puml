use super::common::{self, CommonDirectives, LegendTextMode, RawSyntaxContext};
use super::*;

pub(super) fn normalize_state(document: Document) -> Result<StateDocument, Diagnostic> {
    let mut nodes: Vec<StateNode> = Vec::new();
    let mut transitions: Vec<ModelStateTransition> = Vec::new();
    let mut common = CommonDirectives::default();
    let mut state_style = StateStyle::default();
    let mut hide_empty_description = false;
    let mut monochrome_mode = None;
    let mut warnings: Vec<Diagnostic> = Vec::new();
    let mut note_counter = 0usize;
    let mut projection_counter = 0usize;
    let mut last_transition: Option<(String, String)> = None;
    let mut edge_routing = crate::render::graph_layout::EdgeRouting::default();

    for stmt in &document.statements {
        match &stmt.kind {
            StatementKind::StateDecl(decl) => {
                let node = state_decl_to_node(decl);
                // Collect all transitions nested inside this composite state declaration
                collect_decl_transitions(decl, &mut nodes, &mut transitions);
                upsert_state_node(&mut nodes, node);
            }
            StatementKind::StateTransition(t) => {
                // Ensure endpoints exist as nodes
                ensure_state_node(&mut nodes, &t.from);
                ensure_state_node(&mut nodes, &t.to);
                transitions.push(ModelStateTransition {
                    from: t.from.clone(),
                    to: t.to.clone(),
                    label: t.label.clone(),
                    line_color: t.line_color.clone(),
                    dashed: t.dashed,
                    hidden: t.hidden,
                    thickness: t.thickness,
                    direction: t.direction.clone(),
                });
                last_transition = Some((t.from.clone(), t.to.clone()));
            }
            StatementKind::StateInternalAction(a) => {
                ensure_state_node(&mut nodes, &a.state);
                // Add internal action to existing node
                if let Some(node) = nodes.iter_mut().find(|n| n.name == a.state) {
                    node.internal_actions.push(ModelStateInternalAction {
                        kind: a.kind.clone(),
                        action: a.action.clone(),
                    });
                }
            }
            StatementKind::StateHistory { deep } => {
                let kind = if *deep {
                    StateNodeKind::HistoryDeep
                } else {
                    StateNodeKind::HistoryShallow
                };
                upsert_state_node(
                    &mut nodes,
                    StateNode {
                        name: if *deep {
                            "[H*]".to_string()
                        } else {
                            "[H]".to_string()
                        },
                        display: Some(if *deep {
                            "H*".to_string()
                        } else {
                            "H".to_string()
                        }),
                        kind,
                        stereotype: None,
                        style: Default::default(),
                        internal_actions: Vec::new(),
                        regions: Vec::new(),
                    },
                );
            }
            StatementKind::Note(note) => {
                note_counter += 1;
                let note_name = format!("__state_note_{note_counter:04}");
                let note_text = if note.text.trim().is_empty() {
                    "note".to_string()
                } else {
                    note.text.trim().to_string()
                };
                nodes.push(StateNode {
                    name: note_name.clone(),
                    display: Some(note_text),
                    kind: StateNodeKind::Note,
                    stereotype: Some(note.position.clone()),
                    style: Default::default(),
                    internal_actions: Vec::new(),
                    regions: Vec::new(),
                });

                if note
                    .target
                    .as_deref()
                    .is_some_and(|target| target.eq_ignore_ascii_case("on link"))
                {
                    if let Some((from, to)) = &last_transition {
                        transitions.push(ModelStateTransition {
                            from: from.clone(),
                            to: note_name,
                            label: None,
                            line_color: Some("#6b7280".to_string()),
                            dashed: true,
                            hidden: false,
                            thickness: Some(1),
                            direction: Some(format!("on-link|{}|{}", note.position, to)),
                        });
                    }
                } else if let Some(target) = note.target.clone() {
                    ensure_state_node(&mut nodes, &target);
                    transitions.push(ModelStateTransition {
                        from: target,
                        to: note_name,
                        label: None,
                        line_color: Some("#6b7280".to_string()),
                        dashed: true,
                        hidden: false,
                        thickness: Some(1),
                        direction: Some(note.position.clone()),
                    });
                }
            }
            StatementKind::JsonProjection { alias, body } => {
                projection_counter += 1;
                let projection_name = format!("__state_json_{projection_counter:04}");
                nodes.push(StateNode {
                    name: projection_name,
                    display: Some(format!("{}\n{}", alias.trim(), body.trim())),
                    kind: StateNodeKind::JsonProjection,
                    stereotype: Some("json".to_string()),
                    style: Default::default(),
                    internal_actions: Vec::new(),
                    regions: Vec::new(),
                });
            }
            StatementKind::YamlProjection { alias, body } => {
                projection_counter += 1;
                let projection_name = format!("__state_yaml_{projection_counter:04}");
                nodes.push(StateNode {
                    name: projection_name,
                    display: Some(format!("{}\n{}", alias.trim(), body.trim())),
                    kind: StateNodeKind::JsonProjection,
                    stereotype: Some("yaml".to_string()),
                    style: Default::default(),
                    internal_actions: Vec::new(),
                    regions: Vec::new(),
                });
            }
            StatementKind::Title(v) => common.title(v.clone()),
            StatementKind::Header(v) => common.raw_header(v.clone()),
            StatementKind::Footer(v) => common.raw_footer(v.clone()),
            StatementKind::Caption(v) => common.caption(v.clone()),
            StatementKind::Legend(v) => {
                common.legend(v.clone(), LegendTextMode::Raw);
            }
            StatementKind::SkinParam { key, value } => {
                if key.trim().eq_ignore_ascii_case("linetype") {
                    if let Some(mode) =
                        crate::render::graph_layout::EdgeRouting::parse_linetype(value)
                    {
                        edge_routing = mode;
                    }
                    continue;
                }
                if key.trim().eq_ignore_ascii_case("monochrome") {
                    match classify_sequence_skinparam(key, value) {
                        SequenceSkinParamSupport::SupportedNoop => {}
                        SequenceSkinParamSupport::SupportedWithValue(
                            SequenceSkinParamValue::Monochrome(mode),
                        ) => monochrome_mode = Some(mode),
                        _ => warnings.push(common::unsupported_skinparam_value_warning(
                            key, value, stmt.span,
                        )),
                    }
                    continue;
                }
                if key.trim().eq_ignore_ascii_case("handwritten") {
                    match classify_sequence_skinparam(key, value) {
                        SequenceSkinParamSupport::SupportedNoop
                        | SequenceSkinParamSupport::SupportedWithValue(
                            SequenceSkinParamValue::Handwritten(_),
                        ) => {}
                        _ => warnings.push(common::unsupported_skinparam_value_warning(
                            key, value, stmt.span,
                        )),
                    }
                    continue;
                }
                use crate::theme::StateSkinParamValue;
                match classify_state_skinparam(key, value) {
                    SkinParamSupport::SupportedNoop => {}
                    SkinParamSupport::SupportedWithValue(v) => match v {
                        StateSkinParamValue::BackgroundColor(c) => {
                            state_style.background_color = c;
                        }
                        StateSkinParamValue::BorderColor(c) => {
                            state_style.border_color = c;
                        }
                        StateSkinParamValue::ArrowColor(c) => {
                            state_style.arrow_color = c;
                        }
                        StateSkinParamValue::StartColor(c) => {
                            state_style.start_color = c;
                        }
                        StateSkinParamValue::FontColor(c) => {
                            state_style.font_color = c;
                        }
                        StateSkinParamValue::FontSize(n) => {
                            state_style.font_size = Some(n);
                        }
                    },
                    SkinParamSupport::UnsupportedKey => {
                        warnings.push(common::unsupported_skinparam_warning(key, stmt.span));
                    }
                    SkinParamSupport::UnsupportedValue => {
                        warnings.push(common::unsupported_skinparam_value_warning(
                            key, value, stmt.span,
                        ));
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
                    use crate::theme::StateSkinParamValue;
                    match classify_state_skinparam(key, value) {
                        SkinParamSupport::SupportedNoop => {}
                        SkinParamSupport::SupportedWithValue(v) => match v {
                            StateSkinParamValue::BackgroundColor(c) => {
                                state_style.background_color = c;
                            }
                            StateSkinParamValue::BorderColor(c) => {
                                state_style.border_color = c;
                            }
                            StateSkinParamValue::ArrowColor(c) => {
                                state_style.arrow_color = c;
                            }
                            StateSkinParamValue::StartColor(c) => {
                                state_style.start_color = c;
                            }
                            StateSkinParamValue::FontColor(c) => {
                                state_style.font_color = c;
                            }
                            StateSkinParamValue::FontSize(n) => {
                                state_style.font_size = Some(n);
                            }
                        },
                        SkinParamSupport::UnsupportedKey => {
                            warnings.push(common::unsupported_skinparam_warning(key, stmt.span));
                        }
                        SkinParamSupport::UnsupportedValue => {
                            warnings.push(common::unsupported_skinparam_value_warning(
                                key, value, stmt.span,
                            ));
                        }
                    }
                } else {
                    warnings.push(common::unsupported_style_warning(
                        selector.as_deref(),
                        property,
                        stmt.span,
                    ));
                }
            }
            StatementKind::Theme(value) => {
                state_style = state_style_from_sequence_theme(
                    &resolve_sequence_theme_preset(value)
                        .map_err(|msg| Diagnostic::error(msg).with_span(stmt.span))?
                        .style,
                );
            }
            StatementKind::HideOption(option) => {
                if option.eq_ignore_ascii_case("empty description") {
                    hide_empty_description = true;
                }
            }
            StatementKind::Pragma(_)
            | StatementKind::AllowMixing
            | StatementKind::Include(_)
            | StatementKind::Define { .. }
            | StatementKind::Undef(_) => {}
            StatementKind::StateRegionDivider => {}
            kind if kind.raw_syntax().is_some() => {
                let raw = kind.raw_syntax().expect("raw syntax guard");
                match raw.category {
                    crate::ast::RawSyntaxCategory::Unsupported
                    | crate::ast::RawSyntaxCategory::LegacyUnknown => {
                        // Graceful degradation: skip the unsupported line and emit a
                        // non-fatal feature-loss warning so the valid remainder renders.
                        warnings.push(common::raw_syntax_feature_loss_warning(
                            raw,
                            stmt.span,
                            RawSyntaxContext::State,
                        ));
                    }
                    _ => {
                        return Err(common::raw_syntax_diagnostic(
                            raw,
                            stmt.span,
                            RawSyntaxContext::State,
                        ));
                    }
                }
            }
            _ => {
                return Err(Diagnostic::error(
                    "[E_STATE_MIXED] mixed diagram families are not supported in one document",
                )
                .with_span(stmt.span));
            }
        }
    }

    // ── Post-process: split [*] into initial pseudostate + final state ──────
    // Per UML spec, the initial pseudostate [*] should have exactly ONE outgoing
    // transition (to the first sub-state). Exit transitions (Foo --> [*]) must
    // terminate at a distinct FinalState node (filled circle with ring), not at
    // the same dot as the initial pseudostate.
    //
    // If [*] is used as BOTH source (initial) and target (final) in this diagram,
    // we create a synthetic node "[*]__end" with kind=End and rewrite all
    // incoming transitions from "[*]" to "[*]__end".
    {
        let star_as_source = transitions.iter().any(|t| t.from == "[*]");
        let star_as_target = transitions.iter().any(|t| t.to == "[*]");
        if star_as_source && star_as_target {
            // Insert the synthetic final-state node
            let final_node = StateNode {
                name: "[*]__end".to_string(),
                display: None,
                kind: StateNodeKind::End,
                stereotype: None,
                style: Default::default(),
                internal_actions: Vec::new(),
                regions: Vec::new(),
            };
            nodes.push(final_node);
            // Rewrite all transitions whose target is [*] to point at [*]__end
            for t in transitions.iter_mut() {
                if t.to == "[*]" {
                    t.to = "[*]__end".to_string();
                }
            }
        }
    }

    if let Some(mode) = monochrome_mode {
        apply_monochrome_to_state_style(&mut state_style, mode);
    }

    attach_scoped_history_endpoints(&mut nodes, &transitions);

    Ok(StateDocument {
        kind: document.kind,
        nodes,
        transitions,
        title: common.title,
        header: common.header,
        footer: common.footer,
        caption: common.caption,
        legend: common.legend,
        state_style,
        hide_empty_description,
        warnings,
        edge_routing,
    })
}

mod nodes;

use nodes::*;
