#!/bin/sh
# Instaluje wsparcie Zolta w Helixie:
#   1. binarkę `zolt` do ~/.cargo/bin (Helix uruchamia z niej `zolt lsp`),
#   2. gramatykę tree-sitter i zapytania (kolory, wcięcia, obiekty tekstowe),
#   3. wpis języka w languages.toml (tylko jeśli jeszcze go nie ma).
# Można uruchamiać wiele razy: za każdym razem odświeża binarkę, gramatykę i zapytania.
set -eu

ROOT=$(cd "$(dirname "$0")/../.." && pwd)
TS="$ROOT/editors/tree-sitter-zolt"
CFG="${XDG_CONFIG_HOME:-$HOME/.config}/helix"

echo "1/3 cargo install zolt"
cargo install --quiet --path "$ROOT" --force

echo "2/3 gramatyka i zapytania -> $CFG/runtime"
mkdir -p "$CFG/runtime/grammars" "$CFG/runtime/queries/zolt"
cc -shared -fPIC -O2 -I "$TS/src" "$TS/src/parser.c" -o "$CFG/runtime/grammars/zolt.so"
cp "$TS"/queries/*.scm "$CFG/runtime/queries/zolt/"

echo "3/3 languages.toml"
if grep -q '^name = "zolt"' "$CFG/languages.toml" 2>/dev/null; then
    echo "    wpis dla zolt już jest, zostawiam"
else
    cat "$ROOT/editors/helix/languages.toml" >> "$CFG/languages.toml"
    echo "    dopisany"
fi

echo "gotowe; sprawdzenie: helix --health zolt"
