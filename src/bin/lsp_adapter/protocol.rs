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
        if let Some(v) = line.strip_prefix("Content-Length:") {
            len = v.trim().parse::<usize>().ok();
        }
    }
    let n = match len {
        Some(v) => v,
        None => return Ok(None),
    };
    let mut b = vec![0; n];
    std::io::Read::read_exact(r, &mut b)?;
    Ok(serde_json::from_slice(&b).ok())
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
