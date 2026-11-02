#!/bin/sh
# Zolt kompiluje sam siebie. Wszystko ląduje w target/boot/.
#   etap 0: zolt w Ruście tłumaczy boot/zolt.zt na C   -> zolt0
#   etap 1: zolt0 tłumaczy boot/zolt.zt                -> zolt1.c -> zolt1
#   etap 2: zolt1 tłumaczy boot/zolt.zt                -> zolt2.c
# zolt1.c i zolt2.c muszą być identyczne. Na koniec przykłady i boot/test.zt
# z obu kompilatorów (Rust i Zolt) muszą wypisać to samo.
set -eu
cd "$(dirname "$0")/.."
out=target/boot
cc="cc -std=gnu23 -O2 -fwrapv -I boot"
mkdir -p "$out"

cargo build --release -q
./target/release/zolt c boot/zolt.zt > "$out/zolt0.c"
$cc "$out/zolt0.c" -lm -o "$out/zolt0"
"$out/zolt0" boot/zolt.zt > "$out/zolt1.c"
$cc "$out/zolt1.c" -lm -o "$out/zolt1"
"$out/zolt1" boot/zolt.zt > "$out/zolt2.c"
if ! cmp -s "$out/zolt1.c" "$out/zolt2.c"; then
    echo "błąd: zolt1.c i zolt2.c się różnią"
    exit 1
fi
echo "ok: Zolt kompiluje sam siebie ($out/zolt1)"

# każdy przykład z obu kompilatorów musi wypisać to samo (kalkulator dostaje 1 + 2)
for f in examples/*.zt boot/test.zt; do
    n=$(basename "$f" .zt)
    printf '1\n+\n2\n' | ./target/release/zolt run "$f" > "$out/$n.rust.txt"
    "$out/zolt1" "$f" > "$out/$n.c"
    $cc "$out/$n.c" -lm -o "$out/$n"
    printf '1\n+\n2\n' | "$out/$n" > "$out/$n.zolt.txt"
    if ! cmp -s "$out/$n.rust.txt" "$out/$n.zolt.txt"; then
        echo "błąd: $f wypisuje co innego w obu kompilatorach:"
        diff "$out/$n.rust.txt" "$out/$n.zolt.txt"
        exit 1
    fi
done
echo "ok: examples/*.zt i boot/test.zt wypisują to samo w obu kompilatorach"
