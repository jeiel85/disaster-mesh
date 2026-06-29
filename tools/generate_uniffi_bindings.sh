#!/usr/bin/env sh
set -eu

root="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
output="$root/apps/android/core-bridge/src/main/kotlin"

cd "$root"
cargo build --locked --package mesh-ffi
case "$(uname -s)" in
  Darwin) library="target/debug/libmesh_ffi.dylib" ;;
  MINGW*|MSYS*|CYGWIN*) library="target/debug/mesh_ffi.dll" ;;
  *) library="target/debug/libmesh_ffi.so" ;;
esac
cargo run --locked --package mesh-ffi --features bindgen-cli --bin uniffi-bindgen -- \
  generate --no-format --library "$library" --language kotlin --out-dir "$output"

binding="$output/org/disastermesh/core/mesh_ffi.kt"
python3 - "$binding" <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
lines = [line.rstrip() for line in path.read_text(encoding="utf-8").splitlines()]
lines = [
    line.replace(
        '@file:Suppress("NAME_SHADOWING")',
        '@file:Suppress("NAME_SHADOWING", "UNUSED_EXPRESSION")',
    )
    for line in lines
]
while lines and not lines[-1]:
    lines.pop()
path.write_text("\n".join(lines) + "\n", encoding="utf-8")
PY
