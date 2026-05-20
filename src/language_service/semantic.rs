use super::util::is_ident;
use crate::source::Span;

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

pub fn semantic_tokens(source: &str) -> Vec<SemanticToken> {
    let mut hits = Vec::<SemanticToken>::new();
    for (text, kind) in [
        ("participant", SemanticTokenKind::Keyword),
        ("actor", SemanticTokenKind::Keyword),
        ("note", SemanticTokenKind::Keyword),
        ("alt", SemanticTokenKind::Keyword),
        ("else", SemanticTokenKind::Keyword),
        ("end", SemanticTokenKind::Keyword),
        ("activate", SemanticTokenKind::Keyword),
        ("deactivate", SemanticTokenKind::Keyword),
        ("create", SemanticTokenKind::Keyword),
        ("destroy", SemanticTokenKind::Keyword),
        ("return", SemanticTokenKind::Keyword),
        ("autonumber", SemanticTokenKind::Keyword),
        ("-->", SemanticTokenKind::Operator),
        ("<--", SemanticTokenKind::Operator),
        ("->", SemanticTokenKind::Operator),
    ] {
        for span in find_token_spans(source, text) {
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
