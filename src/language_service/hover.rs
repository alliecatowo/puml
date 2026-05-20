use super::completion::{resolve_completion_item, CompletionItem};
use super::util::{is_ident, lc_to_offset};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hover {
    pub markdown: String,
}

pub fn hover(source: &str, position: (u64, u64)) -> Option<Hover> {
    if let Some(symbol) = symbol_at_pos(source, position) {
        if let Some(spec) = resolve_completion_item(symbol) {
            return Some(hover_for_completion(&spec));
        }
    }
    let (start, end) = word_range_at_pos(source, position)?;
    let word = &source[start..end];
    if let Some(spec) = resolve_completion_item(word) {
        return Some(hover_for_completion(&spec));
    }
    Some(Hover {
        markdown: format!("`{word}`"),
    })
}

fn hover_for_completion(spec: &CompletionItem) -> Hover {
    Hover {
        markdown: format!("`{}`\n\n{}", spec.label, spec.documentation),
    }
}

fn symbol_at_pos(src: &str, posn: (u64, u64)) -> Option<&'static str> {
    let off = lc_to_offset(src, posn.0 as usize, posn.1 as usize);
    if off >= src.len() {
        return None;
    }
    const SYMBOLS: &[&str] = &[
        "-[#color,dashed]>",
        "-[#color,bold]>",
        "-[#color]>",
        "-->>",
        "<<--",
        "<-->",
        "->>",
        "<<-",
        "-->",
        "<--",
        "<->",
        "->x",
        "x->",
        "->o",
        "o->",
        "->",
        "<-",
        "-x",
        "++",
        "--",
        "**",
        "!!",
    ];
    for symbol in SYMBOLS {
        for (start, _) in src.match_indices(symbol) {
            let end = start + symbol.len();
            if off >= start && off < end {
                return Some(symbol);
            }
        }
    }
    None
}

fn word_range_at_pos(src: &str, posn: (u64, u64)) -> Option<(usize, usize)> {
    let off = lc_to_offset(src, posn.0 as usize, posn.1 as usize);
    if off >= src.len() {
        return None;
    }
    let b = src.as_bytes();
    if !is_ident(b[off] as char) {
        return None;
    }
    let mut s = off;
    while s > 0 && is_ident(b[s - 1] as char) {
        s -= 1;
    }
    let mut e = off;
    while e < b.len() && is_ident(b[e] as char) {
        e += 1;
    }
    Some((s, e))
}
