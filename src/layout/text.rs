use crate::scene::TextOverflowPolicy;

pub(super) fn normalize_label_lines(
    text: &str,
    max_chars: usize,
    policy: TextOverflowPolicy,
) -> Vec<String> {
    match policy {
        TextOverflowPolicy::EllipsisSingleLine => {
            let one_line = text.replace('\n', " ");
            vec![ellipsize(&one_line, max_chars)]
        }
        TextOverflowPolicy::WrapAndGrow => text
            .lines()
            .flat_map(|line| wrap_line(line, max_chars))
            .collect::<Vec<_>>(),
    }
}

/// Count the *visual* (display) characters in a word, stripping creole/HTML
/// markup tags so that `<color:red>`, `</color>`, `<size:18>`, `</size>`,
/// `<b>`, `</b>`, `<i>`, `</i>`, `<u>`, `</u>`, `<&icon>`, etc. do not
/// inflate the character count used for line-wrapping decisions.
fn visual_char_count(word: &str) -> usize {
    let chars: Vec<char> = word.chars().collect();
    let len = chars.len();
    let mut count = 0;
    let mut i = 0;
    while i < len {
        if chars[i] == '<' {
            // Try to skip a markup tag: collect up to '>'.
            let mut j = i + 1;
            // Allow at most 32 chars inside the tag to avoid consuming large
            // non-tag `<...` sequences (e.g. math operators).
            while j < len && j - i <= 32 && chars[j] != '>' {
                j += 1;
            }
            if j < len && chars[j] == '>' {
                // Consumed a tag — skip it entirely (no visual chars).
                i = j + 1;
                continue;
            }
            // No closing '>' found within limit — treat '<' as a visual char.
            count += 1;
            i += 1;
        } else {
            count += 1;
            i += 1;
        }
    }
    count
}

pub(super) fn wrap_line(line: &str, max_chars: usize) -> Vec<String> {
    if line.is_empty() {
        return vec![String::new()];
    }
    let words = line.split_whitespace().collect::<Vec<_>>();
    if words.is_empty() {
        return vec![String::new()];
    }

    let mut lines = Vec::new();
    let mut current = String::new();
    // Track the *visual* length of `current` separately from its raw length.
    let mut current_visual: usize = 0;
    for word in words {
        let word_visual = visual_char_count(word);
        if current.is_empty() {
            if word_visual <= max_chars {
                current.push_str(word);
                current_visual = word_visual;
            } else {
                // Word is visually longer than max_chars.  If it contains
                // markup (visual_len < raw len) keep it whole rather than
                // splitting mid-tag; otherwise chunk it the old way.
                let word_raw = word.chars().count();
                if word_visual < word_raw {
                    // Contains markup — don't chunk it.
                    current.push_str(word);
                    current_visual = word_visual;
                } else {
                    for chunk in chunk_text(word, max_chars) {
                        lines.push(chunk);
                    }
                }
            }
            continue;
        }

        let next_visual = current_visual + 1 + word_visual;
        if next_visual <= max_chars {
            current.push(' ');
            current.push_str(word);
            current_visual = next_visual;
        } else {
            lines.push(current);
            let word_raw = word.chars().count();
            if word_visual <= max_chars {
                current = word.to_string();
                current_visual = word_visual;
            } else if word_visual < word_raw {
                // Contains markup — keep whole.
                current = word.to_string();
                current_visual = word_visual;
            } else {
                let mut chunks = chunk_text(word, max_chars);
                let tail = chunks.pop().unwrap_or_default();
                lines.extend(chunks);
                current_visual = visual_char_count(&tail);
                current = tail;
            }
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    debug_assert!(!lines.is_empty());
    lines
}

pub(super) fn chunk_text(text: &str, max_chars: usize) -> Vec<String> {
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

pub(super) fn ellipsize(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    if max_chars == 0 {
        return String::new();
    }
    if max_chars == 1 {
        return "…".to_string();
    }
    let mut out = String::new();
    for ch in text.chars().take(max_chars - 1) {
        out.push(ch);
    }
    out.push('…');
    out
}
