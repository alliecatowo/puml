use super::*;
use groups::GroupFrame;
use lifecycle::ActivationFrame;

pub(super) struct SequenceNormalizeState {
    pub(super) participants: Vec<Participant>,
    pub(super) participant_ix: BTreeMap<String, usize>,
    pub(super) participant_order: BTreeMap<String, i32>,
    pub(super) events: Vec<SequenceEvent>,
    pub(super) common: CommonDirectives,
    pub(super) skinparams: Vec<(String, String)>,
    pub(super) footbox_visible: bool,
    pub(super) style: SequenceStyle,
    pub(super) monochrome_mode: Option<crate::theme::MonochromeMode>,
    pub(super) warnings: Vec<Diagnostic>,
    pub(super) teoz: bool,
    pub(super) alive_by_id: BTreeMap<String, bool>,
    pub(super) activation_stack: Vec<ActivationFrame>,
    pub(super) group_stack: Vec<GroupFrame>,
    pub(super) participant_group_stack: Vec<SequenceParticipantGroup>,
    pub(super) participant_groups: Vec<SequenceParticipantGroup>,
    pub(super) last_message: Option<(String, String)>,
    pub(super) ignore_newpage: bool,
    pub(super) hide_unlinked: bool,
    pub(super) sprites: crate::sprites::SpriteRegistry,
    pub(super) list_sprites: bool,
    /// Participant IDs created mid-flow via `create X` (not pre-declared at the top).
    pub(super) created_participants: std::collections::BTreeSet<String>,
}

impl Default for SequenceNormalizeState {
    fn default() -> Self {
        Self {
            participants: Vec::new(),
            participant_ix: BTreeMap::new(),
            participant_order: BTreeMap::new(),
            events: Vec::new(),
            common: CommonDirectives::default(),
            skinparams: Vec::new(),
            footbox_visible: true,
            style: SequenceStyle::default(),
            monochrome_mode: None,
            warnings: Vec::new(),
            teoz: false,
            alive_by_id: BTreeMap::new(),
            activation_stack: Vec::new(),
            group_stack: Vec::new(),
            participant_group_stack: Vec::new(),
            participant_groups: Vec::new(),
            last_message: None,
            ignore_newpage: false,
            hide_unlinked: false,
            sprites: crate::sprites::SpriteRegistry::new(),
            list_sprites: false,
            created_participants: std::collections::BTreeSet::new(),
        }
    }
}

impl SequenceNormalizeState {
    pub(super) fn handle_statement(&mut self, stmt: Statement) -> Result<(), Diagnostic> {
        match stmt.kind {
            StatementKind::SpriteDef(sprite) => {
                self.sprites.insert(sprite.name.clone(), sprite);
            }
            StatementKind::ListSprites => self.list_sprites = true,
            StatementKind::HideUnlinked => self.hide_unlinked = true,
            StatementKind::Mainframe(title_text) => self.common.mainframe(title_text),
            StatementKind::Participant(p) => self.handle_participant(stmt.span, p)?,
            StatementKind::Message(m) => self.handle_message(stmt.span, m)?,
            StatementKind::Note(n) => {
                groups::mark_group_content(&mut self.group_stack);
                self.events.push(SequenceEvent {
                    span: stmt.span,
                    kind: SequenceEventKind::Note {
                        kind: n.kind,
                        position: n.position,
                        target: n.target,
                        text: n.text,
                        aligned: n.aligned,
                    },
                });
            }
            StatementKind::Group(g) => self.handle_group(stmt.span, g)?,
            StatementKind::Title(v) => self.common.title(v),
            StatementKind::Header(v) => self.common.header(v),
            StatementKind::Footer(v) => self.common.footer(v),
            StatementKind::Caption(v) => self.common.caption(v),
            StatementKind::Legend(v) => self.common.legend(v, LegendTextMode::ParsePackedPosition),
            StatementKind::SkinParam { key, value } => self.handle_skinparam(stmt.span, key, value),
            StatementKind::StyleParam {
                selector,
                property,
                key,
                value,
            } => {
                if let Some(key) = key {
                    self.handle_skinparam(stmt.span, key, value);
                } else {
                    self.warnings.push(common::unsupported_style_warning(
                        selector.as_deref(),
                        &property,
                        stmt.span,
                    ));
                }
            }
            StatementKind::Theme(name) => self.handle_theme(stmt.span, name)?,
            StatementKind::Pragma(value) => self.handle_pragma(stmt.span, value),
            StatementKind::Footbox(v) => {
                groups::mark_group_content(&mut self.group_stack);
                self.footbox_visible = v;
            }
            StatementKind::Delay(v) => self.push_event(stmt.span, SequenceEventKind::Delay(v)),
            StatementKind::Divider(v) => self.push_event(stmt.span, SequenceEventKind::Divider(v)),
            StatementKind::Separator(v) => {
                self.push_event(stmt.span, SequenceEventKind::Separator(v))
            }
            StatementKind::Spacer(pixels) => {
                self.push_event(stmt.span, SequenceEventKind::Spacer(pixels))
            }
            StatementKind::NewPage(v) => {
                groups::mark_group_content(&mut self.group_stack);
                if !self.ignore_newpage {
                    self.events.push(SequenceEvent {
                        span: stmt.span,
                        kind: SequenceEventKind::NewPage(v),
                    });
                }
            }
            StatementKind::IgnoreNewPage => {
                groups::mark_group_content(&mut self.group_stack);
                self.ignore_newpage = true;
            }
            StatementKind::Autonumber(v) => self.handle_autonumber(stmt.span, v)?,
            StatementKind::Activate(id) => self.handle_activate(stmt.span, id)?,
            StatementKind::Deactivate(id) => self.handle_deactivate(stmt.span, id)?,
            StatementKind::Destroy(id) => self.handle_destroy(stmt.span, id)?,
            StatementKind::Create(id) => self.handle_create(stmt.span, id)?,
            StatementKind::Return(v) => self.handle_return(stmt.span, v)?,
            StatementKind::AllowMixing
            | StatementKind::Include(_)
            | StatementKind::Define { .. }
            | StatementKind::Undef(_)
            | StatementKind::RawBlockContent(_) => {}
            // Phase A: StyleBlock is parsed but not yet applied by family normalizers.
            // The compat shim already emits legacy StyleParam triples; skip the typed
            // AST node silently until Phase B wires up per-family application.
            StatementKind::StyleBlock(_) => {}
            StatementKind::Scale(body) => {
                groups::mark_group_content(&mut self.group_stack);
                self.common.scale(&body);
            }
            StatementKind::LegendPos(pos) => {
                groups::mark_group_content(&mut self.group_stack);
                self.common.legend_position(&pos);
            }
            StatementKind::SetOption { .. } | StatementKind::HideOption(_) => {}
            kind if kind.raw_syntax().is_some() => {
                let raw = kind.raw_syntax().expect("raw syntax guard");
                match raw.category {
                    crate::ast::RawSyntaxCategory::Unsupported
                    | crate::ast::RawSyntaxCategory::LegacyUnknown => {
                        // Graceful degradation: skip the unsupported line and emit a
                        // non-fatal feature-loss warning so the valid remainder renders.
                        if raw.line.trim() != "---" {
                            self.warnings.push(common::raw_syntax_feature_loss_warning(
                                raw,
                                stmt.span,
                                common::RawSyntaxContext::Sequence,
                            ));
                        }
                    }
                    _ => handle_sequence_raw_syntax(stmt.span, raw)?,
                }
            }
            _ => {
                return Err(Diagnostic::error(
                    "[E_FAMILY_MIXED] mixed diagram families are not supported in one document",
                )
                .with_span(stmt.span));
            }
        }
        Ok(())
    }

    pub(super) fn finish(mut self) -> Result<SequenceDocument, Diagnostic> {
        if let Some(open) = self.group_stack.pop() {
            return Err(Diagnostic::error(format!(
                "[E_GROUP_UNCLOSED] missing `end` for open `{}` block",
                open.kind
            ))
            .with_span(open.span));
        }

        let mut hidden_participants = Vec::new();
        if self.hide_unlinked {
            self.apply_hide_unlinked(&mut hidden_participants);
        }
        while let Some(group) = self.participant_group_stack.pop() {
            if !group.participant_ids.is_empty() {
                self.participant_groups.push(group);
            }
        }
        self.apply_participant_order();
        common::sort_diagnostics_by_message_and_span(&mut self.warnings);
        if let Some(mode) = self.monochrome_mode {
            apply_monochrome_to_sequence_style(&mut self.style, mode);
        }

        Ok(SequenceDocument {
            participants: self.participants,
            participant_groups: self.participant_groups,
            events: self.events,
            teoz: self.teoz,
            title: self.common.title,
            header: self.common.header,
            header_align: self.common.header_align,
            footer: self.common.footer,
            footer_align: self.common.footer_align,
            caption: self.common.caption,
            legend: self.common.legend,
            skinparams: self.skinparams,
            style: self.style,
            footbox_visible: self.footbox_visible,
            scale: self.common.scale,
            legend_halign: self.common.legend_halign,
            legend_valign: self.common.legend_valign,
            warnings: self.warnings,
            hide_unlinked: self.hide_unlinked,
            hidden_participants,
            sprites: self.sprites,
            list_sprites: self.list_sprites,
            mainframe: self.common.mainframe,
            created_participants: self.created_participants,
        })
    }

    pub(super) fn push_event(&mut self, span: crate::source::Span, kind: SequenceEventKind) {
        groups::mark_group_content(&mut self.group_stack);
        self.events.push(SequenceEvent { span, kind });
    }
}

fn handle_sequence_raw_syntax(
    span: crate::source::Span,
    raw: crate::ast::RawSyntax<'_>,
) -> Result<(), Diagnostic> {
    if raw.line.trim() == "---" {
        return Ok(());
    }
    Err(common::raw_syntax_diagnostic(
        raw,
        span,
        common::RawSyntaxContext::Sequence,
    ))
}
