// ─── Family 1: @startregex ────────────────────────────────────────────────────

use super::railroad::{render_railroad, RailNode};
use super::shared::strip_block;
use crate::diagnostic::Diagnostic;

pub(super) fn render_regex(source: &str) -> Result<String, Diagnostic> {
    let (body, _title) = strip_block(source, "@startregex", "@endregex");
    let mut locale = RegexLocale::English;
    let mut patterns = Vec::new();
    for raw in body.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('\'') {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if let Some(rest) = lower
            .strip_prefix("locale ")
            .or_else(|| lower.strip_prefix("language "))
            .or_else(|| lower.strip_prefix("lang "))
        {
            locale = RegexLocale::from_name(rest.trim());
            continue;
        }
        patterns.push(line.to_string());
    }
    let pattern = patterns.join("\n");
    if pattern.is_empty() {
        return Err(Diagnostic::error(
            "[E_REGEX_EMPTY] @startregex body is empty",
        ));
    }
    let node = if patterns.len() == 1 {
        parse_regex_to_rail(&patterns[0], locale)
    } else {
        RailNode::Alternation(
            patterns
                .iter()
                .map(|pattern| parse_regex_to_rail(pattern, locale))
                .collect(),
        )
    };
    let title = format!("/{}/", pattern.replace('\n', " | "));
    let svg = render_railroad(&title, &node).replacen(
        "<svg ",
        &format!(
            "<svg data-regex-locale=\"{}\" data-regex-pattern-count=\"{}\" ",
            locale.name(),
            patterns.len()
        ),
        1,
    );
    Ok(svg)
}

#[derive(Debug, Clone, Copy)]
enum RegexLocale {
    English,
    French,
    Spanish,
}

impl RegexLocale {
    fn name(self) -> &'static str {
        match self {
            Self::English => "en",
            Self::French => "fr",
            Self::Spanish => "es",
        }
    }

    fn from_name(name: &str) -> Self {
        match name {
            "fr" | "fra" | "fre" | "french" | "francais" | "français" => Self::French,
            "es" | "spa" | "spanish" | "espanol" | "español" => Self::Spanish,
            _ => Self::English,
        }
    }

    fn label(self, key: &str) -> &'static str {
        match (self, key) {
            (Self::French, "digit") => "chiffre",
            (Self::French, "word") => "mot",
            (Self::French, "space") => "espace",
            (Self::French, "any") => "tout",
            (Self::French, "start") => "debut",
            (Self::French, "end") => "fin",
            (Self::Spanish, "digit") => "digito",
            (Self::Spanish, "word") => "palabra",
            (Self::Spanish, "space") => "espacio",
            (Self::Spanish, "any") => "cualquiera",
            (Self::Spanish, "start") => "inicio",
            (Self::Spanish, "end") => "fin",
            (_, "digit") => "digit",
            (_, "word") => "word",
            (_, "space") => "whitespace",
            (_, "any") => "any char",
            (_, "start") => "start",
            (_, "end") => "end",
            _ => "regex",
        }
    }
}

/// Parse a regex pattern string into a RailNode AST.
/// Supports: literals, `.`, `|`, `(...)`, `[...]`, `*`, `+`, `?`, `^`, `$`.
fn parse_regex_to_rail(pattern: &str, locale: RegexLocale) -> RailNode {
    let chars: Vec<char> = pattern.chars().collect();
    let (node, _) = parse_regex_alternation(&chars, 0, locale);
    node
}

fn parse_regex_alternation(chars: &[char], start: usize, locale: RegexLocale) -> (RailNode, usize) {
    let mut branches = Vec::new();
    let (first, mut pos) = parse_regex_sequence(chars, start, locale);
    branches.push(first);
    while pos < chars.len() && chars[pos] == '|' {
        pos += 1;
        let (branch, new_pos) = parse_regex_sequence(chars, pos, locale);
        branches.push(branch);
        pos = new_pos;
    }
    if branches.len() == 1 {
        (branches.remove(0), pos)
    } else {
        (RailNode::Alternation(branches), pos)
    }
}

fn parse_regex_sequence(chars: &[char], start: usize, locale: RegexLocale) -> (RailNode, usize) {
    let mut items = Vec::new();
    let mut pos = start;
    while pos < chars.len() {
        match chars[pos] {
            ')' | '|' => break,
            '^' | '$' => {
                let sym = if chars[pos] == '^' {
                    locale.label("start").to_string()
                } else {
                    locale.label("end").to_string()
                };
                pos += 1;
                items.push(RailNode::Anchor(sym));
            }
            '(' => {
                pos += 1; // consume '('
                let (inner, new_pos) = parse_regex_alternation(chars, pos, locale);
                pos = new_pos;
                if pos < chars.len() && chars[pos] == ')' {
                    pos += 1;
                }
                // Check quantifier
                let (node, new_pos2) = apply_quantifier(inner, chars, pos);
                pos = new_pos2;
                items.push(node);
            }
            '[' => {
                pos += 1;
                let mut cls = String::new();
                while pos < chars.len() && chars[pos] != ']' {
                    cls.push(chars[pos]);
                    pos += 1;
                }
                if pos < chars.len() {
                    pos += 1; // consume ']'
                }
                let node = RailNode::CharClass(cls);
                let (node, new_pos) = apply_quantifier(node, chars, pos);
                pos = new_pos;
                items.push(node);
            }
            '\\' => {
                pos += 1;
                let escaped = if pos < chars.len() {
                    let c = chars[pos];
                    pos += 1;
                    if matches!(c, 'p' | 'P') && pos < chars.len() && chars[pos] == '{' {
                        let negated = c == 'P';
                        pos += 1;
                        let mut class = String::new();
                        while pos < chars.len() && chars[pos] != '}' {
                            class.push(chars[pos]);
                            pos += 1;
                        }
                        if pos < chars.len() && chars[pos] == '}' {
                            pos += 1;
                        }
                        regex_unicode_category_label(&class, negated)
                    } else {
                        regex_escape_label(c, locale)
                    }
                } else {
                    "\\".to_string()
                };
                let node = RailNode::CharClass(escaped);
                let (node, new_pos) = apply_quantifier(node, chars, pos);
                pos = new_pos;
                items.push(node);
            }
            '.' => {
                pos += 1;
                let node = RailNode::CharClass(locale.label("any").to_string());
                let (node, new_pos) = apply_quantifier(node, chars, pos);
                pos = new_pos;
                items.push(node);
            }
            c => {
                let lit = c.to_string();
                pos += 1;
                let node = RailNode::Literal(lit);
                let (node, new_pos) = apply_quantifier(node, chars, pos);
                pos = new_pos;
                items.push(node);
            }
        }
    }
    if items.is_empty() {
        (RailNode::Empty, pos)
    } else if items.len() == 1 {
        (items.remove(0), pos)
    } else {
        (RailNode::Sequence(items), pos)
    }
}

fn regex_escape_label(ch: char, locale: RegexLocale) -> String {
    match ch {
        'd' => format!("\\d {}", locale.label("digit")),
        'D' => format!("\\D not {}", locale.label("digit")),
        'w' => format!("\\w {}", locale.label("word")),
        'W' => format!("\\W not {}", locale.label("word")),
        's' => format!("\\s {}", locale.label("space")),
        'S' => format!("\\S not {}", locale.label("space")),
        't' => "\\t tab".to_string(),
        'n' => "\\n newline".to_string(),
        other => format!("\\{other}"),
    }
}

fn regex_unicode_category_label(class: &str, negated: bool) -> String {
    let label = match class {
        "L" | "Letter" => "unicode letter",
        "Lu" | "Uppercase_Letter" => "unicode uppercase letter",
        "Ll" | "Lowercase_Letter" => "unicode lowercase letter",
        "N" | "Number" => "unicode number",
        "Nd" | "Decimal_Number" => "unicode decimal digit",
        "Zs" | "Separator" => "unicode separator",
        "P" | "Punctuation" => "unicode punctuation",
        other => return format!("\\p{{{other}}} unicode category"),
    };
    if negated {
        format!("not {label}")
    } else {
        label.to_string()
    }
}

fn apply_quantifier(node: RailNode, chars: &[char], pos: usize) -> (RailNode, usize) {
    if pos >= chars.len() {
        return (node, pos);
    }
    match chars[pos] {
        '*' => (
            RailNode::Repeat(Box::new(node)),
            consume_lazy_suffix(chars, pos + 1),
        ),
        '+' => (
            RailNode::OneOrMore(Box::new(node)),
            consume_lazy_suffix(chars, pos + 1),
        ),
        '?' => (
            RailNode::Optional(Box::new(node)),
            consume_lazy_suffix(chars, pos + 1),
        ),
        '{' => {
            let mut p = pos + 1;
            while p < chars.len() && chars[p] != '}' {
                p += 1;
            }
            if p < chars.len() && chars[p] == '}' {
                let spec: String = chars[pos..=p].iter().collect();
                return (
                    RailNode::CountedRepeat(Box::new(node), spec),
                    consume_lazy_suffix(chars, p + 1),
                );
            }
            (node, pos)
        }
        _ => (node, pos),
    }
}

fn consume_lazy_suffix(chars: &[char], pos: usize) -> usize {
    if pos < chars.len() && chars[pos] == '?' {
        pos + 1
    } else {
        pos
    }
}
