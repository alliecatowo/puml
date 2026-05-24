use super::text::TextOutputMode;
use super::text_output::{
    finish_text, optional_label, push_meta, text_value, tree_branch, tree_leaf,
};
use crate::model::{
    ArchimateDocument, ChartDocument, DitaaDocument, EbnfDocument, EbnfToken, JsonDocument,
    MathDocument, NwdiagDocument, RegexDocument, RegexToken, RepeatKind, SdlDocument, YamlDocument,
};

pub(super) fn render_json_text(doc: &JsonDocument, mode: TextOutputMode) -> String {
    let mut lines = Vec::new();
    push_meta(&mut lines, "title", doc.title.as_deref(), mode);
    lines.push("json".to_string());
    for (idx, node) in doc.nodes.iter().enumerate() {
        let branch = if idx + 1 == doc.nodes.len() {
            tree_leaf(mode)
        } else {
            tree_branch(mode)
        };
        lines.push(format!(
            "{}{}{}",
            "  ".repeat(node.depth),
            branch,
            text_value(&node.label, mode)
        ));
    }
    finish_text(lines)
}

pub(super) fn render_yaml_text(doc: &YamlDocument, mode: TextOutputMode) -> String {
    let mut lines = Vec::new();
    push_meta(&mut lines, "title", doc.title.as_deref(), mode);
    lines.push("yaml".to_string());
    for (idx, node) in doc.nodes.iter().enumerate() {
        let branch = if idx + 1 == doc.nodes.len() {
            tree_leaf(mode)
        } else {
            tree_branch(mode)
        };
        lines.push(format!(
            "{}{}{}",
            "  ".repeat(node.depth),
            branch,
            text_value(&node.label, mode)
        ));
    }
    finish_text(lines)
}

pub(super) fn render_nwdiag_text(doc: &NwdiagDocument, mode: TextOutputMode) -> String {
    let mut lines = Vec::new();
    push_meta(&mut lines, "title", doc.title.as_deref(), mode);
    lines.push("nwdiag".to_string());
    for network in &doc.networks {
        lines.push(format!(
            "network {}{}",
            text_value(&network.name, mode),
            optional_label(network.address.as_deref(), mode)
        ));
        for node in &network.nodes {
            let address = node
                .address
                .as_deref()
                .map(|v| format!(" address={}", text_value(v, mode)))
                .unwrap_or_default();
            lines.push(format!(
                "  node {}{}",
                text_value(node.label.as_deref().unwrap_or(&node.name), mode),
                address
            ));
        }
    }
    if !doc.groups.is_empty() {
        lines.push("groups".to_string());
        for group in &doc.groups {
            lines.push(format!(
                "  {} members=[{}]",
                text_value(group.label.as_deref().unwrap_or(&group.name), mode),
                group
                    .nodes
                    .iter()
                    .map(|v| text_value(v, mode))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
    }
    finish_text(lines)
}

pub(super) fn render_archimate_text(doc: &ArchimateDocument, mode: TextOutputMode) -> String {
    let mut lines = Vec::new();
    push_meta(&mut lines, "title", doc.title.as_deref(), mode);
    lines.push("archimate".to_string());
    lines.push(format!("elements ({})", doc.elements.len()));
    for element in &doc.elements {
        lines.push(format!(
            "  {} {} {}",
            text_value(&element.layer, mode),
            text_value(&element.kind, mode),
            text_value(&element.name, mode)
        ));
    }
    if !doc.relations.is_empty() {
        lines.push(format!("relations ({})", doc.relations.len()));
        for relation in &doc.relations {
            lines.push(format!(
                "  {} -{}-> {}{}",
                text_value(&relation.from, mode),
                text_value(&relation.kind, mode),
                text_value(&relation.to, mode),
                optional_label(relation.label.as_deref(), mode)
            ));
        }
    }
    finish_text(lines)
}

pub(super) fn render_regex_text(doc: &RegexDocument, mode: TextOutputMode) -> String {
    let mut lines = Vec::new();
    push_meta(&mut lines, "title", doc.title.as_deref(), mode);
    lines.push("regex".to_string());
    for pattern in &doc.patterns {
        lines.push(format!("pattern {}", text_value(&pattern.source, mode)));
        for token in &pattern.tokens {
            lines.push(format!("  {}", regex_token_text(token, mode)));
        }
    }
    finish_text(lines)
}

pub(super) fn render_ebnf_text(doc: &EbnfDocument, mode: TextOutputMode) -> String {
    let mut lines = Vec::new();
    push_meta(&mut lines, "title", doc.title.as_deref(), mode);
    lines.push("ebnf".to_string());
    for rule in &doc.rules {
        lines.push(format!(
            "{} ::= {}",
            text_value(&rule.name, mode),
            text_value(&rule.body, mode)
        ));
        for token in &rule.tokens {
            lines.push(format!("  {}", ebnf_token_text(token, mode)));
        }
    }
    finish_text(lines)
}

pub(super) fn render_math_text(doc: &MathDocument, mode: TextOutputMode) -> String {
    let mut lines = Vec::new();
    push_meta(&mut lines, "title", doc.title.as_deref(), mode);
    lines.push("math".to_string());
    for line in doc.body.lines() {
        lines.push(format!("  {}", text_value(line, mode)));
    }
    finish_text(lines)
}

pub(super) fn render_sdl_text(doc: &SdlDocument, mode: TextOutputMode) -> String {
    let mut lines = Vec::new();
    push_meta(&mut lines, "title", doc.title.as_deref(), mode);
    lines.push("sdl".to_string());
    lines.push(format!("states ({})", doc.states.len()));
    for state in &doc.states {
        lines.push(format!(
            "  {:?} {}",
            state.kind,
            text_value(&state.name, mode)
        ));
    }
    if !doc.transitions.is_empty() {
        lines.push(format!("transitions ({})", doc.transitions.len()));
        for transition in &doc.transitions {
            lines.push(format!(
                "  {} -> {}{}",
                text_value(&transition.from, mode),
                text_value(&transition.to, mode),
                optional_label(transition.signal.as_deref(), mode)
            ));
        }
    }
    finish_text(lines)
}

pub(super) fn render_ditaa_text(doc: &DitaaDocument, mode: TextOutputMode) -> String {
    let mut lines = Vec::new();
    push_meta(&mut lines, "title", doc.title.as_deref(), mode);
    lines.push("ditaa".to_string());
    for line in doc.body.lines() {
        lines.push(text_value(line, mode));
    }
    finish_text(lines)
}

pub(super) fn render_chart_text(doc: &ChartDocument, mode: TextOutputMode) -> String {
    let mut lines = Vec::new();
    push_meta(&mut lines, "title", doc.title.as_deref(), mode);
    lines.push(format!("chart {:?}", doc.subtype));
    if !doc.data.is_empty() {
        lines.push(format!("data ({})", doc.data.len()));
        for point in &doc.data {
            lines.push(format!(
                "  {} = {}",
                text_value(&point.label, mode),
                format_number(point.value)
            ));
        }
    }
    if !doc.series.is_empty() {
        lines.push(format!("series ({})", doc.series.len()));
        for series in &doc.series {
            let values = series
                .values
                .iter()
                .map(|value| format_number(*value))
                .collect::<Vec<_>>()
                .join(", ");
            lines.push(format!(
                "  {} = [{}]",
                text_value(&series.name, mode),
                values
            ));
        }
    }
    push_meta(&mut lines, "caption", doc.caption.as_deref(), mode);
    finish_text(lines)
}

fn regex_token_text(token: &RegexToken, mode: TextOutputMode) -> String {
    match token {
        RegexToken::Literal(value) => format!("literal {}", text_value(value, mode)),
        RegexToken::CharClass(value) => format!("class {}", text_value(value, mode)),
        RegexToken::Group(tokens) => format!(
            "group [{}]",
            token_list_text(tokens, regex_token_text, mode)
        ),
        RegexToken::Alt(branches) => format!(
            "alt {}",
            branches
                .iter()
                .map(|branch| token_list_text(branch, regex_token_text, mode))
                .collect::<Vec<_>>()
                .join(" | ")
        ),
        RegexToken::Repeat { inner, kind } => {
            format!(
                "repeat {} {}",
                repeat_kind_text(kind),
                regex_token_text(inner, mode)
            )
        }
        RegexToken::Escape(ch) => format!("escape {}", text_value(&ch.to_string(), mode)),
        RegexToken::AnyChar => "any".to_string(),
        RegexToken::Anchor(value) => format!("anchor {}", text_value(value, mode)),
        RegexToken::Unsupported(value) => format!("unsupported {}", text_value(value, mode)),
    }
}

fn ebnf_token_text(token: &EbnfToken, mode: TextOutputMode) -> String {
    match token {
        EbnfToken::Terminal(value) => format!("terminal {}", text_value(value, mode)),
        EbnfToken::NonTerminal(value) => format!("nonterminal {}", text_value(value, mode)),
        EbnfToken::Alt(branches) => format!(
            "alt {}",
            branches
                .iter()
                .map(|branch| token_list_text(branch, ebnf_token_text, mode))
                .collect::<Vec<_>>()
                .join(" | ")
        ),
        EbnfToken::Group(tokens) => {
            format!("group [{}]", token_list_text(tokens, ebnf_token_text, mode))
        }
        EbnfToken::Optional(tokens) => {
            format!(
                "optional [{}]",
                token_list_text(tokens, ebnf_token_text, mode)
            )
        }
        EbnfToken::Repetition(tokens) => {
            format!(
                "repetition [{}]",
                token_list_text(tokens, ebnf_token_text, mode)
            )
        }
        EbnfToken::Repeat { inner, kind } => {
            format!(
                "repeat {} {}",
                repeat_kind_text(kind),
                ebnf_token_text(inner, mode)
            )
        }
        EbnfToken::Unsupported(value) => format!("unsupported {}", text_value(value, mode)),
    }
}

fn token_list_text<T>(
    tokens: &[T],
    formatter: fn(&T, TextOutputMode) -> String,
    mode: TextOutputMode,
) -> String {
    tokens
        .iter()
        .map(|token| formatter(token, mode))
        .collect::<Vec<_>>()
        .join(", ")
}

fn repeat_kind_text(kind: &RepeatKind) -> String {
    match kind {
        RepeatKind::ZeroOrOne => "?".to_string(),
        RepeatKind::ZeroOrMore => "*".to_string(),
        RepeatKind::OneOrMore => "+".to_string(),
        RepeatKind::Exact(value) => format!("{{{value}}}"),
        RepeatKind::Range { min, max } => format!(
            "{{{},{}}}",
            min.map(|v| v.to_string()).unwrap_or_default(),
            max.map(|v| v.to_string()).unwrap_or_default()
        ),
    }
}

fn format_number(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{value:.0}")
    } else {
        let mut raw = format!("{value:.6}");
        while raw.contains('.') && raw.ends_with('0') {
            raw.pop();
        }
        if raw.ends_with('.') {
            raw.pop();
        }
        raw
    }
}
