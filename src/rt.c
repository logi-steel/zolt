/* runtime Zolt (PoC) */
#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <stdbool.h>
#include <string.h>
#include <math.h>

static void zt_pi128(FILE *f, __int128 v) {
    char b[48];
    int i = sizeof b - 1;
    b[i] = 0;
    unsigned __int128 u = v < 0 ? -(unsigned __int128)v : (unsigned __int128)v;
    do { b[--i] = '0' + (int)(u % 10); u /= 10; } while (u);
    if (v < 0) b[--i] = '-';
    fputs(b + i, f);
}

static const char *zt_readline(void) {
    fflush(stdout);
    char *s = NULL;
    size_t cap = 0;
    ssize_t n = getline(&s, &cap, stdin);
    if (n < 0) { free(s); return ""; }
    if (n > 0 && s[n - 1] == '\n') s[n - 1] = 0;
    return s;
}

static double zt_num(const char *s) {
    char *end;
    double d = strtod(s, &end);
    while (*end == ' ' || *end == '\t' || *end == '\r') end++;
    if (end == s || *end) {
        fprintf(stderr, "błąd: \"%s\" to nie liczba\n", s);
        exit(1);
    }
    return d;
}

static double zt_dyn(double a, const char *op, double b) {
    while (*op == ' ') op++;
    size_t n = strlen(op);
    while (n && op[n - 1] == ' ') n--;
    if (n == 1) switch (op[0]) {
        case '+': return a + b;
        case '-': return a - b;
        case '*': return a * b;
        case '/': return a / b;
        case '%': return fmod(a, b);
    }
    fprintf(stderr, "błąd: nie znam działania \"%s\" (umiem: + - * / %%)\n", op);
    exit(1);
}

/* sys::arg1, sys::arg2, ... (brak = pusty tekst) */
static int zt_argc;
static char **zt_argv;

static const char *zt_arg(int i) { return i < zt_argc ? zt_argv[i] : ""; }

/* sys::file("plik") - cały plik jako tekst */
static const char *zt_file(const char *name) {
    FILE *f = fopen(name, "rb");
    if (!f) {
        fprintf(stderr, "błąd: nie mogę otworzyć pliku \"%s\"\n", name);
        exit(1);
    }
    char *s = NULL, b[4096];
    size_t n = 0, k;
    FILE *m = open_memstream(&s, &n);
    while ((k = fread(b, 1, sizeof b, f))) fwrite(b, 1, k, m);
    fclose(f);
    fclose(m);
    return s;
}

/* tekst.len i tekst.at i: liczymy znaki (UTF-8), od 1.
   Teksty się nie zmieniają, więc dla ostatnich pamiętamy długość i miejsce
   ostatniego `.at` - pętla `while i <= kod.len { kod.at i }` nie liczy od początku. */
typedef struct { const char *s, *p; int64_t i, len; } zt_tpos;

static zt_tpos *zt_tcache(const char *s) {
    static zt_tpos c[8]; /* ostatnio użyty jest pierwszy */
    int k = 0;
    while (k < 7 && c[k].s != s) k++;
    zt_tpos t = c[k].s == s ? c[k] : (zt_tpos){ s, s, 1, -1 };
    memmove(c + 1, c, k * sizeof *c);
    c[0] = t;
    return &c[0];
}

static int64_t zt_len(const char *s) {
    zt_tpos *t = zt_tcache(s);
    if (t->len < 0) {
        t->len = 0;
        for (; *s; s++) t->len += (*s & 0xC0) != 0x80;
    }
    return t->len;
}

static const char *zt_at(const char *s, int64_t i) {
    static char one[128][2];
    zt_tpos *t = zt_tcache(s);
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
    if (i < 1 || !*p) {
        fflush(stdout);
        fprintf(stderr, "błąd: znak numer %lld nie istnieje, tekst ma %lld znaków\n", (long long)i, (long long)zt_len(s));
        exit(1);
    }
    t->p = p, t->i = k;
    if ((unsigned char)*p < 128) {
        one[(int)*p][0] = *p;
        return one[(int)*p];
    }
    size_t n = 1;
    while ((p[n] & 0xC0) == 0x80) n++;
    char *r = malloc(n + 1);
    memcpy(r, p, n);
    r[n] = 0;
    return r;
}

[[noreturn]] static void zt_noret(const char *f) {
    fprintf(stderr, "błąd: funkcja `%s` skończyła się bez `retrn`\n", f);
    exit(1);
}

/* element listy: tekst, liczba, ułamek albo prawda/fałsz */
enum { ZT_TXT, ZT_INT, ZT_FLT, ZT_BOOL };
typedef struct { int t; int64_t i; double f; const char *s; } zt_val;

static zt_val zt_vt(const char *s) { return (zt_val){ .t = ZT_TXT, .s = s }; }
static zt_val zt_vi(int64_t i) { return (zt_val){ .t = ZT_INT, .i = i }; }
static zt_val zt_vf(double f) { return (zt_val){ .t = ZT_FLT, .f = f }; }
static zt_val zt_vb(bool b) { return (zt_val){ .t = ZT_BOOL, .i = b }; }

static bool zt_val_eq(zt_val a, zt_val b) {
    bool an = a.t == ZT_INT || a.t == ZT_FLT, bn = b.t == ZT_INT || b.t == ZT_FLT;
    if (an && bn) return (a.t == ZT_INT ? (double)a.i : a.f) == (b.t == ZT_INT ? (double)b.i : b.f);
    if (a.t != b.t) return false;
    return a.t == ZT_TXT ? !strcmp(a.s, b.s) : a.i == b.i;
}

static void zt_val_print(FILE *f, zt_val v) {
    switch (v.t) {
    case ZT_TXT: fputs(v.s, f); break;
    case ZT_INT: fprintf(f, "%lld", (long long)v.i); break;
    case ZT_FLT: fprintf(f, "%.15g", v.f); break;
    default: fputs(v.i ? "true" : "false", f);
    }
}

typedef struct { zt_val *v; size_t n, cap; } zt_lst;

static zt_lst zt_lst_new(size_t n, const zt_val *init) {
    zt_lst l = { malloc((n + 4) * sizeof(zt_val)), n, n + 4 };
    if (n) memcpy(l.v, init, n * sizeof(zt_val));
    return l;
}

static zt_lst zt_lst_copy(zt_lst l) { return zt_lst_new(l.n, l.v); }

static void zt_lst_add(zt_lst *l, zt_val x) {
    if (l->n == l->cap) l->v = realloc(l->v, (l->cap *= 2) * sizeof(zt_val));
    l->v[l->n++] = x;
}

static long zt_lst_find(zt_lst l, zt_val x) {
    for (size_t i = 0; i < l.n; i++)
        if (zt_val_eq(l.v[i], x)) return (long)i;
    return -1;
}

static bool zt_lst_has(zt_lst l, zt_val x) { return zt_lst_find(l, x) >= 0; }

static void zt_lst_del(zt_lst *l, zt_val x) {
    long i = zt_lst_find(*l, x);
    if (i < 0) return;
    memmove(l->v + i, l->v + i + 1, (l->n - i - 1) * sizeof(zt_val));
    l->n--;
}

static void zt_lst_chnm(zt_lst *l, zt_val a, zt_val b) {
    long i = zt_lst_find(*l, a);
    if (i >= 0) l->v[i] = b;
}

static void zt_lst_print(FILE *f, zt_lst l) {
    putc('[', f);
    for (size_t i = 0; i < l.n; i++) {
        if (i) fputs(", ", f);
        zt_val_print(f, l.v[i]);
    }
    putc(']', f);
}
