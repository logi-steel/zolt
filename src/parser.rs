/* This program is free software. It comes without any warranty, to
 * the extent permitted by applicable law. You can redistribute it
 * and/or modify it under the terms of the Do What The Fuck You Want
 * To Public License, Version 2, as published by Sam Hocevar. See
 * http://www.wtfpl.net/ for more details. */

use crate::Blad;
use crate::lexer::{Item, StrPart, Tok, Token, lex, lex_at};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Op {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Lt,
    Gt,
    Le,
    Ge,
}

pub enum Part {
    Lit(String),
    Code(E),
}

pub enum Ex {
    Int(u128, Option<u32>),
    Float(String, Option<u32>),
    Str(Vec<Part>),
    Bool(bool),
    Var(String),
    Read(usize), // sys::read1
    Inf(usize),  // loop.inf.1
    Neg(Box<E>),
    Bin(Op, Box<E>, Box<E>),
    Dyn(Box<E>, Box<E>, Box<E>), // a <>op<> b
    Lst(String),                 // spoczko.lst
    ListLit(Vec<Item>),
    Has(Box<E>, Box<E>),
    Call(String, Vec<Arg>), // dodaj(5, &_x)
    Field(Box<E>, String),  // gracz.1.hp
    MCall(Box<E>, String, Vec<Arg>), // gracz.1.exp()
}

pub struct E {
    pub ex: Ex,
    pub line: usize,
    pub col: usize,
}

impl E {
    pub fn err(&self, m: impl Into<String>) -> Blad {
        Blad::new(self.line, self.col, m)
    }
}

pub enum LoopKind {
    Times(E),
    Inf(usize),
}

pub struct Arm {
    pub cond: Option<E>, // None = else
    pub lp: Option<LoopKind>,
    pub body: Vec<St>,
}

pub enum St {
    Let { name: String, mutable: bool, val: E, line: usize, col: usize },
    Set { target: E, val: E },
    Out(E),
    Read(E, usize),
    If(Vec<Arm>),
    Loop(LoopKind, Vec<St>),
    While(E, Vec<St>),
    Break(usize, usize), // loop,end
    Call(E),             // Ex::Call albo Ex::MCall
    New { name: String, tpl: String, mutable: bool, line: usize, col: usize }, // let &_gracz.1
    Ret(Option<E>, usize, usize),
    ListSet { name: String, val: E },
    ListOp { name: String, op: ListOp, line: usize, col: usize },
}

pub enum ListOp {
    Add(E),
    Del(E),
    Chnm(E, E),
}

// a, &a (pożyczka do czytania), &_a (pożyczka do zmian)
#[derive(Clone, Copy, PartialEq)]
pub enum Kind {
    Val,
    Read,
    Write,
}

pub struct Param {
    pub name: String,
    pub kind: Kind,
    pub list: bool, // x.lst
    pub line: usize,
    pub col: usize,
}

impl Param {
    pub fn show(&self) -> String {
        let pre = match self.kind {
            Kind::Val => "",
            Kind::Read => "&",
            Kind::Write => "&_",
        };
        format!("{pre}{}{}", self.name, if self.list { ".lst" } else { "" })
    }
}

pub struct Arg {
    pub kind: Kind,
    pub e: E, // przy pożyczce: Ex::Var albo Ex::Lst
}

pub struct Func {
    pub name: String,
    pub params: Vec<Param>,
    pub uses: Vec<String>,
    pub body: Vec<St>,
    pub reads: usize,
    pub infs: usize,
    pub rets: bool,           // jest `retrn wartość`
    pub owner: Option<usize>, // metoda szablonu nr ...
}

// gracz : { hp : 30  exp : { ... } }
pub struct Tpl {
    pub name: String,
    pub fields: Vec<(String, E)>,
}

pub struct Program {
    pub funcs: Vec<Func>,
    pub tpls: Vec<Tpl>,
}

pub const KEYWORDS: &[&str] =
    &["fn", "let", "if", "else", "elseif", "loop", "while", "has", "true", "false", "use", "sys", "retrn"];

struct Parser {
    t: Vec<Token>,
    p: usize,
    reads: usize,      // ile sys::read(...) było do tej pory w funkcji
    infs: usize,       // ile loop.inf było do tej pory w funkcji
    uses: Vec<String>, // use => ... bieżącej funkcji
    rets: bool,        // było `retrn wartość` w funkcji
}

pub fn parse(src: &str) -> Result<Program, Blad> {
    let mut p = Parser { t: lex(src)?, p: 0, reads: 0, infs: 0, uses: vec![], rets: false };
    let mut funcs: Vec<Func> = Vec::new();
    let mut tpls: Vec<Tpl> = Vec::new();
    while *p.peek() != Tok::Eof {
        if p.is_ident("fn") {
            let f = p.func(&funcs)?;
            if tpls.iter().any(|t| t.name == f.name) {
                return p.err(format!("`{}` to już nazwa szablonu", f.name));
            }
            funcs.push(f);
        } else {
            p.tpl(&mut funcs, &mut tpls)?;
        }
    }
    if !funcs.iter().any(|f| f.name == "main") {
        return Err(Blad::new(1, 1, "brakuje `fn main () { ... }`"));
    }
    Ok(Program { funcs, tpls })
}

impl Parser {
    fn func(&mut self, done: &[Func]) -> Result<Func, Blad> {
        if !self.is_ident("fn") {
            return self.err("tu powinna zacząć się funkcja: `fn nazwa (...) {`");
        }
        self.next();
        let (line, col) = self.pos();
        let name = self.ident("nazwa funkcji")?;
        if KEYWORDS.contains(&name.as_str()) {
            return Err(Blad::new(line, col, format!("`{name}` to słowo kluczowe, nie może być nazwą funkcji")));
        }
        if done.iter().any(|f| f.name == name) {
            return Err(Blad::new(line, col, format!("funkcja `{name}` już jest")));
        }
        self.rest(name, None)
    }

    // (parametry use => ...) { ciało }; metoda szablonu może nie mieć nawiasu
    fn rest(&mut self, name: String, owner: Option<usize>) -> Result<Func, Blad> {
        let mut params = Vec::new();
        self.uses = Vec::new();
        if owner.is_none() || *self.peek() == Tok::LParen {
            self.expect(Tok::LParen, "`(`")?;
            params = self.params()?;
            self.uses = self.uses()?;
            self.expect(Tok::RParen, "`)`")?;
        }
        if let Some(p) = params.first()
            && name == "main"
        {
            return Err(Blad::new(p.line, p.col, "`main` nie bierze parametrów"));
        }
        if let Some(p) = params.iter().find(|p| p.list)
            && let Some(m) = self.lacks("std::lists", "listy")
        {
            return Err(Blad::new(p.line, p.col, m));
        }
        (self.reads, self.infs, self.rets) = (0, 0, false);
        let body = self.block()?;
        Ok(Func {
            name,
            params,
            uses: std::mem::take(&mut self.uses),
            body,
            reads: self.reads,
            infs: self.infs,
            rets: self.rets,
            owner,
        })
    }

    // gracz : { hp : 30  exp : { ... }  pokaz : (use => std::io) { ... } }
    fn tpl(&mut self, funcs: &mut Vec<Func>, tpls: &mut Vec<Tpl>) -> Result<(), Blad> {
        let (line, col) = self.pos();
        let name = match self.peek().clone() {
            Tok::Ident(n) if *self.peek_at(1) == Tok::Colon => n,
            _ => return self.err("tu powinna zacząć się funkcja `fn nazwa (...) {` albo szablon `nazwa : { ... }`"),
        };
        if KEYWORDS.contains(&name.as_str()) {
            return Err(Blad::new(line, col, format!("`{name}` to słowo kluczowe, nie może być nazwą szablonu")));
        }
        if tpls.iter().any(|t| t.name == name) || funcs.iter().any(|f| f.name == name) {
            return Err(Blad::new(line, col, format!("`{name}` już jest")));
        }
        self.next();
        self.next();
        self.expect(Tok::LBrace, "`{` (szablon: `nazwa : { pole : wartość }`)")?;
        let me = tpls.len();
        let mut fields: Vec<(String, E)> = Vec::new();
        let mut methods: Vec<String> = Vec::new();
        while !self.eat(&Tok::RBrace) {
            if *self.peek() == Tok::Eof {
                return self.err("szablon nie ma zamykającego `}`");
            }
            let (fl, fc) = self.pos();
            let f = self.ident("nazwa pola, np. `hp : 30`")?;
            if KEYWORDS.contains(&f.as_str()) {
                return Err(Blad::new(fl, fc, format!("`{f}` to słowo kluczowe, nie może być nazwą pola")));
            }
            if fields.iter().any(|(n, _)| *n == f) || methods.contains(&f) {
                return Err(Blad::new(fl, fc, format!("`{f}` już jest w szablonie")));
            }
            self.expect(Tok::Colon, "`:` (np. `hp : 30`)")?;
            if *self.peek() == Tok::LBrace || self.method_parens() {
                methods.push(f.clone());
                funcs.push(self.rest(format!("{name}.{f}"), Some(me))?);
            } else {
                (self.uses, self.reads, self.infs) = (vec![], 0, 0);
                fields.push((f, self.expr()?));
            }
        }
        tpls.push(Tpl { name, fields });
        Ok(())
    }

    // `(...) {` po dwukropku w szablonie to metoda z parametrami
    fn method_parens(&self) -> bool {
        if *self.peek() != Tok::LParen {
            return false;
        }
        let mut depth = 0;
        for k in 0.. {
            match self.peek_at(k) {
                Tok::LParen => depth += 1,
                Tok::RParen => {
                    depth -= 1;
                    if depth == 0 {
                        return *self.peek_at(k + 1) == Tok::LBrace;
                    }
                }
                Tok::Eof => break,
                _ => {}
            }
        }
        false
    }

    // gracz + .1 = gracz.1
    fn obj_num(&mut self, name: String) -> String {
        if *self.peek() == Tok::Dot
            && let &Tok::Int(n, None) = self.peek_at(1)
        {
            self.next();
            self.next();
            return format!("{name}.{n}");
        }
        name
    }

    // gracz.1.hp, gracz.1.exp()
    fn postfix(&mut self, mut e: E) -> Result<E, Blad> {
        while *self.peek() == Tok::Dot && matches!(self.peek_at(1), Tok::Ident(s) if s != "lst") {
            self.next();
            let (line, col) = self.pos();
            let f = self.ident("nazwa pola")?;
            e = if self.eat(&Tok::LParen) {
                E { ex: Ex::MCall(Box::new(e), f, self.args()?), line, col }
            } else {
                E { ex: Ex::Field(Box::new(e), f), line, col }
            };
        }
        Ok(e)
    }

    // (a, &b, &_c.lst use => ...)
    fn params(&mut self) -> Result<Vec<Param>, Blad> {
        let mut v: Vec<Param> = Vec::new();
        while *self.peek() != Tok::RParen && !self.is_ident("use") {
            let kind = self.kind();
            let (line, col) = self.pos();
            let name = self.ident("nazwa parametru albo `use => ...`")?;
            if KEYWORDS.contains(&name.as_str()) {
                return Err(Blad::new(line, col, format!("`{name}` to słowo kluczowe, nie może być nazwą parametru")));
            }
            let list = self.lst();
            if v.iter().any(|p| p.name == name && p.list == list) {
                return Err(Blad::new(line, col, format!("parametr `{name}` już jest")));
            }
            v.push(Param { name, kind, list, line, col });
            if !self.eat(&Tok::Comma) && *self.peek() != Tok::RParen && !self.is_ident("use") {
                return self.err("parametry oddzielamy przecinkiem, np. `fn dodaj (a, b, use => std::io)`");
            }
        }
        Ok(v)
    }

    // po `nazwa(`: 5, &x, &_lista.lst)
    fn args(&mut self) -> Result<Vec<Arg>, Blad> {
        let mut v = Vec::new();
        while !self.eat(&Tok::RParen) {
            let kind = self.kind();
            let e = if kind == Kind::Val {
                self.expr()?
            } else {
                let (line, col) = self.pos();
                let name = self.ident("nazwa zmiennej (pożyczyć można tylko zmienną)")?;
                let ex = if self.lst() { Ex::Lst(name) } else { Ex::Var(self.obj_num(name)) };
                E { ex, line, col }
            };
            v.push(Arg { kind, e });
            if !self.eat(&Tok::Comma) && *self.peek() != Tok::RParen {
                return self.err("argumenty oddzielamy przecinkiem, a na końcu jest `)`");
            }
        }
        Ok(v)
    }

    fn kind(&mut self) -> Kind {
        if self.eat(&Tok::Amp) {
            Kind::Read
        } else if self.eat(&Tok::Mut) {
            Kind::Write
        } else {
            Kind::Val
        }
    }

    // .lst po nazwie
    fn lst(&mut self) -> bool {
        let ok = *self.peek() == Tok::Dot && matches!(self.peek_at(1), Tok::Ident(s) if s == "lst");
        if ok {
            self.next();
            self.next();
        }
        ok
    }

    fn uses(&mut self) -> Result<Vec<String>, Blad> {
        let mut uses = Vec::new();
        if !self.is_ident("use") {
            return Ok(uses);
        }
        self.next();
        self.expect(Tok::Arrow, "`=>` (np. `use => std::io`)")?;
        while *self.peek() != Tok::RParen {
            let mut path = self.ident("ścieżka biblioteki, np. `std::io`")?;
            while self.eat(&Tok::Path) {
                path += "::";
                path += &self.ident("nazwa po `::`")?;
            }
            uses.push(path);
            if !self.eat(&Tok::Comma) && !matches!(self.peek(), Tok::Ident(_) | Tok::RParen) {
                return self.err("w `use` piszemy ścieżki jak `std::io`, oddzielone spacją albo przecinkiem");
            }
        }
        Ok(uses)
    }

    // np. pętle bez `use => std::loops` to błąd
    fn needs(&self, lib: &str, what: &str) -> Result<(), Blad> {
        match self.lacks(lib, what) {
            Some(m) => self.err(m),
            None => Ok(()),
        }
    }

    fn lacks(&self, lib: &str, what: &str) -> Option<String> {
        if self.uses.iter().any(|u| u == lib) {
            return None;
        }
        let old = lib.replace("std::", "sys::");
        Some(if self.uses.contains(&old) {
            format!("`{old}` to teraz `{lib}`")
        } else {
            format!("{what} wymagają `use => {lib}` w nawiasie funkcji")
        })
    }

    fn peek(&self) -> &Tok {
        &self.t[self.p].tok
    }
    fn peek_at(&self, k: usize) -> &Tok {
        &self.t[(self.p + k).min(self.t.len() - 1)].tok
    }
    fn pos(&self) -> (usize, usize) {
        (self.t[self.p].line, self.t[self.p].col)
    }
    fn next(&mut self) -> Token {
        let t = self.t[self.p].clone();
        if self.p < self.t.len() - 1 {
            self.p += 1;
        }
        t
    }
    fn err<T>(&self, m: impl Into<String>) -> Result<T, Blad> {
        let (l, c) = self.pos();
        Err(Blad::new(l, c, m))
    }
    fn eat(&mut self, t: &Tok) -> bool {
        let ok = self.peek() == t;
        if ok {
            self.next();
        }
        ok
    }
    fn expect(&mut self, t: Tok, what: &str) -> Result<(), Blad> {
        if self.eat(&t) { Ok(()) } else { self.err(format!("tu powinno być {what}")) }
    }
    fn is_ident(&self, s: &str) -> bool {
        matches!(self.peek(), Tok::Ident(x) if x == s)
    }
    fn ident(&mut self, what: &str) -> Result<String, Blad> {
        match self.peek().clone() {
            Tok::Ident(s) => {
                self.next();
                Ok(s)
            }
            _ => self.err(format!("tu powinna być {what}")),
        }
    }

    fn block(&mut self) -> Result<Vec<St>, Blad> {
        self.expect(Tok::LBrace, "`{`")?;
        let mut v = Vec::new();
        while !self.eat(&Tok::RBrace) {
            if *self.peek() == Tok::Eof {
                return self.err("brakuje zamykającego `}`");
            }
            v.push(self.stmt()?);
        }
        Ok(v)
    }

    fn stmt(&mut self) -> Result<St, Blad> {
        let (line, col) = self.pos();
        let Tok::Ident(w) = self.peek().clone() else {
            return self.err("tu powinna zacząć się instrukcja (let, if, loop, sys::out, ...)");
        };
        match w.as_str() {
            "loop" if *self.peek_at(1) == Tok::Comma => {
                self.next();
                self.next();
                if !self.is_ident("end") {
                    return self.err("wyjście z pętli to `loop,end`");
                }
                self.next();
                Ok(St::Break(line, col))
            }
            "let" => {
                self.next();
                if *self.peek() == Tok::Amp {
                    return self.err("zmienną do zmieniania piszesz `let &_x`; samo `&x` to pożyczka w wywołaniu funkcji");
                }
                let mutable = self.eat(&Tok::Mut);
                let (line, col) = self.pos();
                let name = self.ident("nazwa zmiennej")?;
                if KEYWORDS.contains(&name.as_str()) {
                    return Err(Blad::new(line, col, format!("`{name}` to słowo kluczowe, nie może być nazwą zmiennej")));
                }
                let full = self.obj_num(name.clone());
                if full != name {
                    if *self.peek() == Tok::Colon {
                        return self.err(format!("obiekt z szablonu tworzysz bez wartości: `let &_{full}`"));
                    }
                    return Ok(St::New { name: full, tpl: name, mutable, line, col });
                }
                if *self.peek() == Tok::Eq {
                    return self.err("w Zolt przypisujemy dwukropkiem: `let x : 12` (`=` porównuje)");
                }
                self.expect(Tok::Colon, "`:` i wartość, np. `let x : 12`")?;
                Ok(St::Let { name, mutable, val: self.expr()?, line, col })
            }
            "if" => self.if_(),
            // loop.inf.1 : ... to przypisanie, a nie pętla
            "loop" if !(matches!(self.peek_at(2), Tok::Ident(s) if s == "inf") && *self.peek_at(3) == Tok::Dot) => {
                self.needs("std::loops", "pętle")?;
                self.next();
                let lp = self.loop_kind()?;
                Ok(St::Loop(lp, self.block()?))
            }
            "else" | "elseif" => self.err(format!("`{w}` bez `if` przed nim")),
            "while" => {
                self.next();
                let cond = self.expr()?;
                Ok(St::While(cond, self.block()?))
            }
            "sys" | "std" => self.sys_call(),
            // wartość tylko w tej samej linii, bo `retrn` może też stać sam
            "retrn" => {
                self.next();
                if self.t[self.p].line != line || matches!(self.peek(), Tok::RBrace | Tok::Eof) {
                    return Ok(St::Ret(None, line, col));
                }
                self.rets = true;
                Ok(St::Ret(Some(self.expr()?), line, col))
            }
            _ if *self.peek_at(1) == Tok::LParen => {
                self.next();
                self.next();
                Ok(St::Call(E { ex: Ex::Call(w, self.args()?), line, col }))
            }
            _ if *self.peek_at(1) == Tok::Dot && matches!(self.peek_at(2), Tok::Ident(s) if s == "lst") => self.list_stmt(),
            _ => {
                let target = self.primary()?;
                if matches!(target.ex, Ex::MCall(..)) {
                    return Ok(St::Call(target));
                }
                if !matches!(target.ex, Ex::Var(_) | Ex::Inf(_) | Ex::Field(..)) {
                    return Err(Blad::new(line, col, "tu powinna zacząć się instrukcja (let, if, loop, sys::out, ...)"));
                }
                if *self.peek() == Tok::Eq {
                    return self.err("zmianę wartości piszemy `x : nowa` (`=` porównuje)");
                }
                self.expect(Tok::Colon, "`:` (zmiana wartości: `x : 5`)")?;
                Ok(St::Set { target, val: self.expr()? })
            }
        }
    }

    fn list_stmt(&mut self) -> Result<St, Blad> {
        self.needs("std::lists", "listy")?;
        let (line, col) = self.pos();
        let name = self.ident("nazwa listy")?;
        self.next(); // .
        self.next(); // lst
        if self.eat(&Tok::Colon) {
            return Ok(St::ListSet { name, val: self.expr()? });
        }
        self.expect(Tok::Dot, "`:` albo `.add` / `.del` / `.chnm`")?;
        let (ml, mc) = self.pos();
        let m = self.ident("`add`, `del` albo `chnm`")?;
        if !["add", "del", "chnm"].contains(&m.as_str()) {
            return Err(Blad::new(ml, mc, format!("listy nie mają `.{m}` (są: add, del, chnm)")));
        }
        let a = self.expr()?;
        let op = match m.as_str() {
            "add" => ListOp::Add(a),
            "del" => ListOp::Del(a),
            _ => {
                self.expect(Tok::RArrow, "`->` (np. `.chnm \"stare\" -> \"nowe\"`)")?;
                ListOp::Chnm(a, self.expr()?)
            }
        };
        Ok(St::ListOp { name, op, line, col })
    }

    fn sys_call(&mut self) -> Result<St, Blad> {
        self.needs("std::io", "`sys::out` i `sys::read`")?;
        let ns = self.ident("")?;
        self.expect(Tok::Path, "`::`")?;
        let (l, c) = self.pos();
        let f = self.ident("nazwa funkcji, np. `out`")?;
        if ns != "sys" {
            return Err(Blad::new(l, c, format!("nie znam `{ns}::{f}`, czy chodziło o `sys::{f}`?")));
        }
        if f != "out" && f != "read" {
            return Err(Blad::new(l, c, format!("nie znam `sys::{f}` (są: sys::out, sys::read)")));
        }
        self.expect(Tok::LParen, "`(`")?;
        let arg = self.expr()?;
        self.expect(Tok::RParen, "`)`")?;
        if f == "out" {
            Ok(St::Out(arg))
        } else {
            self.reads += 1;
            Ok(St::Read(arg, self.reads))
        }
    }

    // po `loop`: .3.times / .n.times / .inf
    fn loop_kind(&mut self) -> Result<LoopKind, Blad> {
        self.expect(Tok::Dot, "`.` (np. `loop.3.times` albo `loop.inf`)")?;
        if self.is_ident("inf") {
            self.next();
            self.infs += 1;
            return Ok(LoopKind::Inf(self.infs));
        }
        let (line, col) = self.pos();
        let ex = match self.next().tok {
            Tok::Int(v, None) => Ex::Int(v, None),
            Tok::Ident(s) => Ex::Var(s),
            _ => return Err(Blad::new(line, col, "po `loop.` powinna być liczba albo `inf`")),
        };
        self.expect(Tok::Dot, "`.times`")?;
        if !self.is_ident("times") {
            return self.err("tu powinno być `times` (np. `loop.3.times`)");
        }
        self.next();
        Ok(LoopKind::Times(E { ex, line, col }))
    }

    fn opt_loop(&mut self) -> Result<Option<LoopKind>, Blad> {
        if !self.is_ident("loop") {
            return Ok(None);
        }
        self.needs("std::loops", "pętle")?;
        self.next();
        Ok(Some(self.loop_kind()?))
    }

    fn if_(&mut self) -> Result<St, Blad> {
        self.next();
        let mut arms = Vec::new();
        loop {
            let cond = self.expr()?;
            let lp = self.opt_loop()?;
            arms.push(Arm { cond: Some(cond), lp, body: self.block()? });
            if self.is_ident("elseif") {
                self.next();
                if *self.peek() == Tok::LBrace {
                    return self.err("`elseif` potrzebuje warunku; bez warunku pisz `else`");
                }
                continue;
            }
            if self.is_ident("else") {
                self.next();
                if self.is_ident("if") {
                    return self.err("w Zolt `else if` piszemy razem: `elseif`");
                }
                let lp = self.opt_loop()?;
                arms.push(Arm { cond: None, lp, body: self.block()? });
            }
            return Ok(St::If(arms));
        }
    }

    // porównanie < <>op<> < + - < * / % < minus < reszta
    fn expr(&mut self) -> Result<E, Blad> {
        let l = self.dyn_()?;
        if self.is_ident("has") {
            let (line, col) = self.pos();
            self.next();
            let r = self.dyn_()?;
            return Ok(E { ex: Ex::Has(Box::new(l), Box::new(r)), line, col });
        }
        let op = match self.peek() {
            Tok::Eq => Op::Eq,
            Tok::Lt => Op::Lt,
            Tok::Gt => Op::Gt,
            Tok::Le => Op::Le,
            Tok::Ge => Op::Ge,
            _ => return Ok(l),
        };
        let (line, col) = self.pos();
        self.next();
        let r = self.dyn_()?;
        Ok(E { ex: Ex::Bin(op, Box::new(l), Box::new(r)), line, col })
    }

    fn dyn_(&mut self) -> Result<E, Blad> {
        let mut l = self.add()?;
        while *self.peek() == Tok::Dyn {
            let (line, col) = self.pos();
            self.next();
            let op = self.primary()?;
            self.expect(Tok::Dyn, "zamykające `<>` (np. `a <>sys::read2<> b`)")?;
            let r = self.add()?;
            l = E { ex: Ex::Dyn(Box::new(l), Box::new(op), Box::new(r)), line, col };
        }
        Ok(l)
    }

    fn add(&mut self) -> Result<E, Blad> {
        self.bin(&[(Tok::Plus, Op::Add), (Tok::Minus, Op::Sub)], Self::mul)
    }

    fn mul(&mut self) -> Result<E, Blad> {
        self.bin(&[(Tok::Star, Op::Mul), (Tok::Slash, Op::Div), (Tok::Percent, Op::Mod)], Self::unary)
    }

    fn bin(&mut self, ops: &[(Tok, Op)], next: fn(&mut Self) -> Result<E, Blad>) -> Result<E, Blad> {
        let mut l = next(self)?;
        while let Some(&(_, op)) = ops.iter().find(|(t, _)| t == self.peek()) {
            let (line, col) = self.pos();
            self.next();
            let r = next(self)?;
            l = E { ex: Ex::Bin(op, Box::new(l), Box::new(r)), line, col };
        }
        Ok(l)
    }

    fn unary(&mut self) -> Result<E, Blad> {
        if *self.peek() != Tok::Minus {
            return self.primary();
        }
        let (line, col) = self.pos();
        self.next();
        Ok(E { ex: Ex::Neg(Box::new(self.unary()?)), line, col })
    }

    fn primary(&mut self) -> Result<E, Blad> {
        let (line, col) = self.pos();
        let mk = |ex| Ok::<E, Blad>(E { ex, line, col });
        match self.next().tok {
            Tok::Int(v, b) => mk(Ex::Int(v, b)),
            Tok::Float(t, b) => mk(Ex::Float(t, b)),
            Tok::List(v) => mk(Ex::ListLit(v)),
            Tok::Str(parts) => {
                let mut v = Vec::new();
                for p in parts {
                    v.push(match p {
                        StrPart::Lit(s) => Part::Lit(s),
                        StrPart::Code(code, l, c) => Part::Code(self.sub(&code, l, c)?),
                    });
                }
                mk(Ex::Str(v))
            }
            Tok::LParen => {
                let e = self.expr()?;
                self.expect(Tok::RParen, "`)`")?;
                Ok(e)
            }
            Tok::Ident(s) => match s.as_str() {
                "true" => mk(Ex::Bool(true)),
                "false" => mk(Ex::Bool(false)),
                "b" if *self.peek() == Tok::Dot && matches!(self.peek_at(1), Tok::Ident(v) if v == "true" || v == "false") => {
                    self.next();
                    mk(Ex::Bool(matches!(self.next().tok, Tok::Ident(v) if v == "true")))
                }
                _ if *self.peek() == Tok::Dot && matches!(self.peek_at(1), Tok::Ident(v) if v == "lst") => {
                    self.needs("std::lists", "listy")?;
                    self.next();
                    self.next();
                    mk(Ex::Lst(s.clone()))
                }
                "sys" => {
                    self.expect(Tok::Path, "`::`")?;
                    let (l, c) = self.pos();
                    let f = self.ident("np. `read1`")?;
                    let Some(n) = f.strip_prefix("read").and_then(|n| n.parse::<usize>().ok()).filter(|&n| n >= 1) else {
                        return Err(Blad::new(l, c, format!("w wartości z `sys::` są tylko odczyty `sys::read1`, `sys::read2`, ... (a nie `sys::{f}`)")));
                    };
                    if n > self.reads {
                        return Err(Blad::new(l, c, format!("`sys::read{n}` jeszcze nie istnieje, do tej pory było {} `sys::read(...)`", self.reads)));
                    }
                    mk(Ex::Read(n))
                }
                "loop" => {
                    self.expect(Tok::Dot, "`.`")?;
                    if !self.is_ident("inf") {
                        return self.err("jako wartość znam tylko licznik `loop.inf.1`, `loop.inf.2`, ...");
                    }
                    self.next();
                    self.expect(Tok::Dot, "`.` i numer pętli, np. `loop.inf.1`")?;
                    let (l, c) = self.pos();
                    let Tok::Int(n, None) = self.next().tok else {
                        return Err(Blad::new(l, c, "tu powinien być numer pętli, np. `loop.inf.1`"));
                    };
                    let n = n as usize;
                    if n == 0 || n > self.infs {
                        return Err(Blad::new(l, c, format!("`loop.inf.{n}` nie istnieje, do tej pory było {} `loop.inf`", self.infs)));
                    }
                    mk(Ex::Inf(n))
                }
                k if KEYWORDS.contains(&k) => Err(Blad::new(line, col, format!("`{k}` to słowo kluczowe, tu powinna być wartość"))),
                _ if *self.peek() == Tok::LParen => {
                    self.next();
                    mk(Ex::Call(s.clone(), self.args()?))
                }
                _ => {
                    let name = self.obj_num(s.clone());
                    self.postfix(E { ex: Ex::Var(name), line, col })
                }
            },
            Tok::Amp | Tok::Mut => Err(Blad::new(line, col, "pożyczkę (`&x`, `&_x`) dajemy tylko jako argument funkcji, np. `dodaj(&_x)`")),
            Tok::Eof => Err(Blad::new(line, col, "nagły koniec pliku")),
            _ => Err(Blad::new(line, col, "tu powinna być wartość (liczba, tekst, zmienna)")),
        }
    }

    // wstawka "<...>" w tekście
    fn sub(&mut self, code: &str, line: usize, col: usize) -> Result<E, Blad> {
        let mut s =
            Parser { t: lex_at(code, line, col)?, p: 0, reads: self.reads, infs: self.infs, uses: self.uses.clone(), rets: false };
        let e = s.expr()?;
        if *s.peek() != Tok::Eof {
            return s.err("w `<...>` może być tylko jedna wartość, np. `<oki>`");
        }
        Ok(e)
    }
}
