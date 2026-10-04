use puml::language_service::offset_to_lc;
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};

pub fn caps() -> Value {
    puml::lsp_capabilities()
}

pub fn read_pos(msg: &Value) -> Option<(u64, u64)> {
    Some((
        msg.pointer("/params/position/line")?.as_u64()?,
        msg.pointer("/params/position/character")?.as_u64()?,
    ))
}

pub fn range(src: &str, s: usize, e: usize) -> Value {
    json!({"start":pos(src,s),"end":pos(src,e.max(s+1))})
}

pub fn pos(src: &str, off: usize) -> Value {
    let (l, c) = offset_to_lc(src, off);
    json!({"line":l,"character":c})
}

const MAX_FRAME_BYTES: usize = 64 * 1024 * 1024;

pub fn read_msg(r: &mut impl BufRead) -> io::Result<Option<Value>> {
    let mut len = None;
    loop {
        let mut line = String::new();
        if r.read_line(&mut line)? == 0 {
            return Ok(None);
        };
        if line == "\r\n" {
            break;
        }
        if line.len() >= 15 && line.as_bytes()[..15].eq_ignore_ascii_case(b"content-length:") {
            len = line[15..].trim().parse::<usize>().ok();
        }
    }
    // A frame without a usable Content-Length is skipped, not fatal.
    let n = match len {
        Some(v) if v <= MAX_FRAME_BYTES => v,
        Some(_) => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "LSP frame exceeds size limit",
            ))
        }
        None => return Ok(Some(Value::Null)),
    };
    let mut b = vec![0; n];
    std::io::Read::read_exact(r, &mut b)?;
    // Malformed JSON yields a Null message (ignored by the dispatch loop)
    // rather than ending the session.
    Ok(Some(serde_json::from_slice(&b).unwrap_or(Value::Null)))
}

pub fn resp(w: &mut impl Write, id: Value, result: Value) -> io::Result<()> {
    send(w, &json!({"jsonrpc":"2.0","id":id,"result":result}))
}

pub fn err(w: &mut impl Write, id: Value, code: i32, m: &str) -> io::Result<()> {
    send(
        w,
        &json!({"jsonrpc":"2.0","id":id,"error":{"code":code,"message":m}}),
    )
}

pub fn notif(w: &mut impl Write, m: &str, p: Value) -> io::Result<()> {
    send(w, &json!({"jsonrpc":"2.0","method":m,"params":p}))
}

pub fn send(w: &mut impl Write, v: &Value) -> io::Result<()> {
    let b = serde_json::to_vec(v)?;
    write!(w, "Content-Length: {}\r\n\r\n", b.len())?;
    w.write_all(&b)?;
    w.flush()
}
