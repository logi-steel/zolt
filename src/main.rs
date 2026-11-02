/* This program is free software. It comes without any warranty, to
 * the extent permitted by applicable law. You can redistribute it
 * and/or modify it under the terms of the Do What The Fuck You Want
 * To Public License, Version 2, as published by Sam Hocevar. See
 * http://www.wtfpl.net/ for more details. */

mod codegen;
mod lexer;
mod lsp;
mod parser;

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio, exit};
use std::{env, fs};

pub struct Blad {
    pub line: usize,
    pub col: usize,
    pub msg: String,
}

impl Blad {
    pub fn new(line: usize, col: usize, msg: impl Into<String>) -> Self {
        Blad { line, col, msg: msg.into() }
    }
}

const UZYCIE: &str = "użycie: zolt run plik.zt [argumenty] | zolt build plik.zt | zolt c plik.zt | zolt lsp";

pub fn sprawdz(src: &str) -> Result<String, Blad> {
    parser::parse(src).and_then(|p| codegen::generate(&p))
}

fn main() {
    let a: Vec<String> = env::args().collect();
    if a.len() == 2 && a[1] == "lsp" {
        lsp::run();
        return;
    }
    if a.len() < 3 || (a.len() > 3 && a[1] != "run") {
        eprintln!("{UZYCIE}");
        exit(2);
    }
    let plik = Path::new(&a[2]);
    if plik.extension().and_then(|e| e.to_str()) != Some("zt") {
        eprintln!("zolt: plik musi mieć rozszerzenie .zt");
        exit(2);
    }
    let src = fs::read_to_string(plik).unwrap_or_else(|e| {
        eprintln!("zolt: nie mogę czytać {}: {e}", plik.display());
        exit(2)
    });
    let c = match sprawdz(&src) {
        Ok(c) => c,
        Err(b) => {
            pokaz(&src, &a[2], &b);
            exit(1)
        }
    };
    match a[1].as_str() {
        "c" => print!("{c}"),
        "build" => kompiluj(&c, &plik.with_extension("")),
        "run" => {
            let dir = env::temp_dir().join(format!("zolt-{}", std::process::id()));
            fs::create_dir_all(&dir).expect("zolt: nie mogę utworzyć katalogu tymczasowego");
            let bin = dir.join("prog");
            kompiluj(&c, &bin);
            let st = Command::new(&bin).args(&a[3..]).status();
            let _ = fs::remove_dir_all(&dir);
            exit(st.ok().and_then(|s| s.code()).unwrap_or(1));
        }
        _ => {
            eprintln!("{UZYCIE}");
            exit(2)
        }
    }
}

fn kompiluj(c: &str, out: &Path) {
    let mut cc = Command::new("cc")
        .args(["-std=gnu23", "-O2", "-fwrapv", "-x", "c", "-", "-x", "none", "-lm", "-o"])
        .arg(out)
        .stdin(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| {
            eprintln!("zolt: nie mogę uruchomić cc: {e}");
            exit(1)
        });
    cc.stdin.take().unwrap().write_all(c.as_bytes()).unwrap();
    if !cc.wait().is_ok_and(|s| s.success()) {
        eprintln!("zolt: kompilator C się wywalił (to błąd w zolt, nie w twoim kodzie)");
        exit(1);
    }
}

fn pokaz(src: &str, plik: &str, b: &Blad) {
    eprintln!("błąd: {}", b.msg);
    eprintln!(" --> {plik}:{}:{}", b.line, b.col);
    if let Some(l) = src.lines().nth(b.line.saturating_sub(1)) {
        let nr = b.line.to_string();
        let pad = " ".repeat(nr.len());
        let pod: String = l.chars().take(b.col.saturating_sub(1)).map(|c| if c == '\t' { '\t' } else { ' ' }).collect();
        eprintln!("{pad} |\n{nr} | {l}\n{pad} | {pod}^");
    }
}
