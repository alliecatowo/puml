pub(crate) fn is_ident(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

pub fn word_range_at_pos(src: &str, posn: (u64, u64)) -> Option<(usize, usize)> {
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

/// Convert an LSP position (line, UTF-16 code unit column) to a byte offset.
///
/// Columns past the end of a line clamp to the line end (never into a later
/// line); a column inside a surrogate pair rounds up to the next character.
pub fn lc_to_offset(src: &str, line: usize, ch: usize) -> usize {
    let mut l = 0usize;
    let mut c = 0usize;
    for (i, k) in src.char_indices() {
        if l == line && (c >= ch || k == '\n') {
            return i;
        }
        if k == '\n' {
            l += 1;
            c = 0;
        } else {
            c += k.len_utf16();
        }
    }
    src.len()
}

/// Convert a byte offset to an LSP position (line, UTF-16 code unit column).
pub fn offset_to_lc(src: &str, off: usize) -> (usize, usize) {
    let mut l = 0usize;
    let mut c = 0usize;
    for (i, k) in src.char_indices() {
        if i >= off.min(src.len()) {
            break;
        }
        if k == '\n' {
            l += 1;
            c = 0;
        } else {
            c += k.len_utf16();
        }
    }
    (l, c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn word_range_and_offsets_respect_identifier_boundaries() {
        assert_eq!(word_range_at_pos("Alice -> Bob", (0, 0)), Some((0, 5)));
        assert!(word_range_at_pos("Alice -> Bob", (0, 5)).is_none());
        assert_eq!(lc_to_offset("a\nβ", 1, 1), "a\nβ".len());
        assert_eq!(offset_to_lc("a\nβ", "a\n".len()), (1, 0));
    }

    #[test]
    fn positions_use_utf16_code_units() {
        let src = "\u{1F600}ab\ncd";
        // The emoji is 2 UTF-16 units, so `a` starts at column 2.
        assert_eq!(lc_to_offset(src, 0, 2), "\u{1F600}".len());
        assert_eq!(lc_to_offset(src, 0, 3), "\u{1F600}a".len());
        assert_eq!(offset_to_lc(src, "\u{1F600}a".len()), (0, 3));
        // BMP non-ASCII is a single unit.
        assert_eq!(offset_to_lc("\u{3b2}x", "\u{3b2}".len()), (0, 1));
        // Middle of a surrogate pair rounds up to the next character.
        assert_eq!(lc_to_offset(src, 0, 1), "\u{1F600}".len());
    }

    #[test]
    fn over_long_columns_clamp_to_line_end() {
        let src = "ab\ncd\n";
        assert_eq!(lc_to_offset(src, 0, 99), 2);
        assert_eq!(lc_to_offset(src, 1, 99), 5);
        // Beyond the last line: end of document.
        assert_eq!(lc_to_offset(src, 9, 0), src.len());
    }
}
