/* This program is free software. It comes without any warranty, to
 * the extent permitted by applicable law. You can redistribute it
 * and/or modify it under the terms of the Do What The Fuck You Want
 * To Public License, Version 2, as published by Sam Hocevar. See
 * http://www.wtfpl.net/ for more details. */

use crate::Blad;

#[derive(Debug, Clone, PartialEq)]
pub enum Tok {
    Ident(String),
    Int(u128, Option<u32>),     // 12, c8_12
    Float(String, Option<u32>), // 3.14, c32_3.14
    Str(Vec<StrPart>),
    List(Vec<Item>), // [oki, baj, po co, 12, true]
    Mut,             // &_
    Amp,             // &
    Colon,
    Eq,
    Lt,
    Gt,
    Le,
    Ge,
    Dyn, // <>
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Path,  // ::
    Arrow,  // =>
    RArrow, // ->
    Dot,
    Comma,
    LParen,
    RParen,
    LBrace,
    RBrace,
    Eof,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Txt(String),
    Int(i64),
    Flt(String),
    Bool(bool),
}

#[derive(Debug, Clone, PartialEq)]
pub enum StrPart {
    Lit(String),
    Code(String, usize, usize), // treść <...> i jej pozycja
}

#[derive(Debug, Clone)]
pub struct Token {
    pub tok: Tok,
    pub line: usize,
    pub col: usize,
}

struct Lx {
    cs: Vec<char>,
    i: usize,
    line: usize,
    col: usize,
}

impl Lx {
    fn peek(&self, k: usize) -> char {
        *self.cs.get(self.i + k).unwrap_or(&'\0')
    }
    fn end(&self) -> bool {
        self.i >= self.cs.len()
    }
    fn bump(&mut self) -> char {
        let c = self.peek(0);
        self.i += 1;
        if c == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        c
    }
}

pub fn lex(src: &str) -> Result<Vec<Token>, Blad> {
    lex_at(src, 1, 1)
}

pub fn lex_at(src: &str, line: usize, col: usize) -> Result<Vec<Token>, Blad> {
    let mut l = Lx { cs: src.chars().collect(), i: 0, line, col };
    let mut out = Vec::new();
    while !l.end() {
        let c = l.peek(0);
        if c.is_whitespace() {
            l.bump();
            continue;
        }
        let (line, col) = (l.line, l.col);
        let blad = move |m: &str| Blad::new(line, col, m);
        // __ komentarz __
        if c == '_' && l.peek(1) == '_' {
            l.bump();
            l.bump();
            while !(l.peek(0) == '_' && l.peek(1) == '_') {
                if l.end() {
                    return Err(blad("komentarz `__` nie ma zamykającego `__`"));
                }
                l.bump();
            }
            l.bump();
            l.bump();
            continue;
        }
        let tok = if c.is_ascii_digit() {
            liczba(&mut l, None).map_err(|m| blad(m))?
        } else if c == 'c' && typed_ahead(&l) {
            // c32_3.14: rozmiar w bitach przyklejony do wartości
            l.bump();
            let mut bits = String::new();
            while l.peek(0).is_ascii_digit() {
                bits.push(l.bump());
            }
            l.bump(); // _
            let bits = bits.parse().map_err(|_| blad("za duży rozmiar (max c128)"))?;
            liczba(&mut l, Some(bits)).map_err(|m| blad(m))?
        } else if c.is_alphabetic() || c == '_' {
            let mut s = String::new();
            while (l.peek(0).is_alphanumeric() || l.peek(0) == '_')
                && !(l.peek(0) == '_' && l.peek(1) == '_')
            {
                s.push(l.bump());
            }
            Tok::Ident(s)
        } else if c == '"' {
            l.bump();
            Tok::Str(tekst(&mut l, line, col)?)
        } else if c == '[' {
            // gołe słowa to tekst, jak w Bariście
            l.bump();
            let mut raw = String::new();
            while l.peek(0) != ']' {
                if l.end() {
                    return Err(blad("lista nie ma zamykającego `]`"));
                }
                raw.push(l.bump());
            }
            l.bump();
            let items = if raw.trim().is_empty() { vec![] } else { raw.split(',').map(item).collect() };
            Tok::List(items)
        } else {
            let two = match (c, l.peek(1)) {
                ('&', '_') => Some(Tok::Mut),
                ('<', '>') => Some(Tok::Dyn),
                ('<', '=') => Some(Tok::Le),
                ('>', '=') => Some(Tok::Ge),
                ('=', '>') => Some(Tok::Arrow),
                ('-', '>') => Some(Tok::RArrow),
                (':', ':') => Some(Tok::Path),
                _ => None,
            };
            if let Some(t) = two {
                l.bump();
                l.bump();
                t
            } else {
                let t = match c {
                    ':' => Tok::Colon,
                    '=' => Tok::Eq,
                    '<' => Tok::Lt,
                    '>' => Tok::Gt,
                    '+' => Tok::Plus,
                    '-' => Tok::Minus,
                    '*' => Tok::Star,
                    '/' => Tok::Slash,
                    '%' => Tok::Percent,
                    '.' => Tok::Dot,
                    ',' => Tok::Comma,
                    '(' => Tok::LParen,
                    ')' => Tok::RParen,
                    '{' => Tok::LBrace,
                    '}' => Tok::RBrace,
                    '&' => Tok::Amp,
                    _ => return Err(blad(&format!("nie znam znaku `{c}`"))),
                };
                l.bump();
                t
            }
        };
        out.push(Token { tok, line, col });
    }
    out.push(Token { tok: Tok::Eof, line: l.line, col: l.col });
    Ok(out)
}

// element listy: "tekst", liczba, true/false, a reszta to tekst
fn item(s: &str) -> Item {
    let s = s.trim();
    if let Some(t) = s.strip_prefix('"').and_then(|s| s.strip_suffix('"')) {
        return Item::Txt(t.to_string());
    }
    match s {
        "true" | "b.true" => return Item::Bool(true),
        "false" | "b.false" => return Item::Bool(false),
        _ => {}
    }
    if s.chars().all(|c| c.is_ascii_digit() || c == '.' || c == '-') && s.chars().any(|c| c.is_ascii_digit()) {
        if let Ok(v) = s.parse() {
            return Item::Int(v);
        }
        if s.parse::<f64>().is_ok() {
            return Item::Flt(s.to_string());
        }
    }
    Item::Txt(s.to_string())
}

// c, cyfry, _, cyfra
fn typed_ahead(l: &Lx) -> bool {
    let mut j = 1;
    while l.peek(j).is_ascii_digit() {
        j += 1;
    }
    j > 1 && l.peek(j) == '_' && l.peek(j + 1).is_ascii_digit()
}

fn liczba(l: &mut Lx, bits: Option<u32>) -> Result<Tok, &'static str> {
    let mut s = String::new();
    while l.peek(0).is_ascii_digit() {
        s.push(l.bump());
    }
    if l.peek(0) == '.' && l.peek(1).is_ascii_digit() {
        s.push(l.bump());
        while l.peek(0).is_ascii_digit() {
            s.push(l.bump());
        }
        return Ok(Tok::Float(s, bits));
    }
    s.parse().map(|v| Tok::Int(v, bits)).map_err(|_| "liczba jest za duża (max 128 bitów)")
}

fn tekst(l: &mut Lx, line: usize, col: usize) -> Result<Vec<StrPart>, Blad> {
    let mut parts = Vec::new();
    let mut lit = String::new();
    loop {
        if l.end() {
            return Err(Blad::new(line, col, "tekst nie ma zamykającego `\"`"));
        }
        let (cl, cc) = (l.line, l.col);
        match l.bump() {
            '"' => break,
            '\\' => {
                let e = l.bump();
                lit.push(match e {
                    'n' => '\n',
                    't' => '\t',
                    '"' | '\\' | '<' | '>' => e,
                    _ => return Err(Blad::new(cl, cc, format!("nie znam znaku specjalnego `\\{e}`"))),
                });
            }
            '<' => {
                let (sl, sc) = (l.line, l.col);
                let mut code = String::new();
                while l.peek(0) != '>' {
                    if l.end() || l.peek(0) == '"' {
                        return Err(Blad::new(cl, cc, "`<` w tekście otwiera wstawkę, ale brak `>` (zwykły znak `<` to `\\<`)"));
                    }
                    code.push(l.bump());
                }
                l.bump();
                if code.trim().is_empty() {
                    return Err(Blad::new(cl, cc, "pusta wstawka `<>` (zwykły znak `<` to `\\<`)"));
                }
                if !lit.is_empty() {
                    parts.push(StrPart::Lit(std::mem::take(&mut lit)));
                }
                parts.push(StrPart::Code(code, sl, sc));
            }
            ch => lit.push(ch),
        }
    }
    if !lit.is_empty() || parts.is_empty() {
        parts.push(StrPart::Lit(lit));
    }
    Ok(parts)
}
