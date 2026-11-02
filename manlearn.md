# Zolt od 0 do 100

Zolt to mały język kompilowany do C. Ma trzy cechy, które wyróżniają go na tle innych:

- **`:` przypisuje, `=` porównuje.** Nie da się ich pomylić w warunku.
- **`&_` znaczy „można zmieniać”.** Tę samą końcówkę mają zmienne, pożyczki i obiekty.
- **`use =>` w nagłówku funkcji mówi, z czego funkcja korzysta.** Funkcja bez `std::io` nic nie wypisze.

Rozdziały idą po kolei i każdy korzysta z poprzednich. Najlepiej przepisywać przykłady samemu i
uruchamiać je, a potem coś w nich psuć i czytać błędy.

---

## 0. Pierwszy program

Kompilator buduje się raz:

```
cargo build --release
```

Plik `witaj.zt`:

```zolt
fn main (use => std::io) {
    sys::out("cześć, świecie")
}
```

```
zolt run witaj.zt
```

Każdy program zaczyna się w funkcji `main`. `use => std::io` pozwala jej wypisywać, a
`sys::out(...)` wypisuje tekst i przechodzi do nowej linii.

Inne komendy:

| komenda | co robi |
|---|---|
| `zolt run plik.zt` | kompiluje i od razu uruchamia |
| `zolt build plik.zt` | tworzy plik wykonywalny obok (`plik`) |
| `zolt c plik.zt` | pokazuje kod C, który powstał z programu |

**Spróbuj:** usuń `use => std::io` i uruchom program jeszcze raz. Przeczytaj błąd.

---

## 1. Komentarze

Komentarz zaczyna się i kończy dwoma podkreślnikami. Może zajmować kilka linii.

```zolt
__ to jest komentarz __

__ ten też,
   tylko dłuższy __
fn main (use => std::io) {
    sys::out("komentarze nic nie robią")
}
```

---

## 2. Stałe i zmienne

`let` tworzy nazwę, a `:` nadaje jej wartość.

```zolt
fn main (use => std::io) {
    let wiek : 17
    let &_punkty : 0
    punkty : punkty + 10
    sys::out("wiek <wiek>, punkty <punkty>")
}
```

- `let wiek : 17` to **stała**. Później nie da się jej zmienić.
- `let &_punkty : 0` to **zmienna**. Końcówka `&_` przed nazwą pozwala ją zmieniać.
- `punkty : punkty + 10` przypisuje nową wartość, już bez `let`.

Próba zmiany stałej kończy się błędem, który od razu mówi, co poprawić:

```
błąd: `wiek` jest stała; żeby ją zmieniać, zadeklaruj ją jako `let &_wiek`
```

---

## 3. Liczby

Zwykła liczba całkowita ma 64 bity, a liczba z kropką jest ułamkiem.

```zolt
fn main (use => std::io) {
    sys::out("<7 + 2> <7 - 2> <7 * 2>")
    sys::out("<7 / 2> <7 % 2>")
    sys::out("<7.0 / 2> <-5 + 1>")
}
```

```
9 5 14
3 1
3.5 -4
```

Dzielenie dwóch liczb całkowitych odcina część po przecinku (`7 / 2` to `3`). `%` to reszta z
dzielenia. Żeby dostać `3.5`, jedna z liczb musi być ułamkiem.

### Liczby z rozmiarem

Przed liczbą można podać, ile ma mieć bitów: `c` + liczba bitów + `_` + wartość.

```zolt
fn main (use => std::io) {
    let bajt : c8_100
    let duza : c128_170141183460469231731687303715884105727
    let pi : c32_3.14159265
    sys::out("<bajt> <duza> <pi>")
}
```

- `c1` … `c128`: liczby całkowite o dokładnie tylu bitach,
- `c32` i `c64`: ułamki pojedynczej i podwójnej precyzji.

Liczba, która się nie mieści, to błąd kompilacji, a nie cicha pomyłka:

```
błąd: 200 nie mieści się w c8 (max 127)
```

**Spróbuj:** jaka jest największa liczba, która mieści się w `c16`?

---

## 4. Prawda, fałsz i porównania

Prawda to `b.true`, fałsz to `b.false`.

| zapis | znaczy |
|---|---|
| `a = b` | równe |
| `a < b`, `a > b` | mniejsze, większe |
| `a <= b`, `a >= b` | mniejsze lub równe, większe lub równe |

`=` zawsze porównuje, a przypisuje tylko `:`. Porównywać można też tekst: `imie = "Ala"`.

Zolt nie ma „różne od” ani łączenia warunków przez „i” / „lub”. Takie rzeczy zapisuje się
przez `else` i zagnieżdżone `if` (rozdział 7).

---

## 5. Tekst

Tekst piszemy w cudzysłowie. W `< >` można wstawić wartość, rachunek albo wywołanie funkcji:

```zolt
fn main (use => std::io) {
    let imie : "Ala"
    let lata : 12
    sys::out("<imie> ma <lata> lat, za rok <lata + 1>")
    sys::out("zwykły znak mniejszości to \<, o tak: 3 \< 5")
}
```

```
Ala ma 12 lat, za rok 13
zwykły znak mniejszości to <, o tak: 3 < 5
```

Skoro `<` otwiera wstawkę, zwykły znak `<` pisze się jako `\<`.

Wstawki działają w każdym tekście, nie tylko w `sys::out`. Tak skleja się teksty:

```zolt
fn main (use => std::io) {
    let imie : "Ala"
    let &_napis : "<imie> ma kota"
    napis : "<napis>, a kot ma <imie>"
    sys::out(napis)
}
```

### Znaki w tekście

`.len` mówi, ile tekst ma znaków, a `.at` podaje znak pod numerem. Liczymy od 1.

```zolt
fn main (use => std::io) {
    let t : "żółw"
    sys::out("<t.len> znaki, pierwszy to <t.at 1>, ostatni to <t.at t.len>")
}
```

```
4 znaki, pierwszy to ż, ostatni to w
```

Po `.at` stoi jedna wartość. Rachunek trzeba wziąć w nawias: `t.at (i + 1)`. Numer spoza tekstu kończy
program błędem.

`+` zawsze liczy, nawet przy tekście: tekst zamienia się wtedy w liczbę. `"17" + 1` daje `18`, a
`"kot" + 1` kończy program błędem `"kot" to nie liczba`. Teksty łączy się przez wstawki, jak wyżej.

---

## 6. Wczytywanie

`sys::read("pytanie")` wypisuje pytanie i czeka na odpowiedź. Odpowiedzi są numerowane po kolei:
pierwsza to `sys::read1`, druga `sys::read2` i tak dalej.

```zolt
fn main (use => std::io) {
    sys::read("ile masz lat?")
    let wiek : sys::read1
    sys::out("za rok będziesz mieć <wiek + 1> lat")
}
```

Odpowiedź jest tekstem, ale w rachunku zamienia się w liczbę (rozdział 5).

### Działanie z wejścia

Między `<>` a `<>` może stać odpowiedź, która jest znakiem działania. Tak powstaje kalkulator:

```zolt
fn main (use => std::io) {
    sys::read("pierwsza liczba?")
    sys::read("działanie (+ - * / %)?")
    sys::read("druga liczba?")
    let wynik : sys::read1 <>sys::read2<> sys::read3
    sys::out("wynik: <wynik>")
}
```

**Spróbuj:** dopisz trzecie pytanie o liczbę i policz `a <>op<> b <>op<> c` z tym samym działaniem.

### Pliki i argumenty

`sys::file("nazwa")` daje cały plik jako jeden tekst. `sys::arg1`, `sys::arg2`, ... to słowa wpisane po
nazwie programu. Gdy słowa nie ma, dostajesz pusty tekst `""`.

```zolt
fn main (use => std::io) {
    let plik : sys::arg1
    if plik = "" { sys::out("podaj nazwę pliku") }
    else { sys::out("<plik> ma <sys::file(plik).len> znaków") }
}
```

```
zolt run licz.zt witaj.zt
```

`sys::err("komunikat")` wypisuje komunikat jako błąd i od razu kończy program:

```zolt
fn main (use => std::io) {
    if sys::arg1 = "" { sys::err("podaj nazwę pliku") }
    sys::out("mam <sys::arg1>")
}
```

---

## 7. Warunki

```zolt
fn main (use => std::io) {
    let temperatura : 15
    if temperatura < 0 {
        sys::out("mróz")
    } elseif temperatura < 20 {
        sys::out("chłodno")
    } else {
        sys::out("ciepło")
    }
}
```

`elseif` sprawdza kolejny warunek, `else` łapie całą resztę. Obie części są opcjonalne.

„Różne od” zapisuje się przez `else`:

```zolt
fn main (use => std::io) {
    let imie : "Olek"
    if imie = "Ala" { } else { sys::out("to nie Ala") }
}
```

---

## 8. Pętle

Pętle `loop` wymagają `use => std::loops`.

### Określona liczba razy

```zolt
fn main (use => std::io std::loops) {
    loop.3.times {
        sys::out("hej")
    }
    let n : 2
    loop.n.times {
        sys::out("ho")
    }
}
```

Warunek może od razu prowadzić do pętli, bez dodatkowych klamer:

```zolt
fn main (use => std::io std::loops) {
    let x : 1
    if x = 1 loop.2.times { sys::out("x to jeden") }
}
```

### Bez końca, z licznikiem

`loop.inf` kręci się, aż przerwie ją `loop,end`. Każda `loop.inf` w funkcji ma licznik obrotów,
który zaczyna od `0`. Licznik pierwszej pętli to `loop.inf.1`, drugiej `loop.inf.2` i tak dalej.

```zolt
fn main (use => std::io std::loops) {
    loop.inf {
        sys::out("obrót <loop.inf.1>")
        if loop.inf.1 = 2 { loop,end }
    }
    loop.inf {
        if loop.inf.2 = 5 { loop,end }
    }
    sys::out("pierwsza skończyła na <loop.inf.1>, druga na <loop.inf.2>")
}
```

```
obrót 0
obrót 1
obrót 2
pierwsza skończyła na 2, druga na 5
```

### Dopóki warunek jest prawdziwy

`while` nie potrzebuje żadnego `use`. Też można ją przerwać przez `loop,end`.

```zolt
fn main (use => std::io) {
    let &_odliczanie : 3
    while odliczanie > 0 {
        sys::out("<odliczanie>…")
        odliczanie : odliczanie - 1
    }
    sys::out("start!")
}
```

**Spróbuj:** wypisz tabliczkę mnożenia przez 7 (od `7` do `70`) za pomocą `loop.inf` i jej licznika.

---

## 9. Funkcje

```zolt
fn main (use => std::io) {
    sys::out("<dodaj(2, 3)>")
    sys::out("<dodaj(1.5, 2)>")
    let wynik : dodaj(10, 20)
    sys::out("<wynik>")
}

fn dodaj (a, b) {
    retrn a + b
}
```

- Parametry wypisuje się po przecinku, bez typów.
- `retrn` oddaje wartość i kończy funkcję. Wartość musi stać w tej samej linii co `retrn`.
- Ta sama funkcja działa dla liczb całkowitych i ułamków. Kompilator tworzy osobną wersję dla
  każdego zestawu typów, z jakim ją wywołano.

Funkcja może wywoływać samą siebie:

```zolt
fn main (use => std::io) {
    sys::out("fib(20) = <fib(20)>")
}

fn fib (n) {
    if n < 2 { retrn n }
    retrn fib(n - 1) + fib(n - 2)
}
```

Samo `retrn` kończy funkcję bez wartości. Każda funkcja deklaruje własne `use =>`, a parametry
stoją przed nim:

```zolt
fn main (use => std::io) {
    powitaj(0)
    powitaj(1)
}

fn powitaj (n use => std::io) {
    if n = 0 { retrn }
    sys::out("cześć")
}
```

---

## 10. Pożyczki

Zwykły parametr to kopia: funkcja może z nim robić, co chce, a oryginał się nie zmieni. Żeby
funkcja pracowała na oryginale, trzeba go **pożyczyć**:

| w wywołaniu i w nagłówku | znaczy |
|---|---|
| `&x` | pożyczka do czytania: funkcja widzi oryginał, ale nie może go zmienić |
| `&_x` | pożyczka do zmian: funkcja może zmienić oryginał |

```zolt
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

```
licznik: 2
```

Zasady, których pilnuje kompilator:

1. **`&_` tylko dla zmiennej.** Stałej (`let a : 1`) nie da się pożyczyć do zmian.
2. **`&_` musi być jedyna.** W jednym wywołaniu zmienna może mieć tylko jedną pożyczkę do zmian
   i żadnej innej obok: `zamien(&_a, &_a)` to błąd.
3. **`&x` nie zmienia.** Funkcja, która dostała `&x`, nie może przypisać do `x`.

Dzięki temu z samego wywołania widać, co funkcja może zmienić: `podbij(&_licznik)` może, a
`pokaz(&licznik)` na pewno nie.

**Spróbuj:** napisz `zamien(&_a, &_b)`, które zamienia wartości dwóch zmiennych.

---

## 11. Listy

Listy wymagają `use => std::lists`. Nazwa listy kończy się na `.lst`.

```zolt
fn main (use => std::io std::lists) {
    zakupy.lst : [mleko, 2, true, "dwa jajka"]
    sys::out("<zakupy.lst>")

    if zakupy.lst has "mleko" { sys::out("jest mleko") }
    zakupy.lst.add "chleb"
    zakupy.lst.del 2
    zakupy.lst.chnm "mleko" -> "kefir"
    sys::out("<zakupy.lst>")
}
```

```
[mleko, 2, true, dwa jajka]
jest mleko
[kefir, true, dwa jajka, chleb]
```

- W jednej liście mogą być tekst, liczby oraz `true`/`false`. Słowa bez cudzysłowu też są tekstem.
- `has` sprawdza, czy element jest na liście.
- `.add` dopisuje na końcu, `.del` usuwa element, a `.chnm a -> b` zamienia `a` na `b`.
- `[]` to pusta lista.

Przypisanie **kopiuje** listę:

```zolt
fn main (use => std::io std::lists) {
    a.lst : [1, 2]
    b.lst : a.lst
    b.lst.add 3
    sys::out("a: <a.lst>  b: <b.lst>")
}
```

```
a: [1, 2]  b: [1, 2, 3]
```

Do funkcji listę przekazuje się pożyczką. `&x.lst` pozwala tylko czytać, `&_x.lst` także zmieniać.
Funkcja, która dostaje listę, też potrzebuje `std::lists`, nawet jeśli tylko ją czyta:

```zolt
fn main (use => std::io std::lists) {
    zakupy.lst : [mleko]
    dopisz(&_zakupy.lst, "chleb")
    pokaz(&zakupy.lst)
}

fn dopisz (&_lista.lst, co use => std::lists) {
    lista.lst.add co
}

fn pokaz (&lista.lst use => std::io std::lists) {
    sys::out("lista: <lista.lst>")
}
```

---

## 12. Szablony i obiekty

Szablon opisuje, z czego składa się obiekt: pola (liczby, tekst, prawda/fałsz) z wartościami
początkowymi i metody. Szablony stoją poza funkcjami.

```zolt
gracz : {
    imie : "bezimienny"
    hp : 30
    exp : 0
    zdobadz : (ile) {
        exp : exp + ile
    }
    pokaz : (use => std::io) {
        sys::out("<imie>: hp <hp>, exp <exp>")
    }
}

fn main (use => std::io) {
    let &_gracz.1
    let &_gracz.2
    gracz.1.imie : "Ala"
    gracz.2.hp : gracz.2.hp - 25
    gracz.1.zdobadz(50)
    gracz.1.pokaz()
    gracz.2.pokaz()
    sys::out("cały gracz 2: <gracz.2>")
}
```

```
Ala: hp 30, exp 50
bezimienny: hp 5, exp 0
cały gracz 2: {imie: bezimienny, hp: 5, exp: 0}
```

- `let &_gracz.1` tworzy pierwszy obiekt z szablonu `gracz`, a `let &_gracz.2` drugi. Oba
  startują z wartościami z szablonu.
- Do pól sięga się przez kropkę: `gracz.1.hp`.
- Metody widzą pola swojego obiektu po samej nazwie (`exp`, `imie`). Parametry i `use =>`
  zapisuje się jak w funkcji, tylko bez `fn`.
- Bez `&_` (`let gracz.1`) obiekt jest stały: nie można zmieniać jego pól ani wywoływać metod,
  które je zmieniają.

Obiekt można pożyczyć funkcji tak samo jak zmienną:

```zolt
gracz : { hp : 10 }

fn main (use => std::io) {
    let &_gracz.1
    lecz(&_gracz.1)
    sys::out("<gracz.1.hp>")
}

fn lecz (&_g) {
    g.hp : g.hp + 10
}
```

---

## 13. `use =>` w pigułce

| `use =>` | daje |
|---|---|
| `std::io` | `sys::out`, `sys::read`, `sys::readN` |
| `std::loops` | `loop.N.times`, `loop.inf` |
| `std::lists` | listy `.lst` i wszystko, co na nich działa |

`if`, `while`, `loop,end`, funkcje, liczby, tekst i obiekty nie potrzebują żadnego `use`. Kilka pozycji
można oddzielić spacją albo przecinkiem: `use => std::io std::loops`.

---

## 14. Czytanie błędów

Każdy błąd kompilacji ma ten sam kształt: co jest nie tak, plik z numerem linii i kolumny oraz
wskazany kawałek kodu.

```
błąd: nie znam zmiennej `licznk`, czy chodziło o `licznik`?
 --> gra.zt:3:16
  |
3 |     sys::out("<licznk>")
```

Przy literówkach Zolt podpowiada najbliższą znaną nazwę. Błędy, które wychodzą dopiero przy
uruchomieniu (np. tekst w rachunku), wypisują sam opis, bez numeru linii.

---

## 15. Projekt końcowy: walka

Wszystko naraz: szablon, dwa obiekty, pętla z licznikiem, funkcja z dwiema pożyczkami i lista.

```zolt
__ mała walka: dwóch graczy, tury do skutku __
gracz : {
    imie : "bezimienny"
    hp : 30
    sila : 4
    pokaz : (use => std::io) {
        sys::out("<imie>: <hp> hp")
    }
}

fn main (use => std::io std::loops std::lists) {
    let &_gracz.1
    let &_gracz.2
    gracz.1.imie : "Ala"
    gracz.1.hp : 40
    gracz.2.imie : "Olek"
    gracz.2.sila : 6
    dziennik.lst : []

    loop.inf {
        uderz(&gracz.1, &_gracz.2)
        dziennik.lst.add "Ala"
        if gracz.2.hp <= 0 { loop,end }
        uderz(&gracz.2, &_gracz.1)
        dziennik.lst.add "Olek"
        if gracz.1.hp <= 0 { loop,end }
    }

    gracz.1.pokaz()
    gracz.2.pokaz()
    sys::out("rund: <loop.inf.1 + 1>, ciosy: <dziennik.lst>")
    if gracz.1.hp > 0 { sys::out("wygrywa Ala") } else { sys::out("wygrywa Olek") }
}

fn uderz (&a, &_b) {
    b.hp : b.hp - a.sila
}
```

`uderz(&gracz.1, &_gracz.2)` od razu mówi, kto bije, a kto obrywa: atakujący jest pożyczony
tylko do czytania, a obrońca do zmian.

**Spróbuj:**
1. Dodaj do szablonu pole `obrona` i odejmuj je od siły ciosu w `uderz`.
2. Dodaj trzeciego gracza i walkę „każdy z każdym”.
3. Wczytaj imiona graczy przez `sys::read`.
