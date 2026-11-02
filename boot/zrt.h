/* This program is free software. It comes without any warranty, to
 * the extent permitted by applicable law. You can redistribute it
 * and/or modify it under the terms of the Do What The Fuck You Want
 * To Public License, Version 2, as published by Sam Hocevar. See
 * http://www.wtfpl.net/ for more details. */

/* runtime programów z kompilatora Zolt napisanego w Zolcie (boot/zolt.zt).
   Każda wartość to `zv`: liczba, ułamek, tekst albo prawda/fałsz,
   więc typy sprawdzamy dopiero przy działaniu programu. */
#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <stdbool.h>
#include <string.h>
#include <stdarg.h>
#include <math.h>

enum { ZNIC, ZINT, ZFLT, ZTXT, ZBOOL };
typedef struct { int t; int64_t i; double f; const char *s; } zv;

static int zargc;
static char **zargv;

static zv zi(int64_t i) { return (zv){ .t = ZINT, .i = i }; }
static zv zf(double f) { return (zv){ .t = ZFLT, .f = f }; }
static zv zs(const char *s) { return (zv){ .t = ZTXT, .s = s }; }
static zv zb(bool b) { return (zv){ .t = ZBOOL, .i = b }; }
static zv znic(void) { return (zv){ .t = ZNIC }; }

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

static const char *znazwa(zv v) {
    switch (v.t) {
    case ZINT: return "liczba";
    case ZFLT: return "ułamek";
    case ZTXT: return "tekst";
    case ZBOOL: return "prawda/fałsz";
    }
    return "nic (funkcja bez `retrn wartość`)";
}

static void zput(FILE *f, zv v) {
    switch (v.t) {
    case ZINT: fprintf(f, "%lld", (long long)v.i); break;
    case ZFLT: fprintf(f, "%.15g", v.f); break;
    case ZTXT: fputs(v.s, f); break;
    case ZBOOL: fputs(v.i ? "true" : "false", f); break;
    default: zfail("tu nie ma wartości (funkcja bez `retrn wartość`)");
    }
}

/* sys::out i sys::err */
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
    if (v.t != ZBOOL) zfail("warunek musi dawać prawdę/fałsz (np. `x = 12`), a daje: %s", znazwa(v));
    return v.i;
}

/* tekst w rachunku zamienia się w liczbę */
static zv znum(zv v) {
    if (v.t == ZINT || v.t == ZFLT) return v;
    if (v.t != ZTXT) zfail("%s to nie liczba", znazwa(v));
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
        if ((op == '/' || op == '%') && b.i == 0) zfail("dzielenie przez zero");
        switch (op) {
        case '+': return zi(a.i + b.i);
        case '-': return zi(a.i - b.i);
        case '*': return zi(a.i * b.i);
        case '/': return zi(a.i / b.i);
        default: return zi(a.i % b.i);
        }
    }
    double x = zd(a), y = zd(b);
    switch (op) {
    case '+': return zf(x + y);
    case '-': return zf(x - y);
    case '*': return zf(x * y);
    case '/': return zf(x / y);
    default: return zf(fmod(x, y));
    }
}

static zv zadd(zv a, zv b) { return zop('+', a, b); }
static zv zsub(zv a, zv b) { return zop('-', a, b); }
static zv zmul(zv a, zv b) { return zop('*', a, b); }
static zv zdiv(zv a, zv b) { return zop('/', a, b); }
static zv zmod(zv a, zv b) { return zop('%', a, b); }

static zv zneg(zv v) {
    v = znum(v);
    return v.t == ZINT ? zi(-v.i) : zf(-v.f);
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

/* sys::arg1, sys::arg2, ... (brak = pusty tekst) */
static zv zarg(int i) { return zs(i < zargc ? zargv[i] : ""); }

/* sys::file("plik") - cały plik jako tekst */
static zv zfile(zv name) {
    if (name.t != ZTXT) zfail("`sys::file` potrzebuje nazwy pliku (tekstu), a tu jest %s", znazwa(name));
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

static zv zlen(zv v) {
    if (v.t != ZTXT) zfail("`.len` działa na tekście, a tu jest %s", znazwa(v));
    return zi(zlen_c(v.s));
}

static zv zat(zv v, zv n) {
    static char one[128][2];
    if (v.t != ZTXT) zfail("`.at` działa na tekście, a tu jest %s", znazwa(v));
    if (n.t != ZINT) zfail("po `.at` powinien być numer znaku (liczba całkowita), a jest %s", znazwa(n));
    const char *s = v.s;
    int64_t i = n.i;
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
