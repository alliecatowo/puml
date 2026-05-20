pub(super) fn is_ident(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

pub(super) fn lc_to_offset(src: &str, line: usize, ch: usize) -> usize {
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
