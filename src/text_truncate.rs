/// Truncates the string to at most `max_chars` characters, appending an
/// ellipsis (`…`) when truncation occurs.
///
/// If the string's length is less than or equal to `max_chars`, it is returned
/// unchanged.  Otherwise the string is shortened and `…` is appended.
pub fn truncate_ellipsis(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        return s.to_string();
    }
    s.chars().take(max_chars).collect::<String>() + "…"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_string_unchanged() {
        let s = "hello";
        assert_eq!(truncate_ellipsis(s, 10), "hello");
    }

    #[test]
    fn exact_length_unchanged() {
        let s = "hello";
        assert_eq!(truncate_ellipsis(s, 5), "hello");
    }

    #[test]
    fn truncates_and_appends_ellipsis() {
        let result = truncate_ellipsis("hello world", 5);
        // Expect first 5 chars + ellipsis: "hello…"
        assert_eq!(result, "hello…");
        // Check total visible characters: 5 chars + 1 ellipsis symbol = 6 chars
        assert_eq!(result.chars().count(), 6);
    }

    #[test]
    fn unicode_exact_length_unchanged() {
        // "café" is 4 chars / 5 bytes; must not be truncated at max_chars=4
        assert_eq!(truncate_ellipsis("café", 4), "café");
    }

    #[test]
    fn unicode_input_truncated() {
        // "café" = 4 chars, 5 bytes
        let result = truncate_ellipsis("café au lait", 4);
        assert_eq!(result, "café…");
    }

    #[test]
    fn max_chars_zero_returns_ellipsis() {
        // With max_chars=0 the early return fires when s is empty, otherwise
        // we take(0) chars and append ellipsis.
        let result = truncate_ellipsis("hi", 0);
        assert_eq!(result, "…");
    }
}
