/* This program is free software. It comes without any warranty, to
 * the extent permitted by applicable law. You can redistribute it
 * and/or modify it under the terms of the Do What The Fuck You Want
 * To Public License, Version 2, as published by Sam Hocevar. See
 * http://www.wtfpl.net/ for more details. */

use crate::Blad;
use crate::lexer::Item;
use crate::parser::{Arg, E, Ex, Kind, ListOp, LoopKind, Op, Part, Program, St};

#[derive(Debug, Clone, Copy, PartialEq)]
enum Ty {
    Int(u32),   // c1..c128
    Float(u32), // c32, c64
    Text,
    Bool,
    List,
    Obj(usize), // obiekt z szablonu nr ...
}

impl Ty {
    fn c(self) -> String {
        match self {
            Ty::Int(1) => "unsigned _BitInt(1)".into(),
            Ty::Int(64) => "int64_t".into(),
            Ty::Int(n) => format!("_BitInt({n})"),
            Ty::Float(32) => "float".into(),
            Ty::Float(_) => "double".into(),
            Ty::Text => "const char *".into(),
            Ty::Bool => "bool".into(),
            Ty::List => "zt_lst".into(),
            Ty::Obj(i) => format!("zt_s{i}"),
        }
    }
    fn name(self) -> String {
        match self {
            Ty::Int(n) => format!("liczba c{n}"),
            Ty::Float(n) => format!("ułamek c{n}"),
            Ty::Text => "tekst".into(),
            Ty::Bool => "prawda/fałsz".into(),
            Ty::List => "lista".into(),
            Ty::Obj(_) => "obiekt".into(),
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Ret {
    Void,
    Unknown, // jest `retrn wartość`, ale typ jeszcze nieznany (rekurencja)
    Val(Ty),
}

impl Ret {
    fn c(self) -> String {
        match self {
            Ret::Val(t) => t.c(),
            _ => "void".into(),
        }
    }
}

struct Var {
    ty: Ty,
    mutable: bool,
    c: String,
    field: bool, // pole obiektu widziane w metodzie
}

// jedna wersja funkcji dla jednego zestawu typów argumentów
struct Inst {
    f: usize,
    tys: Vec<Ty>,
    self_mut: Option<bool>, // metoda: czy obiekt można zmieniać
    cn: String,
    sig: String,
    ret: Ret,
    body: String,
}

struct Ctx<'p> {
    p: &'p Program,
    inst: Vec<Inst>,
    tpls: Vec<Vec<(String, Ty, String)>>, // pola szablonów: nazwa, typ, wartość domyślna w C
}

struct Gen<'a, 'p> {
    cx: &'a mut Ctx<'p>,
    me: usize, // indeks w cx.inst
    out: String,
    ind: usize,
    scopes: Vec<Vec<(String, Var)>>,
    n: usize,
    depth: usize, // ile pętli wokół (dla loop,end)
}

pub fn generate(p: &Program) -> Result<String, Blad> {
    let mut cx = Ctx { p, inst: Vec::new(), tpls: Vec::new() };
    for t in &p.tpls {
        let mut g = Gen { cx: &mut cx, me: 0, out: String::new(), ind: 1, scopes: vec![vec![]], n: 0, depth: 0 };
        let mut fs = Vec::new();
        for (name, e) in &t.fields {
            let (c, ty) = g.expr(e)?;
            if matches!(ty, Ty::List | Ty::Obj(_)) {
                return Err(e.err("w szablonie są na razie tylko liczby, tekst i prawda/fałsz"));
            }
            fs.push((name.clone(), ty, c));
        }
        cx.tpls.push(fs);
    }
    let m = p.funcs.iter().position(|f| f.name == "main").unwrap();
    let mi = instance(&mut cx, m, vec![], None)?;
    // funkcje bez parametrów sprawdzamy, nawet jeśli nikt ich nie woła
    for (i, f) in p.funcs.iter().enumerate() {
        if f.params.is_empty() {
            instance(&mut cx, i, vec![], f.owner.map(|_| true))?;
        }
    }
    let mut fns = String::new();
    if cx.inst.len() > 1 {
        fns += "\n";
        for x in &cx.inst {
            fns += &format!("static {} {}({});\n", x.ret.c(), x.cn, x.sig);
        }
    }
    for x in &cx.inst {
        let f = &p.funcs[x.f];
        fns += &format!("\n/* fn {}, use => {} */\n", f.name, f.uses.join(", "));
        fns += &format!("static {} {}({}) {{\n{}}}\n", x.ret.c(), x.cn, x.sig, x.body);
    }
    let mut tpl = String::new();
    for (i, fs) in cx.tpls.iter().enumerate() {
        let decl: Vec<String> = fs.iter().enumerate().map(|(k, (n, t, _))| format!("{} f{k}_{};", t.c(), ascii(n))).collect();
        tpl += &format!("\n/* szablon {} */\ntypedef struct {{ {} }} zt_s{i};\n", p.tpls[i].name, decl.join(" "));
        if !fns.contains(&format!("zt_s{i}_print(")) {
            continue;
        }
        tpl += &format!("static void zt_s{i}_print(FILE *f, zt_s{i} v) {{\n    putc('{{', f);\n");
        for (k, (n, t, _)) in fs.iter().enumerate() {
            let label = cstr(&format!("{}{n}: ", if k > 0 { ", " } else { "" }));
            tpl += &format!("    fputs({label}, f);\n    {}\n", print_c(&format!("v.f{k}_{}", ascii(n)), *t, "f"));
        }
        tpl += "    putc('}', f);\n}\n";
    }
    let rt = runtime(&(tpl.clone() + &fns));
    let main = if rt.contains("zt_argc") {
        format!("int main(int argc, char **argv) {{\n    zt_argc = argc;\n    zt_argv = argv;\n    {}();\n    return 0;\n}}\n", cx.inst[mi].cn)
    } else {
        format!("int main(void) {{\n    {}();\n    return 0;\n}}\n", cx.inst[mi].cn)
    };
    let parts: Vec<&str> = [rt.as_str(), &tpl, &fns, &main].into_iter().map(str::trim).filter(|s| !s.is_empty()).collect();
    let body = parts.join("\n\n");
    Ok(format!("/* wygenerowane przez zolt */\n{}\n{body}\n", naglowki(&body)))
}

// z rt.c bierzemy tylko kawałki, których program używa; kawałek zaczyna linia `/*@ nazwy */`
// i trafia do programu, gdy któraś z tych nazw jest w kodzie (też w innym wziętym kawałku)
fn runtime(kod: &str) -> String {
    let mut parts: Vec<(Vec<&str>, String)> = Vec::new();
    for l in include_str!("rt.c").lines() {
        if let Some(m) = l.strip_prefix("/*@").and_then(|m| m.strip_suffix("*/")) {
            parts.push((m.split_whitespace().collect(), String::new()));
        } else if let Some((_, s)) = parts.last_mut() {
            *s += l;
            s.push('\n');
        }
    }
    let mut used = vec![false; parts.len()];
    let mut text = kod.to_string();
    let mut more = true;
    while more {
        more = false;
        for (k, (names, s)) in parts.iter().enumerate() {
            if !used[k] && names.iter().any(|n| text.contains(n)) {
                used[k] = true;
                text += s;
                more = true;
            }
        }
    }
    parts.iter().zip(used).filter(|(_, u)| *u).map(|((_, s), _)| s.as_str()).collect()
}

// tylko nagłówki, z których kod coś bierze (bool, true i false są w C23 bez nagłówka)
fn naglowki(kod: &str) -> String {
    const H: &[(&str, &[&str])] = &[
        ("math.h", &["fmod("]),
        ("stdint.h", &["int64_t", "INT64_C"]),
        ("stdio.h", &["printf(", "puts(", "putc(", "putchar(", "FILE", "getline("]),
        ("stdlib.h", &["exit(", "malloc(", "realloc(", "free(", "strtod("]),
        ("string.h", &["strcmp(", "strlen(", "memcpy(", "memmove("]),
    ];
    H.iter().filter(|(_, n)| n.iter().any(|n| kod.contains(n))).map(|(h, _)| format!("#include <{h}>\n")).collect()
}

// funkcja bez typów w parametrach: typy biorą się z argumentów przy wywołaniu
fn instance(cx: &mut Ctx, fi: usize, tys: Vec<Ty>, self_mut: Option<bool>) -> Result<usize, Blad> {
    if let Some(i) = cx.inst.iter().position(|x| x.f == fi && x.tys == tys && x.self_mut == self_mut) {
        return Ok(i);
    }
    let p = cx.p;
    let f = &p.funcs[fi];
    let k = cx.inst.iter().filter(|x| x.f == fi).count();
    let mut cn = format!("f{fi}_{}", ascii(&f.name));
    if k > 0 {
        cn += &format!("_{k}");
    }
    let me = cx.inst.len();
    let ret = if f.rets { Ret::Unknown } else { Ret::Void };
    let mut sig = Vec::new();
    let mut scope = Vec::new();
    // metoda: pierwszy parametr to obiekt, a jego pola widać po samej nazwie
    if let (Some(ti), Some(m)) = (f.owner, self_mut) {
        sig.push(format!("zt_s{ti} {}*self", if m { "" } else { "const " }));
        for (k, (n, t, _)) in cx.tpls[ti].iter().enumerate() {
            scope.push((n.clone(), Var { ty: *t, mutable: m, c: format!("self->f{k}_{}", ascii(n)), field: true }));
        }
    }
    for (i, (pa, &t)) in f.params.iter().zip(&tys).enumerate() {
        let pc = format!("p{i}_{}", ascii(&pa.name));
        let (decl, c, mutable) = match pa.kind {
            Kind::Val => (format!("{} {pc}", t.c()), pc, t == Ty::List),
            Kind::Read => (format!("{} const *{pc}", t.c()), format!("(*{pc})"), false),
            Kind::Write => (format!("{} *{pc}", t.c()), format!("(*{pc})"), true),
        };
        sig.push(decl);
        let key = if pa.list { format!("{}.lst", pa.name) } else { pa.name.clone() };
        scope.push((key, Var { ty: t, mutable, c, field: false }));
    }
    let sig = if sig.is_empty() { "void".into() } else { sig.join(", ") };
    cx.inst.push(Inst { f: fi, tys, self_mut, cn, sig, ret, body: String::new() });
    let mut g = Gen { cx, me, out: String::new(), ind: 1, scopes: vec![scope], n: 0, depth: 0 };
    g.stmts(&f.body)?;
    let mut body = String::new();
    if f.reads > 0 {
        body += &format!("    const char *zt_rd[{}] = {{0}};\n", f.reads + 1);
    }
    for k in 1..=f.infs {
        body += &format!("    int64_t zt_inf{k} = 0;\n");
    }
    body += &g.out;
    if let Ret::Val(_) = g.cx.inst[me].ret {
        body += &format!("    zt_noret({});\n", cstr(&f.name));
    }
    g.cx.inst[me].body = body;
    Ok(me)
}

impl Gen<'_, '_> {
    fn line(&mut self, s: &str) {
        self.out += &"    ".repeat(self.ind);
        self.out += s;
        self.out += "\n";
    }

    fn find(&self, name: &str) -> Option<&Var> {
        self.scopes.iter().rev().find_map(|s| s.iter().rev().find(|(n, _)| n == name).map(|(_, v)| v))
    }

    fn unknown(&self, n: &str) -> String {
        let mut best: Option<(usize, &str)> = None;
        for (name, _) in self.scopes.iter().flatten() {
            let d = if name.to_lowercase() == n.to_lowercase() { 0 } else { lev(name, n) };
            if d <= 2 && best.is_none_or(|(bd, _)| d < bd) {
                best = Some((d, name));
            }
        }
        match best {
            Some((_, b)) => format!("nie znam zmiennej `{n}`, czy chodziło o `{b}`?"),
            None => format!("nie znam zmiennej `{n}`"),
        }
    }

    fn stmts(&mut self, v: &[St]) -> Result<(), Blad> {
        v.iter().try_for_each(|s| self.stmt(s))
    }

    fn block(&mut self, v: &[St]) -> Result<(), Blad> {
        self.scopes.push(vec![]);
        self.stmts(v)?;
        self.scopes.pop();
        Ok(())
    }

    fn stmt(&mut self, s: &St) -> Result<(), Blad> {
        match s {
            St::Let { name, mutable, val, line, col } => {
                if self.scopes.last().unwrap().iter().any(|(n, _)| n == name) {
                    return Err(Blad::new(*line, *col, format!("`{name}` już istnieje w tym bloku")));
                }
                let (c, ty) = self.expr(val)?;
                if ty == Ty::List {
                    return Err(val.err("listę tworzysz bez `let`: `nazwa.lst : [a, b, c]`"));
                }
                self.n += 1;
                let cn = format!("v{}_{}", self.n, ascii(name));
                self.line(&format!("{} {cn} = {c};", ty.c()));
                self.scopes.last_mut().unwrap().push((name.clone(), Var { ty, mutable: *mutable, c: cn, field: false }));
            }
            St::Set { target, val } => {
                let (tc, tt) = match &target.ex {
                    Ex::Var(n) => match self.find(n) {
                        None => return Err(target.err(self.unknown(n))),
                        Some(v) if !v.mutable && v.field => {
                            return Err(target.err(format!(
                                "metoda zmienia pole `{n}`, a obiekt nie jest zmienny; utwórz go przez `let &_nazwa.1`"
                            )));
                        }
                        Some(v) if !v.mutable => {
                            return Err(target.err(format!("`{n}` jest stała; żeby ją zmieniać, zadeklaruj ją jako `let &_{n}`")));
                        }
                        Some(v) => (v.c.clone(), v.ty),
                    },
                    Ex::Field(b, _) => {
                        if let Ex::Var(bn) = &b.ex
                            && let Some(v) = self.find(bn)
                            && !v.mutable
                        {
                            return Err(target.err(format!("`{bn}` jest stały; żeby zmieniać jego pola, utwórz go przez `let &_{bn}`")));
                        }
                        self.expr(target)?
                    }
                    _ => self.expr(target)?,
                };
                let (vc, vt) = self.expr(val)?;
                if !fits(tt, vt) {
                    return Err(val.err(format!("tu jest {}, a do zmiennej pasuje tylko {}", vt.name(), tt.name())));
                }
                self.line(&format!("{tc} = {vc};"));
            }
            St::Out(e) => self.print(e, "stdout", '\n')?,
            St::Err(e) => {
                self.line("fflush(stdout);");
                self.print(e, "stderr", '\n')?;
                self.line("exit(1);");
            }
            St::Read(e, k) => {
                self.print(e, "stdout", ' ')?;
                self.line(&format!("zt_rd[{k}] = zt_readline();"));
            }
            St::Loop(lp, body) => self.looped(lp, body)?,
            St::While(c, body) => {
                let cc = self.cond(c)?;
                self.line(&format!("while ({cc}) {{"));
                self.inner(body)?;
                self.line("}");
            }
            St::Break(l, c) => {
                if self.depth == 0 {
                    return Err(Blad::new(*l, *c, "`loop,end` działa tylko w pętli"));
                }
                self.line("break;");
            }
            St::Call(e) => {
                let (c, _) = self.any_call(e)?;
                self.line(&format!("{c};"));
            }
            St::New { name, tpl, mutable, line, col } => {
                let p = self.cx.p;
                let Some(ti) = p.tpls.iter().position(|t| t.name == *tpl) else {
                    return Err(Blad::new(*line, *col, format!("nie ma szablonu `{tpl}` (robisz go tak: `{tpl} : {{ hp : 30 }}`)")));
                };
                if self.scopes.last().unwrap().iter().any(|(n, _)| n == name) {
                    return Err(Blad::new(*line, *col, format!("`{name}` już istnieje w tym bloku")));
                }
                let Some(fs) = self.cx.tpls.get(ti) else {
                    return Err(Blad::new(*line, *col, format!("szablon `{tpl}` jest jeszcze w budowie (wartość domyślna innego szablonu go używa)")));
                };
                let init: Vec<String> = fs.iter().enumerate().map(|(k, (n, _, c))| format!(".f{k}_{} = {c}", ascii(n))).collect();
                self.n += 1;
                let cn = format!("v{}_{}", self.n, ascii(name));
                self.line(&format!("zt_s{ti} {cn} = {{{}}};", init.join(", ")));
                self.scopes.last_mut().unwrap().push((name.clone(), Var { ty: Ty::Obj(ti), mutable: *mutable, c: cn, field: false }));
            }
            St::Ret(v, l, c) => {
                let p = self.cx.p;
                let f = &p.funcs[self.cx.inst[self.me].f];
                let Some(e) = v else {
                    if f.rets {
                        return Err(Blad::new(*l, *c, format!("`{}` zwraca wartość, więc tu też: `retrn wartość`", f.name)));
                    }
                    self.line("return;");
                    return Ok(());
                };
                if f.name == "main" {
                    return Err(e.err("`main` nic nie zwraca; samo `retrn` kończy program"));
                }
                let (vc, vt) = self.expr(e)?;
                match self.cx.inst[self.me].ret {
                    Ret::Val(rt) if !fits(rt, vt) => {
                        return Err(e.err(format!("`{}` zwraca już {}, a tu jest {}", f.name, rt.name(), vt.name())));
                    }
                    Ret::Val(_) => {}
                    _ => self.cx.inst[self.me].ret = Ret::Val(vt),
                }
                let vc = if vt == Ty::List { fresh(e, vc) } else { vc };
                self.line(&format!("return {vc};"));
            }
            St::ListSet { name, val } => {
                let (vc, vt) = self.expr(val)?;
                if vt != Ty::List {
                    return Err(val.err(format!("do listy pasuje tylko lista `[a, b, c]`, a tu jest {}", vt.name())));
                }
                let vc = fresh(val, vc);
                let key = format!("{name}.lst");
                if let Some(v) = self.find(&key) {
                    if !v.mutable {
                        return Err(val.err(read_only(name)));
                    }
                    let cn = v.c.clone();
                    self.line(&format!("{cn} = {vc};"));
                } else {
                    self.n += 1;
                    let cn = format!("v{}_{}_lst", self.n, ascii(name));
                    self.line(&format!("zt_lst {cn} = {vc};"));
                    self.scopes.last_mut().unwrap().push((key, Var { ty: Ty::List, mutable: true, c: cn, field: false }));
                }
            }
            St::ListOp { name, op, line, col } => {
                let Some(v) = self.find(&format!("{name}.lst")) else {
                    return Err(Blad::new(*line, *col, format!("nie ma listy `{name}.lst`")));
                };
                if !v.mutable {
                    return Err(Blad::new(*line, *col, read_only(name)));
                }
                let cn = v.c.clone();
                let s = match op {
                    ListOp::Add(a) => format!("zt_lst_add(&{cn}, {});", self.val(a)?),
                    ListOp::Del(a) => format!("zt_lst_del(&{cn}, {});", self.val(a)?),
                    ListOp::Chnm(a, b) => format!("zt_lst_chnm(&{cn}, {}, {});", self.val(a)?, self.val(b)?),
                };
                self.line(&s);
            }
            St::If(arms) => {
                for (i, arm) in arms.iter().enumerate() {
                    let head = match &arm.cond {
                        Some(c) => {
                            let cc = self.cond(c)?;
                            format!("{}if ({cc}) {{", if i == 0 { "" } else { "} else " })
                        }
                        None => "} else {".into(),
                    };
                    self.line(&head);
                    self.ind += 1;
                    match &arm.lp {
                        Some(lp) => self.looped(lp, &arm.body)?,
                        None => self.block(&arm.body)?,
                    }
                    self.ind -= 1;
                }
                self.line("}");
            }
        }
        Ok(())
    }

    fn looped(&mut self, lp: &LoopKind, body: &[St]) -> Result<(), Blad> {
        match lp {
            LoopKind::Times(n) => {
                let (nc, nt) = self.expr(n)?;
                if !matches!(nt, Ty::Int(_)) {
                    return Err(n.err(format!("`loop.N.times` potrzebuje liczby całkowitej, a tu jest {}", nt.name())));
                }
                self.n += 1;
                let nc = if nc.bytes().all(|b| b.is_ascii_digit()) { nc } else { format!("(int64_t)({nc})") };
                self.line(&format!("for (int64_t zt_i{0} = 0; zt_i{0} < {1}; zt_i{0}++) {{", self.n, nc));
            }
            LoopKind::Inf(k) => {
                self.line(&format!("zt_inf{k} = 0;"));
                self.line(&format!("for (;; zt_inf{k}++) {{"));
            }
        }
        self.inner(body)?;
        self.line("}");
        Ok(())
    }

    // ciało pętli
    fn inner(&mut self, body: &[St]) -> Result<(), Blad> {
        self.ind += 1;
        self.depth += 1;
        self.block(body)?;
        self.depth -= 1;
        self.ind -= 1;
        Ok(())
    }

    fn cond(&mut self, c: &E) -> Result<String, Blad> {
        let (cc, ct) = self.expr(c)?;
        if ct != Ty::Bool {
            return Err(c.err(format!("warunek musi dawać prawdę/fałsz (np. `x = 12`), a daje: {}", ct.name())));
        }
        Ok(cc)
    }

    // wartość jako element listy
    fn val(&mut self, e: &E) -> Result<String, Blad> {
        let (c, t) = self.expr(e)?;
        Ok(match t {
            Ty::Text => format!("zt_vt({c})"),
            Ty::Bool => format!("zt_vb({c})"),
            Ty::Int(n) if n <= 64 => format!("zt_vi((int64_t)({c}))"),
            Ty::Int(n) => return Err(e.err(format!("c{n} jest za duże do listy (max c64)"))),
            Ty::Float(_) => format!("zt_vf((double)({c}))"),
            Ty::List => return Err(e.err("listy w liście jeszcze nie ma")),
            Ty::Obj(_) => return Err(e.err("obiektu w liście jeszcze nie ma")),
        })
    }

    fn any_call(&mut self, e: &E) -> Result<(String, Ret), Blad> {
        match &e.ex {
            Ex::Call(name, args) => {
                let Some(fi) = self.cx.p.funcs.iter().position(|f| f.owner.is_none() && f.name == *name) else {
                    return Err(e.err(format!("nie ma funkcji `{name}`")));
                };
                self.call(e, fi, args, None)
            }
            Ex::MCall(obj, m, args) => {
                let Ex::Var(on) = &obj.ex else { return Err(obj.err("metody ma tylko obiekt, np. `gracz.1.exp()`")) };
                let Some(v) = self.find(on) else { return Err(obj.err(self.unknown(on))) };
                let Ty::Obj(ti) = v.ty else { return Err(obj.err(format!("`{on}` to nie obiekt z szablonu, więc nie ma metod"))) };
                let me = (on.clone(), format!("&{}", v.c), v.mutable);
                let p = self.cx.p;
                let t = &p.tpls[ti].name;
                let Some(fi) = p.funcs.iter().position(|f| f.owner == Some(ti) && f.name == format!("{t}.{m}")) else {
                    return Err(e.err(format!("szablon `{t}` nie ma metody `{m}`")));
                };
                self.call(e, fi, args, Some(me))
            }
            _ => unreachable!(),
        }
    }

    // me: obiekt metody (nazwa, &obiekt w C, czy zmienny)
    fn call(&mut self, e: &E, fi: usize, args: &[Arg], me: Option<(String, String, bool)>) -> Result<(String, Ret), Blad> {
        let p = self.cx.p;
        let f = &p.funcs[fi];
        let name = &f.name;
        if f.params.len() != args.len() {
            let ps: Vec<String> = f.params.iter().map(|p| p.show()).collect();
            return Err(e.err(format!("`{name}({})`: liczba argumentów ma być {}, a jest {}", ps.join(", "), ps.len(), args.len())));
        }
        let (mut cs, mut tys) = (Vec::new(), Vec::new());
        let mut borrowed: Vec<(String, Kind)> = Vec::new();
        let self_mut = me.map(|(key, c, m)| {
            cs.push(c);
            borrowed.push((key, if m { Kind::Write } else { Kind::Read }));
            m
        });
        for (pa, a) in f.params.iter().zip(args) {
            if pa.kind != a.kind {
                let want = match pa.kind {
                    Kind::Val => "zwykłej wartości (bez `&`)",
                    Kind::Read => "pożyczki do czytania: `&zmienna`",
                    Kind::Write => "pożyczki do zmian: `&_zmienna`",
                };
                return Err(a.e.err(format!("parametr `{}` w `{name}` chce {want}", pa.show())));
            }
            let (c, t) = if a.kind == Kind::Val {
                let (c, t) = self.expr(&a.e)?;
                (if t == Ty::List { fresh(&a.e, c) } else { c }, t)
            } else {
                let key = match &a.e.ex {
                    Ex::Lst(n) => format!("{n}.lst"),
                    Ex::Var(n) => n.clone(),
                    _ => unreachable!(),
                };
                let Some(v) = self.find(&key) else { return Err(a.e.err(self.unknown(&key))) };
                if a.kind == Kind::Write && !v.mutable {
                    return Err(a.e.err(format!("`{key}` jest tylko do czytania, więc nie da się jej pożyczyć do zmian (`&_{key}`)")));
                }
                let (c, t) = (format!("&{}", v.c), v.ty);
                // borrow checker w małym: pożyczka do zmian musi być jedyna
                if let Some((_, k)) = borrowed.iter().find(|(n, _)| *n == key)
                    && (*k == Kind::Write || a.kind == Kind::Write)
                {
                    return Err(a.e.err(format!("`{key}` jest już pożyczona w tym wywołaniu, a pożyczka do zmian (`&_`) musi być jedyna")));
                }
                borrowed.push((key, a.kind));
                (c, t)
            };
            if pa.list != (t == Ty::List) {
                return Err(a.e.err(if pa.list {
                    format!("parametr `{}` to lista, a tu jest {}", pa.show(), t.name())
                } else {
                    format!("parametr `{}` to nie lista (lista ma w nazwie `.lst`)", pa.show())
                }));
            }
            cs.push(c);
            tys.push(t);
        }
        let i = instance(self.cx, fi, tys, self_mut)?;
        Ok((format!("{}({})", self.cx.inst[i].cn, cs.join(", ")), self.cx.inst[i].ret))
    }

    // wypisuje e, a na końcu znak end (doklejony do ostatniego kawałka tekstu, jeśli się da)
    fn print(&mut self, e: &E, f: &str, end: char) -> Result<(), Blad> {
        let parts: &[Part] = if let Ex::Str(p) = &e.ex { p } else { &[] };
        if let [Part::Lit(s)] = parts
            && f == "stdout"
            && end == '\n'
        {
            self.line(&format!("puts({});", cstr(s)));
            return Ok(());
        }
        if parts.is_empty() {
            self.print_val(e, f)?;
        }
        for (k, p) in parts.iter().enumerate() {
            match p {
                Part::Lit(s) if k + 1 == parts.len() => {
                    self.line(&format!("fputs({}, {f});", cstr(&format!("{s}{end}"))));
                    return Ok(());
                }
                Part::Lit(s) => self.line(&format!("fputs({}, {f});", cstr(s))),
                Part::Code(c) => self.print_val(c, f)?,
            }
        }
        let ch = if end == '\n' { "'\\n'".to_string() } else { format!("'{end}'") };
        self.line(&if f == "stdout" { format!("putchar({ch});") } else { format!("putc({ch}, {f});") });
        Ok(())
    }

    fn print_val(&mut self, e: &E, f: &str) -> Result<(), Blad> {
        let (c, t) = self.expr(e)?;
        self.line(&print_c(&c, t, f));
        Ok(())
    }

    fn expr(&mut self, e: &E) -> Result<(String, Ty), Blad> {
        Ok(match &e.ex {
            Ex::Int(v, bits) => {
                let n = bits.unwrap_or(64);
                if n == 0 || n > 128 {
                    return Err(e.err(format!("c{n}: rozmiar musi być od 1 do 128 bitów")));
                }
                let max: u128 = if n == 1 { 1 } else { (1u128 << (n - 1)) - 1 };
                if *v > max {
                    return Err(e.err(format!("{v} nie mieści się w c{n} (max {max})")));
                }
                let t = Ty::Int(n);
                let c = match n {
                    64 => int_lit(*v as i128),
                    ..64 => format!("(({}){})", t.c(), int_lit(*v as i128)),
                    _ => format!("(({}){v}wb)", t.c()),
                };
                (c, t)
            }
            Ex::Float(t, bits) => match bits.unwrap_or(64) {
                32 => (format!("{t}f"), Ty::Float(32)),
                64 => (t.clone(), Ty::Float(64)),
                n => return Err(e.err(format!("liczby z kropką mają na razie tylko c32 albo c64 (a nie c{n})"))),
            },
            Ex::Str(parts) if parts.iter().all(|p| matches!(p, Part::Lit(_))) => {
                let s: String = parts.iter().map(|p| if let Part::Lit(l) = p { l.as_str() } else { "" }).collect();
                (cstr(&s), Ty::Text)
            }
            // tekst ze wstawkami: piszemy go do bufora tak samo jak sys::out na ekran
            Ex::Str(parts) => {
                self.n += 1;
                let k = self.n;
                let f = format!("zt_f{k}");
                let mut s = format!("({{ char *zt_b{k}; size_t zt_n{k}; FILE *{f} = open_memstream(&zt_b{k}, &zt_n{k}); ");
                for p in parts {
                    match p {
                        Part::Lit(l) => s += &format!("fputs({}, {f}); ", cstr(l)),
                        Part::Code(c) => {
                            let (cc, t) = self.expr(c)?;
                            s += &print_c(&cc, t, &f);
                            s += " ";
                        }
                    }
                }
                (s + &format!("fclose({f}); (const char *)zt_b{k}; }})"), Ty::Text)
            }
            Ex::Bool(b) => (b.to_string(), Ty::Bool),
            Ex::Var(n) => match self.find(n) {
                Some(v) => (v.c.clone(), v.ty),
                None => return Err(e.err(self.unknown(n))),
            },
            Ex::Read(k) => (format!("zt_rd[{k}]"), Ty::Text),
            Ex::Arg(k) => (format!("zt_arg({k})"), Ty::Text),
            Ex::File(n) => {
                let (nc, nt) = self.expr(n)?;
                if nt != Ty::Text {
                    return Err(n.err(format!("`sys::file` potrzebuje nazwy pliku (tekstu), a tu jest {}", nt.name())));
                }
                (format!("zt_file({nc})"), Ty::Text)
            }
            Ex::At(b, i) => {
                let (bc, bt) = self.expr(b)?;
                if bt != Ty::Text {
                    return Err(e.err(format!("`.at` działa na tekście, a tu jest {}", bt.name())));
                }
                let (ic, it) = self.expr(i)?;
                if !matches!(it, Ty::Int(_)) {
                    return Err(i.err(format!("po `.at` powinien być numer znaku (liczba całkowita), a jest {}", it.name())));
                }
                (format!("zt_at({bc}, (int64_t)({ic}))"), Ty::Text)
            }
            Ex::Inf(k) => (format!("zt_inf{k}"), Ty::Int(64)),
            Ex::Neg(x) => {
                let (c, t) = self.num(x)?;
                (format!("(-{c})"), t)
            }
            Ex::Bin(op, a, b) => self.bin(*op, a, b)?,
            Ex::Dyn(a, op, b) => {
                let (oc, ot) = self.expr(op)?;
                if ot != Ty::Text {
                    return Err(op.err(format!("między `<>` musi być tekst z działaniem (np. `sys::read2`), a jest {}", ot.name())));
                }
                let (ac, _) = self.num(a)?;
                let (bc, _) = self.num(b)?;
                (format!("zt_dyn((double)({ac}), {oc}, (double)({bc}))"), Ty::Float(64))
            }
            Ex::Lst(n) => match self.find(&format!("{n}.lst")) {
                Some(v) => (v.c.clone(), Ty::List),
                None => return Err(e.err(format!("nie ma listy `{n}.lst`"))),
            },
            Ex::ListLit(items) if items.is_empty() => ("zt_lst_new(0, NULL)".into(), Ty::List),
            Ex::ListLit(items) => {
                let v: Vec<String> = items
                    .iter()
                    .map(|it| match it {
                        Item::Txt(s) => format!("zt_vt({})", cstr(s)),
                        Item::Int(i) => format!("zt_vi({})", int_lit(*i as i128)),
                        Item::Flt(f) => format!("zt_vf({f})"),
                        Item::Bool(b) => format!("zt_vb({b})"),
                    })
                    .collect();
                (format!("zt_lst_new({}, (zt_val[]){{{}}})", v.len(), v.join(", ")), Ty::List)
            }
            Ex::Has(l, x) => {
                let (lc, lt) = self.expr(l)?;
                if lt != Ty::List {
                    return Err(l.err(format!("`has` działa na listach (`x.lst has \"oki\"`), a tu jest {}", lt.name())));
                }
                (format!("zt_lst_has({lc}, {})", self.val(x)?), Ty::Bool)
            }
            Ex::Field(b, f) => {
                let (bc, bt) = self.expr(b)?;
                if bt == Ty::Text {
                    if f != "len" {
                        return Err(e.err(format!("tekst nie ma `.{f}` (ma `.len` i `.at`)")));
                    }
                    return Ok((format!("zt_len({bc})"), Ty::Int(64)));
                }
                let Ty::Obj(ti) = bt else {
                    return Err(b.err(format!("`.{f}` działa na obiektach z szablonu, a tu jest {}", bt.name())));
                };
                let p = self.cx.p;
                let t = &p.tpls[ti].name;
                match self.cx.tpls[ti].iter().position(|(n, _, _)| n == f) {
                    Some(k) => (format!("{bc}.f{k}_{}", ascii(f)), self.cx.tpls[ti][k].1),
                    None if p.funcs.iter().any(|x| x.name == format!("{t}.{f}")) => {
                        return Err(e.err(format!("`{f}` to metoda, wywołaj ją: `.{f}()`")));
                    }
                    None => return Err(e.err(format!("szablon `{t}` nie ma pola `{f}`"))),
                }
            }
            Ex::Call(name, _) | Ex::MCall(_, name, _) => match self.any_call(e)? {
                (c, Ret::Val(t)) => (c, t),
                (_, Ret::Void) => return Err(e.err(format!("`{name}` nic nie zwraca (nie ma w niej `retrn wartość`)"))),
                (_, Ret::Unknown) => {
                    return Err(e.err(format!(
                        "`{name}` woła samą siebie, zanim wiadomo, co zwraca; daj wcześniej `retrn` bez wywołania, np. `if n < 2 {{ retrn n }}`"
                    )));
                }
            },
        })
    }

    fn num(&mut self, e: &E) -> Result<(String, Ty), Blad> {
        let (c, t) = self.expr(e)?;
        to_num(c, t, e)
    }

    fn bin(&mut self, op: Op, a: &E, b: &E) -> Result<(String, Ty), Blad> {
        let sym = match op {
            Op::Add => "+",
            Op::Sub => "-",
            Op::Mul => "*",
            Op::Div => "/",
            Op::Mod => "%",
            Op::Eq => "==",
            Op::Lt => "<",
            Op::Gt => ">",
            Op::Le => "<=",
            Op::Ge => ">=",
        };
        let cmp = matches!(op, Op::Eq | Op::Lt | Op::Gt | Op::Le | Op::Ge);
        let (ac, ta) = self.expr(a)?;
        let (bc, tb) = self.expr(b)?;
        if cmp && ta == Ty::Text && tb == Ty::Text {
            return Ok((format!("(strcmp({ac}, {bc}) {sym} 0)"), Ty::Bool));
        }
        if cmp && (ta == Ty::Bool || tb == Ty::Bool) {
            if ta == tb && op == Op::Eq {
                return Ok((format!("({ac} == {bc})"), Ty::Bool));
            }
            return Err(a.err("prawdę/fałsz można tylko porównać `=` z drugą prawdą/fałszem"));
        }
        let (ac, ta) = to_num(ac, ta, a)?;
        let (bc, tb) = to_num(bc, tb, b)?;
        let t = match (ta, tb) {
            (Ty::Int(x), Ty::Int(y)) => Ty::Int(x.max(y)),
            (Ty::Float(32), Ty::Float(32)) => Ty::Float(32),
            _ => Ty::Float(64),
        };
        let tc = t.c();
        if cmp {
            return Ok((format!("(({tc})({ac}) {sym} ({tc})({bc}))"), Ty::Bool));
        }
        if op == Op::Mod && matches!(t, Ty::Float(_)) {
            return Ok((format!("(({tc})fmod({ac}, {bc}))"), t));
        }
        Ok((format!("(({tc})({ac}) {sym} ({tc})({bc}))"), t))
    }
}

// liczba w C: mała jest typu int, a większa musi być int64_t (rachunki i tak rzutują na typ Zolta)
fn int_lit(v: i128) -> String {
    if v.unsigned_abs() <= i32::MAX as u128 { v.to_string() } else { format!("INT64_C({v})") }
}

// wypisanie wartości do strumienia f (stdout albo bufor tekstu ze wstawkami)
fn print_c(c: &str, t: Ty, f: &str) -> String {
    let pf = if f == "stdout" { "printf(".to_string() } else { format!("fprintf({f}, ") };
    match t {
        Ty::Int(n) if n <= 64 => format!("{pf}\"%lld\", (long long)({c}));"),
        Ty::Int(_) => format!("zt_pi128({f}, (__int128)({c}));"),
        Ty::Float(32) => format!("{pf}\"%.7g\", (double)({c}));"),
        Ty::Float(_) => format!("{pf}\"%.15g\", {c});"),
        Ty::Text => format!("fputs({c}, {f});"),
        Ty::Bool => format!("fputs(({c}) ? \"true\" : \"false\", {f});"),
        Ty::List => format!("zt_lst_print({f}, {c});"),
        Ty::Obj(i) => format!("zt_s{i}_print({f}, {c});"),
    }
}

// czy wartość typu v pasuje tam, gdzie jest typ t
fn fits(t: Ty, v: Ty) -> bool {
    match (t, v) {
        (Ty::Int(_), Ty::Int(_)) | (Ty::Float(_), Ty::Int(_) | Ty::Float(_)) => true,
        (a, b) => a == b,
    }
}

// lista z innej zmiennej jest kopiowana, żeby dwie nazwy nie dzieliły jednej listy
fn fresh(e: &E, c: String) -> String {
    match e.ex {
        Ex::ListLit(_) | Ex::Call(..) | Ex::MCall(..) => c,
        _ => format!("zt_lst_copy({c})"),
    }
}

fn read_only(name: &str) -> String {
    format!("`{name}.lst` jest pożyczona tylko do czytania (`&{name}.lst`); do zmian trzeba `&_{name}.lst`")
}

// tekst z sys::readN zamienia się w liczbę dopiero przy liczeniu
fn to_num(c: String, t: Ty, e: &E) -> Result<(String, Ty), Blad> {
    match t {
        Ty::Text => Ok((format!("zt_num({c})"), Ty::Float(64))),
        Ty::Bool => Err(e.err("prawda/fałsz to nie liczba")),
        Ty::List => Err(e.err("lista to nie liczba")),
        Ty::Obj(_) => Err(e.err("obiekt to nie liczba")),
        _ => Ok((c, t)),
    }
}

fn cstr(s: &str) -> String {
    let mut o = String::from("\"");
    for ch in s.chars() {
        match ch {
            '"' => o += "\\\"",
            '\\' => o += "\\\\",
            '\n' => o += "\\n",
            '\t' => o += "\\t",
            _ => o.push(ch),
        }
    }
    o + "\""
}

fn ascii(name: &str) -> String {
    name.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect()
}

fn lev(a: &str, b: &str) -> usize {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for i in 1..=a.len() {
        let mut cur = vec![i; b.len() + 1];
        for j in 1..=b.len() {
            let k = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + k);
        }
        prev = cur;
    }
    prev[b.len()]
}
