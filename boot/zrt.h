/* This program is free software. It comes without any warranty, to
 * the extent permitted by applicable law. You can redistribute it
 * and/or modify it under the terms of the Do What The Fuck You Want
 * To Public License, Version 2, as published by Sam Hocevar. See
 * http://www.wtfpl.net/ for more details. */

/* runtime programów z kompilatora Zolt napisanego w Zolcie (boot/zolt.zt).
   Każda wartość to `zv`: liczba, ułamek, tekst, prawda/fałsz, lista albo obiekt,
   więc typy sprawdzamy dopiero przy działaniu programu. */
#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <stdbool.h>
#include <string.h>
#include <stdarg.h>
#include <math.h>

enum { ZNIC, ZINT, ZFLT, ZTXT, ZBOOL, ZLST, ZOBJ };
typedef struct zl zl;
typedef struct zo zo;
typedef struct {
    int t, bits; /* bits: rozmiar liczby (c1..c128) albo ułamka (c32, c64) */
    union {
        __int128 i; /* liczba, prawda/fałsz */
        double f;
        const char *s;
        zl *l;
        zo *o;
    };
} zv;
struct zl { zv *v; size_t n, cap; };

/* szablon: pola i metody (metoda dostaje obiekt, czy jest tylko do czytania, argumenty) */
typedef zv (*zmf)(zv *self, bool ro, zv **a);
typedef struct { const char *nazwa, *rodzaje; zmf f; } zmet;
typedef struct { const char *nazwa; int np; const char *const *pola; int nm; const zmet *met; } ztpl;
struct zo { const ztpl *t; zv *f; };

static int zargc;
static char **zargv;

[[noreturn]] static void zfail(const char *fmt, ...) {
    va_list a;
    va_start(a, fmt);
    fflush(stdout);
    fputs("błąd: ", stderr);
    vfprintf(stderr, fmt, a);
    fputc('\n', stderr);
    va_end(a);
    exit(1);
}

/* liczba cN zawija się jak _BitInt(N); c1 to 0 albo 1 */
static __int128 zwrap(__int128 v, int bits) {
    if (bits >= 128) return v;
    if (bits == 1) return v & 1;
    unsigned __int128 m = ((unsigned __int128)1 << bits) - 1, u = (unsigned __int128)v & m;
    if (u >> (bits - 1)) u |= ~m;
    return (__int128)u;
}

static zv zn(__int128 i, int bits) { return (zv){ .t = ZINT, .bits = bits, .i = zwrap(i, bits) }; }
static zv zi(int64_t i) { return zn(i, 64); }
static zv zf(double f) { return (zv){ .t = ZFLT, .bits = 64, .f = f }; }
static zv zf32(double f) { return (zv){ .t = ZFLT, .bits = 32, .f = (float)f }; }
static zv zs(const char *s) { return (zv){ .t = ZTXT, .s = s }; }
static zv zb(bool b) { return (zv){ .t = ZBOOL, .i = b }; }
static zv znic(void) { return (zv){ .t = ZNIC }; }

static const char *ztyp(zv v) {
    static char buf[4][32];
    static int k;
    char *b = buf[k++ % 4];
    switch (v.t) {
    case ZINT: snprintf(b, 32, "liczba c%d", v.bits); return b;
    case ZFLT: snprintf(b, 32, "ułamek c%d", v.bits); return b;
    case ZTXT: return "tekst";
    case ZBOOL: return "prawda/fałsz";
    case ZLST: return "lista";
    case ZOBJ: return "obiekt";
    }
    return "nic (funkcja bez `retrn wartość`)";
}

static void zpi(FILE *f, __int128 v) {
    char b[48];
    int i = sizeof b - 1;
    b[i] = 0;
    unsigned __int128 u = v < 0 ? -(unsigned __int128)v : (unsigned __int128)v;
    do { b[--i] = '0' + (int)(u % 10); u /= 10; } while (u);
    if (v < 0) b[--i] = '-';
    fputs(b + i, f);
}

static void zput(FILE *f, zv v) {
    switch (v.t) {
    case ZINT: zpi(f, v.i); break;
    case ZFLT: fprintf(f, v.bits == 32 ? "%.7g" : "%.15g", v.f); break;
    case ZTXT: fputs(v.s, f); break;
    case ZBOOL: fputs(v.i ? "true" : "false", f); break;
    case ZLST:
        putc('[', f);
        for (size_t k = 0; k < v.l->n; k++) {
            if (k) fputs(", ", f);
            zput(f, v.l->v[k]);
        }
        putc(']', f);
        break;
    case ZOBJ:
        putc('{', f);
        for (int k = 0; k < v.o->t->np; k++) {
            fprintf(f, "%s%s: ", k ? ", " : "", v.o->t->pola[k]);
            zput(f, v.o->f[k]);
        }
        putc('}', f);
        break;
    default: zfail("tu nie ma wartości (funkcja bez `retrn wartość`)");
    }
}

/* sys::out, sys::err, sys::read */
static void zout(zv v) {
    zput(stdout, v);
    putchar('\n');
}

[[noreturn]] static void zerr(zv v) {
    fflush(stdout);
    zput(stderr, v);
    fputc('\n', stderr);
    exit(1);
}

static void zread(zv *dst, zv pytanie) {
    zput(stdout, pytanie);
    putchar(' ');
    fflush(stdout);
    char *s = NULL;
    size_t cap = 0;
    ssize_t n = getline(&s, &cap, stdin);
    if (n < 0) s = "";
    else if (n > 0 && s[n - 1] == '\n') s[n - 1] = 0;
    *dst = zs(s);
}

/* tekst ze wstawkami: "a <x> b" */
static zv zcat(int n, const zv *p) {
    char *b;
    size_t len;
    FILE *f = open_memstream(&b, &len);
    for (int k = 0; k < n; k++) zput(f, p[k]);
    fclose(f);
    return zs(b);
}

static bool ztak(zv v) {
    if (v.t != ZBOOL) zfail("warunek musi dawać prawdę/fałsz (np. `x = 12`), a daje: %s", ztyp(v));
    return v.i;
}

[[noreturn]] static void zbezretrn(const char *f) { zfail("funkcja `%s` skończyła się bez `retrn`", f); }

/* ---------- listy i obiekty: przypisanie kopiuje ---------- */

static zv zlista(size_t n, const zv *v) {
    zl *l = malloc(sizeof *l);
    l->n = n;
    l->cap = n + 4;
    l->v = malloc(l->cap * sizeof(zv));
    if (n) memcpy(l->v, v, n * sizeof(zv));
    return (zv){ .t = ZLST, .l = l };
}

static zv znowy(const ztpl *t, const zv *pola) {
    zo *o = malloc(sizeof *o);
    o->t = t;
    o->f = malloc((t->np + 1) * sizeof(zv));
    if (t->np) memcpy(o->f, pola, t->np * sizeof(zv));
    return (zv){ .t = ZOBJ, .o = o };
}

static zv zcopy(zv v) {
    if (v.t == ZLST) return zlista(v.l->n, v.l->v);
    if (v.t == ZOBJ) return znowy(v.o->t, v.o->f);
    return v;
}

static zv zlet(zv v) {
    if (v.t == ZLST) zfail("listę tworzysz bez `let`: `nazwa.lst : [a, b, c]`");
    if (v.t == ZNIC) zfail("tu nie ma wartości (funkcja bez `retrn wartość`)");
    return zcopy(v);
}

/* zmienna ma stały typ: liczbę wolno wpisać do ułamka, reszta musi się zgadzać */
static zv zset(zv old, zv v) {
    if (old.t == ZINT && v.t == ZINT) return zn(v.i, old.bits);
    if (old.t == ZFLT && (v.t == ZINT || v.t == ZFLT)) {
        double d = v.t == ZINT ? (double)v.i : v.f;
        return old.bits == 32 ? zf32(d) : zf(d);
    }
    if (old.t == v.t && old.t != ZNIC && (old.t != ZOBJ || old.o->t == v.o->t)) return zcopy(v);
    zfail("tu jest %s, a do zmiennej pasuje tylko %s", ztyp(v), ztyp(old));
}

static void zpisz(zv *d, zv v) { *d = zset(*d, v); }

static zv zlistset(zv v) {
    if (v.t != ZLST) zfail("do listy pasuje tylko lista `[a, b, c]`, a tu jest %s", ztyp(v));
    return zcopy(v);
}

/* parametr bez `&`: lista tylko do parametru z `.lst` */
static zv zparam(zv v, bool lista, const char *nazwa) {
    if (lista && v.t != ZLST) zfail("parametr `%s` to lista, a tu jest %s", nazwa, ztyp(v));
    if (!lista && v.t == ZLST) zfail("parametr `%s` to nie lista (lista ma w nazwie `.lst`)", nazwa);
    return v;
}

/* element listy: tekst, liczba (max c64), ułamek albo prawda/fałsz */
static zv zel(zv v) {
    switch (v.t) {
    case ZINT:
        if (v.bits > 64) zfail("c%d jest za duże do listy (max c64)", v.bits);
        return zn(v.i, 64);
    case ZFLT: return zf(v.f);
    case ZLST: zfail("listy w liście jeszcze nie ma");
    case ZOBJ: zfail("obiektu w liście jeszcze nie ma");
    case ZNIC: zfail("tu nie ma wartości (funkcja bez `retrn wartość`)");
    }
    return v;
}

static bool zel_eq(zv a, zv b) {
    bool an = a.t == ZINT || a.t == ZFLT, bn = b.t == ZINT || b.t == ZFLT;
    if (an && bn) return (a.t == ZINT ? (double)a.i : a.f) == (b.t == ZINT ? (double)b.i : b.f);
    if (a.t != b.t) return false;
    return a.t == ZTXT ? !strcmp(a.s, b.s) : a.i == b.i;
}

static zl *zlst(zv v, const char *co) {
    if (v.t != ZLST) zfail("%s działa na listach, a tu jest %s", co, ztyp(v));
    return v.l;
}

static long zl_find(zl *l, zv x) {
    for (size_t i = 0; i < l->n; i++)
        if (zel_eq(l->v[i], x)) return (long)i;
    return -1;
}

static zv zhas(zv l, zv x) { return zb(zl_find(zlst(l, "`has` (`x.lst has \"oki\"`)"), zel(x)) >= 0); }

static void zl_add(zv *v, zv x) {
    zl *l = zlst(*v, "`.add`");
    x = zel(x);
    if (l->n == l->cap) l->v = realloc(l->v, (l->cap *= 2) * sizeof(zv));
    l->v[l->n++] = x;
}

static void zl_del(zv *v, zv x) {
    zl *l = zlst(*v, "`.del`");
    long i = zl_find(l, zel(x));
    if (i < 0) return;
    memmove(l->v + i, l->v + i + 1, (l->n - i - 1) * sizeof(zv));
    l->n--;
}

static void zl_chnm(zv *v, zv a, zv b) {
    zl *l = zlst(*v, "`.chnm`");
    long i = zl_find(l, zel(a));
    b = zel(b);
    if (i >= 0) l->v[i] = b;
}

/* pole obiektu po nazwie; zpolew sprawdza, czy metoda może zmieniać pola */
static zv *zpole(zv v, const char *n) {
    if (v.t != ZOBJ) zfail("`.%s` działa na obiektach z szablonu, a tu jest %s", n, ztyp(v));
    const ztpl *t = v.o->t;
    for (int k = 0; k < t->np; k++)
        if (!strcmp(t->pola[k], n)) return &v.o->f[k];
    for (int k = 0; k < t->nm; k++)
        if (!strcmp(t->met[k].nazwa, n)) zfail("`%s` to metoda, wywołaj ją: `.%s()`", n, n);
    zfail("szablon `%s` nie ma pola `%s`", t->nazwa, n);
}

static zv *zpolew(zv v, bool ro, const char *n) {
    if (ro) zfail("metoda zmienia pole `%s`, a obiekt nie jest zmienny; utwórz go przez `let &_nazwa.1`", n);
    return zpole(v, n);
}

static const char *zopis(char r) {
    switch (r) {
    case 'r': case 'R': return "pożyczki do czytania: `&zmienna`";
    case 'w': case 'W': return "pożyczki do zmian: `&_zmienna`";
    }
    return "zwykłej wartości (bez `&`)";
}

/* rodzaje: v wartość, r &x, w &_x; wielka litera = lista */
static zv zmcall(zv *self, bool ro, const char *n, const char *rodzaje, zv **a) {
    if (self->t != ZOBJ) zfail("to nie obiekt z szablonu (tylko %s), więc nie ma metod", ztyp(*self));
    const ztpl *t = self->o->t;
    for (int k = 0; k < t->nm; k++) {
        const zmet *m = &t->met[k];
        if (strcmp(m->nazwa, n)) continue;
        size_t want = strlen(m->rodzaje), have = strlen(rodzaje);
        if (want != have) zfail("`%s.%s`: liczba argumentów ma być %zu, a jest %zu", t->nazwa, n, want, have);
        for (size_t i = 0; i < want; i++) {
            char p = m->rodzaje[i] | 0x20, c = rodzaje[i] | 0x20;
            if (p != c) zfail("parametr nr %zu w `%s.%s` chce %s", i + 1, t->nazwa, n, zopis(p));
            if (c != 'v' && m->rodzaje[i] != rodzaje[i])
                zfail(m->rodzaje[i] < 'a' ? "parametr nr %zu w `%s.%s` to lista" : "parametr nr %zu w `%s.%s` to nie lista (lista ma w nazwie `.lst`)", i + 1, t->nazwa, n);
        }
        return m->f(self, ro, a);
    }
    zfail("szablon `%s` nie ma metody `%s`", t->nazwa, n);
}

/* ---------- rachunki ---------- */

/* tekst w rachunku zamienia się w liczbę */
static zv znum(zv v) {
    switch (v.t) {
    case ZINT: case ZFLT: return v;
    case ZBOOL: zfail("prawda/fałsz to nie liczba");
    case ZLST: zfail("lista to nie liczba");
    case ZOBJ: zfail("obiekt to nie liczba");
    case ZNIC: zfail("tu nie ma wartości (funkcja bez `retrn wartość`)");
    }
    char *end;
    double d = strtod(v.s, &end);
    while (*end == ' ' || *end == '\t' || *end == '\r') end++;
    if (end == v.s || *end) zfail("\"%s\" to nie liczba", v.s);
    return zf(d);
}

static double zd(zv v) { return v.t == ZINT ? (double)v.i : v.f; }

static zv zop(char op, zv a, zv b) {
    a = znum(a), b = znum(b);
    if (a.t == ZINT && b.t == ZINT) {
        int bits = a.bits > b.bits ? a.bits : b.bits;
        unsigned __int128 x = a.i, y = b.i;
        if ((op == '/' || op == '%') && b.i == 0) zfail("dzielenie przez zero");
        switch (op) {
        case '+': return zn(x + y, bits);
        case '-': return zn(x - y, bits);
        case '*': return zn(x * y, bits);
        case '/': return zn(a.i / b.i, bits);
        default: return zn(a.i % b.i, bits);
        }
    }
    bool f32 = a.t == ZFLT && b.t == ZFLT && a.bits == 32 && b.bits == 32;
    double x = zd(a), y = zd(b), r;
    switch (op) {
    case '+': r = x + y; break;
    case '-': r = x - y; break;
    case '*': r = x * y; break;
    case '/': r = x / y; break;
    default: r = fmod(x, y);
    }
    return f32 ? zf32(r) : zf(r);
}

static zv zadd(zv a, zv b) { return zop('+', a, b); }
static zv zsub(zv a, zv b) { return zop('-', a, b); }
static zv zmul(zv a, zv b) { return zop('*', a, b); }
static zv zdiv(zv a, zv b) { return zop('/', a, b); }
static zv zmod(zv a, zv b) { return zop('%', a, b); }

static zv zneg(zv v) {
    v = znum(v);
    if (v.t == ZINT) return zn(-(unsigned __int128)v.i, v.bits);
    return v.bits == 32 ? zf32(-v.f) : zf(-v.f);
}

/* a <>op<> b: działanie przychodzi jako tekst */
static zv zdyn(zv a, zv op, zv b) {
    if (op.t != ZTXT) zfail("między `<>` musi być tekst z działaniem (np. `sys::read2`), a jest %s", ztyp(op));
    double x = zd(znum(a)), y = zd(znum(b));
    const char *s = op.s;
    while (*s == ' ') s++;
    size_t n = strlen(s);
    while (n && s[n - 1] == ' ') n--;
    if (n == 1) switch (s[0]) {
        case '+': return zf(x + y);
        case '-': return zf(x - y);
        case '*': return zf(x * y);
        case '/': return zf(x / y);
        case '%': return zf(fmod(x, y));
        }
    zfail("nie znam działania \"%s\" (umiem: + - * / %%)", op.s);
}

/* tekst z tekstem porównujemy po literach, resztę jako liczby */
static int zcmp(zv a, zv b, bool rowne) {
    if (a.t == ZTXT && b.t == ZTXT) return strcmp(a.s, b.s);
    if (a.t == ZBOOL || b.t == ZBOOL) {
        if (rowne && a.t == b.t) return a.i != b.i;
        zfail("prawdę/fałsz można tylko porównać `=` z drugą prawdą/fałszem");
    }
    a = znum(a), b = znum(b);
    if (a.t == ZINT && b.t == ZINT) return (a.i > b.i) - (a.i < b.i);
    double x = zd(a), y = zd(b);
    return (x > y) - (x < y);
}

static zv zeq(zv a, zv b) { return zb(zcmp(a, b, true) == 0); }
static zv zlt(zv a, zv b) { return zb(zcmp(a, b, false) < 0); }
static zv zgt(zv a, zv b) { return zb(zcmp(a, b, false) > 0); }
static zv zle(zv a, zv b) { return zb(zcmp(a, b, false) <= 0); }
static zv zge(zv a, zv b) { return zb(zcmp(a, b, false) >= 0); }

/* loop.N.times i licznik loop.inf.N */
static int64_t zcalk(zv v) {
    if (v.t != ZINT) zfail("`loop.N.times` potrzebuje liczby całkowitej, a tu jest %s", ztyp(v));
    return (int64_t)v.i;
}

static zv zinc(zv v) { return zn(v.i + 1, v.bits); }

/* ---------- tekst i pliki ---------- */

/* sys::arg1, sys::arg2, ... (brak = pusty tekst) */
static zv zarg(int i) { return zs(i < zargc ? zargv[i] : ""); }

/* sys::file("plik") - cały plik jako tekst */
static zv zfile(zv name) {
    if (name.t != ZTXT) zfail("`sys::file` potrzebuje nazwy pliku (tekstu), a tu jest %s", ztyp(name));
    FILE *f = fopen(name.s, "rb");
    if (!f) zfail("nie mogę otworzyć pliku \"%s\"", name.s);
    char *s = NULL, b[4096];
    size_t n = 0, k;
    FILE *m = open_memstream(&s, &n);
    while ((k = fread(b, 1, sizeof b, f))) fwrite(b, 1, k, m);
    fclose(f);
    fclose(m);
    return zs(s);
}

/* tekst.len i tekst.at i: liczymy znaki (UTF-8), od 1.
   Teksty się nie zmieniają, więc dla ostatnich pamiętamy długość i miejsce
   ostatniego `.at` - pętla `while i <= kod.len { kod.at i }` nie liczy od początku. */
typedef struct { const char *s, *p; int64_t i, len; } ztpos;

static ztpos *ztcache(const char *s) {
    static ztpos c[8]; /* ostatnio użyty jest pierwszy */
    int k = 0;
    while (k < 7 && c[k].s != s) k++;
    ztpos t = c[k].s == s ? c[k] : (ztpos){ s, s, 1, -1 };
    memmove(c + 1, c, k * sizeof *c);
    c[0] = t;
    return &c[0];
}

static int64_t zlen_c(const char *s) {
    ztpos *t = ztcache(s);
    if (t->len < 0) {
        t->len = 0;
        for (; *s; s++) t->len += (*s & 0xC0) != 0x80;
    }
    return t->len;
}

/* .len: długość tekstu albo pole `len` obiektu */
static zv zlen(zv v) {
    if (v.t == ZOBJ) return *zpole(v, "len");
    if (v.t != ZTXT) zfail("tekst ma `.len`, a tu jest %s", ztyp(v));
    return zi(zlen_c(v.s));
}

static zv zat(zv v, zv n) {
    static char one[128][2];
    if (v.t != ZTXT) zfail("`.at` działa na tekście, a tu jest %s", ztyp(v));
    if (n.t != ZINT) zfail("po `.at` powinien być numer znaku (liczba całkowita), a jest %s", ztyp(n));
    const char *s = v.s;
    int64_t i = (int64_t)n.i;
    ztpos *t = ztcache(s);
    const char *p = t->p;
    int64_t k = t->i;
    if (i >= 1 && i < k) {
        if (i <= k - i) p = s, k = 1;
        else
            while (k > i) {
                p--;
                while ((*p & 0xC0) == 0x80) p--;
                k--;
            }
    }
    while (i >= 1 && *p && k < i) {
        p++;
        while ((*p & 0xC0) == 0x80) p++;
        k++;
    }
    if (i < 1 || !*p) zfail("znak numer %lld nie istnieje, tekst ma %lld znaków", (long long)i, (long long)zlen_c(s));
    t->p = p, t->i = k;
    if ((unsigned char)*p < 128) {
        one[(int)*p][0] = *p;
        return zs(one[(int)*p]);
    }
    size_t len = 1;
    while ((p[len] & 0xC0) == 0x80) len++;
    char *r = malloc(len + 1);
    memcpy(r, p, len);
    r[len] = 0;
    return zs(r);
}
