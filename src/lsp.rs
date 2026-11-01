/* This program is free software. It comes without any warranty, to
 * the extent permitted by applicable law. You can redistribute it
 * and/or modify it under the terms of the Do What The Fuck You Want
 * To Public License, Version 2, as published by Sam Hocevar. See
 * http://www.wtfpl.net/ for more details. */

// zolt lsp: serwer LSP na stdin/stdout, na razie tylko błędy (diagnostics)
use serde_json::{Value, json};
use std::io::{BufRead, Write};

use crate::Blad;

pub fn run() {
    let mut inp = std::io::stdin().lock();
    let mut out = std::io::stdout().lock();
    while let Some(msg) = read(&mut inp) {
        let id = msg.get("id").cloned();
        let p = &msg["params"];
        match msg["method"].as_str().unwrap_or("") {
            "initialize" => reply(&mut out, id, json!({ "capabilities": { "textDocumentSync": 1 }, "serverInfo": { "name": "zolt" } })),
            "shutdown" => reply(&mut out, id, Value::Null),
            "exit" => return,
            "textDocument/didOpen" => check(&mut out, &p["textDocument"]["uri"], p["textDocument"]["text"].as_str().unwrap_or("")),
            // textDocumentSync 1 = przy każdej zmianie przychodzi cały plik
            "textDocument/didChange" => {
                let text = p["contentChanges"].as_array().and_then(|c| c.last()).and_then(|c| c["text"].as_str());
                check(&mut out, &p["textDocument"]["uri"], text.unwrap_or(""));
            }
            "textDocument/didClose" => publish(&mut out, &p["textDocument"]["uri"], vec![]),
            "" => {}
            _ => {
                if let Some(id) = id {
                    send(&mut out, json!({ "jsonrpc": "2.0", "id": id, "error": { "code": -32601, "message": "zolt lsp tego jeszcze nie umie" } }));
                }
            }
        }
    }
}

// Content-Length: N\r\n\r\n{json}
fn read(inp: &mut impl BufRead) -> Option<Value> {
    let mut len = 0;
    loop {
        let mut line = String::new();
        if inp.read_line(&mut line).ok()? == 0 {
            return None;
        }
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        if let Some(v) = line.strip_prefix("Content-Length:") {
            len = v.trim().parse().ok()?;
        }
    }
    let mut buf = vec![0; len];
    inp.read_exact(&mut buf).ok()?;
    Some(serde_json::from_slice(&buf).unwrap_or(Value::Null))
}

fn send(out: &mut impl Write, v: Value) {
    let s = v.to_string();
    let _ = write!(out, "Content-Length: {}\r\n\r\n{s}", s.len());
    let _ = out.flush();
}

fn reply(out: &mut impl Write, id: Option<Value>, result: Value) {
    send(out, json!({ "jsonrpc": "2.0", "id": id, "result": result }));
}

fn publish(out: &mut impl Write, uri: &Value, diags: Vec<Value>) {
    send(out, json!({ "jsonrpc": "2.0", "method": "textDocument/publishDiagnostics", "params": { "uri": uri, "diagnostics": diags } }));
}

fn check(out: &mut impl Write, uri: &Value, text: &str) {
    let diags = match std::panic::catch_unwind(|| crate::sprawdz(text)) {
        Ok(Ok(_)) => vec![],
        Ok(Err(b)) => vec![diag(text, &b)],
        Err(_) => vec![diag(text, &Blad::new(1, 1, "zolt się wywalił na tym pliku (to błąd w zolt, nie w twoim kodzie)"))],
    };
    publish(out, uri, diags);
}

// Blad liczy od 1 w znakach, LSP od 0 w jednostkach UTF-16; podkreślamy całe słowo
fn diag(text: &str, b: &Blad) -> Value {
    let line = b.line.saturating_sub(1);
    let chars: Vec<char> = text.lines().nth(line).unwrap_or("").chars().collect();
    let s = b.col.saturating_sub(1).min(chars.len());
    let mut e = s;
    while e < chars.len() && (chars[e].is_alphanumeric() || chars[e] == '_') {
        e += 1;
    }
    if e == s {
        e = (s + 1).min(chars.len());
    }
    let u = |n: usize| chars[..n].iter().map(|c| c.len_utf16()).sum::<usize>();
    json!({
        "range": { "start": { "line": line, "character": u(s) }, "end": { "line": line, "character": u(e) } },
        "severity": 1,
        "source": "zolt",
        "message": b.msg,
    })
}
