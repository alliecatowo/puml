use serde_json::{json, Value};

#[derive(Clone, Debug)]
pub(crate) struct RefHit {
    pub(crate) start: usize,
    pub(crate) end: usize,
}

pub(crate) fn is_ident(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

pub(crate) fn word_range_at_pos(src: &str, posn: (u64, u64)) -> Option<(usize, usize)> {
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

pub(crate) fn lc_to_offset(src: &str, line: usize, ch: usize) -> usize {
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

pub(crate) fn offset_to_lc(src: &str, off: usize) -> (usize, usize) {
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

pub(crate) fn read_pos(msg: &Value) -> Option<(u64, u64)> {
    Some((
        msg.pointer("/params/position/line")?.as_u64()?,
        msg.pointer("/params/position/character")?.as_u64()?,
    ))
}

pub(crate) fn range(src: &str, s: usize, e: usize) -> Value {
    json!({"start":pos(src,s),"end":pos(src,e.max(s+1))})
}

pub(crate) fn pos(src: &str, off: usize) -> Value {
    let (l, c) = offset_to_lc(src, off);
    json!({"line":l,"character":c})
}
