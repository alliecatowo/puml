use super::text::TextOutputMode;

pub(super) fn tree_branch(mode: TextOutputMode) -> &'static str {
    if mode.unicode() {
        "\u{251c}\u{2500} "
    } else {
        "|- "
    }
}

pub(super) fn tree_leaf(mode: TextOutputMode) -> &'static str {
    if mode.unicode() {
        "\u{2514}\u{2500} "
    } else {
        "`- "
    }
}

pub(super) fn optional_label(value: Option<&str>, mode: TextOutputMode) -> String {
    value
        .map(|v| format!(" {}", text_value(v, mode)))
        .unwrap_or_default()
}

pub(super) fn push_meta(
    lines: &mut Vec<String>,
    key: &str,
    value: Option<&str>,
    mode: TextOutputMode,
) {
    if let Some(value) = value {
        let value = text_value(value, mode);
        if !value.is_empty() {
            lines.push(format!("{key}: {value}"));
        }
    }
}

pub(super) fn text_value(value: &str, mode: TextOutputMode) -> String {
    let single_line = value
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" / ");
    if mode.unicode() {
        return single_line;
    }
    single_line
        .chars()
        .map(|ch| {
            if ch.is_ascii() && !ch.is_control() {
                ch
            } else if ch == '\t' {
                ' '
            } else {
                '?'
            }
        })
        .collect()
}

pub(super) fn finish_text(lines: Vec<String>) -> String {
    let mut out = lines.join("\n");
    out.push('\n');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_mode_replaces_non_ascii_and_flattens_lines() {
        assert_eq!(
            text_value("hello\n∞\tworld", TextOutputMode::Txt),
            "hello / ? world"
        );
    }

    #[test]
    fn unicode_mode_keeps_non_ascii() {
        assert_eq!(text_value("hello\n∞", TextOutputMode::Utxt), "hello / ∞");
    }

    #[test]
    fn tree_glyphs_match_legacy_modes() {
        assert_eq!(tree_branch(TextOutputMode::Txt), "|- ");
        assert_eq!(tree_leaf(TextOutputMode::Txt), "`- ");
        assert_eq!(tree_branch(TextOutputMode::Utxt), "├─ ");
        assert_eq!(tree_leaf(TextOutputMode::Utxt), "└─ ");
    }
}
