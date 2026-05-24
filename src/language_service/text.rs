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

pub fn lc_to_offset(src: &str, line: usize, ch: usize) -> usize {
    let mut l = 0usize;
    let mut c = 0usize;
    for (i, k) in src.char_indices() {
        if l == line && c == ch {
            return i;
        }
        if k == '\n' {
            l += 1;
            c = 0;
        } else {
            c += 1;
        }
    }
    src.len()
}

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
            c += 1;
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
}
