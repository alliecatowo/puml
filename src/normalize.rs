use std::collections::BTreeMap;

use crate::ast::{
    ActivityStepKind, ComponentNodeKind, DiagramKind, Document, ParticipantRole as AstRole,
    StatementKind, TimingDeclKind,
};
use crate::diagnostic::Diagnostic;
use crate::model::FamilyStyle;
use crate::model::{
    ArchimateDocument, ArchimateElement, ArchimateRelation, ChartDocument, ChartPoint,
    ChartSubtype, DitaaDocument, EbnfDocument, EbnfRule, EbnfToken, FamilyDocument, FamilyGroup,
    FamilyNode, FamilyNodeKind, FamilyOrientation, FamilyRelation as ModelFamilyRelation,
    JsonDocument, JsonTreeNode, LegendHAlign, LegendVAlign, MathDocument, MindMapSide,
    NormalizedDocument, NwdiagDocument, NwdiagNetwork, NwdiagNode, Participant, ParticipantRole,
    RegexDocument, RegexPattern, RegexToken, RepeatKind, ScaleSpec, SdlDocument, SdlState,
    SdlStateKind, SdlTransition, SequenceDocument, SequenceEvent, SequenceEventKind, SequencePage,
    StateDocument, StateInternalAction as ModelStateInternalAction, StateNode, StateNodeKind,
    StateTransition as ModelStateTransition, TimelineChronologyEvent, TimelineConstraint,
    TimelineDocument, TimelineMilestone, TimelineTask, VirtualEndpoint, VirtualEndpointKind,
    VirtualEndpointSide, WbsCheckbox, YamlDocument, YamlTreeNode,
};
use crate::scene::TextOverflowPolicy;
use crate::theme::{
    activity_style_from_sequence_theme, chart_style_from_sequence_theme, classify_activity_skinparam,
    classify_chart_skinparam, classify_class_skinparam, classify_component_skinparam,
    classify_sequence_skinparam, classify_state_skinparam, classify_timing_skinparam,
    class_style_from_sequence_theme, component_style_from_sequence_theme,
    resolve_sequence_theme_preset, state_style_from_sequence_theme,
    timing_style_from_sequence_theme, ActivityStyle, ChartStyle, ClassStyle, ComponentStyle,
    SequenceSkinParamSupport, SequenceSkinParamValue, SequenceStyle, SkinParamSupport, StateStyle,
    TimingStyle,
};

#[derive(Debug, Clone, Default)]
pub struct NormalizeOptions {
    pub include_root: Option<std::path::PathBuf>,
}

pub fn normalize(document: Document) -> Result<SequenceDocument, Diagnostic> {
    normalize_with_options(document, &NormalizeOptions::default())
}

pub fn normalize_family(document: Document) -> Result<NormalizedDocument, Diagnostic> {
    normalize_family_with_options(document, &NormalizeOptions::default())
}

pub fn normalize_family_with_options(
    document: Document,
    options: &NormalizeOptions,
) -> Result<NormalizedDocument, Diagnostic> {
    match document.kind {
        DiagramKind::Sequence => {
            normalize_with_options(document, options).map(NormalizedDocument::Sequence)
        }
        DiagramKind::Class | DiagramKind::Object | DiagramKind::UseCase | DiagramKind::Salt => {
            normalize_stub_family(document).map(NormalizedDocument::Family)
        }
        DiagramKind::Gantt | DiagramKind::Chronology => {
            normalize_timeline_baseline(document).map(NormalizedDocument::Timeline)
        }
        DiagramKind::State => normalize_state(document).map(NormalizedDocument::State),
        DiagramKind::MindMap | DiagramKind::Wbs => {
            normalize_family_tree(document).map(NormalizedDocument::Family)
        }
        DiagramKind::Json => normalize_json_document(document).map(NormalizedDocument::Json),
        DiagramKind::Yaml => normalize_yaml_document(document).map(NormalizedDocument::Yaml),
        DiagramKind::Nwdiag => {
            normalize_nwdiag_document(document).map(NormalizedDocument::Nwdiag)
        }
        DiagramKind::Archimate => {
            normalize_archimate_document(document).map(NormalizedDocument::Archimate)
        }
        DiagramKind::Regex => normalize_regex(document).map(NormalizedDocument::Regex),
        DiagramKind::Ebnf => normalize_ebnf(document).map(NormalizedDocument::Ebnf),
        DiagramKind::Math => normalize_math(document).map(NormalizedDocument::Math),
        DiagramKind::Sdl => normalize_sdl(document).map(NormalizedDocument::Sdl),
        DiagramKind::Ditaa => normalize_ditaa(document).map(NormalizedDocument::Ditaa),
        DiagramKind::Chart => normalize_chart(document).map(NormalizedDocument::Chart),
        DiagramKind::Component
        | DiagramKind::Deployment
        | DiagramKind::Activity
        | DiagramKind::Timing => normalize_extended_family(document).map(NormalizedDocument::Family),
        DiagramKind::Unknown => Err(Diagnostic::error(
            "[E_FAMILY_UNKNOWN] unable to detect supported diagram family; expected sequence/class/object/usecase/gantt/chronology syntax",
        )),
    }
}

fn collect_raw_body(document: &Document) -> (Option<String>, Vec<String>) {
    let mut title: Option<String> = None;
    let mut body: Vec<String> = Vec::new();
    for stmt in &document.statements {
        match &stmt.kind {
            StatementKind::Title(v) => title = Some(v.clone()),
            StatementKind::RawBody(line) => {
                let trimmed = line.trim_start();
                if let Some(rest) = trimmed.strip_prefix("title ") {
                    if title.is_none() {
                        title = Some(rest.trim().to_string());
                    }
                    continue;
                }
                if trimmed.eq_ignore_ascii_case("title") {
                    continue;
                }
                body.push(line.clone());
            }
            _ => {}
        }
    }
    (title, body)
}

fn normalize_regex(document: Document) -> Result<RegexDocument, Diagnostic> {
    let (title, body) = collect_raw_body(&document);
    let mut patterns = Vec::new();
    let mut warnings: Vec<Diagnostic> = Vec::new();
    for line in body {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let tokens = parse_regex_tokens(trimmed, &mut warnings);
        patterns.push(RegexPattern {
            source: trimmed.to_string(),
            tokens,
        });
    }
    Ok(RegexDocument {
        title,
        patterns,
        warnings,
    })
}

fn parse_regex_tokens(input: &str, warnings: &mut Vec<Diagnostic>) -> Vec<RegexToken> {
    let chars: Vec<char> = input.chars().collect();
    let mut idx = 0usize;
    let tokens = parse_regex_alt(&chars, &mut idx, false, warnings);
    if idx < chars.len() {
        warnings.push(Diagnostic::warning(format!(
            "[W_REGEX_UNCONSUMED] trailing input not consumed at offset {idx}"
        )));
    }
    tokens
}

fn parse_regex_alt(
    chars: &[char],
    idx: &mut usize,
    in_group: bool,
    warnings: &mut Vec<Diagnostic>,
) -> Vec<RegexToken> {
    let mut branches: Vec<Vec<RegexToken>> = Vec::new();
    let mut current = parse_regex_seq(chars, idx, in_group, warnings);
    while *idx < chars.len() && chars[*idx] == '|' {
        *idx += 1;
        branches.push(std::mem::take(&mut current));
        current = parse_regex_seq(chars, idx, in_group, warnings);
    }
    if branches.is_empty() {
        current
    } else {
        branches.push(current);
        vec![RegexToken::Alt(branches)]
    }
}

fn parse_regex_seq(
    chars: &[char],
    idx: &mut usize,
    in_group: bool,
    warnings: &mut Vec<Diagnostic>,
) -> Vec<RegexToken> {
    let mut out: Vec<RegexToken> = Vec::new();
    let mut literal = String::new();
    while *idx < chars.len() {
        let ch = chars[*idx];
        if ch == ')' && in_group {
            break;
        }
        if ch == '|' {
            break;
        }
        if ch == '(' {
            flush_literal(&mut literal, &mut out);
            *idx += 1;
            let inner = parse_regex_alt(chars, idx, true, warnings);
            if *idx < chars.len() && chars[*idx] == ')' {
                *idx += 1;
            } else {
                warnings.push(Diagnostic::warning(
                    "[W_REGEX_UNBALANCED] missing closing `)`",
                ));
            }
            push_with_repeat(RegexToken::Group(inner), chars, idx, &mut out);
            continue;
        }
        if ch == '[' {
            flush_literal(&mut literal, &mut out);
            *idx += 1;
            let mut class = String::new();
            while *idx < chars.len() && chars[*idx] != ']' {
                class.push(chars[*idx]);
                *idx += 1;
            }
            if *idx < chars.len() {
                *idx += 1;
            } else {
                warnings.push(Diagnostic::warning(
                    "[W_REGEX_UNBALANCED] missing closing `]`",
                ));
            }
            push_with_repeat(RegexToken::CharClass(class), chars, idx, &mut out);
            continue;
        }
        if ch == '\\' {
            flush_literal(&mut literal, &mut out);
            *idx += 1;
            if *idx < chars.len() {
                let esc = chars[*idx];
                *idx += 1;
                push_with_repeat(RegexToken::Escape(esc), chars, idx, &mut out);
            } else {
                warnings.push(Diagnostic::warning(
                    "[W_REGEX_TRAILING_ESCAPE] trailing backslash",
                ));
            }
            continue;
        }
        if ch == '.' {
            flush_literal(&mut literal, &mut out);
            *idx += 1;
            push_with_repeat(RegexToken::AnyChar, chars, idx, &mut out);
            continue;
        }
        if ch == '^' || ch == '$' {
            flush_literal(&mut literal, &mut out);
            *idx += 1;
            out.push(RegexToken::Anchor(ch.to_string()));
            continue;
        }
        if ch == '{' {
            flush_literal(&mut literal, &mut out);
            let mut spec = String::new();
            while *idx < chars.len() && chars[*idx] != '}' {
                spec.push(chars[*idx]);
                *idx += 1;
            }
            if *idx < chars.len() {
                *idx += 1;
            }
            warnings.push(Diagnostic::warning(format!(
                "[W_REGEX_QUANT_UNSUPPORTED] quantifier `{{{}}}` not fully supported",
                spec.trim_start_matches('{')
            )));
            out.push(RegexToken::Unsupported(format!("{{{spec}}}")));
            continue;
        }
        if matches!(ch, '*' | '+' | '?') {
            // Stray quantifier with no prior atom; treat as literal.
            flush_literal(&mut literal, &mut out);
            *idx += 1;
            warnings.push(Diagnostic::warning(format!(
                "[W_REGEX_STRAY_QUANT] stray quantifier `{ch}`"
            )));
            out.push(RegexToken::Unsupported(ch.to_string()));
            continue;
        }
        literal.push(ch);
        *idx += 1;
        // Peek for following quantifier on the last character of literal.
        if *idx < chars.len() && matches!(chars[*idx], '*' | '+' | '?') {
            // Split off the last char as its own atom so the quantifier applies to it.
            let last = literal.pop();
            flush_literal(&mut literal, &mut out);
            if let Some(c) = last {
                push_with_repeat(RegexToken::Literal(c.to_string()), chars, idx, &mut out);
            }
        }
    }
    flush_literal(&mut literal, &mut out);
    out
}

fn flush_literal(literal: &mut String, out: &mut Vec<RegexToken>) {
    if !literal.is_empty() {
        out.push(RegexToken::Literal(std::mem::take(literal)));
    }
}

fn push_with_repeat(token: RegexToken, chars: &[char], idx: &mut usize, out: &mut Vec<RegexToken>) {
    if *idx < chars.len() {
        let kind = match chars[*idx] {
            '*' => Some(RepeatKind::ZeroOrMore),
            '+' => Some(RepeatKind::OneOrMore),
            '?' => Some(RepeatKind::ZeroOrOne),
            _ => None,
        };
        if let Some(kind) = kind {
            *idx += 1;
            out.push(RegexToken::Repeat {
                inner: Box::new(token),
                kind,
            });
            return;
        }
    }
    out.push(token);
}

fn normalize_ebnf(document: Document) -> Result<EbnfDocument, Diagnostic> {
    let (title, body) = collect_raw_body(&document);
    let mut rules = Vec::new();
    let mut warnings: Vec<Diagnostic> = Vec::new();
    let joined = body.join("\n");
    // Split rules on `;` terminator.
    for chunk in joined.split(';') {
        let chunk = chunk.trim();
        if chunk.is_empty() {
            continue;
        }
        let Some((name, body)) = chunk.split_once('=') else {
            warnings.push(Diagnostic::warning(format!(
                "[W_EBNF_RULE_MALFORMED] missing `=` in rule `{chunk}`"
            )));
            continue;
        };
        let name = name.trim().to_string();
        let body = body.trim().to_string();
        let tokens = parse_ebnf_tokens(&body, &mut warnings);
        rules.push(EbnfRule { name, body, tokens });
    }
    Ok(EbnfDocument {
        title,
        rules,
        warnings,
    })
}

fn parse_ebnf_tokens(input: &str, warnings: &mut Vec<Diagnostic>) -> Vec<EbnfToken> {
    let chars: Vec<char> = input.chars().collect();
    let mut idx = 0usize;
    let tokens = parse_ebnf_alt(&chars, &mut idx, None, warnings);
    if idx < chars.len() {
        warnings.push(Diagnostic::warning(format!(
            "[W_EBNF_UNCONSUMED] trailing input at offset {idx}"
        )));
    }
    tokens
}

fn parse_ebnf_alt(
    chars: &[char],
    idx: &mut usize,
    terminator: Option<char>,
    warnings: &mut Vec<Diagnostic>,
) -> Vec<EbnfToken> {
    let mut branches: Vec<Vec<EbnfToken>> = Vec::new();
    let mut current = parse_ebnf_seq(chars, idx, terminator, warnings);
    while *idx < chars.len() && chars[*idx] == '|' {
        *idx += 1;
        branches.push(std::mem::take(&mut current));
        current = parse_ebnf_seq(chars, idx, terminator, warnings);
    }
    if branches.is_empty() {
        current
    } else {
        branches.push(current);
        vec![EbnfToken::Alt(branches)]
    }
}

fn parse_ebnf_seq(
    chars: &[char],
    idx: &mut usize,
    terminator: Option<char>,
    warnings: &mut Vec<Diagnostic>,
) -> Vec<EbnfToken> {
    let mut out: Vec<EbnfToken> = Vec::new();
    while *idx < chars.len() {
        let ch = chars[*idx];
        if Some(ch) == terminator {
            break;
        }
        if ch == '|' {
            break;
        }
        if ch.is_whitespace() {
            *idx += 1;
            continue;
        }
        if ch == '"' || ch == '\'' {
            let quote = ch;
            *idx += 1;
            let mut s = String::new();
            while *idx < chars.len() && chars[*idx] != quote {
                s.push(chars[*idx]);
                *idx += 1;
            }
            if *idx < chars.len() {
                *idx += 1;
            }
            let token = EbnfToken::Terminal(s);
            push_ebnf_with_repeat(token, chars, idx, &mut out);
            continue;
        }
        if ch == '(' {
            *idx += 1;
            let inner = parse_ebnf_alt(chars, idx, Some(')'), warnings);
            if *idx < chars.len() && chars[*idx] == ')' {
                *idx += 1;
            } else {
                warnings.push(Diagnostic::warning(
                    "[W_EBNF_UNBALANCED] missing closing `)`",
                ));
            }
            push_ebnf_with_repeat(EbnfToken::Group(inner), chars, idx, &mut out);
            continue;
        }
        if ch == '[' {
            *idx += 1;
            let inner = parse_ebnf_alt(chars, idx, Some(']'), warnings);
            if *idx < chars.len() && chars[*idx] == ']' {
                *idx += 1;
            } else {
                warnings.push(Diagnostic::warning(
                    "[W_EBNF_UNBALANCED] missing closing `]`",
                ));
            }
            out.push(EbnfToken::Optional(inner));
            continue;
        }
        if ch == '{' {
            *idx += 1;
            let inner = parse_ebnf_alt(chars, idx, Some('}'), warnings);
            if *idx < chars.len() && chars[*idx] == '}' {
                *idx += 1;
            } else {
                warnings.push(Diagnostic::warning(
                    "[W_EBNF_UNBALANCED] missing closing `}`",
                ));
            }
            out.push(EbnfToken::Repetition(inner));
            continue;
        }
        if ch.is_alphanumeric() || ch == '_' {
            let mut name = String::new();
            while *idx < chars.len()
                && (chars[*idx].is_alphanumeric() || chars[*idx] == '_' || chars[*idx] == '-')
            {
                name.push(chars[*idx]);
                *idx += 1;
            }
            push_ebnf_with_repeat(EbnfToken::NonTerminal(name), chars, idx, &mut out);
            continue;
        }
        // Unknown character; skip with warning.
        warnings.push(Diagnostic::warning(format!(
            "[W_EBNF_UNSUPPORTED_CHAR] unsupported character `{ch}`"
        )));
        out.push(EbnfToken::Unsupported(ch.to_string()));
        *idx += 1;
    }
    out
}

fn push_ebnf_with_repeat(
    token: EbnfToken,
    chars: &[char],
    idx: &mut usize,
    out: &mut Vec<EbnfToken>,
) {
    if *idx < chars.len() {
        let kind = match chars[*idx] {
            '*' => Some(RepeatKind::ZeroOrMore),
            '+' => Some(RepeatKind::OneOrMore),
            '?' => Some(RepeatKind::ZeroOrOne),
            _ => None,
        };
        if let Some(kind) = kind {
            *idx += 1;
            out.push(EbnfToken::Repeat {
                inner: Box::new(token),
                kind,
            });
            return;
        }
    }
    out.push(token);
}

fn normalize_math(document: Document) -> Result<MathDocument, Diagnostic> {
    let (title, body) = collect_raw_body(&document);
    Ok(MathDocument {
        title,
        body: body.join("\n"),
        warnings: Vec::new(),
    })
}

fn normalize_ditaa(document: Document) -> Result<DitaaDocument, Diagnostic> {
    let (title, body) = collect_raw_body(&document);
    Ok(DitaaDocument {
        title,
        body: body.join("\n"),
        warnings: Vec::new(),
    })
}

fn normalize_sdl(document: Document) -> Result<SdlDocument, Diagnostic> {
    let (title, body) = collect_raw_body(&document);
    let mut states = Vec::new();
    let mut transitions = Vec::new();
    let warnings: Vec<Diagnostic> = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    let record_state = |name: &str,
                        kind: SdlStateKind,
                        seen: &mut std::collections::BTreeSet<String>,
                        states: &mut Vec<SdlState>| {
        if !seen.contains(name) {
            seen.insert(name.to_string());
            states.push(SdlState {
                name: name.to_string(),
                kind,
            });
        }
    };
    for line in body {
        let line = line.trim();
        if line.is_empty() || line.starts_with('\'') {
            continue;
        }
        // Recognized forms:
        //   state <name>
        //   start <name>            (alias: declares <name> with Start kind)
        //   stop <name>
        //   <from> -> <to> : <signal>
        //   <from> -> <to>
        let lower = line.to_ascii_lowercase();
        if let Some(rest) = lower.strip_prefix("state ") {
            let name = line[6..].trim();
            let _ = rest;
            record_state(name, SdlStateKind::State, &mut seen, &mut states);
            continue;
        }
        if let Some(_rest) = lower.strip_prefix("start ") {
            let name = line[6..].trim();
            record_state(name, SdlStateKind::Start, &mut seen, &mut states);
            continue;
        }
        if let Some(_rest) = lower.strip_prefix("stop ") {
            let name = line[5..].trim();
            record_state(name, SdlStateKind::Stop, &mut seen, &mut states);
            continue;
        }
        if let Some((core, signal)) = line.split_once(':') {
            if let Some((from, to)) = core.split_once("->") {
                let from = from.trim().to_string();
                let to = to.trim().to_string();
                record_state(&from, SdlStateKind::State, &mut seen, &mut states);
                record_state(&to, SdlStateKind::State, &mut seen, &mut states);
                transitions.push(SdlTransition {
                    from,
                    to,
                    signal: Some(signal.trim().to_string()),
                });
                continue;
            }
        }
        if let Some((from, to)) = line.split_once("->") {
            let from = from.trim().to_string();
            let to = to.trim().to_string();
            record_state(&from, SdlStateKind::State, &mut seen, &mut states);
            record_state(&to, SdlStateKind::State, &mut seen, &mut states);
            transitions.push(SdlTransition {
                from,
                to,
                signal: None,
            });
            continue;
        }
        // Otherwise treat as a state declaration.
        record_state(line, SdlStateKind::State, &mut seen, &mut states);
    }
    Ok(SdlDocument {
        title,
        states,
        transitions,
        warnings,
    })
}

fn normalize_chart(document: Document) -> Result<ChartDocument, Diagnostic> {
    let (title, body) = collect_raw_body(&document);
    let mut subtype = ChartSubtype::Bar;
    let mut data = Vec::new();
    let mut style = ChartStyle::default();
    let mut warnings: Vec<Diagnostic> = Vec::new();
    let mut first_non_empty = true;
    for line in body {
        let line = line.trim();
        if line.is_empty() || line.starts_with('\'') {
            continue;
        }
        if let Some(theme_name) = line.strip_prefix("!theme ") {
            style = chart_style_from_sequence_theme(
                &resolve_sequence_theme_preset(theme_name)
                    .map_err(Diagnostic::error)?
                    .style,
            );
            continue;
        }
        if line.to_ascii_lowercase().starts_with("skinparam ") {
            let rest = line[10..].trim();
            let mut parts = rest.splitn(2, char::is_whitespace);
            let key = parts.next().unwrap_or("").trim();
            let value = parts.next().unwrap_or("").trim();
            use crate::theme::ChartSkinParamValue;
            match classify_chart_skinparam(key, value) {
                SkinParamSupport::SupportedNoop => {}
                SkinParamSupport::SupportedWithValue(v) => match v {
                    ChartSkinParamValue::BackgroundColor(c) => style.background_color = c,
                    ChartSkinParamValue::AxisColor(c) => style.axis_color = c,
                    ChartSkinParamValue::GridColor(c) => style.grid_color = c,
                    ChartSkinParamValue::SeriesColor(c) => style.series_color = c,
                    ChartSkinParamValue::BarColor(c) => style.bar_color = c,
                    ChartSkinParamValue::LineColor(c) => style.line_color = c,
                    ChartSkinParamValue::PieBorderColor(c) => style.pie_border_color = c,
                    ChartSkinParamValue::FontColor(c) => style.font_color = c,
                },
                SkinParamSupport::UnsupportedKey => warnings.push(Diagnostic::warning(format!(
                    "[W_SKINPARAM_UNSUPPORTED] unsupported skinparam `{}`",
                    key
                ))),
                SkinParamSupport::UnsupportedValue => warnings.push(Diagnostic::warning(format!(
                    "[W_SKINPARAM_UNSUPPORTED_VALUE] unsupported value `{}` for skinparam `{}`",
                    value, key
                ))),
            }
            continue;
        }
        if first_non_empty {
            first_non_empty = false;
            match line.to_ascii_lowercase().as_str() {
                "bar" | "bars" => {
                    subtype = ChartSubtype::Bar;
                    continue;
                }
                "line" | "lines" => {
                    subtype = ChartSubtype::Line;
                    continue;
                }
                "pie" => {
                    subtype = ChartSubtype::Pie;
                    continue;
                }
                _ => {
                    // not a subtype keyword; fall through to data parsing.
                }
            }
        }
        // Parse data point: "Label" value  OR  Label value
        let (label, rest) = if let Some(stripped) = line.strip_prefix('"') {
            if let Some(end) = stripped.find('"') {
                (stripped[..end].to_string(), stripped[end + 1..].trim())
            } else {
                warnings.push(Diagnostic::warning(format!(
                    "[W_CHART_UNQUOTED] unterminated quoted label on line `{line}`"
                )));
                (stripped.to_string(), "")
            }
        } else {
            let mut parts = line.splitn(2, char::is_whitespace);
            let head = parts.next().unwrap_or("");
            let tail = parts.next().unwrap_or("").trim();
            (head.to_string(), tail)
        };
        let value_str = rest.split_whitespace().next().unwrap_or("");
        match value_str.parse::<f64>() {
            Ok(v) => data.push(ChartPoint { label, value: v }),
            Err(_) => warnings.push(Diagnostic::warning(format!(
                "[W_CHART_NUMERIC] could not parse numeric value `{value_str}`"
            ))),
        }
    }
    Ok(ChartDocument {
        title,
        subtype,
        data,
        style,
        warnings,
    })
}

fn normalize_timeline_baseline(document: Document) -> Result<TimelineDocument, Diagnostic> {
    let mut tasks = Vec::new();
    let mut milestones = Vec::new();
    let mut constraints = Vec::new();
    let mut chronology_events = Vec::new();
    let mut title = None;
    let mut header = None;
    let mut footer = None;
    let mut caption = None;
    let mut legend = None;

    for stmt in document.statements {
        match stmt.kind {
            StatementKind::GanttTaskDecl {
                name,
                start_date,
                duration_days,
                resources,
                ..
            } => tasks.push(TimelineTask {
                name,
                start_day: start_date
                    .as_deref()
                    .and_then(parse_iso_date_day)
                    .unwrap_or(0),
                duration_days: duration_days.unwrap_or(1).max(1),
                resources,
            }),
            StatementKind::GanttMilestoneDecl { name, happens_on } => {
                if let Some(target) = &happens_on {
                    constraints.push(TimelineConstraint {
                        subject: name.clone(),
                        kind: "happens".to_string(),
                        target: target.clone(),
                    });
                }
                milestones.push(TimelineMilestone { name, happens_on })
            }
            StatementKind::GanttConstraint {
                subject,
                kind,
                target,
            } => constraints.push(TimelineConstraint {
                subject,
                kind,
                target,
            }),
            StatementKind::ChronologyHappensOn { subject, when } => {
                chronology_events.push(TimelineChronologyEvent { subject, when })
            }
            StatementKind::Title(v) => title = Some(v),
            StatementKind::Header(v) => header = Some(v),
            StatementKind::Footer(v) => footer = Some(v),
            StatementKind::Caption(v) => caption = Some(v),
            StatementKind::Legend(v) => {
                legend = Some(strip_legend_pos_prefix(&v));
            }
            StatementKind::SkinParam { .. }
            | StatementKind::Theme(_)
            | StatementKind::Pragma(_)
            | StatementKind::Scale(_)
            | StatementKind::LegendPos(_)
            | StatementKind::SetOption { .. }
            | StatementKind::HideOption(_) => {}
            StatementKind::Unknown(line) => {
                return Err(Diagnostic::error(line).with_span(stmt.span));
            }
            _ => {
                let family = family_kind_name(document.kind);
                return Err(Diagnostic::error(format!(
                    "[E_TIMELINE_BASELINE_UNSUPPORTED] unsupported {family} syntax in baseline slice"
                ))
                .with_span(stmt.span));
            }
        }
    }

    let project_start = constraints
        .iter()
        .find(|c| {
            c.subject.eq_ignore_ascii_case("Project")
                && c.kind.eq_ignore_ascii_case("starts")
                && parse_iso_date_day(&c.target).is_some()
        })
        .map(|c| c.target.clone());
    let project_start_day = project_start.as_deref().and_then(parse_iso_date_day);

    if document.kind == DiagramKind::Gantt && !tasks.is_empty() {
        let fallback_anchor = project_start_day.unwrap_or_else(|| {
            tasks
                .iter()
                .filter(|t| t.start_day > 0)
                .map(|t| t.start_day)
                .min()
                .unwrap_or(0)
        });
        let mut cursor = fallback_anchor;
        for task in &mut tasks {
            if task.start_day == 0 {
                task.start_day = cursor;
            }
            let task_end = task.start_day.saturating_add(task.duration_days);
            if task_end > cursor {
                cursor = task_end;
            }
        }
    }

    Ok(TimelineDocument {
        kind: document.kind,
        tasks,
        milestones,
        constraints,
        chronology_events,
        project_start,
        project_start_day,
        title,
        header,
        footer,
        caption,
        legend,
        warnings: Vec::new(),
    })
}

fn parse_iso_date_day(raw: &str) -> Option<u32> {
    let mut parts = raw.trim().split('-');
    let y = parts.next()?.parse::<i64>().ok()?;
    let m = parts.next()?.parse::<i64>().ok()?;
    let d = parts.next()?.parse::<i64>().ok()?;
    if parts.next().is_some() || !(1..=12).contains(&m) || !(1..=31).contains(&d) || y < 0 {
        return None;
    }
    let y_adj = y - if m <= 2 { 1 } else { 0 };
    let era = if y_adj >= 0 { y_adj } else { y_adj - 399 } / 400;
    let yoe = y_adj - era * 400;
    let mp = m + if m > 2 { -3 } else { 9 };
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe - 719468;
    if days < 0 {
        return None;
    }
    u32::try_from(days).ok()
}

fn collect_raw_block(document: &Document) -> (String, Option<String>) {
    let mut lines: Vec<String> = Vec::new();
    let mut title: Option<String> = None;
    for stmt in &document.statements {
        match &stmt.kind {
            StatementKind::RawBlockContent(s) | StatementKind::RawBody(s) => lines.push(s.clone()),
            StatementKind::Title(v) => title = Some(v.clone()),
            _ => {}
        }
    }
    (lines.join("\n"), title)
}

fn normalize_json_document(document: Document) -> Result<JsonDocument, Diagnostic> {
    let (raw, title) = collect_raw_block(&document);
    let nodes = match serde_json::from_str::<serde_json::Value>(raw.trim()) {
        Ok(value) => {
            let mut out = Vec::new();
            flatten_json_value(&value, None, 0, &mut out);
            out
        }
        Err(_) => raw
            .lines()
            .map(|line| JsonTreeNode {
                depth: 0,
                label: line.trim_end().to_string(),
            })
            .collect(),
    };
    Ok(JsonDocument {
        raw,
        nodes,
        title,
        warnings: Vec::new(),
    })
}

fn flatten_json_value(
    value: &serde_json::Value,
    label: Option<&str>,
    depth: usize,
    out: &mut Vec<JsonTreeNode>,
) {
    use serde_json::Value;
    match value {
        Value::Object(map) => {
            let header = label
                .map(|l| format!("{l}: {{...}}"))
                .unwrap_or_else(|| "{...}".to_string());
            out.push(JsonTreeNode {
                depth,
                label: header,
            });
            for (k, v) in map {
                flatten_json_value(v, Some(k), depth + 1, out);
            }
        }
        Value::Array(items) => {
            let header = label
                .map(|l| format!("{l}: [...]"))
                .unwrap_or_else(|| "[...]".to_string());
            out.push(JsonTreeNode {
                depth,
                label: header,
            });
            for (i, v) in items.iter().enumerate() {
                let key = format!("[{i}]");
                flatten_json_value(v, Some(&key), depth + 1, out);
            }
        }
        Value::String(s) => out.push(JsonTreeNode {
            depth,
            label: label
                .map(|l| format!("{l}: \"{s}\""))
                .unwrap_or_else(|| format!("\"{s}\"")),
        }),
        Value::Number(n) => out.push(JsonTreeNode {
            depth,
            label: label
                .map(|l| format!("{l}: {n}"))
                .unwrap_or_else(|| n.to_string()),
        }),
        Value::Bool(b) => out.push(JsonTreeNode {
            depth,
            label: label
                .map(|l| format!("{l}: {b}"))
                .unwrap_or_else(|| b.to_string()),
        }),
        Value::Null => out.push(JsonTreeNode {
            depth,
            label: label
                .map(|l| format!("{l}: null"))
                .unwrap_or_else(|| "null".to_string()),
        }),
    }
}

fn normalize_yaml_document(document: Document) -> Result<YamlDocument, Diagnostic> {
    let (raw, title) = collect_raw_block(&document);
    let mut nodes = Vec::new();
    for line in raw.lines() {
        // Strip trailing whitespace; skip fully blank lines and comment-only lines.
        let trimmed_end = line.trim_end();
        if trimmed_end.is_empty() {
            continue;
        }
        let indent = trimmed_end.len() - trimmed_end.trim_start().len();
        let depth = indent / 2;
        let content = trimmed_end.trim_start();
        if content.starts_with('#') {
            continue;
        }
        nodes.push(YamlTreeNode {
            depth,
            label: content.to_string(),
        });
    }
    Ok(YamlDocument {
        raw,
        nodes,
        title,
        warnings: Vec::new(),
    })
}

fn normalize_nwdiag_document(document: Document) -> Result<NwdiagDocument, Diagnostic> {
    let (raw, title) = collect_raw_block(&document);
    let mut networks: Vec<NwdiagNetwork> = Vec::new();
    let mut current: Option<NwdiagNetwork> = None;
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("network ") {
            // close any previous network without explicit `}` (lenient)
            if let Some(net) = current.take() {
                networks.push(net);
            }
            let name = rest.trim_end_matches('{').trim().to_string();
            current = Some(NwdiagNetwork {
                name,
                address: None,
                nodes: Vec::new(),
            });
            continue;
        }
        if trimmed == "}" {
            if let Some(net) = current.take() {
                networks.push(net);
            }
            continue;
        }
        if let Some(net) = current.as_mut() {
            // address = "..."
            if let Some(rest) = trimmed.strip_prefix("address") {
                let value = rest
                    .trim_start_matches([' ', '='])
                    .trim()
                    .trim_matches('"')
                    .to_string();
                net.address = Some(value);
                continue;
            }
            // NodeName [address = "..."] or NodeName
            let (name_part, attrs) = match trimmed.split_once('[') {
                Some((n, rest)) => (n.trim().to_string(), Some(rest.trim_end_matches(']'))),
                None => (trimmed.to_string(), None),
            };
            let mut node_address: Option<String> = None;
            if let Some(attrs) = attrs {
                for kv in attrs.split(',') {
                    if let Some((k, v)) = kv.split_once('=') {
                        if k.trim() == "address" {
                            node_address = Some(v.trim().trim_matches('"').to_string());
                        }
                    }
                }
            }
            net.nodes.push(NwdiagNode {
                name: name_part,
                address: node_address,
            });
        }
    }
    if let Some(net) = current.take() {
        networks.push(net);
    }
    Ok(NwdiagDocument {
        networks,
        title,
        warnings: Vec::new(),
    })
}

fn normalize_archimate_document(document: Document) -> Result<ArchimateDocument, Diagnostic> {
    let (raw, title) = collect_raw_block(&document);
    let mut elements: Vec<ArchimateElement> = Vec::new();
    let mut relations: Vec<ArchimateRelation> = Vec::new();

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('\'') {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("archimate ") {
            // archimate "Name" as alias <<layer>>
            if let Some(elem) = parse_archimate_element(rest) {
                elements.push(elem);
                continue;
            }
        }
        // ArchiMate stdlib-style declarations:
        // Business_Actor(customer, "Customer")
        // Application_Component(service, "Order Service")
        // Technology_Node(host, "Runtime")
        if let Some(elem) = parse_archimate_macro_element(trimmed) {
            elements.push(elem);
            continue;
        }
        // Relation macros: Rel_Association(a, b, "label"), Rel_Realization(a, b)
        if let Some(open) = trimmed.find('(') {
            let macro_name = trimmed[..open].trim();
            if let Some(kind) = archimate_rel_kind_from_macro(macro_name) {
                let inside = trimmed[open + 1..].trim_end_matches([')', ' ', '\t']);
                let args: Vec<String> = split_csv_args(inside);
                if args.len() >= 2 {
                    let from = args[0].trim().trim_matches('"').to_string();
                    let to = args[1].trim().trim_matches('"').to_string();
                    let label = args
                        .get(2)
                        .map(|s| s.trim().trim_matches('"').to_string())
                        .filter(|s| !s.is_empty());
                    relations.push(ArchimateRelation {
                        from,
                        to,
                        kind: kind.to_string(),
                        label,
                    });
                    continue;
                }
            }
        }
        // Plain arrow: a --> b : label
        if let Some(rel) = parse_archimate_arrow(trimmed) {
            relations.push(rel);
            continue;
        }
    }

    Ok(ArchimateDocument {
        elements,
        relations,
        title,
        warnings: Vec::new(),
    })
}

fn parse_archimate_element(rest: &str) -> Option<ArchimateElement> {
    // expect: "Name" as alias <<layer>>  OR  Name <<layer>>  OR  "Name" <<layer>>
    let mut s = rest.trim().to_string();
    let mut layer = "business".to_string();
    if let Some(open) = s.find("<<") {
        if let Some(close) = s[open + 2..].find(">>") {
            layer = s[open + 2..open + 2 + close].trim().to_string();
            s = format!("{} {}", &s[..open], &s[open + 2 + close + 2..]);
        }
    }
    let s = s.trim();
    let (name, alias) = if let Some(stripped) = s.strip_prefix('"') {
        let close = stripped.find('"')?;
        let name = stripped[..close].to_string();
        let rest = stripped[close + 1..].trim();
        let alias = rest.strip_prefix("as ").map(|a| a.trim().to_string());
        (name, alias)
    } else {
        let mut parts = s.split_whitespace();
        let name = parts.next()?.to_string();
        let alias = if parts.next() == Some("as") {
            parts.next().map(|s| s.to_string())
        } else {
            None
        };
        (name, alias)
    };
    Some(ArchimateElement { name, alias, layer })
}

fn parse_archimate_macro_element(line: &str) -> Option<ArchimateElement> {
    let open = line.find('(')?;
    let macro_name = line[..open].trim();
    let layer = archimate_layer_from_macro(macro_name)?;
    let inside = line[open + 1..].trim_end_matches([')', ' ', '\t']);
    let args = split_csv_args(inside);
    let alias = args.first()?.trim().trim_matches('"').to_string();
    if alias.is_empty() {
        return None;
    }
    let name = args
        .get(1)
        .map(|s| s.trim().trim_matches('"').to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| alias.clone());
    Some(ArchimateElement {
        name,
        alias: Some(alias),
        layer: layer.to_string(),
    })
}

fn archimate_layer_from_macro(name: &str) -> Option<&'static str> {
    let lower = name.to_ascii_lowercase();
    if lower.starts_with("strategy_") {
        Some("strategy")
    } else if lower.starts_with("business_") {
        Some("business")
    } else if lower.starts_with("application_") {
        Some("application")
    } else if lower.starts_with("technology_") {
        Some("technology")
    } else if lower.starts_with("physical_") {
        Some("technology")
    } else if lower.starts_with("motivation_") {
        Some("motivation")
    } else if lower.starts_with("implementation_") || lower.starts_with("migration_") {
        Some("strategy")
    } else {
        None
    }
}

fn archimate_rel_kind_from_macro(name: &str) -> Option<&'static str> {
    match name {
        "Rel_Access" => Some("access"),
        "Rel_Aggregation" => Some("aggregation"),
        "Rel_Association" => Some("association"),
        "Rel_Assignment" => Some("assignment"),
        "Rel_Composition" => Some("composition"),
        "Rel_Flow" => Some("flow"),
        "Rel_Influence" => Some("influence"),
        "Rel_Realization" => Some("realization"),
        "Rel_Serving" => Some("serving"),
        "Rel_Specialization" => Some("specialization"),
        "Rel_Triggering" => Some("triggering"),
        "Rel_Used_By" => Some("used_by"),
        _ => None,
    }
}

fn split_csv_args(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_quotes = false;
    for ch in s.chars() {
        if ch == '"' {
            in_quotes = !in_quotes;
            cur.push(ch);
        } else if ch == ',' && !in_quotes {
            out.push(std::mem::take(&mut cur));
        } else {
            cur.push(ch);
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn parse_archimate_arrow(line: &str) -> Option<ArchimateRelation> {
    for arrow in ["-->", "->", "<--", "<-"] {
        if let Some(ix) = line.find(arrow) {
            let lhs = line[..ix].trim();
            let rhs_full = line[ix + arrow.len()..].trim();
            if lhs.is_empty() || rhs_full.is_empty() {
                return None;
            }
            let (rhs, label) = match rhs_full.split_once(':') {
                Some((r, l)) => (r.trim(), Some(l.trim().to_string())),
                None => (rhs_full, None),
            };
            return Some(ArchimateRelation {
                from: lhs.to_string(),
                to: rhs.to_string(),
                kind: "uses".to_string(),
                label,
            });
        }
    }
    None
}

fn normalize_stub_family(document: Document) -> Result<FamilyDocument, Diagnostic> {
    let family_kind = document.kind;
    let node_kind = match family_kind {
        DiagramKind::Class => FamilyNodeKind::Class,
        DiagramKind::Object => FamilyNodeKind::Object,
        DiagramKind::UseCase => FamilyNodeKind::UseCase,
        DiagramKind::Salt => FamilyNodeKind::Salt,
        _ => {
            return Err(Diagnostic::error(
                "[E_FAMILY_STUB_INTERNAL] invalid family for stub normalization",
            ));
        }
    };

    let mut nodes = Vec::new();
    let mut relations = Vec::new();
    let mut groups = Vec::new();
    let mut json_projections: Vec<crate::model::JsonProjection> = Vec::new();
    let mut hide_options = std::collections::BTreeSet::new();
    let mut namespace_separator: Option<String> = None;
    let mut title = None;
    let mut header = None;
    let mut footer = None;
    let mut caption = None;
    let mut legend = None;
    let mut class_style = ClassStyle::default();
    let mut warnings: Vec<Diagnostic> = Vec::new();

    for stmt in document.statements {
        match stmt.kind {
            StatementKind::SkinParam { key, value } => {
                match classify_class_skinparam(&key, &value) {
                    SkinParamSupport::SupportedNoop => {}
                    SkinParamSupport::SupportedWithValue(v) => {
                        use crate::theme::ClassSkinParamValue;
                        match v {
                            ClassSkinParamValue::BackgroundColor(c) => {
                                class_style.background_color = c;
                            }
                            ClassSkinParamValue::BorderColor(c) => {
                                class_style.border_color = c;
                            }
                            ClassSkinParamValue::HeaderBackgroundColor(c) => {
                                class_style.header_color = c;
                            }
                            ClassSkinParamValue::MemberFontColor(c) => {
                                class_style.member_color = c;
                            }
                            ClassSkinParamValue::ArrowColor(c) => {
                                class_style.arrow_color = c;
                            }
                            ClassSkinParamValue::FontSize(n) => {
                                class_style.font_size = Some(n);
                            }
                            ClassSkinParamValue::FontName(n) => {
                                class_style.font_name = Some(n);
                            }
                        }
                    }
                    SkinParamSupport::UnsupportedKey => {
                        // Class diagrams accept generic sequence keys silently
                        // (PlantUML applies them across all families).
                        use crate::theme::{classify_sequence_skinparam, SequenceSkinParamSupport};
                        if !matches!(
                            classify_sequence_skinparam(&key, &value),
                            SequenceSkinParamSupport::UnsupportedKey
                        ) {
                            // Recognized sequence key — no warning.
                        } else {
                            warnings.push(
                                Diagnostic::warning(format!(
                                    "[W_SKINPARAM_UNSUPPORTED] unsupported skinparam `{}`",
                                    key
                                ))
                                .with_span(stmt.span),
                            );
                        }
                    }
                    SkinParamSupport::UnsupportedValue => {
                        warnings.push(
                            Diagnostic::warning(format!(
                                "[W_SKINPARAM_UNSUPPORTED_VALUE] unsupported value `{}` for skinparam `{}`",
                                value, key
                            ))
                            .with_span(stmt.span),
                        );
                    }
                }
            }
            StatementKind::JsonProjection { alias, body } => {
                json_projections.push(crate::model::JsonProjection { alias, body });
            }
            StatementKind::YamlProjection { alias, body } => {
                json_projections.push(crate::model::JsonProjection { alias, body });
            }
            StatementKind::ClassDecl(decl) => {
                if node_kind != FamilyNodeKind::Class {
                    return Err(Diagnostic::error(format!(
                        "[E_FAMILY_MIXED] mixed diagram families are not supported: found class declaration in {} diagram",
                        family_kind_name(family_kind)
                    ))
                    .with_span(stmt.span));
                }
                nodes.push(FamilyNode {
                    kind: FamilyNodeKind::Class,
                    name: decl.name,
                    alias: decl.alias,
                    members: decl.members,
                    depth: 0,
                    label: None,
                    mindmap_side: MindMapSide::Right,
                    wbs_checkbox: None,
                });
            }
            StatementKind::ObjectDecl(decl) => {
                if node_kind != FamilyNodeKind::Object {
                    return Err(Diagnostic::error(format!(
                        "[E_FAMILY_MIXED] mixed diagram families are not supported: found object declaration in {} diagram",
                        family_kind_name(family_kind)
                    ))
                    .with_span(stmt.span));
                }
                // Detect and strip C4 stereotypes embedded in the alias
                // (e.g. `u <<person>>` → alias `u`, kind `C4Person`).
                let (clean_alias, c4_kind) = extract_c4_stereotype(decl.alias);
                let resolved_kind = c4_kind.unwrap_or(FamilyNodeKind::Object);
                nodes.push(FamilyNode {
                    kind: resolved_kind,
                    name: decl.name,
                    alias: clean_alias,
                    members: decl.members,
                    depth: 0,
                    label: None,
                    mindmap_side: MindMapSide::Right,
                    wbs_checkbox: None,
                });
            }
            StatementKind::UseCaseDecl(decl) => {
                if node_kind != FamilyNodeKind::UseCase {
                    return Err(Diagnostic::error(format!(
                        "[E_FAMILY_MIXED] mixed diagram families are not supported: found usecase declaration in {} diagram",
                        family_kind_name(family_kind)
                    ))
                    .with_span(stmt.span));
                }
                nodes.push(FamilyNode {
                    kind: FamilyNodeKind::UseCase,
                    name: decl.name,
                    alias: decl.alias,
                    members: decl.members,
                    depth: 0,
                    label: None,
                    mindmap_side: MindMapSide::Right,
                    wbs_checkbox: None,
                });
            }
            StatementKind::FamilyRelation(rel) => relations.push(ModelFamilyRelation {
                from: rel.from,
                to: rel.to,
                arrow: rel.arrow,
                label: rel.label,
                left_cardinality: rel.left_cardinality,
                right_cardinality: rel.right_cardinality,
                left_role: rel.left_role,
                right_role: rel.right_role,
            }),
            StatementKind::ClassGroup {
                kind,
                label,
                members,
            } => {
                // Auto-create nodes for members declared inside a package/namespace block
                // if they haven't already been declared as top-level statements.
                for member_id in &members {
                    let already_exists = nodes.iter().any(|n: &FamilyNode| {
                        n.name == *member_id || n.alias.as_deref() == Some(member_id.as_str())
                    });
                    if !already_exists {
                        let nk = match node_kind {
                            FamilyNodeKind::Object => FamilyNodeKind::Object,
                            FamilyNodeKind::UseCase => FamilyNodeKind::UseCase,
                            _ => FamilyNodeKind::Class,
                        };
                        nodes.push(FamilyNode {
                            kind: nk,
                            name: member_id.clone(),
                            alias: None,
                            members: Vec::new(),
                            depth: 0,
                            label: None,
                            mindmap_side: MindMapSide::Right,
                            wbs_checkbox: None,
                        });
                    }
                }
                groups.push(FamilyGroup {
                    kind,
                    label,
                    member_ids: members,
                });
            }
            StatementKind::SetOption { key, value } => {
                if key.eq_ignore_ascii_case("namespaceSeparator") {
                    namespace_separator = Some(value);
                }
            }
            StatementKind::HideOption(opt) => {
                hide_options.insert(opt.to_ascii_lowercase());
            }
            StatementKind::Title(v) => title = Some(v),
            StatementKind::Header(v) => header = Some(v),
            StatementKind::Footer(v) => footer = Some(v),
            StatementKind::Caption(v) => caption = Some(v),
            StatementKind::Legend(v) => legend = Some(v),
            StatementKind::Theme(value) => {
                class_style = class_style_from_sequence_theme(
                    &resolve_sequence_theme_preset(&value)
                        .map_err(|msg| Diagnostic::error(msg).with_span(stmt.span))?
                        .style,
                );
            }
            StatementKind::Pragma(_)
            | StatementKind::Include(_)
            | StatementKind::Define { .. }
            | StatementKind::Undef(_) => {}
            StatementKind::SaltGridRow { cells } => {
                if family_kind != DiagramKind::Salt {
                    return Err(Diagnostic::error(
                        "[E_FAMILY_MIXED] mixed diagram families are not supported in one document",
                    )
                    .with_span(stmt.span));
                }
                // Encode the cells into the name field using a special separator
                // so the renderer can reconstruct the grid row.
                use crate::ast::SaltCell as SC;
                let cell_strs: Vec<String> = cells
                    .into_iter()
                    .map(|c| match c {
                        SC::Label(t) => format!("L:{t}"),
                        SC::Input(t) => format!("I:{t}"),
                        SC::Button(t) => format!("B:{t}"),
                        SC::Combo(t) => format!("C:{t}"),
                        SC::CheckboxChecked(t) => format!("CX:{t}"),
                        SC::CheckboxUnchecked(t) => format!("CU:{t}"),
                        SC::RadioOn(t) => format!("RO:{t}"),
                        SC::RadioOff(t) => format!("RF:{t}"),
                    })
                    .collect();
                nodes.push(FamilyNode {
                    kind: FamilyNodeKind::Salt,
                    name: format!("SALT_ROW\x1f{}", cell_strs.join("\x1e")),
                    alias: None,
                    members: Vec::new(),
                    depth: 0,
                    label: None,
                    mindmap_side: MindMapSide::Right,
                    wbs_checkbox: None,
                });
            }
            StatementKind::Unknown(line) if family_kind == DiagramKind::Salt => {
                if line.trim() == "---" {
                    continue;
                }
                // Treat non-row unknown lines as plain label rows
                nodes.push(FamilyNode {
                    kind: FamilyNodeKind::Salt,
                    name: format!("SALT_ROW\x1fL:{line}"),
                    alias: None,
                    members: Vec::new(),
                    depth: 0,
                    label: None,
                    mindmap_side: MindMapSide::Right,
                    wbs_checkbox: None,
                });
            }
            StatementKind::Unknown(line) => {
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

    Ok(FamilyDocument {
        kind: family_kind,
        nodes,
        relations,
        groups,
        json_projections,
        hide_options,
        namespace_separator,
        title,
        header,
        footer,
        caption,
        legend,
        orientation: FamilyOrientation::TopToBottom,
        style: SequenceStyle::default(),
        family_style: Some(FamilyStyle::Class(class_style)),
        text_overflow_policy: TextOverflowPolicy::WrapAndGrow,
        warnings,
    })
}

fn normalize_state(document: Document) -> Result<StateDocument, Diagnostic> {
    let mut nodes: Vec<StateNode> = Vec::new();
    let mut transitions: Vec<ModelStateTransition> = Vec::new();
    let mut title = None;
    let mut header = None;
    let mut footer = None;
    let mut caption = None;
    let mut legend = None;
    let mut state_style = StateStyle::default();
    let mut warnings: Vec<Diagnostic> = Vec::new();

    for stmt in &document.statements {
        match &stmt.kind {
            StatementKind::StateDecl(decl) => {
                let node = state_decl_to_node(decl);
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
                });
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
                        internal_actions: Vec::new(),
                        regions: Vec::new(),
                    },
                );
            }
            StatementKind::Title(v) => title = Some(v.clone()),
            StatementKind::Header(v) => header = Some(v.clone()),
            StatementKind::Footer(v) => footer = Some(v.clone()),
            StatementKind::Caption(v) => caption = Some(v.clone()),
            StatementKind::Legend(v) => legend = Some(v.clone()),
            StatementKind::SkinParam { key, value } => {
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
                        StateSkinParamValue::FontSize(n) => {
                            state_style.font_size = Some(n);
                        }
                    },
                    SkinParamSupport::UnsupportedKey => {
                        warnings.push(
                            Diagnostic::warning(format!(
                                "[W_SKINPARAM_UNSUPPORTED] unsupported skinparam `{}`",
                                key
                            ))
                            .with_span(stmt.span),
                        );
                    }
                    SkinParamSupport::UnsupportedValue => {
                        warnings.push(
                            Diagnostic::warning(format!(
                                "[W_SKINPARAM_UNSUPPORTED_VALUE] unsupported value `{}` for skinparam `{}`",
                                value, key
                            ))
                            .with_span(stmt.span),
                        );
                    }
                }
            }
            StatementKind::Theme(value) => {
                state_style = state_style_from_sequence_theme(
                    &resolve_sequence_theme_preset(&value)
                        .map_err(|msg| Diagnostic::error(msg).with_span(stmt.span))?
                        .style,
                );
            }
            StatementKind::Pragma(_)
            | StatementKind::Include(_)
            | StatementKind::Define { .. }
            | StatementKind::Undef(_) => {}
            StatementKind::StateRegionDivider => {}
            StatementKind::Unknown(line) => {
                return Err(Diagnostic::error(format!(
                    "[E_STATE_UNSUPPORTED_SYNTAX] unsupported state diagram syntax: `{}`",
                    line
                ))
                .with_span(stmt.span));
            }
            _ => {
                return Err(Diagnostic::error(
                    "[E_STATE_MIXED] mixed diagram families are not supported in one document",
                )
                .with_span(stmt.span));
            }
        }
    }

    Ok(StateDocument {
        kind: document.kind,
        nodes,
        transitions,
        title,
        header,
        footer,
        caption,
        legend,
        state_style,
        warnings,
    })
}

fn state_decl_to_node(decl: &crate::ast::StateDecl) -> StateNode {
    let kind = match decl.stereotype.as_deref() {
        Some("fork") => StateNodeKind::Fork,
        Some("join") => StateNodeKind::Join,
        Some("choice") => StateNodeKind::Choice,
        Some("end") => StateNodeKind::End,
        _ => StateNodeKind::Normal,
    };

    // Parse children into regions separated by region_dividers
    let mut regions: Vec<Vec<StateNode>> = Vec::new();
    let mut current_region: Vec<StateNode> = Vec::new();
    let mut divider_iter = decl.region_dividers.iter().peekable();

    for (child_idx, child_stmt) in decl.children.iter().enumerate() {
        // Check if a divider appears before this child
        while divider_iter.peek() == Some(&&child_idx) {
            divider_iter.next();
            regions.push(std::mem::take(&mut current_region));
        }
        match &child_stmt.kind {
            StatementKind::StateDecl(child_decl) => {
                current_region.push(state_decl_to_node(child_decl));
            }
            StatementKind::StateHistory { deep } => {
                current_region.push(StateNode {
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
                    kind: if *deep {
                        StateNodeKind::HistoryDeep
                    } else {
                        StateNodeKind::HistoryShallow
                    },
                    internal_actions: Vec::new(),
                    regions: Vec::new(),
                });
            }
            StatementKind::StateInternalAction(a) => {
                // Apply to parent node's internal actions (will be collected below)
                let _ = a;
            }
            _ => {}
        }
    }
    regions.push(current_region);

    // Collect internal actions from direct children
    let mut internal_actions: Vec<ModelStateInternalAction> = Vec::new();
    for child_stmt in &decl.children {
        if let StatementKind::StateInternalAction(a) = &child_stmt.kind {
            // Only collect actions targeted at this parent state
            if a.state == decl.name {
                internal_actions.push(ModelStateInternalAction {
                    kind: a.kind.clone(),
                    action: a.action.clone(),
                });
            }
        }
    }

    StateNode {
        name: decl.alias.clone().unwrap_or_else(|| decl.name.clone()),
        display: Some(decl.name.clone()),
        kind,
        internal_actions,
        regions,
    }
}

/// Ensure a state node exists in the list, creating a Normal node if absent.
fn ensure_state_node(nodes: &mut Vec<StateNode>, name: &str) {
    if nodes.iter().any(|n| n.name == name) {
        return;
    }
    let kind = match name {
        "[*]" => StateNodeKind::StartEnd,
        "[H]" => StateNodeKind::HistoryShallow,
        "[H*]" => StateNodeKind::HistoryDeep,
        _ => StateNodeKind::Normal,
    };
    let display = match name {
        "[*]" => None,
        "[H]" => Some("H".to_string()),
        "[H*]" => Some("H*".to_string()),
        _ => None,
    };
    nodes.push(StateNode {
        name: name.to_string(),
        display,
        kind,
        internal_actions: Vec::new(),
        regions: Vec::new(),
    });
}

/// Upsert a state node: if one with the same name already exists, update it; otherwise push.
fn upsert_state_node(nodes: &mut Vec<StateNode>, node: StateNode) {
    if let Some(existing) = nodes.iter_mut().find(|n| n.name == node.name) {
        // Merge: preserve richer kind, regions, internal_actions
        if existing.kind == StateNodeKind::Normal && node.kind != StateNodeKind::Normal {
            existing.kind = node.kind;
        }
        if !node.regions.is_empty() {
            existing.regions = node.regions;
        }
        existing.internal_actions.extend(node.internal_actions);
        if node.display.is_some() && existing.display.is_none() {
            existing.display = node.display;
        }
    } else {
        nodes.push(node);
    }
}

fn normalize_family_tree(document: Document) -> Result<FamilyDocument, Diagnostic> {
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
                left_cardinality: None,
                right_cardinality: None,
                left_role: None,
                right_role: None,
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
}

/// Parse a MindMap / WBS node line. Handles:
///
/// - `* Root`, `** Child`, `*** Grandchild` — star-depth (depth = stars - 1)
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
    if label.is_empty() {
        return None;
    }

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
    })
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

fn normalize_extended_family(document: Document) -> Result<FamilyDocument, Diagnostic> {
    let family_kind = document.kind;
    let mut nodes = Vec::new();
    let mut relations = Vec::new();
    let mut title = None;
    let mut header = None;
    let mut footer = None;
    let mut caption = None;
    let mut legend = None;
    let mut activity_step_counter: usize = 0;
    let mut activity_active_partition: Option<String> = None;
    let mut activity_fork_depth: usize = 0;
    let mut activity_fork_branch: usize = 0;
    let mut timing_current_time: Option<String> = None;
    let mut component_style = ComponentStyle::default();
    let mut activity_style = ActivityStyle::default();
    let mut timing_style = TimingStyle::default();
    let mut ext_warnings: Vec<Diagnostic> = Vec::new();

    for stmt in document.statements {
        match stmt.kind {
            StatementKind::ComponentDecl {
                kind,
                name,
                alias,
                label,
            } => {
                let node_kind = component_node_kind(kind);
                nodes.push(FamilyNode {
                    kind: node_kind,
                    name,
                    alias,
                    members: Vec::new(),
                    depth: 0,
                    label,
                    mindmap_side: MindMapSide::Right,
                    wbs_checkbox: None,
                });
            }
            StatementKind::StateDecl(decl) => nodes.push(FamilyNode {
                kind: FamilyNodeKind::State,
                name: decl.name,
                alias: decl.alias,
                members: Vec::new(),
                depth: 0,
                label: None,
                mindmap_side: MindMapSide::Right,
                wbs_checkbox: None,
            }),
            StatementKind::ActivityStep(step) => {
                activity_step_counter += 1;
                let kind = activity_step_node_kind(&step.kind);
                let name = format!("__act_{activity_step_counter:04}");
                match step.kind {
                    ActivityStepKind::PartitionStart => {
                        activity_active_partition = step.label.clone();
                    }
                    ActivityStepKind::PartitionEnd => {
                        activity_active_partition = None;
                    }
                    ActivityStepKind::Fork => {
                        activity_fork_depth += 1;
                        activity_fork_branch = 0;
                    }
                    ActivityStepKind::ForkAgain => {
                        activity_fork_branch += 1;
                    }
                    ActivityStepKind::EndFork => {
                        activity_fork_depth = activity_fork_depth.saturating_sub(1);
                        activity_fork_branch = 0;
                    }
                    _ => {}
                }
                let lane = activity_active_partition
                    .clone()
                    .unwrap_or_else(|| "default".to_string());
                let alias = format!(
                    "activity::{:?}|lane={}|fork_depth={}|fork_branch={}",
                    step.kind, lane, activity_fork_depth, activity_fork_branch
                );
                nodes.push(FamilyNode {
                    kind,
                    name,
                    alias: Some(alias),
                    members: Vec::new(),
                    depth: 0,
                    label: step.label,
                    mindmap_side: MindMapSide::Right,
                    wbs_checkbox: None,
                });
            }
            StatementKind::TimingDecl {
                kind,
                name,
                label,
                controls,
            } => {
                let node_kind = timing_decl_node_kind(kind);
                nodes.push(FamilyNode {
                    kind: node_kind,
                    name,
                    alias: None,
                    members: controls
                        .into_iter()
                        .map(|text| crate::ast::ClassMember {
                            text,
                            modifier: None,
                        })
                        .collect(),
                    depth: 0,
                    label,
                    mindmap_side: MindMapSide::Right,
                    wbs_checkbox: None,
                });
            }
            StatementKind::TimingEvent {
                time,
                signal,
                state,
                note,
            } => {
                let effective_time = if time.is_empty() {
                    timing_current_time.clone().unwrap_or_default()
                } else {
                    timing_current_time = Some(time.clone());
                    time
                };
                let display = match (&signal, &state, &note) {
                    (Some(s), Some(st), _) => format!("{s} is {st}"),
                    (None, None, Some(n)) => n.clone(),
                    _ => String::new(),
                };
                nodes.push(FamilyNode {
                    kind: FamilyNodeKind::TimingEvent,
                    name: effective_time,
                    alias: signal,
                    members: state
                        .into_iter()
                        .map(|s| crate::ast::ClassMember {
                            text: s,
                            modifier: None,
                        })
                        .collect(),
                    depth: 0,
                    label: if display.is_empty() {
                        None
                    } else {
                        Some(display)
                    },
                    mindmap_side: MindMapSide::Right,
                    wbs_checkbox: None,
                });
            }
            StatementKind::FamilyRelation(rel) => relations.push(ModelFamilyRelation {
                from: rel.from,
                to: rel.to,
                arrow: rel.arrow,
                label: rel.label,
                left_cardinality: rel.left_cardinality,
                right_cardinality: rel.right_cardinality,
                left_role: rel.left_role,
                right_role: rel.right_role,
            }),
            StatementKind::Title(v) => title = Some(v),
            StatementKind::Header(v) => header = Some(v),
            StatementKind::Footer(v) => footer = Some(v),
            StatementKind::Caption(v) => caption = Some(v),
            StatementKind::Legend(v) => {
                legend = Some(strip_legend_pos_prefix(&v));
            }
            StatementKind::SkinParam { key, value } => {
                let mut handled = false;
                if matches!(
                    family_kind,
                    DiagramKind::Component | DiagramKind::Deployment
                ) {
                    use crate::theme::ComponentSkinParamValue;
                    match classify_component_skinparam(&key, &value) {
                        SkinParamSupport::SupportedNoop => {
                            handled = true;
                        }
                        SkinParamSupport::SupportedWithValue(v) => {
                            handled = true;
                            match v {
                                ComponentSkinParamValue::BackgroundColor(c) => {
                                    component_style.background_color = c;
                                }
                                ComponentSkinParamValue::BorderColor(c) => {
                                    component_style.border_color = c;
                                }
                                ComponentSkinParamValue::InterfaceColor(c) => {
                                    component_style.interface_color = c;
                                }
                                ComponentSkinParamValue::ArrowColor(c) => {
                                    component_style.arrow_color = c;
                                }
                            }
                        }
                        SkinParamSupport::UnsupportedKey => {}
                        SkinParamSupport::UnsupportedValue => {
                            handled = true;
                            ext_warnings.push(
                                Diagnostic::warning(format!(
                                    "[W_SKINPARAM_UNSUPPORTED_VALUE] unsupported value `{}` for skinparam `{}`",
                                    value, key
                                ))
                                .with_span(stmt.span),
                            );
                        }
                    }
                }
                if !handled && matches!(family_kind, DiagramKind::Activity) {
                    use crate::theme::ActivitySkinParamValue;
                    match classify_activity_skinparam(&key, &value) {
                        SkinParamSupport::SupportedNoop => {
                            handled = true;
                        }
                        SkinParamSupport::SupportedWithValue(v) => {
                            handled = true;
                            match v {
                                ActivitySkinParamValue::BackgroundColor(c) => {
                                    activity_style.background_color = c;
                                }
                                ActivitySkinParamValue::BorderColor(c) => {
                                    activity_style.border_color = c;
                                }
                                ActivitySkinParamValue::DiamondBackgroundColor(c) => {
                                    activity_style.diamond_color = c;
                                }
                                ActivitySkinParamValue::BarColor(c) => {
                                    activity_style.fork_color = c;
                                }
                                ActivitySkinParamValue::ArrowColor(c) => {
                                    activity_style.arrow_color = c;
                                }
                            }
                        }
                        SkinParamSupport::UnsupportedKey => {}
                        SkinParamSupport::UnsupportedValue => {
                            handled = true;
                            ext_warnings.push(
                                Diagnostic::warning(format!(
                                    "[W_SKINPARAM_UNSUPPORTED_VALUE] unsupported value `{}` for skinparam `{}`",
                                    value, key
                                ))
                                .with_span(stmt.span),
                            );
                        }
                    }
                }
                if !handled && matches!(family_kind, DiagramKind::Timing) {
                    use crate::theme::TimingSkinParamValue;
                    match classify_timing_skinparam(&key, &value) {
                        SkinParamSupport::SupportedNoop => {
                            handled = true;
                        }
                        SkinParamSupport::SupportedWithValue(v) => {
                            handled = true;
                            match v {
                                TimingSkinParamValue::BackgroundColor(c) => {
                                    timing_style.background_color = c;
                                }
                                TimingSkinParamValue::AxisColor(c) => {
                                    timing_style.axis_color = c;
                                }
                                TimingSkinParamValue::GridColor(c) => {
                                    timing_style.grid_color = c;
                                }
                                TimingSkinParamValue::SignalBackgroundColor(c) => {
                                    timing_style.signal_background_color = c;
                                }
                                TimingSkinParamValue::SignalBorderColor(c) => {
                                    timing_style.signal_border_color = c;
                                }
                                TimingSkinParamValue::ArrowColor(c) => {
                                    timing_style.arrow_color = c;
                                }
                                TimingSkinParamValue::FontColor(c) => {
                                    timing_style.font_color = c;
                                }
                            }
                        }
                        SkinParamSupport::UnsupportedKey => {}
                        SkinParamSupport::UnsupportedValue => {
                            handled = true;
                            ext_warnings.push(
                                Diagnostic::warning(format!(
                                    "[W_SKINPARAM_UNSUPPORTED_VALUE] unsupported value `{}` for skinparam `{}`",
                                    value, key
                                ))
                                .with_span(stmt.span),
                            );
                        }
                    }
                }
                if !handled {
                    ext_warnings.push(
                        Diagnostic::warning(format!(
                            "[W_SKINPARAM_UNSUPPORTED] unsupported skinparam `{}`",
                            key
                        ))
                        .with_span(stmt.span),
                    );
                }
            }
            StatementKind::Theme(value) => {
                let style = resolve_sequence_theme_preset(&value)
                    .map_err(|msg| Diagnostic::error(msg).with_span(stmt.span))?
                    .style;
                match family_kind {
                    DiagramKind::Component | DiagramKind::Deployment => {
                        component_style = component_style_from_sequence_theme(&style);
                    }
                    DiagramKind::Activity => {
                        activity_style = activity_style_from_sequence_theme(&style);
                    }
                    DiagramKind::Timing => {
                        timing_style = timing_style_from_sequence_theme(&style);
                    }
                    _ => {}
                }
            }
            StatementKind::Pragma(_)
            | StatementKind::Include(_)
            | StatementKind::Define { .. }
            | StatementKind::Undef(_)
            | StatementKind::Scale(_)
            | StatementKind::LegendPos(_) => {}
            StatementKind::Unknown(line) => {
                return Err(Diagnostic::error(format!(
                    "[E_PARSE_UNKNOWN] unsupported syntax: `{}`",
                    line
                ))
                .with_span(stmt.span));
            }
            _ => {
                return Err(Diagnostic::error(format!(
                    "[E_FAMILY_{}_UNSUPPORTED_STMT] unsupported {} syntax",
                    family_kind_name(family_kind).to_uppercase(),
                    family_kind_name(family_kind)
                ))
                .with_span(stmt.span));
            }
        }
    }

    let family_style = match family_kind {
        DiagramKind::Component | DiagramKind::Deployment => {
            Some(FamilyStyle::Component(component_style))
        }
        DiagramKind::Activity => Some(FamilyStyle::Activity(activity_style)),
        DiagramKind::Timing => Some(FamilyStyle::Timing(timing_style)),
        _ => None,
    };

    Ok(FamilyDocument {
        kind: family_kind,
        nodes,
        relations,
        title,
        header,
        footer,
        caption,
        legend,
        orientation: FamilyOrientation::TopToBottom,
        style: SequenceStyle::default(),
        family_style,
        text_overflow_policy: TextOverflowPolicy::WrapAndGrow,
        warnings: ext_warnings,
        groups: Vec::new(),
        json_projections: Vec::new(),
        hide_options: std::collections::BTreeSet::new(),
        namespace_separator: None,
    })
}

fn component_node_kind(kind: ComponentNodeKind) -> FamilyNodeKind {
    match kind {
        ComponentNodeKind::Component => FamilyNodeKind::Component,
        ComponentNodeKind::Interface => FamilyNodeKind::Interface,
        ComponentNodeKind::Port => FamilyNodeKind::Port,
        ComponentNodeKind::Node => FamilyNodeKind::Node,
        ComponentNodeKind::Artifact => FamilyNodeKind::Artifact,
        ComponentNodeKind::Cloud => FamilyNodeKind::Cloud,
        ComponentNodeKind::Frame => FamilyNodeKind::Frame,
        ComponentNodeKind::Storage => FamilyNodeKind::Storage,
        ComponentNodeKind::Database => FamilyNodeKind::Database,
        ComponentNodeKind::Package => FamilyNodeKind::Package,
        ComponentNodeKind::Rectangle => FamilyNodeKind::Rectangle,
        ComponentNodeKind::Folder => FamilyNodeKind::Folder,
        ComponentNodeKind::File => FamilyNodeKind::File,
        ComponentNodeKind::Card => FamilyNodeKind::Card,
        ComponentNodeKind::Actor => FamilyNodeKind::Actor,
    }
}

fn activity_step_node_kind(kind: &ActivityStepKind) -> FamilyNodeKind {
    match kind {
        ActivityStepKind::Start => FamilyNodeKind::ActivityStart,
        ActivityStepKind::Stop | ActivityStepKind::End => FamilyNodeKind::ActivityStop,
        ActivityStepKind::Action => FamilyNodeKind::ActivityAction,
        ActivityStepKind::IfStart
        | ActivityStepKind::WhileStart
        | ActivityStepKind::RepeatWhile => FamilyNodeKind::ActivityDecision,
        ActivityStepKind::Else | ActivityStepKind::EndIf | ActivityStepKind::EndWhile => {
            FamilyNodeKind::ActivityMerge
        }
        ActivityStepKind::Fork | ActivityStepKind::ForkAgain => FamilyNodeKind::ActivityFork,
        ActivityStepKind::EndFork => FamilyNodeKind::ActivityForkEnd,
        ActivityStepKind::RepeatStart => FamilyNodeKind::ActivityMerge,
        ActivityStepKind::PartitionStart | ActivityStepKind::PartitionEnd => {
            FamilyNodeKind::ActivityPartition
        }
    }
}

fn timing_decl_node_kind(kind: TimingDeclKind) -> FamilyNodeKind {
    match kind {
        TimingDeclKind::Concise => FamilyNodeKind::TimingConcise,
        TimingDeclKind::Robust => FamilyNodeKind::TimingRobust,
        TimingDeclKind::Clock => FamilyNodeKind::TimingClock,
        TimingDeclKind::Binary => FamilyNodeKind::TimingBinary,
    }
}

fn family_kind_name(kind: DiagramKind) -> &'static str {
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

pub fn paginate(document: &SequenceDocument) -> Vec<SequencePage> {
    let mut pages = Vec::new();
    let mut page_events = Vec::new();
    let mut current_title = document.title.clone();

    for event in &document.events {
        if let SequenceEventKind::NewPage(next_title) = &event.kind {
            pages.push(page_from(document, &page_events, current_title.clone()));
            page_events.clear();
            current_title = cleaned_title(next_title).or_else(|| document.title.clone());
            continue;
        }
        page_events.push(event.clone());
    }

    pages.push(page_from(document, &page_events, current_title));
    pages
}

fn page_from(
    document: &SequenceDocument,
    events: &[SequenceEvent],
    title: Option<String>,
) -> SequencePage {
    SequencePage {
        participants: document.participants.clone(),
        events: events.to_vec(),
        title,
        header: document.header.clone(),
        footer: document.footer.clone(),
        caption: document.caption.clone(),
        legend: document.legend.clone(),
        skinparams: document.skinparams.clone(),
        style: document.style.clone(),
        footbox_visible: document.footbox_visible,
        scale: document.scale.clone(),
        legend_halign: document.legend_halign,
        legend_valign: document.legend_valign,
        warnings: document.warnings.clone(),
        hide_unlinked: document.hide_unlinked,
        hidden_participants: document.hidden_participants.clone(),
    }
}

fn cleaned_title(value: &Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(ToOwned::to_owned)
}

pub fn normalize_with_options(
    document: Document,
    _options: &NormalizeOptions,
) -> Result<SequenceDocument, Diagnostic> {
    if document.kind != DiagramKind::Sequence {
        return Err(unsupported_family_diagnostic(document.kind));
    }

    let mut participants: Vec<Participant> = Vec::new();
    let mut participant_ix: BTreeMap<String, usize> = BTreeMap::new();
    let mut events = Vec::new();

    let mut title = None;
    let mut header = None;
    let mut footer = None;
    let mut caption = None;
    let mut legend = None;
    let mut skinparams = Vec::new();
    let mut footbox_visible = true;
    let mut style = SequenceStyle::default();
    let mut scale: Option<ScaleSpec> = None;
    let mut legend_halign = LegendHAlign::default();
    let mut legend_valign = LegendVAlign::default();
    let mut warnings: Vec<Diagnostic> = Vec::new();
    let mut alive_by_id: BTreeMap<String, bool> = BTreeMap::new();
    let mut activation_stack: Vec<ActivationFrame> = Vec::new();
    let mut group_stack: Vec<GroupFrame> = Vec::new();
    let mut last_message: Option<(String, String)> = None;
    let mut ignore_newpage = false;
    let mut hide_unlinked = false;

    for stmt in document.statements {
        match stmt.kind {
            StatementKind::HideUnlinked => {
                hide_unlinked = true;
            }
            StatementKind::Participant(p) => {
                mark_group_content(&mut group_stack);
                let id = p.alias.unwrap_or_else(|| p.name.clone());
                let display = p.display.unwrap_or_else(|| p.name.clone());
                upsert_participant(
                    &mut participants,
                    &mut participant_ix,
                    id,
                    display,
                    map_role(p.role),
                    true,
                )
                .map_err(|e| Diagnostic::error(e).with_span(stmt.span))?;
            }
            StatementKind::Message(m) => {
                mark_group_content(&mut group_stack);
                let parsed_arrow = parse_message_arrow(&m.arrow).ok_or_else(|| {
                    Diagnostic::error(format!(
                        "[E_ARROW_INVALID] malformed sequence arrow syntax: `{}`",
                        m.arrow
                    ))
                    .with_span(stmt.span)
                })?;
                let directions = if parsed_arrow.bidirectional {
                    vec![
                        (m.from.clone(), m.to.clone()),
                        (m.to.clone(), m.from.clone()),
                    ]
                } else {
                    vec![(m.from.clone(), m.to.clone())]
                };

                for (from, to) in directions {
                    let from_virtual = virtual_endpoint(from.as_str(), true);
                    let to_virtual = virtual_endpoint(to.as_str(), false);
                    validate_virtual_endpoint_combination(
                        stmt.span,
                        &from,
                        &to,
                        from_virtual,
                        to_virtual,
                    )?;
                    validate_and_touch_message_lifecycle(
                        stmt.span,
                        &from,
                        &to,
                        &mut participants,
                        &mut participant_ix,
                        &mut alive_by_id,
                    )?;
                    if !is_virtual_endpoint(&from) && !is_virtual_endpoint(&to) {
                        last_message = Some((from.clone(), to.clone()));
                    }
                    events.push(SequenceEvent {
                        span: stmt.span,
                        kind: SequenceEventKind::Message {
                            from: from.clone(),
                            to: to.clone(),
                            arrow: parsed_arrow.render_arrow.clone(),
                            label: m.label.clone(),
                            from_virtual,
                            to_virtual,
                        },
                    });
                }
                apply_lifecycle_shortcuts(
                    stmt.span,
                    &m.from,
                    &m.to,
                    &parsed_arrow,
                    &mut participants,
                    &mut participant_ix,
                    &mut alive_by_id,
                    &mut activation_stack,
                    &mut events,
                )?;
            }
            StatementKind::Note(n) => {
                mark_group_content(&mut group_stack);
                events.push(SequenceEvent {
                    span: stmt.span,
                    kind: SequenceEventKind::Note {
                        position: n.position,
                        target: n.target,
                        text: n.text,
                    },
                });
            }
            StatementKind::Group(g) => {
                if g.kind == "end" {
                    let Some(open) = group_stack.pop() else {
                        return Err(Diagnostic::error(
                            "[E_GROUP_END_UNMATCHED] `end` without an open group block",
                        )
                        .with_span(stmt.span));
                    };
                    if let Some(expected) = g.label.as_deref() {
                        if expected != open.kind {
                            return Err(Diagnostic::error(format!(
                                "[E_GROUP_END_KIND] `end {}` does not match open `{}` block",
                                expected, open.kind
                            ))
                            .with_span(stmt.span));
                        }
                    }
                    if rejects_empty_group(open.kind.as_str()) && !open.branch_has_content {
                        return Err(Diagnostic::error(format!(
                            "[E_GROUP_EMPTY] `{}` block must not be empty",
                            open.kind
                        ))
                        .with_span(stmt.span));
                    }
                    events.push(SequenceEvent {
                        span: stmt.span,
                        kind: SequenceEventKind::GroupEnd,
                    });
                } else if g.kind == "else" {
                    let Some(top) = group_stack.last_mut() else {
                        return Err(Diagnostic::error(
                            "[E_GROUP_ELSE_UNMATCHED] `else` without an open group block",
                        )
                        .with_span(stmt.span));
                    };
                    if !allows_else(top.kind.as_str()) {
                        return Err(Diagnostic::error(format!(
                            "[E_GROUP_ELSE_KIND] `else` is not valid inside `{}`",
                            top.kind
                        ))
                        .with_span(stmt.span));
                    }
                    if rejects_empty_group(top.kind.as_str()) && !top.branch_has_content {
                        return Err(Diagnostic::error(format!(
                            "[E_GROUP_EMPTY_BRANCH] `{}` block contains an empty branch before `else`",
                            top.kind
                        ))
                        .with_span(stmt.span));
                    }
                    top.branch_has_content = false;
                    events.push(SequenceEvent {
                        span: stmt.span,
                        kind: SequenceEventKind::GroupStart {
                            kind: g.kind,
                            label: g.label,
                        },
                    });
                } else {
                    mark_group_content(&mut group_stack);
                    if g.kind != "ref" {
                        group_stack.push(GroupFrame {
                            kind: g.kind.clone(),
                            span: stmt.span,
                            branch_has_content: false,
                        });
                    }
                    events.push(SequenceEvent {
                        span: stmt.span,
                        kind: SequenceEventKind::GroupStart {
                            kind: g.kind,
                            label: g.label,
                        },
                    });
                }
            }
            StatementKind::Title(v) => title = Some(v),
            StatementKind::Header(v) => header = Some(v),
            StatementKind::Footer(v) => footer = Some(v),
            StatementKind::Caption(v) => caption = Some(v),
            StatementKind::Legend(v) => {
                // Parse packed "LEGEND_POS:<pos>\n<text>" format emitted by the parser
                // when a multiline legend block has positioning qualifiers.
                if let Some(rest) = v.strip_prefix("LEGEND_POS:") {
                    if let Some(newline_idx) = rest.find('\n') {
                        let pos = &rest[..newline_idx];
                        let text = &rest[newline_idx + 1..];
                        legend = Some(text.to_string());
                        let lower_pos = pos.to_ascii_lowercase();
                        for token in lower_pos.split_whitespace() {
                            match token {
                                "left" => legend_halign = LegendHAlign::Left,
                                "right" => legend_halign = LegendHAlign::Right,
                                "center" => legend_halign = LegendHAlign::Center,
                                "top" => legend_valign = LegendVAlign::Top,
                                "bottom" => legend_valign = LegendVAlign::Bottom,
                                _ => {}
                            }
                        }
                    } else {
                        // Just position, no text
                        let lower_pos = rest.to_ascii_lowercase();
                        for token in lower_pos.split_whitespace() {
                            match token {
                                "left" => legend_halign = LegendHAlign::Left,
                                "right" => legend_halign = LegendHAlign::Right,
                                "center" => legend_halign = LegendHAlign::Center,
                                "top" => legend_valign = LegendVAlign::Top,
                                "bottom" => legend_valign = LegendVAlign::Bottom,
                                _ => {}
                            }
                        }
                    }
                } else {
                    legend = Some(v);
                }
            }
            StatementKind::SkinParam { key, value } => {
                mark_group_content(&mut group_stack);
                skinparams.push((key.clone(), value.clone()));
                match classify_sequence_skinparam(&key, &value) {
                    SequenceSkinParamSupport::SupportedNoop => {}
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::FootboxVisible(visible),
                    ) => {
                        footbox_visible = visible;
                    }
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::ArrowColor(color),
                    ) => style.arrow_color = color,
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::LifelineBorderColor(color),
                    ) => style.lifeline_border_color = color,
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::ParticipantBackgroundColor(color),
                    ) => style.participant_background_color = color,
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::ParticipantBorderColor(color),
                    ) => style.participant_border_color = color,
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::NoteBackgroundColor(color),
                    ) => style.note_background_color = color,
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::NoteBorderColor(color),
                    ) => style.note_border_color = color,
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::GroupBackgroundColor(color),
                    ) => style.group_background_color = color,
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::GroupBorderColor(color),
                    ) => style.group_border_color = color,
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::RoundCorner(n),
                    ) => style.round_corner = n,
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::Shadowing(enabled),
                    ) => style.shadowing = enabled,
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::DefaultFontName(name),
                    ) => style.default_font_name = Some(name),
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::DefaultFontSize(sz),
                    ) => style.default_font_size = Some(sz),
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::BackgroundColor(color),
                    ) => style.background_color = Some(color),
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::DefaultTextAlignment(align),
                    ) => style.text_alignment = align,
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::ParticipantPadding(n),
                    ) => style.participant_padding = Some(n),
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::BoxPadding(n),
                    ) => style.box_padding = Some(n),
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::MessageAlign(align),
                    ) => style.message_align = align,
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::ResponseMessageBelowArrow(enabled),
                    ) => style.response_message_below_arrow = enabled,
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::LifelineThickness(n),
                    ) => style.lifeline_thickness = Some(n),
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::MessageLineColor(color),
                    ) => style.message_line_color = Some(color),
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::ReferenceBackgroundColor(color),
                    ) => style.reference_background_color = Some(color),
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::ReferenceBorderColor(color),
                    ) => style.reference_border_color = Some(color),
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::GroupHeaderFontColor(color),
                    ) => style.group_header_font_color = Some(color),
                    SequenceSkinParamSupport::SupportedWithValue(
                        SequenceSkinParamValue::GroupHeaderFontStyle(fs),
                    ) => style.group_header_font_style = fs,
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
            StatementKind::Theme(name) => {
                mark_group_content(&mut group_stack);
                let preset = resolve_sequence_theme_preset(&name)
                    .map_err(|msg| Diagnostic::error(msg).with_span(stmt.span))?;
                style = preset.style;
            }
            StatementKind::Pragma(value) => {
                mark_group_content(&mut group_stack);
                let trimmed = value.trim();
                let lower = trimmed.to_ascii_lowercase();
                if lower.starts_with("teoz ") || lower == "teoz" {
                    // Accept teoz pragma as a deterministic no-op compatibility hint.
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
            StatementKind::Footbox(v) => {
                mark_group_content(&mut group_stack);
                footbox_visible = v
            }
            StatementKind::Delay(v) => {
                mark_group_content(&mut group_stack);
                events.push(SequenceEvent {
                    span: stmt.span,
                    kind: SequenceEventKind::Delay(v),
                })
            }
            StatementKind::Divider(v) => {
                mark_group_content(&mut group_stack);
                events.push(SequenceEvent {
                    span: stmt.span,
                    kind: SequenceEventKind::Divider(v),
                })
            }
            StatementKind::Separator(v) => {
                mark_group_content(&mut group_stack);
                events.push(SequenceEvent {
                    span: stmt.span,
                    kind: SequenceEventKind::Separator(v),
                })
            }
            StatementKind::Spacer => {
                mark_group_content(&mut group_stack);
                events.push(SequenceEvent {
                    span: stmt.span,
                    kind: SequenceEventKind::Spacer,
                })
            }
            StatementKind::NewPage(v) => {
                mark_group_content(&mut group_stack);
                if !ignore_newpage {
                    events.push(SequenceEvent {
                        span: stmt.span,
                        kind: SequenceEventKind::NewPage(v),
                    });
                }
            }
            StatementKind::IgnoreNewPage => {
                mark_group_content(&mut group_stack);
                ignore_newpage = true;
            }
            StatementKind::Autonumber(v) => {
                mark_group_content(&mut group_stack);
                if let Some(raw) = v.as_deref() {
                    validate_autonumber_raw(raw).map_err(|reason| {
                        Diagnostic::error(format!("[E_AUTONUMBER_FORMAT_UNSUPPORTED] {reason}"))
                            .with_span(stmt.span)
                    })?;
                }
                events.push(SequenceEvent {
                    span: stmt.span,
                    kind: SequenceEventKind::Autonumber(
                        v.as_deref().and_then(canonicalize_autonumber_raw),
                    ),
                })
            }
            StatementKind::Activate(id) => {
                mark_group_content(&mut group_stack);
                ensure_implicit(&mut participants, &mut participant_ix, &id);
                if !is_alive(&alive_by_id, &id) {
                    return Err(Diagnostic::error(format!(
                        "[E_LIFECYCLE_ACTIVATE_DESTROYED] cannot activate destroyed participant `{}`",
                        id
                    ))
                    .with_span(stmt.span));
                }
                alive_by_id.insert(id.clone(), true);
                let caller = match &last_message {
                    Some((from, to)) if to == &id => Some(from.clone()),
                    _ => activation_stack.last().map(|f| f.participant.clone()),
                };
                activation_stack.push(ActivationFrame {
                    participant: id.clone(),
                    caller,
                });
                events.push(SequenceEvent {
                    span: stmt.span,
                    kind: SequenceEventKind::Activate(id),
                });
            }
            StatementKind::Deactivate(id) => {
                mark_group_content(&mut group_stack);
                ensure_implicit(&mut participants, &mut participant_ix, &id);
                if !is_alive(&alive_by_id, &id) {
                    return Err(Diagnostic::error(format!(
                        "[E_LIFECYCLE_DEACTIVATE_DESTROYED] cannot deactivate destroyed participant `{}`",
                        id
                    ))
                    .with_span(stmt.span));
                }
                alive_by_id.insert(id.clone(), true);
                match activation_stack.last() {
                    Some(frame) if frame.participant == id => {
                        activation_stack.pop();
                    }
                    Some(frame) => {
                        return Err(Diagnostic::error(format!(
                            "[E_LIFECYCLE_DEACTIVATE_ORDER] deactivate `{}` does not match current activation `{}`",
                            id, frame.participant
                        ))
                        .with_span(stmt.span));
                    }
                    None => {
                        return Err(Diagnostic::error(format!(
                            "[E_LIFECYCLE_DEACTIVATE_EMPTY] cannot deactivate `{}` without an active activation",
                            id
                        ))
                        .with_span(stmt.span));
                    }
                }
                events.push(SequenceEvent {
                    span: stmt.span,
                    kind: SequenceEventKind::Deactivate(id),
                });
            }
            StatementKind::Destroy(id) => {
                mark_group_content(&mut group_stack);
                ensure_implicit(&mut participants, &mut participant_ix, &id);
                if !is_alive(&alive_by_id, &id) {
                    return Err(Diagnostic::error(format!(
                        "[E_LIFECYCLE_DESTROY_TWICE] participant `{}` is already destroyed",
                        id
                    ))
                    .with_span(stmt.span));
                }
                if activation_stack.iter().any(|f| f.participant == id) {
                    return Err(Diagnostic::error(format!(
                        "[E_LIFECYCLE_DESTROY_ACTIVE] cannot destroy active participant `{}`",
                        id
                    ))
                    .with_span(stmt.span));
                }
                alive_by_id.insert(id.clone(), false);
                events.push(SequenceEvent {
                    span: stmt.span,
                    kind: SequenceEventKind::Destroy(id),
                });
            }
            StatementKind::Create(id) => {
                mark_group_content(&mut group_stack);
                ensure_implicit(&mut participants, &mut participant_ix, &id);
                if alive_by_id.get(&id).copied() == Some(true) {
                    return Err(Diagnostic::error(format!(
                        "[E_LIFECYCLE_CREATE_EXISTING] participant `{}` already exists; destroy before create",
                        id
                    ))
                    .with_span(stmt.span));
                }
                alive_by_id.insert(id.clone(), true);
                events.push(SequenceEvent {
                    span: stmt.span,
                    kind: SequenceEventKind::Create(id),
                });
            }
            StatementKind::Return(v) => {
                mark_group_content(&mut group_stack);
                events.push(SequenceEvent {
                    span: stmt.span,
                    kind: infer_return_event(stmt.span, v, &mut activation_stack, &last_message)?,
                })
            }
            StatementKind::Include(_) | StatementKind::Define { .. } | StatementKind::Undef(_) => {
                // Preprocessor directives should be expanded before normalization.
            }
            StatementKind::RawBlockContent(_) => {
                // Raw block content is only meaningful in dedicated raw-body families
                // (json/yaml/nwdiag/archimate); ignore in sequence normalization.
            }
            StatementKind::Scale(body) => {
                mark_group_content(&mut group_stack);
                scale = parse_scale_spec(&body).or(scale);
            }
            StatementKind::LegendPos(pos) => {
                mark_group_content(&mut group_stack);
                let lower = pos.to_ascii_lowercase();
                for token in lower.split_whitespace() {
                    match token {
                        "left" => legend_halign = LegendHAlign::Left,
                        "right" => legend_halign = LegendHAlign::Right,
                        "center" => legend_halign = LegendHAlign::Center,
                        "top" => legend_valign = LegendVAlign::Top,
                        "bottom" => legend_valign = LegendVAlign::Bottom,
                        _ => {}
                    }
                }
            }
            StatementKind::ClassDecl(_)
            | StatementKind::ObjectDecl(_)
            | StatementKind::UseCaseDecl(_)
            | StatementKind::FamilyRelation(_)
            | StatementKind::StateDecl(_)
            | StatementKind::StateTransition(_)
            | StatementKind::StateInternalAction(_)
            | StatementKind::StateRegionDivider
            | StatementKind::StateHistory { .. }
            | StatementKind::GanttTaskDecl { .. }
            | StatementKind::GanttMilestoneDecl { .. }
            | StatementKind::GanttConstraint { .. }
            | StatementKind::ChronologyHappensOn { .. }
            | StatementKind::ComponentDecl { .. }
            | StatementKind::ActivityStep(_)
            | StatementKind::TimingDecl { .. }
            | StatementKind::TimingEvent { .. }
            | StatementKind::RawBody(_)
            | StatementKind::ClassGroup { .. }
            | StatementKind::JsonProjection { .. }
            | StatementKind::YamlProjection { .. }
            | StatementKind::SaltGridRow { .. } => {
                return Err(Diagnostic::error(
                    "[E_FAMILY_MIXED] mixed diagram families are not supported in one document",
                )
                .with_span(stmt.span));
            }
            // Class-family-only options: silently ignored in sequence context
            StatementKind::SetOption { .. } | StatementKind::HideOption(_) => {}
            StatementKind::Unknown(line) => {
                if line.trim() == "---" {
                    continue;
                }
                return Err(Diagnostic::error(format!(
                    "[E_PARSE_UNKNOWN] unsupported syntax: `{}`",
                    line
                ))
                .with_span(stmt.span));
            }
        }
    }

    if let Some(open) = group_stack.pop() {
        return Err(Diagnostic::error(format!(
            "[E_GROUP_UNCLOSED] missing `end` for open `{}` block",
            open.kind
        ))
        .with_span(open.span));
    }

    // Apply `hide unlinked` filter: collect all participant IDs that appear in
    // events, then drop explicit participant declarations that are never referenced.
    if hide_unlinked {
        let mut referenced: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        for ev in &events {
            match &ev.kind {
                SequenceEventKind::Message { from, to, .. } => {
                    // Only count real (non-virtual) endpoints.
                    if !from.starts_with('[') && from != "]" {
                        referenced.insert(from.clone());
                    }
                    if !to.starts_with('[') && to != "]" {
                        referenced.insert(to.clone());
                    }
                }
                SequenceEventKind::Note {
                    target: Some(t), ..
                } => {
                    // target may be comma-separated for `note over A,B`
                    for part in t.split(',') {
                        referenced.insert(part.trim().to_string());
                    }
                }
                SequenceEventKind::Note { target: None, .. } => {}
                SequenceEventKind::Activate(id)
                | SequenceEventKind::Deactivate(id)
                | SequenceEventKind::Destroy(id)
                | SequenceEventKind::Create(id) => {
                    referenced.insert(id.clone());
                }
                SequenceEventKind::Return { from, to, .. } => {
                    if let Some(f) = from {
                        referenced.insert(f.clone());
                    }
                    if let Some(t) = to {
                        referenced.insert(t.clone());
                    }
                }
                _ => {}
            }
        }
        let before_len = participants.len();
        participants.retain(|p| !p.explicit || referenced.contains(&p.id));
        let hidden_count = before_len - participants.len();
        if hidden_count > 0 {
            warnings.push(Diagnostic::warning(format!(
                "[I_HIDE_UNLINKED_FILTERED] hide unlinked: removed {} unreferenced participant(s)",
                hidden_count
            )));
            // Rebuild the participant index map.
            participant_ix.clear();
            for (idx, p) in participants.iter().enumerate() {
                participant_ix.insert(p.id.clone(), idx);
            }
        }
    }

    warnings.sort_by(|a, b| {
        let sa = a.span.map(|s| s.start).unwrap_or_default();
        let sb = b.span.map(|s| s.start).unwrap_or_default();
        (a.message.as_str(), sa).cmp(&(b.message.as_str(), sb))
    });

    Ok(SequenceDocument {
        participants,
        events,
        title,
        header,
        footer,
        caption,
        legend,
        skinparams,
        style,
        footbox_visible,
        scale,
        legend_halign,
        legend_valign,
        warnings,
        hide_unlinked,
        hidden_participants: Vec::new(),
    })
}

/// Strip the LEGEND_POS prefix from a packed legend value, returning just the text.
fn strip_legend_pos_prefix(v: &str) -> String {
    if let Some(rest) = v.strip_prefix("LEGEND_POS:") {
        if let Some(nl) = rest.find('\n') {
            return rest[nl + 1..].to_string();
        }
        return String::new();
    }
    v.to_string()
}

/// Parse a scale body (everything after "scale ").
/// Supports:
///   "1.5"          → Factor(1.5)
///   "800*600"      → Fixed { width: 800, height: 600 }
///   "max 800"      → Max(800)
fn parse_scale_spec(body: &str) -> Option<ScaleSpec> {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return None;
    }
    let lower = trimmed.to_ascii_lowercase();
    if let Some(rest) = lower.strip_prefix("max ") {
        let n: u32 = rest.trim().parse().ok()?;
        return Some(ScaleSpec::Max(n));
    }
    if let Some(idx) = trimmed.find('*') {
        let w: u32 = trimmed[..idx].trim().parse().ok()?;
        let h: u32 = trimmed[idx + 1..].trim().parse().ok()?;
        return Some(ScaleSpec::Fixed {
            width: w,
            height: h,
        });
    }
    let f: f64 = trimmed.parse().ok()?;
    if f > 0.0 {
        Some(ScaleSpec::Factor(f))
    } else {
        None
    }
}

fn unsupported_family_diagnostic(kind: DiagramKind) -> Diagnostic {
    let (code, family) = match kind {
        DiagramKind::Component => ("E_FAMILY_COMPONENT_UNSUPPORTED", "component"),
        DiagramKind::Deployment => ("E_FAMILY_DEPLOYMENT_UNSUPPORTED", "deployment"),
        DiagramKind::State => ("E_FAMILY_STATE_UNSUPPORTED", "state"),
        DiagramKind::Activity => ("E_FAMILY_ACTIVITY_UNSUPPORTED", "activity"),
        DiagramKind::Timing => ("E_FAMILY_TIMING_UNSUPPORTED", "timing"),
        DiagramKind::Gantt => ("E_FAMILY_GANTT_UNSUPPORTED", "gantt"),
        DiagramKind::Chronology => ("E_FAMILY_CHRONOLOGY_UNSUPPORTED", "chronology"),
        _ => ("E_FAMILY_UNSUPPORTED", "unknown"),
    };

    Diagnostic::error_code(
        code,
        format!(
            "diagram family `{family}` is not implemented yet; sequence is currently supported"
        ),
    )
}

fn is_alive(alive_by_id: &BTreeMap<String, bool>, id: &str) -> bool {
    alive_by_id.get(id).copied().unwrap_or(true)
}

#[derive(Debug, Clone)]
struct ActivationFrame {
    participant: String,
    caller: Option<String>,
}

#[derive(Debug, Clone)]
struct GroupFrame {
    kind: String,
    span: crate::source::Span,
    branch_has_content: bool,
}

fn mark_group_content(group_stack: &mut [GroupFrame]) {
    for frame in group_stack {
        frame.branch_has_content = true;
    }
}

fn allows_else(kind: &str) -> bool {
    matches!(kind, "alt" | "par" | "critical")
}

fn rejects_empty_group(kind: &str) -> bool {
    matches!(kind, "alt" | "par" | "critical")
}

fn infer_return_event(
    span: crate::source::Span,
    label: Option<String>,
    activation_stack: &mut Vec<ActivationFrame>,
    last_message: &Option<(String, String)>,
) -> Result<SequenceEventKind, Diagnostic> {
    if activation_stack.is_empty() {
        if let Some((from, to)) = last_message {
            return Ok(SequenceEventKind::Return {
                label,
                from: Some(to.clone()),
                to: Some(from.clone()),
            });
        }
    }
    let Some(frame) = activation_stack.pop() else {
        return Err(Diagnostic::error(
            "[E_RETURN_INFER_EMPTY] cannot infer `return` sender/target without an active activation",
        )
        .with_span(span));
    };

    let Some(caller) = frame.caller else {
        return Err(Diagnostic::error(format!(
            "[E_RETURN_INFER_CALLER] cannot infer `return` target for `{}`; use an explicit return message instead",
            frame.participant
        ))
        .with_span(span));
    };

    Ok(SequenceEventKind::Return {
        label,
        from: Some(frame.participant),
        to: Some(caller),
    })
}

fn ensure_implicit(
    participants: &mut Vec<Participant>,
    index: &mut BTreeMap<String, usize>,
    id: &str,
) {
    if index.contains_key(id) {
        return;
    }
    let pos = participants.len();
    participants.push(Participant {
        id: id.to_string(),
        display: id.to_string(),
        role: ParticipantRole::Participant,
        explicit: false,
    });
    index.insert(id.to_string(), pos);
}

fn upsert_participant(
    participants: &mut Vec<Participant>,
    index: &mut BTreeMap<String, usize>,
    id: String,
    display: String,
    role: ParticipantRole,
    explicit: bool,
) -> Result<(), String> {
    if let Some(ix) = index.get(&id).copied() {
        if explicit && participants[ix].explicit {
            return Err(format!(
                "[E_PARTICIPANT_DUPLICATE] duplicate participant id/alias `{}`",
                id
            ));
        }
        participants[ix].display = display;
        participants[ix].role = role;
        participants[ix].explicit = explicit;
        return Ok(());
    }

    let pos = participants.len();
    participants.push(Participant {
        id: id.clone(),
        display,
        role,
        explicit,
    });
    index.insert(id, pos);
    Ok(())
}

fn map_role(role: AstRole) -> ParticipantRole {
    match role {
        AstRole::Participant => ParticipantRole::Participant,
        AstRole::Actor => ParticipantRole::Actor,
        AstRole::Boundary => ParticipantRole::Boundary,
        AstRole::Control => ParticipantRole::Control,
        AstRole::Entity => ParticipantRole::Entity,
        AstRole::Database => ParticipantRole::Database,
        AstRole::Collections => ParticipantRole::Collections,
        AstRole::Queue => ParticipantRole::Queue,
    }
}

fn is_virtual_endpoint(id: &str) -> bool {
    matches!(id, "[*]" | "[" | "]" | "[o" | "o]" | "[x" | "x]")
}

fn virtual_endpoint(id: &str, is_from: bool) -> Option<VirtualEndpoint> {
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

fn validate_virtual_endpoint_combination(
    span: crate::source::Span,
    from: &str,
    to: &str,
    from_virtual: Option<VirtualEndpoint>,
    to_virtual: Option<VirtualEndpoint>,
) -> Result<(), Diagnostic> {
    if from_virtual.is_some() && to_virtual.is_some() {
        return Err(Diagnostic::error(format!(
            "[E_ENDPOINT_COMBINATION] virtual endpoint messages must include at least one concrete participant: `{}` -> `{}`",
            from, to
        ))
        .with_span(span));
    }
    Ok(())
}

#[derive(Debug, Clone)]
struct ParsedMessageArrow {
    render_arrow: String,
    bidirectional: bool,
    left_modifier: Option<String>,
    right_modifier: Option<String>,
}

fn parse_message_arrow(raw: &str) -> Option<ParsedMessageArrow> {
    let (base, left_modifier, right_modifier) = decode_arrow_modifiers(raw)?;
    let canonical_base = base.replace(['/', '\\'], "");
    if canonical_base.is_empty()
        || !canonical_base
            .chars()
            .all(|c| matches!(c, '-' | '<' | '>' | 'o' | 'x'))
    {
        return None;
    }
    let stripped_left = canonical_base
        .strip_prefix('o')
        .or_else(|| canonical_base.strip_prefix('x'))
        .unwrap_or(&canonical_base);
    let stripped = stripped_left
        .strip_suffix('o')
        .or_else(|| stripped_left.strip_suffix('x'))
        .unwrap_or(stripped_left);
    let bidirectional = matches!(stripped, "<->" | "<-->" | "<<->>" | "<<-->>");
    let render_arrow = if bidirectional {
        if stripped.contains("--") {
            "-->".to_string()
        } else {
            "->".to_string()
        }
    } else {
        canonical_base
    };
    Some(ParsedMessageArrow {
        render_arrow,
        bidirectional,
        left_modifier,
        right_modifier,
    })
}

fn decode_arrow_modifiers(raw: &str) -> Option<(String, Option<String>, Option<String>)> {
    let mut rest = raw;
    let mut left_modifier = None;
    let mut right_modifier = None;
    while let Some(ix) = rest.find("@L").or_else(|| rest.find("@R")) {
        let side = &rest[ix..ix + 2];
        let token = rest.get(ix + 2..ix + 4)?;
        if !matches!(token, "++" | "--" | "**" | "!!") {
            return None;
        }
        if side == "@L" {
            left_modifier = Some(token.to_string());
        } else {
            right_modifier = Some(token.to_string());
        }
        rest = &rest[..ix];
    }
    Some((rest.to_string(), left_modifier, right_modifier))
}

fn validate_and_touch_message_lifecycle(
    span: crate::source::Span,
    from: &str,
    to: &str,
    participants: &mut Vec<Participant>,
    participant_ix: &mut BTreeMap<String, usize>,
    alive_by_id: &mut BTreeMap<String, bool>,
) -> Result<(), Diagnostic> {
    let from_virtual = is_virtual_endpoint(from);
    let to_virtual = is_virtual_endpoint(to);
    if !from_virtual {
        ensure_implicit(participants, participant_ix, from);
    }
    if !to_virtual {
        ensure_implicit(participants, participant_ix, to);
    }
    if !from_virtual && !is_alive(alive_by_id, from) {
        return Err(Diagnostic::error(format!(
            "[E_LIFECYCLE_DESTROYED_SENDER] message sender `{}` is destroyed",
            from
        ))
        .with_span(span));
    }
    if !to_virtual && !is_alive(alive_by_id, to) {
        return Err(Diagnostic::error(format!(
            "[E_LIFECYCLE_DESTROYED_TARGET] message target `{}` is destroyed (recreate it before sending messages to it)",
            to
        ))
        .with_span(span));
    }
    if !from_virtual {
        alive_by_id.insert(from.to_string(), true);
    }
    if !to_virtual {
        alive_by_id.insert(to.to_string(), true);
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn apply_lifecycle_shortcuts(
    span: crate::source::Span,
    from: &str,
    to: &str,
    parsed_arrow: &ParsedMessageArrow,
    participants: &mut Vec<Participant>,
    participant_ix: &mut BTreeMap<String, usize>,
    alive_by_id: &mut BTreeMap<String, bool>,
    activation_stack: &mut Vec<ActivationFrame>,
    events: &mut Vec<SequenceEvent>,
) -> Result<(), Diagnostic> {
    if let Some(token) = &parsed_arrow.left_modifier {
        let caller = shortcut_caller(from, to);
        apply_one_lifecycle_shortcut(
            span,
            from,
            token,
            caller,
            participants,
            participant_ix,
            alive_by_id,
            activation_stack,
            events,
        )?;
    }
    if let Some(token) = &parsed_arrow.right_modifier {
        let id = if token == "--" { from } else { to };
        let caller = shortcut_caller(id, if id == from { to } else { from });
        apply_one_lifecycle_shortcut(
            span,
            id,
            token,
            caller,
            participants,
            participant_ix,
            alive_by_id,
            activation_stack,
            events,
        )?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn apply_one_lifecycle_shortcut(
    span: crate::source::Span,
    id: &str,
    token: &str,
    caller: Option<String>,
    participants: &mut Vec<Participant>,
    participant_ix: &mut BTreeMap<String, usize>,
    alive_by_id: &mut BTreeMap<String, bool>,
    activation_stack: &mut Vec<ActivationFrame>,
    events: &mut Vec<SequenceEvent>,
) -> Result<(), Diagnostic> {
    if is_virtual_endpoint(id) {
        return Err(Diagnostic::error(format!(
            "[E_LIFECYCLE_SHORTCUT_VIRTUAL] cannot apply lifecycle shortcut `{}` to virtual endpoint",
            token
        ))
        .with_span(span));
    }
    ensure_implicit(participants, participant_ix, id);
    match token {
        "++" => {
            if !is_alive(alive_by_id, id) {
                return Err(Diagnostic::error(format!(
                    "[E_LIFECYCLE_ACTIVATE_DESTROYED] cannot activate destroyed participant `{}`",
                    id
                ))
                .with_span(span));
            }
            alive_by_id.insert(id.to_string(), true);
            activation_stack.push(ActivationFrame {
                participant: id.to_string(),
                caller,
            });
            events.push(SequenceEvent {
                span,
                kind: SequenceEventKind::Activate(id.to_string()),
            });
        }
        "--" => {
            if !is_alive(alive_by_id, id) {
                return Err(Diagnostic::error(format!(
                    "[E_LIFECYCLE_DEACTIVATE_DESTROYED] cannot deactivate destroyed participant `{}`",
                    id
                ))
                .with_span(span));
            }
            alive_by_id.insert(id.to_string(), true);
            match activation_stack.last() {
                Some(frame) if frame.participant == id => {
                    activation_stack.pop();
                }
                Some(frame) => {
                    return Err(Diagnostic::error(format!(
                        "[E_LIFECYCLE_DEACTIVATE_ORDER] deactivate `{}` does not match current activation `{}`",
                        id, frame.participant
                    ))
                    .with_span(span));
                }
                None => {
                    return Err(Diagnostic::error(format!(
                        "[E_LIFECYCLE_DEACTIVATE_EMPTY] cannot deactivate `{}` without an active activation",
                        id
                    ))
                    .with_span(span));
                }
            }
            events.push(SequenceEvent {
                span,
                kind: SequenceEventKind::Deactivate(id.to_string()),
            });
        }
        "**" => {
            alive_by_id.insert(id.to_string(), true);
            events.push(SequenceEvent {
                span,
                kind: SequenceEventKind::Create(id.to_string()),
            });
        }
        "!!" => {
            if !is_alive(alive_by_id, id) {
                return Err(Diagnostic::error(format!(
                    "[E_LIFECYCLE_DESTROY_TWICE] participant `{}` is already destroyed",
                    id
                ))
                .with_span(span));
            }
            if activation_stack.iter().any(|f| f.participant == id) {
                return Err(Diagnostic::error(format!(
                    "[E_LIFECYCLE_DESTROY_ACTIVE] cannot destroy active participant `{}`",
                    id
                ))
                .with_span(span));
            }
            alive_by_id.insert(id.to_string(), false);
            events.push(SequenceEvent {
                span,
                kind: SequenceEventKind::Destroy(id.to_string()),
            });
        }
        _ => {
            return Err(Diagnostic::error(format!(
                "[E_LIFECYCLE_SHORTCUT_INVALID] unknown lifecycle shortcut `{}`",
                token
            ))
            .with_span(span));
        }
    }
    Ok(())
}

fn shortcut_caller(active: &str, other: &str) -> Option<String> {
    if is_virtual_endpoint(active) || is_virtual_endpoint(other) {
        None
    } else {
        Some(other.to_string())
    }
}

fn canonicalize_autonumber_raw(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut out = String::with_capacity(trimmed.len());
    let mut in_quotes = false;
    let mut prev_space = false;
    for ch in trimmed.chars() {
        if ch == '"' {
            in_quotes = !in_quotes;
            prev_space = false;
            out.push(ch);
            continue;
        }
        if ch.is_whitespace() && !in_quotes {
            if !prev_space {
                out.push(' ');
            }
            prev_space = true;
            continue;
        }
        prev_space = false;
        out.push(ch);
    }
    Some(out.trim().to_string())
}

fn validate_autonumber_raw(raw: &str) -> Result<(), String> {
    let trimmed = raw.trim();
    if trimmed.is_empty()
        || trimmed.eq_ignore_ascii_case("stop")
        || trimmed.eq_ignore_ascii_case("off")
        || trimmed.eq_ignore_ascii_case("resume")
    {
        return Ok(());
    }

    let (format, body) = if trimmed.contains('"') {
        let Some((format, before)) = trailing_quoted_format(trimmed) else {
            return Err("malformed quoted autonumber format; quote-delimited format must be the final token".to_string());
        };
        (Some(format), before.trim_end())
    } else {
        (None, trimmed)
    };

    let mut tokens: Vec<&str> = body.split_whitespace().collect();
    let mut resume = false;
    if matches!(tokens.first(), Some(token) if token.eq_ignore_ascii_case("resume")) {
        resume = true;
        tokens.remove(0);
    }

    let mut idx = 0usize;
    let expected_numbers = if resume { 1 } else { 2 };
    while idx < tokens.len() && idx < expected_numbers && tokens[idx].parse::<u64>().is_ok() {
        idx += 1;
    }

    let unquoted_format = if idx < tokens.len() {
        let fmt = tokens[idx];
        idx += 1;
        Some(fmt)
    } else {
        None
    };

    if idx < tokens.len() {
        return Err(
            "unsupported autonumber syntax; expected `autonumber [start] [increment] [format]` or `autonumber resume [increment] [format]`".to_string(),
        );
    }

    if let Some(fmt) = format.or(unquoted_format.map(str::to_string)) {
        validate_autonumber_format(&fmt)?;
    }

    Ok(())
}

fn trailing_quoted_format(raw: &str) -> Option<(String, &str)> {
    let trimmed = raw.trim_end();
    let end = trimmed.strip_suffix('"')?;
    let start = end.rfind('"')?;
    let format = end[start + 1..].to_string();
    let prefix = &end[..start];
    Some((format, prefix))
}

fn validate_autonumber_format(format: &str) -> Result<(), String> {
    let fmt = format.trim();
    if fmt.is_empty() {
        return Err("autonumber format must not be empty".to_string());
    }
    if fmt.contains('<') || fmt.contains('>') {
        return Err(
            "autonumber format does not support HTML tags in this deterministic subset".to_string(),
        );
    }
    if fmt.contains('"') {
        return Err("autonumber format must not contain an embedded quote".to_string());
    }
    Ok(())
}

/// Extract `<<stereotype>>` from an alias string like `"myAlias <<person>>"`.
/// Returns `(clean_alias, Option<FamilyNodeKind>)` where `clean_alias` has the
/// stereotype stripped.  When the stereotype is not a recognised C4 marker the
/// kind is `None` and the caller keeps `FamilyNodeKind::Object`.
pub(crate) fn extract_c4_stereotype(
    alias: Option<String>,
) -> (Option<String>, Option<FamilyNodeKind>) {
    use crate::model::FamilyNodeKind;
    let Some(raw) = alias else {
        return (None, None);
    };
    // Find `<<...>>`
    if let Some(start) = raw.find("<<") {
        if let Some(end) = raw[start..].find(">>") {
            let stereotype = raw[start + 2..start + end].trim().to_ascii_lowercase();
            let clean_alias = raw[..start].trim().to_string();
            let kind = match stereotype.as_str() {
                "person" => Some(FamilyNodeKind::C4Person),
                "external-person" => Some(FamilyNodeKind::C4PersonExt),
                "system" => Some(FamilyNodeKind::C4System),
                "external-system" => Some(FamilyNodeKind::C4SystemExt),
                "system-db" | "systemdb" => Some(FamilyNodeKind::C4SystemDb),
                "system-queue" | "systemqueue" => Some(FamilyNodeKind::C4SystemQueue),
                "container" => Some(FamilyNodeKind::C4Container),
                "external-container" => Some(FamilyNodeKind::C4ContainerExt),
                "container-db" | "containerdb" => Some(FamilyNodeKind::C4ContainerDb),
                "container-queue" | "containerqueue" => Some(FamilyNodeKind::C4ContainerQueue),
                "c4-component" | "component" => Some(FamilyNodeKind::C4Component),
                "external-c4-component" | "external-component" => {
                    Some(FamilyNodeKind::C4ComponentExt)
                }
                "component-db" | "componentdb" => Some(FamilyNodeKind::C4ComponentDb),
                "component-queue" | "componentqueue" => Some(FamilyNodeKind::C4ComponentQueue),
                "boundary" | "enterprise-boundary" | "system-boundary" | "container-boundary" => {
                    Some(FamilyNodeKind::C4Boundary)
                }
                _ => None,
            };
            let clean = if clean_alias.is_empty() {
                None
            } else {
                Some(clean_alias)
            };
            return (clean, kind);
        }
    }
    (Some(raw), None)
}
