use super::family::family_kind_name;
use super::*;
use crate::ast::{RawSyntax, RawSyntaxCategory};
use crate::diagnostic::diagnostic_code;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LegendTextMode {
    Raw,
    ParsePackedPosition,
    StripPackedPosition,
}

#[derive(Debug, Clone, Default)]
pub(super) struct CommonDirectives {
    pub title: Option<String>,
    pub header: Option<String>,
    pub header_align: MetadataHAlign,
    pub footer: Option<String>,
    pub footer_align: MetadataHAlign,
    pub caption: Option<String>,
    pub legend: Option<String>,
    pub legend_halign: LegendHAlign,
    pub legend_valign: LegendVAlign,
    pub mainframe: Option<String>,
    pub scale: Option<ScaleSpec>,
}

impl CommonDirectives {
    pub(super) fn title(&mut self, value: String) {
        self.title = Some(value);
    }

    pub(super) fn header(&mut self, value: String) {
        let (align, text) = unpack_metadata_align(value);
        self.header_align = align.unwrap_or_default();
        self.header = Some(text);
    }

    pub(super) fn raw_header(&mut self, value: String) {
        self.header = Some(value);
    }

    pub(super) fn footer(&mut self, value: String) {
        let (align, text) = unpack_metadata_align(value);
        self.footer_align = align.unwrap_or_default();
        self.footer = Some(text);
    }

    pub(super) fn raw_footer(&mut self, value: String) {
        self.footer = Some(value);
    }

    pub(super) fn caption(&mut self, value: String) {
        self.caption = Some(value);
    }

    pub(super) fn legend(&mut self, value: String, mode: LegendTextMode) {
        match mode {
            LegendTextMode::Raw => self.legend = Some(value),
            LegendTextMode::ParsePackedPosition => self.legend_preserving_position(value),
            LegendTextMode::StripPackedPosition => {
                self.legend = Some(strip_legend_pos_prefix(&value));
            }
        }
    }

    pub(super) fn mainframe(&mut self, value: String) {
        self.mainframe = Some(value);
    }

    pub(super) fn scale(&mut self, body: &str) {
        self.scale = parse_scale_spec(body).or_else(|| self.scale.take());
    }

    pub(super) fn legend_position(&mut self, position: &str) {
        apply_legend_position(position, &mut self.legend_halign, &mut self.legend_valign);
    }

    fn legend_preserving_position(&mut self, value: String) {
        let Some(rest) = value.strip_prefix("LEGEND_POS:") else {
            self.legend = Some(value);
            return;
        };

        if let Some(newline_idx) = rest.find('\n') {
            let pos = &rest[..newline_idx];
            let text = &rest[newline_idx + 1..];
            self.legend = Some(text.to_string());
            self.legend_position(pos);
        } else {
            self.legend_position(rest);
        }
    }
}

pub(super) fn unsupported_skinparam_warning(key: &str, span: crate::source::Span) -> Diagnostic {
    Diagnostic::warning(format!(
        "[W_SKINPARAM_UNSUPPORTED] unsupported skinparam `{}`",
        key
    ))
    .with_span(span)
}

pub(super) fn unsupported_skinparam_value_warning(
    key: &str,
    value: &str,
    span: crate::source::Span,
) -> Diagnostic {
    Diagnostic::warning(format!(
        "[W_SKINPARAM_UNSUPPORTED_VALUE] unsupported value `{}` for skinparam `{}`",
        value, key
    ))
    .with_span(span)
}

pub(super) fn unsupported_pragma_warning(value: &str, span: crate::source::Span) -> Diagnostic {
    Diagnostic::warning(format!(
        "[W_PRAGMA_UNSUPPORTED] unsupported pragma `{}`",
        value
    ))
    .with_span(span)
}

pub(super) enum RawSyntaxContext {
    State,
    Timeline(DiagramKind),
    Family(DiagramKind),
}

pub(super) fn raw_syntax_diagnostic(
    raw: RawSyntax<'_>,
    span: crate::source::Span,
    context: RawSyntaxContext,
) -> Diagnostic {
    if matches!(
        raw.category,
        RawSyntaxCategory::Unsupported | RawSyntaxCategory::Malformed
    ) && diagnostic_code(raw.line).is_some()
    {
        return Diagnostic::error(raw.line).with_span(span);
    }

    let diagnostic = match raw.category {
        RawSyntaxCategory::LegacyUnknown => Diagnostic::error(format!(
            "[E_PARSE_UNKNOWN] unsupported syntax: `{}`",
            raw.line
        )),
        RawSyntaxCategory::Unsupported => match context {
            RawSyntaxContext::State => Diagnostic::error(format!(
                "[E_STATE_UNSUPPORTED_SYNTAX] unsupported state diagram syntax: `{}`",
                raw.line
            )),
            RawSyntaxContext::Timeline(kind) => Diagnostic::error(format!(
                "[E_TIMELINE_UNSUPPORTED_SYNTAX] unsupported {} syntax: `{}`",
                family_kind_name(kind),
                raw.line
            )),
            RawSyntaxContext::Family(kind) => Diagnostic::error(format!(
                "[E_FAMILY_{}_UNSUPPORTED_SYNTAX] unsupported {} syntax: `{}`",
                family_kind_name(kind).to_uppercase(),
                family_kind_name(kind),
                raw.line
            )),
        },
        RawSyntaxCategory::Deferred => match context {
            RawSyntaxContext::State => Diagnostic::error(format!(
                "[E_STATE_DEFERRED_RAW] deferred state diagram syntax was not consumed: `{}`",
                raw.line
            )),
            RawSyntaxContext::Timeline(kind) => Diagnostic::error(format!(
                "[E_TIMELINE_DEFERRED_RAW] deferred {} syntax was not consumed: `{}`",
                family_kind_name(kind),
                raw.line
            )),
            RawSyntaxContext::Family(kind) => Diagnostic::error(format!(
                "[E_FAMILY_{}_DEFERRED_RAW] deferred {} syntax was not consumed: `{}`",
                family_kind_name(kind).to_uppercase(),
                family_kind_name(kind),
                raw.line
            )),
        },
        RawSyntaxCategory::CommentLowered => match context {
            RawSyntaxContext::State => Diagnostic::error(format!(
                "[E_STATE_COMMENT_LOWERED] lowered comment is not valid state syntax: `{}`",
                raw.line
            )),
            RawSyntaxContext::Timeline(kind) => Diagnostic::error(format!(
                "[E_TIMELINE_COMMENT_LOWERED] lowered comment is not valid {} syntax: `{}`",
                family_kind_name(kind),
                raw.line
            )),
            RawSyntaxContext::Family(kind) => Diagnostic::error(format!(
                "[E_FAMILY_{}_COMMENT_LOWERED] lowered comment is not valid {} syntax: `{}`",
                family_kind_name(kind).to_uppercase(),
                family_kind_name(kind),
                raw.line
            )),
        },
        RawSyntaxCategory::Malformed => match context {
            RawSyntaxContext::State => Diagnostic::error(format!(
                "[E_STATE_MALFORMED_SYNTAX] malformed state diagram syntax: `{}`",
                raw.line
            )),
            RawSyntaxContext::Timeline(kind) => Diagnostic::error(format!(
                "[E_TIMELINE_MALFORMED_SYNTAX] malformed {} syntax: `{}`",
                family_kind_name(kind),
                raw.line
            )),
            RawSyntaxContext::Family(kind) => Diagnostic::error(format!(
                "[E_FAMILY_{}_MALFORMED_SYNTAX] malformed {} syntax: `{}`",
                family_kind_name(kind).to_uppercase(),
                family_kind_name(kind),
                raw.line
            )),
        },
    };
    diagnostic.with_span(span)
}

pub(super) fn sort_diagnostics_by_message_and_span(warnings: &mut [Diagnostic]) {
    warnings.sort_by(|a, b| {
        let sa = a.span.map(|s| s.start).unwrap_or_default();
        let sb = b.span.map(|s| s.start).unwrap_or_default();
        (a.message.as_str(), sa).cmp(&(b.message.as_str(), sb))
    });
}

pub(super) fn strip_legend_pos_prefix(value: &str) -> String {
    if let Some(rest) = value.strip_prefix("LEGEND_POS:") {
        if let Some(nl) = rest.find('\n') {
            return rest[nl + 1..].to_string();
        }
        return String::new();
    }
    value.to_string()
}

fn apply_legend_position(
    position: &str,
    legend_halign: &mut LegendHAlign,
    legend_valign: &mut LegendVAlign,
) {
    let lower = position.to_ascii_lowercase();
    for token in lower.split_whitespace() {
        match token {
            "left" => *legend_halign = LegendHAlign::Left,
            "right" => *legend_halign = LegendHAlign::Right,
            "center" => *legend_halign = LegendHAlign::Center,
            "top" => *legend_valign = LegendVAlign::Top,
            "bottom" => *legend_valign = LegendVAlign::Bottom,
            _ => {}
        }
    }
}

fn unpack_metadata_align(value: String) -> (Option<MetadataHAlign>, String) {
    let Some(rest) = value.strip_prefix("METADATA_ALIGN:") else {
        return (None, value);
    };
    let Some((align, text)) = rest.split_once('\n') else {
        return (None, value);
    };
    let align = match align {
        "left" => MetadataHAlign::Left,
        "center" => MetadataHAlign::Center,
        "right" => MetadataHAlign::Right,
        _ => return (None, value),
    };
    (Some(align), text.to_string())
}

/// Parse a scale body (everything after "scale ").
/// Supports:
///   "1.5"          -> Factor(1.5)
///   "2/3"          -> Factor(0.666...)
///   "200 width"    -> Width(200)
///   "200 height"   -> Height(200)
///   "800*600"      -> Fixed { width: 800, height: 600 }
///   "max 800"      -> Max(800)
///   "max 800 width"  -> MaxWidth(800)
///   "max 600 height" -> MaxHeight(600)
///   "max 800*600"    -> MaxFixed { width: 800, height: 600 }
pub(super) fn parse_scale_spec(body: &str) -> Option<ScaleSpec> {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return None;
    }
    let lower = trimmed.to_ascii_lowercase();
    if let Some(rest) = lower.strip_prefix("max ") {
        let rest = rest.trim();
        if let Some(value) = rest.strip_suffix(" width") {
            let n: u32 = value.trim().parse().ok()?;
            return Some(ScaleSpec::MaxWidth(n));
        }
        if let Some(value) = rest.strip_suffix(" height") {
            let n: u32 = value.trim().parse().ok()?;
            return Some(ScaleSpec::MaxHeight(n));
        }
        if let Some(idx) = rest.find('*') {
            let w: u32 = rest[..idx].trim().parse().ok()?;
            let h: u32 = rest[idx + 1..].trim().parse().ok()?;
            return Some(ScaleSpec::MaxFixed {
                width: w,
                height: h,
            });
        }
        let n: u32 = rest.parse().ok()?;
        return Some(ScaleSpec::Max(n));
    }
    if let Some(value) = lower.strip_suffix(" width") {
        let n: u32 = value.trim().parse().ok()?;
        return Some(ScaleSpec::Width(n));
    }
    if let Some(value) = lower.strip_suffix(" height") {
        let n: u32 = value.trim().parse().ok()?;
        return Some(ScaleSpec::Height(n));
    }
    if let Some(idx) = trimmed.find('*') {
        let w: u32 = trimmed[..idx].trim().parse().ok()?;
        let h: u32 = trimmed[idx + 1..].trim().parse().ok()?;
        return Some(ScaleSpec::Fixed {
            width: w,
            height: h,
        });
    }
    if let Some(idx) = trimmed.find('/') {
        let numerator: f64 = trimmed[..idx].trim().parse().ok()?;
        let denominator: f64 = trimmed[idx + 1..].trim().parse().ok()?;
        if numerator > 0.0 && denominator > 0.0 {
            return Some(ScaleSpec::Factor(numerator / denominator));
        }
        return None;
    }
    let f: f64 = trimmed.parse().ok()?;
    if f > 0.0 {
        Some(ScaleSpec::Factor(f))
    } else {
        None
    }
}
