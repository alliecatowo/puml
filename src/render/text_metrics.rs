pub(crate) const DEFAULT_MONOSPACE_CHAR_WIDTH: i32 = 7;

pub(crate) fn default_monospace_width(text: &str) -> i32 {
    monospace_width(text, DEFAULT_MONOSPACE_CHAR_WIDTH)
}

pub(crate) fn monospace_width(text: &str, char_width: i32) -> i32 {
    text.chars().count() as i32 * char_width
}

pub(crate) fn proportional_monospace_width(text: &str, font_size: i32) -> i32 {
    ((text.chars().count() as i32) * font_size * 3) / 5
}

pub(crate) fn rounded_proportional_monospace_width(text: &str, font_size: i32) -> i32 {
    ((text.chars().count() as i32 * font_size * 3) + 4) / 5
}

pub(crate) fn wrap_line_by_chars(line: &str, max_chars: usize) -> Vec<String> {
    if line.is_empty() {
        return vec![String::new()];
    }
    let words = line.split_whitespace().collect::<Vec<_>>();
    if words.is_empty() {
        return vec![String::new()];
    }

    let mut lines = Vec::new();
    let mut current = String::new();
    for word in words {
        let word_len = word.chars().count();
        if current.is_empty() {
            if word_len <= max_chars {
                current.push_str(word);
            } else {
                for chunk in chunk_text(word, max_chars) {
                    lines.push(chunk);
                }
            }
            continue;
        }

        let next_len = current.chars().count() + 1 + word_len;
        if next_len <= max_chars {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(current);
            if word_len <= max_chars {
                current = word.to_string();
            } else {
                let mut chunks = chunk_text(word, max_chars);
                let tail = chunks.pop().unwrap_or_default();
                lines.extend(chunks);
                current = tail;
            }
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

pub(crate) fn chunk_text(text: &str, max_chars: usize) -> Vec<String> {
    if max_chars == 0 {
        return vec![text.to_string()];
    }
    let mut out = Vec::new();
    let mut current = String::new();
    for ch in text.chars() {
        current.push(ch);
        if current.chars().count() >= max_chars {
            out.push(current);
            current = String::new();
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
    if out.is_empty() {
        vec![String::new()]
    } else {
        out
    }
}

pub(crate) fn ellipsize_with_dots(text: &str, max_chars: usize) -> String {
    if max_chars == 0 {
        return "...".to_string();
    }
    let count = text.chars().count();
    if count <= max_chars {
        return text.to_string();
    }
    if max_chars <= 3 {
        return "...".to_string();
    }
    text.chars().take(max_chars - 3).collect::<String>() + "..."
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_and_wrap_match_existing_ascii_behavior() {
        assert_eq!(chunk_text("abcdef", 2), vec!["ab", "cd", "ef"]);
        assert_eq!(chunk_text("abc", 0), vec!["abc"]);
        assert_eq!(
            wrap_line_by_chars("seed abcdefghijklmnop", 4),
            vec!["seed", "abcd", "efgh", "ijkl", "mnop"]
        );
    }

    #[test]
    fn width_helpers_keep_family_specific_rounding() {
        assert_eq!(default_monospace_width("abc"), 21);
        assert_eq!(proportional_monospace_width("abc", 16), 28);
        assert_eq!(rounded_proportional_monospace_width("abc", 13), 24);
    }

    #[test]
    fn dots_ellipsis_preserves_legacy_family_edges() {
        assert_eq!(ellipsize_with_dots("abc", 0), "...");
        assert_eq!(ellipsize_with_dots("abcdef", 3), "...");
        assert_eq!(ellipsize_with_dots("abcdef", 5), "ab...");
    }
}
