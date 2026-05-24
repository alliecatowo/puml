use super::completion::{resolve_completion_item, CompletionItem};
use super::text::{lc_to_offset, word_range_at_pos};

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hover_returns_completion_docs_for_symbol_and_word() {
        let source = "@startuml\nAlice --> Bob\nparticipant User\n@enduml\n";

        let symbol_hover = hover(source, (1, 7)).expect("symbol hover should resolve");
        assert!(symbol_hover.markdown.contains("`-->`"));
        assert!(symbol_hover.markdown.contains("Dashed message arrow."));

        let keyword_hover = hover(source, (2, 2)).expect("keyword hover should resolve");
        assert!(keyword_hover.markdown.contains("`participant`"));
        assert!(keyword_hover.markdown.contains("Declare a participant."));
    }

    #[test]
    fn hover_falls_back_to_word_literal_for_unknown_identifier() {
        let source = "@startuml\nfoobar\n@enduml\n";
        let h = hover(source, (1, 1)).expect("hover should produce fallback");
        assert_eq!(h.markdown, "`foobar`");
    }
}
