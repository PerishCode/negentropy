#!/usr/bin/env sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname "$0")/../../../.." && pwd)
cd "$ROOT"

cargo build --release --locked -p cli

tmpdir=$(mktemp -d)
trap 'rm -rf "$tmpdir"' EXIT INT TERM
printf 'fn main() {}\n' >"$tmpdir/sample.rs"

out=$("$ROOT/target/release/negentropy" "$tmpdir")
printf '%s\n' "$out"
[ "$out" = "clean" ] || {
  printf '%s\n' "smoke: expected clean, got: $out" >&2
  exit 1
}
printf '%s\n' "smoke: ok"
