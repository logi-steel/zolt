# Zolt

Mały język programowania kompilowany do C. To, czy coś można zmieniać, widać w nazwie (`&_`),
a to, z czego funkcja korzysta (wejście/wyjście, pętle, listy), widać w jej nagłówku (`use =>`).

*A small language that compiles to C: mutability is visible at the use site (`&_`), and the
capabilities a function needs are listed in its signature (`use =>`). Docs are in Polish.*

```zolt
__ pożyczka do zapisu: podbij może zmienić licznik __
fn main (use => std::io) {
    let &_licznik : 0
    podbij(&_licznik)
    podbij(&_licznik)
    sys::out("licznik: <licznik>")
}

fn podbij (&_x) {
    x : x + 1
}
```

## Uruchamianie

Potrzebny jest Rust (`cargo`) i kompilator C z obsługą `_BitInt` z C23 (np. gcc 14 lub nowszy).

```
cargo build --release
./target/release/zolt run examples/witaj.zt
```

| komenda | co robi |
|---|---|
| `zolt run plik.zt` | kompiluje i uruchamia |
| `zolt build plik.zt` | tworzy plik wykonywalny |
| `zolt c plik.zt` | pokazuje wygenerowany kod C |
| `zolt lsp` | serwer języka dla edytora (błędy na bieżąco) |

## Język w skrócie

**Komentarze:** `__ tak __`.

**Zmienne.** `:` przypisuje. `let x : 5` to stała, `let &_x : 5` można zmieniać.

**Liczby z rozmiarem.** `c8_100`, `c128_...`: całkowite od `c1` do `c128` bitów. `c32_3.14`, `c64_...`: ułamki.
Prawda i fałsz: `b.true`, `b.false`.

**Tekst.** Wstawki w `< >`: `sys::out("wynik: <x>")`.
`sys::read("pytanie?")` zadaje pytanie, a kolejne odpowiedzi to `sys::read1`, `sys::read2`… Działanie
może też przyjść z wejścia: `a <>sys::read2<> b`.

**Warunki i pętle.**
```zolt
if x = 12 { ... } elseif x > 12 { ... } else { ... }
loop.3.times { ... }
loop.inf { if loop.inf.1 = 5 { loop,end } }   __ loop.inf.1 to licznik obrotów __
while x > 0 { x : x - 1 }
```

**Funkcje.** Parametry nie mają typów: każde wywołanie z innymi typami tworzy osobną wersję funkcji w C.
```zolt
fn dodaj (a, b) {
    retrn a + b
}
```

**Pożyczki.** `&x` tylko do odczytu, `&_x` do zapisu. W jednym wywołaniu zmienna może mieć tylko
jedną pożyczkę `&_`, a `&_` wymaga zmiennej, którą wolno zmieniać.

**Listy** (wymagają `std::lists`). Mieszają tekst, liczby i prawdę/fałsz, przy przypisaniu się kopiują.
```zolt
zakupy.lst : [mleko, 2, true]
if zakupy.lst has "mleko" { ... }
zakupy.lst.add "chleb"
zakupy.lst.del 2
zakupy.lst.chnm "mleko" -> "kefir"
```

**Szablony i obiekty.**
```zolt
gracz : {
    hp : 30
    zdobadz : (ile) { hp : hp + ile }
}

fn main (use => std::io) {
    let &_gracz.1
    gracz.1.zdobadz(5)
    sys::out("<gracz.1>")
}
```

**`use =>`.** `std::io` daje `sys::out` i `sys::read`, `std::loops` daje `loop.`, `std::lists` daje listy.

Więcej w `examples/`.

## Edytor

Helix: `editors/helix/zainstaluj.sh` instaluje `zolt` (z serwerem języka), gramatykę tree-sitter
i kolorowanie. Skrypt trzeba uruchomić ponownie po każdej zmianie kompilatora albo gramatyki.

## Stan (2026-11-01)

Działa wszystko z listy wyżej, z podpowiedziami przy literówkach.

Jeszcze nie ma:
- typów parametrów (funkcja, której nikt nie wywołuje, nie jest sprawdzana),
- pełnego borrow checkera: pożyczki są sprawdzane tylko w obrębie jednego wywołania,
- przechodniego `use =>`: funkcja bez `std::io` może dziś wywołać funkcję, która z niego korzysta.

## Licencja

[WTFPL](LICENSE), wersja 2, z klauzulą o braku gwarancji ze strony wtfpl.net:

> This program is free software. It comes without any warranty, to the extent permitted by
> applicable law. You can redistribute it and/or modify it under the terms of the Do What The
> Fuck You Want To Public License, Version 2, as published by Sam Hocevar. See
> http://www.wtfpl.net/ for more details.
