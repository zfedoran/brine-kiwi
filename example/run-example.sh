#!/usr/bin/env bash
#
# Simple demo script for the brine-kiwi workspace (CLI binary = "bkiwi").
#
# 1) Compile simple.kiwi → simple.kiwi.bin
# 2) Generate Rust code → generated.rs
#

set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SCHEMA="$HERE/simple.kiwi"

# 1) Compile into a binary (.kiwi.bin)
echo "⏳  Compiling simple.kiwi → simple.kiwi.bin"
cargo run -p brine-kiwi-cli -- compile -i "$SCHEMA" -o "$HERE/simple.kiwi.bin"

# 2) Generate Rust code
echo "⏳  Generating Rust code from simple.kiwi → generated.rs"
cargo run -p brine-kiwi-cli -- gen-rust -i "$SCHEMA" -o "$HERE/src/generated.rs"

echo "✅  Done!"
echo " - simple.kiwi.bin → $HERE/simple.kiwi.bin"
echo " - generated.rs     → $HERE/src/generated.rs"
