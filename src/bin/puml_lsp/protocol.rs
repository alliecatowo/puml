use serde_json::{json, Value};
use std::io::{self, BufRead, Write};

pub(crate) fn read_msg(r: &mut impl BufRead) -> io::Result<Option<Value>> {
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

pub(crate) fn resp(w: &mut impl Write, id: Value, result: Value) -> io::Result<()> {
    send(w, &json!({"jsonrpc":"2.0","id":id,"result":result}))
}

pub(crate) fn err(w: &mut impl Write, id: Value, code: i32, m: &str) -> io::Result<()> {
    send(
        w,
        &json!({"jsonrpc":"2.0","id":id,"error":{"code":code,"message":m}}),
    )
}

pub(crate) fn notif(w: &mut impl Write, m: &str, p: Value) -> io::Result<()> {
    send(w, &json!({"jsonrpc":"2.0","method":m,"params":p}))
}

fn send(w: &mut impl Write, v: &Value) -> io::Result<()> {
    let b = serde_json::to_vec(v)?;
    write!(w, "Content-Length: {}\r\n\r\n", b.len())?;
    w.write_all(&b)?;
    w.flush()
}
