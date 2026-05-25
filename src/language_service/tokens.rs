use super::syntax::{syntax_token_specs, SyntaxTokenKind, SyntaxTokenSpec};
use super::text::is_ident;
use crate::source::Span;

pub const SEMANTIC_TOKEN_TYPES: &[&str] = &[
    "keyword",
    "operator",
    "string",
    "comment",
    "number",
    "type",
    "class",
    "function",
    "variable",
    "parameter",
    "property",
    "namespace",
    "label",
    "decorator",
    "modifier",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticTokenKind {
    Keyword,
    Operator,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticToken {
    pub span: Span,
    pub kind: SemanticTokenKind,
}

pub fn semantic_token_legend() -> &'static [&'static str] {
    SEMANTIC_TOKEN_TYPES
}

pub fn semantic_tokens(source: &str) -> Vec<SemanticToken> {
    let mut hits = Vec::<SemanticToken>::new();
    for (spec, kind) in semantic_token_specs() {
        for span in find_token_spans(source, spec.lexeme) {
            hits.push(SemanticToken { span, kind });
        }
    }
    hits.sort_by(|a, b| {
        a.span
            .start
            .cmp(&b.span.start)
            .then_with(|| (b.span.end - b.span.start).cmp(&(a.span.end - a.span.start)))
    });

    let mut filtered = Vec::<SemanticToken>::new();
    let mut last_end = 0usize;
    for hit in hits {
        if filtered.is_empty() || hit.span.start >= last_end {
            last_end = hit.span.end;
            filtered.push(hit);
        }
    }
    filtered
}

fn semantic_token_specs() -> impl Iterator<Item = (&'static SyntaxTokenSpec, SemanticTokenKind)> {
    syntax_token_specs()
        .iter()
        .filter_map(|spec| match spec.kind {
            SyntaxTokenKind::Keyword
            | SyntaxTokenKind::Directive
            | SyntaxTokenKind::Preprocessor => Some((spec, SemanticTokenKind::Keyword)),
            SyntaxTokenKind::Operator => Some((spec, SemanticTokenKind::Operator)),
            _ => None,
        })
}

fn find_token_spans(source: &str, token: &str) -> Vec<Span> {
    let mut spans = Vec::new();
    if token.is_empty() {
        return spans;
    }
    let bytes = source.as_bytes();
    let token_bytes = token.as_bytes();
    let mut start = 0;
    while start + token_bytes.len() <= bytes.len() {
        if &bytes[start..start + token_bytes.len()] == token_bytes {
            let left = start == 0 || !is_ident(bytes[start - 1] as char);
            let end = start + token_bytes.len();
            let right = end == bytes.len() || !is_ident(bytes[end] as char);
            if left && right {
                spans.push(Span { start, end });
            }
            start += token_bytes.len();
        } else {
            start += 1;
        }
    }
    spans
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semantic_tokens_prefer_longest_operator_match() {
        let source = "@startuml\nAlice --> Bob\n@enduml\n";

        let tokens = semantic_tokens(source);

        let rendered = tokens
            .iter()
            .map(|token| (&source[token.span.start..token.span.end], token.kind))
            .collect::<Vec<_>>();

        assert_eq!(
            rendered,
            vec![
                ("@startuml", SemanticTokenKind::Keyword),
                ("-->", SemanticTokenKind::Operator),
                ("@enduml", SemanticTokenKind::Keyword),
            ]
        );
    }

    #[test]
    fn semantic_tokens_keep_stable_order_and_keyword_boundaries() {
        let source =
            "@startuml\nparticipant Alice\nAlice -> Bob\nparticipantAlias -> Bob\n@enduml\n";

        let tokens = semantic_tokens(source);
        let rendered = tokens
            .iter()
            .map(|token| (&source[token.span.start..token.span.end], token.kind))
            .collect::<Vec<_>>();

        assert_eq!(
            rendered,
            vec![
                ("@startuml", SemanticTokenKind::Keyword),
                ("participant", SemanticTokenKind::Keyword),
                ("->", SemanticTokenKind::Operator),
                ("->", SemanticTokenKind::Operator),
                ("@enduml", SemanticTokenKind::Keyword),
            ]
        );
    }

    #[test]
    fn semantic_token_legend_matches_token_kind_indices() {
        let legend = semantic_token_legend();
        assert_eq!(legend[SemanticTokenKind::Keyword as usize], "keyword");
        assert_eq!(legend[SemanticTokenKind::Operator as usize], "operator");
    }
}
