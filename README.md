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
| `zolt run plik.zt [argumenty]` | kompiluje i uruchamia (argumenty dostaje program) |
| `zolt build plik.zt` | tworzy plik wykonywalny |
| `zolt c plik.zt` | pokazuje wygenerowany kod C |
| `zolt lsp` | serwer języka dla edytora (błędy na bieżąco) |

## Język w skrócie

**Komentarze:** `__ tak __`.

**Zmienne.** `:` przypisuje. `let x : 5` to stała, `let &_x : 5` można zmieniać.

**Liczby z rozmiarem.** `c8_100`, `c128_...`: całkowite od `c1` do `c128` bitów. `c32_3.14`, `c64_...`: ułamki.
Prawda i fałsz: `b.true`, `b.false`.

**Tekst.** Wstawki w `< >` działają w każdym tekście: `sys::out("wynik: <x>")`, `let t : "<a><b>"`.
`t.len` to liczba znaków, `t.at 2` to drugi znak (liczymy od 1).
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

**Pliki i argumenty.** `sys::file("plik.zt")` daje cały plik jako tekst. `sys::arg1`, `sys::arg2`… to słowa
po nazwie programu (`./prog a b`), a gdy ich brak, pusty tekst. `sys::err("...")` wypisuje błąd i kończy
program.

**`use =>`.** `std::io` daje `sys::out`, `sys::read`, `sys::err`, `sys::file` i `sys::arg1`, `std::loops` daje `loop.`, `std::lists` daje listy.

Więcej w `examples/`.

## Zolt w Zolcie

`boot/zolt.zt` to kompilator Zolta napisany w Zolcie. Umie tyle języka, ile sam używa: funkcje z pożyczkami,
`let`, `if`, `while`, `loop,end`, `retrn`, tekst ze wstawkami, `.len`, `.at`, `sys::out`, `sys::err`,
`sys::file` i `sys::arg1`. Typy sprawdza dopiero uruchomiony program, bo w C każda wartość to `zv` z
`boot/zrt.h`.

```
boot/bootstrap.sh
```

Skrypt tłumaczy `zolt.zt` kompilatorem w Ruście, a powstały program tłumaczy `zolt.zt` jeszcze raz.
Oba wyniki muszą być identyczne. Na koniec `boot/test.zt` musi wypisać to samo z obu kompilatorów.

Ręcznie, po bootstrapie:

```
target/boot/zolt1 plik.zt > plik.c
cc -std=gnu23 -fwrapv -I boot plik.c -lm -o plik
```

## Edytor

Helix: `editors/helix/zainstaluj.sh` instaluje `zolt` (z serwerem języka), gramatykę tree-sitter
i kolorowanie. Skrypt trzeba uruchomić ponownie po każdej zmianie kompilatora albo gramatyki.

## Stan (2026-11-02)

Działa wszystko z listy wyżej, z podpowiedziami przy literówkach.

Kompilator w Zolcie kompiluje sam siebie. Nie zna jeszcze list, obiektów, `loop.` i `sys::read`, a błąd we
wstawce `<...>` pokazuje samą wstawkę zamiast miejsca w pliku.

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
