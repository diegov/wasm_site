#!/usr/bin/env bash

set -e

INPUT="$1"

wasm-snip --snip-rust-fmt-code \
          --snip-rust-panicking-code \
          -o "$INPUT" \
          "$INPUT"

wasm-strip "$INPUT"
wasm-opt -o "$INPUT".tmp.wasm -Os --dce "$INPUT"
mv "$INPUT".tmp.wasm "$INPUT"
